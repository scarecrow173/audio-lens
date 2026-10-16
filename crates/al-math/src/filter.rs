//! 実数 FFT の spectral mask フィルタとスペクトル包絡。
//! 信号区間とゼロ埋め長を保持し、全入力キャッシュの意味は導入しない。

use crate::{Simd, WithSimd, dispatch, fft::RealFft};

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
        let mut core = FilterCore::forward(simd, self.samples, self.sample_rate);
        // FFT は closure 外で実行し、選択した ISA コンテキストへインラインする。
        self.bands
            .iter()
            .map(|&(lo, hi)| core.band(simd, lo, hi))
            .collect()
    }
}

/// 全区間の forward スペクトルと IFFT 用スクラッチ。mask の bin 判定・
/// 切り戻し規則の単一実装をここに置き、`bandpass_many` 一括生成と
/// `BandFilter` 逐次生成の両方が同じ手順を通る。
struct FilterCore {
    plan: RealFft,
    bin_hz: f64,
    len: usize,
    spectrum_re: Vec<f64>,
    spectrum_im: Vec<f64>,
    masked_re: Vec<f64>,
    masked_im: Vec<f64>,
    real: Vec<f64>,
}

impl FilterCore {
    fn forward<S: Simd>(simd: S, samples: &[f64], sample_rate: u32) -> Self {
        if samples.is_empty() {
            return Self {
                plan: RealFft::new(1),
                bin_hz: 0.0,
                len: 0,
                spectrum_re: Vec::new(),
                spectrum_im: Vec::new(),
                masked_re: Vec::new(),
                masked_im: Vec::new(),
                real: Vec::new(),
            };
        }
        let n = samples.len().next_power_of_two();
        let mut plan = RealFft::new(n);
        let mut real = vec![0.0; n];
        real[..samples.len()].copy_from_slice(samples);
        let mut spectrum_re = vec![0.0; n / 2 + 1];
        let mut spectrum_im = vec![0.0; n / 2 + 1];
        plan.forward_with_simd(simd, &real, &mut spectrum_re, &mut spectrum_im);
        Self {
            plan,
            bin_hz: f64::from(sample_rate) / n as f64,
            len: samples.len(),
            spectrum_re,
            spectrum_im,
            masked_re: vec![0.0; n / 2 + 1],
            masked_im: vec![0.0; n / 2 + 1],
            real,
        }
    }

    fn band<S: Simd>(&mut self, simd: S, lo_hz: f64, hi_hz: f64) -> Vec<f64> {
        if self.len == 0 {
            return Vec::new();
        }
        // 連続した passband だけコピーし、範囲外はゼロにする。
        // 比較を周波数値で行い、旧実装の境界丸め規則を保つ。
        let lo_bin = lower_bin(self.spectrum_re.len(), |k| self.bin_hz * (k as f64) < lo_hz);
        let hi_bin = lower_bin(self.spectrum_re.len(), |k| {
            hi_hz.is_nan() || self.bin_hz * (k as f64) < hi_hz
        });
        self.masked_re.fill(0.0);
        self.masked_im.fill(0.0);
        if lo_bin < hi_bin {
            self.masked_re[lo_bin..hi_bin].copy_from_slice(&self.spectrum_re[lo_bin..hi_bin]);
            self.masked_im[lo_bin..hi_bin].copy_from_slice(&self.spectrum_im[lo_bin..hi_bin]);
        }
        self.plan.inverse_with_simd(
            simd,
            &self.masked_re,
            &self.masked_im,
            &mut self.real,
        );
        self.real[..self.len].to_vec()
    }
}

/// 1 回の forward FFT で保持したスペクトルから帯域を逐次生成する
/// フィルタ。帯域を 1 本ずつ消費する呼出側で、全帯域波形の同時保持を
/// 避ける。帯域ごとの出力は `bandpass_many` と同一規則。
pub struct BandFilter {
    core: FilterCore,
}

impl BandFilter {
    /// 全区間の forward FFT を 1 回だけ行い、スペクトルを保持する。
    pub fn new(samples: &[f64], sample_rate: u32) -> Self {
        dispatch(BandForward {
            samples,
            sample_rate,
        })
    }

    /// `[lo_hz, hi_hz)` の帯域波形を 1 本生成する。`bandpass_many` と
    /// 同一の mask・IFFT・切り戻し規則。スクラッチを共有するため
    /// `&mut self` を取る。
    pub fn band(&mut self, lo_hz: f64, hi_hz: f64) -> Vec<f64> {
        dispatch(BandInverse {
            core: &mut self.core,
            lo_hz,
            hi_hz,
        })
    }
}

struct BandForward<'a> {
    samples: &'a [f64],
    sample_rate: u32,
}

impl WithSimd for BandForward<'_> {
    type Output = BandFilter;
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        BandFilter {
            core: FilterCore::forward(simd, self.samples, self.sample_rate),
        }
    }
}

struct BandInverse<'a> {
    core: &'a mut FilterCore,
    lo_hz: f64,
    hi_hz: f64,
}

