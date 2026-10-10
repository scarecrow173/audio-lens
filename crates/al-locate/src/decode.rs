//! Shared decoded-PCM loader.
//!
//! Restores one job input's `decoded_pcm` artifact into
//! channel-deinterleaved, float-normalized samples plus the lineage
//! ids of the artifacts it came from. Sample format comes from the
//! sibling `ingest_metadata` artifact — decoded PCM keeps the
//! source-native scale, so the format decides normalization.
//!
//! Any stage that consumes decoded input uses this loader rather
//! than re-implementing artifact lookup and scaling.

use std::fs;
use std::io;

use al_core::ArtifactId;
use al_job::fsutil::checked_job_path;
use al_job::job::JobInput;
use al_job::stage::StageContext;
use al_media::metadata::SampleFormat;

use crate::dsp::deinterleave;
use crate::normalize::{NormalizedParams, PreparedInput, bad};
use crate::result::{LocateResult, PcmReference, SampleRange};

/// normalized PCM または残差 signal version を artifact ID で解決し、配置と区間を検証する。
pub fn read_normalized_pcm(
    reference: &PcmReference,
    range: SampleRange,
    ctx: &StageContext<'_>,
) -> io::Result<PreparedInput> {
    let record = ctx
        .manifest
        .artifacts
        .iter()
        .find(|a| a.artifact_id == reference.artifact_id)
        .ok_or_else(|| bad("normalized_pcm reference missing from manifest"))?;
    let pcm = read_normalized_record(record, ctx)?;
    if reference.sample_rate != pcm.params.sample_rate || reference.channels != pcm.params.channels
    {
        return Err(bad("normalized_pcm reference rate/channels inconsistent"));
    }
    pcm.params.validate_range(range)?;
    Ok(pcm)
}

pub(crate) fn read_normalized_record(
    record: &al_job::manifest::ArtifactRecord,
    ctx: &StageContext<'_>,
) -> io::Result<PreparedInput> {
    if record.role == "residual" {
        return read_residual_record(record, ctx);
    }
    if record.role != "normalized_pcm" || record.produced_by != Some(al_core::StageKind::Locate) {
        return Err(bad("artifact is not locate normalized_pcm"));
    }
    let params: NormalizedParams = serde_json::from_value(record.params.clone()).map_err(bad)?;
    params.validate()?;
    let input = ctx
        .manifest
        .inputs
        .iter()
        .find(|i| i.input_id.as_str() == params.input_id)
        .ok_or_else(|| bad("normalized_pcm input missing"))?;
    let declared = ctx
        .manifest
        .input_artifact(&input.input_id)
        .ok_or_else(|| bad("normalized_pcm input artifact missing"))?;
    let originals: Vec<_> = record
        .inputs
        .iter()
        .map(|id| {
            ctx.manifest
                .artifacts
                .iter()
                .find(|a| &a.artifact_id == id)
                .ok_or_else(|| bad("normalized_pcm original lineage missing"))
        })
        .collect::<io::Result<_>>()?;
    if originals.iter().filter(|a| a.role == "decoded_pcm").count() != 1
        || originals
            .iter()
            .filter(|a| a.role == "ingest_metadata")
            .count()
            > 1
        || originals.iter().any(|a| {
            !a.inputs.contains(&declared)
                || !matches!(a.role.as_str(), "decoded_pcm" | "ingest_metadata")
        })
        || originals
            .iter()
            .filter(|a| a.role == "decoded_pcm")
            .any(|a| {
                a.params["sample_rate"].as_u64() != Some(params.original_sample_rate as u64)
                    || a.params["channels"].as_u64() != Some(params.channels as u64)
            })
    {
        return Err(bad("normalized_pcm original lineage inconsistent"));
    }
    let expected_path =
        std::path::PathBuf::from(format!("locate/normalized/{}.f64le", record.artifact_id));
    if record.path != expected_path {
        return Err(bad("normalized_pcm path inconsistent with artifact id"));
    }
    let bytes = fs::read(checked_job_path(ctx.registry.job_dir(), &record.path)?)?;
    let length = params.frames as u128 * params.channels as u128 * 8;
    if bytes.len() as u128 != length {
        return Err(bad(
            "normalized_pcm byte length inconsistent with frame layout",
        ));
    }
    let samples: Vec<f64> = bytes
        .as_chunks::<8>()
        .0
        .iter()
        .map(|b| f64::from_le_bytes(*b))
        .collect();
    if samples.iter().any(|s| !s.is_finite()) {
        return Err(bad("normalized_pcm has non-finite samples"));
    }
    Ok(PreparedInput {
        decoded: DecodedInput {
            input_id: params.input_id.clone(),
            sample_rate: params.sample_rate,
            channels: params.channels,
            channel_samples: deinterleave(&samples, params.channels),
            artifact_ids: vec![record.artifact_id.clone()],
        },
        record: record.clone(),
        params,
    })
}

