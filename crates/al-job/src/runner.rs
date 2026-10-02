use std::io;
use std::path::PathBuf;

use al_core::{StageKind, StageStatus};

use crate::job::JobDefinition;
use crate::manifest::{Manifest, StageRecord};
use crate::registry::ArtifactRegistry;
use crate::stage::{Stage, StageContext, StageError};

/// Executes a job's stages in dependency order with resumable semantics.
///
/// The manifest is saved after every stage transition, so a partially
/// failed job can be resumed later by reusing completed artifacts.
pub struct StageRunner {
    base_dir: PathBuf,
}

impl StageRunner {
    /// Runner writing job directories under `base_dir` (`<base>/<job-id>/`).
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// Directory of a job managed by this runner.
    pub fn job_dir(&self, job_id: &al_core::JobId) -> PathBuf {
        self.base_dir.join(job_id.as_str())
    }

    /// Validate that every declared input carries its pinned content
    /// hash. `JobDefinition` fields are public, so a caller could build
    /// one by struct literal and bypass `create()`'s pinning; this gate
    /// makes the invariant hold regardless of construction path.
    fn validate_inputs(def: &JobDefinition) -> Result<(), StageError> {
        for input in &def.inputs {
            if input.source_sha256.is_none() {
                return Err(StageError::new(
                    StageKind::Ingest,
                    format!(
                        "input {} has no pinned source hash; build jobs with \
                         JobDefinition::create",
                        input.path.display()
                    ),
                ));
            }
        }
        Ok(())
    }

    /// Validate the declared stage dependency graph.
    ///
    /// Fails if a stage's dependencies are not part of the job's stage plan.
    fn validate_plan(def: &JobDefinition, stages: &[Box<dyn Stage>]) -> Result<(), StageError> {
        for stage in stages {
            for dep in stage.depends_on() {
                if !def.stages.contains(&dep) {
                    return Err(StageError::new(
                        stage.kind(),
                        format!("dependency {dep:?} is not part of the stage plan"),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Run a fresh job: create the job directory, manifest, and execute
    /// every planned stage in order.
    pub fn run(
        &self,
        def: &JobDefinition,
        stages: &mut Vec<Box<dyn Stage>>,
    ) -> Result<Manifest, StageError> {
        Self::validate_plan(def, stages)?;
        Self::validate_inputs(def)?;
        let job_dir = self.job_dir(&def.job_id);
        let mut manifest = Manifest::new(
            def.job_id.clone(),
            def.inputs.clone(),
            def.config.clone(),
            &def.stages,
        );
        self.execute(&mut manifest, &job_dir, def, stages)
    }

    /// Resume an existing job: reload its manifest and execute only the
    /// stages that are not yet completed.
    pub fn resume(
        &self,
        def: &JobDefinition,
        stages: &mut Vec<Box<dyn Stage>>,
    ) -> Result<Manifest, StageError> {
        Self::validate_plan(def, stages)?;
        Self::validate_inputs(def)?;
        let job_dir = self.job_dir(&def.job_id);
        let mut manifest = Manifest::load(&job_dir).map_err(|e| {
            StageError::new(StageKind::Ingest, format!("cannot load manifest: {e}"))
        })?;
        if manifest.job_id != def.job_id {
            return Err(StageError::new(
                StageKind::Ingest,
                "manifest job id does not match the supplied job definition",
            ));
        }
        self.execute(&mut manifest, &job_dir, def, stages)
    }

    fn execute(
        &self,
        manifest: &mut Manifest,
        job_dir: &PathBuf,
        def: &JobDefinition,
        stages: &mut Vec<Box<dyn Stage>>,
    ) -> Result<Manifest, StageError> {
        let registry = ArtifactRegistry::new(job_dir);
        for stage_kind in &def.stages {
            let stage = stages
                .iter_mut()
                .find(|stage| stage.kind() == *stage_kind)
                .ok_or_else(|| {
                    StageError::new(
                        *stage_kind,
                        "no stage implementation registered for this kind",
                    )
                })?;
            let record = manifest
                .stage(*stage_kind)
                .cloned()
                .unwrap_or_else(|| StageRecord {
                    stage: *stage_kind,
                    status: StageStatus::Pending,
                    artifacts: Vec::new(),
                    error: None,
                });
            match record.status {
                StageStatus::Completed | StageStatus::Skipped => continue,
                StageStatus::Pending | StageStatus::Failed => {}
            }
            for dep in stage.depends_on() {
                let dep_status = manifest.stage(dep).map(|r| r.status);
                if dep_status != Some(StageStatus::Completed)
                    && dep_status != Some(StageStatus::Skipped)
                {
                    return Err(StageError::new(
                        *stage_kind,
                        format!("dependency {dep:?} is not completed"),
                    ));
                }
            }
            let ctx = StageContext {
                manifest,
                registry: &registry,
                config: &def.config,
            };
            match stage.run(&ctx) {
                Ok(artifacts) => {
                    for artifact in artifacts {
                        manifest.register_artifact(artifact);
                    }
                    if let Some(record) = manifest.stage_mut(*stage_kind) {
                        record.status = StageStatus::Completed;
                        record.error = None;
                    }
                }
                Err(err) => {
                    if let Some(record) = manifest.stage_mut(*stage_kind) {
                        record.status = StageStatus::Failed;
                        record.error = Some(err.message.clone());
                    }
                    manifest.save(job_dir).map_err(|e| {
                        StageError::new(*stage_kind, format!("cannot save manifest: {e}"))
                    })?;
                    return Err(err);
                }
            }
            manifest
                .save(job_dir)
                .map_err(|e| StageError::new(*stage_kind, format!("cannot save manifest: {e}")))?;
        }
        Ok(manifest.clone())
    }
}

impl From<io::Error> for StageError {
    fn from(value: io::Error) -> Self {
        StageError::new(StageKind::Ingest, value.to_string())
    }
}
