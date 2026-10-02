//! `LocateStage`: pairs `source_asset` inputs with `capture_*`
//! inputs, runs the four locate methods per channel, evaluates
//! consensus, fits the mix, identifies occurrences, and persists
//! every result — accepted and rejected alike.
//!
//! Sample-rate-mismatched pairs are never resampled: they get an
//! explicit `DecodeInputUnavailable` result instead.

use std::fs;
use std::path::PathBuf;

use al_core::{Confidence, InputRole, StageKind, Validity};
use al_job::manifest::ArtifactRecord;
use al_job::stage::{Stage, StageContext, StageError};
use al_media::metadata::SampleFormat;

use crate::candidate::MethodEvidence;
use crate::consensus::{ConsensusConfig, evaluate};
use crate::dsp::deinterleave;
use crate::methods::default_methods;
use crate::mixer::fit_mix;
use crate::result::{LocateResult, SampleRange};
use crate::tracking::{Anchor, Segment, SegmentKind, TimeMap, identify_loop_occurrences};

/// Locate stage configuration (overlaid on defaults).
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct LocateConfig {
    /// Consensus thresholds.
    #[serde(default)]
    pub consensus: ConsensusConfig,
    /// Ridge parameter for mixer fitting.
    #[serde(default = "default_ridge")]
    pub ridge: f64,
}

fn default_ridge() -> f64 {
    1e-3
}

impl Default for LocateConfig {
    fn default() -> Self {
        Self {
            consensus: ConsensusConfig::default(),
            ridge: default_ridge(),
        }
    }
}

/// Locate stage.
#[derive(Debug, Default)]
pub struct LocateStage {
    config: LocateConfig,
}

impl LocateStage {
    /// Stage with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stage with an explicit configuration.
    pub fn with_config(config: LocateConfig) -> Self {
        Self { config }
    }
}

/// Decoded PCM of one input, with channel structure restored.
struct DecodedInput {
    input_id: String,
    sample_rate: u32,
    /// Deinterleaved channels, normalized to float range.
    channel_samples: Vec<Vec<f64>>,
    /// Artifact ids this input's PCM and metadata came from.
    artifact_ids: Vec<al_core::ArtifactId>,
}

fn err(e: impl std::fmt::Display) -> StageError {
    StageError::new(StageKind::Locate, e.to_string())
}

/// Scale factor mapping one `SampleFormat` into `[-1, 1]` float.
/// Integer PCM is stored integer-scaled; spectral-envelope
/// comparisons and residual energies are absolute-scale-sensitive.
fn format_scale(format: SampleFormat) -> f64 {
    match format {
        SampleFormat::PcmU8 => 128.0,
        SampleFormat::PcmS16 => 32768.0,
        SampleFormat::PcmS24 => 8_388_608.0,
        SampleFormat::PcmS32 => 2_147_483_648.0,
        SampleFormat::Float32 | SampleFormat::Float64 | SampleFormat::Other => 1.0,
    }
}

