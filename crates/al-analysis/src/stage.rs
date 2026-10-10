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
use al_locate::decode::{decode_input, read_alignment_pcm};
use al_locate::result::LocateResult;
use al_media::wav::WavEncoding;

use crate::avsync::{self, AvSyncConfig};
use crate::cheap::{self, CheapConfig};
use crate::events::{self, EventConfig};
use crate::expensive::{self, ExpensiveConfig};
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
        let av_cfg = AvSyncConfig::from_job(ctx.config);
        let chunk_bytes = tensor_chunk_bytes(ctx.config);
        let mut writer = open_analysis_dataset(ctx).map_err(err)?;
        let mut records = Vec::new();

        for input in &ctx.manifest.inputs {
            let decoded = decode_input(input, ctx)
                .map_err(|e| StageError::new(StageKind::CheapAnalysis, e.to_string()))?;
            let feature_path = format!("features/cheap/{}", input.input_id);
            // A/V sync consumes the ingest-side video_features artifact
            // independently of audio decode success — missing evidence
            // becomes an explicit invalid record, never silence.
            let video = load_video_features(input, ctx)?;
            let mut lineage: Vec<String> = ctx
                .manifest
                .input_artifact(&input.input_id)
                .iter()
                .map(|a| a.as_str().to_string())
                .collect();
            if let Some((_, artifact_id)) = &video {
                lineage.push(artifact_id.clone());
            }
            let video_features = video.map(|(f, _)| f);
            let audio_start_s = if decoded.is_some()
                && matches!(
                    video_features,
                    Some(al_media::VideoFeatureArtifact::Decoded(_))
                ) {
                let (origin, metadata_id) = load_audio_origin(input, ctx)?;
                lineage.push(metadata_id);
                origin
            } else {
                0.0
            };
            let video_out = crate::video::roi_records(
                &input.input_id.to_string(),
                video_features.as_ref(),
                lineage.clone(),
            );
            for tensor in &video_out.tensors {
                let mut provenance = prov(input, ctx);
                provenance.algorithm_id = "video.roi_luminance.v1".into();
                provenance.parents = lineage.clone();
                provenance.params = video_features
                    .as_ref()
                    .and_then(|v| match v {
                        al_media::video::VideoFeatureArtifact::Decoded(v) => {
                            serde_json::to_value(&v.params).ok()
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| serde_json::json!({}));
                write_tensor(&mut writer, tensor, provenance, chunk_bytes).map_err(err)?;
            }
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
                let mut all = vec![rec];
                all.extend(video_out.records);
                all.extend(avsync::sync_records(
                    &input.input_id.to_string(),
                    None,
                    video_features.as_ref(),
                    audio_start_s,
                    &av_cfg,
                    lineage,
                ));
                writer
                    .write_json(
                        "features/cheap",
                        &input.input_id.to_string(),
                        &serde_json::json!({"records": all}),
                        prov(input, ctx),
                    )
                    .map_err(err)?;
                records.push(artifact(ctx, &feature_path, input));
                continue;
            };

            let mut out = cheap::analyze_input(&dec, &cfg);
            out.records.extend(video_out.records);
            // A/V sync: the persisted onset_times tensor is the audio
            // event series; video events come from ingest. Both are
            // converted to a shared audio PCM origin inside `sync_records`.
            let onset_times: Vec<f64> = out
                .tensors
                .iter()
                .find(|t| t.path == format!("features/cheap/{}/onset_times", dec.input_id))
                .map(|t| t.data.clone())
                .unwrap_or_default();
            out.records.extend(avsync::sync_records(
                &dec.input_id,
                Some(&onset_times),
                video_features.as_ref(),
                audio_start_s,
                &av_cfg,
                lineage,
            ));
            for tensor in &out.tensors {
                write_tensor(&mut writer, tensor, prov_dec(&dec), chunk_bytes).map_err(err)?;
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
        let chunk_bytes = tensor_chunk_bytes(ctx.config);
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
            let result = read_locate_result(StageKind::ReferenceAnalysis, job_dir, artifact)?;
            let pair = format!("{}__{}", result.source_input_id, result.capture_input_id);
            let feature_path = format!("features/reference/{pair}");

            let pcm = read_alignment_pcm(&result, ctx)
                .map_err(|e| StageError::new(StageKind::ReferenceAnalysis, e.to_string()))?;
            let mut lineage = vec![artifact.artifact_id.clone()];
            if let Some((src, cap)) = &pcm {
                lineage.push(src.record.artifact_id.clone());
                lineage.push(cap.record.artifact_id.clone());
            }
            let upstream = if result.accepted {
                Validity::Valid
            } else {
                Validity::AmbiguousAlignment
            };
            let aligned = if result.accepted {
                let (src, cap) = pcm.as_ref().ok_or_else(|| {
                    StageError::new(
                        StageKind::ReferenceAnalysis,
                        "accepted alignment has no PCM",
                    )
                })?;
                let s_start = result.normalized_source_range.start as usize;
                let c_start = result.capture_range.start as usize;
                let len = (result.normalized_source_range.end
                    - result.normalized_source_range.start)
                    .min(result.capture_range.end - result.capture_range.start)
                    as usize;
                Aligned {
                    src: src
                        .decoded
                        .channel_samples
                        .iter()
                        .map(|c| &c[s_start..s_start + len])
                        .collect(),
                    cap: cap
                        .decoded
                        .channel_samples
                        .iter()
                        .map(|c| &c[c_start..c_start + len])
                        .collect(),
                    rate: cap.decoded.sample_rate,
                    upstream_confidence: result.confidence,
                }
            } else {
                Aligned {
                    src: Vec::new(),
                    cap: Vec::new(),
                    rate: pcm
                        .as_ref()
                        .map_or(48000, |(_, cap)| cap.decoded.sample_rate),
                    upstream_confidence: result.confidence,
                }
            };

            let out =
                reference::analyze_pair(&pair, &result.alignment_id, &aligned, upstream, &cfg);
            let tensor_prov = al_dataset::provenance::Provenance {
                input_id: result.source_input_id.clone(),
                algorithm_id: "reference.v1".into(),
                algorithm_version: "1".into(),
                params: serde_json::json!({
                    "capture_input_id": result.capture_input_id,
                }),
                alignment_id: Some(result.alignment_id.to_string()),
                parents: lineage.iter().map(|a| a.as_str().to_string()).collect(),
            };
            for tensor in &out.tensors {
                let mut provenance = tensor_prov.clone();
                if tensor
                    .path
                    .starts_with(&format!("intermediate/{pair}/channel_"))
                {
                    provenance.algorithm_id = "reference.band_lag_ls.v1".into();
                    provenance.params = crate::band_gain::parameters(&cfg, aligned.rate);
                    provenance.params["capture_input_id"] =
                        serde_json::json!(result.capture_input_id);
                }
                write_tensor(&mut writer, tensor, provenance, chunk_bytes).map_err(err)?;
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
                            parents: lineage.iter().map(|a| a.as_str().to_string()).collect(),
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
                        parents: lineage.iter().map(|a| a.as_str().to_string()).collect(),
                    },
                )
                .map_err(err)?;
            records.push(ArtifactRecord {
                artifact_id: ctx.registry.issue(),
                role: "reference_features".into(),
                path: PathBuf::from(format!("analysis/{feature_path}.json")),
                produced_by: Some(StageKind::ReferenceAnalysis),
                inputs: lineage,
                params: serde_json::json!({}),
            });
        }

        writer.finish().map_err(err)?;
        Ok(records)
    }
}

