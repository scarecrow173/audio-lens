//! Cheap feature families: always-on single-capture observations.
//!
//! All measurements are `FeatureRecord`s — failures surface as row
//! validity, never as stage failure. Per-frame trajectories are
//! persisted as tensors and referenced through `Value::Tensor`; the
//! spectral intermediates (`intermediate` role) are produced here so
//! reference/expensive stages can reuse them without recomputation.

use al_core::{Confidence, Validity};
use al_dataset::types::{FeatureRecord, Resolution, ResolutionClass, Value};
use al_locate::decode::DecodedInput;
use al_locate::dsp::{corr0, ncc, rms, sum_sq};

use crate::records::{RecordBuilder, robust_summary};
use crate::stft::{StftMatrix, band_power, complex_coherence, envelope, stft, third_octave_bands};

/// Cheap-stage tunables (`config.analysis.cheap.*` overrides).
#[derive(Debug, Clone)]
pub struct CheapConfig {
    /// STFT analysis window (samples).
    pub stft_window: usize,
    /// STFT hop (samples).
    pub stft_hop: usize,
    /// Level trajectory window (seconds).
    pub level_window_s: f64,
    /// Level trajectory hop (seconds).
    pub level_hop_s: f64,
    /// Sample value treated as clipping.
    pub clip_threshold: f64,
    /// Samples count as "stuck" when exactly equal to the previous.
    pub min_stuck_run: usize,
    /// Fixed analysis bands `[lo, hi)` Hz.
    pub bands: Vec<(f64, f64)>,
    /// Maximum channel-pair lag for IPD estimate (seconds).
    pub ipd_max_lag_s: f64,
}

impl Default for CheapConfig {
    fn default() -> Self {
        Self {
            stft_window: 2048,
            stft_hop: 1024,
            level_window_s: 0.020,
            level_hop_s: 0.010,
            clip_threshold: 0.999,
            min_stuck_run: 4,
            bands: vec![
                (20.0, 120.0),
                (120.0, 400.0),
                (400.0, 1500.0),
                (1500.0, 5000.0),
                (5000.0, 12000.0),
                (12000.0, 20000.0),
            ],
            ipd_max_lag_s: 0.001,
        }
    }
}

impl CheapConfig {
    /// Overlay `config.analysis.cheap` onto the defaults.
    pub fn from_job(config: &serde_json::Value) -> Self {
        let c = &config["analysis"]["cheap"];
        let mut cfg = Self::default();
        if let Some(v) = c["stft_window"].as_u64() {
            cfg.stft_window = v as usize;
        }
        if let Some(v) = c["stft_hop"].as_u64() {
            cfg.stft_hop = v as usize;
        }
        if let Some(v) = c["level_window_s"].as_f64() {
            cfg.level_window_s = v;
        }
        if let Some(v) = c["level_hop_s"].as_f64() {
            cfg.level_hop_s = v;
        }
        if let Some(v) = c["clip_threshold"].as_f64() {
            cfg.clip_threshold = v;
        }
        cfg
    }
}

/// One 2-D tensor to persist under `intermediate` or `features/cheap`.
#[derive(Debug)]
pub struct NamedTensor {
    /// Dataset-relative path (`intermediate/<input>/ch0/psd`).
    pub path: String,
    /// `[rows, cols]`.
    pub shape: [u64; 2],
    /// Row-major data.
    pub data: Vec<f64>,
}

/// Everything one decoded input produced.
#[derive(Debug, Default)]
pub struct CheapResult {
    /// Feature records (JSON rows under `features/cheap/<input>.json`).
    pub records: Vec<FeatureRecord>,
    /// Tensors for the `intermediate` and `features/cheap` roles.
    pub tensors: Vec<NamedTensor>,
}

fn db(x: f64) -> f64 {
    20.0 * x.max(1e-12).log10()
}

fn windowed(samples: &[f64], window: usize, hop: usize) -> Vec<&[f64]> {
    if window == 0 || hop == 0 || samples.len() < window {
        return Vec::new();
    }
    (0..=samples.len() - window)
        .step_by(hop)
        .map(|s| &samples[s..s + window])
        .collect()
}

fn matrix_to_tensor(path: String, m: &[Vec<f64>]) -> NamedTensor {
    let rows = m.len() as u64;
    let cols = m.first().map_or(0, |r| r.len()) as u64;
    let mut data = Vec::with_capacity(m.iter().map(Vec::len).sum());
    for row in m {
        data.extend_from_slice(row);
    }
    NamedTensor {
        path,
        shape: [rows, cols],
        data,
    }
}

