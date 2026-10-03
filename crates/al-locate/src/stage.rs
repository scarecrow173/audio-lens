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

use crate::candidate::{LocateCandidate, MethodEvidence};
use crate::consensus::{ConsensusConfig, evaluate};
use crate::dsp::{deinterleave, ncc};
use crate::methods::{default_methods, peaks_from_correlation};
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
    /// Allowed deviation of a segment's dcapture/dsource rate from 1.0
    /// before it stops being `Continuous`.
    #[serde(default = "default_rate_tolerance")]
    pub rate_tolerance: f64,
    /// Anchors an off-rate run must span to count as `Drift`; shorter
    /// off-rate runs are `Jitter`.
    #[serde(default = "default_min_run")]
    pub min_run: usize,
    /// Source window length for per-window anchor extraction
    /// (samples). A source that fits at least two windows gets one
    /// anchor per window, so drift, jitter, skips, gaps and resumes
    /// inside a single occurrence stay observable; shorter sources
    /// keep one anchor per occurrence.
    #[serde(default = "default_anchor_window")]
    pub anchor_window: u64,
    /// Hop between successive anchor windows (samples).
    #[serde(default = "default_anchor_hop")]
    pub anchor_hop: u64,
    /// Slack between the source and capture advances of consecutive
    /// anchors before their boundary becomes `Gap`/`Skip` (samples).
    #[serde(default = "default_anchor_gap")]
    pub anchor_gap: u64,
}

fn default_ridge() -> f64 {
    1e-3
}

fn default_rate_tolerance() -> f64 {
    0.05
}

fn default_min_run() -> usize {
    3
}

fn default_anchor_window() -> u64 {
    512
}

fn default_anchor_hop() -> u64 {
    512
}

fn default_anchor_gap() -> u64 {
    256
}

