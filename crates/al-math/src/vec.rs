//! f64 スライスのベクトル演算プリミティブ。
//!
//! 各プリミティブは `Simd` ジェネリックな `pub(crate)` カーネルを単一
//! ソースとし、公開 wrapper が入口で `dispatch` を一度呼ぶ。公開
//! シグネチャはスライスと f64 スカラーのみで、pulp 型は露出しない。
//!
//! reduction 系の積和は `mul_add_e_f64s` のみを使う。Scalar(1 レーン)
//! では非融合の `a*b+c` に下り逐次和とビット一致し、x86-v3 では真の
//! FMA に下る。`mul_add_f64s`(Scalar でも融合)と `mul_f64s`+`add_f64s`
//! の 2 命令形(FMA を失う)は使わない。

use crate::{Simd, WithSimd, dispatch};

/// 複数スライスの等長契約。違反は panic とする(zip 黙認しない)。
#[inline(always)]
fn check_same_len(a: usize, b: usize) {
    assert_eq!(a, b, "slice lengths must be equal");
}

/// `a·b` の内積。lane アキュムレータ + `reduce_sum` + scalar tail。
#[inline(always)]
pub(crate) fn dot_kernel<S: Simd>(simd: S, a: &[f64], b: &[f64]) -> f64 {
    check_same_len(a.len(), b.len());
    let (av, at) = S::as_simd_f64s(a);
    let (bv, bt) = S::as_simd_f64s(b);
    let mut sum = simd.splat_f64s(0.0);
    for (&a, &b) in av.iter().zip(bv) {
        sum = simd.mul_add_e_f64s(a, b, sum);
    }
    simd.reduce_sum_f64s(sum) + at.iter().zip(bt).map(|(&a, &b)| a * b).sum::<f64>()
}

/// `Σx²`。`dot_kernel` と同構造で `x·x` を累算する。
#[inline(always)]
pub(crate) fn sum_sq_kernel<S: Simd>(simd: S, x: &[f64]) -> f64 {
    let (xv, xt) = S::as_simd_f64s(x);
    let mut sum = simd.splat_f64s(0.0);
    for &x in xv {
        sum = simd.mul_add_e_f64s(x, x, sum);
    }
    simd.reduce_sum_f64s(sum) + xt.iter().map(|&x| x * x).sum::<f64>()
}

/// 中心化しない `Σxy / √(Σx²·Σy²)`。3 アキュムレータを 1 パスで融合する。
/// `Σx² > 0 かつ Σy² > 0` でなければ 0.0(NaN・縮退エネルギーも 0.0)。
#[inline(always)]
pub(crate) fn corr0_kernel<S: Simd>(simd: S, x: &[f64], y: &[f64]) -> f64 {
    check_same_len(x.len(), y.len());
    let (xv, xt) = S::as_simd_f64s(x);
    let (yv, yt) = S::as_simd_f64s(y);
    let mut sxy = simd.splat_f64s(0.0);
    let mut sxx = simd.splat_f64s(0.0);
    let mut syy = simd.splat_f64s(0.0);
    for (&x, &y) in xv.iter().zip(yv) {
        sxy = simd.mul_add_e_f64s(x, y, sxy);
        sxx = simd.mul_add_e_f64s(x, x, sxx);
        syy = simd.mul_add_e_f64s(y, y, syy);
    }
    let ex = simd.reduce_sum_f64s(sxx) + xt.iter().map(|&x| x * x).sum::<f64>();
    let ey = simd.reduce_sum_f64s(syy) + yt.iter().map(|&y| y * y).sum::<f64>();
    let sxy = simd.reduce_sum_f64s(sxy) + xt.iter().zip(yt).map(|(&x, &y)| x * y).sum::<f64>();
    if ex > 0.0 && ey > 0.0 {
        sxy / (ex * ey).sqrt()
    } else {
        0.0
    }
}

/// `sqrt(Σx² / n)`。空入力は 0.0。`sum_sq_kernel` を再利用する。
#[inline(always)]
pub(crate) fn rms_kernel<S: Simd>(simd: S, x: &[f64]) -> f64 {
    let n = x.len();
    if n == 0 {
        0.0
    } else {
        (sum_sq_kernel(simd, x) / n as f64).sqrt()
    }
}