/// Candidate-driven deep analysis over triggered regions.
#[derive(Debug, Default)]
pub struct ExpensiveAnalysisStage;

impl ExpensiveAnalysisStage {
    /// Create the stage.
    pub fn new() -> Self {
        Self
    }
}

impl Stage for ExpensiveAnalysisStage {
    fn kind(&self) -> StageKind {
        StageKind::ExpensiveAnalysis
    }

    /// Only cheap features are required to produce candidates;
    /// reference features are used when present but not required.
    fn depends_on(&self) -> Vec<StageKind> {
        vec![StageKind::Ingest, StageKind::CheapAnalysis]
    }

    fn run(&mut self, ctx: &StageContext<'_>) -> Result<Vec<ArtifactRecord>, StageError> {
        let err = |e: io::Error| StageError::new(StageKind::ExpensiveAnalysis, e.to_string());
        let e_cfg = ExpensiveConfig::from_job(ctx.config);
        let c_cfg = EventConfig::from_job(ctx.config);
        let chunk_bytes = tensor_chunk_bytes(ctx.config);
        let mut writer = open_analysis_dataset(ctx).map_err(err)?;
        let root = ctx.registry.job_dir().join("analysis");
        let reader = al_dataset::reader::DatasetReader::open(&root).map_err(err)?;
        let mut records = Vec::new();

        // 0. Decode every input once: candidate extraction needs each
        // input's segment duration and the region analysis reuses the
        // same decoded channels.
        let mut decoded: std::collections::HashMap<String, al_locate::decode::DecodedInput> =
            std::collections::HashMap::new();
        for input in &ctx.manifest.inputs {
            if let Some(dec) = decode_input(input, ctx)
                .map_err(|e| StageError::new(StageKind::ExpensiveAnalysis, e.to_string()))?
            {
                decoded.insert(input.input_id.to_string(), dec);
            }
        }
        let duration_of = |input_id: &str| -> Option<f64> {
            let dec = decoded.get(input_id)?;
            let n = dec.channel_samples.iter().map(|c| c.len()).max()?;
            if n == 0 {
                return None;
            }
            Some(n as f64 / dec.sample_rate as f64)
        };

        // 1. Extract candidate events from the cheap/reference
        // feature stores and persist them under the `events` role.
        // The table is written even when empty so `role_produced`
        // distinguishes "ran" from `not_produced`.
        let mut all_events: Vec<events::CandidateEvent> = Vec::new();
        let mut scanned: Vec<String> = Vec::new();
        for role in expensive::CANDIDATE_ROLES {
            for artifact in reader
                .artifacts(role)
                .into_iter()
                .filter(|a| a.store == al_dataset::manifest::StoreKind::Json)
            {
                let name = artifact
                    .path
                    .strip_prefix(&format!("{role}/"))
                    .unwrap_or(&artifact.path);
                let body = reader.read_json(role, name).map_err(err)?;
                let feat_records = expensive::parse_feature_records(&body).map_err(|e| {
                    StageError::new(
                        StageKind::ExpensiveAnalysis,
                        format!("feature records in {} unreadable: {e}", artifact.path),
                    )
                })?;
                // `features/cheap/<input>` maps to that input;
                // `features/reference/<src>__<cap>` events apply to the
                // capture side (the measured pipeline output). The
                // `.json` store suffix is not part of the input id.
                let stem = name.strip_suffix(".json").unwrap_or(name);
                let input_id = stem.split("__").last().unwrap_or(stem).to_string();
                all_events.extend(expensive::extract_candidates(
                    &artifact.path,
                    &input_id,
                    &feat_records,
                    &c_cfg,
                    duration_of(&input_id),
                ));
                scanned.push(artifact.path.clone());
            }
        }
        writer
            .write_table(
                "events",
                "candidates",
                &events::events_schema(),
                &events::events_to_rows(&all_events),
                al_dataset::provenance::Provenance {
                    input_id: "job".into(),
                    algorithm_id: "events.v1".into(),
                    algorithm_version: "1".into(),
                    params: serde_json::to_value(&c_cfg).unwrap_or_default(),
                    alignment_id: None,
                    parents: scanned,
                },
            )
            .map_err(err)?;
        records.push(ArtifactRecord {
            artifact_id: ctx.registry.issue(),
            role: "candidate_events".into(),
            path: PathBuf::from("analysis/events/candidates"),
            produced_by: Some(StageKind::ExpensiveAnalysis),
            inputs: Vec::new(),
            params: serde_json::json!({"event_count": all_events.len()}),
        });

        // 2. Plan triggered regions per input and deep-analyze them.
        // Assemble optional source→capture pair evidence for the
        // pair-dependent families. `locate_result` manifest artifacts
        // define the (source, capture) alignments; reference-stage
        // `intermediate/{pair}/transfer_h` tensors supply H(f) when
        // the job ran ReferenceAnalysis. Both are strictly optional —
        // see `expensive::PairEvidence` for why the DAG dependency
        // is intentionally absent. Jobs without locate/reference
        // simply get no pair evidence and the families emit
        // `pair evidence unavailable` invalid records instead.
        let job_dir = ctx.registry.job_dir();
        let mut loaded_alignments = Vec::new();
        for artifact in ctx
            .manifest
            .artifacts
            .iter()
            .filter(|a| a.role == "locate_result")
        {
            let result = read_locate_result(StageKind::ExpensiveAnalysis, job_dir, artifact)?;
            let pcm = read_alignment_pcm(&result, ctx)
                .map_err(|e| StageError::new(StageKind::ExpensiveAnalysis, e.to_string()))?;
            loaded_alignments.push((artifact, result, pcm));
        }
        let mut pairs: std::collections::HashMap<String, expensive::PairEvidence> =
            std::collections::HashMap::new();
        let mut pair_captures = std::collections::HashMap::new();
        let mut pair_lineage = std::collections::HashMap::new();
        let mut calibration_evidence = Vec::new();
        for (artifact, result, pcm) in &loaded_alignments {
            if !result.accepted || result.validity != Validity::Valid {
                continue;
            }
            let Some((_, capture)) = pcm else { continue };
            let pair = format!("{}__{}", result.source_input_id, result.capture_input_id);
            let path = format!("intermediate/{pair}/transfer_h");
            // H/coherence はこの locate の alignment から生成されたものだけを採用する。
            let current_alignment = [&path, &format!("intermediate/{pair}/coherence")]
                .iter()
                .all(|path| {
                    reader.artifacts("intermediate").into_iter().any(|a| {
                        a.path == **path
                            && a.provenance.alignment_id.as_deref()
                                == Some(result.alignment_id.as_str())
                    })
                });
            let mut h = if current_alignment {
                read_transfer_h(&reader, &pair).map_err(err)?
            } else {
                None
            };
            if let Some(values) = &mut h {
                let coherence = read_complex_tensor(&reader, &pair, "coherence").map_err(err)?;
                if let Some(coherence) = coherence.filter(|c| c.len() == values.len()) {
                    for (value, (msc, _)) in values.iter_mut().zip(coherence) {
                        if !msc.is_finite() || msc < crate::calibration::MIN_COHERENCE {
                            *value = (f64::NAN, f64::NAN);
                        }
                    }
                } else {
                    h = None;
                }
            }
            let mut lineage = vec![artifact.artifact_id.as_str().to_string()];
            if h.is_some() {
                lineage.extend([path, format!("intermediate/{pair}/coherence")]);
            }
            calibration_evidence.push(crate::calibration::TransferEvidence {
                source: result.source_input_id.clone(),
                capture: result.capture_input_id.clone(),
                rate: capture.decoded.sample_rate,
                h,
                confidence: result.confidence,
                lineage,
            });
        }
        // キャプチャ間比較は job 全体の成果物。既存のペア選択・観測は変更しない。
        let mut captures: Vec<_> = ctx
            .manifest
            .inputs
            .iter()
            .filter(|i| {
                matches!(
                    i.role,
                    al_core::InputRole::CaptureBug | al_core::InputRole::CaptureReference
                )
            })
            .collect();
        captures.sort_by_key(|i| {
            (
                i.role != al_core::InputRole::CaptureReference,
                i.input_id.as_str(),
            )
        });
        for (i, reference) in captures.iter().enumerate() {
            for target in &captures[i + 1..] {
                let (comparison, tensor) = crate::calibration::compare(
                    reference.input_id.as_str(),
                    target.input_id.as_str(),
                    &calibration_evidence,
                );
                let name = format!("{}__{}", reference.input_id, target.input_id);
                let provenance = al_dataset::provenance::Provenance {
                    input_id: "job".into(),
                    algorithm_id: "cross_capture.path_gain.v1".into(),
                    algorithm_version: "1".into(),
                    params: comparison.records[0].params.clone(),
                    alignment_id: None,
                    parents: comparison.records[0].lineage.clone(),
                };
                if let Some(tensor) = tensor {
                    write_tensor(&mut writer, &tensor, provenance.clone(), chunk_bytes)
                        .map_err(err)?;
                }
                writer
                    .write_json(
                        "features/cross_capture",
                        &name,
                        &serde_json::to_value(&comparison)
                            .map_err(io::Error::other)
                            .map_err(err)?,
                        provenance,
                    )
                    .map_err(err)?;
                records.push(ArtifactRecord {
                    artifact_id: ctx.registry.issue(),
                    role: "cross_capture_features".into(),
                    path: PathBuf::from(format!("analysis/features/cross_capture/{name}.json")),
                    produced_by: Some(StageKind::ExpensiveAnalysis),
                    inputs: loaded_alignments
                        .iter()
                        .filter(|(_, r, _)| {
                            r.capture_input_id == reference.input_id.as_str()
                                || r.capture_input_id == target.input_id.as_str()
                        })
                        .map(|(a, _, _)| a.artifact_id.clone())
                        .collect(),
                    params: serde_json::json!({"reference_capture_id": reference.input_id, "target_capture_id": target.input_id}),
                });
            }
        }
        for (artifact, result, pcm) in &loaded_alignments {
            if pairs.contains_key(&result.capture_input_id) {
                continue; // first locate result per capture wins
            }
            let pair_name = format!("{}__{}", result.source_input_id, result.capture_input_id);
            let (aligned_src, len): (Vec<&[f64]>, usize) = if result.accepted {
                let (source, capture) = pcm.as_ref().ok_or_else(|| {
                    StageError::new(
                        StageKind::ExpensiveAnalysis,
                        "accepted alignment has no PCM",
                    )
                })?;
                let s_start = result.normalized_source_range.start as usize;
                let len = (result.normalized_source_range.end
                    - result.normalized_source_range.start)
                    .min(result.capture_range.end - result.capture_range.start)
                    as usize;
                pair_captures.insert(result.capture_input_id.clone(), &capture.decoded);
                (
                    source
                        .decoded
                        .channel_samples
                        .iter()
                        .map(|c| &c[s_start..s_start + len])
                        .collect(),
                    len,
                )
            } else {
                (Vec::new(), 0)
            };
            let mut lineage = vec![artifact.artifact_id.clone()];
            if let Some((source, capture)) = pcm {
                lineage.push(source.record.artifact_id.clone());
                lineage.push(capture.record.artifact_id.clone());
            }
            pair_lineage.insert(result.capture_input_id.clone(), lineage);
            let transfer_h = read_transfer_h(&reader, &pair_name).map_err(err)?;
            pairs.insert(
                result.capture_input_id.clone(),
                expensive::PairEvidence {
                    pair: pair_name,
                    alignment_id: result.alignment_id.as_str().to_string(),
                    accepted: result.accepted,
                    validity: if result.accepted {
                        Validity::Valid
                    } else {
                        Validity::AmbiguousAlignment
                    },
                    confidence: result.confidence,
                    aligned_src,
                    cap_start: result.capture_range.start as usize,
                    len,
                    transfer_h,
                },
            );
        }

        // A configured dictionary must parse — silently dropping a
        // malformed dictionary would hide the fact that dictionary
        // matching never ran.
        let dictionary = match ctx.config.get("dictionary_text").and_then(|v| v.as_str()) {
            Some(text) => Some(al_media::Dictionary::from_json(text).map_err(|e| {
                StageError::new(
                    StageKind::ExpensiveAnalysis,
                    format!("dictionary_text is malformed: {e}"),
                )
            })?),
            None => None,
        };
        for input in &ctx.manifest.inputs {
            let input_id = input.input_id.to_string();
            let Some(dec) = decoded.get(&input_id) else {
                continue;
            };
            let Some(duration_s) = duration_of(&input_id) else {
                continue;
            };
            let own_events: Vec<events::CandidateEvent> = all_events
                .iter()
                .filter(|e| e.input_id == input_id)
                .cloned()
                .collect();
            let regions = expensive::plan_regions(duration_s, &own_events, &e_cfg);
            let comparison = pair_captures.get(&input_id).copied().unwrap_or(dec);
            let mut lineage = dec.artifact_ids.clone();
            if let Some(ids) = pair_lineage.get(&input_id) {
                lineage.extend(ids.iter().cloned());
            }
            let chans: Vec<&[f64]> = comparison
                .channel_samples
                .iter()
                .map(|c| c.as_slice())
                .collect();
            for (i, region) in regions.iter().enumerate() {
                let name = format!("{input_id}__r{i:03}");
                let out = expensive::analyze_region(
                    &input_id,
                    &name,
                    &chans,
                    dec.sample_rate,
                    region,
                    dictionary.as_ref(),
                    pairs.get(&input_id),
                    &e_cfg,
                );
                let prov = al_dataset::provenance::Provenance {
                    input_id: input_id.clone(),
                    algorithm_id: "expensive.v1".into(),
                    algorithm_version: "1".into(),
                    params: serde_json::json!({
                        "triggers": region.triggers,
                    }),
                    alignment_id: None,
                    parents: lineage.iter().map(|a| a.as_str().to_string()).collect(),
                };
                for tensor in &out.tensors {
                    write_tensor(&mut writer, tensor, prov.clone(), chunk_bytes).map_err(err)?;
                }
                writer
                    .write_json(
                        "features/expensive",
                        &name,
                        &serde_json::json!({
                            "region_s": [region.start_s, region.end_s],
                            "triggers": region.triggers,
                            "records": out.records,
                        }),
                        prov,
                    )
                    .map_err(err)?;
                records.push(ArtifactRecord {
                    artifact_id: ctx.registry.issue(),
                    role: "expensive_features".into(),
                    path: PathBuf::from(format!("analysis/features/expensive/{name}.json")),
                    produced_by: Some(StageKind::ExpensiveAnalysis),
                    inputs: lineage.clone(),
                    params: serde_json::json!({"triggers": region.triggers}),
                });
            }
        }

        writer.finish().map_err(err)?;
        Ok(records)
    }
}

