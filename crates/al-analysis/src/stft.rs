//! Complex STFT and derived spectral intermediates.
//!
//! The STFT kernel is `ruviform_numeric::stft::StftPlan` (AL#101);
//! one plan is built per call and reused across all frames. Every
//! frame is zero-padded up to the next power of two when the
//! analysis window is not already one. All products are
//! deterministic and persistable as `intermediate`-role tensors.

use al_locate::dsp::{mag_sq, scale};
use ruviform_numeric::{bands, stft as rf_stft, window::Window, xspec};

/// One frame×bin complex matrix produced by [`stft`].
#[derive(Debug, Clone)]
pub struct StftMatrix {
    /// Real part, `frames[frame][bin]` (bins = `fft_size`).
    pub re: Vec<Vec<f64>>,
    /// Imaginary part, same layout.
    pub im: Vec<Vec<f64>>,
    /// Analysis window length (samples, before padding).
    pub window: u64,
    /// Hop between successive frames (samples).
    pub hop: u64,
    /// FFT size (power of two, `>= window`).
    pub fft_size: usize,
    /// Sample rate of the source signal.
    pub sample_rate: u32,
}

impl StftMatrix {
    /// Number of frames.
    pub fn frames(&self) -> usize {
        self.re.len()
    }

    /// Bin center frequency (Hz) of `bin`.
    pub fn bin_freq(&self, bin: usize) -> f64 {
        bin as f64 * self.sample_rate as f64 / self.fft_size as f64
    }

    /// Squared window energy used for PSD normalization.
    fn window_power(&self) -> f64 {
        // Hann window energy = 3/8 * window; matches `hann()` below.
        0.375 * self.window as f64
    }

    /// Power spectral density, `frames[frame][bin]` in |x|²/Hz.
    /// One-sided interpretation is left to the caller — this is the
    /// full `fft_size`-bin transform.
    pub fn psd(&self) -> Vec<Vec<f64>> {
        // Two-sided PSD normalization: |X[k]|² / (fs · Σw²). Summing
        // all bins × bin width recovers the signal power.
        let norm = self.window_power() * self.sample_rate as f64;
        self.re
            .iter()
            .zip(&self.im)
            .map(|(re, im)| {
                // Two passes (mag_sq then scale): the primitives do
                // not allow logical in-place aliasing.
                let mut power = vec![0.0; re.len()];
                mag_sq(re, im, &mut power);
                let mut out = vec![0.0; re.len()];
                scale(&power, 1.0 / norm, &mut out);
                out
            })
            .collect()
    }

    /// Magnitude matrix.
    pub fn magnitude(&self) -> Vec<Vec<f64>> {
        self.re
            .iter()
            .zip(&self.im)
            .map(|(re, im)| {
                let mut m2 = vec![0.0; re.len()];
                mag_sq(re, im, &mut m2);
                m2.iter().map(|&v| v.sqrt()).collect()
            })
            .collect()
    }

    /// Power (squared magnitude) matrix.
    pub fn power(&self) -> Vec<Vec<f64>> {
        self.re
            .iter()
            .zip(&self.im)
            .map(|(re, im)| {
                let mut out = vec![0.0; re.len()];
                mag_sq(re, im, &mut out);
                out
            })
            .collect()
    }
}

/// Hann window coefficients (`ruviform_numeric::window::Window::Hann`)。
pub fn hann(n: usize) -> Vec<f64> {
    Window::Hann.coeffs(n)
}

/// Compute the complex STFT of `samples`.
///
/// Frames are Hann-windowed `window`-sample segments stepped by
/// `hop`, zero-padded to the next power of two. A signal shorter
/// than one window yields zero frames — callers decide the validity
/// (`InsufficientSupport`), not this function.
///
/// `sample_rate` is recorded on the matrix for `bin_freq`; it is not
/// validated here. `window == 0` / `hop == 0` yield zero frames —
/// the `StftPlan` construction is skipped before Ruviform's
/// validation can reject them (旧 `al-math` 経路と同じ不変条件)。
/// plan には `sample_rate.max(1)` を渡す: Ruviform は rate 0 を拒否
/// するが、旧経路は rate を計算に使わず行列へ記録するだけだった
/// ため、その不変条件を維持する。
pub fn stft(samples: &[f64], window: usize, hop: usize, sample_rate: u32) -> StftMatrix {
    let fft_size = window.next_power_of_two();
    let mut re = Vec::new();
    let mut im = Vec::new();
    if samples.len() >= window && window > 0 && hop > 0 {
        let plan = rf_stft::StftPlan::new(window, hop, Window::Hann, sample_rate.max(1))
            .expect("stft window/hop validated by caller contract");
        let mut workspace = rf_stft::StftWorkspace::new(&plan);
        for frame in 0..plan.frame_count(samples.len()) {
            let mut fre = vec![0.0; fft_size];
            let mut fim = vec![0.0; fft_size];
            plan.frame_into(&mut workspace, samples, frame, &mut fre, &mut fim)
                .expect("frame index is bounded by plan.frame_count");
            re.push(fre);
            im.push(fim);
        }
    }
    StftMatrix {
        re,
        im,
        window: window as u64,
        hop: hop as u64,
        fft_size,
        sample_rate,
    }
}

