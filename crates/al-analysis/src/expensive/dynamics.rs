//! Dynamics families: polarity asymmetry, short-term level range,
//! attack/release, compressor/limiter transfer-curve candidates,
//! pumping/ducking modulation, and level-dependent transfer
//! (pair-dependent).

use al_core::{Confidence, Validity};
use al_dataset::types::FeatureRecord;
use al_locate::dsp::rms;

use crate::records::{propagate_confidence, propagate_validity, robust_summary};

use super::{Emit, ExpensiveResult, RegionCtx, emit_unsupported};

/// Emit this family's invalid schema for a region selected by
/// non-`Valid` evidence (same ids `run` would produce, as NaN
/// scalars with the mapped validity). The level-transfer ids are
/// pair-dependent but still emitted here — the trigger-validity
/// rule applies before pair availability is even consulted.
pub(crate) fn emit_invalid(e: &Emit<'_>, records: &mut Vec<FeatureRecord>, reason: &str) {
    for (id, unit) in [
        ("exp.polarity_asymmetry", "ratio"),
        ("exp.dynamic_range_db", "db"),
        ("exp.attack_db_per_20ms", "db"),
        ("exp.release_db_per_20ms", "db"),
        ("exp.transfer_knee_db", "dbfs"),
        ("exp.transfer_slope_above_knee", "ratio"),
        ("exp.pumping_mod_hz", "hz"),
        ("exp.pumping_mod_depth_db", "db"),
        ("exp.level_transfer_slope", "db_per_db"),
        ("exp.level_transfer_corr", "corr"),
    ] {
        let push = if id.starts_with("exp.level_transfer_") {
            Emit::push_pair
        } else {
            Emit::push
        };
        records.push(push(
            e,
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

/// Run the dynamics families over the region.
pub(crate) fn run(ctx: &RegionCtx<'_>, e: &Emit<'_>, out: &mut ExpensiveResult) {
    let seg = ctx.segs[0];
    let n = seg.len();
    let rate = ctx.rate;

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
        out.records.push(e.push(
            "exp.polarity_asymmetry",
            asym,
            "ratio",
            Confidence::Medium,
            Validity::Valid,
            None,
            n as u64,
        ));
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
                out.records.push(e.push(
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
            out.records.push(e.push(
                "exp.attack_db_per_20ms",
                rise,
                "db",
                Confidence::Low,
                Validity::Valid,
                None,
                levels.len() as u64,
            ));
            out.records.push(e.push(
                "exp.release_db_per_20ms",
                fall,
                "db",
                Confidence::Low,
                Validity::Valid,
                None,
                levels.len() as u64,
            ));
        } else {
            emit_unsupported(
                e,
                &mut out.records,
                &[
                    ("exp.dynamic_range_db", "db"),
                    ("exp.attack_db_per_20ms", "db"),
                    ("exp.release_db_per_20ms", "db"),
                ],
                "region shorter than four 20 ms level windows",
            );
        }
    }

    // ==== Transfer-curve candidate / pumping ====
    // Finer hop than the dynamics block above: 20 ms windows at a
    // 10 ms hop give a 100 Hz level-rate series, enough to resolve
    // 0.5–20 Hz pumping modulation.
    {
        let win = ((0.02 * rate as f64) as usize).max(1);
        let hop = (win / 2).max(1);
        if n >= 8 * win {
            let levels: Vec<f64> = (0..=n - win)
                .step_by(hop)
                .map(|s| 20.0 * rms(&seg[s..s + win]).max(1e-12).log10())
                .collect();
            if let Some(s) = robust_summary(&levels) {
                // Transfer-curve candidate: a compressor/limiter
                // flattens the top of the level distribution, so the
                // upper tail (above the p95 knee) compresses relative
                // to the mid range. This observes the *output* level
                // histogram — the input→output scatter requires pair
                // evidence and lives in `exp.level_transfer_*`.
                let mid = (s.p95 - s.p25).max(1e-9);
                let slope_above = (s.max - s.p95) / mid;
                out.records.push(e.push(
                    "exp.transfer_knee_db",
                    s.p95,
                    "dbfs",
                    Confidence::Medium,
                    Validity::Valid,
                    None,
                    s.support,
                ));
                out.records.push(e.push(
                    "exp.transfer_slope_above_knee",
                    slope_above,
                    "ratio",
                    Confidence::Low,
                    Validity::Valid,
                    None,
                    s.support,
                ));
                // Pumping/ducking: dominant low-frequency modulation
                // of the level series. A flat level trajectory cannot
                // carry a modulation estimate.
                let spread = s.p95 - s.p05;
                if spread >= 3.0 && levels.len() >= 32 {
                    let mean = levels.iter().sum::<f64>() / levels.len() as f64;
                    let nfft = (levels.len() * 2).next_power_of_two();
                    let mut re = vec![0.0; nfft];
                    for (i, &l) in levels.iter().enumerate() {
                        re[i] = l - mean;
                    }
                    let mut im = vec![0.0; nfft];
                    al_locate::dsp::fft(&mut re, &mut im, false);
                    let hop_s = hop as f64 / rate as f64;
                    let fs = 1.0 / (hop_s * nfft as f64);
                    let (mut best_k, mut best_a) = (0usize, 0.0f64);
                    for (k, (&rr, &ii)) in re.iter().zip(&im).enumerate() {
                        let f = k as f64 * fs;
                        if !(0.5..=20.0).contains(&f) {
                            continue;
                        }
                        let a = (rr * rr + ii * ii).sqrt();
                        if a > best_a {
                            best_a = a;
                            best_k = k;
                        }
                    }
                    let supported = best_a > 0.0;
                    out.records.push(e.push(
                        "exp.pumping_mod_hz",
                        if supported {
                            best_k as f64 * fs
                        } else {
                            f64::NAN
                        },
                        "hz",
                        Confidence::Medium,
                        if supported {
                            Validity::Valid
                        } else {
                            Validity::InsufficientSupport
                        },
                        if supported {
                            None
                        } else {
                            Some("no modulation energy in 0.5-20 Hz band".into())
                        },
                        levels.len() as u64,
                    ));
                    // The forward DFT is unnormalized: a modulation of
                    // peak amplitude A dB over n windows gives
                    // |X[k]| = A·n/2, so peak-to-peak depth is
                    // 4·|X[k]|/n (dB).
                    out.records.push(e.push(
                        "exp.pumping_mod_depth_db",
                        if supported {
                            4.0 * best_a / levels.len() as f64
                        } else {
                            f64::NAN
                        },
                        "db",
                        Confidence::Low,
                        if supported {
                            Validity::Valid
                        } else {
                            Validity::InsufficientSupport
                        },
                        if supported {
                            None
                        } else {
                            Some("no modulation energy in 0.5-20 Hz band".into())
                        },
                        levels.len() as u64,
                    ));
                } else {
                    emit_unsupported(
                        e,
                        &mut out.records,
                        &[
                            ("exp.pumping_mod_hz", "hz"),
                            ("exp.pumping_mod_depth_db", "db"),
                        ],
                        "level trajectory too flat or too short for modulation estimate",
                    );
                }
            }
        } else {
            emit_unsupported(
                e,
                &mut out.records,
                &[
                    ("exp.transfer_knee_db", "dbfs"),
                    ("exp.transfer_slope_above_knee", "ratio"),
                    ("exp.pumping_mod_hz", "hz"),
                    ("exp.pumping_mod_depth_db", "db"),
                ],
                "region shorter than eight 20 ms level windows",
            );
        }
    }

    // ==== Level-dependent transfer (pair-dependent) ====
    // Source level → gain scatter over 20 ms windows: a compressor
    // shows a negative slope (gain drops as input level rises).
    // Pair evidence is optional — no pair, no estimate, but the
    // feature ids are still recorded as invalid observations.
    {
        let unavailable = |records: &mut Vec<FeatureRecord>,
                           pair: Option<&super::PairEvidence<'_>>,
                           val: Validity,
                           reason: &str| {
            for (id, unit) in [
                ("exp.level_transfer_slope", "db_per_db"),
                ("exp.level_transfer_corr", "corr"),
            ] {
                let mut r = e.push_pair(
                    id,
                    f64::NAN,
                    unit,
                    pair.map_or(Confidence::Medium, |p| {
                        propagate_confidence(p.confidence, Confidence::Medium)
                    }),
                    val,
                    Some(reason.to_string()),
                    0,
                );
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
                // A valid trigger whose pair simply does not overlap the
                // region is still unsupported — never Valid.
                if up_val == Validity::Valid {
                    Validity::InsufficientSupport
                } else {
                    up_val
                },
                if up_val != Validity::Valid {
                    "pair alignment not accepted"
                } else {
                    "pair evidence does not overlap region"
                },
            );
            return;
        }
        let (o0, o1, src_segs) = aligned.unwrap();
        let Some(cap0) = ctx.chans.first() else {
            return;
        };
        let win = ((0.02 * rate as f64) as usize).max(1);
        let mut src_db = Vec::new();
        let mut gain_db = Vec::new();
        let mut w = o0;
        while w + win <= o1 {
            let i = w - o0;
            let s_lvl = rms(&src_segs[0][i..i + win]);
            let c_lvl = rms(&cap0[w..w + win]);
            if s_lvl > 1e-9 && c_lvl > 1e-9 {
                src_db.push(20.0 * s_lvl.log10());
                gain_db.push(20.0 * (c_lvl / s_lvl).log10());
            }
            w += win;
        }
        // The slope/corr is meaningless when the source level barely
        // varies — a flat input cannot reveal level dependence.
        let spread = robust_summary(&src_db)
            .map(|s| s.p95 - s.p05)
            .unwrap_or(0.0);
        if src_db.len() < 8 || spread < 3.0 {
            unavailable(
                &mut out.records,
                Some(pair),
                Validity::InsufficientSupport,
                "source level lacks spread for level-dependence fit",
            );
            return;
        }
        let n_w = src_db.len() as f64;
        let mx = src_db.iter().sum::<f64>() / n_w;
        let my = gain_db.iter().sum::<f64>() / n_w;
        let (sxy, sxx, syy) = src_db
            .iter()
            .zip(&gain_db)
            .fold((0.0, 0.0, 0.0), |a, (&x, &y)| {
                (
                    a.0 + (x - mx) * (y - my),
                    a.1 + (x - mx) * (x - mx),
                    a.2 + (y - my) * (y - my),
                )
            });
        let slope = if sxx > 0.0 { sxy / sxx } else { 0.0 };
        // Pearson r on the scatter — the cosine variant (`corr0`)
        // includes the means and cannot read the slope's sign.
        let corr = if sxx > 0.0 && syy > 0.0 {
            sxy / (sxx * syy).sqrt()
        } else {
            0.0
        };
        for (id, value, unit) in [
            ("exp.level_transfer_slope", slope, "db_per_db"),
            ("exp.level_transfer_corr", corr, "corr"),
        ] {
            let mut r = e.push_pair(
                id,
                value,
                unit,
                propagate_confidence(pair.confidence, Confidence::Medium),
                up_val,
                None,
                src_db.len() as u64,
            );
            r.alignment_id = Some(pair.alignment_id.clone());
            r.params = serde_json::json!({
                "region_s": [e.region().start_s, e.region().end_s],
                "triggers": e.region().triggers,
                "pair": pair.pair,
                "src_level_spread_db": spread,
                "source_channel": 0,
                "capture_channel": 0,
            });
            out.records.push(r);
        }
    }
}