/// Read and parse one `locate_result` artifact body
/// (`{"result": LocateResult, ...}`). `kind` selects which stage the
/// `StageError` blames — the same parse is shared by the reference
/// and expensive stages.
fn read_locate_result(
    kind: StageKind,
    job_dir: &std::path::Path,
    artifact: &ArtifactRecord,
) -> Result<LocateResult, StageError> {
    let text = std::fs::read_to_string(
        checked_job_path(job_dir, &artifact.path)
            .map_err(|e| StageError::new(kind, e.to_string()))?,
    )
    .map_err(|e| StageError::new(kind, e.to_string()))?;
    let body: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        StageError::new(
            kind,
            format!("locate result {} unreadable: {e}", artifact.path.display()),
        )
    })?;
    let result: LocateResult = serde_json::from_value(body["result"].clone()).map_err(|e| {
        StageError::new(
            kind,
            format!("locate result {} malformed: {e}", artifact.path.display()),
        )
    })?;
    for pcm in [&result.source_pcm, &result.capture_pcm]
        .into_iter()
        .flatten()
    {
        if !artifact.inputs.contains(&pcm.artifact_id) {
            return Err(StageError::new(
                kind,
                "locate_result lineage missing normalized PCM",
            ));
        }
    }
    Ok(result)
}