/// `out[i] = a·x[i]`。純粋な乗算のため `mul_f64s` を使う。
#[inline(always)]
pub(crate) fn scale_kernel<S: Simd>(simd: S, x: &[f64], a: f64, out: &mut [f64]) {
    check_same_len(x.len(), out.len());
    let (xv, xt) = S::as_simd_f64s(x);
    let (ov, ot) = S::as_mut_simd_f64s(out);
    let av = simd.splat_f64s(a);
    for (&x, out) in xv.iter().zip(ov) {
        *out = simd.mul_f64s(x, av);
    }
    for (&x, out) in xt.iter().zip(ot) {
        *out = x * a;
    }
}

/// `y[i] += a·x[i]`。
#[inline(always)]
pub(crate) fn axpy_kernel<S: Simd>(simd: S, y: &mut [f64], a: f64, x: &[f64]) {
    check_same_len(y.len(), x.len());
    let (yv, yt) = S::as_mut_simd_f64s(y);
    let (xv, xt) = S::as_simd_f64s(x);
    let av = simd.splat_f64s(a);
    for (y, &x) in yv.iter_mut().zip(xv) {
        *y = simd.mul_add_e_f64s(av, x, *y);
    }
    for (y, &x) in yt.iter_mut().zip(xt) {
        *y += a * x;
    }
}

/// `y[i] -= a·x[i]`。Scalar で `y - a*x` に下り逐次形とビット一致する
/// `negate_mul_add_e_f64s` を使う。
#[inline(always)]
pub(crate) fn sub_scaled_kernel<S: Simd>(simd: S, y: &mut [f64], a: f64, x: &[f64]) {
    check_same_len(y.len(), x.len());
    let (yv, yt) = S::as_mut_simd_f64s(y);
    let (xv, xt) = S::as_simd_f64s(x);
    let av = simd.splat_f64s(a);
    for (y, &x) in yv.iter_mut().zip(xv) {
        *y = simd.negate_mul_add_e_f64s(av, x, *y);
    }
    for (y, &x) in yt.iter_mut().zip(xt) {
        *y -= a * x;
    }
}

/// `out[i] = re[i]² + im[i]²`。`re`/`im`/`out` の 3 スライス全てが等長。
#[inline(always)]
pub(crate) fn mag_sq_kernel<S: Simd>(simd: S, re: &[f64], im: &[f64], out: &mut [f64]) {
    check_same_len(re.len(), im.len());
    check_same_len(re.len(), out.len());
    let (rv, rt) = S::as_simd_f64s(re);
    let (iv, it) = S::as_simd_f64s(im);
    let (ov, ot) = S::as_mut_simd_f64s(out);
    for ((&r, &i), out) in rv.iter().zip(iv).zip(ov) {
        *out = simd.mul_add_e_f64s(r, r, simd.mul_f64s(i, i));
    }
    for ((&r, &i), out) in rt.iter().zip(it).zip(ot) {
        *out = r * r + i * i;
    }
}

struct Dot<'a> {
    a: &'a [f64],
    b: &'a [f64],
}
impl WithSimd for Dot<'_> {
    type Output = f64;
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        dot_kernel(simd, self.a, self.b)
    }
}

struct SumSq<'a>(&'a [f64]);
impl WithSimd for SumSq<'_> {
    type Output = f64;
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        sum_sq_kernel(simd, self.0)
    }
}

struct Corr0<'a> {
    x: &'a [f64],
    y: &'a [f64],
}
impl WithSimd for Corr0<'_> {
    type Output = f64;
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        corr0_kernel(simd, self.x, self.y)
    }
}

struct Rms<'a>(&'a [f64]);
impl WithSimd for Rms<'_> {
    type Output = f64;
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        rms_kernel(simd, self.0)
    }
}

struct Scale<'a> {
    x: &'a [f64],
    a: f64,
    out: &'a mut [f64],
}
impl WithSimd for Scale<'_> {
    type Output = ();
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        scale_kernel(simd, self.x, self.a, self.out)
    }
}

struct Axpy<'a> {
    y: &'a mut [f64],
    a: f64,
    x: &'a [f64],
}
impl WithSimd for Axpy<'_> {
    type Output = ();
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        axpy_kernel(simd, self.y, self.a, self.x)
    }
}

struct SubScaled<'a> {
    y: &'a mut [f64],
    a: f64,
    x: &'a [f64],
}
impl WithSimd for SubScaled<'_> {
    type Output = ();
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        sub_scaled_kernel(simd, self.y, self.a, self.x)
    }
}

