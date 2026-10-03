//! Provenance records: every stored artifact can be traced back to the
//! inputs, algorithm, parameters, alignment, and parents that made it.

use serde::{Deserialize, Serialize};

/// Provenance of one dataset artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    /// Id of the input the artifact derives from.
    pub input_id: String,
    /// Algorithm id that produced the artifact.
    pub algorithm_id: String,
    /// Algorithm version (independent from the dataset schema version).
    pub algorithm_version: String,
    /// Algorithm parameters.
    pub params: serde_json::Value,
    /// Alignment id underpinning the artifact, when applicable.
    pub alignment_id: Option<String>,
    /// Parent / intermediate artifact ids in the derivation chain.
    pub parents: Vec<String>,
}

/// A declared input recorded by reference — never copied into the
/// dataset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputReference {
    /// Input id from the job manifest.
    pub input_id: String,
    /// Role the input was declared with.
    pub role: String,
    /// Absolute path to the source file at job-creation time.
    pub source_path: String,
    /// SHA-256 of the source bytes pinned at job creation.
    pub source_sha256: String,
}

/// Build the `InputReference` list from a job manifest. Shared by
/// every stage that creates the dataset so all writers record the
/// same input identity. Fails explicitly on an unpinned input hash.
pub fn input_references(
    manifest: &al_job::manifest::Manifest,
) -> std::io::Result<Vec<InputReference>> {
    manifest
        .inputs
        .iter()
        .map(|input| {
            let source_sha256 = input.source_sha256.clone().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("input {} has no pinned source hash", input.input_id),
                )
            })?;
            Ok(InputReference {
                input_id: input.input_id.to_string(),
                role: serde_json::to_value(input.role)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_else(|| format!("{:?}", input.role)),
                source_path: input.path.display().to_string(),
                source_sha256,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_core::{InputId, InputRole, JobId, StageKind};
    use al_job::job::JobInput;
    use al_job::manifest::Manifest;
    use std::path::PathBuf;

    fn manifest_with(inputs: Vec<JobInput>) -> Manifest {
        Manifest::new(
            JobId::new(),
            inputs,
            serde_json::json!({}),
            &[StageKind::Ingest],
        )
    }

    #[test]
    fn references_preserve_role_path_and_pin() {
        let m = manifest_with(vec![JobInput {
            input_id: InputId::new(),
            role: InputRole::SourceAsset,
            path: PathBuf::from("F:/assets/src.wav"),
            source_sha256: Some("ab".repeat(32)),
        }]);
        let refs = input_references(&m).unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].role, "source_asset");
        assert_eq!(refs[0].source_path, "F:/assets/src.wav");
        assert_eq!(refs[0].source_sha256, "ab".repeat(32));
    }

    #[test]
    fn unpinned_input_fails_explicitly() {
        let m = manifest_with(vec![JobInput {
            input_id: InputId::new(),
            role: InputRole::Other,
            path: PathBuf::from("x.wav"),
            source_sha256: None,
        }]);
        assert_eq!(
            input_references(&m).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
    }
}
