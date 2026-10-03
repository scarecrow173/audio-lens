//! `DatasetStage`: materializes `<job_dir>/analysis/` — the canonical
//! analysis dataset — and registers one artifact record per store root.
//!
//! Declared inputs stay references (path + SHA-256, never copied) while
//! ingest-produced `decoded_pcm` / `ingest_metadata` artifacts are
//! materialized under `analysis/inputs/` as WAV audio plus a JSON
//! metadata sibling. Until upstream analysis stages land, every other
//! role stays `not_produced` rather than pretending content exists.

use std::fs;
use std::path::{Path, PathBuf};

use al_core::StageKind;
use al_job::manifest::ArtifactRecord;
use al_job::registry::ArtifactRegistry;
use al_job::stage::{Stage, StageContext, StageError};
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

/// WAV encoding that holds `codec`'s decoded samples losslessly.
/// Integer sources keep integer-scale values; `pcm_u8` (decoded to a
/// signed range) fits in s16. Float sources stay float.
fn encoding_for(codec: &str) -> Option<WavEncoding> {
    Some(match codec {
        "pcm_u8" | "pcm_s16le" => WavEncoding::PcmS16,
        "pcm_s24le" => WavEncoding::PcmS24,
        "pcm_s32le" => WavEncoding::PcmS32,
        "pcm_f32le" => WavEncoding::Float32,
        "pcm_f64le" => WavEncoding::Float64,
        _ => return None,
    })
}

/// Find the ingest artifact whose lineage points at `declared`.
fn artifact_for<'a, 'm>(
    hits: &'a [(&'m ArtifactRecord, PathBuf)],
    declared: &al_core::ArtifactId,
) -> Option<(&'m ArtifactRecord, &'a Path)> {
    hits.iter()
        .find(|(record, _)| record.inputs.contains(declared))
        .map(|(record, path)| (*record, path.as_path()))
}

fn materialize_input(
    input_id: &str,
    pcm: &ArtifactRecord,
    pcm_path: &Path,
    meta: &ArtifactRecord,
    meta_path: &Path,
    writer: &mut DatasetWriter,
    registry: &ArtifactRegistry,
) -> Result<Vec<ArtifactRecord>, StageError> {
    let fail = |m: String| StageError::new(StageKind::Dataset, m);

    let pcm_bytes = fs::read(pcm_path).map_err(|e| {
        fail(format!(
            "cannot read decoded PCM {}: {e}",
            pcm_path.display()
        ))
    })?;
    let (frames, rest) = pcm_bytes.as_chunks::<8>();
    if !rest.is_empty() {
        return Err(fail(format!(
            "decoded PCM {} is not f64-aligned",
            pcm_path.display()
        )));
    }
    let samples: Vec<f64> = frames.iter().map(|b| f64::from_le_bytes(*b)).collect();

    let param_u64 = |key: &str| {
        pcm.params
            .get(key)
            .and_then(|v| v.as_u64())
            .ok_or_else(|| fail(format!("decoded_pcm params for {input_id} lack {key}")))
    };
    let codec = pcm
        .params
        .get("codec")
        .and_then(|v| v.as_str())
        .ok_or_else(|| fail(format!("decoded_pcm params for {input_id} lack codec")))?;
    let channels =
        u16::try_from(param_u64("channels")?).map_err(|_| fail("channels out of range".into()))?;
    let sample_rate = u32::try_from(param_u64("sample_rate")?)
        .map_err(|_| fail("sample_rate out of range".into()))?;
    let encoding =
        encoding_for(codec).ok_or_else(|| fail(format!("unsupported decoded codec {codec}")))?;

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
                params: pcm.params.clone(),
                alignment_id: None,
                parents: vec![pcm.artifact_id.to_string(), meta.artifact_id.to_string()],
            },
        )
        .map_err(|e| StageError::new(StageKind::Dataset, e.to_string()))?;

    let meta_bytes = fs::read(meta_path).map_err(|e| {
        fail(format!(
            "cannot read ingest metadata {}: {e}",
            meta_path.display()
        ))
    })?;
    let meta_value: serde_json::Value = serde_json::from_slice(&meta_bytes)
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
                parents: vec![meta.artifact_id.to_string()],
            },
        )
        .map_err(|e| StageError::new(StageKind::Dataset, e.to_string()))?;

    Ok(vec![
        ArtifactRecord {
            artifact_id: registry.issue(),
            role: "dataset_input_audio".into(),
            path: PathBuf::from("analysis/inputs").join(format!("{input_id}.wav")),
            produced_by: Some(StageKind::Dataset),
            inputs: vec![pcm.artifact_id.clone()],
            params: serde_json::json!({
                "input_id": input_id,
                "codec": codec,
                "channels": channels,
                "sample_rate": sample_rate,
            }),
        },
        ArtifactRecord {
            artifact_id: registry.issue(),
            role: "dataset_input_metadata".into(),
            path: PathBuf::from("analysis/inputs").join(format!("{input_id}.metadata.json")),
            produced_by: Some(StageKind::Dataset),
            inputs: vec![meta.artifact_id.clone()],
            params: serde_json::json!({ "input_id": input_id }),
        },
    ])
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
        let fail = |m: String| StageError::new(StageKind::Dataset, m);
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
        writer
            .write_json(
                "inputs",
                "declared",
                &serde_json::json!({ "inputs": inputs }),
                Provenance {
                    input_id: "job".into(),
                    algorithm_id: "dataset-materialize".into(),
                    algorithm_version: SCHEMA_VERSION.to_string(),
                    params: serde_json::json!({}),
                    alignment_id: None,
                    parents: vec![],
                },
            )
            .map_err(err)?;

        // Ingest-produced artifacts materialize into the dataset so the
        // canonical dataset is self-contained: decoded PCM becomes
        // `inputs/<input_id>.wav`, ingest metadata a JSON sibling.
        // Ingest links both to the declared-input artifact, so each
        // input's pair is found through that lineage.
        let decoded = ctx.registry.resolve_by_role(ctx.manifest, "decoded_pcm");
        let ingest_meta = ctx
            .registry
            .resolve_by_role(ctx.manifest, "ingest_metadata");
        let mut produced = Vec::new();
        for input in &ctx.manifest.inputs {
            let input_id = input.input_id.as_str();
            let declared = ctx
                .manifest
                .input_artifact(&input.input_id)
                .ok_or_else(|| fail(format!("no declared_input artifact for {input_id}")))?;
            let (pcm, pcm_path) = artifact_for(&decoded, &declared)
                .ok_or_else(|| fail(format!("ingest produced no decoded_pcm for {input_id}")))?;
            let (meta, meta_path) = artifact_for(&ingest_meta, &declared).ok_or_else(|| {
                fail(format!("ingest produced no ingest_metadata for {input_id}"))
            })?;
            produced.extend(materialize_input(
                input_id,
                pcm,
                pcm_path,
                meta,
                meta_path,
                &mut writer,
                ctx.registry,
            )?);
        }

        let manifest = writer.finish().map_err(err)?;

        let mut records = vec![
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
        ];
        records.extend(produced);
        Ok(records)
    }
}
