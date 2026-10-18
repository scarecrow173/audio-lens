//! 符号付きの窓 NCC と GCC-PHAT。候補の順位付け・validity は利用側が所有する。

use crate::{
    Simd, WithSimd, dispatch,
    fft::{RealFftPlan, RealFftWorkspace},
    vec::axpy_kernel,
};

/// plan が保証する演算範囲。`Full` は線形相関全域をカバーする FFT 長
/// `nextpow2(S+L−1)` で NCC/PHAT の両方を実行できる。`NccValid` は
/// 有効ラグ域 `0..=S−L` の NCC だけを対象に FFT 長を `nextpow2(S)`
/// へ短縮する。返却 index `k ≤ S−L` は `c[k−n]`・`c[k+n]` の
/// wraparound 項がいずれも定義域外(`k−n < −(L−1)`、`k+n ≥ S`)のため
/// `n ≥ S` で無汚染となる。`NccDirect` は同じ有効ラグ域を FFT なしの
/// 直接計算で求める。PHAT の bin 正規化は FFT 長依存なので
/// `NccValid`/`NccDirect` plan での `gcc_phat` は契約違反。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlanMode {
    Full,
    NccValid,
    NccDirect,
}

/// `NccDirect` を選ぶ形状判定。有効ラグ数 `K = S−L` が小さいほど
/// 分子の直接計算 `L·(K+1)` MAC が `n·log2(n)` の 3 回 FFT より
/// 安い。係数 8 は現環境ベンチ(direct ≈ 5 GMAC/s、FFT ≈
/// 1.8 ns·n·log2n、実形状 6 点で実測最速側と一致)で校正した
/// 保守値。`K+1 ≤ 2048` の上限は acc ベクトルを ~16KB(L1)に
/// 留め、大 K 形状でのキャッシュ劣化を構造的に排除する。
fn prefers_direct(template_len: usize, signal_len: usize, n: usize) -> bool {
    let k = (signal_len - template_len + 1) as f64;
    k <= 2048.0 && (template_len as f64 * k) <= 8.0 * (n as f64) * (n as f64).log2()
}

