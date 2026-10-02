use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use al_core::{ArtifactId, JobId, StageKind, StageStatus};
use serde::{Deserialize, Serialize};

use crate::job::JobInput;

/// Record of one produced artifact and its lineage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactRecord {
    /// Stable artifact identifier.
    pub artifact_id: ArtifactId,
    /// Free-form role label (e.g. `decoded_pcm`, `locate_result`).
    pub role: String,
    /// Path relative to the job directory. Empty for artifacts without
    /// job-local storage — e.g. declared inputs, whose external location is
    /// recorded in `params.source_path` instead.
    pub path: PathBuf,
    /// Stage that produced the artifact (`None` for declared inputs).
    pub produced_by: Option<StageKind>,
    /// Upstream artifacts this artifact was derived from.
    pub inputs: Vec<ArtifactId>,
    /// Parameters used to produce the artifact.
    pub params: serde_json::Value,
}

/// Execution record of one stage inside a job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StageRecord {
    /// Stage kind.
    pub stage: StageKind,
    /// Latest execution status.
    pub status: StageStatus,
    /// Artifacts emitted by this stage.
    pub artifacts: Vec<ArtifactId>,
    /// Error message when the stage failed.
    pub error: Option<String>,
}

/// Canonical manifest for one job: inputs, configuration, stage records,
/// and the full artifact lineage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    /// Owning job.
    pub job_id: JobId,
    /// Declared inputs.
    pub inputs: Vec<JobInput>,
    /// Job configuration snapshot.
    pub config: serde_json::Value,
    /// Per-stage execution records in plan order.
    pub stages: Vec<StageRecord>,
    /// Every artifact produced or declared, in registration order.
    pub artifacts: Vec<ArtifactRecord>,
}

impl Manifest {
    /// Manifest filename inside a job directory.
    pub const FILE_NAME: &'static str = "manifest.json";

    /// Create a manifest for a freshly created job. Each declared input is
    /// registered as an artifact with `produced_by: None` so downstream
    /// artifacts can point their lineage at the source they came from.
    pub fn new(
        job_id: JobId,
        inputs: Vec<JobInput>,
        config: serde_json::Value,
        stages: &[StageKind],
    ) -> Self {
        let artifacts = inputs
            .iter()
            .map(|input| ArtifactRecord {
                artifact_id: ArtifactId::new(),
                role: "declared_input".into(),
                // External inputs live outside the job directory, so the
                // location goes to params rather than the job-relative
                // `path` contract.
                path: PathBuf::new(),
                produced_by: None,
                inputs: Vec::new(),
                params: serde_json::json!({
                    "input_id": input.input_id,
                    "input_role": input.role,
                    "source_path": input.path,
                    "source_path_abs": std::path::absolute(&input.path)
                        .ok()
                        .map(|p| p.display().to_string()),
                    "source_sha256": input.source_sha256,
                }),
            })
            .collect();
        Self {
            job_id,
            inputs,
            config,
            stages: stages
                .iter()
                .map(|stage| StageRecord {
                    stage: *stage,
                    status: StageStatus::Pending,
                    artifacts: Vec::new(),
                    error: None,
                })
                .collect(),
            artifacts,
        }
    }

    /// Artifact id registered for a declared input, if any.
    pub fn input_artifact(&self, input_id: &al_core::InputId) -> Option<ArtifactId> {
        self.artifacts
            .iter()
            .find(|record| {
                record.role == "declared_input"
                    && record.params.get("input_id").and_then(|v| v.as_str())
                        == Some(input_id.as_str())
            })
            .map(|record| record.artifact_id.clone())
    }

    /// Persist the manifest under `job_dir`.
    ///
    /// The write is atomic (temp file + rename): a crash mid-save leaves
    /// either the previous manifest or the new one intact, never a
    /// truncated canonical record that would make the job unresumable.
    pub fn save(&self, job_dir: &Path) -> io::Result<()> {
        fs::create_dir_all(job_dir)?;
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let tmp = job_dir.join("manifest.json.tmp");
        fs::write(&tmp, json)?;
        fs::rename(&tmp, job_dir.join(Self::FILE_NAME))
    }