impl WithSimd for BandInverse<'_> {
    type Output = Vec<f64>;
    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        self.core.band(simd, self.lo_hz, self.hi_hz)
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
/// fft_size は 2 のべき乗(0 は不可)。fft_size が信号長より小さい
/// 場合は信号長まで拡大する。bins=0 は空結果を返す。
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
        assert!(
            self.fft_size.is_power_of_two(),
            "fft_size must be a non-zero power of two, got {}",
            self.fft_size
        );
        if self.bins == 0 {
            return Vec::new();
        }
        let n = self.fft_size.max(self.samples.len().next_power_of_two());
        let mut plan = RealFft::new(n);
        let mut real = vec![0.0; n];
        real[..self.samples.len()].copy_from_slice(self.samples);
        let mut spectrum_re = vec![0.0; n / 2 + 1];
        let mut spectrum_im = vec![0.0; n / 2 + 1];
        plan.forward_with_simd(simd, &real, &mut spectrum_re, &mut spectrum_im);
        // bin ごとの |z|² を実部演算で求める。
        let mut magnitudes = vec![0.0; spectrum_re.len()];
        let (srv, srt) = S::as_simd_f64s(&spectrum_re);
        let (siv, sit) = S::as_simd_f64s(&spectrum_im);
        let (mv, mt) = S::as_mut_simd_f64s(&mut magnitudes);
        for ((&re, &im), m) in srv.iter().zip(siv).zip(mv) {
            *m = simd.add_f64s(simd.mul_f64s(re, re), simd.mul_f64s(im, im));
        }
        for ((&re, &im), m) in srt.iter().zip(sit).zip(mt) {
            *m = re * re + im * im;
        }
        let (vectors, tail) = S::as_mut_simd_f64s(&mut magnitudes);
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
                let sum: f64 = magnitudes[lo..hi].iter().sum();
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
            // BandFilter の逐次生成も両経路で一致する。
            let mut native: BandFilter = dispatch(BandForward {
                samples: &input,
                sample_rate: 2048,
            });
            let mut scalar: BandFilter = pulp::Scalar.vectorize(BandForward {
                samples: &input,
                sample_rate: 2048,
            });
            for &(lo, hi) in &bands {
                let a = dispatch(BandInverse {
                    core: &mut native.core,
                    lo_hz: lo,
                    hi_hz: hi,
                });
                let b = pulp::Scalar.vectorize(BandInverse {
                    core: &mut scalar.core,
                    lo_hz: lo,
                    hi_hz: hi,
                });
                assert_eq!(a.len(), b.len());
                for (a, b) in a.iter().zip(&b) {
                    assert!((a - b).abs() < 1e-10);
                }
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

    // BandFilter の逐次生成は `bandpass_many` の一括生成と帯域ごとに
    // 完全一致する(同一 forward・mask・IFFT・切り戻し)。空入力・
    // NaN hi・全帯域のエッジも同じ観測結果を返す。
    #[test]
    fn band_filter_matches_bandpass_many_per_band() {
        for len in [0, 1, 3, 33, 256, 1027, 4096] {
            let input: Vec<_> = (0..len).map(|i| (i as f64 * 0.713).sin()).collect();
            let bands = [
                (0.0, 100.0),
                (100.0, 250.0),
                (0.0, 1025.0),
                (250.0, 250.0),
                (500.0, f64::NAN),
                (0.0, f64::INFINITY),
            ];
            let mut filter = BandFilter::new(&input, 2048);
            let expected = bandpass_many(&input, 2048, &bands);
            for ((lo, hi), expected) in bands.iter().zip(expected) {
                assert_eq!(filter.band(*lo, *hi), expected, "len={len} band=({lo},{hi})");
            }
        }
    }

    // T1 characterization(TASK-034Fo9cS08WsbNrLoq6t6a / test-spec
    // `locate-argmax-tie-eps-calibration.md` Rule B): `Envelope`
    // カーネルの scalar↔dispatch 偏差を実運用サイズで実測する。
    // production では `SpectralEnvelope::locate` が fft_size=1024・
    // bins=32 でテンプレート長(数kサンプル)の窓へ適用し、実効
    // FFT は `max(fft_size, len.next_power_of_two())`。
    #[test]
    fn dispatch_deviation_envelope_at_production_sizes() {
        let mut s = 7u64;
        let mut noise = |len: usize| -> Vec<f64> {
            (0..len)
                .map(|_| {
                    s = s
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    ((s >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0
                })
                .collect()
        };
        for len in [2_048usize, 4_096, 8_192] {
            let input = noise(len);
            let n = 1_024usize.max(len.next_power_of_two());
            let kernel = || Envelope {
                samples: &input,
                fft_size: 1_024,
                bins: 32,
            };
            let native = dispatch(kernel());
            let scalar = pulp::Scalar.vectorize(kernel());
            assert_eq!(native.len(), scalar.len());
            let mut max_abs = 0.0f64;
            for (&a, &b) in native.iter().zip(&scalar) {
                let d = (a - b).abs();
                max_abs = max_abs.max(d);
                // 採用 eps(TIE_REL_EPS=1e-11)より厳密に小さいことを
                // dispatch 不変性の要求として pin する(計測値は
                // ≤ ~5e-16)。
                assert!(d < 1e-12, "n={n}: dispatch {a} vs scalar {b}");
            }
            eprintln!("envelope dispatch-dev: len={len} fft_n={n} max_abs={max_abs:.3e}");
        }
    }

    #[test]
    #[should_panic(expected = "fft_size must be a non-zero power of two")]
    fn envelope_panics_on_zero_fft_size() {
        let input = [0.0; 64];
        spectral_envelope(&input, 0, 8);
    }

    #[test]
    #[should_panic(expected = "fft_size must be a non-zero power of two")]
    fn envelope_panics_on_non_power_of_two_fft_size() {
        let input = [0.0; 64];
        spectral_envelope(&input, 1000, 8);
    }
}
