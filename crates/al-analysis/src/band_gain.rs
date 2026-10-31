//! 帯域ラグ補償 LS。汎用カーネルは ruviform-numeric(AL#101 で
//! `al-math` 経由から移行)、帯域波形は `BandFilter` で帯域ごと
//! 逐次生成する(全帯域同時保持を避ける)。
use al_core::{Confidence, Validity};
use al_dataset::types::{ChannelSelector, Delta, Range, ResolutionClass};
use al_locate::dsp::{BandFilter, CorrelationPlan, CorrelationWorkspace, dot, sum_sq};
use ruviform_numeric::xspec::CrossSpectrum;

const MIN_CORRELATION: f64 = 0.5;
// コヒーレントなパワーが半分以上の帯域だけを支持する。
const MIN_COHERENCE: f64 = 0.5;

use crate::{
    records::{RecordBuilder, propagate_confidence, robust_summary},
    reference::{Aligned, ReferenceConfig, ReferenceResult, matrix_to_tensor},
    stft::{stft, third_octave_bands},
};

// 偶数標本では中央 2 値を平均する。既存 percentile summary の丸めとは分離する。
fn summarize(values: &[f64]) -> Option<crate::records::RobustSummary> {
    let mut summary = robust_summary(values)?;
    let median = |values: &[f64]| {
        let mut v: Vec<_> = values.iter().copied().filter(|v| v.is_finite()).collect();
        v.sort_by(f64::total_cmp);
        (v[(v.len() - 1) / 2] + v[v.len() / 2]) / 2.0
    };
    summary.median = median(values);
    summary.mad = median(
        &values
            .iter()
            .map(|v| (v - summary.median).abs())
            .collect::<Vec<_>>(),
    );
    Some(summary)
}

#[derive(Clone, Debug)]
struct Window {
    start: usize,
    lag: isize,
    gain: f64,
    corr: f64,
    support: usize,
    reason: &'static str,
}

pub(crate) fn parameters(cfg: &ReferenceConfig, rate: u32) -> serde_json::Value {
    serde_json::json!({
        "sample_rate": rate,
        "window_s": cfg.band_window_s,
        "hop_s": cfg.band_hop_s,
        "max_lag_s": cfg.band_max_lag_s,
        "window_samples": (cfg.band_window_s * rate as f64).round().max(1.0) as usize,
        "hop_samples": (cfg.band_hop_s * rate as f64).round().max(1.0) as usize,
        "max_lag_samples": (cfg.band_max_lag_s * rate as f64).round().max(1.0) as usize,
        "coherence_estimator": "welch_magnitude_squared",
        "coherence_window_samples": cfg.stft_window,
        "coherence_hop_samples": cfg.stft_hop,
        "min_absolute_correlation": MIN_CORRELATION,
        "min_coherence": MIN_COHERENCE,
        "filter": "spectral-mask-half-open-zero-pad-truncate-v1",
        "bands_hz": third_octave_bands(40.0, (rate as f64/2.0).min(16000.0)),
        "filter_guard": "one_period_at_band_lower_edge",
        "gain_energy_floor": 1e-20,
        "tie_tolerance": 1e-12,
        "tie_break": "absolute_correlation,absolute_lag,negative_lag,time",
        "aggregation": "polarity_groups_top_ceil_half_signed_gain_median"
    })
}

