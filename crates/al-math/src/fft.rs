//! 2 のべき乗長の複素 FFT と実数 FFT。
//! 長さ調整とゼロ埋めは呼び出し側が所有する。

use crate::{Simd, WithSimd, c64, dispatch};
use std::f64::consts::TAU;

/// 変換長ごとの不変係数。複数の変換・スレッドで再利用できる。
pub struct Fft {
    n: usize,
    twiddles: Vec<c64>,
}

fn roots(n: usize, count: usize) -> Vec<c64> {
    let angle = -TAU / n as f64;
    let step = c64::new(angle.cos(), angle.sin());
    let mut w = c64::new(1.0, 0.0);
    (0..count)
        .map(|k| {
            // 長い逐次再帰の振幅・位相ドリフトを抑える。
            if k % 256 == 0 {
                let a = angle * k as f64;
                w = c64::new(a.cos(), a.sin());
            }
            let out = w;
            w *= step;
            out
        })
        .collect()
}

impl Fft {
    /// 非ゼロの 2 のべき乗長を受ける。不正長は panic する。
    pub fn new(n: usize) -> Self {
        assert!(
            n.is_power_of_two(),
            "FFT length must be a nonzero power of two"
        );
        let mut twiddles = Vec::with_capacity(n - 1);
        let mut len = n;
        while len > 1 {
            twiddles.extend(roots(len, len / 2));
            len /= 2;
        }
        Self { n, twiddles }
    }

    /// 自然順序の in-place 変換。forward は負符号、inverse は正符号と 1/N。
    /// バッファ長が生成時の長さと異なる場合は panic する。
    pub fn transform(&self, values: &mut [c64], inverse: bool) {
        assert_eq!(values.len(), self.n, "FFT buffer length mismatch");
        dispatch(Transform {
            plan: self,
            values,
            inverse,
        });
    }
}

struct Transform<'a> {
    plan: &'a Fft,
    values: &'a mut [c64],
    inverse: bool,
}

impl WithSimd for Transform<'_> {
    type Output = ();
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) {
        transform(simd, self.plan, self.values, self.inverse);
    }
}

#[inline(always)]
fn transform<S: Simd>(simd: S, plan: &Fft, values: &mut [c64], inverse: bool) {
    butterfly(simd, values, &plan.twiddles, inverse);
    let n = values.len();
    if n < 1024 {
        for i in 1..n {
            let j = i.reverse_bits() >> (usize::BITS - n.trailing_zeros());
            if i < j {
                values.swap(i, j);
            }
        }
    } else {
        // 高位/低位 5 bit をタイル化し、相互の行をキャッシュに留める。
        let middle_bits = n.trailing_zeros() - 10;
        for middle in 0..(n >> 10) {
            let reversed = if middle_bits == 0 {
                0
            } else {
                middle.reverse_bits() >> (usize::BITS - middle_bits)
            };
            if middle > reversed {
                continue;
            }
            for high in 0usize..32 {
                let low_out = high.reverse_bits() >> (usize::BITS - 5);
                for low in 0usize..32 {
                    let high_out = low.reverse_bits() >> (usize::BITS - 5);
                    let i = (high << (middle_bits + 5)) | (middle << 5) | low;
                    let j = (high_out << (middle_bits + 5)) | (reversed << 5) | low_out;
                    if middle < reversed || i < j {
                        values.swap(i, j);
                    }
                }
            }
        }
    }
    if inverse {
        let scale = simd.splat_c64s(c64::new(1.0 / n as f64, 0.0));
        let (vectors, tail) = S::as_mut_simd_c64s(values);
        for v in vectors {
            *v = simd.mul_c64s(*v, scale);
        }
        for v in tail {
            *v /= n as f64;
        }
    }
}

// SIMD 演算を dispatch の ISA コンテキストへインラインする。
#[inline(always)]
fn butterfly<S: Simd>(simd: S, values: &mut [c64], mut twiddles: &[c64], inverse: bool) {
    let mut len = values.len();
    while len > 1024 {
        stage(simd, values, twiddles, len, inverse);
        twiddles = &twiddles[len / 2 + len / 4..];
        len /= 4;
    }
    for block in values.chunks_exact_mut(len) {
        let mut rest = twiddles;
        let mut size = len;
        while size > 1 {
            stage(simd, block, rest, size, inverse);
            let used = size / 2 + size / 4;
            rest = &rest[used..];
            size = if size == 2 { 1 } else { size / 4 };
        }
    }
}

