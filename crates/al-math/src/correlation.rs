//! 符号付きの窓 NCC と GCC-PHAT。候補の順位付け・validity は利用側が所有する。

use crate::{Simd, WithSimd, c64, dispatch, fft::RealFft};

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
        let mut squares = vec![0.0; signal.len()];
        square(simd, signal, &mut squares);
        // 同じ標本順で加算し、レーン数に依存する reduction 順序を避ける。
        let template_energy: f64 = {
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
        let mut a = vec![c64::new(0.0, 0.0); n / 2 + 1];
        let mut b = a.clone();
        real[..len].copy_from_slice(template);
        plan.forward_with_simd(simd, &real, &mut a);
        real.fill(0.0);
        real[..signal.len()].copy_from_slice(signal);
        plan.forward_with_simd(simd, &real, &mut b);
        cross_spectrum(simd, &mut a, &b, phat);
        plan.inverse_with_simd(simd, &a, &mut real);
        real.truncate(max_lag + 1);
        if !phat {
            let mut cumulative = vec![0.0; signal.len() + 1];
            for (i, &value) in squares.iter().enumerate() {
                cumulative[i + 1] = cumulative[i] + value;
            }
            let mut norms: Vec<_> = (0..=max_lag)
                .map(|lag| (cumulative[lag + len] - cumulative[lag]).max(0.0))
                .collect();
            normalize(simd, &mut real, &mut norms, template_energy.sqrt());
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

#[inline(always)]
fn cross_spectrum<S: Simd>(simd: S, a: &mut [c64], b: &[c64], phat: bool) {
    let (av, at) = S::as_mut_simd_c64s(a);
    let (bv, bt) = S::as_simd_c64s(b);
    for (a, &b) in av.iter_mut().zip(bv) {
        *a = simd.mul_c64s(simd.conj_c64s(*a), b);
    }
    for (a, &b) in at.iter_mut().zip(bt) {
        *a = a.conj() * b;
    }
    if phat {
        // abs2 は実部・虚部の両レーンに同じ値を格納する。
        let mut magnitude = vec![c64::new(0.0, 0.0); a.len()];
        let (av, at) = S::as_simd_c64s(a);
        let (mv, mt) = S::as_mut_simd_c64s(&mut magnitude);
        for (&a, m) in av.iter().zip(mv) {
            *m = simd.abs2_c64s(a);
        }
        for (&a, m) in at.iter().zip(mt) {
            let e = a.norm_sqr();
            *m = c64::new(e, e);
        }
        let values = pulp::bytemuck::cast_slice_mut(a);
        let norms = pulp::bytemuck::cast_slice_mut(&mut magnitude);
        normalize(simd, values, norms, 1.0);
    }
}

#[inline(always)]
fn normalize<S: Simd>(simd: S, values: &mut [f64], energy: &mut [f64], template_norm: f64) {
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
}
