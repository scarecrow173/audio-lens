//! Cross-channel families: frequency-dependent L/R coherence, the
//! per-band source→capture mixing matrix (pair-dependent), and
//! intermodulation evidence around strong tones.

use al_core::{Confidence, Validity};
use al_dataset::types::{ChannelSelector, FeatureRecord, Range};

use crate::records::{propagate_confidence, propagate_validity};
use crate::stft::{csd, stft, third_octave_bands};

use super::{Emit, ExpensiveResult, RegionCtx};

/// Emit this family's invalid schema for a region selected by
/// non-`Valid` evidence (same ids `run` would produce, as NaN
/// scalars with the mapped validity). The L/R coherence record only
/// exists for stereo inputs, mirroring `run`.
pub(crate) fn emit_invalid(
    e: &Emit<'_>,
    records: &mut Vec<FeatureRecord>,
    reason: &str,
    chans: usize,
) {
    if chans >= 2 {
        let mut r = e.push_spec(
            "exp.freq_lr_coherence",
            f64::NAN,
            "coherence",
            Confidence::High,
            e.trigger_val,
            Some(reason.to_string()),
            0,
        );
        r.channels = Some(ChannelSelector::ChannelPair(0, 1));
        records.push(r);
    }
    for (id, unit) in [
        ("exp.mix_fit_quality", "ratio"),
        ("exp.mix_polarity", "sign"),
        ("exp.mix_condition", "ratio"),
    ] {
        records.push(e.push(
            id,
            f64::NAN,
            unit,
            Confidence::High,
            e.trigger_val,
            Some(reason.to_string()),
            0,
        ));
    }
    for (id, unit) in [
        ("exp.mix_band_coeff", "ratio"),
        ("exp.intermod_ratio", "ratio"),
        ("exp.intermod_count", "count"),
    ] {
        records.push(e.push_spec(
            id,
            f64::NAN,
            unit,
            Confidence::High,
            e.trigger_val,
            Some(reason.to_string()),
            0,
        ));
    }
}

