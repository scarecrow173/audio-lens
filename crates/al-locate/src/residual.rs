//! Two-stage residual search.
//!
//! Stage 1 fits the accepted alignment's mix onto the capture and
//! subtracts it; stage 2 re-runs the locate methods on the residual
//! so a quieter source becomes findable. The result records the
//! parent source, parent alignment id, estimated gain, the channel
//! transformation/mixing matrix that was subtracted, pre/post
//! subtraction energy, confidence, and the child locate result.

use serde::{Deserialize, Serialize};

use al_core::{AlignmentId, ArtifactId};

use crate::dsp;
use crate::mixer::{MixerEstimate, fit_mix};
use crate::normalize::{NormalizedParams, PreparedInput, bad};
use crate::result::{LocateResult, PcmReference};

/// 残差 signal version の配置と減算来歴。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ResidualPcmParams {
    pub pcm: NormalizedParams,
    pub encoding: String,
    pub parent_source: String,
    pub parent_alignment: AlignmentId,
    pub parent_artifact: ArtifactId,
    pub source_pcm: PcmReference,
    pub original_capture_pcm: PcmReference,
    pub channel_transform: Vec<Vec<f64>>,
    pub energy: ResidualEnergy,
}

/// 探索と同じ f64 サンプルを WAV に保存する。親の全 child で共有する。
pub(crate) fn persist_residual(
    prepared: &PreparedResidual,
    source: &PreparedInput,
    capture: &PreparedInput,
    parent: &LocateResult,
    parent_artifact: &ArtifactId,
    ctx: &al_job::stage::StageContext<'_>,
) -> std::io::Result<PreparedInput> {
    let frames = capture.params.frames as usize;
    if prepared.channels.len() != capture.params.channels as usize
        || prepared
            .channels
            .iter()
            .any(|c| c.len() != frames || c.iter().any(|s| !s.is_finite()))
    {
        return Err(bad("residual channel layout inconsistent"));
    }
    let artifact_id = ctx.registry.issue();
    let path = std::path::PathBuf::from(format!(
        "locate/residual/{}_{artifact_id}.wav",
        source.decoded.input_id
    ));
    let samples: Vec<f64> = (0..frames)
        .flat_map(|f| prepared.channels.iter().map(move |c| c[f]))
        .collect();
    al_dataset::write_wav(
        &al_job::fsutil::checked_job_path(ctx.registry.job_dir(), &path)?,
        al_media::wav::WavEncoding::Float64,
        capture.params.channels,
        capture.params.sample_rate,
        &samples,
    )?;
    let params = ResidualPcmParams {
        pcm: capture.params.clone(),
        encoding: "wav-f64le".into(),
        parent_source: source.decoded.input_id.clone(),
        parent_alignment: parent.alignment_id.clone(),
        parent_artifact: parent_artifact.clone(),
        source_pcm: source.reference(),
        original_capture_pcm: capture.reference(),
        channel_transform: prepared.estimate.matrix.clone(),
        energy: prepared.energy.clone(),
    };
    let record = al_job::manifest::ArtifactRecord {
        artifact_id: artifact_id.clone(),
        role: "residual".into(),
        path,
        produced_by: Some(al_core::StageKind::Locate),
        inputs: vec![
            source.record.artifact_id.clone(),
            capture.record.artifact_id.clone(),
            parent_artifact.clone(),
        ],
        params: serde_json::to_value(params).map_err(bad)?,
    };
    Ok(PreparedInput {
        decoded: crate::decode::DecodedInput {
            input_id: capture.decoded.input_id.clone(),
            sample_rate: capture.params.sample_rate,
            channels: capture.params.channels,
            channel_samples: prepared.channels.clone(),
            artifact_ids: vec![artifact_id],
        },
        record,
        params: capture.params.clone(),
    })
}

/// Energy figures bracketing the subtraction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResidualEnergy {
    /// Capture RMS before subtraction.
    pub before: f64,
    /// Residual RMS after subtraction.
    pub after: f64,
}

/// Result of a two-stage residual search for one source pair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResidualSearch {
    /// 探索に使った残差の明示参照。
    pub residual_pcm: Option<PcmReference>,
    /// Parent source identity (manifest input id).
    pub parent_source: String,
    /// Alignment id of the parent acceptance that was subtracted.
    pub parent_alignment: AlignmentId,
    /// Estimated overall gain of the subtracted source.
    pub estimated_gain: f64,
    /// The channel transformation/mixing matrix actually subtracted:
    /// `matrix[capture_ch][source_ch]`, signed coefficients.
    pub channel_transform: Vec<Vec<f64>>,
    /// Pre/post subtraction energy.
    pub energy: ResidualEnergy,
    /// Confidence in the residual result `0..=1`: derived from the
    /// energy actually removed and the child's own acceptance.
    pub confidence: f64,
    /// Child locate result computed on the residual capture.
    pub child: LocateResult,
}

fn rms(channels: &[Vec<f64>]) -> f64 {
    let (sum, n) = channels
        .iter()
        .fold((0.0, 0usize), |(s, n), c| (s + dsp::sum_sq(c), n + c.len()));
    if n == 0 { 0.0 } else { (sum / n as f64).sqrt() }
}

