//! locate 入力準備: レート統一、永続化、元座標への対応を所有する。

use std::{fs, io, path::PathBuf};

use al_job::{manifest::ArtifactRecord, stage::StageContext};
use ruviform_numeric::resample::{ResampleParams, ResamplePlan};
use serde::{Deserialize, Serialize};

/// `transform` 記録のアルゴリズムバージョン。アルゴリズム同一性は
/// [`ResampleParams::ALGORITHM_ID`] が正本(旧 `al-math` と同一値で、
/// #150 比較記録で bit-exact 検証済み)。
const RESAMPLE_VERSION: &str = "1";

use crate::{
    decode::DecodedInput,
    result::{LocateResult, PcmReference, SampleRange},
};

pub(crate) fn bad(message: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_string())
}

/// 保存 PCM の配置・変換・座標対応の正本。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedParams {
    pub input_id: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: u64,
    pub original_sample_rate: u32,
    pub original_frames: u64,
    pub format: String,
    pub amplitude_scale: String,
    pub transform: serde_json::Value,
    /// t0 秒の有理数。現在の FIR は遅延を完全に補償する。
    pub time_origin_numerator: i64,
    pub time_origin_denominator: u64,
    /// Serialized size of the produced artifact in bytes
    /// (`frames * channels * 8`). Recorded so resume's artifact
    /// verification can detect a truncated relaxed-tier write.
    /// Deterministic for a given transform, so it participates in
    /// dedup equality like any other parameter.
    pub byte_len: u64,
}

/// `normalized_pcm` の provenance 記録。plan 内部表現を個別フィールドへ
/// 写さず、#170 の正準表現(`stable_id`)で変換を一意に識別する
/// (AL#102 で残留と判断した逆結合の解消)。
fn transform(plan: &ResamplePlan) -> serde_json::Value {
    let identity = plan.source_rate() == plan.target_rate();
    serde_json::json!({
        "algorithm_id": if identity { "identity" } else { ResampleParams::ALGORITHM_ID },
        "algorithm_version": RESAMPLE_VERSION,
        "stable_id": plan.stable_id(),
    })
}

impl NormalizedParams {
    pub fn validate(&self) -> io::Result<()> {
        let plan = ResamplePlan::new(self.original_sample_rate, self.sample_rate).map_err(bad)?;
        let original = usize::try_from(self.original_frames).map_err(bad)?;
        if self.channels == 0
            || self.format != "interleaved-f64le"
            || self.amplitude_scale != "decoded-float-normalized"
            || self.time_origin_numerator != 0
            || self.time_origin_denominator != 1
            || self.frames != plan.output_len(original).map_err(bad)? as u64
            || self.byte_len != self.frames * u64::from(self.channels) * 8
            || self.transform != transform(&plan)
        {
            return Err(bad("inconsistent normalized_pcm layout or transform"));
        }
        Ok(())
    }

    /// 半開区間の開始を floor、終了を ceil し、元の長さに制限する。
    pub fn original_range(&self, range: SampleRange) -> io::Result<SampleRange> {
        self.validate_range(range)?;
        let rs = self.original_sample_rate as u128;
        let rn = self.sample_rate as u128;
        Ok(SampleRange {
            start: ((range.start as u128 * rs) / rn).min(self.original_frames as u128) as u64,
            end: (range.end as u128 * rs)
                .div_ceil(rn)
                .min(self.original_frames as u128) as u64,
        })
    }

    pub fn validate_range(&self, range: SampleRange) -> io::Result<()> {
        if range.start > range.end || range.end > self.frames {
            return Err(bad("normalized_pcm interval outside frame bounds"));
        }
        Ok(())
    }
}

pub struct PreparedInput {
    pub decoded: DecodedInput,
    pub record: ArtifactRecord,
    pub params: NormalizedParams,
}

impl PreparedInput {
    pub fn reference(&self) -> PcmReference {
        PcmReference {
            artifact_id: self.record.artifact_id.clone(),
            sample_rate: self.params.sample_rate,
            channels: self.params.channels,
        }
    }

