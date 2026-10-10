//! `LocateStage`: pairs `source_asset` inputs with `capture_*`
//! inputs, runs the four locate methods per channel, evaluates
//! consensus, fits the mix, identifies occurrences, builds the
//! piecewise time map, runs the two-stage residual search, and
//! persists every result — accepted and rejected alike.
//!
//! 素材はキャプチャのレートに正規化し、比較した PCM を永続化する。
//! A pair whose decoded PCM is
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
use crate::dsp::ncc;
use crate::methods::{CoarseSearch, FullNcc, default_methods, peaks_from_correlation};
use crate::mixer::fit_mix;
use crate::normalize::prepare_input;
use crate::prepared::{InputCache, PreparedChannel};
use crate::residual::{assemble_residual_search, prepare_residual};
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

fn err(e: impl std::fmt::Display) -> StageError {
    StageError::new(StageKind::Locate, e.to_string())
}

fn prepare_channels(
    cache: &mut InputCache,
    input: &DecodedInput,
    version: &str,
) -> Result<Vec<std::sync::Arc<PreparedChannel>>, &'static str> {
    input
        .channel_samples
        .iter()
        .enumerate()
        .map(|(channel, samples)| {
            cache.prepare(
                &input.input_id,
                channel,
                input.sample_rate,
                version,
                samples,
            )
        })
        .collect()
}

impl LocateStage {
    /// Explicit non-accepted result for a pair that cannot be
    /// evaluated (missing/empty decode).
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
            normalized_source_range: SampleRange { start: 0, end: 0 },
            source_pcm: None,
            capture_pcm: None,
            evidence: vec![],
            agreeing_methods: 0,
            position_spread: 0,
            confidence: Confidence::Low,
            validity,
            reject_reason: Some(reason.into()),
            accepted: false,
            algorithm_id: "locate-consensus".into(),
            algorithm_version: "1".into(),
            params: serde_json::to_value(&self.config).unwrap_or_default(),
        }
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
        prepared_source: &[std::sync::Arc<PreparedChannel>],
        prepared_capture: &[std::sync::Arc<PreparedChannel>],
    ) -> Vec<PairEvidence> {
        // Tasks are (method, source_channel, capture_channel) tuples,
        // method-major so the result order matches the sequential
        // declaration order that merged_votes expects.
        let n_src = source_channels.len();
        let n_cap = capture_channels.len();
        let n_methods = default_methods().len();
        let mut tasks = Vec::with_capacity(n_methods * n_src * n_cap);
        for m in 0..n_methods {
            for (src_ch, prepared_template) in source_channels.iter().zip(prepared_source) {
                for (cap_ch, prepared_signal) in capture_channels.iter().zip(prepared_capture) {
                    tasks.push(MethodTask {
                        method: m,
                        template: src_ch,
                        signal: cap_ch,
                        rate: sample_rate,
                        prepared_template,
                        prepared_signal,
                    });
                }
            }
        }
        run_method_tasks(&tasks)
            .into_iter()
            .enumerate()
            .map(|(idx, output)| PairEvidence {
                source_channel: (idx / n_cap) % n_src,
                capture_channel: idx % n_cap,
                evidence: output.evidence,
                coarse_search: output.coarse_search,
            })
            .collect()
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

    /// One anchor per source window, located inside `capture[lo..hi]`
    /// around its expected position. `origin` is the capture position
    /// where the occurrence's body start lands; each window's search
    /// range is `origin + w_off` widened by the accumulated rate
    /// drift, position tolerance and gap slack — searching the whole
    /// capture per window is both costlier and less correct, since a
    /// few-hundred-sample window can false-match anywhere. Anchor
    /// source coordinates are `base + w_off` so callers can place
    /// them on the unrolled pass grid; windows whose peak stays below
    /// the consensus thresholds contribute no anchor. Returns an
    /// empty list when the source fits fewer than two windows.
    fn window_anchors_in(
        &self,
        source: &DecodedInput,
        capture: &DecodedInput,
        lo: usize,
        hi: usize,
        base: u64,
        origin: u64,
    ) -> Vec<Anchor> {
        let window = self.config.anchor_window as usize;
        let hop = self.config.anchor_hop.max(1) as usize;
        let n = source.channel_samples[0].len();
        if window == 0 || n < window + hop {
            return Vec::new();
        }
        // Bound the window count for long sources: a dense grid over
        // tens of seconds of audio buys no extra map resolution.
        const MAX_WINDOWS: usize = 512;
        let hop = hop.max((n - window) / MAX_WINDOWS);
        let mut offsets: Vec<usize> = (0..=n - window).step_by(hop).collect();
        if offsets.last().copied() != Some(n - window) {
            offsets.push(n - window);
        }
        let tol = self.config.consensus.position_tolerance;
        let gap = self.config.anchor_gap;
        let rate_tol = self.config.rate_tolerance;
        run_pool(offsets.len(), &|i| {
            let w_off = offsets[i];
            let slack = (w_off as f64 * rate_tol) as u64 + tol + gap;
            let expected = origin + w_off as u64;
            let w_lo = expected.saturating_sub(slack).max(lo as u64) as usize;
            let w_hi = (expected + window as u64 + slack).min(hi as u64) as usize;
            // Narrow first; widen ×8 so jumps still land before paying
            // for the caller's whole range, which misses and large
            // deviations still reach.
            self.best_window_match(source, capture, w_off, window, w_lo, w_hi)
                .or_else(|| {
                    let slack8 = slack.saturating_mul(8);
                    let m_lo = expected.saturating_sub(slack8).max(lo as u64) as usize;
                    let m_hi = (expected + window as u64 + slack8).min(hi as u64) as usize;
                    ((m_lo, m_hi) != (w_lo, w_hi))
                        .then(|| self.best_window_match(source, capture, w_off, window, m_lo, m_hi))
                        .flatten()
                })
                .or_else(|| self.best_window_match(source, capture, w_off, window, lo, hi))
                .map(|(capture, score)| Anchor {
                    source: base + w_off as u64,
                    capture,
                    score,
                })
        })
        .into_iter()
        .flatten()
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
    /// チャンネルペア共通の候補提案を FullNcc 行に一度だけ記録する。
    #[serde(skip_serializing_if = "Option::is_none")]
    coarse_search: Option<CoarseSearch>,
}

struct MethodTask<'a> {
    method: usize,
    template: &'a [f64],
    signal: &'a [f64],
    rate: u32,
    prepared_template: &'a PreparedChannel,
    prepared_signal: &'a PreparedChannel,
}