/// 減算を再実行せず、保存された残差 signal version を検証して復元する。
fn read_residual_record(
    record: &al_job::manifest::ArtifactRecord,
    ctx: &StageContext<'_>,
) -> io::Result<PreparedInput> {
    use crate::residual::ResidualPcmParams;
    let params: ResidualPcmParams = serde_json::from_value(record.params.clone()).map_err(bad)?;
    params.pcm.validate()?;
    let expected_path = std::path::PathBuf::from(format!(
        "locate/residual/{}_{}.wav",
        params.parent_source, record.artifact_id
    ));
    if record.produced_by != Some(al_core::StageKind::Locate)
        || params.encoding != "wav-f64le"
        || record.path != expected_path
        || record.inputs
            != vec![
                params.source_pcm.artifact_id.clone(),
                params.original_capture_pcm.artifact_id.clone(),
                params.parent_artifact.clone(),
            ]
    {
        return Err(bad("residual PCM layout or lineage inconsistent"));
    }
    // 親は必ず通常の normalized PCM。循環・残差の暗黙な再減算を拒否する。
    let resolve_original = |reference: &PcmReference| -> io::Result<PreparedInput> {
        let original = ctx
            .manifest
            .artifacts
            .iter()
            .find(|a| a.artifact_id == reference.artifact_id)
            .ok_or_else(|| bad("residual original PCM missing"))?;
        if original.role != "normalized_pcm" {
            return Err(bad("residual original PCM role inconsistent"));
        }
        read_normalized_pcm(reference, SampleRange { start: 0, end: 0 }, ctx)
    };
    let source = resolve_original(&params.source_pcm)?;
    let capture = resolve_original(&params.original_capture_pcm)?;
    let parent = ctx
        .manifest
        .artifacts
        .iter()
        .find(|a| a.artifact_id == params.parent_artifact)
        .ok_or_else(|| bad("residual parent alignment missing"))?;
    if parent.role != "locate_result" || parent.produced_by != Some(al_core::StageKind::Locate) {
        return Err(bad("residual parent alignment role inconsistent"));
    }
    let body: serde_json::Value = serde_json::from_slice(&fs::read(checked_job_path(
        ctx.registry.job_dir(),
        &parent.path,
    )?)?)
    .map_err(bad)?;
    let alignment: LocateResult = serde_json::from_value(body["result"].clone()).map_err(bad)?;
    if !alignment.accepted
        || alignment.alignment_id != params.parent_alignment
        || alignment.source_input_id != params.parent_source
        || alignment.source_pcm.as_ref() != Some(&params.source_pcm)
        || alignment.capture_pcm.as_ref() != Some(&params.original_capture_pcm)
        || !parent.inputs.contains(&params.source_pcm.artifact_id)
        || !parent
            .inputs
            .contains(&params.original_capture_pcm.artifact_id)
        || source.params.input_id != params.parent_source
        || alignment.capture_input_id != params.pcm.input_id
        || capture.params != params.pcm
        || params.channel_transform.len() != params.pcm.channels as usize
        || params.channel_transform.iter().any(|row| {
            row.len() != source.params.channels as usize || row.iter().any(|v| !v.is_finite())
        })
        || !params.energy.before.is_finite()
        || !params.energy.after.is_finite()
        || params.energy.before < 0.0
        || params.energy.after < 0.0
    {
        return Err(bad("residual subtraction lineage inconsistent"));
    }
    let audio = al_dataset::read_wav(&checked_job_path(ctx.registry.job_dir(), &record.path)?)?;
    if audio.sample_format != SampleFormat::Float64
        || audio.sample_rate != params.pcm.sample_rate
        || audio.channels != params.pcm.channels
        || audio.samples.len() as u128 != params.pcm.frames as u128 * params.pcm.channels as u128
        || audio.samples.iter().any(|v| !v.is_finite())
    {
        return Err(bad("residual WAV inconsistent with PCM layout"));
    }
    Ok(PreparedInput {
        decoded: DecodedInput {
            input_id: params.pcm.input_id.clone(),
            sample_rate: audio.sample_rate,
            channels: audio.channels,
            channel_samples: deinterleave(&audio.samples, audio.channels),
            artifact_ids: vec![record.artifact_id.clone()],
        },
        record: record.clone(),
        params: params.pcm,
    })
}

/// alignment 比較の共有読込境界。元 PCM への fallback は行わない。
pub fn read_alignment_pcm(
    result: &LocateResult,
    ctx: &StageContext<'_>,
) -> io::Result<Option<(PreparedInput, PreparedInput)>> {
    let (Some(src), Some(cap)) = (&result.source_pcm, &result.capture_pcm) else {
        let empty = SampleRange { start: 0, end: 0 };
        if !result.accepted
            && result.source_pcm.is_none()
            && result.capture_pcm.is_none()
            && result.source_range == empty
            && result.capture_range == empty
            && result.normalized_source_range == empty
            && matches!(
                result.validity,
                al_core::Validity::DecodeInputUnavailable | al_core::Validity::InsufficientSupport
            )
        {
            return Ok(None); // 未デコード・空入力の明示的な mark
        }
        return Err(bad("alignment missing normalized PCM references"));
    };
    let source = read_normalized_pcm(src, result.normalized_source_range, ctx)?;
    let capture = read_normalized_pcm(cap, result.capture_range, ctx)?;
    let residual_parent = &result.params["residual_parent_alignment"];
    if source.record.role != "normalized_pcm"
        || (capture.record.role == "residual"
            && (residual_parent.is_null()
                || residual_parent != &capture.record.params["parent_alignment"]))
        || (capture.record.role != "residual" && !residual_parent.is_null())
        || source.params.input_id != result.source_input_id
        || capture.params.input_id != result.capture_input_id
        || source.params.sample_rate != capture.params.sample_rate
        || capture.params.original_sample_rate != capture.params.sample_rate
        || result.params["normalization"] != source.record.params
        || source
            .params
            .original_range(result.normalized_source_range)?
            != result.source_range
        || (result.accepted
            && result.normalized_source_range.end - result.normalized_source_range.start
                < result.capture_range.end - result.capture_range.start)
    {
        return Err(bad("alignment PCM coordinates inconsistent"));
    }
    Ok(Some((source, capture)))
}