impl LocateStage {
    /// Restore one input's decoded PCM from ingest artifacts.
    fn decoded(
        &self,
        input: &al_job::job::JobInput,
        ctx: &StageContext<'_>,
    ) -> Result<Option<DecodedInput>, StageError> {
        let input_artifact = ctx.manifest.input_artifact(&input.input_id);
        let decoded = ctx.manifest.artifacts.iter().find(|a| {
            a.role == "decoded_pcm"
                && input_artifact
                    .as_ref()
                    .is_some_and(|ia| a.inputs.contains(ia))
        });
        let Some(decoded) = decoded else {
            return Ok(None);
        };
        let sample_rate = decoded.params["sample_rate"]
            .as_u64()
            .ok_or_else(|| err("decoded_pcm artifact missing sample_rate"))?
            as u32;
        let channels = decoded.params["channels"]
            .as_u64()
            .ok_or_else(|| err("decoded_pcm artifact missing channels"))?
            as u16;

        // Sample format comes from the sibling ingest metadata —
        // decoded PCM keeps the source-native integer scale, so the
        // format decides the float normalization.
        let metadata = ctx.manifest.artifacts.iter().find(|a| {
            a.role == "ingest_metadata"
                && input_artifact
                    .as_ref()
                    .is_some_and(|ia| a.inputs.contains(ia))
        });
        let scale = match metadata {
            Some(meta) => {
                let text =
                    fs::read_to_string(ctx.registry.job_dir().join(&meta.path)).map_err(err)?;
                let json: serde_json::Value = serde_json::from_str(&text).map_err(err)?;
                let format: SampleFormat = json
                    .pointer("/audio/sample_format")
                    .cloned()
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or(SampleFormat::Other);
                format_scale(format)
            }
            None => 1.0,
        };

        let bytes = fs::read(ctx.registry.job_dir().join(&decoded.path)).map_err(err)?;
        let samples: Vec<f64> = bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|b| f64::from_le_bytes(*b) / scale)
            .collect();
        let channel_samples = deinterleave(&samples, channels);
        let mut artifact_ids = vec![decoded.artifact_id.clone()];
        if let Some(m) = metadata {
            artifact_ids.push(m.artifact_id.clone());
        }
        Ok(Some(DecodedInput {
            input_id: input.input_id.to_string(),
            sample_rate,
            channel_samples,
            artifact_ids,
        }))
    }

    /// Run every method on every channel pair; keep, per method, the
    /// evidence of the strongest channel pair plus the merged
    /// candidates of all pairs so channel detail survives.
    fn locate_pair(&self, source: &DecodedInput, capture: &DecodedInput) -> Vec<MethodEvidence> {
        let methods = default_methods();
        let mut merged: Vec<MethodEvidence> = Vec::new();
        for method in &methods {
            let mut per_pair: Vec<MethodEvidence> = Vec::new();
            for src_ch in &source.channel_samples {
                for cap_ch in &capture.channel_samples {
                    per_pair.push(method.locate(src_ch, cap_ch, source.sample_rate));
                }
            }
            // Best pair by top-candidate score anchors the method vote.
            per_pair.sort_by(|a, b| {
                let sa = a.candidates.first().map(|c| c.score).unwrap_or(0.0);
                let sb = b.candidates.first().map(|c| c.score).unwrap_or(0.0);
                sb.total_cmp(&sa)
            });
            let mut best = per_pair.into_iter().next().unwrap_or(MethodEvidence {
                method: method.id().into(),
                method_version: method.version().into(),
                candidates: vec![],
                validity: Validity::ComputationFailed,
                reject_reason: Some("no channel pair could be evaluated".into()),
            });
            best.method = method.id().into();
            best.method_version = method.version().into();
            merged.push(best);
        }
        merged
    }

    /// Persist one `LocateResult` plus derived artifacts under
    /// `locate/<source>/<capture>/`.
    fn persist_result(
        &self,
        result: &LocateResult,
        extra: serde_json::Value,
        lineage: &[al_core::ArtifactId],
        dir: PathBuf,
        ctx: &StageContext<'_>,
    ) -> Result<Vec<ArtifactRecord>, StageError> {
        let job_dir = ctx.registry.job_dir();
        fs::create_dir_all(job_dir.join(&dir)).map_err(err)?;
        let rel = dir.join("result.json");
        let body = serde_json::json!({
            "result": result,
            "derived": extra,
        });
        al_job::fsutil::write_durable(
            &job_dir.join(&rel),
            serde_json::to_string_pretty(&body).map_err(err)?.as_bytes(),
        )
        .map_err(err)?;
        Ok(vec![ArtifactRecord {
            artifact_id: ctx.registry.issue(),
            role: "locate_result".into(),
            path: rel,
            produced_by: Some(StageKind::Locate),
            inputs: lineage.to_vec(),
            params: serde_json::json!({
                "source_input_id": result.source_input_id,
                "capture_input_id": result.capture_input_id,
                "accepted": result.accepted,
                "validity": result.validity,
                "alignment_id": result.alignment_id,
            }),
        }])
    }
}

impl Stage for LocateStage {
    fn kind(&self) -> StageKind {
        StageKind::Locate
    }

