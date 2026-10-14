//! 符号付きの窓 NCC と GCC-PHAT。候補の順位付け・validity は利用側が所有する。

use crate::{Simd, WithSimd, dispatch, fft::RealFft};

/// `signal[lag..lag + template.len()]` との NCC を返す。
/// 空素材・短い signal は空、ゼロエネルギー窓は 0。極性反転の符号を保持する。
pub fn ncc(template: &[f64], signal: &[f64], max_lag: usize) -> Vec<f64> {
    dispatch(Correlation {
        template,
        signal,
        max_lag,
        phat: false,
    })
}

/// FFT を用いた NCC。窓 NCC と同じ正規化・lag 方向を持つ。
pub use ncc as fft_ncc;

/// `conj(template_fft) * signal_fft` を振幅で白色化する。
/// ゼロ bin は 0、lag は `0..=min(max_lag, signal.len()-template.len())`。
pub fn gcc_phat(template: &[f64], signal: &[f64], max_lag: usize) -> Vec<f64> {
    dispatch(Correlation {
        template,
        signal,
        max_lag,
        phat: true,
    })
}

struct Correlation<'a> {
    template: &'a [f64],
    signal: &'a [f64],
    max_lag: usize,
    phat: bool,
}

impl WithSimd for Correlation<'_> {
    type Output = Vec<f64>;
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let Self {
            template,
            signal,
            max_lag,
            phat,
        } = self;
        let len = template.len();
        if len == 0 || signal.len() < len {
            return Vec::new();
        }
        let max_lag = max_lag.min(signal.len() - len);
        // 同じ標本順で加算し、レーン数に依存する reduction 順序を避ける。
        // PHAT は窓エネルギーを使わず、非ゼロ cross-spectrum のみで正規化する。
        let template_energy: f64 = if phat {
            1.0
        } else {
            let mut values = vec![0.0; len];
            square(simd, template, &mut values);
            values.iter().sum()
        };
        if template_energy == 0.0 {
            return vec![0.0; max_lag + 1];
        }
        let n = (signal.len() + len - 1).next_power_of_two();
        let mut plan = RealFft::new(n);
        let mut real = vec![0.0; n];
        let bins = n / 2 + 1;
        let mut a_re = vec![0.0; bins];
        let mut a_im = vec![0.0; bins];
        let mut b_re = vec![0.0; bins];
        let mut b_im = vec![0.0; bins];
        real[..len].copy_from_slice(template);
        plan.forward_with_simd(simd, &real, &mut a_re, &mut a_im);
        real.fill(0.0);
        real[..signal.len()].copy_from_slice(signal);
        plan.forward_with_simd(simd, &real, &mut b_re, &mut b_im);
        cross_spectrum(simd, &mut a_re, &mut a_im, &b_re, &b_im, phat);
        plan.inverse_with_simd(simd, &a_re, &a_im, &mut real);
        real.truncate(max_lag + 1);
        if !phat {
            let mut squares = vec![0.0; signal.len()];
            square(simd, signal, &mut squares);
            let mut cumulative = vec![0.0; signal.len() + 1];
            for (i, &value) in squares.iter().enumerate() {
                cumulative[i + 1] = cumulative[i] + value;
            }
            let norms: Vec<_> = (0..=max_lag)
                .map(|lag| (cumulative[lag + len] - cumulative[lag]).max(0.0))
                .collect();
            normalize(simd, &mut real, &norms, template_energy.sqrt());
        }
        real
    }
}

#[inline(always)]
fn square<S: Simd>(simd: S, input: &[f64], output: &mut [f64]) {
    let (iv, it) = S::as_simd_f64s(input);
    let (ov, ot) = S::as_mut_simd_f64s(output);
    for (&v, out) in iv.iter().zip(ov) {
        *out = simd.mul_f64s(v, v);
    }
    for (&v, out) in it.iter().zip(ot) {
        *out = v * v;
    }
}

// conj(a)·b = (ar·br + ai·bi, ar·bi − ai·br) の実部演算で書き換える。
#[inline(always)]
fn cross_spectrum<S: Simd>(
    simd: S,
    a_re: &mut [f64],
    a_im: &mut [f64],
    b_re: &[f64],
    b_im: &[f64],
    phat: bool,
) {
    let (arv, art) = S::as_mut_simd_f64s(a_re);
    let (aiv, ait) = S::as_mut_simd_f64s(a_im);
    let (brv, brt) = S::as_simd_f64s(b_re);
    let (biv, bit) = S::as_simd_f64s(b_im);
    for (((ar, ai), &br), &bi) in arv.iter_mut().zip(aiv).zip(brv).zip(biv) {
        // mul_c64s(conj(a), b) と同じ FMA 融合形。
        let re = simd.mul_add_f64s(*ar, br, simd.mul_f64s(*ai, bi));
        let im = simd.mul_add_f64s(*ar, bi, simd.mul_f64s(simd.neg_f64s(*ai), br));
        *ar = re;
        *ai = im;
    }
    for (((ar, ai), &br), &bi) in art.iter_mut().zip(ait).zip(brt).zip(bit) {
        let re = *ar * br + *ai * bi;
        let im = *ar * bi - *ai * br;
        *ar = re;
        *ai = im;
    }
    if phat {
        // bin ごとのエネルギーで re/im 両成分を同一規則で白色化する。
        let mut energy = vec![0.0; a_re.len()];
        let (arv, art) = S::as_simd_f64s(a_re);
        let (aiv, ait) = S::as_simd_f64s(a_im);
        let (ev, et) = S::as_mut_simd_f64s(&mut energy);
        for ((&ar, &ai), e) in arv.iter().zip(aiv).zip(ev) {
            *e = simd.add_f64s(simd.mul_f64s(ar, ar), simd.mul_f64s(ai, ai));
        }
        for ((&ar, &ai), e) in art.iter().zip(ait).zip(et) {
            *e = ar * ar + ai * ai;
        }
        normalize(simd, a_re, &energy, 1.0);
        normalize(simd, a_im, &energy, 1.0);
    }
}

