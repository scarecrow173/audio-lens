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
    /// SHA-256 of the input content pinned at job creation.
    ///
    /// Stages that read the input verify the content against this hash so
    /// a `resume` cannot silently ingest a replaced file under the same
    /// job id. [`JobDefinition::create`] fills this in for every file
    /// input; `None` only remains for inputs built outside `create`.
    #[serde(default)]
    pub source_sha256: Option<String>,
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
    /// Input paths are absolutized and their content is hashed at
    /// creation: the manifest records the resolved location and the exact
    /// bytes the job is pinned to, so a later `resume` from a different
    /// working directory cannot silently read different content. An
    /// unreadable input fails here instead of midway through a stage.
    /// A caller-provided `source_sha256` is kept as-is.
    pub fn create(
        inputs: Vec<JobInput>,
        config: serde_json::Value,
        stages: Vec<StageKind>,
    ) -> std::io::Result<Self> {
        let inputs = inputs
            .into_iter()
            .map(|mut input| -> std::io::Result<JobInput> {
                input.path = std::path::absolute(&input.path)?;
                if input.source_sha256.is_none() {
                    let bytes = std::fs::read(&input.path)?;
                    input.source_sha256 = Some(al_core::sha256_hex(&bytes));
                }
                Ok(input)
            })
            .collect::<Result<Vec<_>, std::io::Error>>()?;
        Ok(Self {
            job_id: JobId::new(),
            inputs,
            config,
            stages,
        })
    }
}

/// Builder for [`JobInput`].
pub fn input(role: InputRole, path: impl Into<PathBuf>) -> JobInput {
    JobInput {
        input_id: InputId::new(),
        role,
        path: path.into(),
        source_sha256: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(tag: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("al-job-{tag}-{}", JobId::new()));
        std::fs::write(&path, b"fixture").unwrap();
        path
    }

    #[test]
    fn reexecution_issues_a_new_job_id() {
        let inputs = vec![input(InputRole::CaptureBug, fixture("reexec"))];
        let a = JobDefinition::create(
            inputs.clone(),
            serde_json::json!({"preset": "default"}),
            vec![StageKind::Ingest],
        )
        .unwrap();
        let b = JobDefinition::create(
            inputs,
            serde_json::json!({"preset": "default"}),
            vec![StageKind::Ingest],
        )
        .unwrap();
        assert_ne!(a.job_id, b.job_id);
        assert_eq!(a.inputs, b.inputs);
    }

    #[test]
    fn create_pins_input_content() {
        let path = fixture("pin");
        let def = JobDefinition::create(
            vec![input(InputRole::CaptureBug, &path)],
            serde_json::json!({}),
            vec![StageKind::Ingest],
        )
        .unwrap();
        let input = &def.inputs[0];
        assert!(input.path.is_absolute());
        assert_eq!(
            input.source_sha256.as_deref(),
            Some(al_core::sha256_hex(b"fixture").as_str())
        );
    }

    #[test]
    fn create_fails_for_unreadable_input() {
        let missing = std::env::temp_dir().join(format!("al-job-missing-{}", JobId::new()));
        assert!(
            JobDefinition::create(
                vec![input(InputRole::CaptureBug, missing)],
                serde_json::json!({}),
                vec![StageKind::Ingest],
            )
            .is_err()
        );
    }
}