/// Read `intermediate/{pair}/transfer_h` — `[re, im] × bins`, the
/// Welch-averaged H(f) written by `ReferenceAnalysisStage` — when the
/// tensor exists in the analysis dataset. `Ok(None)` when the job
/// never ran reference analysis; pair evidence then lacks H(f) and
/// deconvolution-based families degrade to invalid records.
fn read_transfer_h(
    reader: &al_dataset::reader::DatasetReader,
    pair: &str,
) -> io::Result<Option<Vec<(f64, f64)>>> {
    read_complex_tensor(reader, pair, "transfer_h")
}

fn read_complex_tensor(
    reader: &al_dataset::reader::DatasetReader,
    pair: &str,
    kind: &str,
) -> io::Result<Option<Vec<(f64, f64)>>> {
    let name = format!("{pair}/{kind}");
    let path = format!("intermediate/{name}");
    if !reader
        .artifacts("intermediate")
        .iter()
        .any(|a| a.path == path)
    {
        return Ok(None);
    }
    let t = reader.open_tensor("intermediate", &name)?;
    let shape = t.meta().shape.clone();
    if shape.len() != 2 || shape[0] != 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{kind} tensor {path} has unexpected shape {shape:?}"),
        ));
    }
    let bins = shape[1] as usize;
    let (data, _) = reader.read_tensor("intermediate", &name, &[[0, 2], [0, shape[1]]])?;
    let h = (0..bins).map(|k| (data[k], data[bins + k])).collect();
    Ok(Some(h))
}

