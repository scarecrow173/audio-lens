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
    // Channel-block strided gather: each channel row is written
    // contiguously. `chunks_exact` drops a trailing partial frame,
    // same as the element-loop form.
    (0..channels)
        .map(|ch| {
            interleaved
                .chunks_exact(channels)
                .map(|frame| frame[ch])
                .collect()
        })
        .collect()
}

pub use al_math::correlation::{CorrelationPlan, CorrelationWorkspace, gcc_phat, ncc};
pub use al_math::fft::Fft;
pub use al_math::filter::{bandpass, bandpass_many, spectral_envelope};
pub use al_math::vec::{axpy, corr0, dot, mag_sq, rms, scale, sub_scaled, sum_sq};

/// In-place FFT (fused radix-4 stages, large transforms run on a skewed
/// workspace). `re`/`im` lengths must be equal powers of two.
pub fn fft(re: &mut [f64], im: &mut [f64], inverse: bool) {
    let plan = al_math::fft::Fft::new(re.len());
    plan.transform(re, im, inverse);
}

/// Overlap-save convolution on `al_math::fft::RealFft`: returns the
/// first `signal.len()` samples of the linear convolution. An empty
/// `kernel` yields a `signal`-length zero vector and an empty
/// `signal` yields an empty vector — both early returns must precede
/// the block-length computation (`m == 0` would underflow `m - 1`).
pub fn conv(signal: &[f64], kernel: &[f64]) -> Vec<f64> {
    if kernel.is_empty() {
        return vec![0.0; signal.len()];
    }
    if signal.is_empty() {
        return Vec::new();
    }
    let n = signal.len();
    let m = kernel.len();
    // Power-of-two block length (`RealFft::new` contract), bounded to
    // 8192 for m <= 4096; the 1024 floor amortizes per-block call
    // overhead for small kernels. Each block emits `l` fresh outputs.
    let b = (2 * m).next_power_of_two().max(1024);
    let l = b - m + 1;
    let bins = b / 2 + 1;
    let mut plan = al_math::fft::RealFft::new(b);
    // Kernel spectrum, zero-padded to `b` and transformed once.
    let mut hbuf = vec![0.0; b];
    hbuf[..m].copy_from_slice(kernel);
    let mut hre = vec![0.0; bins];
    let mut him = vec![0.0; bins];
    plan.forward(&hbuf, &mut hre, &mut him);
    let mut xbuf = vec![0.0; b];
    let mut xre = vec![0.0; bins];
    let mut xim = vec![0.0; bins];
    let mut obuf = vec![0.0; b];
    let mut y = vec![0.0; n];
    for off in (0..n).step_by(l) {
        let take = l.min(n - off);
        // Carry the previous block's last m-1 inputs to the front
        // (zeros on the first pass), append `take` fresh samples,
        // zero-pad the tail.
        xbuf.copy_within(b + 1 - m.., 0);
        xbuf[m - 1..m - 1 + take].copy_from_slice(&signal[off..off + take]);
        xbuf[m - 1 + take..].fill(0.0);
        plan.forward(&xbuf, &mut xre, &mut xim);
        for k in 0..bins {
            let zre = xre[k] * hre[k] - xim[k] * him[k];
            let zim = xre[k] * him[k] + xim[k] * hre[k];
            xre[k] = zre;
            xim[k] = zim;
        }
        // `RealFft::inverse` already applies the 1/N normalization.
        plan.inverse(&xre, &xim, &mut obuf);
        // The first m-1 outputs carry circular wrap-around: discard.
        y[off..off + take].copy_from_slice(&obuf[m - 1..m - 1 + take]);
    }
    y
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic zero-mean pseudo-noise (LCG, same generator
    /// shape as `reference.rs` tests and `benches/conv.rs`).
    fn noise(n: usize, seed: u64) -> Vec<f64> {
        let mut s = seed;
        (0..n)
            .map(|_| {
                s = s
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ((s >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0
            })
            .collect()
    }

    /// Direct linear convolution returning the first `x.len()`
    /// outputs — reference implementation for A/B against `conv`.
    /// Verbatim copy shared by hand with `reference.rs` tests and
    /// `benches/conv.rs` (not a shared public surface). The `x ==
    /// 0.0` skip of the former production loop is dropped on
    /// purpose: zero terms contribute `±0.0` and are
    /// value-equivalent.
    fn conv_direct(x: &[f64], h: &[f64]) -> Vec<f64> {
        let mut y = vec![0.0; x.len()];
        for (i, &xv) in x.iter().enumerate() {
            let maxk = h.len().min(x.len() - i);
            for (k, &hv) in h.iter().take(maxk).enumerate() {
                y[i + k] += xv * hv;
            }
        }
        y
    }

    /// `conv` must equal `conv_direct` under
    /// `|a-b| <= 1e-12*max(1,|a|,|b|)`
    /// (ADR-034EoHxzsvsHp9BOdCK2Cn).
    #[test]
    fn conv_matches_direct_reference() {
        fn assert_equiv(signal: &[f64], kernel: &[f64]) {
            let actual = conv(signal, kernel);
            let expected = conv_direct(signal, kernel);
            assert_eq!(
                actual.len(),
                expected.len(),
                "length {} vs {}",
                actual.len(),
                expected.len()
            );
            for (i, (&a, &b)) in actual.iter().zip(&expected).enumerate() {
                let tol = 1e-12 * 1.0_f64.max(a.abs()).max(b.abs());
                assert!((a - b).abs() <= tol, "index {i}: {a} vs {b} (tol {tol})");
            }
        }

        // Documented edge cases: an empty kernel yields a
        // signal-length zero vector and an empty signal yields an
        // empty vector (both before block-length computation).
        assert_eq!(conv(&noise(64, 1), &[]), vec![0.0; 64]);
        assert!(conv(&[], &noise(7, 2)).is_empty());

        let cases: Vec<(Vec<f64>, Vec<f64>)> = vec![
            // Both empty / empty kernel through the shared predicate.
            (vec![], vec![]),
            (noise(64, 3), vec![]),
            // n < m.
            (noise(100, 5), noise(256, 6)),
            // n = 1.
            (noise(1, 7), noise(256, 8)),
            // m = 1 (L = 1024); n is not a multiple of L.
            (noise(4_099, 9), noise(1, 10)),
            // m = 3 (L = 1022); n is not a multiple of L.
            (noise(12_345, 11), noise(3, 12)),
            // m = 256 → B = 1024, L = 769; n spans many blocks + tail.
            (noise(65_549, 13), noise(256, 14)),
            // m = 4096 → B = 8192, L = 4097; n = 4L + partial tail.
            (noise(20_000, 15), noise(4_096, 16)),
            // 1M-scale signal with a small kernel (direct stays
            // cheap enough for a debug-build test).
            (noise(1_048_576, 17), noise(8, 18)),
            // All-zero signal / all-zero kernel.
            (vec![0.0; 2_048], noise(256, 19)),
            (noise(2_048, 20), vec![0.0; 300]),
            // Deterministic non-noise fixtures.
            (
                (0..5_000).map(|i| (i as f64 * 0.11).sin()).collect(),
                (0..100).map(|i| 1.0 / (i as f64 + 1.0)).collect(),
            ),
        ];
        for (signal, kernel) in &cases {
            assert_equiv(signal, kernel);
        }
    }

    #[test]
    fn deinterleave_splits_channels() {
        let out = deinterleave(&[1.0, 10.0, 2.0, 20.0, 3.0, 30.0], 2);
        assert_eq!(out, vec![vec![1.0, 2.0, 3.0], vec![10.0, 20.0, 30.0]]);
    }

    // ==== mechanical-caller-cleanup characterization (A15) ====
    // TASK-034EO484s25JM8bsb6zUCh (plan T1). Pins the CURRENT
    // element/byte-level contract of `deinterleave` — frame-major
    // element order, `channels` result entries, dropped trailing
    // partial frame, `channels == 0` → empty — bit-exact via
    // `f64::to_bits` (test-spec Rules C/D).

    /// Reference shape: `out[c][i] = interleaved[i*ch + c]` for
    /// whole frames only.
    fn deinterleave_oracle(interleaved: &[f64], channels: usize) -> Vec<Vec<f64>> {
        if channels == 0 {
            return Vec::new();
        }
        let frames = interleaved.len() / channels;
        (0..channels)
            .map(|c| (0..frames).map(|i| interleaved[i * channels + c]).collect())
            .collect()
    }

    fn assert_deinterleave(interleaved: &[f64], channels: u16, what: &str) {
        let out = deinterleave(interleaved, channels);
        let expected = deinterleave_oracle(interleaved, usize::from(channels));
        assert_eq!(out.len(), expected.len(), "{what} channel count");
        for (c, (a, e)) in out.iter().zip(&expected).enumerate() {
            assert_eq!(a.len(), e.len(), "{what} ch{c} len");
            for (i, (&x, &y)) in a.iter().zip(e).enumerate() {
                assert_eq!(x.to_bits(), y.to_bits(), "{what} ch{c}[{i}]");
            }
        }
    }

    #[test]
    fn a15_deinterleave_element_order_and_edges() {
        // 1/2/6-channel frame-major order over noise (exact f64
        // element equality, not just round-trip shape).
        for channels in [1u16, 2, 6] {
            let frames = 97usize;
            let buf = noise(frames * channels as usize, 0xd515 + channels as u64);
            assert_deinterleave(&buf, channels, "noise");
        }
        // Trailing partial frame is dropped from every channel.
        let mut buf = noise(2 * 40, 0x7a11);
        buf.extend_from_slice(&noise(3, 0x7a12)); // 3 < channels
        assert_deinterleave(&buf, 4, "trailing_partial");
        // `channels == 0` → empty vector (not a panic).
        assert!(deinterleave(&noise(8, 1), 0).is_empty());
        // Empty input → one empty vector per channel.
        let out = deinterleave(&[], 3);
        assert_eq!(out, vec![Vec::<f64>::new(); 3]);
    }

    #[test]
    fn a15_deinterleave_byte_layout_roundtrip() {
        // Byte-exact framing: an interleaved little-endian f64
        // stream (the `.f64le` layout normalize.rs writes) splits
        // back into the same per-channel sequences bit-for-bit.
        let chans: Vec<Vec<f64>> = vec![noise(33, 0xb1), noise(33, 0xb2)];
        let interleaved: Vec<f64> = (0..33)
            .flat_map(|i| chans.iter().map(move |c| c[i]))
            .collect();
        let bytes: Vec<u8> = interleaved.iter().flat_map(|v| v.to_le_bytes()).collect();
        let parsed: Vec<f64> = bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|b| f64::from_le_bytes(*b))
            .collect();
        let out = deinterleave(&parsed, 2);
        for (c, (a, e)) in out.iter().zip(&chans).enumerate() {
            for (i, (&x, &y)) in a.iter().zip(e).enumerate() {
                assert_eq!(x.to_bits(), y.to_bits(), "byte_layout ch{c}[{i}]");
            }
        }
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
        let signal = noise(512, 0x9E3779B9);
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
