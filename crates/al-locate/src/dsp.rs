//! DSP primitives for source localization.
//!
//! Everything here is self-implemented (no external DSP crates, per
//! the implementation-stack ADR). All signals are `f64` slices in
//! whatever native scale the caller uses — the correlation kernels
//! are scale-invariant; callers that compare absolute quantities must
//! normalize themselves first.

#[cfg(test)]
use std::f64::consts::PI;

/// Split interleaved samples into per-channel contiguous vectors.
///
/// Always returns `channels` entries (each possibly empty). A
/// trailing partial frame is dropped; `channels == 0` returns an
/// empty vector.
pub fn deinterleave(interleaved: &[f64], channels: u16) -> Vec<Vec<f64>> {
    let channels = usize::from(channels);
    if channels == 0 {
        return Vec::new();
    }
    let frames = interleaved.len() / channels;
    let mut out = vec![Vec::with_capacity(frames); channels];
    for frame in interleaved.chunks_exact(channels) {
        for (ch, &v) in frame.iter().enumerate() {
            out[ch].push(v);
        }
    }
    out
}

pub use al_math::correlation::{gcc_phat, ncc};
pub use al_math::fft::Fft;
pub use al_math::filter::{bandpass, bandpass_many, spectral_envelope};
pub use al_math::vec::{axpy, corr0, dot, mag_sq, rms, scale, sub_scaled, sum_sq};

/// In-place FFT (fused radix-4 stages, large transforms run on a skewed
/// workspace). `re`/`im` lengths must be equal powers of two.
pub fn fft(re: &mut [f64], im: &mut [f64], inverse: bool) {
    let plan = al_math::fft::Fft::new(re.len());
    plan.transform(re, im, inverse);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deinterleave_splits_channels() {
        let out = deinterleave(&[1.0, 10.0, 2.0, 20.0, 3.0, 30.0], 2);
        assert_eq!(out, vec![vec![1.0, 2.0, 3.0], vec![10.0, 20.0, 30.0]]);
    }

    #[test]
    fn ncc_finds_known_offset() {
        let signal: Vec<f64> = (0..64).map(|i| (i as f64 * 0.7).sin()).collect();
        let template = signal[20..36].to_vec();
        let corr = ncc(&template, &signal, 63);
        let best = corr
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap()
            .0;
        assert_eq!(best, 20);
        assert!((corr[20] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn gcc_phat_finds_known_offset() {
        // Deterministic white noise (LCG): the true lag dominates.
        let mut state = 0x9E3779B9u64;
        let mut lcg = move || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0
        };
        let signal: Vec<f64> = (0..512).map(|_| lcg()).collect();
        let template = signal[200..264].to_vec();
        let corr = gcc_phat(&template, &signal, 447);
        let best = corr
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap()
            .0;
        assert_eq!(best, 200);
    }

    #[test]
    fn bandpass_isolates_band() {
        let n = 256;
        let sig: Vec<f64> = (0..n)
            .map(|i| {
                let t = i as f64 / 256.0;
                (2.0 * PI * 10.0 * t).sin() + (2.0 * PI * 100.0 * t).sin()
            })
            .collect();
        let lo = bandpass(&sig, 256, 5.0, 20.0);
        let e_lo: f64 = lo.iter().map(|v| v * v).sum();
        let hi = bandpass(&sig, 256, 80.0, 120.0);
        let e_hi: f64 = hi.iter().map(|v| v * v).sum();
        // Each band keeps roughly half the total energy.
        assert!(e_lo > e_hi * 0.5 && e_hi > e_lo * 0.5);
    }

    #[test]
    fn least_squares_recovers_coefficients() {
        // y = 2a - b + noise-free.
        let c0: Vec<f64> = (0..20).map(|i| i as f64 * 0.1).collect();
        let c1: Vec<f64> = (0..20).map(|i| (i as f64 * 0.37).sin()).collect();
        let rhs: Vec<f64> = (0..20).map(|i| 2.0 * c0[i] - c1[i]).collect();
        let (x, rms) =
            al_math::linalg::least_squares(&[c0.as_slice(), c1.as_slice()], &rhs, 0.0).unwrap();
        assert!((x[0] - 2.0).abs() < 1e-6);
        assert!((x[1] + 1.0).abs() < 1e-6);
        assert!(rms < 1e-6);
    }

    #[test]
    fn condition_number_reports_singular() {
        assert_eq!(
            al_math::linalg::condition_number(&[vec![1.0, 2.0], vec![2.0, 4.0]]),
            f64::INFINITY
        );
        let good = al_math::linalg::condition_number(&[vec![2.0, 0.0], vec![0.0, 1.0]]);
        assert!((good - 2.0).abs() < 0.2);
    }
}