struct MagSq<'a> {
    re: &'a [f64],
    im: &'a [f64],
    out: &'a mut [f64],
}
impl WithSimd for MagSq<'_> {
    type Output = ();
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        mag_sq_kernel(simd, self.re, self.im, self.out)
    }
}

/// `a·b` の内積。2 スライスは等長でなければ panic。
pub fn dot(a: &[f64], b: &[f64]) -> f64 {
    dispatch(Dot { a, b })
}

/// `Σx²`。空入力は 0.0。
pub fn sum_sq(x: &[f64]) -> f64 {
    dispatch(SumSq(x))
}

/// 中心化しない `Σxy / √(Σx²·Σy²)`。両エネルギーが正でなければ 0.0
/// (NaN 入力も 0.0)。2 スライスは等長でなければ panic。
pub fn corr0(x: &[f64], y: &[f64]) -> f64 {
    dispatch(Corr0 { x, y })
}

/// `sqrt(Σx² / n)`。空入力は 0.0。
pub fn rms(x: &[f64]) -> f64 {
    dispatch(Rms(x))
}

/// `out[i] = a·x[i]`。`x`/`out` は等長でなければ panic。エイリアス不可。
pub fn scale(x: &[f64], a: f64, out: &mut [f64]) {
    dispatch(Scale { x, a, out });
}

/// `y[i] += a·x[i]`。`y`/`x` は等長でなければ panic。エイリアス不可。
pub fn axpy(y: &mut [f64], a: f64, x: &[f64]) {
    dispatch(Axpy { y, a, x });
}

/// `y[i] -= a·x[i]`。`y`/`x` は等長でなければ panic。エイリアス不可。
pub fn sub_scaled(y: &mut [f64], a: f64, x: &[f64]) {
    dispatch(SubScaled { y, a, x });
}

