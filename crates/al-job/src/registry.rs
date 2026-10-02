use std::path::{Path, PathBuf};

use al_core::ArtifactId;

use crate::manifest::{ArtifactRecord, Manifest};

/// Issues artifact ids and resolves their locations inside a job directory.
///
/// The registry never stores artifact state itself; the manifest is the
/// canonical record. The registry only owns the naming convention.
#[derive(Debug, Clone)]
pub struct ArtifactRegistry {
    job_dir: PathBuf,
}

impl ArtifactRegistry {
    /// Create a registry rooted at a job directory.
    pub fn new(job_dir: impl Into<PathBuf>) -> Self {
        Self {
            job_dir: job_dir.into(),
        }
    }

    /// The job directory this registry is rooted at.
    pub fn job_dir(&self) -> &Path {
        &self.job_dir
    }

    /// Issue a fresh artifact id.
    pub fn issue(&self) -> ArtifactId {
        ArtifactId::new()
    }

    /// Resolve an artifact record to its absolute path.
    ///
    /// Artifacts without job-local storage (an empty `path`, e.g. declared
    /// inputs backed by an external file) resolve to `None` rather than to
    /// the job directory itself.
    pub fn resolve(&self, manifest: &Manifest, id: &ArtifactId) -> Option<PathBuf> {
        manifest
            .artifact(id)
            .filter(|record| !record.path.as_os_str().is_empty())
            .map(|record| self.job_dir.join(&record.path))
    }

    /// Resolve all artifacts carrying the given role label.
    /// Artifacts without job-local storage are skipped.
    pub fn resolve_by_role<'m>(
        &self,
        manifest: &'m Manifest,
        role: &str,
    ) -> Vec<(&'m ArtifactRecord, PathBuf)> {
        manifest
            .artifacts
            .iter()
            .filter(|record| record.role == role && !record.path.as_os_str().is_empty())
            .map(|record| (record, self.job_dir.join(&record.path)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::input;
    use crate::manifest::StageRecord;
    use al_core::{InputRole, JobId, StageKind, StageStatus};

    #[test]
    fn resolve_maps_artifact_to_job_dir() {
        let dir = PathBuf::from("jobs/example");
        let registry = ArtifactRegistry::new(&dir);
        let mut manifest = Manifest {
            job_id: JobId::new(),
            inputs: vec![input(InputRole::CaptureBug, "capture.mkv")],
            config: serde_json::json!({}),
            stages: vec![StageRecord {
                stage: StageKind::Ingest,
                status: StageStatus::Pending,
                artifacts: vec![],
                error: None,
            }],
            artifacts: vec![],
        };
        let id = registry.issue();
        manifest.register_artifact(ArtifactRecord {
            artifact_id: id.clone(),
            role: "decoded_pcm".into(),
            path: "intermediate/decoded.wav".into(),
            produced_by: Some(StageKind::Ingest),
            inputs: vec![],
            params: serde_json::json!({}),
        });
        let resolved = registry.resolve(&manifest, &id).unwrap();
        assert_eq!(resolved, dir.join("intermediate/decoded.wav"));
        assert_eq!(registry.resolve_by_role(&manifest, "decoded_pcm").len(), 1);
    }

    #[test]
    fn resolve_returns_none_for_artifacts_without_job_storage() {
        let dir = PathBuf::from("jobs/example");
        let registry = ArtifactRegistry::new(&dir);
        let manifest = Manifest::new(
            JobId::new(),
            vec![input(InputRole::CaptureBug, "capture.mkv")],
            serde_json::json!({}),
            &[],
        );
        let input_id = manifest
            .input_artifact(&manifest.inputs[0].input_id)
            .unwrap();
        // A declared input has no job-local storage; it must not resolve
        // to the job directory itself.
        assert_eq!(registry.resolve(&manifest, &input_id), None);
        assert!(
            registry
                .resolve_by_role(&manifest, "declared_input")
                .is_empty()
        );
    }
}