/// Default persisted-tensor chunk budget (~4 MiB), overridable via
/// `config.analysis.dataset.tensor_chunk_bytes`. Large intermediates
/// (STFT/PSD frames × bins) must stay range-readable without loading
/// the whole array, so one chunk never exceeds the budget.
const DEFAULT_TENSOR_CHUNK_BYTES: u64 = 4 * 1024 * 1024;

/// Byte budget for one persisted tensor chunk.
fn tensor_chunk_bytes(config: &serde_json::Value) -> u64 {
    config
        .get("analysis")
        .and_then(|a| a.get("dataset"))
        .and_then(|d| d.get("tensor_chunk_bytes"))
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_TENSOR_CHUNK_BYTES)
        // Below one f64 element no grid is possible.
        .max(8)
}

/// Chunk grid for a `[time, bins]` tensor: whole bin rows grouped so
/// one chunk stays under `bytes`, splitting along the time axis.
/// A single row wider than the budget is column-split instead so the
/// cap still holds.
fn tensor_chunk_shape(shape: [u64; 2], bytes: u64) -> [u64; 2] {
    let budget = (bytes / 8).max(1);
    let cols = shape[1].min(budget).max(1);
    let rows = (budget / cols).clamp(1, shape[0].max(1));
    [rows, cols]
}