/// Run the cross-channel families over the region.
pub(crate) fn run(ctx: &RegionCtx<'_>, e: &Emit<'_>, out: &mut ExpensiveResult) {
    let seg = ctx.segs[0];
    let rate = ctx.rate;
    let bins = ctx.stft.fft_size;
    let half = bins / 2;
    let bin_hz = ctx.bin_hz;
    let mean_power = &ctx.mean_power;

    // ==== Advanced channel analysis (>= 2ch) ====
    if ctx.chans.len() >= 2
        && let Some(oseg) = ctx.segs.get(1).copied()
        && !oseg.is_empty()
    {
        let n_ch = seg.len().min(oseg.len());
        if n_ch >= ctx.stft.window as usize {
            let bands = third_octave_bands(100.0, rate as f64 / 2.0 * 0.9);
            let mb = stft(&seg[..n_ch], ctx.stft.window as usize, ctx.stft.hop as usize, rate);
            let ob = stft(&oseg[..n_ch], ctx.stft.window as usize, ctx.stft.hop as usize, rate);
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
                let mut rec = e.push_spec(
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

    // ==== Intermodulation evidence ====
    // Around the two strongest tones, nonlinearity creates products
    // at |f1−f2|, f1+f2 and 2f1−f2 / 2f2−f1. Reported as candidate
    // evidence only — the energy share at the product positions.
    {
        let supported = ctx.total_power > 0.0;
        let mut local: Vec<f64> = mean_power[1..half].to_vec();
        local.sort_by(|a, b| a.total_cmp(b));
        let med = local[local.len() / 2].max(1e-18);
        // Strong local maxima (10× neighborhood median).
        let mut peaks: Vec<usize> = (8..half.saturating_sub(8))
            .filter(|&k| {
                mean_power[k] > med * 10.0
                    && mean_power[k]
                        >= mean_power[k - 8..k + 8]
                            .iter()
                            .copied()
                            .fold(0.0, f64::max)
                            * 0.999
            })
            .collect();
        peaks.sort_by(|&a, &b| mean_power[b].total_cmp(&mean_power[a]));
        let f1 = peaks.first().copied().unwrap_or(0);
        let f2 = peaks
            .iter()
            .copied()
            .find(|&k| k.abs_diff(f1) >= 16)
            .unwrap_or(0);
        let two = f1 > 0 && f2 > 0;
        let (f_hi, f_lo) = if f1 > f2 { (f1, f2) } else { (f2, f1) };
        let mut positions = Vec::new();
        if two {
            for p in [
                f_hi - f_lo,
                (f_hi + f_lo).min(half - 1),
                f_hi
                    .saturating_mul(2)
                    .saturating_sub(f_lo)
                    .min(half - 1),
                f_lo
                    .saturating_mul(2)
                    .saturating_sub(f_hi)
                    .min(half - 1),
            ] {
                if p > 0 && !positions.contains(&p) {
                    positions.push(p);
                }
            }
        }
        let count = positions
            .iter()
            .filter(|&&p| mean_power[p] > med * 10.0)
            .count();
        let inter_e: f64 = positions.iter().map(|&p| mean_power[p]).sum();
        let base_e = mean_power[f_hi] + mean_power[f_lo];
        for (id, value, unit) in [
            (
                "exp.intermod_ratio",
                if two {
                    inter_e / base_e.max(1e-30)
                } else {
                    f64::NAN
                },
                "ratio",
            ),
            (
                "exp.intermod_count",
                if two { count as f64 } else { f64::NAN },
                "count",
            ),
        ] {
            out.records.push(e.push_spec(
                id,
                if supported { value } else { f64::NAN },
                unit,
                Confidence::Low,
                if !supported || !two {
                    Validity::InsufficientSupport
                } else {
                    Validity::Valid
                },
                if !supported || !two {
                    Some("fewer than two strong tones for intermodulation check".into())
                } else {
                    None
                },
                positions.len() as u64,
            ));
        }
    }

    // ==== Frequency-dependent mixing matrix (pair-dependent) ====
    // Per third-octave band, complex least squares maps the source
    // channel spectra onto each capture channel: C_c(f) ≈ Σ_s
    // w[c][s]·S_s(f). Signed real parts double as polarity/routing
    // evidence (a polarity-inverted route yields a negative
    // coefficient). Ill-conditioned bands are reported via
    // `exp.mix_condition` and demoted to `LowCoherence`; missing
    // pair evidence yields `pair evidence unavailable` records.
    {
        let unavailable = |records: &mut Vec<FeatureRecord>,
                           pair: Option<&super::PairEvidence<'_>>,
                           val: Validity,
                           reason: &str| {
            for (id, unit, spec) in [
                ("exp.mix_band_coeff", "ratio", true),
                ("exp.mix_fit_quality", "ratio", false),
                ("exp.mix_polarity", "sign", false),
                ("exp.mix_condition", "ratio", false),
            ] {
                let conf = pair.map_or(Confidence::Medium, |p| {
                    propagate_confidence(p.confidence, Confidence::Medium)
                });
                let mut r = if spec {
                    e.push_spec(id, f64::NAN, unit, conf, val, Some(reason.to_string()), 0)
                } else {
                    e.push(id, f64::NAN, unit, conf, val, Some(reason.to_string()), 0)
                };
                if let Some(p) = pair {
                    r.alignment_id = Some(p.alignment_id.clone());
                }
                records.push(r);
            }
        };
        let Some(pair) = ctx.pair else {
            unavailable(
                &mut out.records,
                None,
                Validity::InsufficientSupport,
                "pair evidence unavailable",
            );
            return;
        };
        let up_val = propagate_validity(pair.validity);
        let aligned = pair.aligned_slice(ctx.s0, ctx.s1);
        if up_val != Validity::Valid || aligned.is_none() {
            unavailable(
                &mut out.records,
                Some(pair),
                up_val,
                if up_val != Validity::Valid {
                    "pair alignment not accepted"
                } else {
                    "pair evidence does not overlap region"
                },
            );
            return;
        }
        let (o0, o1, src_segs) = aligned.unwrap();
        let cap_segs: Vec<&[f64]> = ctx
            .chans
            .iter()
            .filter(|c| c.len() >= o1)
            .map(|c| &c[o0..o1])
            .collect();
        let n_src = src_segs.len();
        let n_cap = cap_segs.len();
        let w = ctx.stft.window as usize;
        let h = ctx.stft.hop as usize;
        if n_src == 0 || n_cap == 0 || o1 - o0 < w {
            unavailable(
                &mut out.records,
                Some(pair),
                Validity::InsufficientSupport,
                "aligned pair region shorter than analysis window",
            );
            return;
        }
        let mss: Vec<_> = src_segs
            .iter()
            .map(|s| stft(s, w, h, rate))
            .collect();
        let mcs: Vec<_> = cap_segs
            .iter()
            .map(|s| stft(s, w, h, rate))
            .collect();
        let fr = mss
            .iter()
            .chain(&mcs)
            .map(|m| m.frames())
            .min()
            .unwrap_or(0);
        let bands = third_octave_bands(100.0, rate as f64 / 2.0 * 0.9);
        let mut solved = 0u64;
        let mut max_cond = 0.0f64;
        let mut resid_acc = 0.0;
        let mut resid_n = 0u64;
        let mut dom_sign = 0.0f64;
        let mut dom_mag = 0.0f64;
        for &(lo, hi) in &bands {
            let b0 = (lo / bin_hz) as usize;
            let b1 = ((hi / bin_hz).ceil() as usize).min(half);
            if b1 <= b0 || fr == 0 {
                continue;
            }
            // Normal equations over the band's bins × frames.
            // A[i][j] = Σ conj(S_i)·S_j, b_c[i] = Σ conj(S_i)·C_c.
            let mut a = vec![vec![(0.0, 0.0); n_src]; n_src];
            for f in 0..fr {
                for k in b0..b1 {
                    for i in 0..n_src {
                        let (si_r, si_i) = (mss[i].re[f][k], mss[i].im[f][k]);
                        for j in 0..n_src {
                            let (sj_r, sj_i) = (mss[j].re[f][k], mss[j].im[f][k]);
                            // conj(S_i) * S_j
                            a[i][j].0 += si_r * sj_r + si_i * sj_i;
                            a[i][j].1 += si_r * sj_i - si_i * sj_r;
                        }
                    }
                }
            }
            for (c, mc) in mcs.iter().enumerate() {
                let mut b = vec![(0.0, 0.0); n_src];
                let mut cap_e = 0.0;
                for f in 0..fr {
                    for k in b0..b1 {
                        let (cr, ci) = (mc.re[f][k], mc.im[f][k]);
                        cap_e += cr * cr + ci * ci;
                        for i in 0..n_src {
                            let (si_r, si_i) = (mss[i].re[f][k], mss[i].im[f][k]);
                            // conj(S_i) * C
                            b[i].0 += si_r * cr + si_i * ci;
                            b[i].1 += si_r * ci - si_i * cr;
                        }
                    }
                }
                let Some((coef, cond)) = csolve(&a, &b) else {
                    continue;
                };
                max_cond = max_cond.max(cond);
                // Residual: ||C - Σ w_s S_s||² / ||C||² over the band.
                let mut resid = 0.0;
                for f in 0..fr {
                    for k in b0..b1 {
                        let mut pr = mc.re[f][k];
                        let mut pi = mc.im[f][k];
                        for i in 0..n_src {
                            let (wr, wi) = coef[i];
                            pr -= wr * mss[i].re[f][k] - wi * mss[i].im[f][k];
                            pi -= wr * mss[i].im[f][k] + wi * mss[i].re[f][k];
                        }
                        resid += pr * pr + pi * pi;
                    }
                }
                let resid_ratio = resid / cap_e.max(1e-30);
                resid_acc += resid_ratio;
                resid_n += 1;
                solved += 1;
                // Record the dominant coefficient (largest |w_s|);
                // its real part's sign is polarity-routing evidence.
                let (ds, (wr, wi)) = coef
                    .iter()
                    .enumerate()
                    .max_by(|a, b| {
                        (a.1.0 * a.1.0 + a.1.1 * a.1.1)
                            .total_cmp(&(b.1.0 * b.1.0 + b.1.1 * b.1.1))
                    })
                    .map(|(i, &w)| (i, w))
                    .unwrap_or((0, (0.0, 0.0)));
                let dmag = (wr * wr + wi * wi).sqrt();
                if dmag > dom_mag {
                    dom_mag = dmag;
                    dom_sign = wr.signum();
                }
                let mut r = e.push_spec(
                    "exp.mix_band_coeff",
                    dmag,
                    "ratio",
                    propagate_confidence(pair.confidence, Confidence::Medium),
                    up_val,
                    None,
                    ((b1 - b0) * fr) as u64,
                );
                r.alignment_id = Some(pair.alignment_id.clone());
                r.freq_range = Some(Range { start: lo, end: hi });
                r.channels = Some(ChannelSelector::Channel(c as u16));
                r.params = serde_json::json!({
                    "region_s": [e.region().start_s, e.region().end_s],
                    "triggers": e.region().triggers,
                    "pair": pair.pair,
                    "band": [lo, hi],
                    "coeff": coef.iter().map(|&(re, im)| [re, im]).collect::<Vec<_>>(),
                    "dominant_src_ch": ds,
                    "coeff_sign": wr.signum(),
                    "condition": cond,
                    "residual_ratio": resid_ratio,
                });
                out.records.push(r);
            }
        }
        let summary = |records: &mut Vec<FeatureRecord>,
                       id: &str,
                       value: f64,
                       unit: &str,
                       conf: Confidence,
                       val: Validity,
                       reason: Option<String>,
                       support: u64| {
            let mut r = e.push(
                id,
                value,
                unit,
                propagate_confidence(pair.confidence, conf),
                val,
                reason,
                support,
            );
            r.alignment_id = Some(pair.alignment_id.clone());
            records.push(r);
        };
        let ill = max_cond > 1e3;
        let summ_val = if solved == 0 {
            Validity::InsufficientSupport
        } else if ill {
            Validity::LowCoherence
        } else {
            up_val
        };
        let summ_reason = if solved == 0 {
            Some("no supported band for mix estimate".to_string())
        } else if ill {
            Some(format!("mix estimate ill-conditioned ({max_cond:.3e})"))
        } else {
            None
        };
        summary(
            &mut out.records,
            "exp.mix_fit_quality",
            if solved > 0 {
                resid_acc / resid_n as f64
            } else {
                f64::NAN
            },
            "ratio",
            Confidence::Medium,
            summ_val,
            summ_reason.clone(),
            resid_n,
        );
        summary(
            &mut out.records,
            "exp.mix_polarity",
            if solved > 0 { dom_sign } else { f64::NAN },
            "sign",
            Confidence::Medium,
            summ_val,
            summ_reason.clone(),
            solved,
        );
        summary(
            &mut out.records,
            "exp.mix_condition",
            if solved > 0 { max_cond } else { f64::NAN },
            "ratio",
            Confidence::Low,
            if solved > 0 { up_val } else { summ_val },
            summ_reason,
            solved,
        );
    }
}

/// Solve the complex system `A·w = b` by Gaussian elimination with
/// partial pivoting. Returns the solution and a rough conditioning
/// proxy (largest/smallest pivot magnitude ratio); `None` when the
/// system is singular — the caller decides the validity.
fn csolve(a: &[Vec<(f64, f64)>], b: &[(f64, f64)]) -> Option<(Vec<(f64, f64)>, f64)> {
    let n = a.len();
    if n == 0 || b.len() != n || a.iter().any(|r| r.len() != n) {
        return None;
    }
    let mut m: Vec<Vec<(f64, f64)>> = a
        .iter()
        .zip(b)
        .map(|(row, &bi)| row.iter().copied().chain([bi]).collect())
        .collect();
    let cabs = |v: (f64, f64)| (v.0 * v.0 + v.1 * v.1).sqrt();
    let cdiv = |x: (f64, f64), y: (f64, f64)| {
        let d = y.0 * y.0 + y.1 * y.1;
        ((x.0 * y.0 + x.1 * y.1) / d, (x.1 * y.0 - x.0 * y.1) / d)
    };
    let cmul = |x: (f64, f64), y: (f64, f64)| (x.0 * y.0 - x.1 * y.1, x.0 * y.1 + x.1 * y.0);
    let (mut piv_max, mut piv_min) = (0.0f64, f64::MAX);
    for col in 0..n {
        let pivot = (col..n).max_by(|&i, &j| cabs(m[i][col]).total_cmp(&cabs(m[j][col])))?;
        let pmag = cabs(m[pivot][col]);
        if pmag < 1e-30 {
            return None;
        }
        piv_max = piv_max.max(pmag);
        piv_min = piv_min.min(pmag);
        m.swap(col, pivot);
        for row in col + 1..n {
            let f = cdiv(m[row][col], m[col][col]);
            for c in col..=n {
                m[row][c] = (
                    m[row][c].0 - cmul(f, m[col][c]).0,
                    m[row][c].1 - cmul(f, m[col][c]).1,
                );
            }
        }
    }
    let mut w = vec![(0.0, 0.0); n];
    for row in (0..n).rev() {
        let mut s = (0.0, 0.0);
        for c in row + 1..n {
            let p = cmul(m[row][c], w[c]);
            s = (s.0 + p.0, s.1 + p.1);
        }
        w[row] = cdiv((m[row][n].0 - s.0, m[row][n].1 - s.1), m[row][row]);
    }
    Some((w, if piv_min > 0.0 { piv_max / piv_min } else { f64::INFINITY }))
}
