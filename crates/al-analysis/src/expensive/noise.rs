//! Noise decomposition families: tonal peak / hum-harmonic counts
//! plus the stationary / tonal / modulated classification of the
//! band-energy series.

use al_core::{Confidence, Validity};
use al_dataset::types::FeatureRecord;

use super::{Emit, ExpensiveResult, RegionCtx};

/// Emit this family's invalid schema for a region selected by
/// non-`Valid` evidence (same ids `run` would produce, as NaN
/// scalars with the mapped validity).
pub(crate) fn emit_invalid(e: &Emit<'_>, records: &mut Vec<FeatureRecord>, reason: &str) {
    for (id, unit) in [
        ("exp.tonal_peak_count", "count"),
        ("exp.hum_harmonic_count", "count"),
        ("exp.noise_stationary_score", "score"),
        ("exp.noise_tonal_score", "score"),
        ("exp.noise_modulated_score", "score"),
        ("exp.noise_class", "class"),
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

/// Run the noise-decomposition families over the region.
pub(crate) fn run(ctx: &RegionCtx<'_>, e: &Emit<'_>, out: &mut ExpensiveResult) {
    let rate = ctx.rate;
    let half = ctx.stft.fft_size / 2;
    let frames = ctx.stft.frames();
    let mean_power = &ctx.mean_power;
    let bin_hz = ctx.bin_hz;
    let total_power = ctx.total_power;

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
        out.records.push(e.push_spec(
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
        out.records.push(e.push_spec(
            "exp.hum_harmonic_count",
            hum as f64,
            "count",
            Confidence::Medium,
            Validity::Valid,
            None,
            frames as u64,
        ));
    }

    // ==== Stationary / tonal / modulated classification ====
    // The residual-ish content of a region is characterized by the
    // band-energy series: its temporal variability (stationary), the
    // flatness of the persistent floor spectrum (tonal), and the
    // 0.5–20 Hz share of its envelope spectrum (modulated).
    {
        let supported = frames >= 8 && total_power > 0.0;
        let power = ctx.stft.power();
        // Per-frame total (one-sided) energy.
        let pt: Vec<f64> = power
            .iter()
            .map(|f| f[..half].iter().sum::<f64>())
            .collect();
        let mean_pt = pt.iter().sum::<f64>() / pt.len().max(1) as f64;
        let cv = if mean_pt > 0.0 {
            (pt.iter().map(|v| (v - mean_pt).powi(2)).sum::<f64>() / pt.len() as f64).sqrt()
                / mean_pt
        } else {
            0.0
        };
        // Persistent floor spectrum: per-bin 10th percentile across
        // frames. Its flatness separates broadband noise floors from
        // stable tones.
        let flatness = {
            let mut floor = Vec::with_capacity(half);
            for k in 1..half {
                let mut col: Vec<f64> = power.iter().map(|f| f[k]).collect();
                col.sort_by(|a, b| a.total_cmp(b));
                floor.push(col[(col.len() as f64 * 0.1) as usize] + 1e-18);
            }
            let geo = (floor.iter().map(|v| v.ln()).sum::<f64>() / floor.len() as f64).exp();
            let arith = floor.iter().sum::<f64>() / floor.len() as f64;
            (geo / arith).clamp(0.0, 1.0)
        };
        // Modulation: peakiness of the envelope spectrum inside the
        // 0.5 Hz–0.8·Nyquist band — a periodic modulation
        // concentrates the band's energy at one bin while a flat
        // (stationary) envelope spreads it. A plain band share
        // cannot separate the two when the frame rate barely
        // exceeds the band's top.
        let modulated = {
            let mean = pt.iter().sum::<f64>() / pt.len().max(1) as f64;
            let nfft = (pt.len() * 2).next_power_of_two();
            let mut re = vec![0.0; nfft];
            for (i, &v) in pt.iter().enumerate() {
                re[i] = v - mean;
            }
            let mut im = vec![0.0; nfft];
            al_locate::dsp::fft(&mut re, &mut im, false);
            let fs = rate as f64 / ctx.stft.hop as f64;
            let fres = fs / nfft as f64;
            let f_hi = 20.0f64.min(fs / 2.0 * 0.8);
            let kmin = (0.5 / fres).ceil() as usize;
            let kmax = ((f_hi / fres) as usize).min(nfft / 2);
            if kmax > kmin {
                let bin_e = |k: usize| re[k] * re[k] + im[k] * im[k];
                let band: f64 = (kmin..=kmax).map(&bin_e).sum();
                // Peak share counts the strongest 3 consecutive bins
                // so spectral leakage of an off-grid modulation
                // frequency stays inside the peak.
                let peak = (kmin..kmax.saturating_sub(1))
                    .map(|k| bin_e(k) + bin_e(k + 1) + bin_e(k + 2))
                    .fold(0.0f64, f64::max);
                if band > 0.0 { peak / band } else { 0.0 }
            } else {
                0.0
            }
        };
        let stationary = (1.0 - 2.0 * cv).clamp(0.0, 1.0);
        let tonal = (1.0 - flatness).clamp(0.0, 1.0);
        // Class codes: a clear tone wins first, then a modulated
        // envelope, then stationarity; `unclassified` remains for
        // ambiguous cases.
        let class = if tonal > 0.8 {
            1.0
        } else if modulated > 0.6 {
            2.0
        } else if stationary > 0.5 {
            0.0
        } else {
            3.0
        };
        for (id, value) in [
            ("exp.noise_stationary_score", stationary),
            ("exp.noise_tonal_score", tonal),
            ("exp.noise_modulated_score", modulated),
        ] {
            out.records.push(e.push_spec(
                id,
                if supported { value } else { f64::NAN },
                "score",
                Confidence::Medium,
                if supported {
                    Validity::Valid
                } else {
                    Validity::InsufficientSupport
                },
                if supported {
                    None
                } else {
                    Some("too few frames or no energy for noise decomposition".into())
                },
                frames as u64,
            ));
        }
        let mut r = e.push_spec(
            "exp.noise_class",
            if supported { class } else { f64::NAN },
            "class",
            Confidence::Low,
            if supported {
                Validity::Valid
            } else {
                Validity::InsufficientSupport
            },
            if supported {
                None
            } else {
                Some("too few frames or no energy for noise decomposition".into())
            },
            frames as u64,
        );
        r.params = serde_json::json!({
            "region_s": [e.region().start_s, e.region().end_s],
            "triggers": e.region().triggers,
            "classes": ["stationary", "tonal", "modulated", "unclassified"],
        });
        out.records.push(r);
    }
}