#[inline(always)]
fn normalize<S: Simd>(simd: S, values: &mut [f64], energy: &[f64], template_norm: f64) {
    let zero = simd.splat_f64s(0.0);
    let one = simd.splat_f64s(1.0);
    let tn = simd.splat_f64s(template_norm);
    let (vv, vt) = S::as_mut_simd_f64s(values);
    let (ev, et) = S::as_simd_f64s(energy);
    for (v, &e) in vv.iter_mut().zip(ev) {
        let positive = simd.greater_than_f64s(e, zero);
        let denominator = simd.select_f64s(positive, simd.mul_f64s(tn, simd.sqrt_f64s(e)), one);
        *v = simd.select_f64s(positive, simd.div_f64s(*v, denominator), zero);
    }
    for (v, &e) in vt.iter_mut().zip(et) {
        *v = if e > 0.0 {
            *v / (template_norm * e.sqrt())
        } else {
            0.0
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_and_dispatch_match_for_signed_correlations_and_tails() {
        for len in [1, 2, 3, 7, 16, 33, 127] {
            let signal: Vec<_> = (0..len * 3 + 1).map(|i| (i as f64 * 1.713).sin()).collect();
            for polarity in [-1.0, 1.0] {
                let template: Vec<_> = signal[len..len * 2].iter().map(|x| x * polarity).collect();
                for phat in [false, true] {
                    let kernel = || Correlation {
                        template: &template,
                        signal: &signal,
                        max_lag: usize::MAX,
                        phat,
                    };
                    let native = dispatch(kernel());
                    let scalar = pulp::Scalar.vectorize(kernel());
                    for (a, b) in native.iter().zip(scalar) {
                        assert!((a - b).abs() < 1e-10, "len={len}, phat={phat}: {a} != {b}");
                    }
                }
            }
        }
    }

    /// 決定的ノイズ(al-locate 計測と同じ LCG)。
    fn noise(len: usize, seed: u64) -> Vec<f64> {
        let mut s = seed;
        (0..len)
            .map(|_| {
                s = s
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ((s >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0
            })
            .collect()
    }

    // T1 characterization(TASK-034Fo9cS08WsbNrLoq6t6a / test-spec
    // `locate-argmax-tie-eps-calibration.md` Rule B): 実運用 FFT
    // サイズでの scalar↔dispatch 偏差上限を実測する。本番 NCC は
    // `(signal.len()+template.len()-1).next_power_of_two()` の
    // RealFFT(golden 規模 ~64k-point)で、既存の小入力アサーション
    // (上記、信号 ≤ 382)はその偏差を直接カバーしない。
    #[test]
    fn dispatch_deviation_at_production_fft_size() {
        // (signal_len, template_len, embed_at): n = 65536 と 32768。
        for (signal_len, template_len, embed_at) in [
            (40_000usize, 4_096usize, 9_600usize),
            (24_576, 8_192, 12_000),
        ] {
            let n = (signal_len + template_len - 1).next_power_of_two();
            let template = noise(template_len, 41);
            let mut signal = noise(signal_len, 43);
            for (i, &v) in template.iter().enumerate() {
                signal[embed_at + i] += v;
            }
            for phat in [false, true] {
                let kernel = || Correlation {
                    template: &template,
                    signal: &signal,
                    max_lag: usize::MAX,
                    phat,
                };
                let native = dispatch(kernel());
                let scalar = pulp::Scalar.vectorize(kernel());
                assert_eq!(native.len(), scalar.len());
                let mut max_abs = 0.0f64;
                let mut max_rel = 0.0f64;
                let mut top_dev = 0.0f64;
                // dispatch 側の argmax 近傍(上位値)での偏差も記録する
                // — 同点判定は最大値近傍の偏差だけが反転を起こし得る。
                let (argmax, m) = native
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
                    .map(|(i, v)| (i, v.abs()))
                    .unwrap();
                for (i, (&a, &b)) in native.iter().zip(&scalar).enumerate() {
                    let d = (a - b).abs();
                    max_abs = max_abs.max(d);
                    max_rel = max_rel.max(d / a.abs().max(b.abs()).max(1.0));
                    if a.abs() >= m - 1e-3 {
                        top_dev = top_dev.max(d);
                    }
                    // 採用 eps(TIE_REL_EPS=1e-11)より厳密に小さいこと
                    // を dispatch 不変性の要求として pin する
                    // (計測値は ≤ ~5e-18)。
                    assert!(
                        d < 1e-12,
                        "n={n} phat={phat} lag={i}: dispatch {a} vs scalar {b}"
                    );
                }
                eprintln!(
                    "correlation dispatch-dev: n={n} phat={phat} \
                     max_abs={max_abs:.3e} max_rel={max_rel:.3e} \
                     argmax={argmax} M={m:.6e} dev_within_1e-3_of_M={top_dev:.3e}"
                );
            }
        }
    }
}