/// Split a tensor's `intermediate/...`, `features/cheap/...`,
/// `features/expensive/...`, or `residual/...` path into role/name
/// and persist it chunked along the time axis under `provenance`
/// (producer algorithm + alignment + parents supplied by the
/// calling stage).
fn write_tensor(
    writer: &mut DatasetWriter,
    tensor: &cheap::NamedTensor,
    provenance: al_dataset::provenance::Provenance,
    chunk_bytes: u64,
) -> io::Result<()> {
    let (role, name) = [
        "intermediate/",
        "features/cheap/",
        "features/expensive/",
        "residual/",
    ]
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
    let [rows, cols] = tensor.shape;
    let chunk_shape = tensor_chunk_shape(tensor.shape, chunk_bytes);
    let (r_step, c_step) = (chunk_shape[0], chunk_shape[1]);
    let mut chunks = Vec::new();
    for r0 in (0..rows).step_by(r_step as usize) {
        for c0 in (0..cols).step_by(c_step as usize) {
            let r_end = (r0 + r_step).min(rows);
            let c_end = (c0 + c_step).min(cols);
            let mut data = Vec::with_capacity(((r_end - r0) * (c_end - c0)) as usize);
            for r in r0..r_end {
                let lo = (r * cols + c0) as usize;
                data.extend_from_slice(&tensor.data[lo..lo + (c_end - c0) as usize]);
            }
            chunks.push(al_dataset::writer::TensorChunk {
                coords: vec![r0 / r_step, c0 / c_step],
                data,
            });
        }
    }
    writer.write_tensor(role, name, &tensor.shape, &chunk_shape, &chunks, provenance)
}