/// Integrate a power/PSD row into `[lo_hz, hi_hz)` bands. Returns one
/// value per band; bands outside the bin range contribute zero.
pub fn band_power(
    row: &[f64],
    sample_rate: u32,
    fft_size: usize,
    bands: &[(f64, f64)],
) -> Vec<f64> {
    bands::band_power(row, sample_rate as f64 / fft_size as f64, bands)
}

/// Standard IEC 1/3-octave bands inside `[f_min, f_max]` as
/// `(lower, upper)` edges around `f_c = 1000·2^(k/3)` centers.
pub fn third_octave_bands(f_min: f64, f_max: f64) -> Vec<(f64, f64)> {
    bands::third_octave_bands(f_min, f_max).expect("third-octave range must be positive")
}

/// Cross-spectral density of two aligned frames: `conj(x) * y`
/// per bin (returns `(re, im)` スライスのペア — Ruviform の
/// 分離レイアウトをそのまま引き継ぐ。長さ不一致は panic。
pub fn csd(x_re: &[f64], x_im: &[f64], y_re: &[f64], y_im: &[f64]) -> (Vec<f64>, Vec<f64>) {
    xspec::csd(x_re, x_im, y_re, y_im).expect("csd slice lengths must be equal")
}

/// Complex coherence `Pxy / sqrt(Pxx * Pyy)` per bin. Bins with zero
/// auto-power in either signal return `0`. 長さ不一致は panic。
pub fn complex_coherence(
    pxx: &[f64],
    pyy: &[f64],
    pxy_re: &[f64],
    pxy_im: &[f64],
) -> (Vec<f64>, Vec<f64>) {
    xspec::complex_coherence(pxx, pyy, pxy_re, pxy_im)
        .expect("coherence slice lengths must be equal")
}

/// Spectral envelope of a magnitude row: a smoothed log-frequency
/// curve (moving average in linear magnitude over `bins`-wide
/// windows), used for coarse shape comparison.
pub fn envelope(magnitude_row: &[f64], bins: usize) -> Vec<f64> {
    rf_stft::envelope(magnitude_row, bins)
}

#[cfg(test)]
mod tests {
    use super::*;
    // The bit-exact oracle below intentionally transforms each frame
    // through the single-shot `dsp::fft` path (pre-hoist behavior).
    use al_locate::dsp::fft;

    #[test]
    fn stft_places_sine_in_the_right_bin() {
        let rate = 48000;
        let n = 2048;
        let freq = 3000.0;
        let samples: Vec<f64> = (0..n)
            .map(|i| (2.0 * std::f64::consts::PI * freq * i as f64 / rate as f64).sin())
            .collect();
        let m = stft(&samples, 1024, 512, rate);
        assert_eq!(m.frames(), 3);
        assert_eq!(m.fft_size, 1024);
        let mag = m.magnitude();
        let (bin, _) = mag[0]
            .iter()
            // 実数信号は負周波数にも同じピークを持つ。
            // 微小な丸め差で共役側を選ばないよう非負周波数を調べる。
            .take(m.fft_size / 2 + 1)
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap();
        // 3000 Hz at 46.875 Hz/bin → bins 64/65 both close.
        assert!((bin as i64 - 64).abs() <= 1);
    }

    #[test]
    fn non_power_of_two_window_is_zero_padded() {
        let samples = vec![1.0; 1500];
        let m = stft(&samples, 1500, 500, 48000);
        assert_eq!(m.fft_size, 2048);
        assert_eq!(m.frames(), 1);
    }

    #[test]
    fn psd_integrates_to_signal_power() {
        // Parseval sanity: a unit sine's total power is 0.5.
        let rate = 48000;
        let n = 2048;
        let samples: Vec<f64> = (0..n)
            .map(|i| (2.0 * std::f64::consts::PI * 1000.0 * i as f64 / rate as f64).sin())
            .collect();
        let m = stft(&samples, 2048, 2048, rate);
        let psd = m.psd();
        let bin_hz = rate as f64 / m.fft_size as f64;
        let total: f64 = psd[0].iter().sum::<f64>() * bin_hz;
        assert!((total - 0.5).abs() < 0.15, "total power {total}");
    }

    #[test]
    fn third_octave_bands_cover_range() {
        let bands = third_octave_bands(20.0, 20000.0);
        assert!(bands.len() > 25);
        // Band edges straddle the range: first band starts at or
        // below ~20 Hz, last band reaches the top of the range.
        assert!(bands[0].0 <= 20.0 && bands[0].0 > 10.0);
        assert!(bands.last().unwrap().1 > 16_000.0);
    }

