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
    /// Path relative to the job directory.
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

    /// Create an empty manifest for a freshly created job.
    pub fn new(
        job_id: JobId,
        inputs: Vec<JobInput>,
        config: serde_json::Value,
        stages: &[StageKind],
    ) -> Self {
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
            artifacts: Vec::new(),
        }
    }

    /// Persist the manifest under `job_dir`.
    pub fn save(&self, job_dir: &Path) -> io::Result<()> {
        fs::create_dir_all(job_dir)?;
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(job_dir.join(Self::FILE_NAME), json)
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
        assert_eq!(manifest.artifacts.len(), 2);
        let locate = manifest.stage(StageKind::Locate).unwrap();
        assert_eq!(locate.artifacts.len(), 1);
    }

    #[test]
    fn lineage_walks_upstream_chain() {
        let manifest = sample_manifest();
        let locate = &manifest.artifacts[1];
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
