//! Pipeline stages: `CheapAnalysisStage` persists every decoded
//! input's cheap observations into the Analysis Dataset.

use std::io;
use std::path::PathBuf;

use al_core::{StageKind, Validity};
use al_dataset::types::{FeatureRecord, ResolutionClass};
use al_dataset::writer::DatasetWriter;
use al_job::manifest::ArtifactRecord;
use al_job::stage::{Stage, StageContext, StageError};
use al_locate::decode::decode_input;

use crate::cheap::{self, CheapConfig};
use crate::session::open_analysis_dataset;

/// Always-on cheap observations over every decoded input.
#[derive(Debug, Default)]
pub struct CheapAnalysisStage;

impl CheapAnalysisStage {
    /// Create the stage.
    pub fn new() -> Self {
        Self
    }
}

impl Stage for CheapAnalysisStage {
    fn kind(&self) -> StageKind {
        StageKind::CheapAnalysis
    }

    /// Capture-only observations need no source and no alignment.
    fn depends_on(&self) -> Vec<StageKind> {
        vec![StageKind::Ingest]
    }

    fn run(&mut self, ctx: &StageContext<'_>) -> Result<Vec<ArtifactRecord>, StageError> {
        let err = |e: io::Error| StageError::new(StageKind::CheapAnalysis, e.to_string());
        let cfg = CheapConfig::from_job(ctx.config);
        let mut writer = open_analysis_dataset(ctx).map_err(err)?;
        let mut records = Vec::new();

        for input in &ctx.manifest.inputs {
            let decoded = decode_input(input, ctx)
                .map_err(|e| StageError::new(StageKind::CheapAnalysis, e.to_string()))?;
            let feature_path = format!("features/cheap/{}", input.input_id);
            let Some(dec) = decoded else {
                // Never ingested: record the failure as an invalid
                // observation, not a stage error.
                let rec = FeatureRecord {
                    feature_id: "input.decode".into(),
                    value: al_dataset::types::Value::Scalar(f64::NAN),
                    unit: "none".into(),
                    time_range: None,
                    freq_range: None,
                    channels: None,
                    resolution: None,
                    resolution_class: ResolutionClass::Segment,
                    target: None,
                    reference: None,
                    delta: None,
                    confidence: al_core::Confidence::Low,
                    validity: Validity::DecodeInputUnavailable,
                    invalid_reason: Some("no decoded_pcm artifact".into()),
                    support_count: 0,
                    algorithm_id: "cheap.v1".into(),
                    algorithm_version: "1".into(),
                    params: serde_json::json!({}),
                    input_id: input.input_id.to_string(),
                    alignment_id: None,
                    lineage: Vec::new(),
                };
                writer
                    .write_json(
                        "features/cheap",
                        &input.input_id.to_string(),
                        &serde_json::json!({"records": [rec]}),
                        prov(input, ctx),
                    )
                    .map_err(err)?;
                records.push(artifact(ctx, &feature_path, input));
                continue;
            };

            let out = cheap::analyze_input(&dec, &cfg);
            for tensor in &out.tensors {
                write_tensor(&mut writer, tensor, &dec.input_id, &dec).map_err(err)?;
            }
            writer
                .write_json(
                    "features/cheap",
                    &dec.input_id,
                    &serde_json::json!({"records": out.records}),
                    prov_dec(&dec),
                )
                .map_err(err)?;
            records.push(artifact(ctx, &feature_path, input));
        }

        writer.finish().map_err(err)?;
        Ok(records)
    }
}

/// Split a tensor's `intermediate/...` or `features/cheap/...` path
/// into role/name and persist it as one chunk.
fn write_tensor(
    writer: &mut DatasetWriter,
    tensor: &cheap::NamedTensor,
    input_id: &str,
    dec: &al_locate::decode::DecodedInput,
) -> io::Result<()> {
    let (role, name) = ["intermediate/", "features/cheap/"]
        .iter()
        .find_map(|prefix| {
            tensor
                .path
                .strip_prefix(prefix)
                .map(|name| (prefix.trim_end_matches('/'), name))
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("tensor path outside known roles: {}", tensor.path),
            )
        })?;
    if tensor.data.is_empty() {
        return Ok(());
    }
    let chunk = al_dataset::writer::TensorChunk {
        coords: vec![0, 0],
        data: tensor.data.clone(),
    };
    // Single whole-tensor chunk — the data covers the full extent.
    writer.write_tensor(
        role,
        name,
        &tensor.shape,
        &tensor.shape,
        &[chunk],
        al_dataset::provenance::Provenance {
            input_id: input_id.to_string(),
            algorithm_id: "cheap.v1".into(),
            algorithm_version: "1".into(),
            params: serde_json::json!({}),
            alignment_id: None,
            parents: dec
                .artifact_ids
                .iter()
                .map(|a| a.as_str().to_string())
                .collect(),
        },
    )
}

fn prov(
    input: &al_job::job::JobInput,
    ctx: &StageContext<'_>,
) -> al_dataset::provenance::Provenance {
    let parents: Vec<String> = ctx
        .manifest
        .input_artifact(&input.input_id)
        .iter()
        .map(|a| a.as_str().to_string())
        .collect();
    al_dataset::provenance::Provenance {
        input_id: input.input_id.to_string(),
        algorithm_id: "cheap.v1".into(),
        algorithm_version: "1".into(),
        params: serde_json::json!({}),
        alignment_id: None,
        parents,
    }
}

fn prov_dec(dec: &al_locate::decode::DecodedInput) -> al_dataset::provenance::Provenance {
    al_dataset::provenance::Provenance {
        input_id: dec.input_id.clone(),
        algorithm_id: "cheap.v1".into(),
        algorithm_version: "1".into(),
        params: serde_json::json!({}),
        alignment_id: None,
        parents: dec
            .artifact_ids
            .iter()
            .map(|a| a.as_str().to_string())
            .collect(),
    }
}

fn artifact(ctx: &StageContext<'_>, path: &str, input: &al_job::job::JobInput) -> ArtifactRecord {
    let inputs: Vec<al_core::ArtifactId> = ctx
        .manifest
        .input_artifact(&input.input_id)
        .into_iter()
        .collect();
    ArtifactRecord {
        artifact_id: ctx.registry.issue(),
        role: "cheap_features".into(),
        path: PathBuf::from(format!("analysis/{path}.json")),
        produced_by: Some(StageKind::CheapAnalysis),
        inputs,
        params: serde_json::json!({}),
    }
}
