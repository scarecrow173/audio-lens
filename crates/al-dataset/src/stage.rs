//! `DatasetStage`: materializes `<job_dir>/analysis/` — the canonical
//! analysis dataset — and registers one artifact record per store root.
//!
//! Declared inputs are recorded by reference (path + SHA-256, never
//! copied); decoded PCM from ingest is materialized as WAV plus a JSON
//! metadata sibling under `analysis/inputs/` — the dataset's decoded-
//! audio role. Locate results become the `locate` role. Upstream
//! roles without producers stay `not_produced`.

use std::fs;
use std::path::PathBuf;

use al_core::{StageKind, StageStatus};
use al_job::fsutil::checked_job_path;
use al_job::manifest::ArtifactRecord;
use al_job::stage::{Stage, StageContext, StageError};
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

/// WAV encoding that holds the decoded samples losslessly for a
/// given source codec. Integer sources keep integer-scale values;
/// 8-bit samples (decoded to a signed range) fit in s16. Float
/// sources stay float. The decoded PCM artifact carries
/// integer-scale f64 regardless of byte order, so the selection keys
/// on the codec family (`pcm_s16{le,be}`, `pcm_s8`, …), not on the
/// wire endianness.
fn encoding_for(codec: &str) -> Option<WavEncoding> {
    match codec {
        "pcm_u8" | "pcm_s8" | "pcm_s16le" | "pcm_s16be" => Some(WavEncoding::PcmS16),
        "pcm_s24le" | "pcm_s24be" => Some(WavEncoding::PcmS24),
        "pcm_s32le" | "pcm_s32be" => Some(WavEncoding::PcmS32),
        "pcm_f32le" | "pcm_f32be" => Some(WavEncoding::Float32),
        "pcm_f64le" | "pcm_f64be" => Some(WavEncoding::Float64),
        _ => None,
    }
}

impl DatasetStage {
    /// Materialize one input's decoded PCM as `inputs/<id>.wav` plus a
    /// `<id>.metadata.json` sibling. Partial-ingest contract: an input
    /// whose audio decode was unavailable still gets its metadata
    /// store — it is recorded as `dataset_input_unavailable` instead
    /// of `dataset_input_audio`, so the stage completes while the
    /// absence stays explicit. Returns the lineage ids and the
    /// manifest records the materialized artifacts derive from.
    fn materialize_pcm(
        &self,
        input: &al_job::job::JobInput,
        ctx: &StageContext<'_>,
        writer: &mut DatasetWriter,
    ) -> Result<(Vec<al_core::ArtifactId>, Vec<ArtifactRecord>), StageError> {
        let err = |e: std::io::Error| StageError::new(StageKind::Dataset, e.to_string());
        let fail = |m: String| StageError::new(StageKind::Dataset, m);
        let job_dir = ctx.registry.job_dir();
        let input_id = input.input_id.as_str();

        let declared = ctx
            .manifest
            .input_artifact(&input.input_id)
            .ok_or_else(|| fail(format!("no declared_input artifact for {input_id}")))?;
        let decoded = ctx
            .manifest
            .artifacts
            .iter()
            .find(|a| a.role == "decoded_pcm" && a.inputs.contains(&declared));
        let meta = ctx
            .manifest
            .artifacts
            .iter()
            .find(|a| a.role == "ingest_metadata" && a.inputs.contains(&declared))
            .ok_or_else(|| fail(format!("ingest produced no ingest_metadata for {input_id}")))?;
        let mut lineage = vec![declared.clone(), meta.artifact_id.clone()];
        let mut produced = Vec::new();
        if let Some(decoded) = decoded {
            let pcm_path = checked_job_path(job_dir, &decoded.path).map_err(err)?;
            let bytes = fs::read(&pcm_path).map_err(err)?;
            let (frames, rest) = bytes.as_chunks::<8>();
            if !rest.is_empty() {
                return Err(fail(format!(
                    "decoded PCM {} is not f64-aligned",
                    pcm_path.display()
                )));
            }
            let samples: Vec<f64> = frames.iter().map(|b| f64::from_le_bytes(*b)).collect();

            let param_u64 = |key: &str| {
                decoded
                    .params
                    .get(key)
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| fail(format!("decoded_pcm params for {input_id} lack {key}")))
            };
            let codec = decoded
                .params
                .get("codec")
                .and_then(|v| v.as_str())
                .ok_or_else(|| fail(format!("decoded_pcm params for {input_id} lack codec")))?
                .to_string();
            let channels = u16::try_from(param_u64("channels")?)
                .map_err(|_| fail(format!("channels out of range for {input_id}")))?;
            let sample_rate = u32::try_from(param_u64("sample_rate")?)
                .map_err(|_| fail(format!("sample_rate out of range for {input_id}")))?;
            let encoding = encoding_for(&codec)
                .ok_or_else(|| fail(format!("unsupported decoded codec {codec}")))?;

            lineage.push(decoded.artifact_id.clone());
            writer
                .write_audio(
                    "inputs",
                    input_id,
                    encoding,
                    channels,
                    sample_rate,
                    &samples,
                    Provenance {
                        input_id: input_id.into(),
                        algorithm_id: "dataset-materialize".into(),
                        algorithm_version: SCHEMA_VERSION.to_string(),
                        params: decoded.params.clone(),
                        alignment_id: None,
                        parents: lineage.iter().map(|a| a.as_str().to_string()).collect(),
                    },
                )
                .map_err(err)?;
            produced.push(ArtifactRecord {
                artifact_id: ctx.registry.issue(),
                role: "dataset_input_audio".into(),
                path: PathBuf::from("analysis/inputs").join(format!("{input_id}.wav")),
                produced_by: Some(StageKind::Dataset),
                inputs: vec![decoded.artifact_id.clone()],
                params: serde_json::json!({
                    "input_id": input_id,
                    "codec": codec,
                    "channels": channels,
                    "sample_rate": sample_rate,
                }),
            });
        }

