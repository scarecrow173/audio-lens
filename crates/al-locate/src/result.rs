//! The locate result record: every accepted or rejected localization
//! carries the spec's required output fields plus its alignment id.

use al_core::{AlignmentId, ArtifactId, Confidence, Validity};
use serde::{Deserialize, Serialize};

use crate::candidate::MethodEvidence;

/// A closed `[start, end)` range in samples.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SampleRange {
    /// Start sample (inclusive).
    pub start: u64,
    /// End sample (exclusive).
    pub end: u64,
}

/// alignment が比較に用いた PCM の参照と期待する配置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PcmReference {
    pub artifact_id: ArtifactId,
    pub sample_rate: u32,
    pub channels: u16,
}

/// One localization result for a `(source, capture)` pair.
///
/// `accepted` distinguishes a consensus-accepted alignment from a
/// full evidence record that merely failed acceptance — rejections
/// are persisted too, never silently dropped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocateResult {
    /// Alignment id issued for this result.
    pub alignment_id: AlignmentId,
    /// Input id of the source asset.
    pub source_input_id: String,
    /// Input id of the capture.
    pub capture_input_id: String,
    /// Covered region of the source asset (samples).
    pub source_range: SampleRange,
    /// Matched region of the capture (samples).
    pub capture_range: SampleRange,
    /// 正規化 source PCM 上の区間。source_range は元レートの区間。
    pub normalized_source_range: SampleRange,
    /// 未デコード等の評価不能な結果のみ参照を持たない。
    pub source_pcm: Option<PcmReference>,
    pub capture_pcm: Option<PcmReference>,
    /// Per-method evidence (accepted and rejected alike).
    pub evidence: Vec<MethodEvidence>,
    /// Number of independent methods agreeing on the position.
    pub agreeing_methods: u32,
    /// Spread (max − min) of the agreeing positions in samples.
    pub position_spread: u64,
    /// Confidence in the result.
    pub confidence: Confidence,
    /// Whether the measurement is meaningful.
    pub validity: Validity,
    /// Why the result is invalid or low-confidence.
    pub reject_reason: Option<String>,
    /// Whether consensus accepted this result.
    pub accepted: bool,
    /// Algorithm id.
    pub algorithm_id: String,
    /// Algorithm version (independent from schema versions).
    pub algorithm_version: String,
    /// Algorithm parameters (thresholds, tolerances).
    pub params: serde_json::Value,
}
