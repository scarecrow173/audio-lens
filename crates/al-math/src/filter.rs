//! 実数 FFT の spectral mask フィルタとスペクトル包絡。
//! 信号区間とゼロ埋め長を保持し、全入力キャッシュの意味は導入しない。

use crate::{Simd, WithSimd, c64, dispatch, fft::RealFft};

/// ゼロ位相の `[lo_hz, hi_hz)` spectral mask。Nyquist bin も同じ規則で判定する。
pub fn bandpass(samples: &[f64], sample_rate: u32, lo_hz: f64, hi_hz: f64) -> Vec<f64> {
    bandpass_many(samples, sample_rate, &[(lo_hz, hi_hz)])
        .pop()
        .unwrap()
}

/// 同一区間の forward FFT を一度だけ行い、帯域ごとに IFFT する。
/// 出力順は bands と同じ。元長へ切り戻し、空入力は帯域ごとの空ベクトル。
pub fn bandpass_many(samples: &[f64], sample_rate: u32, bands: &[(f64, f64)]) -> Vec<Vec<f64>> {
    dispatch(Bandpass {
        samples,
        sample_rate,
        bands,
    })
}

struct Bandpass<'a> {
    samples: &'a [f64],
    sample_rate: u32,
    bands: &'a [(f64, f64)],
}

impl WithSimd for Bandpass<'_> {
    type Output = Vec<Vec<f64>>;
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        if self.samples.is_empty() || self.bands.is_empty() {
            return vec![Vec::new(); self.bands.len()];
        }
        let n = self.samples.len().next_power_of_two();
        let mut plan = RealFft::new(n);
        let mut real = vec![0.0; n];
        real[..self.samples.len()].copy_from_slice(self.samples);
        let mut spectrum = vec![c64::new(0.0, 0.0); n / 2 + 1];
        plan.forward_with_simd(simd, &real, &mut spectrum);
        let mut masked = spectrum.clone();
        let bin_hz = f64::from(self.sample_rate) / n as f64;
        // FFT は closure 外で実行し、選択した ISA コンテキストへインラインする。
        let mut output = Vec::with_capacity(self.bands.len());
        for &(lo, hi) in self.bands {
            // 連続した passband だけコピーし、範囲外はゼロにする。
            // 比較を周波数値で行い、旧実装の境界丸め規則を保つ。
            let lo_bin = lower_bin(spectrum.len(), |k| bin_hz * (k as f64) < lo);
            let hi_bin = lower_bin(spectrum.len(), |k| hi.is_nan() || bin_hz * (k as f64) < hi);
            masked.fill(c64::new(0.0, 0.0));
            if lo_bin < hi_bin {
                masked[lo_bin..hi_bin].copy_from_slice(&spectrum[lo_bin..hi_bin]);
            }
            plan.inverse_with_simd(simd, &masked, &mut real);
            output.push(real[..self.samples.len()].to_vec());
        }
        output
    }
}

// 周波数 grid の lower_bound。信号長に比例する mask 比較を避ける。
fn lower_bin(length: usize, predicate: impl Fn(usize) -> bool) -> usize {
    let (mut lo, mut hi) = (0, length);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if predicate(mid) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

/// 非負周波数の区間平均振幅を log に変換する。exp/log は scalar。
/// fft_size は 2 のべき乗。bins=0 は空結果を返す。
pub fn spectral_envelope(samples: &[f64], fft_size: usize, bins: usize) -> Vec<f64> {
    dispatch(Envelope {
        samples,
        fft_size,
        bins,
    })
}

struct Envelope<'a> {
    samples: &'a [f64],
    fft_size: usize,
    bins: usize,
}

impl WithSimd for Envelope<'_> {
    type Output = Vec<f64>;
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        if self.bins == 0 {
            return Vec::new();
        }
        let n = self.fft_size.max(self.samples.len().next_power_of_two());
        let mut plan = RealFft::new(n);
        let mut real = vec![0.0; n];
        real[..self.samples.len()].copy_from_slice(self.samples);
        let mut spectrum = vec![c64::new(0.0, 0.0); n / 2 + 1];
        plan.forward_with_simd(simd, &real, &mut spectrum);
        let mut magnitudes = vec![c64::new(0.0, 0.0); spectrum.len()];
        let (sv, st) = S::as_simd_c64s(&spectrum);
        let (mv, mt) = S::as_mut_simd_c64s(&mut magnitudes);
        for (&s, m) in sv.iter().zip(mv) {
            *m = simd.abs2_c64s(s);
        }
        for (&s, m) in st.iter().zip(mt) {
            let e = s.norm_sqr();
            *m = c64::new(e, e);
        }
        let squared: &mut [f64] = pulp::bytemuck::cast_slice_mut(&mut magnitudes);
        let (vectors, tail) = S::as_mut_simd_f64s(squared);
        for v in vectors {
            *v = simd.sqrt_f64s(*v);
        }
        for v in tail {
            *v = v.sqrt();
        }
        let half = n / 2;
        (0..self.bins)
            .map(|b| {
                let lo = b * half / self.bins;
                let hi = ((b + 1) * half / self.bins).max(lo + 1).min(half);
                let sum: f64 = magnitudes[lo..hi].iter().map(|v| v.re).sum();
                (sum / (hi - lo) as f64 + 1e-12).ln()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_and_dispatch_match_for_filters_and_envelopes() {
        for len in [0, 1, 3, 7, 16, 33, 127, 256] {
            let input: Vec<_> = (0..len).map(|i| (i as f64 * 0.713).sin()).collect();
            let bands = [(0.0, 100.0), (100.0, 250.0), (0.0, 1025.0), (250.0, 250.0)];
            let kernel = || Bandpass {
                samples: &input,
                sample_rate: 2048,
                bands: &bands,
            };
            let native = dispatch(kernel());
            let scalar = pulp::Scalar.vectorize(kernel());
            for (a, b) in native.iter().flatten().zip(scalar.iter().flatten()) {
                assert!((a - b).abs() < 1e-10);
            }
            let kernel = || Envelope {
                samples: &input,
                fft_size: 256,
                bins: 17,
            };
            let native = dispatch(kernel());
            let scalar = pulp::Scalar.vectorize(kernel());
            for (a, b) in native.iter().zip(scalar) {
                assert!((a - b).abs() < 1e-10);
            }
        }
    }
}
