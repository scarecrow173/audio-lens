//! Expensive candidate-driven deep analysis.
//!
//! `ExpensiveAnalysisStage` first extracts candidate events from the
//! cheap/reference feature stores into the `events` role, then plans
//! triggered regions (auto candidates / high-residual / low-similarity
//! / abnormal timing / manual ranges / explicit always-run) and runs
//! the expensive analysis families only inside those regions. Every
//! result persists its trigger reasons, and output goes to
//! `features/expensive` — cheap/reference rows are never overwritten.
//!
//! The stored `intermediate` tensors hold the cheap stage's coarse
//! STFT; the expensive families need the fine-resolution window
//! configured here, so spectra are recomputed inside each triggered
//! region rather than upsampled from stored intermediates.
//!
//! The analysis families live in submodules: [`transfer`] (spectral
//! cutoff/narrowband/THD), [`codec`] (echo/pre-echo/block artifacts),
//! [`dynamics`] (level/gain behaviour), [`noise`] (noise
//! decomposition), [`channel`] (cross-channel/mixing), [`ir`]
//! (impulse response) and [`timing`] (SRC/repeat/splice/local time
//! map). This root keeps the config, trigger planning and the
//! `analyze_region` orchestration; each family module owns its
//! feature ids including the invalid-observation schema emitted for
//! non-`Valid` triggers (`emit_invalid`), so a new family registers
//! both paths in one place.

use al_core::{Confidence, Validity};
use al_dataset::types::{ChannelSelector, FeatureRecord, Range, Resolution, ResolutionClass};

use crate::cheap::NamedTensor;
use crate::events::{CandidateEvent, EventKind};
use crate::records::{RecordBuilder, propagate_confidence, propagate_validity};
use crate::stft::{StftMatrix, stft};

pub mod channel;
pub mod codec;
pub mod dynamics;
pub mod ir;
pub mod noise;
pub mod timing;
pub mod transfer;

/// Expensive-stage configuration.
#[derive(Debug, Clone)]
pub struct ExpensiveConfig {
    /// Run on the full input regardless of candidates.
    pub always_run: bool,
    /// Explicit `[start_s, end_s)` regions.
    pub manual_ranges: Vec<(f64, f64)>,
    /// Half-window (seconds) used to expand point events into
    /// analysis regions.
    pub event_window_s: f64,
    /// Fine STFT window (samples) used for deep analysis.
    pub stft_window: usize,
    /// Fine STFT hop.
    pub stft_hop: usize,
}

impl Default for ExpensiveConfig {
    fn default() -> Self {
        Self {
            always_run: false,
            manual_ranges: Vec::new(),
            event_window_s: 0.25,
            stft_window: 8192,
            stft_hop: 2048,
        }
    }
}

impl ExpensiveConfig {
    /// Read `config.analysis.expensive.*`:
    /// `always_run`, `manual_ranges` (`[[s,e], ...]` or
    /// `[{"start_s","end_s"}, ...]`), `event_window_s`, `stft_window`,
    /// `stft_hop`.
    pub fn from_job(config: &serde_json::Value) -> Self {
        let mut c = Self::default();
        let Some(e) = config.get("analysis").and_then(|a| a.get("expensive")) else {
            return c;
        };
        if let Some(v) = e.get("always_run").and_then(|v| v.as_bool()) {
            c.always_run = v;
        }
        if let Some(v) = e.get("event_window_s").and_then(|v| v.as_f64()) {
            c.event_window_s = v.max(0.001);
        }
        if let Some(v) = e.get("stft_window").and_then(|v| v.as_u64()) {
            c.stft_window = (v as usize).clamp(256, 1 << 22);
            c.stft_hop = c.stft_window / 4;
        }
        if let Some(v) = e.get("stft_hop").and_then(|v| v.as_u64()) {
            c.stft_hop = (v as usize).max(1);
        }
        if let Some(list) = e.get("manual_ranges").and_then(|v| v.as_array()) {
            for item in list {
                let range = if let Some(pair) = item.as_array() {
                    match (
                        pair.first().and_then(|v| v.as_f64()),
                        pair.get(1).and_then(|v| v.as_f64()),
                    ) {
                        (Some(a), Some(b)) => Some((a, b)),
                        _ => None,
                    }
                } else {
                    match (
                        item.get("start_s").and_then(|v| v.as_f64()),
                        item.get("end_s").and_then(|v| v.as_f64()),
                    ) {
                        (Some(a), Some(b)) => Some((a, b)),
                        _ => None,
                    }
                };
                if let Some((a, b)) = range
                    && b > a
                {
                    c.manual_ranges.push((a, b));
                }
            }
        }
        c
    }
}