/// Load this input's ingest-side `video_features` artifact, if one
/// was produced. Returns `(features, artifact_id)` so the consumer can
/// extend its lineage. An absent artifact is `Ok(None)` — the input
/// simply has no video stream.
fn load_video_features(
    input: &al_job::job::JobInput,
    ctx: &StageContext<'_>,
) -> Result<Option<(al_media::VideoFeatureArtifact, String)>, StageError> {
    let job_dir = ctx.registry.job_dir();
    let prefix = PathBuf::from("inputs").join(input.input_id.as_str());
    let Some(artifact) = ctx
        .manifest
        .artifacts
        .iter()
        .find(|a| a.role == "video_features" && a.path.starts_with(&prefix))
    else {
        return Ok(None);
    };
    let path = checked_job_path(job_dir, &artifact.path)
        .map_err(|e| StageError::new(StageKind::CheapAnalysis, e.to_string()))?;
    let text = std::fs::read_to_string(&path).map_err(|e| {
        StageError::new(
            StageKind::CheapAnalysis,
            format!("video features {} unreadable: {e}", artifact.path.display()),
        )
    })?;
    let report: al_media::VideoFeatureReport = serde_json::from_str(&text).map_err(|e| {
        StageError::new(
            StageKind::CheapAnalysis,
            format!("video features {} malformed: {e}", artifact.path.display()),
        )
    })?;
    Ok(Some((
        report.features,
        artifact.artifact_id.as_str().to_string(),
    )))
}