/// Analyze one decoded input. `base` is the dataset path prefix for
/// this input's artifacts (`<input_id>`).
pub fn analyze_input(dec: &DecodedInput, cfg: &CheapConfig) -> CheapResult {
    let mut out = CheapResult::default();
    let rate = dec.sample_rate;
    let lineage: Vec<String> = dec
        .artifact_ids
        .iter()
        .map(|a| a.as_str().to_string())
        .collect();

    // --- Per-channel level + digital anomaly + spectral families ---
    let mut channel_rms: Vec<Vec<f64>> = Vec::new();
    let mut channel_stft: Vec<Option<StftMatrix>> = Vec::new();
    let level_window = (cfg.level_window_s * rate as f64).round() as usize;
    let level_hop = (cfg.level_hop_s * rate as f64).round().max(1.0) as usize;

    for (ch, samples) in dec.channel_samples.iter().enumerate() {
        let path = format!("{}/ch{ch}", dec.input_id);
        let builder = RecordBuilder::new(&dec.input_id, "cheap.v1", ResolutionClass::Segment)
            .lineage(lineage.iter().cloned());
        let ch_sel = Some(al_dataset::types::ChannelSelector::Channel(ch as u16));
        let mut push = |mut rec: FeatureRecord| {
            rec.channels = ch_sel;
            out.records.push(rec);
        };

        if samples.is_empty() {
            push(builder.scalar(
                "level.rms_dbfs",
                f64::NEG_INFINITY,
                "dbfs",
                None,
                Confidence::Low,
                Validity::SilenceInsufficientEnergy,
                Some("empty channel".into()),
                0,
            ));
            channel_rms.push(Vec::new());
            channel_stft.push(None);
            continue;
        }

        // --- Level family ---
        let peak = samples.iter().fold(0.0f64, |m, &v| m.max(v.abs()));
        let dc = samples.iter().sum::<f64>() / samples.len() as f64;
        let rms_all = rms(samples);
        let silent = peak <= 0.0;
        let level_validity = if silent {
            Validity::SilenceInsufficientEnergy
        } else {
            Validity::Valid
        };
        let support = samples.len() as u64;

        push(builder.scalar(
            "level.sample_peak",
            peak,
            "linear",
            None,
            Confidence::High,
            level_validity,
            None,
            support,
        ));
        // True peak estimate: 4x linear interpolation between samples.
        let mut true_peak = peak;
        for w in samples.windows(2) {
            for k in 1..4 {
                let t = k as f64 / 4.0;
                true_peak = true_peak.max((w[0] + (w[1] - w[0]) * t).abs());
            }
        }
        let mut tp = builder.scalar(
            "level.true_peak_est",
            true_peak,
            "linear",
            None,
            Confidence::Medium,
            level_validity,
            None,
            support,
        );
        tp.params = serde_json::json!({"method": "linear4x"});
        push(tp);
        push(builder.scalar(
            "level.dc_offset",
            dc,
            "linear",
            None,
            Confidence::High,
            Validity::Valid,
            None,
            support,
        ));
        push(builder.scalar(
            "level.crest_factor",
            if rms_all > 0.0 { peak / rms_all } else { 0.0 },
            "ratio",
            None,
            Confidence::High,
            level_validity,
            None,
            support,
        ));
        push(builder.scalar(
            "level.rms_dbfs",
            db(rms_all),
            "dbfs",
            None,
            Confidence::High,
            level_validity,
            None,
            support,
        ));

        // Level trajectory (mid-resolution) + robust summary.
        let frames = windowed(samples, level_window, level_hop);
        let rms_traj: Vec<f64> = frames.iter().map(|f| rms(f)).collect();
        channel_rms.push(rms_traj.clone());
        let traj_tensor = matrix_to_tensor(
            format!("features/cheap/{path}/level_trajectory"),
            std::slice::from_ref(&rms_traj),
        );
        let traj_path = traj_tensor.path.clone();
        out.tensors.push(traj_tensor);
        let mut rec = builder.scalar(
            "level.trajectory",
            rms_all,
            "linear",
            None,
            Confidence::High,
            level_validity,
            None,
            rms_traj.len() as u64,
        );
        rec.value = Value::Tensor {
            array: traj_path,
            region: vec![[0, 1], [0, rms_traj.len() as u64]],
        };
        rec.resolution = Some(Resolution {
            sample_rate: rate,
            window: level_window as u64,
            hop: level_hop as u64,
        });
        rec.resolution_class = ResolutionClass::Mid20ms;
        push(rec);

        if let Some(s) = robust_summary(&rms_traj) {
            let rb = || {
                RecordBuilder::new(&dec.input_id, "cheap.v1", ResolutionClass::Mid20ms)
                    .resolution(level_window as u64, level_hop as u64, rate)
                    .lineage(lineage.iter().cloned())
            };
            push({
                let mut r = rb().scalar(
                    "level.rms_median_dbfs",
                    db(s.median),
                    "dbfs",
                    None,
                    Confidence::High,
                    level_validity,
                    None,
                    s.support,
                );
                r.channels = ch_sel;
                r
            });
            push({
                let mut r = rb().scalar(
                    "level.rms_mad_db",
                    db(s.median + s.mad) - db(s.median),
                    "db",
                    None,
                    Confidence::Medium,
                    level_validity,
                    None,
                    s.support,
                );
                r.channels = ch_sel;
                r
            });
            push({
                let mut r = rb().scalar(
                    "level.rms_p05_dbfs",
                    db(s.p05),
                    "dbfs",
                    None,
                    Confidence::High,
                    level_validity,
                    None,
                    s.support,
                );
                r.channels = ch_sel;
                r
            });
            push({
                let mut r = rb().scalar(
                    "level.rms_p95_dbfs",
                    db(s.p95),
                    "dbfs",
                    None,
                    Confidence::High,
                    level_validity,
                    None,
                    s.support,
                );
                r.channels = ch_sel;
                r
            });
        }

        // Momentary (400ms) / short-term (3s) / integrated loudness
        // estimates (energy domain, no K-weighting — params record it).
        for (id, seconds, class) in [
            ("level.momentary_dbfs", 0.4, ResolutionClass::Wide500ms),
            ("level.short_term_dbfs", 3.0, ResolutionClass::Seconds),
        ] {
            let win = ((seconds * rate as f64).round() as usize).max(1);
            let tr: Vec<f64> = windowed(samples, win, win / 2)
                .iter()
                .map(|f| rms(f))
                .collect();
            let mut r = RecordBuilder::new(&dec.input_id, "cheap.v1", class)
                .resolution(win as u64, (win / 2) as u64, rate)
                .lineage(lineage.iter().cloned())
                .scalar(
                    id,
                    tr.iter().fold(f64::NEG_INFINITY, |m, &v| m.max(db(v))),
                    "dbfs",
                    None,
                    Confidence::Medium,
                    if tr.is_empty() {
                        Validity::InsufficientSupport
                    } else {
                        level_validity
                    },
                    None,
                    tr.len() as u64,
                );
            r.channels = ch_sel;
            r.params = serde_json::json!({"estimate": "energy, no K-weighting"});
            push(r);
        }
        // Integrated: mean energy of momentary blocks.
        let m_win = ((0.4 * rate as f64).round() as usize).max(1);
        let mom: Vec<f64> = windowed(samples, m_win, m_win)
            .iter()
            .map(|f| sum_sq(f) / f.len() as f64)
            .collect();
        let integrated = if mom.is_empty() {
            f64::NEG_INFINITY
        } else {
            db((mom.iter().sum::<f64>() / mom.len() as f64).sqrt())
        };
        let mut integ = builder.scalar(
            "level.integrated_dbfs",
            integrated,
            "dbfs",
            None,
            Confidence::Medium,
            if mom.is_empty() {
                Validity::InsufficientSupport
            } else {
                level_validity
            },
            None,
            mom.len() as u64,
        );
        integ.channels = ch_sel;
        integ.params = serde_json::json!({"estimate": "energy, no gating/K-weighting"});
        push(integ);
        // LRA: p95 - p10 of short-term levels.
        let st: Vec<f64> = {
            let w = ((3.0 * rate as f64).round() as usize).max(1);
            windowed(samples, w, w / 2)
                .iter()
                .map(|f| db(rms(f)))
                .collect()
        };
        let lra = robust_summary(&st).map(|s| s.p95 - s.p05);
        let mut lra_rec = builder.scalar(
            "level.loudness_range_db",
            lra.unwrap_or(f64::NAN),
            "db",
            None,
            Confidence::Medium,
            match lra {
                Some(_) => level_validity,
                None => Validity::InsufficientSupport,
            },
            None,
            st.len() as u64,
        );
        lra_rec.channels = ch_sel;
        push(lra_rec);

        // --- Digital anomaly family ---
        let clip_thresh = cfg.clip_threshold;
        let mut clip_count = 0u64;
        let mut clip_run = 0u64;
        let mut clip_max_run = 0u64;
        let mut zero_run = 0u64;
        let mut zero_max_run = 0u64;
        let mut zero_runs = 0u64;
        let mut stuck_run = 0u64;
        let mut stuck_max_run = 0u64;
        let mut max_step = 0.0f64;
        let mut clicks = 0u64;
        let local_rms = rms_all.max(1e-6);
        for (i, &v) in samples.iter().enumerate() {
            if v.abs() >= clip_thresh {
                clip_count += 1;
                clip_run += 1;
                clip_max_run = clip_max_run.max(clip_run);
            } else {
                clip_run = 0;
            }
            if v == 0.0 {
                if zero_run == 0 {
                    zero_runs += 1;
                }
                zero_run += 1;
                zero_max_run = zero_max_run.max(zero_run);
            } else {
                zero_run = 0;
            }
            if i > 0 {
                if v == samples[i - 1] {
                    stuck_run += 1;
                    stuck_max_run = stuck_max_run.max(stuck_run);
                } else {
                    stuck_run = 0;
                }
                let step = (v - samples[i - 1]).abs();
                if step > max_step {
                    max_step = step;
                }
                if step > 4.0 * local_rms && step > 0.1 {
                    clicks += 1;
                }
            }
        }
        let clip_rate = clip_count as f64 / samples.len() as f64;
        push(builder.scalar(
            "dig.clipping_rate",
            clip_rate,
            "fraction",
            None,
            Confidence::High,
            Validity::Valid,
            None,
            support,
        ));
        push(builder.scalar(
            "dig.clip_max_run",
            clip_max_run as f64,
            "samples",
            None,
            Confidence::High,
            Validity::Valid,
            None,
            support,
        ));
        push(builder.scalar(
            "dig.zero_run_count",
            zero_runs as f64,
            "count",
            None,
            Confidence::High,
            Validity::Valid,
            None,
            support,
        ));
        push(builder.scalar(
            "dig.zero_run_max",
            zero_max_run as f64,
            "samples",
            None,
            Confidence::High,
            Validity::Valid,
            None,
            support,
        ));
        push(builder.scalar(
            "dig.stuck_run_max",
            stuck_max_run as f64,
            "samples",
            None,
            Confidence::High,
            Validity::Valid,
            None,
            support,
        ));
        push(builder.scalar(
            "dig.discontinuity_max",
            max_step,
            "linear",
            None,
            Confidence::High,
            Validity::Valid,
            None,
            support,
        ));
        push(builder.scalar(
            "dig.click_count",
            clicks as f64,
            "count",
            None,
            Confidence::Medium,
            Validity::Valid,
            None,
            support,
        ));

        // Identical-block candidates: hash 1024-sample blocks.
        const BLOCK: usize = 1024;
        let mut block_dup = 0u64;
        if samples.len() >= BLOCK * 2 {
            let mut seen = std::collections::HashMap::new();
            for chunk in samples.as_chunks::<BLOCK>().0 {
                let mut h: u64 = 1469598103934665603;
                for v in chunk {
                    h = h.wrapping_mul(1099511628211) ^ v.to_bits();
                }
                if seen.insert(h, ()).is_some() {
                    block_dup += 1;
                }
            }
        }
        push(builder.scalar(
            "dig.duplicate_block_candidates",
            block_dup as f64,
            "count",
            None,
            Confidence::Medium,
            Validity::Valid,
            None,
            (samples.len() / BLOCK) as u64,
        ));
        // Quantization step estimate: smallest positive difference
        // between sorted distinct levels (plausible only when the
        // signal uses a small level set).
        let mut uniq: Vec<u64> = samples.iter().map(|v| v.to_bits()).collect();
        uniq.sort_unstable();
        uniq.dedup();
        let quant = if uniq.len() >= 2 && uniq.len() <= 1_000_000 {
            let mut sorted: Vec<f64> = uniq.iter().map(|b| f64::from_bits(*b)).collect();
            sorted.sort_by(|a, b| a.total_cmp(b));
            sorted
                .windows(2)
                .map(|w| w[1] - w[0])
                .filter(|d| *d > 0.0)
                .fold(f64::INFINITY, f64::min)
        } else {
            f64::NAN
        };
        let mut qr = builder.scalar(
            "dig.quant_step_est",
            quant,
            "linear",
            None,
            Confidence::Low,
            if quant.is_finite() {
                Validity::Valid
            } else {
                Validity::NotApplicable
            },
            None,
            uniq.len() as u64,
        );
        qr.channels = ch_sel;
        push(qr);

        // --- Spectral family + intermediates ---
        let m = stft(samples, cfg.stft_window, cfg.stft_hop, rate);
        if m.frames() == 0 {
            push(builder.scalar(
                "spec.centroid_hz",
                f64::NAN,
                "hz",
                None,
                Confidence::Low,
                Validity::InsufficientSupport,
                Some("signal shorter than one stft window".into()),
                0,
            ));
            channel_stft.push(None);
            continue;
        }
        out.tensors.push(matrix_to_tensor(
            format!("intermediate/{path}/stft_re"),
            &m.re,
        ));
        out.tensors.push(matrix_to_tensor(
            format!("intermediate/{path}/stft_im"),
            &m.im,
        ));
        let mag = m.magnitude();
        let pow = m.power();
        let psd = m.psd();
        out.tensors.push(matrix_to_tensor(
            format!("intermediate/{path}/magnitude"),
            &mag,
        ));
        out.tensors
            .push(matrix_to_tensor(format!("intermediate/{path}/power"), &pow));
        out.tensors
            .push(matrix_to_tensor(format!("intermediate/{path}/psd"), &psd));
        let bands: Vec<Vec<f64>> = pow
            .iter()
            .map(|row| band_power(row, rate, m.fft_size, &cfg.bands))
            .collect();
        out.tensors.push(matrix_to_tensor(
            format!("intermediate/{path}/band_power"),
            &bands,
        ));
        let third = third_octave_bands(20.0, (rate as f64 / 2.0).min(20000.0));
        let third_p: Vec<Vec<f64>> = pow
            .iter()
            .map(|row| band_power(row, rate, m.fft_size, &third))
            .collect();
        out.tensors.push(matrix_to_tensor(
            format!("intermediate/{path}/third_octave"),
            &third_p,
        ));
        let env: Vec<Vec<f64>> = mag.iter().map(|r| envelope(r, 8)).collect();
        out.tensors.push(matrix_to_tensor(
            format!("intermediate/{path}/envelope"),
            &env,
        ));
        channel_stft.push(Some(m));

        // Per-frame spectral shape trajectories + summaries.
        let half = mag[0].len() / 2;
        let bin_hz = rate as f64 / (half * 2) as f64;
        let mut centroid = Vec::with_capacity(mag.len());
        let mut flatness = Vec::with_capacity(mag.len());
        let mut rolloff = Vec::with_capacity(mag.len());
        let mut entropy = Vec::with_capacity(mag.len());
        let mut flux = Vec::with_capacity(mag.len());
        let mut prev: Option<Vec<f64>> = None;
        for row in &mag {
            let row = &row[..half];
            let total: f64 = row.iter().sum();
            let centroid_hz = if total > 0.0 {
                row.iter()
                    .enumerate()
                    .map(|(b, &v)| b as f64 * bin_hz * v)
                    .sum::<f64>()
                    / total
            } else {
                0.0
            };
            centroid.push(centroid_hz);
            let geo = (row.iter().map(|v| (v + 1e-12).ln()).sum::<f64>() / half as f64).exp();
            flatness.push(if total > 0.0 {
                geo / (total / half as f64)
            } else {
                0.0
            });
            // 85% energy rolloff.
            let mut acc = 0.0;
            let target = total * 0.85;
            let mut ro_bin = 0;
            for (b, &v) in row.iter().enumerate() {
                acc += v;
                if acc >= target {
                    ro_bin = b;
                    break;
                }
            }
            rolloff.push(ro_bin as f64 * bin_hz);
            if total > 0.0 {
                let h: f64 = row
                    .iter()
                    .filter(|&&v| v > 0.0)
                    .map(|&v| (v / total) * (v / total).ln())
                    .sum::<f64>();
                entropy.push(-h / (half as f64).ln());
            } else {
                entropy.push(0.0);
            }
            flux.push(match &prev {
                Some(p) => row
                    .iter()
                    .zip(p.iter())
                    .map(|(&a, &b)| (a - b).max(0.0))
                    .sum(),
                // First frame has no predecessor: flux is 0 so every
                // trajectory row shares the frame count.
                None => 0.0,
            });
            prev = Some(row.to_vec());
        }
        let bandwidth: Vec<f64> = mag
            .iter()
            .map(|row| {
                let row = &row[..half];
                let total: f64 = row.iter().sum();
                if total <= 0.0 {
                    return 0.0;
                }
                let c = row
                    .iter()
                    .enumerate()
                    .map(|(b, &v)| b as f64 * bin_hz * v)
                    .sum::<f64>()
                    / total;
                (row.iter()
                    .enumerate()
                    .map(|(b, &v)| {
                        let d = b as f64 * bin_hz - c;
                        d * d * v
                    })
                    .sum::<f64>()
                    / total)
                    .sqrt()
            })
            .collect();
        out.tensors.push(matrix_to_tensor(
            format!("features/cheap/{path}/spec_trajectories"),
            &[
                centroid.clone(),
                bandwidth.clone(),
                rolloff.clone(),
                flatness.clone(),
                entropy.clone(),
                flux.clone(),
            ],
        ));

        let rb = RecordBuilder::new(&dec.input_id, "cheap.v1", ResolutionClass::Mid20ms)
            .resolution(cfg.stft_window as u64, cfg.stft_hop as u64, rate)
            .lineage(lineage.iter().cloned());
        let mut spec_rec = |id: &str, vals: &[f64], unit: &str| {
            if let Some(s) = robust_summary(vals) {
                let mut r = rb.scalar(
                    id,
                    s.median,
                    unit,
                    None,
                    Confidence::High,
                    level_validity,
                    None,
                    s.support,
                );
                r.channels = ch_sel;
                r.params = serde_json::json!({"p95": s.p95, "mad": s.mad});
                out.records.push(r);
            }
        };
        spec_rec("spec.centroid_hz", &centroid, "hz");
        spec_rec("spec.bandwidth_hz", &bandwidth, "hz");
        spec_rec("spec.rolloff_hz", &rolloff, "hz");
        spec_rec("spec.flatness", &flatness, "ratio");
        spec_rec("spec.entropy", &entropy, "norm");
        spec_rec("spec.flux", &flux, "linear");
        // Spectral slope (linear fit of log-magnitude vs log-freq,
        // first 90% of band).
        let mean_row: Vec<f64> = (0..half)
            .map(|b| mag.iter().map(|r| r[b]).sum::<f64>() / mag.len() as f64)
            .collect();
        let xs: Vec<f64> = (1..(half * 9 / 10).max(2))
            .map(|b| (b as f64 * bin_hz).ln())
            .collect();
        let ys: Vec<f64> = (1..(half * 9 / 10).max(2))
            .map(|b| (mean_row[b] + 1e-12).ln())
            .collect();
        let n = xs.len() as f64;
        let (sx, sy, sxy, sxx) = xs
            .iter()
            .zip(&ys)
            .fold((0.0, 0.0, 0.0, 0.0), |(ax, ay, axy, axx), (&x, &y)| {
                (ax + x, ay + y, axy + x * y, axx + x * x)
            });
        let slope = if sxx * n - sx * sx != 0.0 {
            (sxy * n - sx * sy) / (sxx * n - sx * sx)
        } else {
            0.0
        };
        let mut sr = rb.scalar(
            "spec.slope",
            slope,
            "log_log",
            None,
            Confidence::Medium,
            level_validity,
            None,
            mag.len() as u64,
        );
        sr.channels = ch_sel;
        out.records.push(sr);
        // Narrowband peaks / notches: bins >6 dB above/below the
        // local neighborhood median.
        let mut peaks = 0u64;
        let mut notches = 0u64;
        for (b, &v) in mean_row.iter().enumerate().take(half).skip(4) {
            let lo = b.saturating_sub(4);
            let hi = (b + 5).min(half);
            let mut nb: Vec<f64> = mean_row[lo..hi].to_vec();
            nb.sort_by(|a, b2| a.total_cmp(b2));
            let med = nb[nb.len() / 2];
            if v > med * 4.0 {
                peaks += 1;
            }
            if v < med / 4.0 && med > 0.0 {
                notches += 1;
            }
        }
        let mut pr = rb.scalar(
            "spec.narrowband_peaks",
            peaks as f64,
            "count",
            None,
            Confidence::Medium,
            level_validity,
            None,
            half as u64,
        );
        pr.channels = ch_sel;
        out.records.push(pr);
        let mut nr = rb.scalar(
            "spec.narrowband_notches",
            notches as f64,
            "count",
            None,
            Confidence::Medium,
            level_validity,
            None,
            half as u64,
        );
        nr.channels = ch_sel;
        out.records.push(nr);

        // --- Noise / modulation family ---
        // Noise floor: p05 of the level trajectory.
        if let Some(s) = robust_summary(&rms_traj) {
            let mut nf = rb.scalar(
                "noise.floor_dbfs",
                db(s.p05),
                "dbfs",
                None,
                Confidence::Medium,
                Validity::Valid,
                None,
                s.support,
            );
            nf.channels = ch_sel;
            out.records.push(nf);
        }
        // Tonal peak: strongest narrowband peak frequency + level.
        let (peak_bin, peak_val) = mean_row
            .iter()
            .enumerate()
            .skip(1)
            .take(half.saturating_sub(1))
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(b, &v)| (b, v))
            .unwrap_or((0, 0.0));
        let mut tn = rb.scalar(
            "noise.tonal_peak_hz",
            peak_bin as f64 * bin_hz,
            "hz",
            None,
            Confidence::Medium,
            level_validity,
            None,
            mag.len() as u64,
        );
        tn.channels = ch_sel;
        tn.params = serde_json::json!({"peak_mag": peak_val});
        out.records.push(tn);
        // Hum: energy at 50/60Hz fundamentals + first 5 harmonics
        // relative to total.
        let total_e: f64 = sum_sq(&mean_row);
        let hum_e: f64 = [50.0f64, 60.0]
            .iter()
            .flat_map(|&f| (1..=5).map(move |h| f * h as f64))
            .filter(|&f| f < rate as f64 / 2.0)
            .map(|f| {
                let b = (f / bin_hz).round() as usize;
                let lo = b.saturating_sub(1);
                let hi = (b + 2).min(half);
                sum_sq(&mean_row[lo..hi])
            })
            .sum();
        let mut hum = rb.scalar(
            "noise.hum_ratio",
            if total_e > 0.0 { hum_e / total_e } else { 0.0 },
            "fraction",
            None,
            Confidence::Medium,
            level_validity,
            None,
            half as u64,
        );
        hum.channels = ch_sel;
        out.records.push(hum);
        // AM spectrum: dominant modulation frequency of the RMS
        // trajectory below 20 Hz.
        if rms_traj.len() >= 16 {
            let traj_rate = rate as f64 / level_hop as f64;
            let mut tm: Vec<f64> = rms_traj.clone();
            let mean = tm.iter().sum::<f64>() / tm.len() as f64;
            tm.iter_mut().for_each(|v| *v -= mean);
            let mm = crate::stft::stft(&tm, tm.len().min(256), tm.len().min(256), traj_rate as u32);
            if mm.frames() > 0 {
                let mmag = mm.magnitude();
                let row = &mmag[0];
                let bin_f = traj_rate / mm.fft_size as f64;
                let (am_bin, _) = row
                    .iter()
                    .enumerate()
                    .take_while(|(b, _)| *b as f64 * bin_f <= 20.0)
                    .skip(1)
                    .max_by(|a, b| a.1.total_cmp(b.1))
                    .unwrap_or((0, &0.0));
                let mut am = rb.scalar(
                    "mod.am_freq_hz",
                    am_bin as f64 * bin_f,
                    "hz",
                    None,
                    Confidence::Low,
                    Validity::Valid,
                    None,
                    rms_traj.len() as u64,
                );
                am.channels = ch_sel;
                out.records.push(am);
                let depth = if mean > 0.0 {
                    (rms_traj.iter().fold(0.0f64, |m, &v| m.max(v)) / mean) - 1.0
                } else {
                    0.0
                };
                let mut ad = rb.scalar(
                    "mod.am_depth",
                    depth,
                    "ratio",
                    None,
                    Confidence::Low,
                    Validity::Valid,
                    None,
                    rms_traj.len() as u64,
                );
                ad.channels = ch_sel;
                out.records.push(ad);
            }
        }
    }

    // --- Stereo / channel-pair family ---
    for a in 0..dec.channel_samples.len() {
        for b in (a + 1)..dec.channel_samples.len() {
            let (x, y) = (&dec.channel_samples[a], &dec.channel_samples[b]);
            let pair_sel = Some(al_dataset::types::ChannelSelector::ChannelPair(
                a as u16, b as u16,
            ));
            let rb = RecordBuilder::new(&dec.input_id, "cheap.v1", ResolutionClass::Segment)
                .lineage(lineage.iter().cloned());
            let mut push = |mut rec: FeatureRecord| {
                rec.channels = pair_sel;
                out.records.push(rec);
            };
            let n = x.len().min(y.len());
            if n == 0 {
                push(rb.scalar(
                    "stereo.pair_correlation",
                    f64::NAN,
                    "corr",
                    None,
                    Confidence::Low,
                    Validity::SilenceInsufficientEnergy,
                    Some("empty channel pair".into()),
                    0,
                ));
                continue;
            }
            let (xs, ys) = (&x[..n], &y[..n]);
            // Zero-lag normalized correlation. sx/sy are also
            // consumed by the Validity gate and `stereo.ild_db`
            // below, so they are computed separately from `corr0`.
            let corr = corr0(xs, ys);
            let sx = sum_sq(xs);
            let sy = sum_sq(ys);
            push(rb.scalar(
                "stereo.pair_correlation",
                corr,
                "corr",
                None,
                Confidence::High,
                if sx > 0.0 && sy > 0.0 {
                    Validity::Valid
                } else {
                    Validity::SilenceInsufficientEnergy
                },
                None,
                n as u64,
            ));
            // ILD: level difference.
            let ild = db(sy.sqrt() / n.max(1) as f64) - db(sx.sqrt() / n.max(1) as f64);
            push(rb.scalar(
                "stereo.ild_db",
                ild,
                "db",
                None,
                Confidence::High,
                Validity::Valid,
                None,
                n as u64,
            ));
            // IPD: NCC peak lag within ±ipd_max_lag.
            let max_lag = (cfg.ipd_max_lag_s * rate as f64).round() as isize;
            let mut best_lag = 0isize;
            let mut best_v = f64::MIN;
            for lag in -max_lag..=max_lag {
                let (xa, ya) = if lag >= 0 {
                    (&xs[lag as usize..], &ys[..n - lag as usize])
                } else {
                    (&xs[..n - (-lag) as usize], &ys[(-lag) as usize..])
                };
                if xa.is_empty() {
                    continue;
                }
                let c = corr0(xa, ya);
                if c > best_v {
                    best_v = c;
                    best_lag = lag;
                }
            }
            let mut ipd = rb.scalar(
                "stereo.ipd_samples",
                best_lag as f64,
                "samples",
                None,
                Confidence::Medium,
                Validity::Valid,
                None,
                n as u64,
            );
            ipd.params = serde_json::json!({"lag_corr": best_v});
            push(ipd);
            // M/S power + band M/S.
            let mid: f64 = xs
                .iter()
                .zip(ys)
                .map(|(&a, &b2)| (a + b2) * (a + b2) / 4.0)
                .sum();
            let side: f64 = xs
                .iter()
                .zip(ys)
                .map(|(&a, &b2)| (a - b2) * (a - b2) / 4.0)
                .sum();
            push(rb.scalar(
                "stereo.ms_power_ratio",
                if mid > 0.0 { side / mid } else { f64::NAN },
                "ratio",
                None,
                Confidence::High,
                if mid > 0.0 {
                    Validity::Valid
                } else {
                    Validity::SilenceInsufficientEnergy
                },
                None,
                n as u64,
            ));
            // Complex coherence + CSD intermediates from the two
            // channels' STFTs (same geometry).
            if let (Some(ma), Some(mb)) = (&channel_stft[a], &channel_stft[b])
                && ma.frames() == mb.frames()
                && ma.fft_size == mb.fft_size
            {
                let frames = ma.frames().min(64);
                let mut csd_re = vec![0.0f64; ma.fft_size];
                let mut csd_im = vec![0.0f64; ma.fft_size];
                let mut paa = vec![0.0f64; ma.fft_size];
                let mut pbb = vec![0.0f64; ma.fft_size];
                for f in 0..frames {
                    for k in 0..ma.fft_size {
                        let (xr, xi) = (ma.re[f][k], ma.im[f][k]);
                        let (yr, yi) = (mb.re[f][k], mb.im[f][k]);
                        csd_re[k] += xr * yr + xi * yi;
                        csd_im[k] += xr * yi - xi * yr;
                        paa[k] += ma.re[f][k] * ma.re[f][k] + ma.im[f][k] * ma.im[f][k];
                        pbb[k] += mb.re[f][k] * mb.re[f][k] + mb.im[f][k] * mb.im[f][k];
                    }
                }
                let pxy_avg: Vec<(f64, f64)> = csd_re
                    .iter()
                    .zip(&csd_im)
                    .map(|(&r, &i)| (r / frames as f64, i / frames as f64))
                    .collect();
                let paa_avg: Vec<f64> = paa.iter().map(|v| v / frames as f64).collect();
                let pbb_avg: Vec<f64> = pbb.iter().map(|v| v / frames as f64).collect();
                let coh = complex_coherence(&paa_avg, &pbb_avg, &pxy_avg);
                let half = ma.fft_size / 2;
                let coh_mag_mean = coh[..half]
                    .iter()
                    .map(|(r, i)| (r * r + i * i).sqrt())
                    .sum::<f64>()
                    / half as f64;
                let mut cr = rb.scalar(
                    "stereo.coherence_mean",
                    coh_mag_mean,
                    "ratio",
                    None,
                    Confidence::Medium,
                    if coh_mag_mean > 0.3 {
                        Validity::Valid
                    } else {
                        Validity::LowCoherence
                    },
                    None,
                    frames as u64,
                );
                cr.channels = pair_sel;
                cr.resolution = Some(Resolution {
                    sample_rate: rate,
                    window: cfg.stft_window as u64,
                    hop: cfg.stft_hop as u64,
                });
                cr.resolution_class = ResolutionClass::Mid20ms;
                out.records.push(cr);
                out.tensors.push(matrix_to_tensor(
                    format!("intermediate/{}/ch{a}-ch{b}/coherence", dec.input_id),
                    &[
                        coh.iter().map(|c| c.0).collect::<Vec<_>>(),
                        coh.iter().map(|c| c.1).collect::<Vec<_>>(),
                    ],
                ));
                out.tensors.push(matrix_to_tensor(
                    format!("intermediate/{}/ch{a}-ch{b}/csd", dec.input_id),
                    &[csd_re, csd_im],
                ));
            }
        }
    }

    // --- Transient / timing family (from mono mix) ---
    if !channel_rms.is_empty() && !channel_rms[0].is_empty() {
        let rms = &channel_rms[0];
        let rb = RecordBuilder::new(&dec.input_id, "cheap.v1", ResolutionClass::Mid20ms)
            .resolution(level_window as u64, level_hop as u64, rate)
            .lineage(lineage.iter().cloned());
        // Onset strength: positive RMS-trajectory derivative peaks.
        let mut onsets = Vec::new();
        for i in 1..rms.len() {
            let d = rms[i] - rms[i - 1];
            let local_med = {
                let lo = i.saturating_sub(8);
                let mut nb: Vec<f64> = rms[lo..=i.min(rms.len() - 1)].to_vec();
                nb.sort_by(|a, b| a.total_cmp(b));
                nb[nb.len() / 2]
            };
            if d > local_med * 1.5 && d > 0.01 {
                onsets.push(i);
            }
        }
        out.records.push(rb.scalar(
            "trans.onset_count",
            onsets.len() as f64,
            "count",
            None,
            Confidence::Medium,
            Validity::Valid,
            None,
            rms.len() as u64,
        ));
        let onset_times: Vec<f64> = onsets
            .iter()
            .map(|&i| i as f64 * level_hop as f64 / rate as f64)
            .collect();
        let ot = matrix_to_tensor(
            format!("features/cheap/{}/onset_times", dec.input_id),
            &[onset_times],
        );
        let mut rec = rb.scalar(
            "trans.onset_times",
            onsets.len() as f64,
            "seconds",
            None,
            Confidence::Medium,
            Validity::Valid,
            None,
            onsets.len() as u64,
        );
        rec.value = Value::Tensor {
            array: ot.path.clone(),
            region: vec![[0, 1], [0, onsets.len() as u64]],
        };
        out.tensors.push(ot);
        out.records.push(rec);
        // Self-similarity: normalized autocorrelation of the RMS
        // trajectory at lag 2-8 windows (periodicity hint).
        if rms.len() > 16 {
            let best = (2..=8)
                .map(|lag| {
                    let a = &rms[..rms.len() - lag];
                    let b = &rms[lag..];
                    corr0(a, b)
                })
                .fold(0.0f64, f64::max);
            out.records.push(rb.scalar(
                "trans.self_similarity",
                best,
                "corr",
                None,
                Confidence::Low,
                Validity::Valid,
                None,
                rms.len() as u64,
            ));
        }
        // Coarse lag modulation between channels (drift of the IPD
        // estimate over time) — first pair only, per 100ms block.
        if dec.channel_samples.len() >= 2 {
            let x = &dec.channel_samples[0];
            let y = &dec.channel_samples[1];
            let n = x.len().min(y.len());
            let block = ((0.1 * rate as f64).round() as usize).max(64);
            let max_lag = ((cfg.ipd_max_lag_s * 4.0) * rate as f64).round() as usize;
            let mut lags = Vec::new();
            for start in (0..n.saturating_sub(block)).step_by(block) {
                let xa = &x[start..start + block];
                let ya = &y[start..start + block];
                let curve = ncc(xa, ya, max_lag);
                let (li, _) = curve
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.total_cmp(b.1))
                    .unwrap_or((0, &0.0));
                lags.push(li as f64 - max_lag as f64);
            }
            if !lags.is_empty() {
                let drift = lags.iter().copied().fold(f64::MIN, f64::max)
                    - lags.iter().copied().fold(f64::MAX, f64::min);
                let mut lr =
                    RecordBuilder::new(&dec.input_id, "cheap.v1", ResolutionClass::Broad100ms)
                        .resolution(block as u64, block as u64, rate)
                        .lineage(lineage.iter().cloned())
                        .scalar(
                            "mod.lag_drift_samples",
                            drift,
                            "samples",
                            None,
                            Confidence::Low,
                            Validity::Valid,
                            None,
                            lags.len() as u64,
                        );
                lr.channels = Some(al_dataset::types::ChannelSelector::ChannelPair(0, 1));
                out.records.push(lr);
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_core::ArtifactId;

    fn dec(samples: Vec<Vec<f64>>, rate: u32) -> DecodedInput {
        let channels = samples.len() as u16;
        DecodedInput {
            input_id: "IN-1".into(),
            sample_rate: rate,
            channels,
            channel_samples: samples,
            artifact_ids: vec![ArtifactId::new()],
        }
    }

    fn rec<'a>(out: &'a CheapResult, id: &str) -> &'a FeatureRecord {
        out.records
            .iter()
            .find(|r| r.feature_id == id)
            .unwrap_or_else(|| panic!("missing record {id}"))
    }

    #[test]
    fn windowed_records_persist_the_decoded_sample_rate() {
        for rate in [32_000, 44_100] {
            let samples: Vec<f64> = (0..8192)
                .map(|i| 0.5 * (2.0 * std::f64::consts::PI * 440.0 * i as f64 / rate as f64).sin())
                .collect();
            let out = analyze_input(
                &dec(vec![samples.clone(), samples], rate),
                &CheapConfig::default(),
            );
            let windowed: Vec<_> = out
                .records
                .iter()
                .filter(|r| r.resolution.is_some())
                .collect();
            assert!(!windowed.is_empty());
            for record in windowed {
                let json = serde_json::to_value(record).unwrap();
                assert_eq!(
                    json["resolution"]["sample_rate"], rate,
                    "{}",
                    record.feature_id
                );
            }
        }
    }

    #[test]
    fn silent_input_marks_silence_validity() {
        let d = dec(vec![vec![0.0; 48000]], 48000);
        let out = analyze_input(&d, &CheapConfig::default());
        let r = rec(&out, "level.rms_dbfs");
        assert_eq!(r.validity, Validity::SilenceInsufficientEnergy);
    }

    #[test]
    fn sine_yields_expected_levels() {
        let rate = 48000;
        let x: Vec<f64> = (0..48000)
            .map(|i| 0.5 * (2.0 * std::f64::consts::PI * 1000.0 * i as f64 / rate as f64).sin())
            .collect();
        let d = dec(vec![x], rate);
        let out = analyze_input(&d, &CheapConfig::default());
        let peak = rec(&out, "level.sample_peak");
        assert!((peak.target.unwrap() - 0.5).abs() < 0.01);
        let rms = rec(&out, "level.rms_dbfs");
        // 0.5 sine → -9.03 dBFS.
        assert!((rms.target.unwrap() - (-9.03)).abs() < 0.3);
        let centroid = rec(&out, "spec.centroid_hz");
        assert!((centroid.target.unwrap() - 1000.0).abs() < 300.0);
        assert_eq!(centroid.resolution_class, ResolutionClass::Mid20ms);
        // Intermediates persisted as tensors.
        assert!(
            out.tensors
                .iter()
                .any(|t| t.path == "intermediate/IN-1/ch0/psd")
        );
        assert!(
            out.tensors
                .iter()
                .any(|t| t.path == "intermediate/IN-1/ch0/stft_re")
        );
    }

    #[test]
    fn clipping_is_detected() {
        let mut x = vec![0.1f64; 48000];
        x[1000] = 1.0;
        x[1001] = 1.0;
        let d = dec(vec![x], 48000);
        let out = analyze_input(&d, &CheapConfig::default());
        assert!(rec(&out, "dig.clipping_rate").target.unwrap() > 0.0);
        assert_eq!(rec(&out, "dig.clip_max_run").target.unwrap(), 2.0);
    }

    #[test]
    fn stereo_pair_features() {
        let rate = 48000;
        let x: Vec<f64> = (0..48000)
            .map(|i| 0.4 * (2.0 * std::f64::consts::PI * 440.0 * i as f64 / rate as f64).sin())
            .collect();
        let y: Vec<f64> = x.iter().map(|v| v * 0.5).collect();
        let d = dec(vec![x, y], rate);
        let out = analyze_input(&d, &CheapConfig::default());
        let corr = rec(&out, "stereo.pair_correlation");
        assert!((corr.target.unwrap() - 1.0).abs() < 0.01);
        let ild = rec(&out, "stereo.ild_db");
        assert!((ild.target.unwrap() - (-6.02)).abs() < 0.3);
        assert!(out.tensors.iter().any(|t| t.path.contains("coherence")));
    }

    #[test]
    fn short_input_reports_insufficient_support() {
        let d = dec(vec![vec![0.5; 500]], 48000);
        let out = analyze_input(&d, &CheapConfig::default());
        let r = rec(&out, "spec.centroid_hz");
        assert_eq!(r.validity, Validity::InsufficientSupport);
    }

    // ==== mechanical-caller-cleanup characterization ====
    // TASK-034EO484s25JM8bsb6zUCh (plan T1). Pins the CURRENT
    // outputs of the cheap-stage sites the mechanical caller
    // cleanup bundle touches — A9 (magnitude/power/psd tensors +
    // `spec.slope`), A12 (`dig.quant_step_est` in-gate and >1M
    // distinct skip), A7 stereo (`stereo.coherence_mean`) — via
    // `f64::to_bits` / element-exact comparison against oracles
    // that re-implement the current formulas over public API.
    // No tolerance predicate is applied (test-spec Rules C/D/E).

    /// Deterministic zero-mean pseudo-noise (LCG, same generator
    /// shape as the other in-tree test fixtures).
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

    fn tensor<'a>(out: &'a CheapResult, path: &str) -> &'a NamedTensor {
        out.tensors
            .iter()
            .find(|t| t.path == path)
            .unwrap_or_else(|| panic!("missing tensor {path}"))
    }

    fn assert_bits(actual: &[f64], expected: &[f64], what: &str) {
        assert_eq!(
            actual.len(),
            expected.len(),
            "{what} length {} vs {}",
            actual.len(),
            expected.len()
        );
        for (i, (&a, &e)) in actual.iter().zip(expected).enumerate() {
            assert_eq!(a.to_bits(), e.to_bits(), "{what}[{i}]");
        }
    }

    fn flat(m: &[Vec<f64>]) -> Vec<f64> {
        m.iter().flatten().copied().collect()
    }

    /// A9: the persisted magnitude/power/psd tensors must equal the
    /// `StftMatrix::{magnitude,power,psd}` outputs on the same
    /// STFT — the single `power()`-derivation change must keep
    /// every element bit-identical.
    #[test]
    fn a9_mag_pow_psd_tensors_are_bit_exact() {
        let rate = 48_000u32;
        let samples = noise(32_768, 0x5eed);
        let d = dec(vec![samples.clone()], rate);
        let cfg = CheapConfig::default();
        let out = analyze_input(&d, &cfg);

        let m = stft(&samples, cfg.stft_window, cfg.stft_hop, rate);
        assert!(m.frames() > 0);
        let mag_t = tensor(&out, "intermediate/IN-1/ch0/magnitude");
        let pow_t = tensor(&out, "intermediate/IN-1/ch0/power");
        let psd_t = tensor(&out, "intermediate/IN-1/ch0/psd");
        assert_eq!(mag_t.shape, [m.frames() as u64, m.fft_size as u64]);
        assert_bits(&mag_t.data, &flat(&m.magnitude()), "magnitude");
        assert_bits(&pow_t.data, &flat(&m.power()), "power");
        assert_bits(&psd_t.data, &flat(&m.psd()), "psd");
    }

    /// A9: `spec.slope` consumes `mean_row` (per-bin row-mean of
    /// `mag[..half]`). The row-major fused accumulation must keep
    /// the record bit-identical to the current column-scan value.
    #[test]
    fn a9_spec_slope_record_is_bit_exact() {
        let rate = 48_000u32;
        let samples = noise(32_768, 0xbeef);
        let d = dec(vec![samples.clone()], rate);
        let cfg = CheapConfig::default();
        let out = analyze_input(&d, &cfg);

        // Oracle: the current mean_row + log-log linear fit shape.
        let m = stft(&samples, cfg.stft_window, cfg.stft_hop, rate);
        let mag = m.magnitude();
        let half = mag[0].len() / 2;
        let bin_hz = rate as f64 / (half * 2) as f64;
        let mean_row: Vec<f64> = (0..half)
            .map(|b| mag.iter().map(|r| r[b]).sum::<f64>() / mag.len() as f64)
            .collect();
        let xs: Vec<f64> = (1..(half * 9 / 10).max(2))
            .map(|b| (b as f64 * bin_hz).ln())
            .collect();
        let ys: Vec<f64> = (1..(half * 9 / 10).max(2))
            .map(|b| (mean_row[b] + 1e-12).ln())
            .collect();
        let n = xs.len() as f64;
        let (sx, sy, sxy, sxx) = xs
            .iter()
            .zip(&ys)
            .fold((0.0, 0.0, 0.0, 0.0), |(ax, ay, axy, axx), (&x, &y)| {
                (ax + x, ay + y, axy + x * y, axx + x * x)
            });
        let expected = if sxx * n - sx * sx != 0.0 {
            (sxy * n - sx * sy) / (sxx * n - sx * sx)
        } else {
            0.0
        };
        let r = rec(&out, "spec.slope");
        assert_eq!(
            r.target.unwrap().to_bits(),
            expected.to_bits(),
            "spec.slope"
        );
    }

    /// A12 oracle: the current `uniq` sort/dedup → gate → f64
    /// `total_cmp` sort → min positive difference shape. Returns
    /// `(quant, support)`.
    fn quant_oracle(samples: &[f64]) -> (f64, u64) {
        let mut uniq: Vec<u64> = samples.iter().map(|v| v.to_bits()).collect();
        uniq.sort_unstable();
        uniq.dedup();
        let quant = if uniq.len() >= 2 && uniq.len() <= 1_000_000 {
            let mut sorted: Vec<f64> = uniq.iter().map(|b| f64::from_bits(*b)).collect();
            sorted.sort_by(|a, b| a.total_cmp(b));
            sorted
                .windows(2)
                .map(|w| w[1] - w[0])
                .filter(|d| *d > 0.0)
                .fold(f64::INFINITY, f64::min)
        } else {
            f64::NAN
        };
        (quant, uniq.len() as u64)
    }

    fn assert_quant_step(out: &CheapResult, samples: &[f64], what: &str) {
        let (expected, support) = quant_oracle(samples);
        let r = rec(out, "dig.quant_step_est");
        assert_eq!(
            r.target.unwrap().to_bits(),
            expected.to_bits(),
            "{what} quant_step value"
        );
        assert_eq!(r.support_count, support, "{what} support");
        assert_eq!(
            r.validity,
            if expected.is_finite() {
                Validity::Valid
            } else {
                Validity::NotApplicable
            },
            "{what} validity"
        );
    }

    /// A12 in-gate path (2..=1M distinct): a 1/16-grid quantized
    /// signal exercises the sort → min-positive-difference branch.
    #[test]
    fn a12_quant_step_in_gate_is_bit_exact() {
        let rate = 48_000u32;
        let samples: Vec<f64> = noise(65_536, 0x9e37)
            .iter()
            .map(|&v| (v * 16.0).round() / 16.0)
            .collect();
        let (expected, support) = quant_oracle(&samples);
        assert!((2..=1_000_000).contains(&support), "in-gate premise");
        assert!(expected.is_finite(), "in-gate must yield a step");
        let out = analyze_input(&dec(vec![samples.clone()], rate), &CheapConfig::default());
        assert_quant_step(&out, &samples, "in_gate");
    }

    /// A12 >1M distinct skip path: the gate fails and the record
    /// reports NaN / `NotApplicable` with the distinct count as
    /// support — same record semantics the HashSet pre-count keeps.
    #[test]
    fn a12_quant_step_over_1m_skip_is_bit_exact() {
        let rate = 48_000u32;
        let samples = noise(1_300_000, 0x51ed);
        let (_, support) = quant_oracle(&samples);
        assert!(
            support > 1_000_000,
            "fixture must exceed the gate: {support}"
        );
        let out = analyze_input(&dec(vec![samples.clone()], rate), &CheapConfig::default());
        assert_quant_step(&out, &samples, "over_1m");
    }

    /// A7 stereo site: `stereo.coherence_mean` is the mean magnitude
    /// of the complex coherence between channels' STFTs, over at
    /// most `min(64)` frames of `csd` accumulation.
    #[test]
    fn a7_stereo_coherence_mean_is_bit_exact() {
        let rate = 48_000u32;
        let a = noise(65_536, 0xa11);
        let b: Vec<f64> = a
            .iter()
            .enumerate()
            .map(|(i, &v)| if i >= 11 { 0.6 * a[i - 11] } else { v * 0.3 })
            .collect();
        let d = dec(vec![a.clone(), b.clone()], rate);
        let cfg = CheapConfig::default();
        let out = analyze_input(&d, &cfg);

        // `csd` stays public (the production import is pruned in the
        // A7A8 commit); keep the oracle import local to this test.
        use crate::stft::csd;
        // Oracle: the cheap.rs stereo-pair accumulation shape.
        let ma = stft(&a, cfg.stft_window, cfg.stft_hop, rate);
        let mb = stft(&b, cfg.stft_window, cfg.stft_hop, rate);
        assert_eq!(ma.frames(), mb.frames());
        assert_eq!(ma.fft_size, mb.fft_size);
        let frames = ma.frames().min(64);
        let mut csd_re = vec![0.0f64; ma.fft_size];
        let mut csd_im = vec![0.0f64; ma.fft_size];
        let mut paa = vec![0.0f64; ma.fft_size];
        let mut pbb = vec![0.0f64; ma.fft_size];
        for f in 0..frames {
            let pxy = csd(&ma.re[f], &ma.im[f], &mb.re[f], &mb.im[f]);
            for (k, &(pr, pi)) in pxy.iter().enumerate() {
                csd_re[k] += pr;
                csd_im[k] += pi;
                paa[k] += ma.re[f][k] * ma.re[f][k] + ma.im[f][k] * ma.im[f][k];
                pbb[k] += mb.re[f][k] * mb.re[f][k] + mb.im[f][k] * mb.im[f][k];
            }
        }
        let pxy_avg: Vec<(f64, f64)> = csd_re
            .iter()
            .zip(&csd_im)
            .map(|(&r, &i)| (r / frames as f64, i / frames as f64))
            .collect();
        let paa_avg: Vec<f64> = paa.iter().map(|v| v / frames as f64).collect();
        let pbb_avg: Vec<f64> = pbb.iter().map(|v| v / frames as f64).collect();
        let coh = complex_coherence(&paa_avg, &pbb_avg, &pxy_avg);
        let half = ma.fft_size / 2;
        let expected = coh[..half]
            .iter()
            .map(|(r, i)| (r * r + i * i).sqrt())
            .sum::<f64>()
            / half as f64;
        let r = rec(&out, "stereo.coherence_mean");
        assert_eq!(
            r.target.unwrap().to_bits(),
            expected.to_bits(),
            "stereo.coherence_mean"
        );
    }
}
