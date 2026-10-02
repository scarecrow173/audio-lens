//! `DatasetStage`: materializes `<job_dir>/analysis/` — the canonical
//! analysis dataset — and registers one artifact record per store root.
//!
//! Declared inputs are recorded by reference (path + SHA-256, never
//! copied); decoded PCM from ingest is materialized as WAV under
//! `analysis/inputs/` — the dataset's decoded-audio role. Upstream
//! roles without producers stay `not_produced`.

use std::fs;
use std::path::PathBuf;

use al_core::StageKind;
use al_job::manifest::ArtifactRecord;
use al_job::stage::{Stage, StageContext, StageError};
use al_media::metadata::SampleFormat;
use al_media::wav::WavEncoding;

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

fn encoding_for(format: SampleFormat) -> WavEncoding {
    match format {
        SampleFormat::PcmS16 => WavEncoding::PcmS16,
        SampleFormat::PcmS24 => WavEncoding::PcmS24,
        SampleFormat::PcmS32 => WavEncoding::PcmS32,
        // Formats without a native-scale encoding are written as
        // float to preserve the decoded values exactly.
        _ => WavEncoding::Float32,
    }
}

impl DatasetStage {
    /// Materialize one input's decoded PCM as `inputs/<id>.wav`.
    /// Returns the artifact ids the WAV derives from.
    fn materialize_pcm(
        &self,
        input: &al_job::job::JobInput,
        ctx: &StageContext<'_>,
        writer: &mut DatasetWriter,
    ) -> Result<Vec<al_core::ArtifactId>, StageError> {
        let err = |e: std::io::Error| StageError::new(StageKind::Dataset, e.to_string());
        let input_artifact = ctx.manifest.input_artifact(&input.input_id);
        let decoded = ctx.manifest.artifacts.iter().find(|a| {
            a.role == "decoded_pcm"
                && input_artifact
                    .as_ref()
                    .is_some_and(|ia| a.inputs.contains(ia))
        });
        let Some(decoded) = decoded else {
            return Ok(Vec::new());
        };
        let job_dir = ctx.registry.job_dir();
        let bytes = fs::read(job_dir.join(&decoded.path)).map_err(err)?;
        let samples: Vec<f64> = bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|b| f64::from_le_bytes(*b))
            .collect();
        let sample_rate = decoded.params["sample_rate"].as_u64().unwrap_or(0) as u32;
        let channels = decoded.params["channels"].as_u64().unwrap_or(0) as u16;

        let metadata = ctx.manifest.artifacts.iter().find(|a| {
            a.role == "ingest_metadata"
                && input_artifact
                    .as_ref()
                    .is_some_and(|ia| a.inputs.contains(ia))
        });
        let format = metadata
            .and_then(|m| fs::read_to_string(job_dir.join(&m.path)).ok())
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|j| j.pointer("/audio/sample_format").cloned())
            .and_then(|v| serde_json::from_value::<SampleFormat>(v).ok())
            .unwrap_or(SampleFormat::Other);

        let mut lineage = Vec::new();
        if let Some(ia) = input_artifact {
            lineage.push(ia);
        }
        lineage.push(decoded.artifact_id.clone());
        if let Some(m) = metadata {
            lineage.push(m.artifact_id.clone());
        }

        writer
            .write_audio(
                "inputs",
                &input.input_id.to_string(),
                encoding_for(format),
                channels,
                sample_rate,
                &samples,
                Provenance {
                    input_id: input.input_id.to_string(),
                    algorithm_id: "dataset-materialize".into(),
                    algorithm_version: SCHEMA_VERSION.to_string(),
                    params: serde_json::json!({"source": "decoded_pcm"}),
                    alignment_id: None,
                    parents: lineage.iter().map(|a| a.as_str().to_string()).collect(),
                },
            )
            .map_err(err)?;
        Ok(lineage)
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

        // A pre-existing analysis dir can only come from a previous
        // failed attempt of this stage (Completed stages are skipped
        // by the runner). The dataset is regenerated wholesale, so the
        // partial output is removed — resume must actually be able to
        // complete.
        if analysis.exists() {
            fs::remove_dir_all(&analysis).map_err(err)?;
        }

        let mut declared_lineage = Vec::new();
        let inputs: Vec<InputReference> = ctx
            .manifest
            .inputs
            .iter()
            .map(|input| {
                // The runner validates pinning, but a stage must not
                // silently record an unpinned reference either.
                let source_sha256 = input.source_sha256.clone().ok_or_else(|| {
                    StageError::new(
                        StageKind::Dataset,
                        format!("input {} has no pinned source hash", input.input_id),
                    )
                })?;
                if let Some(a) = ctx.manifest.input_artifact(&input.input_id) {
                    declared_lineage.push(a);
                }
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
            .collect::<Result<_, StageError>>()?;

        let mut writer =
            DatasetWriter::create(&analysis, inputs.clone(), ctx.config.clone()).map_err(err)?;
        let prov = |input_id: &str, parents: &[al_core::ArtifactId]| Provenance {
            input_id: input_id.to_string(),
            algorithm_id: "dataset-materialize".into(),
            algorithm_version: SCHEMA_VERSION.to_string(),
            params: serde_json::json!({}),
            alignment_id: None,
            parents: parents.iter().map(|a| a.as_str().to_string()).collect(),
        };
        writer
            .write_json(
                "inputs",
                "declared",
                &serde_json::json!({ "inputs": inputs }),
                prov("job", &declared_lineage),
            )
            .map_err(err)?;

        // Materialize decoded PCM as the dataset's decoded-audio role.
        let mut all_lineage = declared_lineage.clone();
        for input in &ctx.manifest.inputs {
            all_lineage.extend(self.materialize_pcm(input, ctx, &mut writer)?);
        }

        let manifest = writer.finish().map_err(err)?;

        Ok(vec![
            ArtifactRecord {
                artifact_id: ctx.registry.issue(),
                role: "analysis_dataset".into(),
                path: PathBuf::from("analysis"),
                produced_by: Some(StageKind::Dataset),
                inputs: all_lineage.clone(),
                params: serde_json::json!({
                    "schema_version": manifest.schema_version,
                    "artifact_count": manifest.artifacts.len(),
                }),
            },
            ArtifactRecord {
                artifact_id: ctx.registry.issue(),
                role: "dataset_manifest".into(),
                path: PathBuf::from("analysis/manifest.json"),
                produced_by: Some(StageKind::Dataset),
                inputs: all_lineage.clone(),
                params: serde_json::json!({
                    "schema_version": manifest.schema_version,
                }),
            },
            ArtifactRecord {
                artifact_id: ctx.registry.issue(),
                role: "dataset_inputs".into(),
                path: PathBuf::from("analysis/inputs/declared.json"),
                produced_by: Some(StageKind::Dataset),
                inputs: declared_lineage,
                params: serde_json::json!({
                    "input_count": inputs.len(),
                    "by_reference": true,
                }),
            },
        ])
    }
}