/// Decoded PCM of one input, with channel structure restored.
#[derive(Debug)]
pub struct DecodedInput {
    /// Manifest input id.
    pub input_id: String,
    /// Sample rate from the `decoded_pcm` artifact params.
    pub sample_rate: u32,
    /// Channel count from the `decoded_pcm` artifact params.
    pub channels: u16,
    /// Deinterleaved channels, normalized to float range.
    /// Always `channels` entries; each may be empty.
    pub channel_samples: Vec<Vec<f64>>,
    /// Artifact ids this input's PCM and metadata came from.
    pub artifact_ids: Vec<ArtifactId>,
}

/// Scale factor mapping one `SampleFormat` into `[-1, 1]` float.
/// Integer PCM is stored integer-scaled; spectral-envelope
/// comparisons and residual energies are absolute-scale-sensitive.
pub fn format_scale(format: SampleFormat) -> f64 {
    match format {
        SampleFormat::PcmU8 | SampleFormat::PcmS8 => 128.0,
        SampleFormat::PcmS16 => 32768.0,
        SampleFormat::PcmS24 => 8_388_608.0,
        SampleFormat::PcmS32 => 2_147_483_648.0,
        SampleFormat::Float32 | SampleFormat::Float64 | SampleFormat::Other => 1.0,
    }
}

/// Restore `input`'s decoded PCM. Returns `Ok(None)` when the input
/// has no `decoded_pcm` artifact (never ingested).
pub fn decode_input(
    input: &JobInput,
    ctx: &StageContext<'_>,
) -> Result<Option<DecodedInput>, io::Error> {
    let input_artifact = ctx.manifest.input_artifact(&input.input_id);
    let decoded = ctx.manifest.artifacts.iter().find(|a| {
        a.role == "decoded_pcm"
            && input_artifact
                .as_ref()
                .is_some_and(|ia| a.inputs.contains(ia))
    });
    let Some(decoded) = decoded else {
        return Ok(None);
    };
    let job_dir = ctx.registry.job_dir();
    fn bad(msg: impl Into<String>) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, msg.into())
    }
    let sample_rate = decoded.params["sample_rate"]
        .as_u64()
        .ok_or_else(|| bad("decoded_pcm artifact missing sample_rate"))?
        as u32;
    if sample_rate == 0 {
        return Err(bad("decoded_pcm artifact has sample_rate 0"));
    }
    let channels = decoded.params["channels"]
        .as_u64()
        .ok_or_else(|| bad("decoded_pcm artifact missing channels"))? as u16;
    if channels == 0 {
        return Err(bad("decoded_pcm artifact has 0 channels"));
    }

    let metadata = ctx.manifest.artifacts.iter().find(|a| {
        a.role == "ingest_metadata"
            && input_artifact
                .as_ref()
                .is_some_and(|ia| a.inputs.contains(ia))
    });
    let scale = match metadata {
        Some(meta) => {
            let text = fs::read_to_string(checked_job_path(job_dir, &meta.path)?)?;
            let json: serde_json::Value =
                serde_json::from_str(&text).map_err(|e| bad(e.to_string()))?;
            let format: SampleFormat = json
                .pointer("/audio/sample_format")
                .cloned()
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or(SampleFormat::Other);
            format_scale(format)
        }
        None => 1.0,
    };

    let bytes = fs::read(checked_job_path(job_dir, &decoded.path)?)?;
    if bytes.len() % 8 != 0 {
        return Err(bad(format!(
            "decoded_pcm has {} bytes, not a multiple of 8",
            bytes.len()
        )));
    }
    let samples: Vec<f64> = bytes
        .as_chunks::<8>()
        .0
        .iter()
        .map(|b| f64::from_le_bytes(*b) / scale)
        .collect();
    let channel_samples = deinterleave(&samples, channels);
    let mut artifact_ids = vec![decoded.artifact_id.clone()];
    if let Some(m) = metadata {
        artifact_ids.push(m.artifact_id.clone());
    }
    Ok(Some(DecodedInput {
        input_id: input.input_id.to_string(),
        sample_rate,
        channels,
        channel_samples,
        artifact_ids,
    }))
}
