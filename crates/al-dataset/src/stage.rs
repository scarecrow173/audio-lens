//! `DatasetStage`: materializes `<job_dir>/analysis/` — the canonical
//! analysis dataset — and registers one artifact record per store root.
//!
//! Declared inputs are recorded by reference (path + SHA-256, never
//! copied); decoded PCM from ingest is materialized as WAV under
//! `analysis/inputs/` — the dataset's decoded-audio role. Upstream
//! roles without producers stay `not_produced`.

use std::fs;
use std::path::PathBuf;

use al_core::{StageKind, StageStatus};
use al_job::fsutil::checked_job_path;
use al_job::manifest::ArtifactRecord;
use al_job::stage::{Stage, StageContext, StageError};
use al_media::metadata::SampleFormat;
use al_media::wav::WavEncoding;

use crate::manifest::{DatasetManifest, SCHEMA_VERSION};
use crate::provenance::{Provenance, input_references};
use crate::table::{Cell, Column, DataType};
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
        let bytes =
            fs::read(checked_job_path(job_dir, &decoded.path).map_err(err)?).map_err(err)?;
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
            .and_then(|m| {
                checked_job_path(job_dir, &m.path)
                    .ok()
                    .and_then(|p| fs::read_to_string(p).ok())
            })
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

    /// Copy every `locate_result` artifact into the `locate` role as
    /// JSON plus a `locate/summary` table of all alignments.
    fn materialize_locate(
        &self,
        ctx: &StageContext<'_>,
        writer: &mut DatasetWriter,
    ) -> Result<Vec<al_core::ArtifactId>, StageError> {
        let err = |e: std::io::Error| StageError::new(StageKind::Dataset, e.to_string());
        let job_dir = ctx.registry.job_dir();
        let mut lineage = Vec::new();
        let mut rows = Vec::new();
        for artifact in ctx
            .manifest
            .artifacts
            .iter()
            .filter(|a| a.role == "locate_result")
        {
            let text = fs::read_to_string(checked_job_path(job_dir, &artifact.path).map_err(err)?)
                .map_err(err)?;
            let body: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
                StageError::new(
                    StageKind::Dataset,
                    format!(
                        "locate result {} is not valid JSON: {e}",
                        artifact.path.display()
                    ),
                )
            })?;
            let result = &body["result"];
            let src = result["source_input_id"].as_str().unwrap_or("unknown");
            let cap = result["capture_input_id"].as_str().unwrap_or("unknown");
            let mut parents: Vec<String> = vec![artifact.artifact_id.as_str().to_string()];
            parents.extend(artifact.inputs.iter().map(|a| a.as_str().to_string()));
            writer
                .write_json(
                    "locate",
                    &format!("{src}/{cap}"),
                    &body,
                    Provenance {
                        input_id: src.to_string(),
                        algorithm_id: "dataset-materialize".into(),
                        algorithm_version: SCHEMA_VERSION.to_string(),
                        params: serde_json::json!({"source": "locate_result"}),
                        alignment_id: result["alignment_id"].as_str().map(str::to_owned),
                        parents: parents.clone(),
                    },
                )
                .map_err(err)?;
            rows.push(vec![
                Cell::Str(src.to_string()),
                Cell::Str(cap.to_string()),
                Cell::Str(result["alignment_id"].as_str().unwrap_or("").to_string()),
                Cell::Str(if result["accepted"].as_bool() == Some(true) {
                    "accepted".into()
                } else {
                    "rejected".into()
                }),
                Cell::Str(result["validity"].as_str().unwrap_or("unknown").to_string()),
                Cell::Str(
                    result["confidence"]
                        .as_str()
                        .unwrap_or("unknown")
                        .to_string(),
                ),
                Cell::I64(result["agreeing_methods"].as_u64().unwrap_or(0) as i64),
                Cell::I64(result["capture_range"]["start"].as_u64().unwrap_or(0) as i64),
                Cell::I64(result["capture_range"]["end"].as_u64().unwrap_or(0) as i64),
            ]);
            lineage.push(artifact.artifact_id.clone());
        }
        if !rows.is_empty() {
            let schema = [
                "source_input_id",
                "capture_input_id",
                "alignment_id",
                "status",
                "validity",
                "confidence",
                "agreeing_methods",
                "capture_start",
                "capture_end",
            ]
            .iter()
            .map(|name| Column {
                name: name.to_string(),
                data_type: match *name {
                    "agreeing_methods" | "capture_start" | "capture_end" => DataType::I64,
                    _ => DataType::Utf8,
                },
            })
            .collect::<Vec<_>>();
            let parents: Vec<String> = lineage.iter().map(|a| a.as_str().to_string()).collect();
            writer
                .write_table(
                    "locate",
                    "summary",
                    &schema,
                    &rows,
                    Provenance {
                        input_id: "job".into(),
                        algorithm_id: "dataset-materialize".into(),
                        algorithm_version: SCHEMA_VERSION.to_string(),
                        params: serde_json::json!({"source": "locate_result"}),
                        alignment_id: None,
                        parents,
                    },
                )
                .map_err(err)?;
        }
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

        let inputs = input_references(ctx.manifest).map_err(err)?;

        // Append mode: analysis stages may already have created the
        // dataset. A missing/corrupt manifest is safe to regenerate
        // only when no upstream analysis stage has completed — wiping
        // their output would destroy committed stage results, so that
        // case is an explicit failure instead.
        if analysis.exists() && DatasetManifest::load(&analysis).is_err() {
            let upstream_done = ctx.manifest.stages.iter().any(|s| {
                matches!(
                    s.stage,
                    StageKind::CheapAnalysis
                        | StageKind::ReferenceAnalysis
                        | StageKind::ExpensiveAnalysis
                ) && s.status == StageStatus::Completed
            });
            if upstream_done {
                return Err(StageError::new(
                    StageKind::Dataset,
                    "analysis/ exists but its manifest is missing or corrupt \
                     while analysis stages already completed — refusing to \
                     destroy their output; repair the manifest or remove \
                     analysis/ manually",
                ));
            }
            fs::remove_dir_all(&analysis).map_err(err)?;
        }

        let mut declared_lineage = Vec::new();
        for input in &ctx.manifest.inputs {
            if let Some(a) = ctx.manifest.input_artifact(&input.input_id) {
                declared_lineage.push(a);
            }
        }

        let mut writer =
            DatasetWriter::open_or_create(&analysis, inputs.clone(), ctx.config.clone())
                .map_err(err)?;
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

        // Materialize locate results so the dataset alone explains
        // every alignment the pipeline used.
        all_lineage.extend(self.materialize_locate(ctx, &mut writer)?);

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
