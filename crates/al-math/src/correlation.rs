//! 符号付きの窓 NCC と GCC-PHAT。候補の順位付け・validity は利用側が所有する。

use crate::{
    Simd, WithSimd, dispatch,
    fft::{RealFftPlan, RealFftWorkspace},
};

/// 固定入力形状の不変相関 plan。NCC/PHAT は実行時に選ぶ。
pub struct CorrelationPlan {
    template_len: usize,
    signal_len: usize,
    fft: Option<RealFftPlan>,
    n: usize,
}

/// 相関の排他的作業領域。すべての確保は constructor が所有する。
pub struct CorrelationWorkspace {
    template_len: usize,
    signal_len: usize,
    fft: Option<RealFftWorkspace>,
    real: Vec<f64>,
    a_re: Vec<f64>,
    a_im: Vec<f64>,
    b_re: Vec<f64>,
    b_im: Vec<f64>,
    template_squares: Vec<f64>,
    signal_squares: Vec<f64>,
    cumulative: Vec<f64>,
    norms: Vec<f64>,
    phat_energy: Vec<f64>,
}

impl CorrelationPlan {
    /// 空素材/短い signal は演算不要 shape。長さの算術 overflow は panic する。
    pub fn new(template_len: usize, signal_len: usize) -> Self {
        let n = if template_len == 0 || signal_len < template_len {
            0
        } else {
            signal_len
                .checked_add(template_len - 1)
                .and_then(usize::checked_next_power_of_two)
                .expect("correlation FFT length overflow")
        };
        Self {
            template_len,
            signal_len,
            fft: (n != 0).then(|| RealFftPlan::new(n)),
            n,
        }
    }

    /// NCC curve を workspace から借用する。shape 不一致は演算前に panic する。
    /// 借用結果を保持したまま workspace を再使用できない。
    ///
    /// ```compile_fail
    /// use al_math::correlation::{CorrelationPlan, CorrelationWorkspace};
    /// let plan = CorrelationPlan::new(2, 4);
    /// let mut ws = CorrelationWorkspace::new(&plan);
    /// let first = plan.ncc(&mut ws, &[1.0; 2], &[1.0; 4], 2);
    /// let second = plan.ncc(&mut ws, &[1.0; 2], &[1.0; 4], 2);
    /// assert_eq!(first, second);
    /// ```
    pub fn ncc<'w>(
        &self,
        workspace: &'w mut CorrelationWorkspace,
        template: &[f64],
        signal: &[f64],
        max_lag: usize,
    ) -> &'w [f64] {
        self.run(workspace, template, signal, max_lag, false)
    }

    /// 同じ FFT plan で PHAT curve を生成する。ゼロ cross bin は 0。
    pub fn gcc_phat<'w>(
        &self,
        workspace: &'w mut CorrelationWorkspace,
        template: &[f64],
        signal: &[f64],
        max_lag: usize,
    ) -> &'w [f64] {
        self.run(workspace, template, signal, max_lag, true)
    }

    fn run<'w>(
        &self,
        workspace: &'w mut CorrelationWorkspace,
        template: &[f64],
        signal: &[f64],
        max_lag: usize,
        phat: bool,
    ) -> &'w [f64] {
        assert_eq!(
            template.len(),
            self.template_len,
            "correlation template shape"
        );
        assert_eq!(signal.len(), self.signal_len, "correlation signal shape");
        assert_eq!(
            (workspace.template_len, workspace.signal_len),
            (self.template_len, self.signal_len),
            "correlation workspace shape"
        );
        if self.n == 0 {
            return &workspace.real[..0];
        }
        dispatch(Correlation {
            plan: self,
            workspace,
            template,
            signal,
            max_lag,
            phat,
        })
    }
}

impl CorrelationWorkspace {
    /// shape ごとの最大領域を確保し、実行時の resize を不要にする。
    pub fn new(plan: &CorrelationPlan) -> Self {
        let bins = if plan.n == 0 { 0 } else { plan.n / 2 + 1 };
        let len = if plan.n == 0 { 0 } else { plan.template_len };
        let signal_len = if plan.n == 0 { 0 } else { plan.signal_len };
        let count = if plan.n == 0 { 0 } else { signal_len - len + 1 };
        Self {
            template_len: plan.template_len,
            signal_len: plan.signal_len,
            fft: plan.fft.as_ref().map(RealFftWorkspace::new),
            real: vec![0.0; plan.n],
            a_re: vec![0.0; bins],
            a_im: vec![0.0; bins],
            b_re: vec![0.0; bins],
            b_im: vec![0.0; bins],
            template_squares: vec![0.0; len],
            signal_squares: vec![0.0; signal_len],
            cumulative: vec![0.0; if plan.n == 0 { 0 } else { signal_len + 1 }],
            norms: vec![0.0; count],
            phat_energy: vec![0.0; bins],
        }
    }
}

/// `signal[lag..lag + template.len()]` との符号付き NCC。owned curve の便利な入口。
pub fn ncc(template: &[f64], signal: &[f64], max_lag: usize) -> Vec<f64> {
    let plan = CorrelationPlan::new(template.len(), signal.len());
    let mut workspace = CorrelationWorkspace::new(&plan);
    plan.ncc(&mut workspace, template, signal, max_lag).to_vec()
}