/// Estimate the mix of `source_channels` in `capture_channels`,
/// subtract it, and return the residual channels plus the fit.
///
/// Channels are deinterleaved; the source is assumed already aligned
/// to the capture window. Returns `None` when the fit cannot be
/// solved.
pub fn subtract_fit(
    source_channels: &[Vec<f64>],
    capture_channels: &[Vec<f64>],
    ridge: f64,
) -> Option<(Vec<Vec<f64>>, MixerEstimate)> {
    let len = capture_channels.first()?.len();
    let src: Vec<Vec<f64>> = source_channels
        .iter()
        .map(|c| {
            let mut v = c.clone();
            v.resize(len, 0.0);
            v
        })
        .collect();
    let est = fit_mix(&src, capture_channels, ridge)?;
    let mut residual = capture_channels.to_vec();
    for (cap, row) in residual.iter_mut().zip(&est.matrix) {
        for (t, v) in cap.iter_mut().enumerate() {
            let pred: f64 = src.iter().zip(row).map(|(s, &m)| s[t] * m).sum();
            *v -= pred;
        }
    }
    Some((residual, est))
}

/// Capture-minus-fitted-parent residual plus the fit metadata.
///
/// The residual depends only on the parent alignment, so the stage
/// prepares it once and re-runs every child source's locate methods
/// over it — recomputing the subtraction per child would repeat the
/// same fit dozens of times.
pub struct PreparedResidual {
    /// Residual channels after subtracting the fitted parent mix.
    pub channels: Vec<Vec<f64>>,
    /// The fitted mix that was subtracted.
    pub estimate: MixerEstimate,
    /// Pre/post subtraction energy.
    pub energy: ResidualEnergy,
}

/// Fit the parent's mix onto the capture and subtract it.
/// Returns `None` when the fit cannot be solved.
pub fn prepare_residual(
    source_channels: &[Vec<f64>],
    capture_channels: &[Vec<f64>],
    ridge: f64,
) -> Option<PreparedResidual> {
    let before = rms(capture_channels);
    let (residual, est) = subtract_fit(source_channels, capture_channels, ridge)?;
    let after = rms(&residual);
    Some(PreparedResidual {
        channels: residual,
        estimate: est,
        energy: ResidualEnergy { before, after },
    })
}

/// Assemble the residual-search record for one child locate result
/// computed over a [`PreparedResidual`].
pub fn assemble_residual_search(
    parent_source: &str,
    parent_alignment: AlignmentId,
    prepared: &PreparedResidual,
    child: LocateResult,
) -> ResidualSearch {
    let removed = if prepared.energy.before > 0.0 {
        (1.0 - prepared.energy.after / prepared.energy.before).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let gain = prepared
        .estimate
        .matrix
        .iter()
        .flat_map(|r| r.iter())
        .map(|v| v * v)
        .sum::<f64>()
        .sqrt();
    let child_accepted = f64::from(child.accepted as u8);
    ResidualSearch {
        residual_pcm: child.capture_pcm.clone(),
        parent_source: parent_source.to_string(),
        parent_alignment,
        estimated_gain: gain,
        channel_transform: prepared.estimate.matrix.clone(),
        energy: prepared.energy.clone(),
        confidence: (0.5 * removed + 0.5 * child_accepted).min(1.0),
        child,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidate::MethodEvidence;
    use crate::result::SampleRange;
    use al_core::{Confidence, Validity};

    fn noise(seed: u64, n: usize) -> Vec<f64> {
        let mut s = seed;
        (0..n)
            .map(|_| {
                s = s
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ((s >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0
            })
            .collect()
    }

    #[test]
    fn quiet_source_found_after_subtraction() {
        let bgm = noise(1, 4096);
        let quiet = noise(2, 200);
        let mut cap = bgm.clone();
        for (i, &v) in quiet.iter().enumerate() {
            cap[1000 + i] += 0.02 * v;
        }
        let (res, est) = subtract_fit(&[bgm], &[cap.clone()], 1e-4).unwrap();
        assert!(est.fit_quality < 0.05);
        let before = rms(&[cap]);
        let after = rms(&res);
        assert!(after < before * 0.1);
    }

    #[test]
    fn provenance_is_preserved() {
        let src = noise(1, 2048);
        let cap: Vec<f64> = src.iter().map(|v| 0.5 * v).collect();
        let child_locate = |_: &[Vec<f64>]| LocateResult {
            alignment_id: AlignmentId::new(),
            source_input_id: "src".into(),
            capture_input_id: "cap".into(),
            source_range: SampleRange { start: 0, end: 1 },
            capture_range: SampleRange { start: 0, end: 1 },
            normalized_source_range: SampleRange { start: 0, end: 0 },
            source_pcm: None,
            capture_pcm: None,
            evidence: vec![MethodEvidence {
                method: "stub".into(),
                method_version: "0".into(),
                candidates: vec![],
                validity: Validity::Valid,
                reject_reason: None,
            }],
            agreeing_methods: 2,
            position_spread: 0,
            confidence: Confidence::High,
            validity: Validity::Valid,
            reject_reason: None,
            accepted: true,
            algorithm_id: "stub".into(),
            algorithm_version: "0".into(),
            params: serde_json::json!({}),
        };
        let prepared = prepare_residual(&[src], &[cap], 1e-4).unwrap();
        let out = assemble_residual_search(
            "source-1",
            AlignmentId::new(),
            &prepared,
            child_locate(&prepared.channels),
        );
        assert_eq!(out.parent_source, "source-1");
        assert!((out.estimated_gain - 0.5).abs() < 0.05);
        assert_eq!(out.channel_transform.len(), 1);
        assert!(out.energy.before > out.energy.after);
        assert!(out.confidence > 0.5);
        assert!(out.child.accepted);
    }
}
