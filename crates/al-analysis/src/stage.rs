//! Pipeline stages: `CheapAnalysisStage` persists every decoded
//! input's cheap observations into the Analysis Dataset.

use std::io;
use std::path::PathBuf;

use al_core::{StageKind, Validity};
use al_dataset::types::{FeatureRecord, ResolutionClass};
use al_dataset::writer::DatasetWriter;
use al_job::fsutil::checked_job_path;
use al_job::manifest::ArtifactRecord;
use al_job::stage::{Stage, StageContext, StageError};
use al_locate::decode::decode_input;
use al_locate::result::LocateResult;
use al_media::wav::WavEncoding;

use crate::cheap::{self, CheapConfig};
use crate::reference::{self, Aligned, ReferenceConfig};
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

/// Reference-aligned comparison over accepted locate alignments.
#[derive(Debug, Default)]
pub struct ReferenceAnalysisStage;

impl ReferenceAnalysisStage {
    /// Create the stage.
    pub fn new() -> Self {
        Self
    }
}

impl Stage for ReferenceAnalysisStage {
    fn kind(&self) -> StageKind {
        StageKind::ReferenceAnalysis
    }

    /// Needs decoded inputs, locate results, and the cheap-stage
    /// dataset (intermediates are re-derived cheaply here; the
    /// dataset identity check still applies).
    fn depends_on(&self) -> Vec<StageKind> {
        vec![
            StageKind::Ingest,
            StageKind::Locate,
            StageKind::CheapAnalysis,
        ]
    }

    fn run(&mut self, ctx: &StageContext<'_>) -> Result<Vec<ArtifactRecord>, StageError> {
        let err = |e: io::Error| StageError::new(StageKind::ReferenceAnalysis, e.to_string());
        let cfg = ReferenceConfig::from_job(ctx.config);
        let mut writer = open_analysis_dataset(ctx).map_err(err)?;
        let job_dir = ctx.registry.job_dir();
        let mut records = Vec::new();

        // Every locate_result artifact defines a (source, capture)
        // pair. Pairs without one were already decided as nothing to
        // do by the locate stage.
        for artifact in ctx
            .manifest
            .artifacts
            .iter()
            .filter(|a| a.role == "locate_result")
        {
            let text =
                std::fs::read_to_string(checked_job_path(job_dir, &artifact.path).map_err(err)?)
                    .map_err(err)?;
            let body: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
                StageError::new(
                    StageKind::ReferenceAnalysis,
                    format!("locate result {} unreadable: {e}", artifact.path.display()),
                )
            })?;
            let result: LocateResult =
                serde_json::from_value(body["result"].clone()).map_err(|e| {
                    StageError::new(
                        StageKind::ReferenceAnalysis,
                        format!("locate result {} malformed: {e}", artifact.path.display()),
                    )
                })?;
            let pair = format!("{}__{}", result.source_input_id, result.capture_input_id);
            let feature_path = format!("features/reference/{pair}");

            let (src_in, cap_in) = match (
                ctx.manifest
                    .inputs
                    .iter()
                    .find(|i| i.input_id.to_string() == result.source_input_id),
                ctx.manifest
                    .inputs
                    .iter()
                    .find(|i| i.input_id.to_string() == result.capture_input_id),
            ) {
                (Some(s), Some(c)) => (s, c),
                _ => continue,
            };
            let src_dec = decode_input(src_in, ctx)
                .map_err(|e| StageError::new(StageKind::ReferenceAnalysis, e.to_string()))?;
            let cap_dec = decode_input(cap_in, ctx)
                .map_err(|e| StageError::new(StageKind::ReferenceAnalysis, e.to_string()))?;
            let (Some(src_dec), Some(cap_dec)) = (src_dec, cap_dec) else {
                continue;
            };

            let upstream = if result.accepted {
                Validity::Valid
            } else {
                Validity::AmbiguousAlignment
            };
            let aligned = if result.accepted {
                let s_start = result.source_range.start as usize;
                let c_start = result.capture_range.start as usize;
                let len = (result.source_range.end - result.source_range.start)
                    .min(result.capture_range.end - result.capture_range.start)
                    as usize;
                let src: Vec<&[f64]> = src_dec
                    .channel_samples
                    .iter()
                    .map(|c| &c[s_start..(s_start + len).min(c.len())])
                    .collect();
                let cap: Vec<&[f64]> = cap_dec
                    .channel_samples
                    .iter()
                    .map(|c| &c[c_start..(c_start + len).min(c.len())])
                    .collect();
                Aligned {
                    src,
                    cap,
                    rate: cap_dec.sample_rate,
                    upstream_confidence: result.confidence,
                }
            } else {
                Aligned {
                    src: Vec::new(),
                    cap: Vec::new(),
                    rate: cap_dec.sample_rate,
                    upstream_confidence: result.confidence,
                }
            };

            let out = reference::analyze_pair(&pair, &aligned, upstream, &cfg);
            for tensor in &out.tensors {
                write_tensor(&mut writer, tensor, &pair, &cap_dec).map_err(err)?;
            }
            for audio in &out.residual_audio {
                writer
                    .write_audio(
                        "residual",
                        &audio.name,
                        WavEncoding::Float32,
                        audio.channels,
                        audio.sample_rate,
                        &audio.samples,
                        al_dataset::provenance::Provenance {
                            input_id: result.source_input_id.clone(),
                            algorithm_id: "reference.v1".into(),
                            algorithm_version: "1".into(),
                            params: serde_json::json!({
                                "capture_input_id": result.capture_input_id,
                            }),
                            alignment_id: Some(result.alignment_id.to_string()),
                            parents: cap_dec
                                .artifact_ids
                                .iter()
                                .chain(src_dec.artifact_ids.iter())
                                .map(|a| a.as_str().to_string())
                                .collect(),
                        },
                    )
                    .map_err(err)?;
            }
            writer
                .write_json(
                    "features/reference",
                    &pair,
                    &serde_json::json!({"records": out.records}),
                    al_dataset::provenance::Provenance {
                        input_id: result.source_input_id.clone(),
                        algorithm_id: "reference.v1".into(),
                        algorithm_version: "1".into(),
                        params: serde_json::json!({}),
                        alignment_id: Some(result.alignment_id.to_string()),
                        parents: vec![artifact.artifact_id.as_str().to_string()],
                    },
                )
                .map_err(err)?;
            records.push(ArtifactRecord {
                artifact_id: ctx.registry.issue(),
                role: "reference_features".into(),
                path: PathBuf::from(format!("analysis/{feature_path}.json")),
                produced_by: Some(StageKind::ReferenceAnalysis),
                inputs: vec![artifact.artifact_id.clone()],
                params: serde_json::json!({}),
            });
        }

        writer.finish().map_err(err)?;
        Ok(records)
    }
}

/// Split a tensor's `intermediate/...`, `features/cheap/...`, or
/// `residual/...` path into role/name and persist it as one chunk.
fn write_tensor(
    writer: &mut DatasetWriter,
    tensor: &cheap::NamedTensor,
    input_id: &str,
    dec: &al_locate::decode::DecodedInput,
) -> io::Result<()> {
    let (role, name) = ["intermediate/", "features/cheap/", "residual/"]
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
