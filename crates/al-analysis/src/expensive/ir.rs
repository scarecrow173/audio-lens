//! Impulse-response family: IR candidate estimation plus the
//! early/late energy ratio and decay slope descriptors.
//!
//! This is a pair-dependent family: the IR estimate is a
//! deconvolution of capture by source and therefore requires the
//! reference-stage `intermediate/{pair}/transfer_h` artifact (see
//! [`super::PairEvidence`] for how pair evidence is located and why
//! it is optional). With no usable pair evidence the family emits
//! `pair evidence unavailable` invalid records — the design's
//! conservative alternative to a single-input homomorphic fallback.
//! Numerical instability is guarded by recording the supported-bin
//! fraction and refusing weak fits (`InsufficientSupport`).

use al_core::{Confidence, Validity};
use al_dataset::types::FeatureRecord;

use crate::records::{propagate_confidence, propagate_validity};

use super::{Emit, ExpensiveResult, RegionCtx};

const IDS: [(&str, &str); 2] = [
    ("exp.ir_early_late_ratio", "ratio"),
    ("exp.ir_decay_db_per_s", "db_per_s"),
];

/// Emit this family's invalid schema for a region selected by
/// non-`Valid` evidence (same ids `run` would produce, as NaN
/// scalars with the mapped validity).
pub(crate) fn emit_invalid(e: &Emit<'_>, records: &mut Vec<FeatureRecord>, reason: &str) {
    for (id, unit) in IDS {
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
}

/// Push one pair-derived record: carries the locate alignment id and
/// never exceeds the pair's own confidence.
#[allow(clippy::too_many_arguments)]
fn push_pair(
    e: &Emit<'_>,
    pair: &super::PairEvidence<'_>,
    id: &str,
    value: f64,
    unit: &str,
    conf: Confidence,
    val: Validity,
    reason: Option<String>,
    support: u64,
) -> FeatureRecord {
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
    r.params = serde_json::json!({
        "region_s": [e.region().start_s, e.region().end_s],
        "triggers": e.region().triggers,
        "pair": pair.pair,
        "method": "deconvolution",
    });
    r
}

/// Emit the family's records as invalid with one shared reason —
/// used for missing/unusable pair evidence.
fn emit_unavailable(
    e: &Emit<'_>,
    pair: Option<&super::PairEvidence<'_>>,
    records: &mut Vec<FeatureRecord>,
    val: Validity,
    reason: &str,
) {
    for (id, unit) in IDS {
        let mut r = e.push(
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
}

/// Run the IR family over the region.
pub(crate) fn run(ctx: &RegionCtx<'_>, e: &Emit<'_>, out: &mut ExpensiveResult) {
    let rate = ctx.rate;
    let Some(pair) = ctx.pair else {
        emit_unavailable(
            e,
            None,
            &mut out.records,
            Validity::InsufficientSupport,
            "pair evidence unavailable",
        );
        return;
    };
    let up_val = propagate_validity(pair.validity);
    if !pair.accepted || up_val != Validity::Valid {
        emit_unavailable(
            e,
            Some(pair),
            &mut out.records,
            up_val,
            "pair alignment not accepted",
        );
        return;
    }
    let Some(h) = pair.transfer_h.as_ref() else {
        emit_unavailable(
            e,
            Some(pair),
            &mut out.records,
            Validity::InsufficientSupport,
            "pair evidence unavailable",
        );
        return;
    };
    let n_bins = h.len();
    if n_bins < 64 || !n_bins.is_power_of_two() {
        emit_unavailable(
            e,
            Some(pair),
            &mut out.records,
            Validity::InsufficientSupport,
            "transfer_h too short to deconvolve",
        );
        return;
    }

    // IFFT H(f) → IR candidate h(t). Support guard: deconvolution
    // divides by |X|², so bins with (near-)zero source energy make
    // the estimate unstable — the supported fraction is recorded and
    // a mostly-unsupported spectrum marks the records invalid.
    let half = n_bins / 2;
    let max_mag2 = h[..half]
        .iter()
        .map(|(r, i)| r * r + i * i)
        .fold(0.0f64, f64::max);
    let supported_bins = h[..half]
        .iter()
        .filter(|(r, i)| r * r + i * i > max_mag2 * 1e-6)
        .count();
    let support_fraction = supported_bins as f64 / half as f64;
    if support_fraction < 0.1 {
        emit_unavailable(
            e,
            Some(pair),
            &mut out.records,
            Validity::InsufficientSupport,
            "transfer_h supported on too few bins for deconvolution",
        );
        return;
    }

    let mut re: Vec<f64> = h.iter().map(|v| v.0).collect();
    let mut im: Vec<f64> = h.iter().map(|v| v.1).collect();
    al_locate::dsp::fft(&mut re, &mut im, true);
    let mag2: Vec<f64> = re
        .iter()
        .zip(&im)
        .map(|(r, i)| r * r + i * i)
        .collect();
    let (peak, _) = mag2
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .unwrap_or((0, &0.0));

    // Early energy: main-peak neighborhood (0.25 ms pre-peak guard
    // through 2.5 ms). Late energy: the remaining tail from 5 ms.
    let early_lo = peak.saturating_sub((0.00025 * rate as f64) as usize);
    let early_hi = (peak + (0.0025 * rate as f64) as usize).min(mag2.len());
    let late_lo = (peak + (0.005 * rate as f64) as usize).min(mag2.len());
    let e_early: f64 = mag2[early_lo..early_hi].iter().sum();
    let e_late: f64 = mag2[late_lo..].iter().sum();
    let early_late = e_early / e_late.max(1e-30);
    let mut rec = push_pair(
        e,
        pair,
        "exp.ir_early_late_ratio",
        early_late,
        "ratio",
        Confidence::Medium,
        Validity::Valid,
        None,
        supported_bins as u64,
    );
    rec.params["peak_sample"] = serde_json::json!(peak);
    rec.params["supported_bin_fraction"] = serde_json::json!(support_fraction);
    out.records.push(rec);

    // Decay: Schroeder-style backward-integrated energy decay curve,
    // linear fit over the −5..−35 dB range (a 30 dB evaluation
    // range). A tail too short to span that range cannot carry a
    // slope — record InsufficientSupport rather than a fake number.
    let total_e: f64 = mag2.iter().sum();
    if total_e <= 0.0 {
        emit_unavailable(
            e,
            Some(pair),
            &mut out.records,
            Validity::InsufficientSupport,
            "IR estimate has no energy",
        );
        return;
    }
    let mut edc = vec![0.0; mag2.len()];
    let mut acc = 0.0;
    for i in (0..mag2.len()).rev() {
        acc += mag2[i];
        edc[i] = acc;
    }
    let fit_range: Vec<usize> = (0..mag2.len())
        .filter(|&i| {
            let db = 10.0 * (edc[i] / total_e).log10();
            (-35.0..=-5.0).contains(&db)
        })
        .collect();
    if fit_range.len() < 8 {
        out.records.push(push_pair(
            e,
            pair,
            "exp.ir_decay_db_per_s",
            f64::NAN,
            "db_per_s",
            Confidence::Medium,
            Validity::InsufficientSupport,
            Some("IR tail too short to fit decay slope".into()),
            fit_range.len() as u64,
        ));
        return;
    }
    let (sx, sy, sxy, sxx) = fit_range.iter().fold((0.0, 0.0, 0.0, 0.0), |a, &i| {
        let x = i as f64;
        let y = 10.0 * (edc[i] / total_e).log10();
        (a.0 + x, a.1 + y, a.2 + x * y, a.3 + x * x)
    });
    let nf = fit_range.len() as f64;
    let denom = sxx * nf - sx * sx;
    let slope = if denom != 0.0 {
        (sxy * nf - sx * sy) / denom * rate as f64
    } else {
        0.0
    };
    let mut dec = push_pair(
        e,
        pair,
        "exp.ir_decay_db_per_s",
        slope,
        "db_per_s",
        Confidence::Medium,
        Validity::Valid,
        None,
        fit_range.len() as u64,
    );
    dec.params["edc_range_db"] = serde_json::json!([-5.0, -35.0]);
    dec.params["peak_sample"] = serde_json::json!(peak);
    dec.params["supported_bin_fraction"] = serde_json::json!(support_fraction);
    out.records.push(dec);
}