    fn run(&mut self, ctx: &StageContext<'_>) -> Result<Vec<ArtifactRecord>, StageError> {
        // Overlay locate config from the job config, when present.
        if let Some(v) = ctx.config.get("locate") {
            self.config = serde_json::from_value(v.clone()).map_err(err)?;
        }

        let sources: Vec<_> = ctx
            .manifest
            .inputs
            .iter()
            .filter(|i| i.role == InputRole::SourceAsset)
            .collect();
        let captures: Vec<_> = ctx
            .manifest
            .inputs
            .iter()
            .filter(|i| matches!(i.role, InputRole::CaptureBug | InputRole::CaptureReference))
            .collect();
        if sources.is_empty() || captures.is_empty() {
            return Err(err(
                "locate requires at least one source_asset and one capture_* input",
            ));
        }

        let mut records = Vec::new();
        for src_input in &sources {
            let Some(source) = self.decoded(src_input, ctx)? else {
                return Err(err(format!(
                    "no decoded_pcm artifact for source {}",
                    src_input.input_id
                )));
            };
            for cap_input in &captures {
                let Some(capture) = self.decoded(cap_input, ctx)? else {
                    return Err(err(format!(
                        "no decoded_pcm artifact for capture {}",
                        cap_input.input_id
                    )));
                };
                let dir = PathBuf::from("locate")
                    .join(&source.input_id)
                    .join(&capture.input_id);

                // Sample-rate mismatch: never resample — mark the pair
                // explicitly instead.
                if source.sample_rate != capture.sample_rate {
                    let result = LocateResult {
                        alignment_id: al_core::AlignmentId::new(),
                        source_input_id: source.input_id.clone(),
                        capture_input_id: capture.input_id.clone(),
                        source_range: SampleRange { start: 0, end: 0 },
                        capture_range: SampleRange { start: 0, end: 0 },
                        evidence: vec![],
                        agreeing_methods: 0,
                        position_spread: 0,
                        confidence: Confidence::Low,
                        validity: Validity::DecodeInputUnavailable,
                        reject_reason: Some(format!(
                            "sample rate mismatch: source {} Hz vs capture {} Hz",
                            source.sample_rate, capture.sample_rate
                        )),
                        accepted: false,
                        algorithm_id: "locate-consensus".into(),
                        algorithm_version: "1".into(),
                        params: serde_json::to_value(&self.config).unwrap_or_default(),
                    };
                    let mut lineage = source.artifact_ids.clone();
                    lineage.extend(capture.artifact_ids.iter().cloned());
                    records.extend(self.persist_result(
                        &result,
                        serde_json::json!({"sample_rate_mismatch": true}),
                        &lineage,
                        dir,
                        ctx,
                    )?);
                    continue;
                }

                let evidence = self.locate_pair(&source, &capture);
                let source_len = source.channel_samples[0].len() as u64;
                let capture_len = capture.channel_samples[0].len() as u64;
                let result = evaluate(
                    &evidence,
                    &source.input_id,
                    &capture.input_id,
                    source_len,
                    capture_len,
                    &self.config.consensus,
                    "locate-consensus",
                    "1",
                );

                // Additional occurrences: cluster every method's
                // candidates beyond the primary position so multiple
                // legitimate appearances stay separate.
                let mut positions: Vec<u64> = evidence
                    .iter()
                    .filter(|e| e.validity == Validity::Valid)
                    .flat_map(|e| e.candidates.iter())
                    .filter(|c| {
                        c.score >= self.config.consensus.min_score
                            && c.peak_to_background >= self.config.consensus.min_peak_to_background
                    })
                    .map(|c| c.position)
                    .collect();
                positions.sort_unstable();
                positions.dedup();
                let primary = result.capture_range.start;
                let occurrences: Vec<u64> = positions
                    .into_iter()
                    .filter(|p| {
                        result.accepted
                            && (*p).abs_diff(primary) > self.config.consensus.position_tolerance
                    })
                    .collect();

                // Derived data for the primary result: mixer fit on the
                // matched window + a time map covering every occurrence.
                let mut derived = serde_json::json!({"occurrences": occurrences});
                if result.accepted {
                    let start = primary as usize;
                    let len = source_len as usize;
                    if start + len <= capture.channel_samples[0].len() {
                        let cap_window: Vec<Vec<f64>> = capture
                            .channel_samples
                            .iter()
                            .map(|c| c[start..start + len].to_vec())
                            .collect();
                        if let Some(mix) =
                            fit_mix(&source.channel_samples, &cap_window, self.config.ridge)
                        {
                            derived["mixer"] = serde_json::to_value(&mix).unwrap_or_default();
                        }
                    }
                    let mut occurrence_positions = vec![primary];
                    occurrence_positions.extend(&occurrences);
                    let loops = identify_loop_occurrences(&occurrence_positions, source_len.max(1));
                    let anchors: Vec<Anchor> = occurrence_positions
                        .iter()
                        .map(|&capture| Anchor {
                            source: 0,
                            capture,
                            score: 1.0,
                        })
                        .collect();
                    let segments = anchors
                        .iter()
                        .map(|a| Segment {
                            kind: SegmentKind::Continuous,
                            anchors: vec![*a],
                            source_range: [0, source_len],
                            capture_range: [a.capture, (a.capture + source_len).min(capture_len)],
                            rate: Some(1.0),
                        })
                        .collect();
                    let time_map = TimeMap { segments };
                    derived["loop_occurrences"] = serde_json::to_value(&loops).unwrap_or_default();
                    derived["time_map"] = serde_json::to_value(&time_map).unwrap_or_default();
                }

                let mut lineage = source.artifact_ids.clone();
                lineage.extend(capture.artifact_ids.iter().cloned());
                records.extend(self.persist_result(&result, derived, &lineage, dir, ctx)?);
            }
        }
        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_overlays_defaults() {
        let cfg: LocateConfig =
            serde_json::from_str(r#"{"consensus":{"min_agreeing_methods":3,"position_tolerance":100,"min_score":0.3,"min_peak_to_background":2.0}}"#)
                .unwrap();
        assert_eq!(cfg.consensus.min_agreeing_methods, 3);
        assert_eq!(cfg.ridge, default_ridge());
    }
}
