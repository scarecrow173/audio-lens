//! Locate candidates and per-method evidence records.

use al_core::Validity;
use serde::{Deserialize, Serialize};

/// One candidate position produced by a locate method.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocateCandidate {
    /// Capture sample offset of the candidate.
    pub position: u64,
    /// Correlation score at the peak (`-1..=1` for NCC-like methods;
    /// negative scores indicate polarity-inverted matches).
    pub score: f64,
    /// Peak magnitude relative to the correlation background (median).
    pub peak_to_background: f64,
    /// Width of the peak at half maximum, in samples.
    pub peak_width: u64,
    /// First/second peak ratio, recorded on the top-ranked candidate
    /// when a second peak exists.
    pub peak_ratio_12: Option<f64>,
    /// `[source_channel, capture_channel]` that produced this
    /// candidate, when evaluated per channel pair.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub channel_pair: Option<[u32; 2]>,
}

/// Evidence produced by one independent locate method.
///
/// Rejection is recorded, not thrown: consensus and downstream
/// provenance inspect `validity`/`reject_reason` for every method.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MethodEvidence {
    /// Method identifier (e.g. `full_ncc`).
    pub method: String,
    /// Method version (independent from any schema version).
    pub method_version: String,
    /// Ranked candidate positions (empty when rejected).
    pub candidates: Vec<LocateCandidate>,
    /// Whether the method could produce meaningful evidence.
    pub validity: Validity,
    /// Why the method's evidence is weak/unusable, when applicable.
    pub reject_reason: Option<String>,
}
