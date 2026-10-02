//! `LocateStage`: pairs `source_asset` inputs with `capture_*`
//! inputs, runs the four locate methods per channel, evaluates
//! consensus, fits the mix, identifies occurrences, builds the
//! piecewise time map, runs the two-stage residual search, and
//! persists every result — accepted and rejected alike.
//!
//! Sample-rate-mismatched pairs are never resampled: they get an
//! explicit marked result instead. A pair whose decoded PCM is
//! missing or empty is marked per-pair — one bad input never aborts
//! the stage's other pairs.

use std::fs;
use std::path::PathBuf;

use al_core::{AlignmentId, Confidence, InputRole, StageKind, Validity};
use al_job::manifest::ArtifactRecord;
use al_job::stage::{Stage, StageContext, StageError};

use crate::candidate::{LocateCandidate, MethodEvidence};
use crate::consensus::{ConsensusConfig, evaluate};
use crate::decode::{DecodedInput, decode_input};
use crate::dsp;
use crate::methods::default_methods;
use crate::mixer::fit_mix;
use crate::residual::residual_search;
use crate::result::{LocateResult, SampleRange};
use crate::tracking::{Anchor, build_time_map, identify_loop_occurrences};

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

fn err(e: impl std::fmt::Display) -> StageError {
    StageError::new(StageKind::Locate, e.to_string())
}

/// Sub-block anchors used to build a pair's time map: each block of
/// the source is re-located inside the matched capture window so
/// drift/skips inside one occurrence become visible.
const ANCHOR_BLOCKS: usize = 16;
/// Minimum block energy to emit an anchor.
const MIN_ANCHOR_ENERGY: f64 = 1e-9;
/// Candidates merged across channel pairs per method.
const MAX_MERGED_CANDIDATES: usize = 16;

/// Locate `block`-sized pieces of `source` inside the capture window
/// starting at `pos` and return them as anchors.
fn block_anchors(source: &[f64], capture_window: &[f64], pos: usize, blocks: usize) -> Vec<Anchor> {
    let n = source.len();
    if n == 0 || capture_window.is_empty() {
        return Vec::new();
    }
    let block = (n / blocks.max(1)).max(1);
    let mut anchors = Vec::new();
    for b0 in (0..n).step_by(block) {
        let b1 = (b0 + block).min(n);
        let template = &source[b0..b1];
        if template.iter().map(|v| v * v).sum::<f64>() < MIN_ANCHOR_ENERGY {
            continue;
        }
        if capture_window.len() < template.len() {
            continue;
        }
        let corr = dsp::ncc(
            template,
            capture_window,
            capture_window.len() - template.len(),
        );
        if let Some((bi, &bs)) = corr
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
        {
            anchors.push(Anchor {
                source: b0 as u64,
                capture: (pos + bi) as u64,
                score: bs,
            });
        }
    }
    anchors
}

impl LocateStage {
    /// Run every method on every channel pair; merge all pairs'
    /// candidates (each tagged with its channel pair) into one
    /// evidence per method. The method's validity comes from the
    /// strongest pair — but no pair's candidates are dropped.
    fn locate_pair(&self, source: &DecodedInput, capture: &DecodedInput) -> Vec<MethodEvidence> {
        let methods = default_methods();
        let mut merged: Vec<MethodEvidence> = Vec::new();
        for method in &methods {
            let mut per_pair: Vec<MethodEvidence> = Vec::new();
            for (si, src_ch) in source.channel_samples.iter().enumerate() {
                for (ci, cap_ch) in capture.channel_samples.iter().enumerate() {
                    let mut ev = method.locate(src_ch, cap_ch, source.sample_rate);
                    for c in &mut ev.candidates {
                        c.channel_pair = Some([si as u32, ci as u32]);
                    }
                    per_pair.push(ev);
                }
            }
            // Best pair by top-candidate score magnitude anchors the
            // method's validity; candidates merge across all pairs.
            let best = per_pair
                .iter()
                .max_by(|a, b| {
                    let sa = a.candidates.first().map(|c| c.score.abs()).unwrap_or(0.0);
                    let sb = b.candidates.first().map(|c| c.score.abs()).unwrap_or(0.0);
                    sa.total_cmp(&sb)
                })
                .cloned();
            let mut candidates: Vec<LocateCandidate> =
                per_pair.into_iter().flat_map(|e| e.candidates).collect();
            candidates.sort_by(|a, b| b.score.abs().total_cmp(&a.score.abs()));
            candidates.truncate(MAX_MERGED_CANDIDATES);
            match best {
                Some(mut ev) => {
                    ev.candidates = candidates;
                    ev.method = method.id().into();
                    ev.method_version = method.version().into();
                    merged.push(ev);
                }
                None => merged.push(MethodEvidence {
                    method: method.id().into(),
                    method_version: method.version().into(),
                    candidates: vec![],
                    validity: Validity::ComputationFailed,
                    reject_reason: Some("no channel pair could be evaluated".into()),
                }),
            }
        }
        merged
    }