// radix-2 の 2 段を融合する。出力の組順は 0,2,1,3 で binary bit reversal を保つ。
#[inline(always)]
fn stage<S: Simd>(simd: S, values: &mut [c64], w: &[c64], len: usize, inverse: bool) {
    if len == 2 {
        for pair in values.as_chunks_mut::<2>().0.iter_mut() {
            let a = pair[0];
            pair[0] = a + pair[1];
            pair[1] = a - pair[1];
        }
        return;
    }
    let q = len / 4;
    let (w1, _) = S::as_simd_c64s(&w[..q]);
    let (w2, _) = S::as_simd_c64s(&w[len / 2..len / 2 + q]);
    let rotate = simd.splat_c64s(c64::new(0.0, if inverse { 1.0 } else { -1.0 }));
    for block in values.chunks_exact_mut(len) {
        let (a, tail) = block.split_at_mut(q);
        let (b, tail) = tail.split_at_mut(q);
        let (c, d) = tail.split_at_mut(q);
        let (av, at) = S::as_mut_simd_c64s(a);
        let (bv, bt) = S::as_mut_simd_c64s(b);
        let (cv, ct) = S::as_mut_simd_c64s(c);
        let (dv, dt) = S::as_mut_simd_c64s(d);
        for (((((a, b), c), d), &w1), &w2) in av.iter_mut().zip(bv).zip(cv).zip(dv).zip(w1).zip(w2)
        {
            let w1 = if inverse { simd.conj_c64s(w1) } else { w1 };
            let w2 = if inverse { simd.conj_c64s(w2) } else { w2 };
            let s0 = simd.add_c64s(*a, *c);
            let s1 = simd.add_c64s(*b, *d);
            let d0 = simd.sub_c64s(*a, *c);
            let d1 = simd.mul_c64s(simd.sub_c64s(*b, *d), rotate);
            *a = simd.add_c64s(s0, s1);
            *b = simd.mul_c64s(simd.sub_c64s(s0, s1), w2);
            *c = simd.mul_c64s(simd.add_c64s(d0, d1), w1);
            *d = simd.mul_c64s(simd.sub_c64s(d0, d1), simd.mul_c64s(w1, w2));
        }
        let first = q - at.len();
        for (k, (((a, b), c), d)) in at.iter_mut().zip(bt).zip(ct).zip(dt).enumerate() {
            let k = first + k;
            let w1 = if inverse { w[k].conj() } else { w[k] };
            let w2 = if inverse {
                w[len / 2 + k].conj()
            } else {
                w[len / 2 + k]
            };
            let s0 = *a + *c;
            let s1 = *b + *d;
            let d0 = *a - *c;
            let diff = *b - *d;
            let d1 = if inverse {
                c64::new(-diff.im, diff.re)
            } else {
                c64::new(diff.im, -diff.re)
            };
            *a = s0 + s1;
            *b = (s0 - s1) * w2;
            *c = (d0 + d1) * w1;
            *d = (d0 - d1) * w1 * w2;
        }
    }
}
/// 実数変換の係数と半長の scratch。寿命と再利用は呼び出し側が管理する。
/// spectrum の正本は非負周波数 N/2+1 bins (N=1 は 1 bin)。
pub struct RealFft {
    n: usize,
    fft: Fft,
    roots: Vec<c64>,
    scratch: Vec<c64>,
}

impl RealFft {
    /// 非ゼロの 2 のべき乗長を受ける。不正長は panic する。
    pub fn new(n: usize) -> Self {
        assert!(
            n.is_power_of_two(),
            "real FFT length must be a nonzero power of two"
        );
        Self {
            n,
            fft: Fft::new((n / 2).max(1)),
            roots: roots(n, n / 2 + 1)
                .into_iter()
                .map(|w| c64::new(0.5 * w.im, -0.5 * w.re))
                .collect(),
            scratch: vec![c64::new(0.0, 0.0); (n / 2).max(1)],
        }
    }