/// One region selected for deep analysis plus the reasons it was
/// selected (persisted into every produced record).
#[derive(Debug, Clone, PartialEq)]
pub struct TriggeredRegion {
    /// Region start in seconds.
    pub start_s: f64,
    /// Region end in seconds (exclusive).
    pub end_s: f64,
    /// Trigger reasons (`candidate:<id>`, `high_residual`,
    /// `low_similarity`, `abnormal_timing`, `manual`, `always_run`).
    pub triggers: Vec<String>,
    /// Weakest validity among the upstream evidence that selected
    /// this region. `manual` / `always_run` are independent triggers
    /// and carry `Valid`; a candidate-selected region keeps the
    /// event's validity so outputs never out-trust their trigger.
    pub validity: Validity,
    /// Weakest confidence among the selecting evidence (`High` for
    /// `manual` / `always_run`).
    pub confidence: Confidence,
    /// Whether at least one `Valid` trigger (`manual`, `always_run`,
    /// or a `Valid` event) selected this region. A region selected
    /// only by non-`Valid` automatic events is recorded as invalid
    /// observations — no estimation runs on untrusted evidence.
    pub has_valid_trigger: bool,
}

/// Map an event kind to its semantic trigger reason. The candidate
/// id is always recorded alongside (`candidate:<id>`) so a merged
/// region stays traceable to the specific event that raised it.
fn event_triggers(e: &CandidateEvent) -> Vec<String> {
    let semantic = match e.kind {
        EventKind::LargeResidual => "high_residual".to_string(),
        EventKind::LowCoherence => "low_similarity".to_string(),
        EventKind::LagJump | EventKind::TransferChangeCandidate => "abnormal_timing".to_string(),
        _ => format!("candidate:{}", e.event_id),
    };
    let id_ref = format!("candidate:{}", e.event_id);
    if semantic == id_ref {
        vec![semantic]
    } else {
        vec![semantic, id_ref]
    }
}

/// Weakest of two upstream validity states: `Valid` loses to any
/// non-`Valid` state — a merged region must not claim more trust
/// than the least trustworthy evidence that selected it.
fn weakest_validity(a: Validity, b: Validity) -> Validity {
    if a == Validity::Valid { b } else { a }
}