    /// Load a manifest from `job_dir`.
    pub fn load(job_dir: &Path) -> io::Result<Self> {
        let text = fs::read_to_string(job_dir.join(Self::FILE_NAME))?;
        serde_json::from_str(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Mutable access to a stage record.
    pub fn stage_mut(&mut self, stage: StageKind) -> Option<&mut StageRecord> {
        self.stages.iter_mut().find(|record| record.stage == stage)
    }

    /// Immutable access to a stage record.
    pub fn stage(&self, stage: StageKind) -> Option<&StageRecord> {
        self.stages.iter().find(|record| record.stage == stage)
    }

    /// Register a produced artifact and attach it to its producing stage.
    pub fn register_artifact(&mut self, record: ArtifactRecord) {
        if let Some(producer) = record.produced_by
            && let Some(stage) = self.stage_mut(producer)
        {
            stage.artifacts.push(record.artifact_id.clone());
        }
        self.artifacts.push(record);
    }

    /// Look up an artifact by id.
    pub fn artifact(&self, id: &ArtifactId) -> Option<&ArtifactRecord> {
        self.artifacts
            .iter()
            .find(|record| &record.artifact_id == id)
    }

    /// Reconstruct the full upstream lineage of an artifact.
    pub fn lineage_of(&self, id: &ArtifactId) -> Vec<ArtifactId> {
        let mut order = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![id.clone()];
        while let Some(current) = stack.pop() {
            if !seen.insert(current.clone()) {
                continue;
            }
            if let Some(record) = self.artifact(&current) {
                order.push(record.artifact_id.clone());
                for upstream in &record.inputs {
                    stack.push(upstream.clone());
                }
            }
        }
        order
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::input;
    use al_core::InputRole;

    fn sample_manifest() -> Manifest {
        let mut manifest = Manifest::new(
            JobId::new(),
            vec![input(InputRole::CaptureBug, "capture.mkv")],
            serde_json::json!({}),
            &[StageKind::Ingest, StageKind::Locate],
        );
        let decoded = ArtifactId::new();
        manifest.register_artifact(ArtifactRecord {
            artifact_id: decoded.clone(),
            role: "decoded_pcm".into(),
            path: "intermediate/decoded.wav".into(),
            produced_by: Some(StageKind::Ingest),
            inputs: vec![],
            params: serde_json::json!({}),
        });
        manifest.register_artifact(ArtifactRecord {
            artifact_id: ArtifactId::new(),
            role: "locate_result".into(),
            path: "intermediate/locate.json".into(),
            produced_by: Some(StageKind::Locate),
            inputs: vec![decoded],
            params: serde_json::json!({"method": "ncc"}),
        });
        manifest
    }

    #[test]
    fn register_links_artifact_to_stage() {
        let manifest = sample_manifest();
        // One declared input artifact + two produced artifacts.
        assert_eq!(manifest.artifacts.len(), 3);
        let locate = manifest.stage(StageKind::Locate).unwrap();
        assert_eq!(locate.artifacts.len(), 1);
    }

    #[test]
    fn declared_input_is_an_artifact() {
        let manifest = sample_manifest();
        let input = &manifest.inputs[0];
        let id = manifest.input_artifact(&input.input_id).unwrap();
        let record = manifest.artifact(&id).unwrap();
        assert_eq!(record.role, "declared_input");
        assert_eq!(record.produced_by, None);
        // External inputs keep their location in params, not in the
        // job-relative path contract.
        assert!(record.path.as_os_str().is_empty());
        assert_eq!(record.params["source_path"].as_str(), Some("capture.mkv"));
    }

    #[test]
    fn lineage_walks_upstream_chain() {
        let manifest = sample_manifest();
        let locate = manifest
            .artifacts
            .iter()
            .find(|a| a.role == "locate_result")
            .unwrap();
        let lineage = manifest.lineage_of(&locate.artifact_id);
        assert_eq!(lineage.len(), 2);
        assert_eq!(lineage[0], locate.artifact_id);
    }

    #[test]
    fn manifest_round_trips_through_json() {
        let manifest = sample_manifest();
        let dir = std::env::temp_dir().join(format!("al-test-{}", JobId::new()));
        manifest.save(&dir).unwrap();
        let loaded = Manifest::load(&dir).unwrap();
        assert_eq!(manifest, loaded);
        std::fs::remove_dir_all(&dir).ok();
    }
}