    /// 実数から非負周波数へ変換する。入力/出力長が不一致なら panic する。
    pub fn forward(&mut self, input: &[f64], output: &mut [c64]) {
        assert_eq!(input.len(), self.n);
        assert_eq!(output.len(), self.n / 2 + 1);
        dispatch(RealTransform {
            plan: self,
            real: RealBuffer::Input(input),
            spectrum: SpectrumBuffer::Output(output),
        });
    }

    /// 非負周波数から実数へ 1/N 正規化して復元する。
    /// DC/Nyquist の虚部は使用しない。入力/出力長が不一致なら panic する。
    pub fn inverse(&mut self, input: &[c64], output: &mut [f64]) {
        assert_eq!(output.len(), self.n);
        assert_eq!(input.len(), self.n / 2 + 1);
        dispatch(RealTransform {
            plan: self,
            real: RealBuffer::Output(output),
            spectrum: SpectrumBuffer::Input(input),
        });
    }

    // 複合カーネルは一度選択した ISA を全変換で共有する。
    #[inline(always)]
    pub(crate) fn forward_with_simd<S: Simd>(
        &mut self,
        simd: S,
        input: &[f64],
        output: &mut [c64],
    ) {
        assert_eq!(input.len(), self.n);
        assert_eq!(output.len(), self.n / 2 + 1);
        RealTransform {
            plan: self,
            real: RealBuffer::Input(input),
            spectrum: SpectrumBuffer::Output(output),
        }
        .with_simd(simd);
    }

    #[inline(always)]
    pub(crate) fn inverse_with_simd<S: Simd>(
        &mut self,
        simd: S,
        input: &[c64],
        output: &mut [f64],
    ) {
        assert_eq!(output.len(), self.n);
        assert_eq!(input.len(), self.n / 2 + 1);
        RealTransform {
            plan: self,
            real: RealBuffer::Output(output),
            spectrum: SpectrumBuffer::Input(input),
        }
        .with_simd(simd);
    }
}

enum RealBuffer<'a> {
    Input(&'a [f64]),
    Output(&'a mut [f64]),
}
enum SpectrumBuffer<'a> {
    Input(&'a [c64]),
    Output(&'a mut [c64]),
}
struct RealTransform<'a> {
    plan: &'a mut RealFft,
    real: RealBuffer<'a>,
    spectrum: SpectrumBuffer<'a>,
}

impl WithSimd for RealTransform<'_> {
    type Output = ();
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) {
        let p = self.plan;
        let m = p.n / 2;
        match (self.real, self.spectrum) {
            (RealBuffer::Input(input), SpectrumBuffer::Output(output)) => {
                if p.n == 1 {
                    output[0] = c64::new(input[0], 0.0);
                    return;
                }
                for (z, pair) in p.scratch.iter_mut().zip(input.as_chunks::<2>().0.iter()) {
                    *z = c64::new(pair[0], pair[1]);
                }
                transform(simd, &p.fft, &mut p.scratch, false);
                let z = p.scratch[0];
                output[0] = c64::new(z.re + z.im, 0.0);
                output[m] = c64::new(z.re - z.im, 0.0);
                if m > 1 {
                    let mid = m / 2;
                    output[mid] = p.scratch[mid].conj();
                    let (low, high) = output[1..m].split_at_mut(mid);
                    let low = &mut low[..mid - 1];
                    for (out, z) in low.iter_mut().zip(p.scratch[mid + 1..m].iter().rev()) {
                        *out = z.conj();
                    }
                    recombine_pair(simd, &p.scratch[1..mid], &p.roots[1..mid], low, high);
                    high.reverse();
                }
            }
            (RealBuffer::Output(output), SpectrumBuffer::Input(input)) => {
                if p.n == 1 {
                    output[0] = input[0].re;
                    return;
                }
                p.scratch[0] = c64::new(
                    (input[0].re + input[m].re) * 0.5,
                    (input[0].re - input[m].re) * 0.5,
                );
                for (z, x) in p.scratch[1..m].iter_mut().zip(input[1..m].iter().rev()) {
                    *z = x.conj();
                }
                recombine(
                    simd,
                    &input[1..m],
                    &p.roots[1..m],
                    &mut p.scratch[1..m],
                    true,
                );
                transform(simd, &p.fft, &mut p.scratch, true);
                for (pair, z) in output.as_chunks_mut::<2>().0.iter_mut().zip(&p.scratch) {
                    pair[0] = z.re;
                    pair[1] = z.im;
                }
            }
            _ => unreachable!(),
        }
    }
}

