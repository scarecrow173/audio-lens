//! Codec-artifact families: cepstrum echo detection, spectral holes
//! and block-boundary periodicity. Estimation code is unchanged
//! from the pre-split `expensive.rs`; the module boundary only owns
//! the feature ids.

use al_core::{Confidence, Validity};
use al_dataset::types::FeatureRecord;

use super::{Emit, ExpensiveResult, RegionCtx, emit_unsupported};

/// Emit this family's invalid schema for a region selected by
/// non-`Valid` evidence (same ids `run` would produce, as NaN
/// scalars with the mapped validity).
pub(crate) fn emit_invalid(e: &Emit<'_>, records: &mut Vec<FeatureRecord>, reason: &str) {
    for (id, unit) in [
        ("exp.codec_block_score", "ratio"),
        ("exp.codec_block_size", "samples"),
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
        ("exp.pre_echo_ratio", "ratio"),
        ("exp.transient_smear_ms", "ms"),
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
        ("exp.echo_delay_s", "seconds"),
        ("exp.cepstrum_prominence", "ratio"),
        ("exp.echo2_delay_s", "seconds"),
        ("exp.echo2_level_ratio", "ratio"),
        ("exp.spectral_hole_ratio", "ratio"),
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

/// Run the codec-artifact families over the region.
pub(crate) fn run(ctx: &RegionCtx<'_>, e: &Emit<'_>, out: &mut ExpensiveResult) {
    let seg = ctx.segs[0];
    let n = seg.len();
    let rate = ctx.rate;
    let bins = ctx.stft.fft_size;
    let half = bins / 2;
    let frames = ctx.stft.frames();
    let mean_power = &ctx.mean_power;
    let total_power = ctx.total_power;

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
        ruviform_numeric::fft::Fft::new(size)
            .expect("fft length must be a power of two")
            .transform(&mut re, &mut im, true)
            .expect("fft buffer lengths must match the plan");
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
            out.records.push(e.push_spec(
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
            out.records.push(e.push_spec(
                "exp.cepstrum_prominence",
                prominence,
                "ratio",
                Confidence::Medium,
                Validity::Valid,
                None,
                frames as u64,
            ));
            // Secondary echo: strongest cepstral peak outside a
            // guard window around the primary AND outside integer
            // multiples of its quefrency — a single comb echoes
            // back at every m·qi, so those replicas are the first
            // reflection's harmonics, not evidence of a second one.
            let guard = (qi / 8).max(qmin / 2).max(4);
            let harmonic =
                |q: usize| qi > 0 && (2..=qmax / qi).any(|m| q.abs_diff(m * qi) <= guard);
            let (qi2, &peak2) = re[qmin..=qmax]
                .iter()
                .enumerate()
                .filter(|(i, _)| {
                    let q = i + qmin;
                    q.abs_diff(qi) > guard && !harmonic(q)
                })
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(i, v)| (i + qmin, v))
                .unwrap_or((qmin, &0.0));
            let prominence2 = if baseline.abs() > 1e-12 {
                (peak2 - baseline) / baseline.abs()
            } else {
                0.0
            };
            let supported2 = supported && prominence2 > 0.5;
            out.records.push(e.push_spec(
                "exp.echo2_delay_s",
                if supported2 {
                    qi2 as f64 / rate as f64
                } else {
                    f64::NAN
                },
                "seconds",
                Confidence::Medium,
                if supported2 {
                    Validity::Valid
                } else {
                    Validity::InsufficientSupport
                },
                if supported2 {
                    None
                } else {
                    Some("no significant secondary cepstral peak".into())
                },
                frames as u64,
            ));
            out.records.push(e.push_spec(
                "exp.echo2_level_ratio",
                if supported2 {
                    peak2 / peak.max(1e-30)
                } else {
                    f64::NAN
                },
                "ratio",
                Confidence::Low,
                if supported2 {
                    Validity::Valid
                } else {
                    Validity::InsufficientSupport
                },
                if supported2 {
                    None
                } else {
                    Some("no significant secondary cepstral peak".into())
                },
                frames as u64,
            ));
        } else {
            super::emit_unsupported_spec(
                e,
                &mut out.records,
                &[
                    ("exp.echo_delay_s", "seconds"),
                    ("exp.cepstrum_prominence", "ratio"),
                    ("exp.echo2_delay_s", "seconds"),
                    ("exp.echo2_level_ratio", "ratio"),
                ],
                "no supported quefrency interval",
            );
        }
    }

    // ==== Pre-echo / transient smear ====
    // A small-hop STFT resolves the ~5–20 ms window where transform
    // codecs smear transient energy backward in time — the region's
    // fine STFT hop (~43 ms at the default config) is too coarse.
    {
        let pw = 1024usize;
        let ph = 256usize;
        if n >= 2 * pw {
            let mm = crate::stft::stft(seg, pw, ph, rate);
            let p = mm.power();
            let fr = mm.frames();
            let k_lo = ((4_000.0 * mm.fft_size as f64) / rate as f64) as usize;
            let k_hi = mm.fft_size / 2;
            if fr >= 4 && k_lo < k_hi {
                // High-band energy per frame — codec pre-echo lives
                // in the band the transform quantizes coarsely.
                let hb: Vec<f64> = (0..fr)
                    .map(|f| p[f][k_lo..k_hi].iter().sum::<f64>())
                    .collect();
                // Strongest onset: largest frame-to-frame rise.
                let mut onset = 0usize;
                let mut best_rise = 0.0f64;
                for t in 1..fr {
                    let r = hb[t] / hb[t - 1].max(1e-18);
                    if r > best_rise {
                        best_rise = r;
                        onset = t;
                    }
                }
                let onset_ok = best_rise > 4.0 && hb[onset] > 1e-12;
                // Pre-echo: high-band energy in the ~10 ms preceding
                // the onset, relative to right after it. A clean
                // transient has near-zero pre energy.
                let pre_lo = onset.saturating_sub(2);
                let pre_e = hb[pre_lo..onset].iter().sum::<f64>() / (onset - pre_lo).max(1) as f64;
                let post_hi = (onset + 3).min(fr);
                let post_e = hb[onset + 1..post_hi].iter().sum::<f64>()
                    / (post_hi - onset - 1).max(1) as f64;
                out.records.push(e.push(
                    "exp.pre_echo_ratio",
                    pre_e / post_e.max(1e-18),
                    "ratio",
                    Confidence::Medium,
                    if onset_ok {
                        Validity::Valid
                    } else {
                        Validity::InsufficientSupport
                    },
                    if onset_ok {
                        None
                    } else {
                        Some("no clear transient onset in region".into())
                    },
                    fr as u64,
                ));
                // Smear: 10%→90% rise time of the high-band envelope
                // around the onset. A codec-smeared transient spreads
                // over several small hops instead of one.
                let lo = 0.1 * hb[onset];
                let hi = 0.9 * hb[onset];
                let t10 = (0..onset)
                    .rev()
                    .find(|&t| hb[t] < lo)
                    .map(|t| t + 1)
                    .unwrap_or(0);
                let t90 = (t10..fr).find(|&t| hb[t] >= hi).unwrap_or(onset);
                let smear_ms = (t90.saturating_sub(t10)) as f64 * ph as f64 / rate as f64 * 1e3;
                out.records.push(e.push(
                    "exp.transient_smear_ms",
                    smear_ms,
                    "ms",
                    Confidence::Low,
                    if onset_ok {
                        Validity::Valid
                    } else {
                        Validity::InsufficientSupport
                    },
                    if onset_ok {
                        None
                    } else {
                        Some("no clear transient onset in region".into())
                    },
                    fr as u64,
                ));
            } else {
                emit_unsupported(
                    e,
                    &mut out.records,
                    &[
                        ("exp.pre_echo_ratio", "ratio"),
                        ("exp.transient_smear_ms", "ms"),
                    ],
                    "insufficient fine-STFT frames or no high band bins",
                );
            }
        } else {
            emit_unsupported(
                e,
                &mut out.records,
                &[
                    ("exp.pre_echo_ratio", "ratio"),
                    ("exp.transient_smear_ms", "ms"),
                ],
                "region shorter than two fine-analysis windows",
            );
        }
    }

    // ==== Codec artifact structure ====
    {
        // Spectral holes: bins in the top half below 1e-4 of max.
        if total_power > 0.0 {
            let maxp = mean_power[..half].iter().copied().fold(0.0, f64::max);
            let floor = maxp * 1e-4;
            let top = &mean_power[half / 2..half];
            let holes = top.iter().filter(|&&p| p < floor).count();
            out.records.push(e.push_spec(
                "exp.spectral_hole_ratio",
                holes as f64 / top.len().max(1) as f64,
                "ratio",
                Confidence::Medium,
                Validity::Valid,
                None,
                frames as u64,
            ));
        } else {
            super::emit_unsupported_spec(
                e,
                &mut out.records,
                &[("exp.spectral_hole_ratio", "ratio")],
                "spectrum has no supported energy",
            );
        }
        // ブロック境界の生エネルギーと内部の生エネルギーの比を、
        // 一般的な codec frame size ごとに評価する。
        let block_e: Vec<f64> = (0..n / 64)
            .map(|i| ruviform_numeric::vec::sum_sq(&seg[i * 64..(i + 1) * 64]))
            .collect();
        let mut best: Option<(f64, u64)> = None;
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
            if total_power > 0.0 && best.is_none_or(|b| ratio > b.0) {
                best = Some((ratio, size as u64));
            }
        }
        out.records.push(e.push(
            "exp.codec_block_score",
            best.map_or(f64::NAN, |b| b.0),
            "ratio",
            Confidence::Low,
            if best.is_some() {
                Validity::Valid
            } else {
                Validity::InsufficientSupport
            },
            if best.is_some() {
                None
            } else {
                Some("no supported codec block candidate".into())
            },
            if best.is_some() {
                block_e.len() as u64
            } else {
                0
            },
        ));
        out.records.push(e.push(
            "exp.codec_block_size",
            best.map_or(f64::NAN, |b| b.1 as f64),
            "samples",
            Confidence::Low,
            if best.is_some() {
                Validity::Valid
            } else {
                Validity::InsufficientSupport
            },
            if best.is_some() {
                None
            } else {
                Some("no supported codec block candidate".into())
            },
            if best.is_some() {
                block_e.len() as u64
            } else {
                0
            },
        ));
    }
}