struct MethodOutput {
    evidence: MethodEvidence,
    coarse_search: Option<CoarseSearch>,
}

/// Run `n_tasks` independent units of work over a worker pool,
/// collecting one result per task in input order.
fn run_pool<T: Send>(n_tasks: usize, work: &(dyn Fn(usize) -> T + Sync)) -> Vec<T> {
    let slots: Vec<std::sync::Mutex<Option<T>>> =
        (0..n_tasks).map(|_| std::sync::Mutex::new(None)).collect();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(n_tasks);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let idx = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if idx >= n_tasks {
                        return;
                    }
                    *slots[idx].lock().unwrap() = Some(work(idx));
                }
            });
        }
    });
    slots
        .into_iter()
        .map(|s| s.into_inner().unwrap().unwrap())
        .collect()
}

/// Run `(method_index, template, signal, sample_rate)` tasks over a
/// worker pool, returning one [`MethodEvidence`] per task in input
/// order. Locate methods are pure but not `Send`, so each worker
/// builds its own method set and only evidence crosses the thread
/// boundary.
fn run_method_tasks(tasks: &[MethodTask<'_>]) -> Vec<MethodOutput> {
    run_pool(tasks.len(), &|idx| {
        let methods = default_methods();
        let task = &tasks[idx];
        if methods[task.method].id() == "full_ncc" {
            let (evidence, coarse_search) = FullNcc.locate_coarse_to_fine(
                task.template,
                task.signal,
                task.rate,
                task.prepared_template,
                task.prepared_signal,
            );
            return MethodOutput {
                evidence,
                coarse_search,
            };
        }
        // F3: 候補提案の成否に関係なく、証拠は全レート・元の全探索域から生成。
        let evidence = methods[task.method].locate_prepared(
            task.template,
            task.signal,
            task.rate,
            task.prepared_template,
            task.prepared_signal,
        );
        MethodOutput {
            evidence,
            coarse_search: None,
        }
    })
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
        // Nothing to pair (capture-only or source-only job) →
        // complete with empty output so the pipeline can continue to
        // capture-only analysis.
        if sources.is_empty() || captures.is_empty() {
            return Ok(Vec::new());
        }

        // Decode every input once up front; the residual pass needs
        // all sources after the per-pair results exist. A missing
        // decode becomes a per-pair mark below, never an abort for
        // the whole stage.
        let mut decoded_sources = Vec::with_capacity(sources.len());
        for input in &sources {
            decoded_sources.push(decode_input(input, ctx).map_err(err)?);
        }
        let mut decoded_captures = Vec::with_capacity(captures.len());
        for input in &captures {
            decoded_captures.push(decode_input(input, ctx).map_err(err)?);
        }

        let mut records = Vec::new();
        let mut dsp_inputs = InputCache::default();
        for (cap_index, cap_input) in captures.iter().enumerate() {
            // A capture with missing or empty decoded PCM marks every
            // source pair; it never aborts the stage.
            let capture_mark = match &decoded_captures[cap_index] {
                None => Some((
                    Validity::DecodeInputUnavailable,
                    format!("no decoded_pcm artifact for capture {}", cap_input.input_id),
                )),
                Some(c)
                    if c.channel_samples.is_empty()
                        || c.channel_samples.iter().all(|ch| ch.is_empty()) =>
                {
                    Some((
                        Validity::InsufficientSupport,
                        "decoded PCM is empty".to_string(),
                    ))
                }
                Some(_) => None,
            };
            if let Some((validity, reason)) = capture_mark {
                for (src_index, src_input) in sources.iter().enumerate() {
                    let dir = PathBuf::from("locate")
                        .join(src_input.input_id.to_string())
                        .join(cap_input.input_id.to_string());
                    let lineage = decoded_sources[src_index]
                        .as_ref()
                        .map(|s| s.artifact_ids.clone())
                        .unwrap_or_default();
                    let result = self.marked_result(
                        &src_input.input_id.to_string(),
                        &cap_input.input_id.to_string(),
                        validity,
                        reason.clone(),
                    );
                    records.push(self.persist_json(
                        &dir,
                        "result.json",
                        "locate_result",
                        &serde_json::json!({"result": result, "derived": {}}),
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
                }
                continue;
            }
            let capture_original = decoded_captures[cap_index].as_ref().unwrap();
            let prepared_capture = prepare_input(
                capture_original,
                capture_original.sample_rate,
                &mut records,
                ctx,
            )
            .map_err(err)?;
            let capture = &prepared_capture.decoded;
            let capture_dsp =
                prepare_channels(&mut dsp_inputs, capture, "original").map_err(err)?;
            let prepared_sources = decoded_sources
                .iter()
                .map(|s| {
                    s.as_ref()
                        .map(|s| prepare_input(s, capture.sample_rate, &mut records, ctx))
                        .transpose()
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(err)?;
            let decoded_sources: Vec<Option<&DecodedInput>> = prepared_sources
                .iter()
                .map(|s| s.as_ref().map(|p| &p.decoded))
                .collect();
            let capture_len = capture.channel_samples[0].len() as u64;
            let tolerance = self.config.consensus.position_tolerance;
            // Per-source results kept alive for the residual pass;
            // accepted pairs additionally carry their `locate_result`
            // artifact id for lineage.
            let mut results: Vec<Option<LocateResult>> = Vec::new();
            results.resize_with(decoded_sources.len(), || None);
            let mut accepted: Vec<(usize, al_core::ArtifactId)> = Vec::new();
            for (src_index, src_input) in sources.iter().enumerate() {
                let dir = PathBuf::from("locate")
                    .join(src_input.input_id.to_string())
                    .join(cap_input.input_id.to_string());
                let mut lineage = decoded_sources[src_index]
                    .as_ref()
                    .map(|s| s.artifact_ids.clone())
                    .unwrap_or_default();
                lineage.extend(capture.artifact_ids.iter().cloned());

                // Missing or empty decode → per-pair explicit mark;
                // One bad pair never aborts the others.
                let mark = match &decoded_sources[src_index] {
                    None => Some((
                        Validity::DecodeInputUnavailable,
                        format!("no decoded_pcm artifact for source {}", src_input.input_id),
                        serde_json::json!({}),
                    )),
                    Some(s)
                        if s.channel_samples.is_empty()
                            || s.channel_samples.iter().all(|c| c.is_empty()) =>
                    {
                        Some((
                            Validity::InsufficientSupport,
                            "decoded PCM is empty".to_string(),
                            serde_json::json!({"empty_input": true}),
                        ))
                    }
                    Some(_) => None,
                };
                if let Some((validity, reason, derived)) = mark {
                    let result = self.marked_result(
                        &src_input.input_id.to_string(),
                        &cap_input.input_id.to_string(),
                        validity,
                        reason,
                    );
                    records.push(self.persist_json(
                        &dir,
                        "result.json",
                        "locate_result",
                        &serde_json::json!({"result": result, "derived": derived}),
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
                    results[src_index] = Some(result);
                    continue;
                }
                let source = decoded_sources[src_index].as_ref().unwrap();
                let source_dsp =
                    prepare_channels(&mut dsp_inputs, source, "original").map_err(err)?;

                let pairs = self.locate_channels(
                    &source.channel_samples,
                    &capture.channel_samples,
                    source.sample_rate,
                    &source_dsp,
                    &capture_dsp,
                );
                let evidence = merged_votes(&pairs);
                let source_len = source.channel_samples[0].len() as u64;
                let mut result = evaluate(
                    &evidence,
                    &source.input_id,
                    &capture.input_id,
                    source_len,
                    capture_len,
                    &self.config.consensus,
                    "locate-consensus",
                    "1",
                );

                prepared_sources[src_index]
                    .as_ref()
                    .unwrap()
                    .attach(&prepared_capture, &mut result)
                    .map_err(err)?;

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
                let mut derived = serde_json::json!({"occurrences": occurrences, "resampled": prepared_sources[src_index].as_ref().unwrap().params.original_sample_rate != capture.sample_rate});
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
                    // Sorted order so each occurrence's midpoint cell
                    // covers real capture regions — the primary can
                    // land anywhere in the sorted list.
                    occurrence_positions.sort_unstable();
                    let body_len = source_len.max(1);
                    let loops = identify_loop_occurrences(&occurrence_positions, body_len, primary);
                    let min_index = loops.iter().map(|o| o.loop_index).min().unwrap_or(0);
                    // An occurrence is only real when the body start
                    // window matches at its own position (within
                    // position tolerance); secondary correlation
                    // peaks are not replays. Unverified candidates
                    // are dropped before the pass grid and the
                    // reported occurrences are built.
                    let cap_n = capture.channel_samples[0].len();
                    let detected: Vec<u64> = loops.iter().map(|o| o.occurrence_position).collect();
                    let cell_of = |positions: &[u64], i: usize| -> (usize, usize) {
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
                    // The verification template is the body start —
                    // clamped to the body so sources shorter than the
                    // anchor window still verify whole-body replays.
                    let window =
                        (self.config.anchor_window as usize).min(source.channel_samples[0].len());
                    let tol = tolerance as usize;
                    let verified: Vec<usize> = (0..loops.len())
                        .filter(|&i| {
                            detected[i] == primary || {
                                let pos = detected[i] as usize;
                                let lo = pos.saturating_sub(tol);
                                let hi = (pos + window + tol).min(cap_n);
                                self.best_window_match(source, capture, 0, window, lo, hi)
                                    .is_some()
                            }
                        })
                        .collect();
                    let loops: Vec<_> = verified.iter().map(|&i| loops[i].clone()).collect();
                    let positions: Vec<u64> = loops.iter().map(|o| o.occurrence_position).collect();
                    // Reported occurrences are verified replays only
                    // (every position but the primary).
                    occurrences = positions
                        .iter()
                        .copied()
                        .filter(|&p| p != primary)
                        .collect();
                    derived["occurrences"] = serde_json::json!(occurrences);

                    // A source long enough to split into windows gets
                    // one anchor per window, so drift, jitter, skips,
                    // gaps and resumes inside a single occurrence
                    // stay observable. With one verified occurrence
                    // the mapping is tracked in plain source
                    // coordinates; several verified occurrences are
                    // real passes, so their windows unroll onto the
                    // pass grid.
                    let mut anchors = if loops.len() <= 1 {
                        self.window_anchors_in(source, capture, 0, cap_n, 0, primary)
                    } else {
                        let min_v = loops.iter().map(|o| o.loop_index).min().unwrap_or(0);
                        loops
                            .iter()
                            .enumerate()
                            .flat_map(|(i, o)| {
                                let base = ((o.loop_index - min_v) * body_len as i64) as u64;
                                let (lo, hi) = cell_of(&positions, i);
                                self.window_anchors_in(source, capture, lo, hi, base, positions[i])
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
                        "source_coordinate_space": "normalized_pcm",
                        "source_pcm": result.source_pcm,
                        "capture_pcm": result.capture_pcm,
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
                let Some(parent) = &decoded_sources[parent_idx] else {
                    continue;
                };
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
                // The residual depends only on the parent — compute it
                // once, then run every child's method tasks over it in
                // one shared pool.
                let Some(prepared) =
                    prepare_residual(&aligned, &capture.channel_samples, self.config.ridge)
                else {
                    continue;
                };
                let children: Vec<&DecodedInput> = decoded_sources
                    .iter()
                    .flatten()
                    .copied()
                    .filter(|c| c.sample_rate == capture.sample_rate)
                    .collect();
                let n_methods = default_methods().len();
                let n_cap = prepared.channels.len();
                // 親ごとに版を分離し、この残差の全 child が完了した時点で解放する。
                let version = format!("residual:{}", parent_result.alignment_id);
                let mut residual_cache = InputCache::default();
                let residual_dsp = prepared
                    .channels
                    .iter()
                    .enumerate()
                    .map(|(channel, samples)| {
                        residual_cache.prepare(
                            &capture.input_id,
                            channel,
                            capture.sample_rate,
                            &version,
                            samples,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(err)?;
                let child_dsp = children
                    .iter()
                    .map(|child| prepare_channels(&mut dsp_inputs, child, "original"))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(err)?;
                let mut tasks = Vec::new();
                for (child, prepared_child) in children.iter().zip(&child_dsp) {
                    for m in 0..n_methods {
                        for (src_ch, prepared_template) in
                            child.channel_samples.iter().zip(prepared_child)
                        {
                            for (cap_ch, prepared_signal) in
                                prepared.channels.iter().zip(&residual_dsp)
                            {
                                tasks.push(MethodTask {
                                    method: m,
                                    template: src_ch,
                                    signal: cap_ch,
                                    rate: child.sample_rate,
                                    prepared_template,
                                    prepared_signal,
                                });
                            }
                        }
                    }
                }
                let mut evidences = run_method_tasks(&tasks).into_iter();
                for child in children {
                    let n_src = child.channel_samples.len();
                    let pairs: Vec<PairEvidence> = (0..n_methods * n_src * n_cap)
                        .map(|idx| {
                            let output = evidences.next().unwrap();
                            PairEvidence {
                                source_channel: (idx / n_cap) % n_src,
                                capture_channel: idx % n_cap,
                                evidence: output.evidence,
                                coarse_search: output.coarse_search,
                            }
                        })
                        .collect();
                    let child_len = child.channel_samples[0].len() as u64;
                    let mut child_result = evaluate(
                        &merged_votes(&pairs),
                        &child.input_id,
                        &capture.input_id,
                        child_len,
                        capture_len,
                        &self.config.consensus,
                        "locate-consensus",
                        "1",
                    );
                    child_result.params["coarse_search"] = serde_json::to_value(
                        pairs
                            .iter()
                            .filter_map(|pair| {
                                pair.coarse_search.as_ref().map(|search| {
                                    serde_json::json!({"source_channel": pair.source_channel,
                                "capture_channel": pair.capture_channel, "search": search})
                                })
                            })
                            .collect::<Vec<_>>(),
                    )
                    .map_err(err)?;
                    let prepared_child = prepared_sources
                        .iter()
                        .flatten()
                        .find(|p| p.decoded.input_id == child.input_id)
                        .unwrap();
                    prepared_child
                        .attach(&prepared_capture, &mut child_result)
                        .map_err(err)?;
                    let search = assemble_residual_search(
                        &parent.input_id,
                        parent_result.alignment_id.clone(),
                        &prepared,
                        child_result,
                    );
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
            dsp_inputs.release(&capture.input_id, "original");
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