/// Plan deep-analysis regions for one input from its events, manual
/// ranges, and always-run flag. Overlapping regions merge and union
/// their trigger reasons; output is sorted and clamped to
/// `[0, duration_s]`. Event-selected regions carry the events'
/// validity/confidence (`manual` / `always_run` count as independent
/// `Valid`/`High` triggers) so downstream never promotes weak
/// evidence into strong claims. `has_valid_trigger` tracks whether
/// any `Valid` evidence selected the merged region — regions without
/// it are analyzed as invalid observations only.
pub fn plan_regions(
    duration_s: f64,
    events: &[CandidateEvent],
    cfg: &ExpensiveConfig,
) -> Vec<TriggeredRegion> {
    let mut regions: Vec<TriggeredRegion> = Vec::new();
    if cfg.always_run {
        regions.push(TriggeredRegion {
            start_s: 0.0,
            end_s: duration_s,
            triggers: vec!["always_run".into()],
            validity: Validity::Valid,
            confidence: Confidence::High,
            has_valid_trigger: true,
        });
    }
    for &(a, b) in &cfg.manual_ranges {
        regions.push(TriggeredRegion {
            start_s: a.max(0.0),
            end_s: b.min(duration_s),
            triggers: vec!["manual".into()],
            validity: Validity::Valid,
            confidence: Confidence::High,
            has_valid_trigger: true,
        });
    }
    let half = cfg.event_window_s;
    for e in events {
        let (a, b) = match e.end_s {
            Some(end) => (e.start_s, end),
            None => (e.start_s - half, e.start_s + half),
        };
        regions.push(TriggeredRegion {
            start_s: a.max(0.0),
            end_s: b.min(duration_s).max(a.max(0.0)),
            triggers: event_triggers(e),
            validity: e.validity,
            confidence: e.confidence,
            has_valid_trigger: e.validity == Validity::Valid,
        });
    }
    // Merge overlapping / abutting regions per input.
    regions.retain(|r| r.end_s > r.start_s);
    regions.sort_by(|a, b| a.start_s.total_cmp(&b.start_s));
    let mut merged: Vec<TriggeredRegion> = Vec::new();
    for r in regions {
        if let Some(last) = merged.last_mut()
            && r.start_s <= last.end_s
        {
            last.end_s = last.end_s.max(r.end_s);
            last.validity = weakest_validity(last.validity, r.validity);
            last.confidence = last.confidence.min(r.confidence);
            last.has_valid_trigger |= r.has_valid_trigger;
            for t in r.triggers {
                if !last.triggers.contains(&t) {
                    last.triggers.push(t);
                }
            }
            continue;
        }
        merged.push(r);
    }
    merged
}

/// Deep-analysis outputs for one region.
#[derive(Debug, Default)]
pub struct ExpensiveResult {
    /// Feature records for `features/expensive/<input>__<region>`.
    pub records: Vec<FeatureRecord>,
    /// Intermediate tensors emitted by families (referenced from
    /// records via `Value::Tensor`).
    pub tensors: Vec<NamedTensor>,
}

pub(crate) fn rms(x: &[f64]) -> f64 {
    if x.is_empty() {
        return 0.0;
    }
    (x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64).sqrt()
}

pub(crate) fn corr0(x: &[f64], y: &[f64]) -> f64 {
    let (ex, ey): (f64, f64) = (x.iter().map(|v| v * v).sum(), y.iter().map(|v| v * v).sum());
    if ex > 0.0 && ey > 0.0 {
        x.iter().zip(y).map(|(&a, &b)| a * b).sum::<f64>() / (ex * ey).sqrt()
    } else {
        0.0
    }
}

/// Record emitter bound to one triggered region. Every family pushes
/// through this so all records share the region's push conventions:
/// the region's time range and trigger params are stamped, confidence
/// never exceeds the trigger's, and a non-`Valid` trigger downgrades
/// each record to the mapped validity with an explanation.
pub(crate) struct Emit<'a> {
    input_id: &'a str,
    region: &'a TriggeredRegion,
    cfg: &'a ExpensiveConfig,
    range: Range,
    /// `propagate_validity(region.validity)` — the mapped validity a
    /// region without trusted evidence must carry.
    pub(crate) trigger_val: Validity,
}

impl<'a> Emit<'a> {
    fn new(input_id: &'a str, region: &'a TriggeredRegion, cfg: &'a ExpensiveConfig) -> Self {
        Self {
            input_id,
            region,
            cfg,
            range: Range {
                start: region.start_s,
                end: region.end_s,
            },
            trigger_val: propagate_validity(region.validity),
        }
    }