/// 選択された音声 stream の時計を ingest の正本から読む。
/// secondary audio の原点を first audio と取り違えない。
fn load_audio_origin(
    input: &al_job::job::JobInput,
    ctx: &StageContext<'_>,
) -> Result<(f64, String), StageError> {
    let fail = |msg: String| StageError::new(StageKind::CheapAnalysis, msg);
    let declared = ctx
        .manifest
        .input_artifact(&input.input_id)
        .ok_or_else(|| fail("missing declared input".into()))?;
    let artifact = ctx
        .manifest
        .artifacts
        .iter()
        .find(|a| a.role == "ingest_metadata" && a.inputs.contains(&declared))
        .ok_or_else(|| fail("missing ingest metadata for audio clock".into()))?;
    let path = checked_job_path(ctx.registry.job_dir(), &artifact.path)
        .map_err(|e| fail(e.to_string()))?;
    let metadata: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).map_err(|e| fail(e.to_string()))?)
            .map_err(|e| fail(e.to_string()))?;
    let streams: Vec<al_media::decoder::StreamResult> =
        serde_json::from_value(metadata["decoder"]["streams"].clone())
            .map_err(|e| fail(e.to_string()))?;
    let timings: Vec<al_media::StreamTiming> =
        serde_json::from_value(metadata["av_timing"].clone()).map_err(|e| fail(e.to_string()))?;
    let selected = streams
        .iter()
        .find(|s| s.kind == "audio" && s.decoded)
        .and_then(|s| s.stream);
    let origin = timings
        .iter()
        .find(|t| {
            Some(t.stream) == selected && t.observation.clock == al_media::timing::ClockKind::Audio
        })
        .filter(|t| t.observation.timebase_hz > 0)
        .map(|t| t.start_tick as f64 / t.observation.timebase_hz as f64)
        .unwrap_or(f64::NAN);
    Ok((origin, artifact.artifact_id.to_string()))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn noise(n: usize, seed: u64) -> Vec<f64> {
        let mut s = seed;
        (0..n)
            .map(|_| {
                s = s
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ((s >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0
            })
            .collect()
    }

    #[test]
    fn chunk_shape_splits_the_time_axis_under_the_budget() {
        // 16 KiB budget = 2048 f64 elements → 4 rows of 512 bins.
        assert_eq!(tensor_chunk_shape([100, 512], 16 * 1024), [4, 512]);
        // Budget above the whole tensor → a single chunk.
        assert_eq!(
            tensor_chunk_shape([61, 2048], DEFAULT_TENSOR_CHUNK_BYTES),
            [61, 2048]
        );
        // A row wider than the budget is column-split so the cap
        // still holds.
        let shape = tensor_chunk_shape([8, 1_000_000], 16 * 1024);
        assert_eq!(shape, [1, 2048]);
        // Degenerate budgets still yield a legal grid.
        assert_eq!(tensor_chunk_shape([3, 7], 8), [1, 1]);
    }

    #[test]
    fn persisted_stft_reads_partially_across_chunk_boundaries() {
        // A real STFT persisted under a small budget must become
        // multiple chunks, and a range read crossing chunk boundaries
        // must return exact values without the whole array.
        let rate = 48_000u32;
        let m = crate::stft::stft(&noise(1 << 15, 0x5eed), 2048, 512, rate);
        let frames = m.frames() as u64;
        let cols = m.fft_size as u64;
        let data: Vec<f64> = m.re.iter().flatten().copied().collect();
        let tensor = cheap::NamedTensor {
            path: "intermediate/in/stft_re".into(),
            shape: [frames, cols],
            data: data.clone(),
        };

        let dir = std::env::temp_dir().join(format!("al-stage-{}", al_core::JobId::new()));
        let mut writer = DatasetWriter::create(&dir, Vec::new(), serde_json::json!({})).unwrap();
        let prov = al_dataset::provenance::Provenance {
            input_id: "in".into(),
            algorithm_id: "reference.v1".into(),
            algorithm_version: "1".into(),
            params: serde_json::json!({}),
            alignment_id: Some("aln-x".into()),
            parents: vec!["ART-parent".into()],
        };
        // 64 KiB budget = 8192 elements → 4 frames per 2048-bin chunk.
        write_tensor(&mut writer, &tensor, prov, 64 * 1024).unwrap();
        writer.finish().unwrap();

        let reader = al_dataset::reader::DatasetReader::open(&dir).unwrap();
        let t = reader.open_tensor("intermediate", "in/stft_re").unwrap();
        assert_eq!(t.meta().chunk_shape, vec![4, 2048]);
        let chunk_files = std::fs::read_dir(dir.join("intermediate/in/stft_re/chunks"))
            .unwrap()
            .count();
        assert!(chunk_files > 1, "expected multiple chunks");

        // Caller-supplied provenance lands on the manifest artifact.
        let artifact = reader
            .manifest()
            .artifacts
            .iter()
            .find(|a| a.path == "intermediate/in/stft_re")
            .expect("tensor artifact registered");
        assert_eq!(artifact.provenance.algorithm_id, "reference.v1");
        assert_eq!(artifact.provenance.alignment_id.as_deref(), Some("aln-x"));

        // Rows 3..9 span chunks 0..2 (4 rows per chunk).
        let (vals, shape) = reader
            .read_tensor("intermediate", "in/stft_re", &[[3, 9], [0, cols]])
            .unwrap();
        assert_eq!(shape, vec![6, cols]);
        let expect: Vec<f64> = (3..9)
            .flat_map(|r| {
                data[(r * cols) as usize..((r + 1) * cols) as usize]
                    .iter()
                    .copied()
            })
            .collect();
        assert_eq!(vals, expect);

        std::fs::remove_dir_all(&dir).ok();
    }
}