    pub fn attach(&self, capture: &Self, result: &mut LocateResult) -> io::Result<()> {
        result.normalized_source_range = result.source_range;
        result.source_range = self.params.original_range(result.normalized_source_range)?;
        result.source_pcm = Some(self.reference());
        result.capture_pcm = Some(capture.reference());
        result.params["normalization"] = serde_json::to_value(&self.params).map_err(bad)?;
        Ok(())
    }
}

/// params と元成果物 ID をキーに、同じ変換のみ再利用する。
pub fn prepare_input(
    input: &DecodedInput,
    rate: u32,
    records: &mut Vec<ArtifactRecord>,
    ctx: &StageContext<'_>,
) -> io::Result<PreparedInput> {
    let plan = ResamplePlan::new(input.sample_rate, rate).map_err(bad)?;
    let original_frames = input.channel_samples.first().map_or(0, Vec::len);
    if input.channel_samples.len() != input.channels as usize
        || input
            .channel_samples
            .iter()
            .any(|c| c.len() != original_frames)
    {
        return Err(bad("decoded PCM channel layout inconsistent"));
    }
    let mut params = NormalizedParams {
        input_id: input.input_id.clone(),
        sample_rate: rate,
        channels: input.channels,
        frames: plan.output_len(original_frames).map_err(bad)? as u64,
        original_sample_rate: input.sample_rate,
        original_frames: original_frames as u64,
        format: "interleaved-f64le".into(),
        amplitude_scale: "decoded-float-normalized".into(),
        transform: transform(&plan),
        time_origin_numerator: 0,
        time_origin_denominator: 1,
        byte_len: 0, // replaced below once frames are known
    };
    params.byte_len = params.frames * u64::from(params.channels) * 8;
    let value = serde_json::to_value(&params).map_err(bad)?;
    if let Some(record) = records
        .iter()
        .chain(&ctx.manifest.artifacts)
        .find(|r| r.role == "normalized_pcm" && r.inputs == input.artifact_ids && r.params == value)
    {
        return crate::decode::read_normalized_record(record, ctx);
    }
    let channels: Vec<Vec<f64>> = input
        .channel_samples
        .iter()
        .map(|c| plan.resample(c).map_err(bad))
        .collect::<io::Result<_>>()?;
    let artifact_id = ctx.registry.issue();
    let path = PathBuf::from(format!("locate/normalized/{artifact_id}.f64le"));
    let absolute = al_job::fsutil::checked_job_path(ctx.registry.job_dir(), &path)?;
    fs::create_dir_all(
        absolute
            .parent()
            .ok_or_else(|| bad("PCM path has no parent"))?,
    )?;
    // Frame-major interleave staged through one reusable row
    // buffer: one `extend_from_slice` per frame instead of one
    // 8-byte extend per sample.
    let ch = channels.len();
    let mut bytes = Vec::with_capacity(params.frames as usize * ch * 8);
    let mut row = vec![0u8; ch * 8];
    for frame in 0..params.frames as usize {
        for (c, channel) in channels.iter().enumerate() {
            row[c * 8..c * 8 + 8].copy_from_slice(&channel[frame].to_le_bytes());
        }
        bytes.extend_from_slice(&row);
    }
    // Relaxed tier: normalized PCM is regenerable from the decoded
    // input, so the fsync is skipped and resume's artifact
    // verification re-runs the stage if power loss dropped it.
    al_job::fsutil::write_atomic(&absolute, &bytes)?;
    let record = ArtifactRecord {
        artifact_id: artifact_id.clone(),
        role: "normalized_pcm".into(),
        path,
        produced_by: Some(al_core::StageKind::Locate),
        inputs: input.artifact_ids.clone(),
        params: value,
    };
    records.push(record.clone());
    Ok(PreparedInput {
        decoded: DecodedInput {
            input_id: input.input_id.clone(),
            sample_rate: rate,
            channels: input.channels,
            channel_samples: channels,
            artifact_ids: vec![artifact_id],
        },
        record,
        params,
    })
}
