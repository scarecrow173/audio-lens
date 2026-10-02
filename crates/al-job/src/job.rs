use std::path::PathBuf;

use al_core::{InputId, InputRole, JobId, StageKind};
use serde::{Deserialize, Serialize};

/// One declared input to an analysis job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobInput {
    /// Stable input identifier.
    pub input_id: InputId,
    /// Generalized role (capture_bug / capture_reference / source_asset / other).
    pub role: InputRole,
    /// Source path recorded for provenance. Absolutized when the job is
    /// created so the stored location does not depend on the working
    /// directory the job was launched from.
    pub path: PathBuf,
}

/// Immutable definition of one analysis job.
///
/// A job is created once and never mutated; re-executing the same inputs and
/// configuration produces a new [`JobId`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobDefinition {
    /// Stable job identifier issued at creation.
    pub job_id: JobId,
    /// Declared inputs with generalized roles.
    pub inputs: Vec<JobInput>,
    /// Free-form job configuration (stage selection, parameters).
    pub config: serde_json::Value,
    /// Ordered stage plan; execution order honours stage dependencies.
    pub stages: Vec<StageKind>,
}

impl JobDefinition {
    /// Create a new immutable job. Always issues a fresh [`JobId`].
    ///
    /// Input paths are absolutized at creation: the manifest records the
    /// resolved location, and a later `resume` from a different working
    /// directory cannot silently read a different file (or lose it).
    pub fn create(
        inputs: Vec<JobInput>,
        config: serde_json::Value,
        stages: Vec<StageKind>,
    ) -> Self {
        let inputs = inputs
            .into_iter()
            .map(|mut input| {
                if let Ok(abs) = std::path::absolute(&input.path) {
                    input.path = abs;
                }
                input
            })
            .collect();
        Self {
            job_id: JobId::new(),
            inputs,
            config,
            stages,
        }
    }
}

/// Builder for [`JobInput`].
pub fn input(role: InputRole, path: impl Into<PathBuf>) -> JobInput {
    JobInput {
        input_id: InputId::new(),
        role,
        path: path.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reexecution_issues_a_new_job_id() {
        let inputs = vec![input(InputRole::CaptureBug, "capture.mkv")];
        let a = JobDefinition::create(
            inputs.clone(),
            serde_json::json!({"preset": "default"}),
            vec![StageKind::Ingest],
        );
        let b = JobDefinition::create(
            inputs,
            serde_json::json!({"preset": "default"}),
            vec![StageKind::Ingest],
        );
        assert_ne!(a.job_id, b.job_id);
        assert_eq!(a.inputs, b.inputs);
    }

    #[test]
    fn create_absolutizes_input_paths() {
        let def = JobDefinition::create(
            vec![input(InputRole::CaptureBug, "relative/capture.mkv")],
            serde_json::json!({}),
            vec![StageKind::Ingest],
        );
        assert!(def.inputs[0].path.is_absolute());
    }
}