fn windows(
    x: &[f64],
    y: &[f64],
    window: usize,
    hop: usize,
    max: usize,
    guard: usize,
) -> Vec<Window> {
    let n = x.len().min(y.len());
    let margin = guard + max;
    if n < 2 * margin + window {
        return Vec::new();
    }
    let plan = CorrelationPlan::new_ncc(window, window + 2 * max)
        .expect("correlation plan shape must be valid");
    let mut workspace = CorrelationWorkspace::new(&plan);
    (margin..=n - margin - window)
        .step_by(hop)
        .map(|start| {
            let a = &x[start..start + window];
            let curve = plan
                .ncc(
                    &mut workspace,
                    a,
                    &y[start - max..start + window + max],
                    2 * max,
                )
                .expect("correlation template/signal shapes must be valid")
                .values;
            let peak = curve.iter().map(|c| c.abs()).fold(0.0, f64::max);
            // FFT の丸め誤差内の同点は、絶対 lag と固定符号順で選ぶ。
            let best = (0..curve.len())
                .filter(|&i| peak - curve[i].abs() <= 1e-12 * peak.max(1.0))
                .min_by_key(|&i| ((i as isize - max as isize).abs(), i))
                .unwrap();
            let lag = best as isize - max as isize;
            let b = &y[start - max + best..start - max + best + window];
            let energy: f64 = sum_sq(a);
            let gain = if energy > 1e-20 {
                dot(a, b) / energy
            } else {
                f64::NAN
            };
            let corr = curve[best].clamp(-1.0, 1.0);
            let reason = if !gain.is_finite() || sum_sq(b) <= 1e-20 {
                "silence"
            } else if lag.unsigned_abs() == max {
                "search_edge"
            } else if corr == 0.0 || gain == 0.0 || corr.signum() != gain.signum() {
                "polarity_unsupported"
            } else if corr.abs() < MIN_CORRELATION {
                "low_correlation"
            } else {
                "supported"
            };
            Window {
                start,
                lag,
                gain,
                corr,
                support: if reason == "silence" { 0 } else { window },
                reason,
            }
        })
        .collect()
}

fn select(windows: &[Window], inverted: bool) -> Vec<&Window> {
    let mut group: Vec<_> = windows
        .iter()
        .filter(|w| {
            matches!(w.reason, "supported" | "low_correlation") && (w.gain < 0.0) == inverted
        })
        .collect();
    group.sort_by(|a, b| {
        b.corr
            .abs()
            .total_cmp(&a.corr.abs())
            .then(a.start.cmp(&b.start))
    });
    group.truncate(group.len().div_ceil(2));
    group
}

fn coherence(
    x: &[f64],
    y: &[f64],
    rate: u32,
    cfg: &ReferenceConfig,
    bands: &[(f64, f64)],
) -> Vec<f64> {
    let a = stft(x, cfg.stft_window, cfg.stft_hop, rate);
    let b = stft(y, cfg.stft_window, cfg.stft_hop, rate);
    let frames = a.frames().min(b.frames());
    let bins = a.fft_size;
    // Welch の累積は `ruviform_numeric::xspec::CrossSpectrum` (AL#101。
    // 旧インライン loop との bit-exact 等価は #153 比較記録で検証済み)。
    let mut spectrum = CrossSpectrum::new(bins);
    for f in 0..frames {
        spectrum
            .accumulate(&a.re[f], &a.im[f], &b.re[f], &b.im[f])
            .expect("stft frame bin count must match");
    }
    let mut coh_re = vec![0.0; bins];
    let mut coh_im = vec![0.0; bins];
    spectrum
        .coherence_into(&mut coh_re, &mut coh_im)
        .expect("coherence buffers match spectrum bins");
    bands
        .iter()
        .map(|&(lo, hi)| {
            let low = (lo * bins as f64 / rate as f64).ceil() as usize;
            let high = ((hi * bins as f64 / rate as f64).ceil() as usize).min(bins / 2 + 1);
            if frames < 2 || high <= low {
                0.0
            } else {
                coh_re[low..high]
                    .iter()
                    .zip(&coh_im[low..high])
                    .map(|(&r, &i)| r * r + i * i)
                    .sum::<f64>()
                    / (high - low) as f64
            }
        })
        .collect()
}

