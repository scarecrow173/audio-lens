//! Reference-aligned comparison features: what an accepted
//! alignment says about how the capture differs from the source —
//! without collapsing the answer into one similarity score.
//!
//! Every measurement lands as a `FeatureRecord` under
//! `features/reference`; rejected/ambiguous alignments are recorded
//! as invalid rows (`AmbiguousAlignment`), never as stage failure.
//! Staged residuals R0–R3 keep waveform + STFT + PSD + band energy +
//! RMS ratio + source correlation under the `residual` role.

use al_core::{Confidence, Validity};
use al_dataset::types::{FeatureRecord, ResolutionClass};
use al_locate::dsp::{bandpass, conv, corr0, dot, fft, gcc_phat, rms, spectral_envelope, sum_sq};
use al_locate::mixer::fit_mix;

use crate::cheap::NamedTensor;
use crate::records::{RecordBuilder, propagate_confidence, propagate_validity, robust_summary};
use crate::stft::{StftMatrix, band_power, complex_coherence, csd, stft, third_octave_bands};

/// Reference-stage tunables (`config.analysis.reference.*`).
#[derive(Debug, Clone)]
pub struct ReferenceConfig {
    /// STFT window for spectral comparison (samples).
    pub stft_window: usize,
    /// STFT hop (samples).
    pub stft_hop: usize,
    /// Lag-tracking window (samples).
    pub lag_window: usize,
    /// Lag-tracking hop (samples).
    pub lag_hop: usize,
    /// Maximum per-window lag search (samples).
    pub lag_max: usize,
    /// 帯域推定窓 (秒)。実レートから標本数へ変換する。
    pub band_window_s: f64,
    /// 帯域推定 hop (秒)。
    pub band_hop_s: f64,
    /// 帯域ラグの最大絶対値 (秒)。
    pub band_max_lag_s: f64,
    /// Gain trajectory window (samples).
    pub gain_window: usize,
    /// Mixer ridge parameter.
    pub ridge: f64,
}

impl Default for ReferenceConfig {
    fn default() -> Self {
        Self {
            stft_window: 4096,
            stft_hop: 2048,
            lag_window: 8192,
            lag_hop: 4096,
            lag_max: 512,
            band_window_s: 0.1,
            band_hop_s: 0.05,
            band_max_lag_s: 0.04,
            gain_window: 4096,
            ridge: 1e-6,
        }
    }
}

impl ReferenceConfig {
    /// Overlay `config.analysis.reference` onto the defaults.
    pub fn from_job(config: &serde_json::Value) -> Self {
        let c = &config["analysis"]["reference"];
        let mut cfg = Self::default();
        for (key, dest) in [
            ("band_window_s", &mut cfg.band_window_s),
            ("band_hop_s", &mut cfg.band_hop_s),
            ("band_max_lag_s", &mut cfg.band_max_lag_s),
        ] {
            if let Some(v) = c[key]
                .as_f64()
                .filter(|v| v.is_finite() && *v > 0.0 && *v <= 1.0)
            {
                *dest = v;
            }
        }
        if let Some(v) = c["stft_window"].as_u64() {
            cfg.stft_window = v as usize;
        }
        if let Some(v) = c["stft_hop"].as_u64() {
            cfg.stft_hop = v as usize;
        }
        if let Some(v) = c["lag_window"].as_u64() {
            cfg.lag_window = v as usize;
        }
        if let Some(v) = c["lag_max"].as_u64() {
            cfg.lag_max = v as usize;
        }
        cfg
    }
}

/// Residual audio to persist under the `residual` role.
#[derive(Debug)]
pub struct ResidualAudio {
    /// Dataset-relative name (`<src>__<cap>/r0`).
    pub name: String,
    /// Interleaved samples (all channels).
    pub samples: Vec<f64>,
    /// Channel count.
    pub channels: u16,
    /// Sample rate.
    pub sample_rate: u32,
}

/// Everything one accepted `(source, capture)` pair produced.
#[derive(Debug, Default)]
pub struct ReferenceResult {
    /// Feature records (JSON rows under `features/reference/...`).
    pub records: Vec<FeatureRecord>,
    /// Tensors for `intermediate` / `residual` roles.
    pub tensors: Vec<NamedTensor>,
    /// Residual waveforms R0–R3.
    pub residual_audio: Vec<ResidualAudio>,
}

/// Least-squares gain `g` minimizing `||y - g·x||`. The zero
/// denominator guard is load-bearing: `dot / sum_sq` would yield
/// NaN on a silent `x`.
fn ls_gain(x: &[f64], y: &[f64]) -> f64 {
    let den = sum_sq(x);
    if den > 0.0 { dot(x, y) / den } else { 0.0 }
}

/// Normalized correlation of `x` and `y` at relative `lag` (y
/// shifted by `lag` samples; positive lag = y later). The shorter
/// overlap decides the support.
fn corr_at(x: &[f64], y: &[f64], lag: isize) -> f64 {
    let (xs, ys) = if lag >= 0 {
        (
            &x[..x.len() - (lag as usize).min(x.len())],
            &y[lag as usize..],
        )
    } else {
        (
            &x[(-lag) as usize..],
            &y[..y.len() - (-lag as usize).min(y.len())],
        )
    };
    let n = xs.len().min(ys.len());
    if n == 0 {
        return 0.0;
    }
    corr0(&xs[..n], &ys[..n])
}

/// Peak lag + score + half-max width over `[-max_lag, max_lag]`.
fn local_lag(x: &[f64], y: &[f64], max_lag: usize) -> (isize, f64, f64) {
    let max_lag = max_lag as isize;
    let mut best = (0isize, f64::MIN);
    let mut curve = Vec::with_capacity((2 * max_lag + 1) as usize);
    for lag in -max_lag..=max_lag {
        let c = corr_at(x, y, lag);
        curve.push(c);
        if c > best.1 {
            best = (lag, c);
        }
    }
    let half = best.1 * 0.5;
    let width = curve.iter().filter(|&&v| v >= half).count() as f64;
    (best.0, best.1, width)
}

/// Welch-estimated impulse response: the mean cross-spectral
/// density of `from_stft`→`to` inverse-transformed to a real IR.
/// Returns a sequence of length `from_stft.fft_size` (not yet
/// truncated to the convolution cap). When no frame pair is
/// available (`fr == 0`) a zero IR of the same length is returned —
/// the caller convolves unconditionally and a zero IR yields a zero
/// model.
fn fir_ir(from_stft: &StftMatrix, to: &[f64], window: usize, hop: usize, rate: u32) -> Vec<f64> {
    let mt = stft(to, window, hop, rate);
    let fr = from_stft.frames().min(mt.frames());
    let bins = from_stft.fft_size;
    if fr == 0 {
        return vec![0.0; bins];
    }
    let mut cxx = vec![0.0; bins];
    let mut cxy = vec![(0.0, 0.0); bins];
    for f in 0..fr {
        let c = csd(&from_stft.re[f], &from_stft.im[f], &mt.re[f], &mt.im[f]);
        for k in 0..bins {
            cxx[k] += from_stft.re[f][k].powi(2) + from_stft.im[f][k].powi(2);
            cxy[k].0 += c[k].0;
            cxy[k].1 += c[k].1;
        }
    }
    let mut hre: Vec<f64> = (0..bins)
        .map(|k| if cxx[k] > 0.0 { cxy[k].0 / cxx[k] } else { 0.0 })
        .collect();
    let mut him: Vec<f64> = (0..bins)
        .map(|k| if cxx[k] > 0.0 { cxy[k].1 / cxx[k] } else { 0.0 })
        .collect();
    // irfft (inverse FFT already normalizes by 1/N).
    fft(&mut hre, &mut him, true);
    hre
}