/// `out[i] = re[i]² + im[i]²`。3 スライス全て等長でなければ panic。
/// エイリアス不可。
pub fn mag_sq(re: &[f64], im: &[f64], out: &mut [f64]) {
    dispatch(MagSq { re, im, out });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tolerance: f64) {
        assert!((a - b).abs() <= tolerance, "{a} != {b}");
    }

    // 逐次 `.iter()` 参照実装。Scalar 経路はこれとビット一致を要求する。
    fn ref_dot(a: &[f64], b: &[f64]) -> f64 {
        a.iter().zip(b).map(|(&x, &y)| x * y).sum()
    }
    fn ref_sum_sq(x: &[f64]) -> f64 {
        x.iter().map(|&x| x * x).sum()
    }
    fn ref_corr0(x: &[f64], y: &[f64]) -> f64 {
        let mut sxy = 0.0;
        let mut ex = 0.0;
        let mut ey = 0.0;
        for (&x, &y) in x.iter().zip(y) {
            sxy += x * y;
            ex += x * x;
            ey += y * y;
        }
        if ex > 0.0 && ey > 0.0 {
            sxy / (ex * ey).sqrt()
        } else {
            0.0
        }
    }
    fn ref_rms(x: &[f64]) -> f64 {
        if x.is_empty() {
            0.0
        } else {
            (ref_sum_sq(x) / x.len() as f64).sqrt()
        }
    }

    // 0..=33(空・短入力・lane 端数)・1k・64k の決定的入力。
    fn inputs(n: usize) -> (Vec<f64>, Vec<f64>) {
        let x: Vec<f64> = (0..n).map(|i| (i as f64 * 0.71).sin() - 0.25).collect();
        let y: Vec<f64> = (0..n).map(|i| (i as f64 * 1.37).cos() + 0.5).collect();
        (x, y)
    }

    #[test]
    fn corr0_returns_ratio_only_when_both_energies_are_positive() {
        let (x, y) = inputs(33);
        close(corr0(&x, &y), ref_corr0(&x, &y), 1e-12);
        // corr0(x, x) = Σx²/|Σx²| = 1(正のエネルギーのとき)。
        assert_eq!(corr0(&x, &x), 1.0);
        // 直交入力は sxy = 0 → 0.0。
        assert_eq!(corr0(&[1.0, 0.0, 0.0], &[0.0, 1.0, 0.0]), 0.0);
    }

    #[test]
    fn corr0_maps_zero_or_nan_energy_to_zero() {
        let x = [1.0, -2.0, 3.0];
        let zeros = [0.0; 3];
        assert_eq!(corr0(&zeros, &x), 0.0);
        assert_eq!(corr0(&x, &zeros), 0.0);
        assert_eq!(corr0(&zeros, &zeros), 0.0);
        assert_eq!(corr0(&[], &[]), 0.0);
        assert_eq!(corr0(&[f64::NAN, 1.0], &[1.0, 1.0]), 0.0);
        assert_eq!(corr0(&[1.0, 1.0], &[1.0, f64::NAN]), 0.0);
    }

    #[test]
    fn rms_is_zero_for_empty_and_abs_for_constant_input() {
        assert_eq!(rms(&[]), 0.0);
        for n in 1..=16 {
            assert_eq!(rms(&vec![-0.5; n]), 0.5);
        }
    }

    #[test]
    #[should_panic]
    fn dot_panics_on_length_mismatch() {
        dot(&[1.0; 3], &[1.0; 4]);
    }

    #[test]
    #[should_panic]
    fn corr0_panics_on_length_mismatch() {
        corr0(&[1.0; 3], &[1.0; 4]);
    }

    #[test]
    #[should_panic]
    fn scale_panics_on_length_mismatch() {
        scale(&[1.0; 3], 2.0, &mut [0.0; 4]);
    }

    #[test]
    #[should_panic]
    fn axpy_panics_on_length_mismatch() {
        axpy(&mut [0.0; 3], 2.0, &[1.0; 4]);
    }

    #[test]
    #[should_panic]
    fn sub_scaled_panics_on_length_mismatch() {
        sub_scaled(&mut [0.0; 3], 2.0, &[1.0; 4]);
    }

    #[test]
    #[should_panic]
    fn mag_sq_panics_on_re_im_mismatch() {
        mag_sq(&[1.0; 3], &[1.0; 4], &mut [0.0; 3]);
    }

    #[test]
    #[should_panic]
    fn mag_sq_panics_on_re_out_mismatch() {
        mag_sq(&[1.0; 3], &[1.0; 3], &mut [0.0; 4]);
    }

    #[test]
    fn scalar_reductions_match_sequential_reference_bit_for_bit() {
        for n in (0..=33).chain([1_024, 65_536]) {
            let (x, y) = inputs(n);
            assert_eq!(
                pulp::Scalar.vectorize(Dot { a: &x, b: &y }),
                ref_dot(&x, &y),
                "dot n={n}"
            );
            assert_eq!(
                pulp::Scalar.vectorize(SumSq(&x)),
                ref_sum_sq(&x),
                "sum_sq n={n}"
            );
            assert_eq!(
                pulp::Scalar.vectorize(Corr0 { x: &x, y: &y }),
                ref_corr0(&x, &y),
                "corr0 n={n}"
            );
            assert_eq!(pulp::Scalar.vectorize(Rms(&x)), ref_rms(&x), "rms n={n}");
        }
    }

    #[test]
    fn scalar_elementwise_match_sequential_reference_bit_for_bit() {
        for n in (0..=33).chain([1_024, 65_536]) {
            let (x, y) = inputs(n);
            let a = -2.5;

            let mut out = vec![0.0; n];
            pulp::Scalar.vectorize(Scale {
                x: &x,
                a,
                out: &mut out,
            });
            let expected: Vec<f64> = x.iter().map(|&x| x * a).collect();
            assert_eq!(out, expected, "scale n={n}");

            let mut acc = y.clone();
            pulp::Scalar.vectorize(Axpy {
                y: &mut acc,
                a,
                x: &x,
            });
            let expected: Vec<f64> = y.iter().zip(&x).map(|(&y, &x)| y + a * x).collect();
            assert_eq!(acc, expected, "axpy n={n}");

            let mut acc = y.clone();
            pulp::Scalar.vectorize(SubScaled {
                y: &mut acc,
                a,
                x: &x,
            });
            let expected: Vec<f64> = y.iter().zip(&x).map(|(&y, &x)| y - a * x).collect();
            assert_eq!(acc, expected, "sub_scaled n={n}");

            let mut out = vec![0.0; n];
            pulp::Scalar.vectorize(MagSq {
                re: &x,
                im: &y,
                out: &mut out,
            });
            let expected: Vec<f64> = x.iter().zip(&y).map(|(&r, &i)| r * r + i * i).collect();
            assert_eq!(out, expected, "mag_sq n={n}");
        }
    }
}