// 共役対を一度に求め、和/差と twiddle 積を共有する。
#[inline(always)]
fn recombine_pair<S: Simd>(
    simd: S,
    input: &[c64],
    roots: &[c64],
    low: &mut [c64],
    high: &mut [c64],
) {
    let half = simd.splat_c64s(c64::new(0.5, 0.0));

    let (av, at) = S::as_simd_c64s(input);
    let (wv, wt) = S::as_simd_c64s(roots);
    let (bv, bt) = S::as_mut_simd_c64s(low);
    let (hv, ht) = S::as_mut_simd_c64s(high);
    for (((&a, &w), b), h) in av.iter().zip(wv).zip(bv).zip(hv) {
        let sum = simd.mul_c64s(simd.add_c64s(a, *b), half);
        let diff = simd.mul_c64s(w, simd.sub_c64s(a, *b));
        *b = simd.add_c64s(sum, diff);
        *h = simd.conj_c64s(simd.sub_c64s(sum, diff));
    }
    for (((&a, &w), b), h) in at.iter().zip(wt).zip(bt).zip(ht) {
        let sum = (a + *b) * 0.5;
        let diff = w * (a - *b);
        *b = sum + diff;
        *h = (sum - diff).conj();
    }
}
// mirrored は反転済み共役 spectrum。前後方向とも同一の復元規則を使う。
#[inline(always)]
fn recombine<S: Simd>(simd: S, input: &[c64], roots: &[c64], mirrored: &mut [c64], inverse: bool) {
    let half = simd.splat_c64s(c64::new(0.5, 0.0));

    let (av, at) = S::as_simd_c64s(input);
    let (wv, wt) = S::as_simd_c64s(roots);
    let (bv, bt) = S::as_mut_simd_c64s(mirrored);
    for ((&a, &w), b) in av.iter().zip(wv).zip(bv) {
        let w = if inverse { simd.conj_c64s(w) } else { w };
        let diff = simd.mul_c64s(w, simd.sub_c64s(a, *b));
        *b = simd.add_c64s(simd.mul_c64s(simd.add_c64s(a, *b), half), diff);
    }
    for ((&a, &w), b) in at.iter().zip(wt).zip(bt) {
        let w = if inverse { w.conj() } else { w };
        *b = (a + *b) * 0.5 + w * (a - *b);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scalar_and_dispatch_share_the_kernel() {
        for power in 0..=10 {
            let n = 1 << power;
            let plan = Fft::new(n);
            for inverse in [false, true] {
                let mut native: Vec<_> = (0..n)
                    .map(|i| c64::new((i as f64 * 0.7).sin(), (i as f64 * 0.3).cos()))
                    .collect();
                let mut scalar = native.clone();
                plan.transform(&mut native, inverse);
                pulp::Scalar.vectorize(Transform {
                    plan: &plan,
                    values: &mut scalar,
                    inverse,
                });
                for (a, b) in native.iter().zip(&scalar) {
                    assert!((*a - *b).l1_norm() < 1e-10 * (1.0 + b.l1_norm()));
                }
            }
            let input: Vec<_> = (0..n).map(|i| (i as f64 * 0.7).sin()).collect();
            let mut plan = RealFft::new(n);
            let mut native = vec![c64::new(0.0, 0.0); n / 2 + 1];
            let mut scalar = native.clone();
            plan.forward(&input, &mut native);
            pulp::Scalar.vectorize(RealTransform {
                plan: &mut plan,
                real: RealBuffer::Input(&input),
                spectrum: SpectrumBuffer::Output(&mut scalar),
            });
            for (a, b) in native.iter().zip(&scalar) {
                assert!((*a - *b).l1_norm() < 1e-10 * (1.0 + b.l1_norm()));
            }
            let mut output = vec![0.0; n];
            pulp::Scalar.vectorize(RealTransform {
                plan: &mut plan,
                real: RealBuffer::Output(&mut output),
                spectrum: SpectrumBuffer::Input(&scalar),
            });
            for (a, b) in output.iter().zip(&input) {
                assert!((a - b).abs() < 1e-10);
            }
        }
    }
}