pub(crate) fn analyze(
    pair: &str,
    alignment: &str,
    src: &Aligned<'_>,
    cfg: &ReferenceConfig,
    out: &mut ReferenceResult,
) {
    let rate = src.rate;
    let bands = third_octave_bands(40.0, (rate as f64 / 2.0).min(16000.0));
    let window = (cfg.band_window_s * rate as f64).round().max(1.0) as usize;
    let hop = (cfg.band_hop_s * rate as f64).round().max(1.0) as usize;
    let max = (cfg.band_max_lag_s * rate as f64).round().max(1.0) as usize;
    for (channel, (x, y)) in src.src.iter().zip(&src.cap).enumerate() {
        let mut src_bands = BandFilter::new(x, rate).expect("band filter input must be valid");
        let mut cap_bands = BandFilter::new(y, rate).expect("band filter input must be valid");
        let coh = coherence(x, y, rate, cfg, &bands);
        let all: Vec<_> = bands
            .iter()
            .map(|&(lo, hi)| {
                let x = src_bands.band(lo, hi);
                let y = cap_bands.band(lo, hi);
                windows(&x, &y, window, hop, max, (rate as f64 / lo).ceil() as usize)
            })
            .collect();
        // 400Hz は同じ帯域・同じ極性群の推定値だけを参照する。
        let anchor = bands.iter().position(|&(lo, hi)| lo <= 400.0 && 400.0 < hi);
        for inverted in [false, true] {
            let anchor_gain = anchor.and_then(|i| {
                let s = select(&all[i], inverted);
                (coh[i] >= MIN_COHERENCE && s.iter().all(|w| w.corr.abs() >= MIN_CORRELATION))
                    .then(|| {
                        summarize(&s.iter().map(|w| w.gain).collect::<Vec<_>>()).map(|s| s.median)
                    })
                    .flatten()
            });
            for (i, (&(lo, hi), ws)) in bands.iter().zip(&all).enumerate() {
                let selected = select(ws, inverted);
                let gains: Vec<_> = selected.iter().map(|w| w.gain).collect();
                let summary = summarize(&gains);
                let correlation = summarize(&selected.iter().map(|w| w.corr).collect::<Vec<_>>());
                let lag = summarize(&selected.iter().map(|w| w.lag as f64).collect::<Vec<_>>());
                let validity = if selected.is_empty() {
                    Validity::InsufficientSupport
                } else if selected.iter().any(|w| w.corr.abs() < MIN_CORRELATION) {
                    Validity::LowCorrelation
                } else if coh[i] < MIN_COHERENCE {
                    Validity::LowCoherence
                } else {
                    Validity::Valid
                };
                let gain = summary.map_or(f64::NAN, |s| s.median);
                let mut r = RecordBuilder::new(
                    pair.split("__").next().unwrap_or(pair),
                    "reference.band_lag_ls.v1",
                    ResolutionClass::Segment,
                )
                .resolution(window as u64, hop as u64, rate)
                .scalar(
                    "ref.band_gain_m2",
                    gain,
                    "ratio",
                    None,
                    propagate_confidence(
                        src.upstream_confidence,
                        if validity == Validity::Valid {
                            Confidence::High
                        } else {
                            Confidence::Low
                        },
                    ),
                    validity,
                    (validity != Validity::Valid).then(|| format!("{validity:?}")),
                    selected.len() as u64,
                );
                r.alignment_id = Some(alignment.into());
                r.channels = Some(ChannelSelector::Channel(channel as u16));
                r.freq_range = Some(Range { start: lo, end: hi });
                r.reference = anchor_gain;
                r.delta = anchor_gain
                    .filter(|g| *g != 0.0 && gain.is_finite())
                    .map(|g| Delta {
                        difference: Some(gain - g),
                        ratio: Some(gain / g),
                    });
                r.params = parameters(cfg, rate);
                r.params.as_object_mut().unwrap().extend(serde_json::json!({
                    "band_index": i,
                    "polarity": if inverted {"inverted"} else {"normal"},
                    "selected_count": selected.len(),
                    "group_count": ws.iter().filter(|w|matches!(w.reason,"supported"|"low_correlation") && (w.gain<0.0)==inverted).count(),
                    "total_windows": ws.len(),
                    "correlation": correlation.map(|s|s.median),
                    "absolute_correlation": correlation.map(|s|s.median.abs()),
                    "coherence": coh[i],
                    "mad": summary.map(|s|s.mad),
                    "lag_samples": lag.map(|s|s.median),
                    "gain_db": (gain.is_finite()&&gain!=0.0).then(||20.0*gain.abs().log10()),
                    "lower_bound_interpretation": validity==Validity::LowCorrelation || validity==Validity::LowCoherence,
                    "filter_guard_samples": (rate as f64/lo).ceil() as usize,
                    "window_reasons": ws.iter().map(|w|w.reason).collect::<Vec<_>>()
                }).as_object().unwrap().clone());
                if Some(i) == anchor {
                    let mut anchor_record = r.clone();
                    anchor_record.feature_id = "ref.gain_400hz".into();
                    out.records.push(anchor_record);
                }
                out.records.push(r);
            }
        }
        // 帯域ごとの端点除外で窓数が違うため、欠損を NaN / support=0 で埋める。
        let rows = all.iter().map(Vec::len).max().unwrap_or(0);
        for (name, field) in [
            ("lag", 0),
            ("gain", 1),
            ("correlation", 2),
            ("support", 3),
            ("start_samples", 4),
        ] {
            let values: Vec<Vec<f64>> = (0..rows)
                .map(|row| {
                    all.iter()
                        .map(|ws| {
                            ws.get(row)
                                .map_or(if field == 3 { 0.0 } else { f64::NAN }, |w| match field {
                                    0 => w.lag as f64,
                                    1 => w.gain,
                                    2 => w.corr,
                                    3 => w.support as f64,
                                    _ => w.start as f64,
                                })
                        })
                        .collect()
                })
                .collect();
            out.tensors.push(matrix_to_tensor(
                format!("intermediate/{pair}/channel_{channel}/band_{name}"),
                &values,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_dataset::types::Value;
    use al_locate::dsp::bandpass_many;

    fn noise(n: usize, mut seed: u64) -> Vec<f64> {
        (0..n)
            .map(|_| {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                (seed >> 33) as f64 / (1u64 << 30) as f64 - 1.0
            })
            .collect()
    }
    fn scalar(r: &al_dataset::types::FeatureRecord) -> f64 {
        match r.value {
            Value::Scalar(v) => v,
            _ => panic!("scalar expected"),
        }
    }
    fn run(x: &[Vec<f64>], y: &[Vec<f64>], rate: u32) -> ReferenceResult {
        let aligned = Aligned {
            src: x.iter().map(Vec::as_slice).collect(),
            cap: y.iter().map(Vec::as_slice).collect(),
            rate,
            upstream_confidence: Confidence::High,
        };
        let mut out = ReferenceResult::default();
        analyze(
            "source__capture",
            "test-alignment",
            &aligned,
            &ReferenceConfig::default(),
            &mut out,
        );
        out
    }

    #[test]
    fn independent_band_delays_and_channel_polarity_are_recovered() {
        // 8kHz と 48kHz の両方で 40ms の探索幅を実レートから作る。
        for rate in [8_000, 48_000] {
            let n = rate as usize;
            let components =
                bandpass_many(&noise(n, 12), rate, &[(300.0, 500.0), (1800.0, 2800.0)]);
            let x: Vec<_> = (0..n)
                .map(|i| components[0][i] + components[1][i])
                .collect();
            let delays = [
                (0.025 * rate as f64) as usize,
                (0.015 * rate as f64) as usize,
            ];
            let y: Vec<_> = (0..n)
                .map(|i| {
                    components
                        .iter()
                        .zip(delays)
                        .zip([0.5, 1.4])
                        .map(|((c, d), g)| if i >= d { c[i - d] * g } else { 0.0 })
                        .sum()
                })
                .collect();
            let out = run(
                &[x.clone(), x],
                &[y.clone(), y.iter().map(|v| -v).collect()],
                rate,
            );
            for (channel, polarity, sign) in [(0, "normal", 1.0), (1, "inverted", -1.0)] {
                for (frequency, delay, gain) in [(400.0, delays[0], 0.5), (2200.0, delays[1], 1.4)]
                {
                    let r = out
                        .records
                        .iter()
                        .find(|r| {
                            r.feature_id == "ref.band_gain_m2"
                                && r.channels == Some(ChannelSelector::Channel(channel))
                                && r.params["polarity"] == polarity
                                && r.freq_range
                                    .is_some_and(|b| b.start <= frequency && frequency < b.end)
                        })
                        .unwrap();
                    assert!(
                        (scalar(r) - sign * gain).abs() < 0.06,
                        "{rate} {frequency}: {r:?}"
                    );
                    assert!(
                        (r.params["lag_samples"].as_f64().unwrap() - delay as f64).abs() <= 2.0,
                        "{r:?}"
                    );
                    assert_eq!(r.params["max_lag_samples"], (rate as f64 * 0.04) as usize);
                    assert!(r.params["correlation"].as_f64().unwrap() * sign > 0.95);
                    assert_eq!(r.alignment_id.as_deref(), Some("test-alignment"));
                }
            }
            assert!(out.tensors.iter().any(|t| t.path.ends_with("band_lag")));
        }
    }

    #[test]
    fn top_half_is_selected_per_polarity_with_time_ties_and_true_median() {
        let ws: Vec<_> = [
            (3, 8.0, 0.9),
            (1, 2.0, 0.9),
            (2, 4.0, 0.8),
            (4, 100.0, 0.1),
            (5, -3.0, -0.95),
            (6, -5.0, -0.8),
        ]
        .into_iter()
        .map(|(start, gain, corr)| Window {
            start,
            lag: 0,
            gain,
            corr,
            support: 100,
            reason: "supported",
        })
        .collect();
        let normal = select(&ws, false);
        assert_eq!(
            normal.iter().map(|w| w.start).collect::<Vec<_>>(),
            vec![1, 3]
        );
        assert_eq!(
            summarize(&normal.iter().map(|w| w.gain).collect::<Vec<_>>())
                .unwrap()
                .median,
            5.0
        );
        let inverted = select(&ws, true);
        assert_eq!(inverted.len(), 1);
        assert_eq!(inverted[0].gain, -3.0);
    }

    #[test]
    fn silence_short_input_and_search_edge_are_explicit() {
        let empty = run(&[vec![0.0; 80]], &[vec![0.0; 80]], 8000);
        assert!(
            empty
                .records
                .iter()
                .all(|r| r.validity == Validity::InsufficientSupport && r.support_count == 0)
        );
        let silent = windows(&vec![0.0; 2000], &vec![0.0; 2000], 800, 400, 320, 20);
        assert!(silent.iter().all(|w| w.reason == "silence"));
        let x = noise(8000, 1);
        let y: Vec<_> = (0..8000)
            .map(|i| if i >= 320 { x[i - 320] } else { 0.0 })
            .collect();
        let edge = windows(&x, &y, 800, 400, 320, 20);
        assert!(edge.iter().all(|w| w.reason == "search_edge"));
        assert!(select(&edge, false).is_empty());
        let tied = windows(&vec![1.0; 2000], &vec![1.0; 2000], 800, 400, 320, 20);
        assert!(tied.iter().all(|w| w.lag == 0));
    }

    #[test]
    fn unrelated_loud_capture_never_becomes_supported_frequency_response() {
        let x = noise(16000, 8);
        let y: Vec<_> = noise(16000, 9).iter().map(|v| v * 100.0).collect();
        let out = run(&[x], &[y], 8000);
        assert!(
            out.records.iter().all(|r| r.validity != Validity::Valid),
            "valid bands: {:?}",
            out.records
                .iter()
                .filter(|r| r.validity == Validity::Valid)
                .map(|r| (r.freq_range, r.params["coherence"].clone()))
                .collect::<Vec<_>>()
        );
        assert!(out.records.iter().all(|r| r.reference.is_none()));
        assert!(
            out.records
                .iter()
                .filter(|r| r.support_count > 0)
                .all(|r| r.params["lower_bound_interpretation"] == true)
        );
    }
}