    /// Positions where ≥ `min_agreeing_methods` *distinct* methods
    /// placed a qualifying candidate within `position_tolerance` of
    /// each other. The accepted primary cluster is excluded; on a
    /// contested result every qualifying cluster is reported (the
    /// spec's "複数の正当な出現" case).
    fn occurrences(&self, evidence: &[MethodEvidence], result: &LocateResult) -> Vec<u64> {
        let cfg = &self.config.consensus;
        let mut votes: Vec<(String, u64)> = evidence
            .iter()
            .filter(|e| e.validity == Validity::Valid)
            .flat_map(|e| {
                e.candidates
                    .iter()
                    .filter(|c| {
                        c.score.abs() >= cfg.min_score
                            && c.peak_to_background >= cfg.min_peak_to_background
                    })
                    .map(move |c| (e.method.clone(), c.position))
            })
            .collect();
        votes.sort_by_key(|v| v.1);
        let mut clusters: Vec<Vec<(String, u64)>> = Vec::new();
        for v in votes {
            match clusters.last_mut() {
                Some(c) if v.1 - c[0].1 <= cfg.position_tolerance => c.push(v),
                _ => clusters.push(vec![v]),
            }
        }
        let primary = result.capture_range.start;
        clusters
            .iter()
            .filter(|c| {
                let mut ms: Vec<&str> = c.iter().map(|v| v.0.as_str()).collect();
                ms.sort_unstable();
                ms.dedup();
                ms.len() as u32 >= cfg.min_agreeing_methods
            })
            .map(|c| c[c.len() / 2].1)
            .filter(|&p| !(result.accepted && p.abs_diff(primary) <= cfg.position_tolerance))
            .collect()
    }

    /// A marked result for a pair that cannot be located.
    fn marked_result(
        &self,
        source_input_id: &str,
        capture_input_id: &str,
        validity: Validity,
        reason: impl Into<String>,
    ) -> LocateResult {
        LocateResult {
            alignment_id: AlignmentId::new(),
            source_input_id: source_input_id.into(),
            capture_input_id: capture_input_id.into(),
            source_range: SampleRange { start: 0, end: 0 },
            capture_range: SampleRange { start: 0, end: 0 },
            evidence: vec![],
            agreeing_methods: 0,
            position_spread: 0,
            confidence: Confidence::Low,
            validity,
            reject_reason: Some(reason.into()),
            accepted: false,
            algorithm_id: "locate-consensus".into(),
            algorithm_version: "1".into(),
            params: serde_json::to_value(&self.config.consensus).unwrap_or_default(),
        }
    }