    /// The triggered region this emitter stamps onto records —
    /// exposed for families that extend `params` with extra fields.
    pub(crate) fn region(&self) -> &'a TriggeredRegion {
        self.region
    }

    /// Segment-class scalar record for `id`. A `Valid` estimate
    /// inside a region selected by non-`Valid` evidence is demoted
    /// to the mapped trigger validity — untrusted upstream evidence
    /// must not yield `Valid` numbers — and no record may exceed the
    /// confidence of the evidence that selected the region.
    pub(crate) fn push(
        &self,
        id: &str,
        value: f64,
        unit: &str,
        conf: Confidence,
        val: Validity,
        reason: Option<String>,
        support: u64,
    ) -> FeatureRecord {
        let (val, reason) = if val == Validity::Valid && self.trigger_val != Validity::Valid {
            (
                self.trigger_val,
                Some(reason.unwrap_or_else(|| {
                    format!(
                        "selected by non-valid upstream evidence ({:?})",
                        self.region.validity
                    )
                })),
            )
        } else {
            (val, reason)
        };
        let mut r = RecordBuilder::new(self.input_id, "expensive.v1", ResolutionClass::Segment)
            .scalar(
                id,
                value,
                unit,
                Some(self.range),
                propagate_confidence(self.region.confidence, conf),
                val,
                reason,
                support,
            );
        r.channels = Some(ChannelSelector::Channel(0));
        r.params = serde_json::json!({
            "region_s": [self.region.start_s, self.region.end_s],
            "triggers": self.region.triggers,
        });
        r
    }

    /// [`Emit::push`] plus the fine-analysis STFT resolution stamp so
    /// consumers can compare across feature families.
    pub(crate) fn push_spec(
        &self,
        id: &str,
        value: f64,
        unit: &str,
        conf: Confidence,
        val: Validity,
        reason: Option<String>,
        support: u64,
    ) -> FeatureRecord {
        let mut r = self.push(id, value, unit, conf, val, reason, support);
        r.resolution = Some(Resolution {
            window: self.cfg.stft_window as u64,
            hop: self.cfg.stft_hop as u64,
        });
        r
    }
}

/// Source→capture pair evidence for the pair-dependent families
/// (IR deconvolution, level-dependent transfer, band mixing matrix,
/// local time map).
///
/// How the stage locates pair evidence — the same
/// manifest/artifacts pattern `al_locate::decode::decode_input` uses
/// for `decoded_pcm` + `ingest_metadata`:
/// - `ctx.manifest.artifacts` with `role == "locate_result"` define
///   `(source_input_id, capture_input_id, source_range,
///   capture_range, accepted, confidence)`;
/// - the source input's decoded PCM is already in the stage's
///   `decoded` map — `source_range`/`capture_range` slice the aligned
///   view (`aligned_src[c][i]` pairs with capture sample
///   `cap_start + i`);
/// - `intermediate/{src}__{cap}/transfer_h` in the analysis dataset
///   (written by `ReferenceAnalysisStage`) supplies the
///   Welch-averaged H(f) when the job ran reference analysis.
///
/// `ExpensiveAnalysisStage` deliberately does NOT list `Locate` or
/// `ReferenceAnalysis` in `depends_on`: al-job validates that every
/// declared dependency appears earlier in the stage plan, so a hard
/// dependency would make pair-free jobs fail plan validation instead
/// of merely degrading these families to `pair evidence
/// unavailable` invalid records.
#[derive(Debug)]
pub struct PairEvidence<'a> {
    /// `{source_input_id}__{capture_input_id}` dataset path stem.
    pub pair: String,
    /// Alignment id issued by the locate result (stamped on
    /// pair-derived records).
    pub alignment_id: String,
    /// Whether consensus accepted this alignment. Rejected results
    /// are still evidence — families emit invalid records mapped
    /// from `validity` rather than pretending no locate ran.
    pub accepted: bool,
    /// Validity carried by the locate result (`Valid` when accepted,
    /// `AmbiguousAlignment` otherwise).
    pub validity: Validity,
    /// Locate confidence — pair-derived records never out-trust it.
    pub confidence: Confidence,
    /// Aligned source channels: `aligned_src[c][i]` pairs with
    /// capture sample `cap_start + i`. Empty when `!accepted`.
    pub aligned_src: Vec<&'a [f64]>,
    /// Capture sample index paired with `aligned_src[·][0]`.
    pub cap_start: usize,
    /// Aligned length in samples.
    pub len: usize,
    /// Welch-averaged `H(f) = Pxy/Pxx` as `(re, im)` per bin when the
    /// reference stage persisted `intermediate/{pair}/transfer_h`.
    pub transfer_h: Option<Vec<(f64, f64)>>,
}

