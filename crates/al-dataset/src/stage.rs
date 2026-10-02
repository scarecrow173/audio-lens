//! `DatasetStage`: materializes `<job_dir>/analysis/` — the canonical
//! analysis dataset — and registers one artifact record per store root.
//!
//! Until upstream analysis stages land, the dataset records declared
//! inputs by reference (path + SHA-256, never copied) and marks every
//! upstream role `not_produced` rather than pretending content exists.

use std::path::PathBuf;

use al_core::StageKind;
use al_job::manifest::ArtifactRecord;
use al_job::stage::{Stage, StageContext, StageError};

use crate::manifest::SCHEMA_VERSION;
use crate::provenance::{InputReference, Provenance};
use crate::writer::DatasetWriter;

/// Dataset materialization stage.
#[derive(Debug, Default)]
pub struct DatasetStage;

impl DatasetStage {
    /// Construct the stage.
    pub fn new() -> Self {
        Self
    }
}

impl Stage for DatasetStage {
    fn kind(&self) -> StageKind {
        StageKind::Dataset
    }

    /// The dataset only needs decoded inputs today; as analysis stages
    /// are implemented they insert themselves between Ingest and
    /// Dataset in the pipeline order and become dependencies then.
    fn depends_on(&self) -> Vec<StageKind> {
        vec![StageKind::Ingest]
    }

    fn run(&mut self, ctx: &StageContext<'_>) -> Result<Vec<ArtifactRecord>, StageError> {
        let err = |e: std::io::Error| StageError::new(StageKind::Dataset, e.to_string());
        let job_dir = ctx.registry.job_dir();
        let analysis = job_dir.join("analysis");

        let inputs: Vec<InputReference> = ctx
            .manifest
            .inputs
            .iter()
            .map(|input| InputReference {
                input_id: input.input_id.to_string(),
                role: serde_json::to_value(input.role)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_else(|| format!("{:?}", input.role)),
                source_path: input.path.display().to_string(),
                source_sha256: input.source_sha256.clone().unwrap_or_default(),
            })
            .collect();

        let mut writer =
            DatasetWriter::create(&analysis, inputs.clone(), ctx.config.clone()).map_err(err)?;
        let prov = Provenance {
            input_id: "job".into(),
            algorithm_id: "dataset-materialize".into(),
            algorithm_version: SCHEMA_VERSION.to_string(),
            params: serde_json::json!({}),
            alignment_id: None,
            parents: vec![],
        };
        writer
            .write_json(
                "inputs",
                "declared",
                &serde_json::json!({ "inputs": inputs }),
                prov,
            )
            .map_err(err)?;
        let manifest = writer.finish().map_err(err)?;

        Ok(vec![
            ArtifactRecord {
                artifact_id: ctx.registry.issue(),
                role: "analysis_dataset".into(),
                path: PathBuf::from("analysis"),
                produced_by: Some(StageKind::Dataset),
                inputs: vec![],
                params: serde_json::json!({
                    "schema_version": manifest.schema_version,
                    "artifact_count": manifest.artifacts.len(),
                }),
            },
            ArtifactRecord {
                artifact_id: ctx.registry.issue(),
                role: "dataset_inputs".into(),
                path: PathBuf::from("analysis/inputs/declared.json"),
                produced_by: Some(StageKind::Dataset),
                inputs: vec![],
                params: serde_json::json!({
                    "input_count": inputs.len(),
                    "by_reference": true,
                }),
            },
        ])
    }
}