    /// Persist one `LocateResult` plus derived artifacts under
    /// `locate/<source>/<capture>/`.
    fn persist_result(
        &self,
        result: &LocateResult,
        extra: serde_json::Value,
        residual_pcm: Option<&[f64]>,
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
        let mut records = vec![ArtifactRecord {
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
        }];
        if let Some(samples) = residual_pcm {
            let rel = dir.join("residual.f64le");
            let bytes: Vec<u8> = samples.iter().flat_map(|v| v.to_le_bytes()).collect();
            al_job::fsutil::write_durable(&job_dir.join(&rel), &bytes).map_err(err)?;
            records.push(ArtifactRecord {
                artifact_id: ctx.registry.issue(),
                role: "locate_residual".into(),
                path: rel,
                produced_by: Some(StageKind::Locate),
                inputs: lineage.to_vec(),
                params: serde_json::json!({
                    "source_input_id": result.source_input_id,
                    "capture_input_id": result.capture_input_id,
                    "alignment_id": result.alignment_id,
                    "format": "f64le-interleaved",
                }),
            });
        }
        Ok(records)
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
        // Nothing to pair (capture-only or source-only job) →
        // complete with empty output so the pipeline can continue to
        // capture-only analysis. Pairable work always exists only
        // when both roles are present.
        if sources.is_empty() || captures.is_empty() {
            return Ok(Vec::new());
        }

        // Decode each input once — a missing decode becomes a
        // per-pair mark below, never an abort for the whole stage.
        let mut decoded_sources = Vec::with_capacity(sources.len());
        for input in &sources {
            decoded_sources.push(decode_input(input, ctx).map_err(err)?);
        }
        let mut decoded_captures = Vec::with_capacity(captures.len());
        for input in &captures {
            decoded_captures.push(decode_input(input, ctx).map_err(err)?);
        }

        let mut records = Vec::new();
        for (si, src_input) in sources.iter().enumerate() {
            for (ci, cap_input) in captures.iter().enumerate() {
                let dir = PathBuf::from("locate")
                    .join(src_input.input_id.to_string())
                    .join(cap_input.input_id.to_string());

                // Any missing decode → per-pair explicit mark.
                let (source, capture) = match (&decoded_sources[si], &decoded_captures[ci]) {
                    (Some(s), Some(c)) => (s, c),
                    _ => {
                        let missing = match (&decoded_sources[si], &decoded_captures[ci]) {
                            (None, _) => {
                                format!("no decoded_pcm artifact for source {}", src_input.input_id)
                            }
                            (_, None) => format!(
                                "no decoded_pcm artifact for capture {}",
                                cap_input.input_id
                            ),
                            _ => unreachable!(),
                        };
                        let result = self.marked_result(
                            &src_input.input_id.to_string(),
                            &cap_input.input_id.to_string(),
                            Validity::DecodeInputUnavailable,
                            missing,
                        );
                        let mut lineage = decoded_sources[si]
                            .as_ref()
                            .map(|s| s.artifact_ids.clone())
                            .unwrap_or_default();
                        if let Some(c) = &decoded_captures[ci] {
                            lineage.extend(c.artifact_ids.iter().cloned());
                        }
                        records.extend(self.persist_result(
                            &result,
                            serde_json::json!({}),
                            None,
                            &lineage,
                            dir,
                            ctx,
                        )?);
                        continue;
                    }
                };

                // Empty/degenerate decoded input → explicit mark, not
                // a panic.
                if source.channel_samples.is_empty()
                    || capture.channel_samples.is_empty()
                    || source.channel_samples.iter().all(|c| c.is_empty())
                    || capture.channel_samples.iter().all(|c| c.is_empty())
                {
                    let result = self.marked_result(
                        &source.input_id,
                        &capture.input_id,
                        Validity::InsufficientSupport,
                        "decoded PCM is empty",
                    );
                    let mut lineage = source.artifact_ids.clone();
                    lineage.extend(capture.artifact_ids.iter().cloned());
                    records.extend(self.persist_result(
                        &result,
                        serde_json::json!({"empty_input": true}),
                        None,
                        &lineage,
                        dir,
                        ctx,
                    )?);
                    continue;
                }

                // Sample-rate mismatch: never resample — mark the pair
                // explicitly instead.
                if source.sample_rate != capture.sample_rate {
                    let result = self.marked_result(
                        &source.input_id,
                        &capture.input_id,
                        Validity::OutOfSupportedFrequencyRange,
                        format!(
                            "sample rate mismatch: source {} Hz vs capture {} Hz",
                            source.sample_rate, capture.sample_rate
                        ),
                    );
                    let mut lineage = source.artifact_ids.clone();
                    lineage.extend(capture.artifact_ids.iter().cloned());
                    records.extend(self.persist_result(
                        &result,
                        serde_json::json!({"sample_rate_mismatch": true}),
                        None,
                        &lineage,
                        dir,
                        ctx,
                    )?);
                    continue;
                }

                let evidence = self.locate_pair(source, capture);
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

                let occurrences = self.occurrences(&evidence, &result);
                let mut derived = serde_json::json!({"occurrences": occurrences});
                let mut residual_pcm: Option<Vec<f64>> = None;

                if result.accepted {
                    let primary = result.capture_range.start as usize;
                    let len = source_len as usize;
                    if primary + len <= capture.channel_samples[0].len() {
                        let cap_window: Vec<Vec<f64>> = capture
                            .channel_samples
                            .iter()
                            .map(|c| c[primary..primary + len].to_vec())
                            .collect();
                        if let Some(mix) =
                            fit_mix(&source.channel_samples, &cap_window, self.config.ridge)
                        {
                            derived["mixer"] = serde_json::to_value(&mix).unwrap_or_default();
                        }

                        // Two-stage residual search: subtract the fit,
                        // then re-run the methods on the residual so a
                        // quieter repetition becomes findable.
                        let cfg = self.config.consensus.clone();
                        let src_id = source.input_id.clone();
                        let cap_id = capture.input_id.clone();
                        let stage_config = self.config.clone();
                        let search = residual_search(
                            &source.input_id,
                            result.alignment_id.clone(),
                            &source.channel_samples,
                            &cap_window,
                            self.config.ridge,
                            |residual| {
                                let residual_input = DecodedInput {
                                    input_id: cap_id.clone(),
                                    sample_rate: capture.sample_rate,
                                    channels: capture.channels,
                                    channel_samples: residual.to_vec(),
                                    artifact_ids: vec![],
                                };
                                let residual_stage = LocateStage::with_config(stage_config);
                                let child_evidence =
                                    residual_stage.locate_pair(source, &residual_input);
                                evaluate(
                                    &child_evidence,
                                    &src_id,
                                    &cap_id,
                                    source_len,
                                    len as u64,
                                    &cfg,
                                    "locate-residual",
                                    "1",
                                )
                            },
                        );
                        if let Some((search, residual_chs)) = search {
                            // Persist the residual itself (interleaved
                            // f64) alongside the search record.
                            let mut interleaved = Vec::new();
                            if let Some(first) = residual_chs.first() {
                                let frames = first.len();
                                interleaved.reserve(frames * residual_chs.len());
                                for t in 0..frames {
                                    for ch in &residual_chs {
                                        interleaved.push(ch[t]);
                                    }
                                }
                            }
                            residual_pcm = Some(interleaved);
                            derived["residual_search"] =
                                serde_json::to_value(&search).unwrap_or_default();
                        }
                    }

                    // Occurrence anchors → piecewise time map. Use the
                    // best-scoring candidate's channel pair (fallback
                    // 0/0) for the anchor scan.
                    let pair = evidence
                        .iter()
                        .flat_map(|e| e.candidates.iter())
                        .max_by(|a, b| a.score.abs().total_cmp(&b.score.abs()))
                        .and_then(|c| c.channel_pair)
                        .unwrap_or([0, 0]);
                    let src_ch = &source.channel_samples[pair[0] as usize];
                    let cap_ch = &capture.channel_samples[pair[1] as usize];
                    let mut occurrence_positions = vec![primary as u64];
                    occurrence_positions.extend(&occurrences);
                    let mut anchors: Vec<Anchor> = Vec::new();
                    for &pos in &occurrence_positions {
                        let p = pos as usize;
                        let end = (p + len).min(cap_ch.len());
                        if p >= end {
                            continue;
                        }
                        anchors.extend(block_anchors(src_ch, &cap_ch[p..end], p, ANCHOR_BLOCKS));
                    }
                    anchors.sort_by_key(|a| a.capture);
                    let time_map = build_time_map(
                        &anchors,
                        0.10,
                        (len / ANCHOR_BLOCKS.max(1)).max(1) as u64 * 2,
                        3,
                    );
                    let loops = identify_loop_occurrences(&occurrence_positions, source_len.max(1));
                    derived["loop_occurrences"] = serde_json::to_value(&loops).unwrap_or_default();
                    derived["time_map"] = serde_json::to_value(&time_map).unwrap_or_default();
                }

                let mut lineage = source.artifact_ids.clone();
                lineage.extend(capture.artifact_ids.iter().cloned());
                records.extend(self.persist_result(
                    &result,
                    derived,
                    residual_pcm.as_deref(),
                    &lineage,
                    dir,
                    ctx,
                )?);
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
