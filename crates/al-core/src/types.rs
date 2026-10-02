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

/// Whether a measurement is meaningful for a range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Validity {
    /// Measurement is valid.
    Valid,
    /// Measurement could not be produced (distinguishes from "no anomaly").
    Unavailable,
    /// Measurement exists but is degraded or partial.
    Degraded,
}

/// Confidence that a measured observation reflects a real phenomenon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    fn validity_is_distinct_from_absence() {
        assert_ne!(Validity::Valid, Validity::Unavailable);
        assert_ne!(Validity::Unavailable, Validity::Degraded);
    }
}