/// FIR model of `from` explaining `to`: convolves `from` with the
/// Welch-estimated impulse response and returns a `from`-length
/// model. `conv_len = ir.len().min(4096)` caps the kernel and the
/// output is the first `from.len()` elements of the linear
/// convolution via `dsp::conv` (overlap-save) — the single
/// convolution-path switch point.
fn fir_model(
    from: &[f64],
    to: &[f64],
    from_stft: &StftMatrix,
    cfg: &ReferenceConfig,
    rate: u32,
) -> Vec<f64> {
    let ir = fir_ir(from_stft, to, cfg.stft_window, cfg.stft_hop, rate);
    conv(from, &ir[..ir.len().min(4096)])
}

/// Per-channel elementwise `residual - model`. Model entries that
/// are missing or shorter than the residual are zero-filled
/// (`m.get(i).copied().unwrap_or(0.0)`): a structural guard only —
/// production inputs share the aligned length
/// (`stage.rs` Aligned contract) and never go ragged.
fn subtract(residual: &[Vec<f64>], model: &[Vec<f64>]) -> Vec<Vec<f64>> {
    residual
        .iter()
        .enumerate()
        .map(|(c, ch)| {
            ch.iter()
                .enumerate()
                .map(|(i, &v)| {
                    v - model
                        .get(c)
                        .map_or(0.0, |m| m.get(i).copied().unwrap_or(0.0))
                })
                .collect()
        })
        .collect()
}

pub(crate) fn matrix_to_tensor(path: String, m: &[Vec<f64>]) -> NamedTensor {
    let rows = m.len() as u64;
    let cols = m.first().map_or(0, |r| r.len()) as u64;
    let mut data = Vec::with_capacity(m.iter().map(Vec::len).sum());
    for row in m {
        data.extend_from_slice(row);
    }
    debug_assert_eq!(data.len() as u64, rows * cols, "ragged tensor {path}");
    NamedTensor {
        path,
        shape: [rows, cols],
        data,
    }
}

/// One aligned analysis window pair.
pub struct Aligned<'a> {
    /// Source channel samples restricted to the shared length.
    pub src: Vec<&'a [f64]>,
    /// Capture channel samples restricted to the shared length.
    pub cap: Vec<&'a [f64]>,
    /// Sample rate.
    pub rate: u32,
    /// Upstream confidence from the alignment.
    pub upstream_confidence: Confidence,
}