        let meta_path = checked_job_path(job_dir, &meta.path).map_err(err)?;
        let meta_value: serde_json::Value =
            serde_json::from_slice(&fs::read(&meta_path).map_err(err)?)
                .map_err(|e| fail(format!("ingest metadata {}: {e}", meta_path.display())))?;
        writer
            .write_json(
                "inputs",
                &format!("{input_id}.metadata"),
                &meta_value,
                Provenance {
                    input_id: input_id.into(),
                    algorithm_id: "dataset-materialize".into(),
                    algorithm_version: SCHEMA_VERSION.to_string(),
                    params: serde_json::json!({}),
                    alignment_id: None,
                    parents: vec![meta.artifact_id.as_str().to_string()],
                },
            )
            .map_err(err)?;
        produced.push(ArtifactRecord {
            artifact_id: ctx.registry.issue(),
            // 一つの metadata パスに一つの artifact identity を割り当てる。
            role: if decoded.is_some() { "dataset_input_metadata" } else { "dataset_input_unavailable" }.into(),
            path: PathBuf::from("analysis/inputs").join(format!("{input_id}.metadata.json")),
            produced_by: Some(StageKind::Dataset),
            inputs: vec![declared, meta.artifact_id.clone()],
            params: serde_json::json!({ "input_id": input_id, "audio_available": decoded.is_some() }),
        });
        Ok((lineage, produced))
    }

    /// Copy each input's ingest-side video artifacts
    /// (`video_features` + `video_thumbs`) into the `inputs` role so
    /// the dataset alone carries what the decoded `vids` stream
    /// observed — the export package and report never touch job
    /// scratch artifacts.
    fn materialize_video(
        &self,
        ctx: &StageContext<'_>,
        writer: &mut DatasetWriter,
    ) -> Result<Vec<al_core::ArtifactId>, StageError> {
        let err = |e: std::io::Error| StageError::new(StageKind::Dataset, e.to_string());
        let job_dir = ctx.registry.job_dir();
        let mut lineage = Vec::new();
        for artifact in ctx
            .manifest
            .artifacts
            .iter()
            .filter(|a| matches!(a.role.as_str(), "video_features" | "video_thumbs"))
        {
            let text = fs::read_to_string(checked_job_path(job_dir, &artifact.path).map_err(err)?)
                .map_err(err)?;
            let body: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
                StageError::new(
                    StageKind::Dataset,
                    format!(
                        "video artifact {} is not valid JSON: {e}",
                        artifact.path.display()
                    ),
                )
            })?;
            let input_id = body["input_id"].as_str().unwrap_or("unknown").to_string();
            // `inputs/<id>.video_features.json` — keep the artifact
            // basename so the dataset path states its origin.
            let name = artifact
                .path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("{input_id}.video.json"));
            let name = format!("{input_id}.{}", name.trim_end_matches(".json"));
            let mut parents: Vec<String> = vec![artifact.artifact_id.as_str().to_string()];
            parents.extend(artifact.inputs.iter().map(|a| a.as_str().to_string()));
            writer
                .write_json(
                    "inputs",
                    &name,
                    &body,
                    Provenance {
                        input_id: input_id.clone(),
                        algorithm_id: "dataset-materialize".into(),
                        algorithm_version: SCHEMA_VERSION.to_string(),
                        params: serde_json::json!({"source": artifact.role}),
                        alignment_id: None,
                        parents,
                    },
                )
                .map_err(err)?;
            lineage.push(artifact.artifact_id.clone());
        }
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

        // Materialize decoded PCM as the dataset's decoded-audio role
        // (WAV plus a JSON metadata sibling per input).
        let mut all_lineage = declared_lineage.clone();
        let mut produced = Vec::new();
        for input in &ctx.manifest.inputs {
            let span = tracing::info_span!(
                "dataset::write",
                input_id = %input.input_id,
                len = tracing::field::Empty
            );
            let (lineage, records) =
                span.in_scope(|| self.materialize_pcm(input, ctx, &mut writer))?;
            span.record("len", records.len());
            all_lineage.extend(lineage);
            produced.extend(records);
        }

        // Materialize video observations (features + preview
        // thumbnails) so the dataset carries everything the decoded
        // `vids` stream produced.
        all_lineage.extend(self.materialize_video(ctx, &mut writer)?);

        // Materialize locate results so the dataset alone explains
        // every alignment the pipeline used.
        all_lineage.extend(self.materialize_locate(ctx, &mut writer)?);

        let manifest = writer.finish().map_err(err)?;

        let mut records = vec![
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
        ];
        records.extend(produced);
        Ok(records)
    }
}

#[cfg(test)]
mod codec_tests {
    use super::*;
    #[test]
    fn decoded_codec_names_are_exact() {
        for codec in ["s16", "pcm_s16", "pcm_s16evil", "pcm_f32be_extra", "s24le"] {
            assert!(encoding_for(codec).is_none(), "{codec}");
        }
        for codec in [
            "pcm_u8",
            "pcm_s8",
            "pcm_s16le",
            "pcm_s16be",
            "pcm_s24le",
            "pcm_s24be",
            "pcm_s32le",
            "pcm_s32be",
            "pcm_f32le",
            "pcm_f32be",
            "pcm_f64le",
            "pcm_f64be",
        ] {
            assert!(encoding_for(codec).is_some(), "{codec}");
        }
    }
}
