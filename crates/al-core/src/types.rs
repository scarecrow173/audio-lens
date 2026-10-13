use serde::{Deserialize, Serialize};

/// Generalized input role for an analysis job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputRole {
    /// Capture containing the behavior under investigation.
    CaptureBug,
    /// Normal/reference capture.
    CaptureReference,
    /// Source asset to be located inside captures.
    SourceAsset,
    /// Any other input with a caller-defined role.
    Other,
}

/// Pipeline stage kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageKind {
    /// Media decode and metadata extraction.
    Ingest,
    /// Source localization / alignment.
    Locate,
    /// Always-on cheap analysis features.
    CheapAnalysis,
    /// Reference-aligned comparison features.
    ReferenceAnalysis,
    /// Candidate-driven expensive analysis.
    ExpensiveAnalysis,
    /// Analysis dataset materialization.
    Dataset,
    /// Visualization dataset build.
    Visualization,
    /// Human-readable export.
    Export,
}

impl StageKind {
    /// Canonical pipeline order; earlier kinds are prerequisites of later ones.
    pub const PIPELINE_ORDER: &'static [StageKind] = &[
        StageKind::Ingest,
        StageKind::Locate,
        StageKind::CheapAnalysis,
        StageKind::ReferenceAnalysis,
        StageKind::ExpensiveAnalysis,
        StageKind::Dataset,
        StageKind::Visualization,
        StageKind::Export,
    ];

    /// All stage kinds that must complete before this one.
    pub fn dependencies(&self) -> Vec<StageKind> {
        Self::PIPELINE_ORDER
            .iter()
            .copied()
            .take_while(|kind| kind != self)
            .collect()
    }
}

/// Whether a measurement is meaningful for a range.
///
/// The variant set distinguishes every way a measurement can fail to
/// be trustworthy. `Valid` (no anomaly found) is never conflated with
/// the inability to measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Validity {
    /// Measurement is valid (may still be "no anomaly").
    Valid,
    /// The analyzed range is silent or below usable energy.
    SilenceInsufficientEnergy,
    /// Too few valid windows or samples support the measurement.
    InsufficientSupport,
    /// Correlation evidence is too weak to trust the measurement.
    LowCorrelation,
    /// Coherence is too low to trust the measurement.
    LowCoherence,
    /// The alignment underpinning the measurement is ambiguous —
    /// 合議結果だけでなく、argmax が一意でないメソッド証拠の
    /// マークにも用いる。
    AmbiguousAlignment,
    /// The requested frequency range is outside what is supported.
    OutOfSupportedFrequencyRange,
    /// The input could not be decoded or is unavailable.
    DecodeInputUnavailable,
    /// The measurement does not apply to this artifact.
    NotApplicable,
    /// 必要な比較基準がなく、この観測を構成できない。
    Unsupported,
    /// The computation itself failed.
    ComputationFailed,
}

/// Confidence that a measured observation reflects a real phenomenon.
/// Ordered `Low < Medium < High` so downstream stages can take the
/// weakest upstream confidence with `min`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// Little evidence.
    Low,
    /// Some evidence.
    Medium,
    /// Strong evidence.
    High,
}

/// Outcome of a stage execution inside a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    /// Not yet run.
    Pending,
    /// Completed and artifacts recorded.
    Completed,
    /// Failed; earlier stages are preserved.
    Failed,
    /// Skipped because inputs or configuration made it unnecessary.
    Skipped,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_round_trip_snake_case() {
        let json = serde_json::to_string(&InputRole::CaptureBug).unwrap();
        assert_eq!(json, "\"capture_bug\"");
        let back: InputRole = serde_json::from_str(&json).unwrap();
        assert_eq!(back, InputRole::CaptureBug);
    }

    #[test]
    fn confidence_is_ordered_for_min_propagation() {
        assert!(Confidence::Low < Confidence::Medium);
        assert!(Confidence::Medium < Confidence::High);
        assert_eq!(Confidence::Low.min(Confidence::High), Confidence::Low);
    }

    #[test]
    fn validity_is_distinct_from_absence() {
        // "No anomaly" and "cannot measure" are different states.
        assert_ne!(Validity::Valid, Validity::InsufficientSupport);
        assert_ne!(Validity::Valid, Validity::ComputationFailed);
        assert_ne!(Validity::DecodeInputUnavailable, Validity::NotApplicable);
    }
}
