//! Spectral transfer families: precision cutoff (plus the optional
//! dictionary-matched cutoff delta), narrowband peaks/notches, and
//! the THD proxy. Estimation code is unchanged from the pre-split
//! `expensive.rs`; the module boundary only owns the feature ids.

use al_core::{Confidence, Validity};
use al_dataset::types::FeatureRecord;

use super::{Emit, ExpensiveResult, RegionCtx};

/// Emit this family's invalid schema for a region selected by
/// non-`Valid` evidence (same ids `run` would produce, as NaN
/// scalars with the mapped validity).
pub(crate) fn emit_invalid(
    e: &Emit<'_>,
    records: &mut Vec<FeatureRecord>,
    reason: &str,
    dictionary: bool,
) {
    let mut spectral = vec![
        ("exp.cutoff_hz", "hz"),
        ("exp.narrowband_peak_count", "count"),
        ("exp.narrowband_notch_count", "count"),
        ("exp.thd_ratio", "ratio"),
    ];
    if dictionary {
        spectral.insert(1, ("exp.dictionary_cutoff_delta_khz", "khz"));
    }
    for (id, unit) in spectral {
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

/// Run the spectral transfer families over the region.
pub(crate) fn run(ctx: &RegionCtx<'_>, e: &Emit<'_>, out: &mut ExpensiveResult) {
    let seg = ctx.segs[0];
    let n = seg.len();
    let bins = ctx.stft.fft_size;
    let half = bins / 2;
    let frames = ctx.stft.frames();
    let mean_power = &ctx.mean_power;
    let bin_hz = ctx.bin_hz;
    let total_power = ctx.total_power;
    let one_sided_power = ctx.one_sided_power;

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
        out.records.push(e.push_spec(
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
        if let Some(dict) = ctx.dictionary {
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
                let mut r = e.push_spec(
                    "exp.dictionary_cutoff_delta_khz",
                    d,
                    "khz",
                    Confidence::Medium,
                    Validity::Valid,
                    None,
                    n as u64,
                );
                r.params = serde_json::json!({
                    "region_s": [e.region().start_s, e.region().end_s],
                    "triggers": e.region().triggers,
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
        out.records.push(e.push_spec(
            "exp.narrowband_peak_count",
            peaks as f64,
            "count",
            Confidence::Medium,
            Validity::Valid,
            None,
            frames as u64,
        ));
        out.records.push(e.push_spec(
            "exp.narrowband_notch_count",
            notches as f64,
            "count",
            Confidence::Medium,
            Validity::Valid,
            None,
            frames as u64,
        ));
    }

    // ==== THD proxy ====
    // Strongest tonal bin's harmonic energy share.
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
        out.records.push(e.push_spec(
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
