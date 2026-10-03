//! Two-stage residual search.
//!
//! Stage 1 fits the accepted alignment's mix onto the capture and
//! subtracts it; stage 2 re-runs the locate methods on the residual
//! so a quieter source becomes findable. The result records the
//! parent source, parent alignment id, estimated gain, the channel
//! transformation/mixing matrix that was subtracted, pre/post
//! subtraction energy, confidence, and the child locate result.

use serde::{Deserialize, Serialize};

use al_core::AlignmentId;

use crate::mixer::{MixerEstimate, fit_mix};
use crate::result::LocateResult;

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
        .flat_map(|c| c.iter())
        .map(|v| v * v)
        .fold((0.0, 0usize), |(s, n), v| (s + v, n + 1));
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

/// Run the two-stage residual search. `locate_residual` re-runs the
/// method set on the residual capture (provided by the caller so the
/// stage controls the configuration).
pub fn residual_search(
    parent_source: &str,
    parent_alignment: AlignmentId,
    source_channels: &[Vec<f64>],
    capture_channels: &[Vec<f64>],
    ridge: f64,
    locate_residual: impl FnOnce(&[Vec<f64>]) -> LocateResult,
) -> Option<ResidualSearch> {
    let before = rms(capture_channels);
    let (residual, est) = subtract_fit(source_channels, capture_channels, ridge)?;
    let after = rms(&residual);
    let child = locate_residual(&residual);

    let removed = if before > 0.0 {
        (1.0 - after / before).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let gain = est
        .matrix
        .iter()
        .flat_map(|r| r.iter())
        .map(|v| v * v)
        .sum::<f64>()
        .sqrt();
    let child_accepted = f64::from(child.accepted as u8);
    Some(ResidualSearch {
        parent_source: parent_source.to_string(),
        parent_alignment,
        estimated_gain: gain,
        channel_transform: est.matrix,
        energy: ResidualEnergy { before, after },
        confidence: (0.5 * removed + 0.5 * child_accepted).min(1.0),
        child,
    })
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
        let out = residual_search(
            "source-1",
            AlignmentId::new(),
            &[src],
            &[cap],
            1e-4,
            child_locate,
        )
        .unwrap();
        assert_eq!(out.parent_source, "source-1");
        assert!((out.estimated_gain - 0.5).abs() < 0.05);
        assert_eq!(out.channel_transform.len(), 1);
        assert!(out.energy.before > out.energy.after);
        assert!(out.confidence > 0.5);
        assert!(out.child.accepted);
    }
}
