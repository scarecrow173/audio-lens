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
        SampleFormat::PcmU8 => 128.0,
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