/// 固定入力形状の不変相関 plan。NCC/PHAT は実行時に選ぶ。
pub struct CorrelationPlan {
    template_len: usize,
    signal_len: usize,
    fft: Option<RealFftPlan>,
    n: usize,
    mode: PlanMode,
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
    /// NCC と PHAT の両方を実行できる plan。NCC しか使わない経路は
    /// [`Self::new_ncc`] の方が FFT 長を短縮できる。
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
            mode: PlanMode::Full,
        }
    }

    /// 有効ラグ域 `0..=signal_len − template_len` の NCC 専用 plan。
    /// FFT 長は `nextpow2(signal_len)` で、返却される有効ラグ値は
    /// [`Self::new`] の NCC と同一契約。有効ラグ数 `K = S−L` が
    /// 小さい形状では FFT を持たない直接計算 plan(`NccDirect`)を
    /// 選ぶ(係数はベンチ校正済み、選び損ねても出力契約は不変)。
    /// この plan で `gcc_phat` を呼ぶと panic する(PHAT の
    /// 位相正規化は FFT 長依存のため)。
    pub fn new_ncc(template_len: usize, signal_len: usize) -> Self {
        let (n, mode) = if template_len == 0 || signal_len < template_len {
            (0, PlanMode::NccValid)
        } else {
            let n = signal_len
                .checked_next_power_of_two()
                .expect("correlation FFT length overflow");
            let mode = if prefers_direct(template_len, signal_len, n) {
                PlanMode::NccDirect
            } else {
                PlanMode::NccValid
            };
            (n, mode)
        };
        Self {
            template_len,
            signal_len,
            fft: (n != 0 && mode != PlanMode::NccDirect).then(|| RealFftPlan::new(n)),
            n,
            mode,
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
    /// `new_ncc` で作った plan では panic する(短縮 FFT 長では
    /// PHAT の位相正規化結果が変わる)。
    pub fn gcc_phat<'w>(
        &self,
        workspace: &'w mut CorrelationWorkspace,
        template: &[f64],
        signal: &[f64],
        max_lag: usize,
    ) -> &'w [f64] {
        assert_eq!(
            self.mode,
            PlanMode::Full,
            "gcc_phat requires CorrelationPlan::new; new_ncc plans are NCC-only"
        );
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
    /// `NccDirect` plan は FFT バッファを持たず、`real` は有効ラグ
    /// 域 `S−L+1` の出力領域として使う。
    pub fn new(plan: &CorrelationPlan) -> Self {
        let direct = plan.mode == PlanMode::NccDirect;
        let bins = if plan.n == 0 || direct {
            0
        } else {
            plan.n / 2 + 1
        };
        let len = if plan.n == 0 { 0 } else { plan.template_len };
        let signal_len = if plan.n == 0 { 0 } else { plan.signal_len };
        let count = if plan.n == 0 { 0 } else { signal_len - len + 1 };
        let real_len = if plan.n == 0 {
            0
        } else if direct {
            count
        } else {
            plan.n
        };
        Self {
            template_len: plan.template_len,
            signal_len: plan.signal_len,
            fft: plan.fft.as_ref().map(RealFftWorkspace::new),
            real: vec![0.0; real_len],
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

/// 等長窓の両側 lag 相関の不変 plan。shape キーは窓長のみ。
///
/// `CorrelationPlan`(片側スライドの固定長正規化)とは別の演算契約:
/// lag ∈ [−max_lag, +max_lag] の各点で overlap 長 `L−|lag|` の
/// 非中心化正規化相関(overlap 区間の corr0)を返す。
pub struct OverlapCorrPlan {
    window_len: usize,
    fft: Option<RealFftPlan>,
    n: usize,
}

/// OverlapCorr の排他的作業領域。すべての確保は constructor が所有する。
pub struct OverlapCorrWorkspace {
    window_len: usize,
    fft: Option<RealFftWorkspace>,
    real: Vec<f64>,
    a_re: Vec<f64>,
    a_im: Vec<f64>,
    b_re: Vec<f64>,
    b_im: Vec<f64>,
    x_prefix: Vec<f64>,
    y_prefix: Vec<f64>,
    curve: Vec<f64>,
}

impl OverlapCorrPlan {
    /// 窓長 L を shape に取る。N = 2L−1 を 2 のべき乗へ切り上げる。
    /// 長さの算術 overflow は panic する。
    pub fn new(window_len: usize) -> Self {
        let n = if window_len == 0 {
            0
        } else {
            window_len
                .checked_mul(2)
                .and_then(|v| v.checked_sub(1))
                .and_then(usize::checked_next_power_of_two)
                .expect("overlap correlation FFT length overflow")
        };
        Self {
            window_len,
            fft: (n != 0).then(|| RealFftPlan::new(n)),
            n,
        }
    }

    /// `lag = i − max_lag` の相関値を持つ長さ `2·max_lag+1` の曲線を
    /// workspace から借用する。窓長不一致は panic。
    /// 借用結果を保持したまま workspace を再使用できない。
    ///
    /// `max_lag ≥ window_len` では `|lag| ≥ window_len` の要素が 0.0。
    pub fn curve<'w>(
        &self,
        workspace: &'w mut OverlapCorrWorkspace,
        x: &[f64],
        y: &[f64],
        max_lag: usize,
    ) -> &'w [f64] {
        assert_eq!(x.len(), self.window_len, "overlap corr x shape");
        assert_eq!(y.len(), self.window_len, "overlap corr y shape");
        assert_eq!(
            workspace.window_len, self.window_len,
            "overlap corr workspace shape"
        );
        let out_len = 2 * max_lag + 1;
        if self.n == 0 {
            workspace
                .curve
                .resize(out_len.max(workspace.curve.len()), 0.0);
            workspace.curve[..out_len].fill(0.0);
            return &workspace.curve[..out_len];
        }
        dispatch(OverlapCorr {
            plan: self,
            workspace,
            x,
            y,
            max_lag,
        })
    }
}

impl OverlapCorrWorkspace {
    /// shape ごとの最大領域を確保し、実行時の resize を不要にする。
    /// `curve` は全 lag(|lag| ≤ L−1)を収める `2L−1` で確保する。
    pub fn new(plan: &OverlapCorrPlan) -> Self {
        let bins = if plan.n == 0 { 0 } else { plan.n / 2 + 1 };
        let l = plan.window_len;
        Self {
            window_len: l,
            fft: plan.fft.as_ref().map(RealFftWorkspace::new),
            real: vec![0.0; plan.n],
            a_re: vec![0.0; bins],
            a_im: vec![0.0; bins],
            b_re: vec![0.0; bins],
            b_im: vec![0.0; bins],
            x_prefix: vec![0.0; l + 1],
            y_prefix: vec![0.0; l + 1],
            curve: vec![0.0; (2 * l).max(1) - 1],
        }
    }
}

/// 等長窓の両側 lag 相関曲線。owned curve の便利な入口。
/// `curve[i]` は `lag = i − max_lag` の値。
pub fn overlap_corr_curve(x: &[f64], y: &[f64], max_lag: usize) -> Vec<f64> {
    assert_eq!(x.len(), y.len(), "overlap corr window shapes");
    let plan = OverlapCorrPlan::new(x.len());
    let mut workspace = OverlapCorrWorkspace::new(&plan);
    plan.curve(&mut workspace, x, y, max_lag).to_vec()
}

/// `signal[lag..lag + template.len()]` との符号付き NCC。owned curve の便利な入口。
pub fn ncc(template: &[f64], signal: &[f64], max_lag: usize) -> Vec<f64> {
    let plan = CorrelationPlan::new_ncc(template.len(), signal.len());
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

struct OverlapCorr<'p, 'w, 'i> {
    plan: &'p OverlapCorrPlan,
    workspace: &'w mut OverlapCorrWorkspace,
    x: &'i [f64],
    y: &'i [f64],
    max_lag: usize,
}

impl<'w> WithSimd for OverlapCorr<'_, 'w, '_> {
    type Output = &'w [f64];
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let Self {
            plan,
            workspace,
            x,
            y,
            max_lag,
        } = self;
        let l = x.len();
        let n = plan.n;
        let out_len = 2 * max_lag + 1;
        let OverlapCorrWorkspace {
            fft,
            real,
            a_re,
            a_im,
            b_re,
            b_im,
            x_prefix,
            y_prefix,
            curve,
            ..
        } = workspace;
        let fft_plan = plan.fft.as_ref().expect("nonempty shape has FFT plan");
        let fft_workspace = fft.as_mut().expect("nonempty shape has FFT workspace");
        // `max_lag` は呼出し時の値なので、要求長をここで保証する
        // (2L−1 超の max_lag 指定でも |lag| ≥ L は 0.0 を返すだけ)。
        if curve.len() < out_len {
            curve.resize(out_len, 0.0);
        }
        // 直接法は ~3·(2·max_lag+1)·L flop(corr0 の 3 累積)、
        // FFT 法は ~15·N·log2(N) flop(3 変換+合成)。実測の交差
        // (L=128 で直接、L=512/lag=128 で FFT)に合わせた仕事量比較。
        let direct_ops = out_len.saturating_mul(l).saturating_mul(3);
        let fft_ops = 15usize
            .saturating_mul(n)
            .saturating_mul(n.trailing_zeros() as usize);
        if direct_ops < fft_ops {
            for (i, out) in curve.iter_mut().enumerate().take(out_len) {
                let lag = i as isize - max_lag as isize;
                let a = lag.unsigned_abs();
                *out = if a >= l {
                    0.0
                } else {
                    let (xs, ys) = if lag >= 0 {
                        (&x[..l - a], &y[a..])
                    } else {
                        (&x[a..], &y[..l - a])
                    };
                    crate::vec::corr0_kernel(simd, xs, ys)
                };
            }
            return &curve[..out_len];
        }
        // 分子: 線形相互相関 c[k] = Σ x[i]·y[i+k]。N ≥ 2L−1 の
        // ゼロ埋めで巡回 wraparound は混入しない。負 lag −a は
        // c[N−a] = Σ x[i]·y[i−a] に対応する。
        real.fill(0.0);
        real[..l].copy_from_slice(x);
        fft_plan.forward_with_simd(simd, fft_workspace, real, a_re, a_im);
        real.fill(0.0);
        real[..l].copy_from_slice(y);
        fft_plan.forward_with_simd(simd, fft_workspace, real, b_re, b_im);
        cross_spectrum(simd, a_re, a_im, b_re, b_im, false, &mut []);
        fft_plan.inverse_with_simd(simd, fft_workspace, a_re, a_im, real);
        // 分母: overlap 範囲のエネルギーを prefix 二乗和から得る。
        x_prefix[0] = 0.0;
        y_prefix[0] = 0.0;
        for i in 0..l {
            x_prefix[i + 1] = x_prefix[i] + x[i] * x[i];
            y_prefix[i + 1] = y_prefix[i] + y[i] * y[i];
        }
        for (i, out) in curve.iter_mut().enumerate().take(out_len) {
            let lag = i as isize - max_lag as isize;
            let a = lag.unsigned_abs();
            *out = if a >= l {
                0.0
            } else {
                let (num, ex, ey) = if lag >= 0 {
                    // x[..L−a] vs y[a..L]
                    (real[a], x_prefix[l - a], y_prefix[l] - y_prefix[a])
                } else {
                    // x[a..L] vs y[..L−a]、分子は巡回インデックス N−a
                    (real[n - a], x_prefix[l] - x_prefix[a], y_prefix[l - a])
                };
                if ex > 0.0 && ey > 0.0 {
                    num / (ex * ey).sqrt()
                } else {
                    0.0
                }
            };
        }
        &curve[..out_len]
    }
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
        if plan.mode == PlanMode::NccDirect {
            // 分子を直接計算: real[k] = Σ_i template[i]·signal[k+i]。
            // real は有効ラグ域長、signal の参照窓は i に対し K+1
            // 要素のスライドなのでキャッシュ局所性が高い。
            for (i, &ai) in template.iter().enumerate() {
                axpy_kernel(simd, &mut real[..=max_lag], ai, &signal[i..i + max_lag + 1]);
            }
        } else {
            let fft_plan = plan.fft.as_ref().expect("nonempty shape has FFT plan");
            let fft_workspace = fft.as_mut().expect("nonempty shape has FFT workspace");
            real.fill(0.0);
            real[..len].copy_from_slice(template);
            fft_plan.forward_with_simd(simd, fft_workspace, real, a_re, a_im);
            real.fill(0.0);
            real[..signal.len()].copy_from_slice(signal);
            fft_plan.forward_with_simd(simd, fft_workspace, real, b_re, b_im);
            cross_spectrum(simd, a_re, a_im, b_re, b_im, phat, phat_energy);
            fft_plan.inverse_with_simd(simd, fft_workspace, a_re, a_im, real);
        }
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

    /// reference.rs `corr_at` の逐点再現 oracle。overlap 区間の
    /// `corr0` を lag ごとに直接評価する。
    fn corr_at(x: &[f64], y: &[f64], lag: isize) -> f64 {
        let (xs, ys) = if lag >= 0 {
            (
                &x[..x.len() - (lag as usize).min(x.len())],
                &y[lag as usize..],
            )
        } else {
            (
                &x[(-lag) as usize..],
                &y[..y.len() - (-lag as usize).min(y.len())],
            )
        };
        let n = xs.len().min(ys.len());
        if n == 0 {
            return 0.0;
        }
        crate::vec::corr0(&xs[..n], &ys[..n])
    }

    #[test]
    fn overlap_corr_matches_corr_at_pointwise() {
        // 既知オフセット埋め込み + 複数窓サイズ・max_lag で網羅。
        for (l, max_lag, shift) in [
            (64usize, 16usize, 5isize),
            (256, 40, -17),
            (1024, 96, 200),
            (300, 130, 41), // 非2冪の窓長
        ] {
            let x = noise(l, 7);
            let mut y = noise(l, 11);
            let a = shift.unsigned_abs().min(l);
            // y に x のシフトコピーを混ぜて確定的なピークを作る。
            for i in 0..l - a {
                if shift >= 0 {
                    y[i + a] = 0.8 * x[i] + 0.2 * y[i + a];
                } else {
                    y[i] = 0.8 * x[i + a] + 0.2 * y[i];
                }
            }
            let curve = overlap_corr_curve(&x, &y, max_lag);
            assert_eq!(curve.len(), 2 * max_lag + 1);
            let oracle: Vec<f64> = (-(max_lag as isize)..=max_lag as isize)
                .map(|lag| corr_at(&x, &y, lag))
                .collect();
            let peak_new = curve
                .iter()
                .enumerate()
                .max_by(|p, q| p.1.total_cmp(q.1))
                .map(|(i, _)| i as isize - max_lag as isize)
                .unwrap();
            let peak_old = oracle
                .iter()
                .enumerate()
                .max_by(|p, q| p.1.total_cmp(q.1))
                .map(|(i, _)| i as isize - max_lag as isize)
                .unwrap();
            assert_eq!(peak_new, peak_old, "l={l} peak lag mismatch");
            for (i, (&new, &old)) in curve.iter().zip(&oracle).enumerate() {
                let rel = (new - old).abs() / old.abs().max(1e-12);
                assert!(
                    rel < 1e-9 || (new - old).abs() < 1e-12,
                    "l={l} lag={}: {new} vs {old}",
                    i as isize - max_lag as isize
                );
            }
        }
    }

    #[test]
    fn overlap_corr_guards_silence_and_out_of_range_lags() {
        let zero = vec![0.0; 128];
        let x = noise(128, 3);
        // 片側無音 → 全 lag が 0.0
        let curve = overlap_corr_curve(&x, &zero, 30);
        assert!(curve.iter().all(|&v| v == 0.0));
        // max_lag ≥ L: |lag| ≥ L の要素は 0.0 で panic しない
        let curve = overlap_corr_curve(&x, &x, 200);
        assert_eq!(curve.len(), 401);
        for (i, &v) in curve.iter().enumerate() {
            let lag = i as isize - 200;
            if lag.unsigned_abs() >= 128 {
                assert_eq!(v, 0.0, "lag {lag} must be 0.0");
            }
        }
        // 自己相関の lag=0 は 1.0
        assert!((curve[200] - 1.0).abs() < 1e-12);
        // 空窓は全 0
        let curve = overlap_corr_curve(&[], &[], 4);
        assert_eq!(curve, vec![0.0; 9]);
    }

    #[test]
    fn overlap_corr_scalar_and_dispatch_match() {
        for l in [1usize, 33, 100, 511] {
            let x = noise(l, 5);
            let y = noise(l, 9);
            let max_lag = l.min(40);
            let plan = OverlapCorrPlan::new(l);
            let mut native_ws = OverlapCorrWorkspace::new(&plan);
            let mut scalar_ws = OverlapCorrWorkspace::new(&plan);
            let native = dispatch(OverlapCorr {
                plan: &plan,
                workspace: &mut native_ws,
                x: &x,
                y: &y,
                max_lag,
            })
            .to_vec();
            let scalar = pulp::Scalar
                .vectorize(OverlapCorr {
                    plan: &plan,
                    workspace: &mut scalar_ws,
                    x: &x,
                    y: &y,
                    max_lag,
                })
                .to_vec();
            for (i, (&a, &b)) in native.iter().zip(&scalar).enumerate() {
                assert!(
                    (a - b).abs() < 1e-12,
                    "l={l} idx={i}: dispatch {a} vs scalar {b}"
                );
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

    /// NCC-valid plan(FFT `nextpow2(S)`)は返却される全有効ラグで
    /// Full plan(`nextpow2(S+L−1)`)と同一契約の値を返す。
    /// wraparound 汚染は index `> S−L` にのみ発生する。
    #[test]
    fn ncc_valid_plan_matches_full_plan_on_all_returned_lags() {
        for (template_len, signal_len, max_lag) in [
            (1usize, 8usize, usize::MAX),
            (4, 4, usize::MAX), // 等長(K=1)
            (4, 5, usize::MAX),
            (3, 33, usize::MAX),
            (33, 64, usize::MAX),
            (64, 100, usize::MAX), // 16384 未満でも FFT 半減する形状
            (100, 1000, usize::MAX),
            (512, 4096, usize::MAX),
            (4410, 7938, usize::MAX), // 44.1k band_gain 形状(16K→8K)
            (4800, 8640, usize::MAX), // 48k band_gain 形状(縮小なし)
            (96_000, 100_800, usize::MAX),
            (16, 64, 10), // max_lag < S−L
            (16, 64, 0),  // lag 0 のみ
        ] {
            let template = noise(template_len, 11);
            let signal = noise(signal_len, 13);
            let full = CorrelationPlan::new(template_len, signal_len);
            let valid = CorrelationPlan::new_ncc(template_len, signal_len);
            let mut full_ws = CorrelationWorkspace::new(&full);
            let mut valid_ws = CorrelationWorkspace::new(&valid);
            let expected = full.ncc(&mut full_ws, &template, &signal, max_lag).to_vec();
            let actual = valid
                .ncc(&mut valid_ws, &template, &signal, max_lag)
                .to_vec();
            assert_eq!(
                expected.len(),
                actual.len(),
                "shape=({template_len},{signal_len})"
            );
            let argmax_full = expected
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
                .map(|(i, _)| i);
            let argmax_valid = actual
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
                .map(|(i, _)| i);
            for (i, (&a, &b)) in expected.iter().zip(&actual).enumerate() {
                let d = (a - b).abs();
                assert!(
                    d < 1e-10 || d / a.abs().max(1.0) < 1e-10,
                    "shape=({template_len},{signal_len}) lag={i}: full {a} vs valid {b}"
                );
            }
            assert_eq!(
                argmax_full, argmax_valid,
                "shape=({template_len},{signal_len}) peak"
            );
        }
    }

    /// valid plan の NCC も scalar↔runtime-dispatch で等しい。
    #[test]
    fn ncc_valid_plan_scalar_and_dispatch_match() {
        for (template_len, signal_len) in [(4usize, 5usize), (33, 64), (4410, 7938)] {
            let template = noise(template_len, 17);
            let signal = noise(signal_len, 19);
            let plan = CorrelationPlan::new_ncc(template_len, signal_len);
            let mut native_ws = CorrelationWorkspace::new(&plan);
            let mut scalar_ws = CorrelationWorkspace::new(&plan);
            let native = dispatch(Correlation {
                plan: &plan,
                workspace: &mut native_ws,
                template: &template,
                signal: &signal,
                max_lag: usize::MAX,
                phat: false,
            })
            .to_vec();
            let scalar = pulp::Scalar
                .vectorize(Correlation {
                    plan: &plan,
                    workspace: &mut scalar_ws,
                    template: &template,
                    signal: &signal,
                    max_lag: usize::MAX,
                    phat: false,
                })
                .to_vec();
            assert_eq!(native.len(), scalar.len());
            for (i, (&a, &b)) in native.iter().zip(&scalar).enumerate() {
                assert!(
                    (a - b).abs() < 1e-10,
                    "shape=({template_len},{signal_len}) lag={i}: dispatch {a} vs scalar {b}"
                );
            }
        }
    }

    /// 形状ベンチの 6 代表形状で、コストモデルが実測最速側を選ぶ
    /// ことを固定する。閾値回帰で FFT/direct が逆転しないよう
    /// shape→mode を pin する。
    #[test]
    fn ncc_plan_mode_pins_benchmarked_shapes() {
        for (template_len, signal_len, expected) in [
            (2_205_000usize, 2_205_236usize, PlanMode::NccDirect), // band_ncc/full_ncc
            (551_250, 551_309, PlanMode::NccDirect),               // coarse_ncc
            (4_410, 7_938, PlanMode::NccValid),                    // band_gain window
            (4_410, 13_230, PlanMode::NccValid),                   // narrow anchor 小
            (4_410, 66_150, PlanMode::NccValid),                   // narrow anchor 中
            (800, 1_438, PlanMode::NccValid),                      // tiny 8kHz
        ] {
            let plan = CorrelationPlan::new_ncc(template_len, signal_len);
            assert_eq!(plan.mode, expected, "shape=({template_len},{signal_len})");
            assert_eq!(
                plan.fft.is_none(),
                expected == PlanMode::NccDirect,
                "shape=({template_len},{signal_len})"
            );
        }
    }

    /// direct plan は FFT plan/バッファを持たず、`real` は有効ラグ
    /// 域長の出力領域になる。共有の正規化バッファのみ残る。
    #[test]
    fn ncc_direct_plan_skips_fft_buffers() {
        let plan = CorrelationPlan::new_ncc(256, 300);
        assert_eq!(plan.mode, PlanMode::NccDirect);
        assert!(plan.fft.is_none());
        let ws = CorrelationWorkspace::new(&plan);
        assert!(ws.fft.is_none());
        assert!(ws.a_re.is_empty() && ws.a_im.is_empty());
        assert!(ws.b_re.is_empty() && ws.b_im.is_empty());
        assert!(ws.phat_energy.is_empty());
        assert_eq!(ws.real.len(), 300 - 256 + 1);
        assert_eq!(ws.template_squares.len(), 256);
        assert_eq!(ws.signal_squares.len(), 300);
        assert_eq!(ws.cumulative.len(), 301);
        assert_eq!(ws.norms.len(), 45);
    }

    /// direct plan の NCC は Full FFT plan と有効ラグ全域で一致し、
    /// argmax・max_lag クランプ・戻り範囲も同一。
    #[test]
    fn ncc_direct_matches_full_plan_on_all_returned_lags() {
        for (template_len, signal_len, max_lag) in [
            (4usize, 4usize, usize::MAX), // 等長(K=0)
            (1, 8, usize::MAX),
            (3, 33, usize::MAX),
            (33, 64, usize::MAX),
            (64, 100, usize::MAX),
            (256, 300, usize::MAX),
            (16, 64, 10), // max_lag < S−L
            (16, 64, 0),  // lag 0 のみ
        ] {
            let template = noise(template_len, 11);
            let signal = noise(signal_len, 13);
            let full = CorrelationPlan::new(template_len, signal_len);
            let direct = CorrelationPlan::new_ncc(template_len, signal_len);
            assert_eq!(
                direct.mode,
                PlanMode::NccDirect,
                "shape=({template_len},{signal_len})"
            );
            let mut full_ws = CorrelationWorkspace::new(&full);
            let mut direct_ws = CorrelationWorkspace::new(&direct);
            let expected = full.ncc(&mut full_ws, &template, &signal, max_lag).to_vec();
            let actual = direct
                .ncc(&mut direct_ws, &template, &signal, max_lag)
                .to_vec();
            assert_eq!(
                expected.len(),
                actual.len(),
                "shape=({template_len},{signal_len})"
            );
            let argmax = |c: &[f64]| {
                c.iter()
                    .enumerate()
                    .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
                    .map(|(i, _)| i)
            };
            for (i, (&a, &b)) in expected.iter().zip(&actual).enumerate() {
                let d = (a - b).abs();
                assert!(
                    d < 1e-10 || d / a.abs().max(1.0) < 1e-10,
                    "shape=({template_len},{signal_len}) lag={i}: full {a} vs direct {b}"
                );
            }
            assert_eq!(
                argmax(&expected),
                argmax(&actual),
                "shape=({template_len},{signal_len}) peak"
            );
        }
    }

    /// direct 経路も scalar↔runtime-dispatch で一致する。
    #[test]
    fn ncc_direct_scalar_and_dispatch_match() {
        for (template_len, signal_len) in [(4usize, 5usize), (33, 64), (256, 300)] {
            let template = noise(template_len, 17);
            let signal = noise(signal_len, 19);
            let plan = CorrelationPlan::new_ncc(template_len, signal_len);
            assert_eq!(plan.mode, PlanMode::NccDirect);
            let mut native_ws = CorrelationWorkspace::new(&plan);
            let mut scalar_ws = CorrelationWorkspace::new(&plan);
            let native = dispatch(Correlation {
                plan: &plan,
                workspace: &mut native_ws,
                template: &template,
                signal: &signal,
                max_lag: usize::MAX,
                phat: false,
            })
            .to_vec();
            let scalar = pulp::Scalar
                .vectorize(Correlation {
                    plan: &plan,
                    workspace: &mut scalar_ws,
                    template: &template,
                    signal: &signal,
                    max_lag: usize::MAX,
                    phat: false,
                })
                .to_vec();
            assert_eq!(native.len(), scalar.len());
            for (i, (&a, &b)) in native.iter().zip(&scalar).enumerate() {
                assert!(
                    (a - b).abs() < 1e-10,
                    "shape=({template_len},{signal_len}) lag={i}: dispatch {a} vs scalar {b}"
                );
            }
        }
    }

    /// direct plan でも Full plan と同じエッジ契約(ゼロ
    /// テンプレートの全 0 返却、無音窓の 0、max_lag クランプ、
    /// template > signal の空返却)が成り立つ。
    #[test]
    fn ncc_direct_preserves_edge_contracts() {
        let plan = CorrelationPlan::new_ncc(4, 8);
        assert_eq!(plan.mode, PlanMode::NccDirect);
        let mut ws = CorrelationWorkspace::new(&plan);
        // ゼロテンプレート → 全ラグ 0(workspace 再利用の stale
        // を残さないよう非ゼロ入力の後に検査する)。
        let _ = plan.ncc(&mut ws, &noise(4, 37), &noise(8, 41), usize::MAX);
        let curve = plan.ncc(&mut ws, &[0.0; 4], &noise(8, 43), usize::MAX);
        assert_eq!(curve.len(), 5);
        assert!(curve.iter().all(|&v| v == 0.0));
        // 無音窓 → そのラグの出力は 0
        let mut signal = noise(8, 29);
        signal[2..6].fill(0.0);
        let curve = plan.ncc(&mut ws, &noise(4, 31), &signal, usize::MAX);
        assert_eq!(curve.len(), 5);
        assert_eq!(curve[2], 0.0);
        // template > signal → 空
        let empty = CorrelationPlan::new_ncc(8, 4);
        let mut empty_ws = CorrelationWorkspace::new(&empty);
        let curve = empty.ncc(&mut empty_ws, &[0.0; 8], &[0.0; 4], usize::MAX);
        assert!(curve.is_empty());
        // ゼロテンプレート → 空(template_len==0)
        let empty = CorrelationPlan::new_ncc(0, 4);
        let mut empty_ws = CorrelationWorkspace::new(&empty);
        let curve = empty.ncc(&mut empty_ws, &[], &[0.0; 4], usize::MAX);
        assert!(curve.is_empty());
    }

    /// NCC 専用 plan(Valid FFT / Direct いずれも)での PHAT 呼出は
    /// FFT 長依存の位相正規化契約に反するため panic で検出する。
    #[test]
    #[should_panic(expected = "NCC-only")]
    fn gcc_phat_panics_on_ncc_valid_plan() {
        let plan = CorrelationPlan::new_ncc(4_410, 7_938);
        assert_eq!(plan.mode, PlanMode::NccValid);
        let mut ws = CorrelationWorkspace::new(&plan);
        plan.gcc_phat(&mut ws, &[1.0; 4_410], &[1.0; 7_938], 3_528);
    }

    /// direct plan での PHAT 呼出も同じ契約で panic する。
    #[test]
    #[should_panic(expected = "NCC-only")]
    fn gcc_phat_panics_on_ncc_direct_plan() {
        let plan = CorrelationPlan::new_ncc(4, 8);
        assert_eq!(plan.mode, PlanMode::NccDirect);
        let mut ws = CorrelationWorkspace::new(&plan);
        plan.gcc_phat(&mut ws, &[1.0; 4], &[1.0; 8], 4);
    }
}