impl Default for LocateConfig {
    fn default() -> Self {
        Self {
            consensus: ConsensusConfig::default(),
            ridge: default_ridge(),
            rate_tolerance: default_rate_tolerance(),
            min_run: default_min_run(),
            anchor_window: default_anchor_window(),
            anchor_hop: default_anchor_hop(),
            anchor_gap: default_anchor_gap(),
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

    /// Run every method on every channel pair of
    /// `source_channels` × `capture_channels`. Returned evidence is
    /// grouped by method (declaration order) so [`merged_votes`] can
    /// pick each method's strongest pair, and the full per-pair list
    /// is kept for the `locate_evidence` artifact so channel detail
    /// survives.
    fn locate_channels(
        &self,
        source_channels: &[Vec<f64>],
        capture_channels: &[Vec<f64>],
        sample_rate: u32,
    ) -> Vec<PairEvidence> {
        let mut pairs = Vec::new();
        for method in default_methods() {
            for (source_channel, src_ch) in source_channels.iter().enumerate() {
                for (capture_channel, cap_ch) in capture_channels.iter().enumerate() {
                    pairs.push(PairEvidence {
                        source_channel,
                        capture_channel,
                        evidence: method.locate(src_ch, cap_ch, sample_rate),
                    });
                }
            }
        }
        pairs
    }

    /// Best full-band NCC peak of `source[..][w_off..w_off+window]`
    /// inside `capture[..][lo..hi]`, taken across every channel pair.
    /// Returns the absolute capture position and score when the peak
    /// clears the consensus thresholds, `None` otherwise.
    fn best_window_match(
        &self,
        source: &DecodedInput,
        capture: &DecodedInput,
        w_off: usize,
        window: usize,
        lo: usize,
        hi: usize,
    ) -> Option<(u64, f64)> {
        if hi.saturating_sub(lo) < window || w_off + window > source.channel_samples[0].len() {
            return None;
        }
        let mut best: Option<LocateCandidate> = None;
        for src_ch in &source.channel_samples {
            let template = &src_ch[w_off..w_off + window];
            for cap_ch in &capture.channel_samples {
                let corr = ncc(template, &cap_ch[lo..hi], hi - lo - window);
                if let Some(c) = peaks_from_correlation(&corr, 1).into_iter().next()
                    && best.as_ref().is_none_or(|b| c.score > b.score)
                {
                    best = Some(c);
                }
            }
        }
        let c = best?;
        (c.score >= self.config.consensus.min_score
            && c.peak_to_background >= self.config.consensus.min_peak_to_background)
            .then(|| (lo as u64 + c.position, c.score))
    }

    /// One anchor per source window, each located inside
    /// `capture[lo..hi]`. Anchor source coordinates are `base + w_off`
    /// so callers can place them on the unrolled pass grid; windows
    /// whose peak stays below the consensus thresholds contribute no
    /// anchor. Returns an empty list when the source fits fewer than
    /// two windows.
    fn window_anchors_in(
        &self,
        source: &DecodedInput,
        capture: &DecodedInput,
        lo: usize,
        hi: usize,
        base: u64,
    ) -> Vec<Anchor> {
        let window = self.config.anchor_window as usize;
        let hop = self.config.anchor_hop.max(1) as usize;
        let n = source.channel_samples[0].len();
        if window == 0 || n < window + hop {
            return Vec::new();
        }
        let mut offsets: Vec<usize> = (0..=n - window).step_by(hop).collect();
        if offsets.last().copied() != Some(n - window) {
            offsets.push(n - window);
        }
        offsets
            .iter()
            .filter_map(|&w_off| {
                self.best_window_match(source, capture, w_off, window, lo, hi)
                    .map(|(capture, score)| Anchor {
                        source: base + w_off as u64,
                        capture,
                        score,
                    })
            })
            .collect()
    }

    /// Persist one JSON artifact under `dir` and return its record.
    #[allow(clippy::too_many_arguments)]
    fn persist_json(
        &self,
        dir: &std::path::Path,
        name: &str,
        role: &str,
        body: &serde_json::Value,
        lineage: &[al_core::ArtifactId],
        params: serde_json::Value,
        ctx: &StageContext<'_>,
    ) -> Result<ArtifactRecord, StageError> {
        let job_dir = ctx.registry.job_dir();
        fs::create_dir_all(job_dir.join(dir)).map_err(err)?;
        let rel = dir.join(name);
        al_job::fsutil::write_durable(
            &job_dir.join(&rel),
            serde_json::to_string_pretty(body).map_err(err)?.as_bytes(),
        )
        .map_err(err)?;
        Ok(ArtifactRecord {
            artifact_id: ctx.registry.issue(),
            role: role.into(),
            path: rel,
            produced_by: Some(StageKind::Locate),
            inputs: lineage.to_vec(),
            params,
        })
    }
}

/// Method evidence from one (source channel, capture channel) pair.
#[derive(Debug, Clone, serde::Serialize)]
struct PairEvidence {
    source_channel: usize,
    capture_channel: usize,
    #[serde(flatten)]
    evidence: MethodEvidence,
}

/// Keep, per method, the evidence of the strongest channel pair.
/// `pairs` must be grouped by method (see [`LocateStage::locate_channels`]).
fn merged_votes(pairs: &[PairEvidence]) -> Vec<MethodEvidence> {
    let top = |e: &MethodEvidence| e.candidates.first().map(|c| c.score).unwrap_or(0.0);
    let mut merged: Vec<MethodEvidence> = Vec::new();
    for pair in pairs {
        match merged.last_mut() {
            Some(m) if m.method == pair.evidence.method => {
                if top(&pair.evidence) > top(m) {
                    *m = pair.evidence.clone();
                }
            }
            _ => merged.push(pair.evidence.clone()),
        }
    }
    merged
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

        // Decode every source once up front; the residual pass needs
        // them all after the per-pair results exist.
        let mut decoded_sources = Vec::new();
        for src_input in &sources {
            let Some(source) = self.decoded(src_input, ctx)? else {
                return Err(err(format!(
                    "no decoded_pcm artifact for source {}",
                    src_input.input_id
                )));
            };
            decoded_sources.push(source);
        }

        let mut records = Vec::new();
        for cap_input in &captures {
            let Some(capture) = self.decoded(cap_input, ctx)? else {
                return Err(err(format!(
                    "no decoded_pcm artifact for capture {}",
                    cap_input.input_id
                )));
            };
            let capture_len = capture.channel_samples[0].len() as u64;
            let tolerance = self.config.consensus.position_tolerance;
            // Per-source results kept alive for the residual pass;
            // accepted pairs additionally carry their `locate_result`
            // artifact id for lineage.
            let mut results: Vec<Option<LocateResult>> = Vec::new();
            results.resize_with(decoded_sources.len(), || None);
            let mut accepted: Vec<(usize, al_core::ArtifactId)> = Vec::new();
            for (src_index, source) in decoded_sources.iter().enumerate() {
                let dir = PathBuf::from("locate")
                    .join(&source.input_id)
                    .join(&capture.input_id);
                let mut lineage = source.artifact_ids.clone();
                lineage.extend(capture.artifact_ids.iter().cloned());

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
                    records.push(self.persist_json(
                        &dir,
                        "result.json",
                        "locate_result",
                        &serde_json::json!({
                            "result": result,
                            "derived": {"sample_rate_mismatch": true},
                        }),
                        &lineage,
                        serde_json::json!({
                            "source_input_id": result.source_input_id,
                            "capture_input_id": result.capture_input_id,
                            "accepted": result.accepted,
                            "validity": result.validity,
                            "alignment_id": result.alignment_id,
                        }),
                        ctx,
                    )?);
                    continue;
                }

                let pairs = self.locate_channels(
                    &source.channel_samples,
                    &capture.channel_samples,
                    source.sample_rate,
                );
                let evidence = merged_votes(&pairs);
                let source_len = source.channel_samples[0].len() as u64;
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

                // Every channel pair's evidence is its own artifact so
                // per-channel detail survives the vote merge.
                records.push(self.persist_json(
                    &dir,
                    "evidence.json",
                    "locate_evidence",
                    &serde_json::json!({"evidence": pairs}),
                    &lineage,
                    serde_json::json!({
                        "source_input_id": source.input_id,
                        "capture_input_id": capture.input_id,
                        "channel_pairs": pairs.len(),
                    }),
                    ctx,
                )?);

                // Additional occurrences: cluster every usable
                // candidate beyond the primary position by width, so
                // one physical pass contributes a single occurrence
                // and multiple legitimate appearances stay separate.
                let mut occurrences: Vec<u64> = Vec::new();
                let mut scores: std::collections::HashMap<u64, f64> =
                    std::collections::HashMap::new();
                if result.accepted {
                    let mut positions: Vec<u64> = Vec::new();
                    for pair in &pairs {
                        if pair.evidence.validity != Validity::Valid {
                            continue;
                        }
                        for c in &pair.evidence.candidates {
                            if c.score >= self.config.consensus.min_score
                                && c.peak_to_background
                                    >= self.config.consensus.min_peak_to_background
                            {
                                positions.push(c.position);
                                let e = scores.entry(c.position).or_insert(c.score);
                                *e = e.max(c.score);
                            }
                        }
                    }
                    positions.sort_unstable();
                    positions.dedup();
                    let primary = result.capture_range.start;
                    let mut cluster: Vec<u64> = Vec::new();
                    for &p in &positions {
                        if p.abs_diff(primary) <= tolerance {
                            continue;
                        }
                        match cluster.first() {
                            Some(&first) if p - first <= tolerance => cluster.push(p),
                            _ => {
                                if !cluster.is_empty() {
                                    occurrences.push(cluster[(cluster.len() - 1) / 2]);
                                }
                                cluster = vec![p];
                            }
                        }
                    }
                    if !cluster.is_empty() {
                        occurrences.push(cluster[(cluster.len() - 1) / 2]);
                    }
                }

                // Derived data for the primary result: mixer fit on the
                // matched window; loop occurrences and the piecewise
                // time map are their own artifacts below.
                let mut derived = serde_json::json!({"occurrences": occurrences});
                let mut time_map_body = None;
                if result.accepted {
                    let primary = result.capture_range.start;
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

                    // Loop occurrences anchor the pass grid at the
                    // accepted position; time-map anchors then unroll
                    // each pass to `loop_index * body_len` so spacing
                    // deviations surface as real segment rates.
                    let mut occurrence_positions = vec![primary];
                    occurrence_positions.extend(&occurrences);
                    let body_len = source_len.max(1);
                    let loops = identify_loop_occurrences(&occurrence_positions, body_len, primary);
                    let min_index = loops.iter().map(|o| o.loop_index).min().unwrap_or(0);
                    // A source long enough to split into windows gets
                    // one anchor per window, so drift, jitter, skips,
                    // gaps and resumes inside a single occurrence
                    // stay observable. Each occurrence owns the
                    // capture region between the midpoints to its
                    // neighbours; an occurrence is only real when the
                    // body start window matches inside its region —
                    // secondary correlation peaks are not replays.
                    // With one verified occurrence the mapping is
                    // tracked in plain source coordinates; several
                    // verified occurrences are real passes, so their
                    // windows unroll onto the pass grid.
                    let cap_n = capture.channel_samples[0].len();
                    let positions: Vec<u64> = loops.iter().map(|o| o.occurrence_position).collect();
                    let cell_of = |i: usize| -> (usize, usize) {
                        let lo = if i == 0 {
                            0
                        } else {
                            ((positions[i - 1] + positions[i]) / 2) as usize
                        };
                        let hi = if i + 1 == positions.len() {
                            cap_n
                        } else {
                            ((positions[i] + positions[i + 1]) / 2) as usize
                        };
                        (lo, hi)
                    };
                    let window = self.config.anchor_window as usize;
                    let verified: Vec<usize> = (0..loops.len())
                        .filter(|&i| {
                            positions[i] == primary || {
                                let (lo, hi) = cell_of(i);
                                self.best_window_match(source, &capture, 0, window, lo, hi)
                                    .is_some()
                            }
                        })
                        .collect();
                    let mut anchors = if verified.len() <= 1 {
                        self.window_anchors_in(source, &capture, 0, cap_n, 0)
                    } else {
                        let min_v = verified
                            .iter()
                            .map(|&i| loops[i].loop_index)
                            .min()
                            .unwrap_or(0);
                        verified
                            .iter()
                            .flat_map(|&i| {
                                let base = ((loops[i].loop_index - min_v) * body_len as i64) as u64;
                                let (lo, hi) = cell_of(i);
                                self.window_anchors_in(source, &capture, lo, hi, base)
                            })
                            .collect()
                    };
                    if anchors.is_empty() {
                        // Short sources (and sources whose windows all
                        // failed to match) keep one anchor per
                        // occurrence on the unrolled pass grid.
                        for o in &loops {
                            anchors.push(Anchor {
                                source: ((o.loop_index - min_index) * body_len as i64) as u64,
                                capture: o.occurrence_position,
                                score: scores.get(&o.occurrence_position).copied().unwrap_or(0.0),
                            });
                        }
                    }
                    anchors.sort_by_key(|a| a.capture);
                    let time_map = build_time_map(
                        &anchors,
                        self.config.rate_tolerance,
                        self.config.anchor_gap,
                        self.config.min_run,
                    );
                    time_map_body = Some(serde_json::json!({
                        "time_map": time_map,
                        "anchors": anchors,
                        "loop_occurrences": loops,
                        "anchor_reference": primary,
                    }));
                }

                if let Some(body) = time_map_body {
                    records.push(self.persist_json(
                        &dir,
                        "time_map.json",
                        "time_map",
                        &body,
                        &lineage,
                        serde_json::json!({
                            "source_input_id": source.input_id,
                            "capture_input_id": capture.input_id,
                        }),
                        ctx,
                    )?);
                }

                let accepted_result = result.accepted;
                let record = self.persist_json(
                    &dir,
                    "result.json",
                    "locate_result",
                    &serde_json::json!({
                        "result": result,
                        "derived": derived,
                    }),
                    &lineage,
                    serde_json::json!({
                        "source_input_id": result.source_input_id,
                        "capture_input_id": result.capture_input_id,
                        "accepted": result.accepted,
                        "validity": result.validity,
                        "alignment_id": result.alignment_id,
                    }),
                    ctx,
                )?;
                if accepted_result {
                    accepted.push((src_index, record.artifact_id.clone()));
                }
                results[src_index] = Some(result);
                records.push(record);
            }

            // Two-stage residual search: for each accepted alignment,
            // subtract the fitted mix from the capture and re-run the
            // locate methods for every source on the residual —
            // quieter sources and repeat occurrences surface once the
            // dominant contribution is removed. Provenance (parent
            // source, alignment, transform, energies, child result) is
            // persisted per (parent, child) pair.
            for &(parent_idx, ref parent_artifact) in &accepted {
                let parent = &decoded_sources[parent_idx];
                let parent_result = results[parent_idx].as_ref().unwrap();
                let start = parent_result.capture_range.start as usize;
                let span =
                    (parent_result.capture_range.end - parent_result.capture_range.start) as usize;
                if span == 0 {
                    continue;
                }
                let mut aligned =
                    vec![vec![0.0; capture_len as usize]; parent.channel_samples.len()];
                for (a, s) in aligned.iter_mut().zip(&parent.channel_samples) {
                    a[start..start + span].copy_from_slice(&s[..span]);
                }
                for child in &decoded_sources {
                    if child.sample_rate != capture.sample_rate {
                        continue;
                    }
                    let child_len = child.channel_samples[0].len() as u64;
                    let Some(search) = residual_search(
                        &parent.input_id,
                        parent_result.alignment_id.clone(),
                        &aligned,
                        &capture.channel_samples,
                        self.config.ridge,
                        |residual| {
                            let pairs = self.locate_channels(
                                &child.channel_samples,
                                residual,
                                child.sample_rate,
                            );
                            evaluate(
                                &merged_votes(&pairs),
                                &child.input_id,
                                &capture.input_id,
                                child_len,
                                capture_len,
                                &self.config.consensus,
                                "locate-consensus",
                                "1",
                            )
                        },
                    ) else {
                        continue;
                    };
                    let dir = PathBuf::from("locate")
                        .join(&child.input_id)
                        .join(&capture.input_id);
                    let name = format!("residual_{}.json", parent.input_id);
                    let mut lineage = parent.artifact_ids.clone();
                    lineage.extend(capture.artifact_ids.iter().cloned());
                    lineage.extend(child.artifact_ids.iter().cloned());
                    lineage.push(parent_artifact.clone());
                    records.push(self.persist_json(
                        &dir,
                        &name,
                        "residual",
                        &serde_json::to_value(&search).unwrap_or_default(),
                        &lineage,
                        serde_json::json!({
                            "parent_source": search.parent_source,
                            "parent_alignment": search.parent_alignment,
                            "child_source": search.child.source_input_id,
                            "child_accepted": search.child.accepted,
                            "confidence": search.confidence,
                        }),
                        ctx,
                    )?);
                }
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