impl<'a> PairEvidence<'a> {
    /// Source slices aligned with capture samples `[cap_s0, cap_s1)`.
    /// Returns the overlapping capture range `(o0, o1)` plus one
    /// slice per source channel. `None` when the alignment was
    /// rejected or the ranges do not overlap.
    pub fn aligned_slice(
        &self,
        cap_s0: usize,
        cap_s1: usize,
    ) -> Option<(usize, usize, Vec<&'a [f64]>)> {
        if !self.accepted || self.aligned_src.is_empty() {
            return None;
        }
        let o0 = cap_s0.max(self.cap_start);
        let o1 = cap_s1.min(self.cap_start + self.len);
        if o1 <= o0 {
            return None;
        }
        let (i0, i1) = (o0 - self.cap_start, o1 - self.cap_start);
        Some((
            o0,
            o1,
            self.aligned_src.iter().map(|c| &c[i0..i1]).collect(),
        ))
    }
}

/// Per-region analysis context shared by the family modules: the
/// region's channel slices, the fine-resolution STFT of channel 0
/// and its derived spectrum, and the optional pair evidence.
pub(crate) struct RegionCtx<'a> {
    /// Dataset artifact name of this region (`<input>__rNNN`), used
    /// to name emitted intermediate tensors.
    pub region_name: &'a str,
    /// Input sample rate.
    pub rate: u32,
    /// Region start/end in capture samples (channel-0 timeline).
    pub s0: usize,
    pub s1: usize,
    /// All input channels at full length (capture timeline).
    pub chans: &'a [&'a [f64]],
    /// Region slice of each channel (clamped to channel length).
    pub segs: Vec<&'a [f64]>,
    /// Fine STFT of channel 0's region slice.
    pub stft: StftMatrix,
    /// Frame-mean power spectrum (`stft.fft_size` bins).
    pub mean_power: Vec<f64>,
    /// Hz per FFT bin.
    pub bin_hz: f64,
    /// Sum over `mean_power` (all bins).
    pub total_power: f64,
    /// Sum over the one-sided half of `mean_power`.
    pub one_sided_power: f64,
    /// Anomaly dictionary when the job config supplied one.
    pub dictionary: Option<&'a al_media::Dictionary>,
    /// Source→capture pair evidence for pair-dependent families.
    pub pair: Option<&'a PairEvidence<'a>>,
}

