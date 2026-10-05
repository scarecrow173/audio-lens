//! Timing families: SRC image detection, repeated-buffer score,
//! dropout gaps, click/splice localization, deeper stuck-buffer
//! analysis and the pair-dependent local time map.

use al_core::{Confidence, Validity};
use al_dataset::types::{FeatureRecord, Value};

use crate::cheap::NamedTensor;
use crate::records::{propagate_confidence, propagate_validity};

use super::{Emit, ExpensiveResult, RegionCtx, corr0, rms};

/// Emit this family's invalid schema for a region selected by
/// non-`Valid` evidence (same ids `run` would produce, as NaN
/// scalars with the mapped validity). `exp.local_time_map` is
/// pair-dependent but still emitted here — trigger validity rules
/// before pair availability is consulted.
pub(crate) fn emit_invalid(e: &Emit<'_>, records: &mut Vec<FeatureRecord>, reason: &str) {
    for (id, unit) in [
        ("exp.repeat_block_score", "corr"),
        ("exp.gap_count", "count"),
        ("exp.splice_count", "count"),
        ("exp.splice_first_s", "seconds"),
        ("exp.stuck_block_count", "count"),
        ("exp.stuck_max_ms", "ms"),
        ("exp.local_time_map", "tensor_ref"),
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
        ("exp.src_image_score", "corr"),
        ("exp.src_candidate_rate", "hz"),
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

/// Run the timing families over the region.
pub(crate) fn run(ctx: &RegionCtx<'_>, e: &Emit<'_>, out: &mut ExpensiveResult) {
    let seg = ctx.segs[0];
    let n = seg.len();
    let rate = ctx.rate;
    let half = ctx.stft.fft_size / 2;
    let frames = ctx.stft.frames();
    let mean_power = &ctx.mean_power;
    let bin_hz = ctx.bin_hz;

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
        out.records.push(e.push_spec(
            "exp.src_image_score",
            if evaluated { best_score } else { f64::NAN },
            "corr",
            Confidence::Medium,
            src_val,
            src_reason.clone(),
            frames as u64,
        ));
        out.records.push(e.push_spec(
            "exp.src_candidate_rate",
            if evaluated { best_rate } else { f64::NAN },
            "hz",
            Confidence::Low,
            src_val,
            src_reason,
            frames as u64,
        ));
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
            out.records.push(e.push(
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
        out.records.push(e.push(
            "exp.gap_count",
            gaps as f64,
            "count",
            Confidence::Medium,
            Validity::Valid,
            None,
            (n / win.max(1)) as u64,
        ));
    }

    // ==== Click/splice localization ====
    // Sample-to-sample discontinuities far above the region's robust
    // baseline mark clicks/splices. A 1 ms refractory merges the
    // oscillating tail of one click into a single detection; an
    // energy floor keeps digital silence's edge steps out.
    {
        let win = ((0.002 * rate as f64) as usize).max(1);
        let mut sorted: Vec<f64> = seg.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let med = sorted.get(sorted.len() / 2).copied().unwrap_or(0.0);
        let thresh = (4.0 * med).max(0.15);
        let refrac = (0.001 * rate as f64) as usize + 1;
        let mut last = 0usize;
        let mut count = 0u64;
        let mut first_s = f64::NAN;
        for (i, &d) in seg
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .collect::<Vec<_>>()
            .iter()
            .enumerate()
        {
            if d <= thresh {
                continue;
            }
            // A step inside digital silence is an edge, not a splice.
            let lo = i.saturating_sub(win);
            if rms(&seg[lo..(i + win).min(n)]) < 1e-4 {
                continue;
            }
            if count > 0 && i - last < refrac {
                continue;
            }
            last = i;
            count += 1;
            if count == 1 {
                first_s = (ctx.s0 + i) as f64 / rate as f64;
            }
        }
        let supported = n >= 2 * win;
        for (id, value, unit) in [
            ("exp.splice_count", count as f64, "count"),
            ("exp.splice_first_s", first_s, "seconds"),
        ] {
            out.records.push(e.push(
                id,
                if supported { value } else { f64::NAN },
                unit,
                Confidence::Medium,
                if supported {
                    Validity::Valid
                } else {
                    Validity::InsufficientSupport
                },
                if supported {
                    None
                } else {
                    Some("region too short for splice analysis".into())
                },
                count,
            ));
        }
    }

    // ==== Stuck / repeated-buffer runs ====
    // `exp.repeat_block_score` measures adjacent-block similarity;
    // a stuck output buffer instead repeats a block *bit-near-
    // exactly*, forming runs. Runs of ≥2 repeated non-silent blocks
    // are counted along with the longest run's duration.
    {
        let block = 1024usize;
        let supported = n >= 2 * block;
        let mut runs = 0u64;
        let mut max_len = 0usize;
        let mut cur = 0usize;
        if supported {
            let blocks: Vec<&[f64]> = seg.chunks_exact(block).collect();
            for w in blocks.windows(2) {
                let same =
                    rms(w[0]) > 1e-4 && w[0].iter().zip(w[1]).all(|(&a, &b)| (a - b).abs() < 1e-9);
                if same {
                    cur += 1;
                    max_len = max_len.max(cur + 1);
                } else {
                    if cur > 0 {
                        runs += 1;
                    }
                    cur = 0;
                }
            }
            if cur > 0 {
                runs += 1;
            }
        }
        for (id, value, unit) in [
            ("exp.stuck_block_count", runs as f64, "count"),
            (
                "exp.stuck_max_ms",
                max_len as f64 * block as f64 / rate as f64 * 1000.0,
                "ms",
            ),
        ] {
            out.records.push(e.push(
                id,
                if supported { value } else { f64::NAN },
                unit,
                Confidence::Medium,
                if supported {
                    Validity::Valid
                } else {
                    Validity::InsufficientSupport
                },
                if supported {
                    None
                } else {
                    Some("region shorter than two analysis blocks".into())
                },
                (n / block) as u64,
            ));
        }
    }

    // ==== Local time map (pair-dependent) ====
    // Per 20 ms window at a 10 ms hop, the residual lag between the
    // aligned source and the capture is searched over ±win/4 —
    // global alignment removed the large offset, so the map exposes
    // local drift/jitter. Persisted as a tensor of
    // `[cap_time_s, lag_samples, ncc]` rows referenced from the
    // `exp.local_time_map` record. No pair → invalid record.
    {
        let path = format!("features/expensive/{}/time_map", ctx.region_name);
        let mut unavailable =
            |val: Validity, reason: &str, pair: Option<&super::PairEvidence<'_>>| {
                let conf = pair.map_or(Confidence::Medium, |p| {
                    propagate_confidence(p.confidence, Confidence::Medium)
                });
                let mut r = e.push(
                    "exp.local_time_map",
                    f64::NAN,
                    "tensor_ref",
                    conf,
                    val,
                    Some(reason.to_string()),
                    0,
                );
                if let Some(p) = pair {
                    r.alignment_id = Some(p.alignment_id.clone());
                }
                out.records.push(r);
            };
        match ctx.pair {
            None => unavailable(
                Validity::InsufficientSupport,
                "pair evidence unavailable",
                None,
            ),
            Some(pair) => {
                let up_val = propagate_validity(pair.validity);
                let aligned = pair.aligned_slice(ctx.s0, ctx.s1);
                if up_val != Validity::Valid || aligned.is_none() {
                    unavailable(
                        // A valid trigger whose pair does not overlap
                        // the region is still unsupported — never Valid.
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
                        Some(pair),
                    );
                } else if let Some((o0, o1, src_segs)) = aligned {
                    let cap = ctx.chans[0];
                    let win = ((0.02 * rate as f64) as usize).max(64);
                    let hop = (win / 2).max(1);
                    let max_lag = (win / 4).max(1);
                    let mut rows: Vec<Vec<f64>> = Vec::new();
                    if o1 - o0 >= win && !src_segs.is_empty() {
                        let src = src_segs[0];
                        let mut w = o0;
                        while w + win <= o1 {
                            let s = &src[w - o0..w - o0 + win];
                            let c = &cap[w..w + win];
                            let (mut best_l, mut best_c) = (0isize, f64::NEG_INFINITY);
                            for l in -(max_lag as isize)..=max_lag as isize {
                                // Overlapping span for lag `l`
                                // (positive = capture delayed).
                                let (a0, b0) = if l >= 0 {
                                    (l as usize, 0usize)
                                } else {
                                    (0usize, (-l) as usize)
                                };
                                let len = win - a0.max(b0);
                                if len == 0 {
                                    continue;
                                }
                                let cc = corr0(&c[a0..a0 + len], &s[b0..b0 + len]);
                                if cc > best_c {
                                    best_c = cc;
                                    best_l = l;
                                }
                            }
                            rows.push(vec![w as f64 / rate as f64, best_l as f64, best_c]);
                            w += hop;
                        }
                    }
                    if rows.is_empty() {
                        unavailable(
                            Validity::InsufficientSupport,
                            "aligned pair region shorter than time-map window",
                            Some(pair),
                        );
                    } else {
                        let nrow = rows.len();
                        let mut rec = e.push(
                            "exp.local_time_map",
                            nrow as f64,
                            "tensor_ref",
                            propagate_confidence(pair.confidence, Confidence::Medium),
                            up_val,
                            None,
                            nrow as u64,
                        );
                        rec.alignment_id = Some(pair.alignment_id.clone());
                        rec.value = Value::Tensor {
                            array: path.clone(),
                            region: vec![[0, nrow as u64], [0, 3]],
                        };
                        out.records.push(rec);
                        let cols = rows[0].len() as u64;
                        let mut data = Vec::with_capacity(nrow * 3);
                        for row in &rows {
                            data.extend_from_slice(row);
                        }
                        out.tensors.push(NamedTensor {
                            path,
                            shape: [nrow as u64, cols],
                            data,
                        });
                    }
                }
            }
        }
    }
}