/// 窓 NCC と同じ正規化・lag 方向を持つ。
pub use ncc as fft_ncc;

/// PHAT の owned curve。lag は `0..=min(max_lag, signal.len()-template.len())`。
pub fn gcc_phat(template: &[f64], signal: &[f64], max_lag: usize) -> Vec<f64> {
    let plan = CorrelationPlan::new(template.len(), signal.len());
    let mut workspace = CorrelationWorkspace::new(&plan);
    plan.gcc_phat(&mut workspace, template, signal, max_lag)
        .to_vec()
}

struct Correlation<'p, 'w, 'i> {
    plan: &'p CorrelationPlan,
    workspace: &'w mut CorrelationWorkspace,
    template: &'i [f64],
    signal: &'i [f64],
    max_lag: usize,
    phat: bool,
}

impl<'w> WithSimd for Correlation<'_, 'w, '_> {
    type Output = &'w [f64];
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let Self {
            plan,
            workspace,
            template,
            signal,
            max_lag,
            phat,
        } = self;
        let len = template.len();
        let max_lag = max_lag.min(signal.len() - len);
        let CorrelationWorkspace {
            fft,
            real,
            a_re,
            a_im,
            b_re,
            b_im,
            template_squares,
            signal_squares,
            cumulative,
            norms,
            phat_energy,
            ..
        } = workspace;
        let fft_plan = plan.fft.as_ref().expect("nonempty shape has FFT plan");
        let fft_workspace = fft.as_mut().expect("nonempty shape has FFT workspace");
        real.fill(0.0);
        // 従来と同じ標本順の加算。PHAT は窓エネルギーを使わない。
        let template_energy = if phat {
            1.0
        } else {
            square(simd, template, template_squares);
            template_squares.iter().sum::<f64>()
        };
        if template_energy == 0.0 {
            return &real[..=max_lag];
        }
        real[..len].copy_from_slice(template);
        fft_plan.forward_with_simd(simd, fft_workspace, real, a_re, a_im);
        real.fill(0.0);
        real[..signal.len()].copy_from_slice(signal);
        fft_plan.forward_with_simd(simd, fft_workspace, real, b_re, b_im);
        cross_spectrum(simd, a_re, a_im, b_re, b_im, phat, phat_energy);
        fft_plan.inverse_with_simd(simd, fft_workspace, a_re, a_im, real);
        if !phat {
            square(simd, signal, signal_squares);
            cumulative[0] = 0.0;
            for (i, &value) in signal_squares.iter().enumerate() {
                cumulative[i + 1] = cumulative[i] + value;
            }
            for (lag, norm) in norms[..=max_lag].iter_mut().enumerate() {
                *norm = (cumulative[lag + len] - cumulative[lag]).max(0.0);
            }
            normalize(
                simd,
                &mut real[..=max_lag],
                &norms[..=max_lag],
                template_energy.sqrt(),
            );
        }
        &real[..=max_lag]
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
    energy: &mut [f64],
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
        let (arv, art) = S::as_simd_f64s(a_re);
        let (aiv, ait) = S::as_simd_f64s(a_im);
        let (ev, et) = S::as_mut_simd_f64s(energy);
        for ((&ar, &ai), e) in arv.iter().zip(aiv).zip(ev) {
            *e = simd.add_f64s(simd.mul_f64s(ar, ar), simd.mul_f64s(ai, ai));
        }
        for ((&ar, &ai), e) in art.iter().zip(ait).zip(et) {
            *e = ar * ar + ai * ai;
        }
        normalize(simd, a_re, energy, 1.0);
        normalize(simd, a_im, energy, 1.0);
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
                    let plan = CorrelationPlan::new(template.len(), signal.len());
                    let mut native_ws = CorrelationWorkspace::new(&plan);
                    let mut scalar_ws = CorrelationWorkspace::new(&plan);
                    let native = dispatch(Correlation {
                        plan: &plan,
                        workspace: &mut native_ws,
                        template: &template,
                        signal: &signal,
                        max_lag: usize::MAX,
                        phat,
                    })
                    .to_vec();
                    let scalar = pulp::Scalar
                        .vectorize(Correlation {
                            plan: &plan,
                            workspace: &mut scalar_ws,
                            template: &template,
                            signal: &signal,
                            max_lag: usize::MAX,
                            phat,
                        })
                        .to_vec();
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
                let plan = CorrelationPlan::new(template.len(), signal.len());
                let mut native_ws = CorrelationWorkspace::new(&plan);
                let mut scalar_ws = CorrelationWorkspace::new(&plan);
                let native = dispatch(Correlation {
                    plan: &plan,
                    workspace: &mut native_ws,
                    template: &template,
                    signal: &signal,
                    max_lag: usize::MAX,
                    phat,
                })
                .to_vec();
                let scalar = pulp::Scalar
                    .vectorize(Correlation {
                        plan: &plan,
                        workspace: &mut scalar_ws,
                        template: &template,
                        signal: &signal,
                        max_lag: usize::MAX,
                        phat,
                    })
                    .to_vec();
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