/// Run the expensive analysis families over one region of one input.
/// `chans` are the input's decoded channels; the region selects the
/// sample range. `region_name` names the dataset artifact (and any
/// intermediate tensors). `triggers` are persisted into every
/// record's params. `pair` carries optional source→capture evidence
/// for the pair-dependent families — absent or rejected pair
/// evidence yields `pair evidence unavailable` invalid records
/// rather than skipped features.
///
/// A region without a `Valid` trigger (`has_valid_trigger == false`)
/// skips estimation entirely: each feature is persisted as an
/// invalid observation (NaN scalar + mapped validity), per the
/// plan's rule that non-`Valid` upstream evidence yields no numbers.
#[allow(clippy::too_many_arguments)]
pub fn analyze_region(
    input_id: &str,
    region_name: &str,
    chans: &[&[f64]],
    rate: u32,
    region: &TriggeredRegion,
    dictionary: Option<&al_media::Dictionary>,
    pair: Option<&PairEvidence<'_>>,
    cfg: &ExpensiveConfig,
) -> ExpensiveResult {
    let mut out = ExpensiveResult::default();
    let e = Emit::new(input_id, region, cfg);
    // Plan contract: when no `Valid` trigger selected the region the
    // upstream evidence is untrusted, so no estimation runs — every
    // feature is persisted as an invalid observation (NaN scalar,
    // mapped validity) rather than a finite number. Each family
    // emits its own feature-id list so the persisted schema always
    // mirrors what the valid path would produce.
    if !region.has_valid_trigger {
        let reason = format!(
            "selected by non-valid upstream evidence ({:?})",
            region.validity
        );
        out.records.push(e.push(
            "exp.region_support",
            f64::NAN,
            "samples",
            Confidence::High,
            e.trigger_val,
            Some(reason.clone()),
            0,
        ));
        transfer::emit_invalid(&e, &mut out.records, &reason, dictionary.is_some());
        codec::emit_invalid(&e, &mut out.records, &reason);
        dynamics::emit_invalid(&e, &mut out.records, &reason);
        noise::emit_invalid(&e, &mut out.records, &reason);
        channel::emit_invalid(&e, &mut out.records, &reason, chans.len());
        ir::emit_invalid(&e, &mut out.records, &reason);
        timing::emit_invalid(&e, &mut out.records, &reason);
        return out;
    }
    let Some(mono) = chans.first().copied() else {
        out.records.push(e.push(
            "exp.region_support",
            f64::NAN,
            "samples",
            Confidence::High,
            Validity::InsufficientSupport,
            Some("no decoded channels for input".into()),
            0,
        ));
        return out;
    };
    let s0 = (region.start_s * rate as f64) as usize;
    let s1 = ((region.end_s * rate as f64) as usize).min(mono.len());
    if s1 <= s0 || s1 - s0 < cfg.stft_window / 2 {
        out.records.push(e.push(
            "exp.region_support",
            f64::NAN,
            "samples",
            Confidence::High,
            Validity::InsufficientSupport,
            Some("region shorter than minimum analysis window".into()),
            s1.saturating_sub(s0) as u64,
        ));
        return out;
    }
    let seg = &mono[s0..s1];
    let n = seg.len();

    // Mean power spectrum + PSD at fine resolution.
    let m = stft(seg, cfg.stft_window, cfg.stft_hop, rate);
    let bins = m.fft_size;
    let half = bins / 2;
    let frames = m.frames();
    if frames == 0 {
        out.records.push(e.push(
            "exp.region_support",
            f64::NAN,
            "samples",
            Confidence::High,
            Validity::InsufficientSupport,
            Some("STFT produced no frames in region".into()),
            n as u64,
        ));
        return out;
    }
    let power = m.power();
    let mean_power: Vec<f64> = (0..bins)
        .map(|k| power.iter().map(|f| f[k]).sum::<f64>() / frames as f64)
        .collect();
    let bin_hz = rate as f64 / bins as f64;
    let total_power: f64 = mean_power.iter().sum();
    let one_sided_power: f64 = mean_power[..half].iter().sum();

    let segs: Vec<&[f64]> = chans
        .iter()
        .map(|c| &c[s0..s1.min(c.len())])
        .collect();
    let ctx = RegionCtx {
        region_name,
        rate,
        s0,
        s1,
        chans,
        segs,
        stft: m,
        mean_power,
        bin_hz,
        total_power,
        one_sided_power,
        dictionary,
        pair,
    };

    transfer::run(&ctx, &e, &mut out);
    codec::run(&ctx, &e, &mut out);
    dynamics::run(&ctx, &e, &mut out);
    noise::run(&ctx, &e, &mut out);
    channel::run(&ctx, &e, &mut out);
    ir::run(&ctx, &e, &mut out);
    timing::run(&ctx, &e, &mut out);

    out
}

/// Which reference/cheap artifacts feed candidate extraction.
pub const CANDIDATE_ROLES: &[&str] = &["features/cheap", "features/reference"];

/// Deserialize feature records stored by the cheap/reference stages
/// (`{"records": [...]}`). A malformed `records` field fails
/// explicitly — silently returning an empty list would turn data
/// corruption into "no candidates" (pre-release policy: fail, never
/// guess).
pub fn parse_feature_records(body: &serde_json::Value) -> std::io::Result<Vec<FeatureRecord>> {
    let Some(records) = body.get("records") else {
        return Ok(Vec::new());
    };
    serde_json::from_value(records.clone())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
}

/// Re-export for the stage: event extraction + config.
pub use crate::events::{EventConfig as CandidateConfig, extract_candidates};

#[cfg(test)]
mod tests;