    // ==== A3 characterization (caller-side optimizations,
    // docs/test-specs/caller-side-optimizations.md Rule B) ====
    //
    // `stft` output must be bit-exact (`f64::to_bits`) against the
    // reference that windows each frame and transforms it via the
    // single-shot `dsp::fft` path — the pre-plan-hoist behavior.
    // The test stays green after the A3 `Fft` plan hoist.

    /// Deterministic fixture: sine mix + LCG noise.
    fn fixture_samples(n: usize, rate: u32) -> Vec<f64> {
        let mut s = 0x9e3779b9u64;
        (0..n)
            .map(|i| {
                s = s
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let noise = ((s >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0;
                let t = i as f64 / rate as f64;
                (2.0 * std::f64::consts::PI * 1000.0 * t).sin()
                    + 0.3 * (2.0 * std::f64::consts::PI * 3000.0 * t).sin()
                    + 0.1 * noise
            })
            .collect()
    }

    /// Reference: each frame independently windowed + single-shot
    /// `dsp::fft`, returning `(re, im)` per frame.
    fn reference_frames(samples: &[f64], window: usize, hop: usize) -> Vec<(Vec<f64>, Vec<f64>)> {
        let fft_size = window.next_power_of_two();
        let win = hann(window);
        let mut out = Vec::new();
        if samples.len() >= window && window > 0 && hop > 0 {
            for start in (0..=samples.len() - window).step_by(hop) {
                let mut re = vec![0.0; fft_size];
                let mut im = vec![0.0; fft_size];
                for (i, &v) in samples[start..start + window].iter().enumerate() {
                    re[i] = v * win[i];
                }
                fft(&mut re, &mut im, false);
                out.push((re, im));
            }
        }
        out
    }

    fn assert_bit_exact(m: &StftMatrix, samples: &[f64], window: usize, hop: usize) {
        let reference = reference_frames(samples, window, hop);
        assert_eq!(m.frames(), reference.len(), "frame count");
        assert_eq!(m.fft_size, window.next_power_of_two(), "fft_size");
        assert_eq!(m.window, window as u64, "window");
        assert_eq!(m.hop, hop as u64, "hop");
        for (f, (re, im)) in reference.iter().enumerate() {
            for k in 0..m.fft_size {
                assert_eq!(m.re[f][k].to_bits(), re[k].to_bits(), "re[{f}][{k}]");
                assert_eq!(m.im[f][k].to_bits(), im[k].to_bits(), "im[{f}][{k}]");
            }
        }
    }

    #[test]
    fn stft_is_bit_exact_multiple_frames() {
        let rate = 48_000;
        let samples = fixture_samples(8192, rate);
        let m = stft(&samples, 1024, 512, rate);
        assert!(m.frames() > 1);
        assert_bit_exact(&m, &samples, 1024, 512);
    }

    #[test]
    fn stft_is_bit_exact_zero_frames() {
        let rate = 48_000;
        let samples = fixture_samples(500, rate);
        let m = stft(&samples, 1024, 512, rate);
        assert_eq!(m.frames(), 0);
        assert_bit_exact(&m, &samples, 1024, 512);
    }

    #[test]
    fn stft_is_bit_exact_single_frame() {
        let rate = 48_000;
        // window <= n < window + hop → exactly one frame.
        let samples = fixture_samples(1500, rate);
        let m = stft(&samples, 1024, 512, rate);
        assert_eq!(m.frames(), 1);
        assert_bit_exact(&m, &samples, 1024, 512);
    }

    #[test]
    fn stft_is_bit_exact_non_power_of_two_window() {
        let rate = 48_000;
        let samples = fixture_samples(2000, rate);
        let m = stft(&samples, 1500, 500, rate);
        assert_eq!(m.fft_size, 2048);
        assert_bit_exact(&m, &samples, 1500, 500);
    }

    #[test]
    fn stft_is_bit_exact_all_zero() {
        let rate = 48_000;
        let samples = vec![0.0; 4096];
        let m = stft(&samples, 1024, 512, rate);
        assert!(m.frames() > 0);
        assert_bit_exact(&m, &samples, 1024, 512);
    }

    #[test]
    fn coherence_is_one_for_identical_signals() {
        let rate = 48000;
        let n = 1024;
        let samples: Vec<f64> = (0..n)
            .map(|i| (2.0 * std::f64::consts::PI * 700.0 * i as f64 / rate as f64).sin())
            .collect();
        let m = stft(&samples, 1024, 1024, rate);
        let p = m.power();
        let (pxy_re, pxy_im) = csd(&m.re[0], &m.im[0], &m.re[0], &m.im[0]);
        let (coh_re, coh_im) = complex_coherence(&p[0], &p[0], &pxy_re, &pxy_im);
        let (bin, _) = p[0]
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap();
        assert!((coh_re[bin] - 1.0).abs() < 1e-9);
        assert!(coh_im[bin].abs() < 1e-9);
    }
}