/// Analyze one `(source, capture)` pair that has an *accepted*
/// locate result. `pair` is the dataset path stem (`<src>__<cap>`)
/// used for artifact naming; `alignment_id` is the id issued by the
/// locate stage for this result and is stamped onto every record so
/// rows and artifact provenance reference the same alignment.
/// `src_seg`/`cap_seg` are the aligned channel slices (same length).
pub fn analyze_pair(
    pair: &str,
    alignment_id: &al_core::AlignmentId,
    src: &Aligned<'_>,
    upstream_validity: Validity,
    cfg: &ReferenceConfig,
) -> ReferenceResult {
    let mut out = ReferenceResult::default();
    let alignment_id = alignment_id.as_str();
    let lineage: Vec<String> = Vec::new(); // filled by caller via params
    let up_conf = src.upstream_confidence;
    let up_val = propagate_validity(upstream_validity);
    let valid = up_val == Validity::Valid;
    let input_id = pair.split("__").next().unwrap_or(pair).to_string();
    let n = src.src.first().map_or(0, |c| c.len());

    let rb = |class: ResolutionClass| {
        RecordBuilder::new(&input_id, "reference.v1", class).lineage(lineage.iter().cloned())
    };
    let mut push_invalid = |id: &str| {
        let mut r = rb(ResolutionClass::Segment).scalar(
            id,
            f64::NAN,
            "none",
            None,
            Confidence::Low,
            up_val,
            Some(format!("upstream validity {upstream_validity:?}")),
            0,
        );
        r.alignment_id = Some(alignment_id.to_string());
        out.records.push(r);
    };

    if !valid || n == 0 {
        for id in [
            "ref.lag_median",
            "ref.gain_ls",
            "ref.band_gain_m2",
            "ref.similarity_fullband",
            "ref.transfer_h",
            "ref.mixer_fit",
            "ref.residual_r3_ratio",
        ] {
            push_invalid(id);
        }
        return out;
    }

    let rate = src.rate;
    let src_mono = src.src[0];
    let cap_mono = src.cap[0];

    // ==== Lag family: per-window local lag τ(t) ====
    let mut lags = Vec::new();
    let mut lag_scores = Vec::new();
    let mut lag_widths = Vec::new();
    if n >= cfg.lag_window {
        for start in (0..=n - cfg.lag_window).step_by(cfg.lag_hop) {
            let xa = &src_mono[start..start + cfg.lag_window];
            let ya = &cap_mono[start..start + cfg.lag_window];
            let (lag, score, width) = local_lag(xa, ya, cfg.lag_max);
            lags.push(lag as f64);
            lag_scores.push(score);
            lag_widths.push(width);
        }
    }
    let lag_rb =
        rb(ResolutionClass::Broad100ms).resolution(cfg.lag_window as u64, cfg.lag_hop as u64, rate);
    if let Some(s) = robust_summary(&lags) {
        let lag_conf = propagate_confidence(
            up_conf,
            if robust_summary(&lag_scores).map(|x| x.median).unwrap_or(0.0) > 0.5 {
                Confidence::High
            } else {
                Confidence::Medium
            },
        );
        for (id, v, unit) in [
            ("ref.lag_median", s.median, "samples"),
            ("ref.lag_mad", s.mad, "samples"),
            ("ref.lag_p05", s.p05, "samples"),
            ("ref.lag_p95", s.p95, "samples"),
        ] {
            let mut r = lag_rb.scalar(id, v, unit, None, lag_conf, up_val, None, s.support);
            r.alignment_id = Some(alignment_id.to_string());
            out.records.push(r);
        }
        // Drift: linear fit slope of τ(t) → samples/sample; local
        // time scale = 1 - drift.
        if lags.len() >= 2 {
            let (sx, sy, sxy, sxx) = lags.iter().enumerate().fold(
                (0.0, 0.0, 0.0, 0.0),
                |(ax, ay, axy, axx), (i, &y)| {
                    let x = i as f64;
                    (ax + x, ay + y, axy + x * y, axx + x * x)
                },
            );
            let nn = lags.len() as f64;
            let slope = if sxx * nn - sx * sx != 0.0 {
                (sxy * nn - sx * sy) / (sxx * nn - sx * sx)
            } else {
                0.0
            };
            let mut r = lag_rb.scalar(
                "ref.lag_drift",
                slope,
                "samples_per_window",
                None,
                lag_conf,
                up_val,
                None,
                s.support,
            );
            r.alignment_id = Some(alignment_id.to_string());
            out.records.push(r);
            let mut ts = lag_rb.scalar(
                "ref.local_time_scale",
                1.0 - slope,
                "ratio",
                None,
                lag_conf,
                up_val,
                None,
                s.support,
            );
            ts.alignment_id = Some(alignment_id.to_string());
            out.records.push(ts);
        }
        // Step candidates: |Δτ| above 4× MAD.
        let mad = s.mad.max(1.0);
        let steps = lags
            .windows(2)
            .filter(|w| (w[1] - w[0]).abs() > 4.0 * mad)
            .count() as f64;
        let mut st = lag_rb.scalar(
            "ref.lag_step_candidates",
            steps,
            "count",
            None,
            lag_conf,
            up_val,
            None,
            s.support,
        );
        st.alignment_id = Some(alignment_id.to_string());
        out.records.push(st);
        if let Some(w) = robust_summary(&lag_widths) {
            let mut pw = lag_rb.scalar(
                "ref.lag_peak_width_median",
                w.median,
                "lags",
                None,
                lag_conf,
                up_val,
                None,
                w.support,
            );
            pw.alignment_id = Some(alignment_id.to_string());
            out.records.push(pw);
        }
        // τ(t) trajectory as a tensor for downstream reuse.
        out.tensors.push(matrix_to_tensor(
            format!("intermediate/{pair}/lag_trajectory"),
            &[lags.clone()],
        ));
    } else {
        let mut r = rb(ResolutionClass::Segment).scalar(
            "ref.lag_median",
            f64::NAN,
            "samples",
            None,
            Confidence::Low,
            Validity::InsufficientSupport,
            Some("aligned region shorter than lag_window".into()),
            0,
        );
        r.alignment_id = Some(alignment_id.to_string());
        out.records.push(r);
    }

    // ==== Gain family ====
    let g_ls = ls_gain(src_mono, cap_mono);
    let g_rb = rb(ResolutionClass::Segment);
    let mut g = g_rb.scalar(
        "ref.gain_ls",
        g_ls,
        "ratio",
        None,
        up_conf,
        up_val,
        None,
        n as u64,
    );
    g.alignment_id = Some(alignment_id.to_string());
    out.records.push(g);
    // Segment RMS ratio + g(t) trajectory + src-level→gain relation.
    let ratio = rms(cap_mono) / rms(src_mono).max(1e-12);
    let mut rr = g_rb.scalar(
        "ref.rms_ratio",
        ratio,
        "ratio",
        None,
        up_conf,
        up_val,
        None,
        n as u64,
    );
    rr.alignment_id = Some(alignment_id.to_string());
    out.records.push(rr);
    let gw = cfg.gain_window.min(n);
    if gw >= 64 {
        let mut gains = Vec::new();
        let mut src_levels = Vec::new();
        for start in (0..=n - gw).step_by(gw) {
            gains.push(ls_gain(
                &src_mono[start..start + gw],
                &cap_mono[start..start + gw],
            ));
            src_levels.push(rms(&src_mono[start..start + gw]));
        }
        out.tensors.push(matrix_to_tensor(
            format!("intermediate/{pair}/gain_trajectory"),
            std::slice::from_ref(&gains),
        ));
        let lvl_gain_corr = corr0(&src_levels, &gains);
        let mut lc = g_rb.scalar(
            "ref.src_level_gain_corr",
            lvl_gain_corr,
            "corr",
            None,
            propagate_confidence(up_conf, Confidence::Medium),
            up_val,
            None,
            gains.len() as u64,
        );
        lc.alignment_id = Some(alignment_id.to_string());
        out.records.push(lc);
    }

    // ==== Spectral intermediates: STFT of aligned signals ====
    let ms = stft(src_mono, cfg.stft_window, cfg.stft_hop, rate);
    let mc = stft(cap_mono, cfg.stft_window, cfg.stft_hop, rate);
    let frames = ms.frames().min(mc.frames());
    if frames == 0 {
        let mut r = rb(ResolutionClass::Segment).scalar(
            "ref.transfer_h",
            f64::NAN,
            "none",
            None,
            Confidence::Low,
            Validity::InsufficientSupport,
            Some("aligned region shorter than stft window".into()),
            0,
        );
        r.alignment_id = Some(alignment_id.to_string());
        out.records.push(r);
        return out;
    }

    // Welch-averaged CSD/PSD → H(f), coherence.
    let bins = ms.fft_size;
    let mut pxx = vec![0.0; bins];
    let mut pyy = vec![0.0; bins];
    let mut pxy = vec![(0.0, 0.0); bins];
    for f in 0..frames {
        let c = csd(&ms.re[f], &ms.im[f], &mc.re[f], &mc.im[f]);
        for k in 0..bins {
            pxx[k] += ms.re[f][k] * ms.re[f][k] + ms.im[f][k] * ms.im[f][k];
            pyy[k] += mc.re[f][k] * mc.re[f][k] + mc.im[f][k] * mc.im[f][k];
            pxy[k].0 += c[k].0;
            pxy[k].1 += c[k].1;
        }
    }
    for k in 0..bins {
        pxx[k] /= frames as f64;
        pyy[k] /= frames as f64;
        pxy[k].0 /= frames as f64;
        pxy[k].1 /= frames as f64;
    }
    let coherence = complex_coherence(&pxx, &pyy, &pxy);
    let h: Vec<(f64, f64)> = pxx
        .iter()
        .zip(&pxy)
        .map(|(&xx, &(pr, pi))| {
            if xx > 0.0 {
                (pr / xx, pi / xx)
            } else {
                (0.0, 0.0)
            }
        })
        .collect();
    let half = bins / 2;
    out.tensors.push(matrix_to_tensor(
        format!("intermediate/{pair}/transfer_h"),
        &[
            h.iter().map(|v| v.0).collect::<Vec<_>>(),
            h.iter().map(|v| v.1).collect::<Vec<_>>(),
        ],
    ));
    out.tensors.push(matrix_to_tensor(
        format!("intermediate/{pair}/coherence"),
        &[
            coherence.iter().map(|v| v.0).collect::<Vec<_>>(),
            coherence.iter().map(|v| v.1).collect::<Vec<_>>(),
        ],
    ));
    // H(f,t) magnitude tensor (per-frame ratio where supported).
    let hft: Vec<Vec<f64>> = (0..frames)
        .map(|f| {
            (0..half)
                .map(|k| {
                    let xs = ms.re[f][k] * ms.re[f][k] + ms.im[f][k] * ms.im[f][k];
                    if xs > 0.0 {
                        let c = csd(
                            &ms.re[f][k..k + 1],
                            &ms.im[f][k..k + 1],
                            &mc.re[f][k..k + 1],
                            &mc.im[f][k..k + 1],
                        );
                        ((c[0].0 * c[0].0 + c[0].1 * c[0].1).sqrt()) / xs
                    } else {
                        0.0
                    }
                })
                .collect()
        })
        .collect();
    out.tensors.push(matrix_to_tensor(
        format!("intermediate/{pair}/transfer_h_t"),
        &hft,
    ));
    // Magnitude / unwrapped phase / group delay records.
    let mag_h: Vec<f64> = h[..half]
        .iter()
        .map(|(r, i)| (r * r + i * i).sqrt())
        .collect();
    let phase: Vec<f64> = h[..half].iter().map(|(r, i)| i.atan2(*r)).collect();
    let mut unwrapped = phase.clone();
    for i in 1..unwrapped.len() {
        let d = unwrapped[i] - unwrapped[i - 1];
        let mut adj = 0.0;
        while d + adj > std::f64::consts::PI {
            adj -= 2.0 * std::f64::consts::PI;
        }
        while d + adj < -std::f64::consts::PI {
            adj += 2.0 * std::f64::consts::PI;
        }
        unwrapped[i] = unwrapped[i - 1] + d + adj;
    }
    let bin_hz = rate as f64 / bins as f64;
    let group_delay: Vec<f64> = unwrapped
        .windows(2)
        .map(|w| -(w[1] - w[0]) / (2.0 * std::f64::consts::PI * bin_hz))
        .collect();
    out.tensors.push(matrix_to_tensor(
        format!("intermediate/{pair}/transfer_detail"),
        &[mag_h.clone(), unwrapped, {
            let mut gd = group_delay.clone();
            gd.insert(0, 0.0);
            gd
        }],
    ));
    if let Some(s) = robust_summary(&group_delay) {
        let mut r = rb(ResolutionClass::Segment).scalar(
            "ref.group_delay_median_s",
            s.median,
            "seconds",
            None,
            propagate_confidence(up_conf, Confidence::Medium),
            up_val,
            None,
            s.support,
        );
        r.alignment_id = Some(alignment_id.to_string());
        out.records.push(r);
    }

    // ==== Band family: 1/3-octave M2 gain + 400Hz ratio + PSD ratio ====
    let bands = third_octave_bands(40.0, (rate as f64 / 2.0).min(16000.0));
    let psd_s = ms.psd();
    let psd_c = mc.psd();
    let mean_psd_s: Vec<f64> = (0..bins)
        .map(|k| psd_s.iter().take(frames).map(|r| r[k]).sum::<f64>() / frames as f64)
        .collect();
    let mean_psd_c: Vec<f64> = (0..bins)
        .map(|k| psd_c.iter().take(frames).map(|r| r[k]).sum::<f64>() / frames as f64)
        .collect();
    let band_e_s = band_power(&mean_psd_s, rate, bins, &bands);
    let band_e_c = band_power(&mean_psd_c, rate, bins, &bands);
    let band_gains: Vec<f64> = band_e_s
        .iter()
        .zip(&band_e_c)
        .map(|(&s, &c)| if s > 0.0 { (c / s).sqrt() } else { f64::NAN })
        .collect();
    // Band support: mean coherence magnitude inside each band.
    let band_coh: Vec<f64> = bands
        .iter()
        .map(|&(lo, hi)| {
            let b0 = (lo / bin_hz) as usize;
            let b1 = ((hi / bin_hz).ceil() as usize).min(half);
            if b1 <= b0 {
                return 0.0;
            }
            coherence[b0..b1]
                .iter()
                .map(|(r, i)| (r * r + i * i).sqrt())
                .sum::<f64>()
                / (b1 - b0) as f64
        })
        .collect();
    out.tensors.push(matrix_to_tensor(
        format!("intermediate/{pair}/band_gain_psd_ratio"),
        &[
            bands.iter().map(|b| (b.0 + b.1) / 2.0).collect::<Vec<_>>(),
            band_gains.clone(),
            band_coh.clone(),
        ],
    ));
    for (i, ((&g, &coh), &(lo, hi))) in band_gains.iter().zip(&band_coh).zip(&bands).enumerate() {
        let mut r = rb(ResolutionClass::Segment).scalar(
            "ref.band_gain_psd_ratio",
            g,
            "ratio",
            None,
            propagate_confidence(up_conf, Confidence::Low),
            if !g.is_finite() {
                Validity::SilenceInsufficientEnergy
            } else if coh < 0.2 {
                Validity::LowCoherence
            } else {
                up_val
            },
            (coh < 0.2).then(|| "secondary PSD evidence: low coherence".into()),
            frames as u64,
        );
        r.alignment_id = Some(alignment_id.to_string());
        r.freq_range = Some(al_dataset::types::Range { start: lo, end: hi });
        r.params = serde_json::json!({"band_index":i,"band_coherence":coh,"estimator":"sqrt_psd_ratio","secondary_evidence":true});
        out.records.push(r);
    }
    crate::band_gain::analyze(pair, alignment_id, src, cfg, &mut out);
    // PSD ratio tensor (secondary evidence).
    let psd_ratio: Vec<f64> = mean_psd_s
        .iter()
        .zip(&mean_psd_c)
        .map(|(&s, &c)| if s > 0.0 { c / s } else { 0.0 })
        .collect();
    out.tensors.push(matrix_to_tensor(
        format!("intermediate/{pair}/psd_ratio"),
        std::slice::from_ref(&psd_ratio),
    ));

    // ==== Similarity family ====
    let full_corr = corr0(src_mono, cap_mono);
    let mut sim = rb(ResolutionClass::Segment).scalar(
        "ref.similarity_fullband",
        full_corr,
        "corr",
        None,
        up_conf,
        up_val,
        None,
        n as u64,
    );
    sim.alignment_id = Some(alignment_id.to_string());
    out.records.push(sim);
    // Band correlation: bandpass each third-octave band, corr per
    // band (first 8 bands to bound cost).
    let mut band_corrs = Vec::new();
    for &(lo, hi) in bands.iter().take(8) {
        let xs = bandpass(src_mono, rate, lo, hi);
        let ys = bandpass(cap_mono, rate, lo, hi);
        band_corrs.push(corr0(&xs, &ys));
    }
    out.tensors.push(matrix_to_tensor(
        format!("intermediate/{pair}/band_correlation"),
        std::slice::from_ref(&band_corrs),
    ));
    if let Some(&worst) = band_corrs.iter().min_by(|a, b| a.total_cmp(b)) {
        let mut bc = rb(ResolutionClass::Segment).scalar(
            "ref.band_correlation_min",
            worst,
            "corr",
            None,
            propagate_confidence(up_conf, Confidence::Medium),
            up_val,
            None,
            band_corrs.len() as u64,
        );
        bc.alignment_id = Some(alignment_id.to_string());
        out.records.push(bc);
    }
    // PHAT evidence: whiten the cross-spectrum of a source template
    // inside the capture and take the peak correlation.
    let tpl_len = n.min(8192);
    let phat = gcc_phat(&src_mono[..tpl_len], cap_mono, n - tpl_len);
    let phat_peak = phat.iter().copied().fold(f64::MIN, f64::max);
    let mut ph = rb(ResolutionClass::Segment).scalar(
        "ref.phat_peak",
        phat_peak,
        "corr",
        None,
        up_conf,
        up_val,
        None,
        n as u64,
    );
    ph.alignment_id = Some(alignment_id.to_string());
    out.records.push(ph);
    // Envelope similarity: corr of spectral envelopes.
    let env_s = spectral_envelope(src_mono, cfg.stft_window, 64);
    let env_c = spectral_envelope(cap_mono, cfg.stft_window, 64);
    let env_n = env_s.len().min(env_c.len());
    let mut es = rb(ResolutionClass::Segment).scalar(
        "ref.envelope_similarity",
        if env_n > 0 {
            corr0(&env_s[..env_n], &env_c[..env_n])
        } else {
            f64::NAN
        },
        "corr",
        None,
        propagate_confidence(up_conf, Confidence::Medium),
        up_val,
        None,
        env_n as u64,
    );
    es.alignment_id = Some(alignment_id.to_string());
    out.records.push(es);

    // ==== Channel family ====
    if let Some(mix) = fit_mix(&src.src, &src.cap, cfg.ridge) {
        let mut r = rb(ResolutionClass::Segment).scalar(
            "ref.mixer_fit",
            mix.fit_quality,
            "ratio",
            None,
            up_conf,
            up_val,
            None,
            n as u64,
        );
        r.alignment_id = Some(alignment_id.to_string());
        r.params = serde_json::json!({
            "matrix": mix.matrix,
            "ridge": mix.ridge,
            "condition_number": mix.condition_number,
        });
        out.records.push(r);
        // Per-output response (mono source): source→each capture ch.
        if src.src.len() == 1 {
            for (c, coeff_row) in mix.matrix.iter().enumerate() {
                let mut pc = rb(ResolutionClass::Segment).scalar(
                    "ref.channel_response",
                    coeff_row.first().copied().unwrap_or(0.0),
                    "ratio",
                    None,
                    up_conf,
                    up_val,
                    None,
                    n as u64,
                );
                pc.alignment_id = Some(alignment_id.to_string());
                pc.channels = Some(al_dataset::types::ChannelSelector::Channel(c as u16));
                pc.params = serde_json::json!({"from_source_ch": 0});
                out.records.push(pc);
            }
        }
        // Polarity: sign of dominant coefficient.
        let dominant = mix
            .matrix
            .iter()
            .flat_map(|r| r.iter())
            .copied()
            .fold(0.0f64, |m, v| if v.abs() > m.abs() { v } else { m });
        let mut pr = rb(ResolutionClass::Segment).scalar(
            "ref.polarity",
            dominant.signum(),
            "sign",
            None,
            up_conf,
            up_val,
            None,
            n as u64,
        );
        pr.alignment_id = Some(alignment_id.to_string());
        out.records.push(pr);
    }
    // L/R band coherence (magnitude + phase) for stereo captures.
    if src.cap.len() >= 2 {
        let m0 = stft(src.cap[0], cfg.stft_window, cfg.stft_hop, rate);
        let m1 = stft(src.cap[1], cfg.stft_window, cfg.stft_hop, rate);
        let fr = m0.frames().min(m1.frames());
        if fr > 0 {
            let mut pa = vec![0.0; bins];
            let mut pb = vec![0.0; bins];
            let mut pab = vec![(0.0, 0.0); bins];
            for f in 0..fr {
                let c = csd(&m0.re[f], &m0.im[f], &m1.re[f], &m1.im[f]);
                for k in 0..bins {
                    pa[k] += m0.re[f][k].powi(2) + m0.im[f][k].powi(2);
                    pb[k] += m1.re[f][k].powi(2) + m1.im[f][k].powi(2);
                    pab[k].0 += c[k].0;
                    pab[k].1 += c[k].1;
                }
            }
            let coh = complex_coherence(&pa, &pb, &pab);
            let band_rows: Vec<Vec<f64>> = bands
                .iter()
                .map(|&(lo, hi)| {
                    let b0 = (lo / bin_hz) as usize;
                    let b1 = ((hi / bin_hz).ceil() as usize).min(half);
                    if b1 <= b0 {
                        return vec![0.0, 0.0];
                    }
                    let (mut re, mut im) = (0.0, 0.0);
                    for &(r, i) in &coh[b0..b1] {
                        re += r;
                        im += i;
                    }
                    let cnt = (b1 - b0) as f64;
                    vec![(re * re + im * im).sqrt() / cnt, im.atan2(re)]
                })
                .collect();
            out.tensors.push(matrix_to_tensor(
                format!("intermediate/{pair}/lr_band_coherence"),
                &band_rows,
            ));
            // Low-band L/R similarity change vs the source bands.
            if let Some(first) = band_rows.first() {
                let mut lr = rb(ResolutionClass::Segment).scalar(
                    "ref.lr_lowband_coherence",
                    first[0],
                    "ratio",
                    None,
                    propagate_confidence(up_conf, Confidence::Medium),
                    if first[0] > 0.2 {
                        up_val
                    } else {
                        Validity::LowCoherence
                    },
                    None,
                    fr as u64,
                );
                lr.alignment_id = Some(alignment_id.to_string());
                lr.freq_range = Some(al_dataset::types::Range {
                    start: bands[0].0,
                    end: bands[0].1,
                });
                lr.params = serde_json::json!({"phase_rad": first[1]});
                out.records.push(lr);
            }
        }
        // M/S change: capture M/S ratio vs "source M/S" (mono → 0).
        let m_e: f64 = src.cap[0]
            .iter()
            .zip(src.cap[1])
            .map(|(&a, &b)| (a + b) * (a + b) / 4.0)
            .sum();
        let s_e: f64 = src.cap[0]
            .iter()
            .zip(src.cap[1])
            .map(|(&a, &b)| (a - b) * (a - b) / 4.0)
            .sum();
        let mut msr = rb(ResolutionClass::Segment).scalar(
            "ref.ms_side_mid_ratio",
            if m_e > 0.0 { s_e / m_e } else { f64::NAN },
            "ratio",
            None,
            up_conf,
            up_val,
            None,
            n as u64,
        );
        msr.alignment_id = Some(alignment_id.to_string());
        msr.channels = Some(al_dataset::types::ChannelSelector::ChannelPair(0, 1));
        out.records.push(msr);
        // Crosstalk / leakage: for stereo source, energy of the
        // weaker fitted cross-coefficient.
        if src.src.len() >= 2
            && let Some(mix) = fit_mix(&src.src, &src.cap, cfg.ridge)
        {
            let off_diag: f64 = mix
                .matrix
                .iter()
                .enumerate()
                .map(|(c, row)| {
                    row.iter()
                        .enumerate()
                        .filter(|(s, _)| *s != c)
                        .map(|(_, v)| v * v)
                        .sum::<f64>()
                })
                .sum();
            let diag: f64 = mix
                .matrix
                .iter()
                .enumerate()
                .map(|(c, row)| row.get(c).copied().unwrap_or(0.0).powi(2))
                .sum();
            let mut xt = rb(ResolutionClass::Segment).scalar(
                "ref.crosstalk_ratio",
                if diag > 0.0 {
                    off_diag / diag
                } else {
                    f64::NAN
                },
                "ratio",
                None,
                propagate_confidence(up_conf, Confidence::Medium),
                up_val,
                None,
                n as u64,
            );
            xt.alignment_id = Some(alignment_id.to_string());
            out.records.push(xt);
        }
    }

    // ==== Dynamics family ====
    // Transient onset offset: difference of first strong onset index
    // between src/cap level trajectories.
    let lw = ((0.02 * rate as f64) as usize).max(16);
    let onsets = |x: &[f64]| -> Vec<usize> {
        let mut env = Vec::new();
        for start in (0..x.len().saturating_sub(lw)).step_by(lw) {
            env.push(rms(&x[start..start + lw]));
        }
        let mut out = Vec::new();
        for i in 1..env.len() {
            if env[i] - env[i - 1] > env[i - 1].max(1e-6) * 1.5 && env[i] - env[i - 1] > 0.01 {
                out.push(i);
            }
        }
        out
    };
    let (os, oc) = (onsets(src_mono), onsets(cap_mono));
    if !os.is_empty() && !oc.is_empty() {
        let offset = oc[0] as f64 - os[0] as f64;
        let mut r = rb(ResolutionClass::Mid20ms)
            .resolution(lw as u64, lw as u64, rate)
            .scalar(
                "ref.transient_onset_offset_ms",
                offset * lw as f64 / rate as f64 * 1000.0,
                "ms",
                None,
                propagate_confidence(up_conf, Confidence::Medium),
                up_val,
                None,
                (os.len() + oc.len()) as u64,
            );
        r.alignment_id = Some(alignment_id.to_string());
        out.records.push(r);
        let mut tr = rb(ResolutionClass::Segment).scalar(
            "ref.transient_strength_ratio",
            oc.len() as f64 / os.len().max(1) as f64,
            "ratio",
            None,
            propagate_confidence(up_conf, Confidence::Low),
            up_val,
            None,
            (os.len() + oc.len()) as u64,
        );
        tr.alignment_id = Some(alignment_id.to_string());
        out.records.push(tr);
    }
    // F0 ratio via autocorrelation peak of each signal.
    let f0 = |x: &[f64]| -> Option<f64> {
        let len = x.len().min(1 << 15);
        let x = &x[..len];
        let mut best = (0usize, 0.0f64);
        let lo = (rate as f64 / 1000.0) as usize;
        let hi = (rate as f64 / 40.0) as usize;
        for lag in lo..hi.min(len / 2) {
            let c = corr0(&x[..len - lag], &x[lag..]);
            if c > best.1 {
                best = (lag, c);
            }
        }
        (best.1 > 0.3 && best.0 > 0).then(|| rate as f64 / best.0 as f64)
    };
    if let (Some(fs), Some(fc)) = (f0(src_mono), f0(cap_mono)) {
        let mut r = rb(ResolutionClass::Segment).scalar(
            "ref.f0_ratio",
            fc / fs,
            "ratio",
            None,
            propagate_confidence(up_conf, Confidence::Low),
            up_val,
            None,
            n as u64,
        );
        r.alignment_id = Some(alignment_id.to_string());
        r.params = serde_json::json!({"src_f0": fs, "cap_f0": fc});
        out.records.push(r);
    }
    // Harmonic frequency ratio: ratio of dominant peak frequencies.
    let dom = |m: &crate::stft::StftMatrix| -> f64 {
        let mag = m.magnitude();
        let mut mean = vec![0.0; m.fft_size / 2];
        for row in &mag {
            for (k, v) in mean.iter_mut().enumerate() {
                *v += row[k];
            }
        }
        mean.iter()
            .enumerate()
            .skip(1)
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(b, _)| b as f64 * bin_hz)
            .unwrap_or(0.0)
    };
    let (ds, dc) = (dom(&ms), dom(&mc));
    if ds > 0.0 && dc > 0.0 {
        let mut r = rb(ResolutionClass::Segment).scalar(
            "ref.harmonic_freq_ratio",
            dc / ds,
            "ratio",
            None,
            propagate_confidence(up_conf, Confidence::Medium),
            up_val,
            None,
            frames as u64,
        );
        r.alignment_id = Some(alignment_id.to_string());
        r.params = serde_json::json!({"src_hz": ds, "cap_hz": dc});
        out.records.push(r);
    }

    // ==== Staged residuals R0–R3 ====
    //
    // Each stage removes what the previous stage left unexplained:
    // models are fit against the previous residual and subtracted
    // from it. Least-squares fits (scalar gain, channel matrix) are
    // non-increasing by construction; the FIR stage is gated on
    // measured energy so R0–R3 stays monotonically non-increasing.
    let energy = |r: &[Vec<f64>]| -> f64 { r.iter().map(|c| sum_sq(c)).sum() };
    let residuals: Vec<(&str, Vec<Vec<f64>>)> = {
        let mut v: Vec<(&str, Vec<Vec<f64>>)> = Vec::new();
        // R0: alignment only — the capture as-is.
        let r0: Vec<Vec<f64>> = src.cap.iter().map(|c| c.to_vec()).collect();
        v.push(("r0", r0.clone()));
        // R1: scalar LS gain against the R0 residual per channel.
        let m1: Vec<Vec<f64>> = r0
            .iter()
            .map(|ch| {
                let g = ls_gain(src_mono, ch);
                src_mono.iter().map(|&x| x * g).collect()
            })
            .collect();
        let r1 = subtract(&r0, &m1);
        v.push(("r1", r1.clone()));
        // R2: channel matrix fit against the R1 residual.
        let r1_refs: Vec<&[f64]> = r1.iter().map(Vec::as_slice).collect();
        if let Some(mix) = fit_mix(&src.src, &r1_refs, cfg.ridge) {
            let m2: Vec<Vec<f64>> = (0..r1.len())
                .map(|c| {
                    (0..n)
                        .map(|i| {
                            src.src
                                .iter()
                                .enumerate()
                                .map(|(s, sch)| {
                                    mix.matrix[c].get(s).copied().unwrap_or(0.0) * sch[i]
                                })
                                .sum()
                        })
                        .collect()
                })
                .collect();
            v.push(("r2", subtract(&r1, &m2)));
        }
        // R3: FIR against the R2 residual, applied only where it
        // actually reduces measured energy ("supported" transfer).
        let r2 = v.last().unwrap().1.clone();
        let m3: Vec<Vec<f64>> = r2
            .iter()
            .map(|ch| fir_model(src_mono, ch, &ms, cfg, rate))
            .collect();
        let candidate = subtract(&r2, &m3);
        let r3 = if energy(&candidate) <= energy(&r2) {
            candidate
        } else {
            r2
        };
        v.push(("r3", r3));
        v
    };
    let mut residual_levels = Vec::new();
    for (idx, (name, residual)) in residuals.iter().enumerate() {
        let interleaved: Vec<f64> = {
            let mut v = Vec::with_capacity(n * residual.len());
            for i in 0..n {
                for ch in residual {
                    v.push(ch[i]);
                }
            }
            v
        };
        let rname = format!("{pair}/{name}");
        let res_rms = rms(&interleaved);
        residual_levels.push(res_rms);
        let res_corr = corr0(&residual[0], src_mono);
        out.residual_audio.push(ResidualAudio {
            name: rname.clone(),
            samples: interleaved,
            channels: src.cap.len() as u16,
            sample_rate: rate,
        });
        // Retained companions: STFT + PSD + band energy.
        let rm = stft(&residual[0], cfg.stft_window, cfg.stft_hop, rate);
        out.tensors.push(matrix_to_tensor(
            format!("residual/{rname}/stft_re"),
            &rm.re,
        ));
        let rpsd = rm.psd();
        out.tensors
            .push(matrix_to_tensor(format!("residual/{rname}/psd"), &rpsd));
        let rband: Vec<Vec<f64>> = rpsd
            .iter()
            .map(|row| band_power(row, rate, rm.fft_size, &bands))
            .collect();
        out.tensors.push(matrix_to_tensor(
            format!("residual/{rname}/band_energy"),
            &rband,
        ));
        let mut rr = rb(ResolutionClass::Segment).scalar(
            "ref.residual_rms_ref_ratio",
            res_rms / rms(cap_mono).max(1e-12),
            "ratio",
            None,
            up_conf,
            up_val,
            None,
            n as u64,
        );
        rr.alignment_id = Some(alignment_id.to_string());
        rr.params = serde_json::json!({"stage": name, "order": idx});
        out.records.push(rr);
        let mut rc = rb(ResolutionClass::Segment).scalar(
            "ref.residual_src_correlation",
            res_corr,
            "corr",
            None,
            up_conf,
            up_val,
            None,
            n as u64,
        );
        rc.alignment_id = Some(alignment_id.to_string());
        rc.params = serde_json::json!({"stage": name});
        out.records.push(rc);
    }
    // R3 summary: final residual ratio as the headline record.
    if let Some(&last) = residual_levels.last() {
        let mut r = rb(ResolutionClass::Segment).scalar(
            "ref.residual_r3_ratio",
            last / rms(cap_mono).max(1e-12),
            "ratio",
            None,
            up_conf,
            up_val,
            None,
            n as u64,
        );
        r.alignment_id = Some(alignment_id.to_string());
        out.records.push(r);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic zero-mean pseudo-noise (LCG).
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

    fn record<'a>(out: &'a ReferenceResult, id: &str) -> &'a FeatureRecord {
        out.records
            .iter()
            .find(|r| r.feature_id == id)
            .unwrap_or_else(|| panic!("missing record {id}"))
    }

    fn scalar(r: &FeatureRecord) -> f64 {
        match &r.value {
            al_dataset::types::Value::Scalar(v) => *v,
            other => panic!("expected scalar, got {other:?}"),
        }
    }

    fn aligned<'a>(src: &'a [Vec<f64>], cap: &'a [Vec<f64>], rate: u32) -> Aligned<'a> {
        Aligned {
            src: src.iter().map(|c| c.as_slice()).collect(),
            cap: cap.iter().map(|c| c.as_slice()).collect(),
            rate,
            upstream_confidence: Confidence::High,
        }
    }

    const N: usize = 1 << 16;
    const RATE: u32 = 48_000;
    const ALN: &str = "ALN-0123456789abcdefghijkl";

    fn analyze<'a>(pair: &str, aligned: &Aligned<'a>, validity: Validity) -> ReferenceResult {
        analyze_pair(
            pair,
            &al_core::AlignmentId::parse(ALN).unwrap(),
            aligned,
            validity,
            &ReferenceConfig::default(),
        )
    }

    #[test]
    fn records_carry_the_locate_alignment_id() {
        let src = vec![noise(N, 0x5eed)];
        let cap = src.clone();
        let pair = aligned(&src, &cap, RATE);
        for validity in [Validity::Valid, Validity::AmbiguousAlignment] {
            let out = analyze("src__cap", &pair, validity);
            assert!(!out.records.is_empty());
            for r in &out.records {
                assert_eq!(
                    r.alignment_id.as_deref(),
                    Some(ALN),
                    "record {}",
                    r.feature_id
                );
            }
        }
    }

    #[test]
    fn known_gain_is_recovered() {
        let src = vec![noise(N, 0x5eed)];
        let cap = vec![src[0].iter().map(|&v| 0.5 * v).collect::<Vec<f64>>()];
        let pair = aligned(&src, &cap, RATE);
        let out = analyze("src__cap", &pair, Validity::Valid);

        let gain = scalar(record(&out, "ref.gain_ls"));
        assert!((gain - 0.5).abs() < 0.02, "gain {gain}");
        let g400 = scalar(record(&out, "ref.gain_400hz"));
        assert!((g400 - 0.5).abs() < 0.05, "400 Hz gain {g400}");
    }

    #[test]
    fn known_residual_delay_is_recovered() {
        // Small residual misalignment inside the aligned pair: the
        // locate stage removes the coarse offset, so `cap` here is a
        // few samples late relative to `src`.
        let src = vec![noise(N, 0x5eed)];
        let delay = 7usize;
        let cap = vec![
            src[0]
                .iter()
                .enumerate()
                .map(|(i, _)| if i >= delay { src[0][i - delay] } else { 0.0 })
                .collect::<Vec<f64>>(),
        ];
        let pair = aligned(&src, &cap, RATE);
        let out = analyze("src__cap", &pair, Validity::Valid);

        let gd = scalar(record(&out, "ref.group_delay_median_s"));
        let expected = delay as f64 / RATE as f64;
        assert!(
            (gd - expected).abs() < expected * 0.2,
            "group delay {gd} vs {expected}"
        );

        let lag = scalar(record(&out, "ref.lag_median")).abs();
        assert!((lag - delay as f64).abs() <= 1.0, "lag {lag}");
    }

    #[test]
    fn residual_energy_is_monotonic_across_stages() {
        let src = vec![noise(N, 0xc0ffee)];
        let cap = vec![
            src[0]
                .iter()
                .enumerate()
                .map(|(i, _)| if i >= 11 { 0.4 * src[0][i - 11] } else { 0.0 })
                .collect::<Vec<f64>>(),
        ];
        let pair = aligned(&src, &cap, RATE);
        let out = analyze("src__cap", &pair, Validity::Valid);

        let mut ratios: Vec<f64> = out
            .records
            .iter()
            .filter(|r| r.feature_id == "ref.residual_rms_ref_ratio")
            .map(scalar)
            .collect();
        assert_eq!(ratios.len(), 4, "expected R0-R3 records: {ratios:?}");
        for w in ratios.windows(2) {
            assert!(w[1] <= w[0] + 1e-9, "residual not monotonic: {ratios:?}");
        }
        // FIR stage explains a pure delay+gain almost fully.
        assert!(ratios[3] < 0.2, "r3 ratio {}", ratios[3]);
        ratios.clear();
    }

    #[test]
    fn identical_capture_leaves_no_residual() {
        let src = vec![noise(N, 0xdead_beef)];
        let cap = src.clone();
        let pair = aligned(&src, &cap, RATE);
        let out = analyze("src__cap", &pair, Validity::Valid);

        let sim = scalar(record(&out, "ref.similarity_fullband"));
        assert!(sim > 0.99, "similarity {sim}");
        let r3 = scalar(record(&out, "ref.residual_r3_ratio"));
        assert!(r3 < 1e-6, "r3 residual {r3}");
    }

    #[test]
    fn rejected_alignment_yields_only_invalid_rows() {
        let src = vec![noise(4096, 1)];
        let cap = src.clone();
        let pair = aligned(&src, &cap, RATE);
        let out = analyze("src__cap", &pair, Validity::AmbiguousAlignment);
        assert!(!out.records.is_empty());
        for r in &out.records {
            assert_ne!(
                r.validity,
                Validity::Valid,
                "record {} must not be valid",
                r.feature_id
            );
            assert_eq!(r.support_count, 0);
        }
        assert!(out.residual_audio.is_empty());
    }

    #[test]
    fn short_input_does_not_panic() {
        let src = vec![noise(2048, 2)];
        let cap = src.clone();
        let pair = aligned(&src, &cap, RATE);
        let out = analyze("src__cap", &pair, Validity::Valid);
        let gain = scalar(record(&out, "ref.gain_ls"));
        assert!((gain - 1.0).abs() < 1e-9);
    }

    // ==== A1 characterization (caller-side optimizations,
    // docs/test-specs/caller-side-optimizations.md Rule C) ====

    /// Direct linear convolution returning the first `x.len()`
    /// outputs — reference implementation for A/B against the
    /// `fir_model` convolution path. Verbatim copy shared by hand
    /// with `dsp.rs` tests and `benches/conv.rs` (not a shared
    /// public surface). The `x == 0.0` skip of the former
    /// production loop is dropped on purpose: zero terms
    /// contribute `±0.0` and are value-equivalent.
    fn conv_direct(x: &[f64], h: &[f64]) -> Vec<f64> {
        let mut y = vec![0.0; x.len()];
        for (i, &xv) in x.iter().enumerate() {
            let maxk = h.len().min(x.len() - i);
            for (k, &hv) in h.iter().take(maxk).enumerate() {
                y[i + k] += xv * hv;
            }
        }
        y
    }

    /// Recomposition tolerance `|a-b| <= 1e-12*max(1,|a|,|b|)`
    /// (ADR-034EoHxzsvsHp9BOdCK2Cn).
    fn assert_close_slice(actual: &[f64], expected: &[f64]) {
        assert_eq!(
            actual.len(),
            expected.len(),
            "length {} vs {}",
            actual.len(),
            expected.len()
        );
        for (i, (&a, &b)) in actual.iter().zip(expected).enumerate() {
            let tol = 1e-12 * 1.0_f64.max(a.abs()).max(b.abs());
            assert!((a - b).abs() <= tol, "index {i}: {a} vs {b} (tol {tol})");
        }
    }

    /// `fir_model` output must equal `fir_ir`'s IR truncated to
    /// `conv_len = ir.len().min(4096)` passed through `conv_direct`.
    /// Green both immediately after the function extraction and
    /// after the `dsp::conv` replacement (A1).
    #[test]
    fn fir_model_matches_fir_ir_truncated_conv_direct() {
        let cases: Vec<(usize, usize, Vec<f64>, Vec<f64>)> = vec![
            // Empty input → zero-frame path, empty model.
            (256, 128, vec![], vec![]),
            // n < conv_len: window 300 → fft_size 512 → conv_len 512
            // while n = 400 (frames still run).
            (300, 100, noise(400, 7), noise(400, 8)),
            // conv_len = 4096 boundary with real truncation:
            // window 5000 → fft_size 8192 → IR cut to 4096.
            (5000, 2500, noise(6000, 9), noise(6000, 10)),
            // All-zero signals: zero energy bins → zero IR.
            (256, 128, vec![0.0; 1024], vec![0.0; 1024]),
            // Representative deterministic noise.
            (256, 128, noise(2048, 11), noise(2048, 12)),
        ];
        for (window, hop, from, to) in cases {
            let cfg = ReferenceConfig {
                stft_window: window,
                stft_hop: hop,
                ..ReferenceConfig::default()
            };
            let from_stft = stft(&from, window, hop, RATE);
            let ir = fir_ir(&from_stft, &to, window, hop, RATE);
            let expected = conv_direct(&from, &ir[..ir.len().min(4096)]);
            let actual = fir_model(&from, &to, &from_stft, &cfg, RATE);
            assert_close_slice(&actual, &expected);
        }
    }

    /// `subtract` zero-fills a short or missing model side instead
    /// of panicking — a structural guard: production inputs share
    /// the aligned length (`stage.rs` Aligned contract) and never
    /// go ragged.
    #[test]
    fn subtract_zero_fills_short_or_missing_model() {
        // Model shorter than residual: the tail is `residual - 0.0`.
        let out = subtract(&[vec![1.0, 2.0, 3.0, 4.0]], &[vec![0.5, 0.25]]);
        assert_eq!(out, vec![vec![0.5, 1.75, 3.0, 4.0]]);
        // Missing model channel: the whole channel is zero-filled.
        let out = subtract(&[vec![1.0], vec![2.0]], &[vec![0.5]]);
        assert_eq!(out, vec![vec![0.5], vec![2.0]]);
        // Equal length is exact subtraction.
        let out = subtract(&[vec![1.0]], &[vec![0.25]]);
        assert_eq!(out, vec![vec![0.75]]);
    }
}
