//! Shared ingest types: errors, audio metadata, decode history, and
//! integrity observations.

use std::fmt;
use std::io;

pub(crate) const DECODE_UNAVAILABLE_KIND: &str = "decode_unavailable";

/// Error produced by the ingest layer.
///
/// 未対応入力は `InputUnavailable`、構造不正は `Malformed` として区別する。
/// 部分 ingest の局所失敗も同じ区分を `UnavailableKind` に保存する。
#[derive(Debug)]
pub enum MediaError {
    /// The input format is unknown or not yet supported.
    InputUnavailable { reason: String },
    /// A recognized container is malformed or truncated.
    Malformed { reason: String },
    /// Filesystem-level failure while reading the input.
    Io(io::Error),
}

impl MediaError {
    /// Construct an `InputUnavailable` error.
    pub fn input_unavailable(reason: impl Into<String>) -> Self {
        Self::InputUnavailable {
            reason: reason.into(),
        }
    }

    /// Construct a `Malformed` error.
    pub fn malformed(reason: impl Into<String>) -> Self {
        Self::Malformed {
            reason: reason.into(),
        }
    }
}

impl fmt::Display for MediaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputUnavailable { reason } => {
                write!(f, "decode/input unavailable: {reason}")
            }
            Self::Malformed { reason } => write!(f, "decode/malformed: {reason}"),
            Self::Io(e) => write!(f, "decode/io error: {e}"),
        }
    }
}

impl std::error::Error for MediaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for MediaError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

/// Codec / sample format of a decoded audio stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SampleFormat {
    /// Unsigned 8-bit PCM.
    PcmU8,
    /// Signed 8-bit PCM.
    PcmS8,
    /// Signed 16-bit PCM.
    PcmS16,
    /// Signed 24-bit PCM.
    PcmS24,
    /// Signed 32-bit PCM.
    PcmS32,
    /// 32-bit IEEE float.
    Float32,
    /// 64-bit IEEE float.
    Float64,
    /// Compressed or otherwise not plain PCM (kept for record only).
    Other,
}

/// Audio stream metadata recorded at ingest.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AudioMetadata {
    /// Codec name (e.g. `pcm_s16le`).
    pub codec: String,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Channel count.
    pub channels: u16,
    /// Sample format / bit depth.
    pub sample_format: SampleFormat,
    /// Total frame count (samples per channel), when known.
    pub frames: Option<u64>,
    /// Stream duration in seconds, when known.
    pub duration_secs: Option<f64>,
    /// SHA-256 of the source file (hex).
    pub source_hash: String,
    /// Normalization / resampling / conversion history applied so far.
    /// Empty means the stored PCM is exactly what the decoder produced.
    pub history: Vec<String>,
}

/// One integrity observation recorded during decode.
///
/// Observations are facts (timestamp gaps, silence spans, ...); whether
/// they matter is decided downstream, never here.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IntegrityObservation {
    /// Observation kind, e.g. `timestamp_gap`, `zero_span`.
    pub kind: String,
    /// Human- / machine-readable detail.
    pub detail: serde_json::Value,
}

impl IntegrityObservation {
    /// decoder と direct parser が共有する利用不能観測の表現。
    pub(crate) fn decode_unavailable(
        media_kind: &str,
        failure: &crate::decoder::Unavailable,
        mut detail: serde_json::Value,
    ) -> Self {
        detail["media_kind"] = media_kind.into();
        detail["kind"] = serde_json::to_value(failure.kind).expect("enum is serializable");
        detail["reason"] = failure.reason.clone().into();
        Self {
            kind: DECODE_UNAVAILABLE_KIND.into(),
            detail,
        }
    }
}
