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

use al_core::{Confidence, Validity};
use al_dataset::types::{ChannelSelector, FeatureRecord, Range, Resolution, ResolutionClass};

use crate::events::{CandidateEvent, EventKind};
use crate::records::{RecordBuilder, propagate_confidence, propagate_validity, robust_summary};
use crate::stft::{csd, stft, third_octave_bands};

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
}

fn rms(x: &[f64]) -> f64 {
    if x.is_empty() {
        return 0.0;
    }
    (x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64).sqrt()
}

fn corr0(x: &[f64], y: &[f64]) -> f64 {
    let (ex, ey): (f64, f64) = (x.iter().map(|v| v * v).sum(), y.iter().map(|v| v * v).sum());
    if ex > 0.0 && ey > 0.0 {
        x.iter().zip(y).map(|(&a, &b)| a * b).sum::<f64>() / (ex * ey).sqrt()
    } else {
        0.0
    }
}

/// Run the expensive analysis families over one region of one input.
/// `chans` are the input's decoded channels; the region selects the
/// sample range. `triggers` are persisted into every record's params.
/// A region without a `Valid` trigger (`has_valid_trigger == false`)
/// skips estimation entirely: each feature is persisted as an
/// invalid observation (NaN scalar + mapped validity), per the
/// plan's rule that non-`Valid` upstream evidence yields no numbers.
pub fn analyze_region(
    input_id: &str,
    chans: &[&[f64]],
    rate: u32,
    region: &TriggeredRegion,
    dictionary: Option<&al_media::Dictionary>,
    cfg: &ExpensiveConfig,
) -> ExpensiveResult {
    let mut out = ExpensiveResult::default();
    let rb = |class: ResolutionClass| RecordBuilder::new(input_id, "expensive.v1", class);
    let range = Range {
        start: region.start_s,
        end: region.end_s,
    };
    // A region selected by non-`Valid` upstream evidence must not
    // yield `Valid` estimates, and no record may exceed the
    // confidence of the evidence that selected it.
    let trigger_val = propagate_validity(region.validity);
    let push = |id: &str,
                value: f64,
                unit: &str,
                conf: Confidence,
                val: Validity,
                reason: Option<String>,
                support: u64| {
        let (val, reason) = if val == Validity::Valid && trigger_val != Validity::Valid {
            (
                trigger_val,
                Some(reason.unwrap_or_else(|| {
                    format!(
                        "selected by non-valid upstream evidence ({:?})",
                        region.validity
                    )
                })),
            )
        } else {
            (val, reason)
        };
        let mut r = rb(ResolutionClass::Segment).scalar(
            id,
            value,
            unit,
            Some(range),
            propagate_confidence(region.confidence, conf),
            val,
            reason,
            support,
        );
        r.channels = Some(ChannelSelector::Channel(0));
        r.params = serde_json::json!({
            "region_s": [region.start_s, region.end_s],
            "triggers": region.triggers,
        });
        r
    };
    // STFT-derived records additionally declare the fine-analysis
    // resolution so consumers can compare across feature families.
    let push_spec = |id: &str,
                     value: f64,
                     unit: &str,
                     conf: Confidence,
                     val: Validity,
                     reason: Option<String>,
                     support: u64| {
        let mut r = push(id, value, unit, conf, val, reason, support);
        r.resolution = Some(Resolution {
            window: cfg.stft_window as u64,
            hop: cfg.stft_hop as u64,
        });
        r
    };
    // Plan contract: when no `Valid` trigger selected the region the
    // upstream evidence is untrusted, so no estimation runs — every
    // feature is persisted as an invalid observation (NaN scalar,
    // mapped validity) rather than a finite number. The feature list
    // mirrors the families computed below so consumers always see
    // the same schema per region.
    if !region.has_valid_trigger {
        let reason = || {
            Some(format!(
                "selected by non-valid upstream evidence ({:?})",
                region.validity
            ))
        };
        for (id, unit) in [
            ("exp.region_support", "samples"),
            ("exp.polarity_asymmetry", "ratio"),
            ("exp.dynamic_range_db", "db"),
            ("exp.attack_db_per_20ms", "db"),
            ("exp.release_db_per_20ms", "db"),
            ("exp.repeat_block_score", "corr"),
            ("exp.gap_count", "count"),
            ("exp.codec_block_score", "ratio"),
            ("exp.codec_block_size", "samples"),
        ] {
            out.records.push(push(
                id,
                f64::NAN,
                unit,
                Confidence::High,
                trigger_val,
                reason(),
                0,
            ));
        }
        let mut spectral = vec![
            ("exp.cutoff_hz", "hz"),
            ("exp.narrowband_peak_count", "count"),
            ("exp.narrowband_notch_count", "count"),
            ("exp.echo_delay_s", "seconds"),
            ("exp.cepstrum_prominence", "ratio"),
            ("exp.src_image_score", "corr"),
            ("exp.src_candidate_rate", "hz"),
            ("exp.thd_ratio", "ratio"),
            ("exp.spectral_hole_ratio", "ratio"),
            ("exp.tonal_peak_count", "count"),
            ("exp.hum_harmonic_count", "count"),
        ];
        if dictionary.is_some() {
            spectral.insert(1, ("exp.dictionary_cutoff_delta_khz", "khz"));
        }
        if chans.len() >= 2 {
            spectral.push(("exp.freq_lr_coherence", "coherence"));
        }
        for (id, unit) in spectral {
            let mut r = push_spec(
                id,
                f64::NAN,
                unit,
                Confidence::High,
                trigger_val,
                reason(),
                0,
            );
            if id == "exp.freq_lr_coherence" {
                r.channels = Some(ChannelSelector::ChannelPair(0, 1));
            }
            out.records.push(r);
        }
        return out;
    }
    let Some(mono) = chans.first().copied() else {
        out.records.push(push(
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
        out.records.push(push(
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
        out.records.push(push(
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

    // ==== Precision transfer / cutoff ====
    // Cutoff: lowest bin whose cumulative-above energy is < 1% of
    // the one-sided (below-Nyquist) spectrum.
    if total_power > 0.0 && one_sided_power > 0.0 {
        let mut above = 0.0;
        let mut cutoff = 0usize;
        for k in (0..half).rev() {
            above += mean_power[k];
            if above / one_sided_power >= 0.01 {
                cutoff = k;
                break;
            }
        }
        let cutoff_hz = cutoff as f64 * bin_hz;
        out.records.push(push_spec(
            "exp.cutoff_hz",
            cutoff_hz,
            "hz",
            Confidence::Medium,
            Validity::Valid,
            None,
            n as u64,
        ));
        // Dictionary match: expected_khz ranges from codec_hf_cutoff
        // entries; distance to the nearest expected boundary.
        if let Some(dict) = dictionary {
            let mut best: Option<(f64, String)> = None;
            for entry in dict.entries.iter().filter(|e| e.topic == "codec_hf_cutoff") {
                if let Some(range) = entry
                    .annotation
                    .get("expected_khz")
                    .and_then(|v| v.as_array())
                {
                    for b in range.iter().filter_map(|v| v.as_f64()) {
                        let d = (cutoff_hz / 1_000.0 - b).abs();
                        if best.as_ref().is_none_or(|(bd, _)| d < *bd) {
                            best = Some((d, entry.annotation.to_string()));
                        }
                    }
                }
            }
            if let Some((d, ann)) = best {
                let mut r = push_spec(
                    "exp.dictionary_cutoff_delta_khz",
                    d,
                    "khz",
                    Confidence::Medium,
                    Validity::Valid,
                    None,
                    n as u64,
                );
                r.params = serde_json::json!({
                    "region_s": [region.start_s, region.end_s],
                    "triggers": region.triggers,
                    "expected": ann,
                    "dictionary_version": dict.version,
                });
                out.records.push(r);
            }
        }
        // Narrowband peaks/notches vs smoothed spectrum.
        let smooth_w = 16usize;
        let mut peaks = 0u64;
        let mut notches = 0u64;
        for k in smooth_w..half.saturating_sub(smooth_w) {
            let local: f64 =
                mean_power[k - smooth_w..k + smooth_w].iter().sum::<f64>() / (2 * smooth_w) as f64;
            if local > 0.0 {
                let ratio = (mean_power[k] / local).log10() * 10.0;
                if ratio > 6.0 {
                    peaks += 1;
                } else if ratio < -6.0 {
                    notches += 1;
                }
            }
        }
        out.records.push(push_spec(
            "exp.narrowband_peak_count",
            peaks as f64,
            "count",
            Confidence::Medium,
            Validity::Valid,
            None,
            frames as u64,
        ));
        out.records.push(push_spec(
            "exp.narrowband_notch_count",
            notches as f64,
            "count",
            Confidence::Medium,
            Validity::Valid,
            None,
            frames as u64,
        ));
    }

    // ==== Comb / echo / reflection: cepstrum ====
    {
        // Cepstrum of the log spectrum with the spectral envelope
        // removed: smoothing subtracts the broad trend so the echo
        // ripple (not the envelope) forms the quefrency peak.
        let log_p: Vec<f64> = mean_power[..half]
            .iter()
            .map(|p| (p + 1e-18).ln())
            .collect();
        let sw = 32usize;
        let flat: Vec<f64> = (0..half)
            .map(|k| {
                let lo = k.saturating_sub(sw);
                let hi = (k + sw).min(half - 1);
                let env: f64 = log_p[lo..=hi].iter().sum::<f64>() / (hi - lo + 1) as f64;
                log_p[k] - env
            })
            .collect();
        // Inverse-FFT size equals the STFT's bin count so a quefrency
        // index maps 1:1 to delay samples (spectrum spans bins/2).
        let size = bins;
        let mut re: Vec<f64> = vec![0.0; size];
        re[..half].copy_from_slice(&flat);
        let mut im = vec![0.0; size];
        al_locate::dsp::fft(&mut re, &mut im, true);
        let qmin = (0.0002 * rate as f64) as usize;
        let qmax = ((0.05 * rate as f64) as usize).min(size - 1);
        if qmax > qmin {
            let (qi, &peak) = re[qmin..=qmax]
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(i, v)| (i + qmin, v))
                .unwrap_or((qmin, &0.0));
            let baseline: f64 = re[qmin..=qmax].iter().sum::<f64>() / (qmax - qmin + 1) as f64;
            let prominence = if baseline.abs() > 1e-12 {
                (peak - baseline) / baseline.abs()
            } else {
                0.0
            };
            // A flat/degenerate cepstrum still yields an argmax — only
            // report a delay when the peak is clearly above baseline.
            let supported = total_power > 0.0 && prominence > 0.5;
            out.records.push(push_spec(
                "exp.echo_delay_s",
                if supported {
                    qi as f64 / rate as f64
                } else {
                    f64::NAN
                },
                "seconds",
                Confidence::Medium,
                if supported {
                    Validity::Valid
                } else {
                    Validity::InsufficientSupport
                },
                if supported {
                    None
                } else {
                    Some("no significant cepstral peak".into())
                },
                frames as u64,
            ));
            out.records.push(push_spec(
                "exp.cepstrum_prominence",
                prominence,
                "ratio",
                Confidence::Medium,
                Validity::Valid,
                None,
                frames as u64,
            ));
        }
    }

    // ==== SRC candidate: spectral mirror image correlation ====
    {
        // For each candidate source rate fs_c, fold the spectrum at
        // fs_c/2 and correlate the two halves — an SRC image mirrors
        // content around the fold.
        let mut best_score = 0.0;
        let mut best_rate = 0.0;
        let mut evaluated = false;
        for cand in [22_050.0, 24_000.0, 32_000.0, 44_100.0] {
            let fold = ((cand / 2.0) / bin_hz) as usize;
            if fold >= half || fold < 64 {
                continue;
            }
            let lo = &mean_power[fold - 64..fold];
            let hi: Vec<f64> = mean_power[fold..(fold + 64).min(half)]
                .iter()
                .rev()
                .copied()
                .collect();
            let n_use = lo.len().min(hi.len());
            if n_use >= 32 {
                evaluated = true;
                let c = corr0(&lo[..n_use], &hi[..n_use]);
                if c > best_score {
                    best_score = c;
                    best_rate = cand;
                }
            }
        }
        let (src_val, src_reason) = if evaluated {
            (Validity::Valid, None)
        } else {
            (
                Validity::InsufficientSupport,
                Some("no source-rate candidate has enough bins".into()),
            )
        };
        out.records.push(push_spec(
            "exp.src_image_score",
            if evaluated { best_score } else { f64::NAN },
            "corr",
            Confidence::Medium,
            src_val,
            src_reason.clone(),
            frames as u64,
        ));
        out.records.push(push_spec(
            "exp.src_candidate_rate",
            if evaluated { best_rate } else { f64::NAN },
            "hz",
            Confidence::Low,
            src_val,
            src_reason,
            frames as u64,
        ));
    }

    // ==== Nonlinear behaviour ====
    {
        let (e_pos, e_neg): (f64, f64) = seg.iter().fold((0.0, 0.0), |(p, nacc), &v| {
            if v >= 0.0 {
                (p + v * v, nacc)
            } else {
                (p, nacc + v * v)
            }
        });
        let asym = if e_pos + e_neg > 0.0 {
            (e_pos - e_neg).abs() / (e_pos + e_neg)
        } else {
            0.0
        };
        out.records.push(push(
            "exp.polarity_asymmetry",
            asym,
            "ratio",
            Confidence::Medium,
            Validity::Valid,
            None,
            n as u64,
        ));
        // THD proxy: strongest tonal bin's harmonic energy share.
        if total_power > 0.0 {
            let (f0, _) = mean_power[4..half.min(2048)]
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(i, v)| (i + 4, *v))
                .unwrap_or((0, 0.0));
            let mut harm = 0.0;
            for h in 2..=8usize {
                let k = f0 * h;
                if k >= half {
                    break;
                }
                harm += mean_power[k.saturating_sub(1)..=(k + 1).min(half - 1)]
                    .iter()
                    .sum::<f64>();
            }
            out.records.push(push_spec(
                "exp.thd_ratio",
                harm / total_power,
                "ratio",
                Confidence::Medium,
                Validity::Valid,
                None,
                frames as u64,
            ));
        }
    }

    // ==== Dynamics: short-term level range + attack/release ====
    {
        let win = ((0.02 * rate as f64) as usize).max(1); // 20 ms
        if n >= 4 * win {
            let levels: Vec<f64> = seg
                .chunks_exact(win)
                .map(|w| 20.0 * rms(w).max(1e-12).log10())
                .collect();
            if let Some(s) = robust_summary(&levels) {
                out.records.push(push(
                    "exp.dynamic_range_db",
                    s.p95 - s.p05,
                    "db",
                    Confidence::Medium,
                    Validity::Valid,
                    None,
                    s.support,
                ));
            }
            // Attack/release: steepest sustained rise/fall between
            // adjacent level windows.
            let mut rise = 0.0f64;
            let mut fall = 0.0f64;
            for w in levels.windows(2) {
                rise = rise.max(w[1] - w[0]);
                fall = fall.max(w[0] - w[1]);
            }
            out.records.push(push(
                "exp.attack_db_per_20ms",
                rise,
                "db",
                Confidence::Low,
                Validity::Valid,
                None,
                levels.len() as u64,
            ));
            out.records.push(push(
                "exp.release_db_per_20ms",
                fall,
                "db",
                Confidence::Low,
                Validity::Valid,
                None,
                levels.len() as u64,
            ));
        }
    }

    // ==== Timing / repeat / dropout ====
    {
        // Repeat-block score: max adjacent-block correlation at
        // 1024-sample blocks.
        let block = 1024usize;
        if n >= 4 * block {
            let best = seg
                .chunks_exact(block)
                .collect::<Vec<_>>()
                .windows(2)
                .map(|w| corr0(w[0], w[1]))
                .fold(0.0, f64::max);
            out.records.push(push(
                "exp.repeat_block_score",
                best,
                "corr",
                Confidence::Medium,
                Validity::Valid,
                None,
                (n / block) as u64,
            ));
        }
        // Gaps: 5 ms windows below -80 dBFS.
        let win = ((0.005 * rate as f64) as usize).max(1);
        let mut gaps = 0u64;
        let mut in_gap = false;
        for w in seg.chunks(win) {
            let low = 20.0 * rms(w).max(1e-12).log10() < -80.0;
            if low && !in_gap {
                gaps += 1;
            }
            in_gap = low;
        }
        out.records.push(push(
            "exp.gap_count",
            gaps as f64,
            "count",
            Confidence::Medium,
            Validity::Valid,
            None,
            (n / win.max(1)) as u64,
        ));
    }

    // ==== Codec artifact structure ====
    {
        // Spectral holes: bins in the top half below 1e-4 of max.
        if total_power > 0.0 {
            let maxp = mean_power[..half].iter().copied().fold(0.0, f64::max);
            let floor = maxp * 1e-4;
            let top = &mean_power[half / 2..half];
            let holes = top.iter().filter(|&&p| p < floor).count();
            out.records.push(push_spec(
                "exp.spectral_hole_ratio",
                holes as f64 / top.len().max(1) as f64,
                "ratio",
                Confidence::Medium,
                Validity::Valid,
                None,
                frames as u64,
            ));
        }
        // Block-boundary periodicity: energy-derivative energy when
        // folded onto common codec frame sizes.
        let block_e: Vec<f64> = (0..n / 64)
            .map(|i| seg[i * 64..(i + 1) * 64].iter().map(|v| v * v).sum::<f64>())
            .collect();
        let mut best = (0.0f64, 0u64);
        for size in [576usize, 1024, 1152] {
            let stride = size / 64;
            if stride == 0 || block_e.len() < 4 * stride {
                continue;
            }
            // Boundary energy vs interior energy.
            let mut bsum = 0.0;
            let mut isum = 0.0;
            for (i, &e) in block_e.iter().enumerate() {
                let edge = i % stride == 0;
                if edge {
                    bsum += e;
                } else {
                    isum += e;
                }
            }
            let nb = (block_e.len() / stride).max(1) as f64;
            let ni = (block_e.len() - block_e.len() / stride).max(1) as f64;
            let ratio = (bsum / nb) / (isum / ni).max(1e-18);
            if ratio > best.0 {
                best = (ratio, size as u64);
            }
        }
        out.records.push(push(
            "exp.codec_block_score",
            best.0,
            "ratio",
            Confidence::Low,
            Validity::Valid,
            None,
            block_e.len() as u64,
        ));
        out.records.push(push(
            "exp.codec_block_size",
            best.1 as f64,
            "samples",
            Confidence::Low,
            Validity::Valid,
            None,
            block_e.len() as u64,
        ));
    }

    // ==== Noise decomposition ====
    {
        // Tonal peaks above local median; hum harmonic count.
        let w = 8usize;
        let mut tonal = 0u64;
        for k in w..half.saturating_sub(w) {
            let mut local: Vec<f64> = mean_power[k - w..k + w].to_vec();
            local.sort_by(|a, b| a.total_cmp(b));
            let med = local[w];
            if med > 0.0 && (mean_power[k] / med).log10() * 10.0 > 15.0 {
                tonal += 1;
            }
        }
        out.records.push(push_spec(
            "exp.tonal_peak_count",
            tonal as f64,
            "count",
            Confidence::Medium,
            Validity::Valid,
            None,
            frames as u64,
        ));
        let mut hum = 0u64;
        for base in [50.0, 60.0] {
            for h in 1..=8usize {
                let f = base * h as f64;
                let k = (f / bin_hz) as usize;
                if k < half && k > 0 {
                    let local: f64 = mean_power[k.saturating_sub(3)..=(k + 3).min(half - 1)]
                        .iter()
                        .sum::<f64>()
                        / 7.0;
                    if total_power > 0.0 && local / (total_power / half as f64) > 10.0 {
                        hum += 1;
                    }
                }
            }
        }
        out.records.push(push_spec(
            "exp.hum_harmonic_count",
            hum as f64,
            "count",
            Confidence::Medium,
            Validity::Valid,
            None,
            frames as u64,
        ));
    }

    // ==== Advanced channel analysis (>= 2ch) ====
    if chans.len() >= 2
        && let Some(other) = chans.get(1).copied()
        && other.len() > s0
    {
        let oseg = &other[s0..s1.min(other.len())];
        let n_ch = seg.len().min(oseg.len());
        if n_ch >= cfg.stft_window {
            let bands = third_octave_bands(100.0, rate as f64 / 2.0 * 0.9);
            let mb = stft(&seg[..n_ch], cfg.stft_window, cfg.stft_hop, rate);
            let ob = stft(&oseg[..n_ch], cfg.stft_window, cfg.stft_hop, rate);
            let fr = mb.frames().min(ob.frames());
            if fr > 0 {
                // Frequency-dependent L/R coherence in the mid band.
                let mut coh_sum = 0.0;
                let mut coh_n = 0u64;
                let band_bins: Vec<usize> = bands
                    .iter()
                    .map(|&(lo, hi)| ((lo / bin_hz) as usize, (hi / bin_hz) as usize))
                    .flat_map(|(a, b)| a..b.min(half))
                    .collect();
                for f in 0..fr {
                    let c = csd(&mb.re[f], &mb.im[f], &ob.re[f], &ob.im[f]);
                    for &k in band_bins.iter().take(half) {
                        let aa = mb.re[f][k].powi(2) + mb.im[f][k].powi(2);
                        let bb = ob.re[f][k].powi(2) + ob.im[f][k].powi(2);
                        if aa * bb > 0.0 {
                            let num = c[k].0 * c[k].0 + c[k].1 * c[k].1;
                            coh_sum += (num / (aa * bb)).sqrt();
                            coh_n += 1;
                        }
                    }
                }
                let supported = coh_n > 0;
                let mut rec = push_spec(
                    "exp.freq_lr_coherence",
                    if supported {
                        coh_sum / coh_n as f64
                    } else {
                        f64::NAN
                    },
                    "coherence",
                    Confidence::Medium,
                    if supported {
                        Validity::Valid
                    } else {
                        Validity::InsufficientSupport
                    },
                    if supported {
                        None
                    } else {
                        Some("no mid-band bins with energy in both channels".into())
                    },
                    coh_n,
                );
                rec.channels = Some(ChannelSelector::ChannelPair(0, 1));
                out.records.push(rec);
            }
        }
    }

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
mod tests {
    use super::*;
    use crate::events::CandidateEvent;
    use al_dataset::types::Value;

    fn ev(kind: EventKind, start: f64, end: Option<f64>) -> CandidateEvent {
        ev_with(kind, start, end, Validity::Valid, Confidence::High)
    }

    fn ev_with(
        kind: EventKind,
        start: f64,
        end: Option<f64>,
        validity: Validity,
        confidence: Confidence,
    ) -> CandidateEvent {
        CandidateEvent {
            event_id: format!("EVT-t-{kind:?}-{validity:?}"),
            kind,
            input_id: "in".into(),
            start_s: start,
            end_s: end,
            strength: 1.0,
            confidence,
            validity,
            source_feature: "f".into(),
            frequency_range: None,
            channels: Vec::new(),
            algorithm_id: "events.v1".into(),
            algorithm_version: "1".into(),
            evidence: Vec::new(),
            params: serde_json::json!({}),
        }
    }

    #[test]
    fn no_triggers_produces_no_regions() {
        let cfg = ExpensiveConfig::default();
        assert!(plan_regions(10.0, &[], &cfg).is_empty());
        assert!(
            plan_regions(10.0, &[ev(EventKind::Clipping, 0.0, None)], &cfg)
                .iter()
                .all(|r| r.end_s <= 10.0)
        );
    }

    #[test]
    fn always_run_and_manual_modes() {
        let cfg = ExpensiveConfig {
            always_run: true,
            ..ExpensiveConfig::default()
        };
        let r = plan_regions(10.0, &[], &cfg);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].triggers, vec!["always_run"]);

        let cfg = ExpensiveConfig {
            manual_ranges: vec![(2.0, 4.0), (20.0, 30.0)],
            ..ExpensiveConfig::default()
        };
        let r = plan_regions(10.0, &[], &cfg);
        assert_eq!(r.len(), 1);
        assert_eq!((r[0].start_s, r[0].end_s), (2.0, 4.0));
        assert_eq!(r[0].triggers, vec!["manual"]);
    }

    #[test]
    fn event_regions_expand_and_merge_with_reasons() {
        let cfg = ExpensiveConfig::default(); // window 0.25 s
        let events = vec![
            ev(EventKind::LargeResidual, 1.0, None),
            ev(EventKind::Clipping, 1.1, None),
            ev(EventKind::LowCoherence, 8.0, Some(9.0)),
        ];
        let r = plan_regions(10.0, &events, &cfg);
        assert_eq!(r.len(), 2, "{r:?}");
        assert!((r[0].start_s - 0.75).abs() < 1e-9);
        assert!(r[0].triggers.contains(&"high_residual".to_string()));
        assert!(
            r[0].triggers
                .iter()
                .any(|t| t.starts_with("candidate:EVT-t-Clipping"))
        );
        assert!(r[1].triggers.contains(&"low_similarity".to_string()));
        assert!(
            r[1].triggers
                .iter()
                .any(|t| t.starts_with("candidate:EVT-t-LowCoherence"))
        );
    }

    #[test]
    fn merged_regions_carry_the_weakest_trigger_trust() {
        let cfg = ExpensiveConfig::default();
        let events = vec![
            ev_with(
                EventKind::Clipping,
                1.0,
                None,
                Validity::Valid,
                Confidence::High,
            ),
            ev_with(
                EventKind::LargeResidual,
                1.1,
                None,
                Validity::AmbiguousAlignment,
                Confidence::Low,
            ),
        ];
        let r = plan_regions(10.0, &events, &cfg);
        assert_eq!(r.len(), 1, "{r:?}");
        assert_eq!(r[0].validity, Validity::AmbiguousAlignment);
        assert_eq!(r[0].confidence, Confidence::Low);
        assert!(r[0].has_valid_trigger);

        // Non-Valid evidence alone still selects a region, flagged,
        // but without a `Valid` trigger it must not be estimated.
        let r = plan_regions(
            10.0,
            &[ev_with(
                EventKind::Clipping,
                3.0,
                None,
                Validity::InsufficientSupport,
                Confidence::Medium,
            )],
            &cfg,
        );
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].validity, Validity::InsufficientSupport);
        assert_eq!(r[0].confidence, Confidence::Medium);
        assert!(!r[0].has_valid_trigger);

        // A `manual` range is an independent `Valid` trigger: merged
        // with a non-Valid event it still licenses estimation.
        let cfg = ExpensiveConfig {
            manual_ranges: vec![(2.5, 3.5)],
            ..ExpensiveConfig::default()
        };
        let r = plan_regions(
            10.0,
            &[ev_with(
                EventKind::Clipping,
                3.0,
                None,
                Validity::InsufficientSupport,
                Confidence::Medium,
            )],
            &cfg,
        );
        assert_eq!(r.len(), 1, "{r:?}");
        assert!(r[0].has_valid_trigger);
        assert_eq!(r[0].validity, Validity::InsufficientSupport);
    }

    #[test]
    fn non_valid_trigger_records_invalid_observations() {
        let rate = 48_000u32;
        let sig = noise(1 << 15, 0xf00d);
        let chans: Vec<&[f64]> = vec![&sig];
        let region = TriggeredRegion {
            start_s: 0.0,
            end_s: sig.len() as f64 / rate as f64,
            triggers: vec!["candidate:EVT-t".into()],
            validity: Validity::AmbiguousAlignment,
            confidence: Confidence::Low,
            has_valid_trigger: false,
        };
        let out = analyze_region(
            "in",
            &chans,
            rate,
            &region,
            None,
            &ExpensiveConfig::default(),
        );
        // Every feature family is persisted as an invalid
        // observation: mapped validity, no finite numbers, no claim
        // beyond the trigger's confidence.
        assert!(!out.records.is_empty());
        for r in &out.records {
            assert_eq!(
                r.validity,
                Validity::AmbiguousAlignment,
                "record {} must carry the mapped validity",
                r.feature_id
            );
            assert_eq!(
                r.confidence,
                Confidence::Low,
                "record {} must not out-trust its trigger",
                r.feature_id
            );
            assert!(
                r.invalid_reason.is_some(),
                "record {} must explain why it is invalid",
                r.feature_id
            );
            match r.value {
                Value::Scalar(v) => assert!(
                    v.is_nan(),
                    "record {} must not emit a finite number",
                    r.feature_id
                ),
                _ => panic!("scalar expected"),
            }
        }
        for id in [
            "exp.cutoff_hz",
            "exp.echo_delay_s",
            "exp.src_image_score",
            "exp.codec_block_score",
            "exp.hum_harmonic_count",
        ] {
            assert!(
                out.records.iter().any(|r| r.feature_id == id),
                "missing invalid observation for {id}"
            );
        }
        // Stereo-only features are not emitted for mono input.
        assert!(
            out.records
                .iter()
                .all(|r| r.feature_id != "exp.freq_lr_coherence")
        );
    }

    #[test]
    fn non_valid_trigger_marks_stereo_features_invalid_too() {
        let rate = 48_000u32;
        let sig = noise(1 << 15, 0xbeef);
        let chans: Vec<&[f64]> = vec![&sig, &sig];
        let region = TriggeredRegion {
            start_s: 0.0,
            end_s: sig.len() as f64 / rate as f64,
            triggers: vec!["candidate:EVT-t".into()],
            validity: Validity::InsufficientSupport,
            confidence: Confidence::Medium,
            has_valid_trigger: false,
        };
        let out = analyze_region(
            "in",
            &chans,
            rate,
            &region,
            None,
            &ExpensiveConfig::default(),
        );
        let coh = record(&out, "exp.freq_lr_coherence");
        // InsufficientSupport maps to SilenceInsufficientEnergy.
        assert_eq!(coh.validity, Validity::SilenceInsufficientEnergy);
        assert_eq!(coh.channels, Some(ChannelSelector::ChannelPair(0, 1)));
        match coh.value {
            Value::Scalar(v) => assert!(v.is_nan()),
            _ => panic!("scalar expected"),
        }
    }

    #[test]
    fn manual_trigger_still_estimates_when_merged_with_non_valid_event() {
        let rate = 48_000u32;
        let raw = noise(1 << 16, 0xabc);
        let sig = al_locate::dsp::bandpass(&raw, rate, 100.0, 8_000.0);
        let chans: Vec<&[f64]> = vec![&sig];
        let region = TriggeredRegion {
            start_s: 0.0,
            end_s: sig.len() as f64 / rate as f64,
            triggers: vec!["manual".into(), "candidate:EVT-t".into()],
            validity: Validity::AmbiguousAlignment,
            confidence: Confidence::Low,
            has_valid_trigger: true,
        };
        let out = analyze_region(
            "in",
            &chans,
            rate,
            &region,
            None,
            &ExpensiveConfig::default(),
        );
        // Estimation ran: finite numbers exist, but records still
        // carry the weakest trigger's validity and confidence.
        let cutoff = record(&out, "exp.cutoff_hz");
        match cutoff.value {
            Value::Scalar(v) => assert!(v.is_finite(), "manual trigger must estimate"),
            _ => panic!("scalar expected"),
        }
        for r in &out.records {
            assert_ne!(
                r.validity,
                Validity::Valid,
                "record {} must not claim Valid",
                r.feature_id
            );
            assert_eq!(r.confidence, Confidence::Low);
        }
    }

    fn noise(n: usize, seed: u64) -> Vec<f64> {
        let mut s = seed;
        (0..n)
            .map(|_| {
                s = s
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ((s >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0
            })
            .map(|v| v - 0.5)
            .collect()
    }

    fn record<'a>(out: &'a ExpensiveResult, id: &str) -> &'a FeatureRecord {
        out.records
            .iter()
            .find(|r| r.feature_id == id)
            .unwrap_or_else(|| panic!("missing {id}"))
    }

    fn val(r: &FeatureRecord) -> f64 {
        match &r.value {
            al_dataset::types::Value::Scalar(v) => *v,
            _ => panic!("scalar expected"),
        }
    }

    #[test]
    fn region_cutoff_and_echo_are_measured() {
        let rate = 48_000u32;
        // Lowpass-ish source: bandpassed noise below ~8 kHz.
        let raw = noise(1 << 16, 0xabc);
        let sig = al_locate::dsp::bandpass(&raw, rate, 100.0, 8_000.0);
        // Add an echo at 5 ms.
        let mut echoed = sig.clone();
        for i in 240..echoed.len() {
            echoed[i] += 0.4 * sig[i - 240];
        }
        let chans: Vec<&[f64]> = vec![&echoed];
        let region = TriggeredRegion {
            start_s: 0.0,
            end_s: echoed.len() as f64 / rate as f64,
            triggers: vec!["always_run".into()],
            validity: Validity::Valid,
            confidence: Confidence::High,
            has_valid_trigger: true,
        };
        let out = analyze_region(
            "in",
            &chans,
            rate,
            &region,
            None,
            &ExpensiveConfig::default(),
        );
        let cutoff = val(record(&out, "exp.cutoff_hz"));
        assert!(cutoff < 12_000.0 && cutoff > 4_000.0, "cutoff {cutoff}");
        let echo = val(record(&out, "exp.echo_delay_s"));
        assert!((echo - 0.005).abs() < 0.001, "echo delay {echo}");
        // Every record carries its trigger.
        assert!(
            out.records
                .iter()
                .all(|r| { r.params["triggers"].as_array().unwrap()[0] == "always_run" })
        );
    }
}
