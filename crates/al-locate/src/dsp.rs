//! DSP primitives for source localization.
//!
//! Everything here is self-implemented (no external DSP crates, per
//! the implementation-stack ADR). All signals are `f64` slices in
//! whatever native scale the caller uses — the correlation kernels
//! are scale-invariant; callers that compare absolute quantities must
//! normalize themselves first.

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

/// Normalized cross-correlation of `template` inside `signal`.
///
/// Returns one coefficient in `[-1, 1]` per lag `0..=max_lag` where the
/// template fits entirely. A lag with zero signal energy returns 0 so
/// silence cannot fake a correlation peak — callers mark it as
/// `SilenceInsufficientEnergy` instead.
pub fn ncc(template: &[f64], signal: &[f64], max_lag: usize) -> Vec<f64> {
    let n = template.len();
    if n == 0 || signal.len() < n {
        return Vec::new();
    }
    let max_lag = max_lag.min(signal.len() - n);
    let t_energy: f64 = template.iter().map(|v| v * v).sum();
    let t_norm = t_energy.sqrt();
    if t_norm == 0.0 {
        return vec![0.0; max_lag + 1];
    }
    // Cross-correlation numerator via FFT (same layout as gcc_phat):
    // corr[lag] = sum_j template[j] * signal[lag + j].
    let size = (signal.len() + n - 1).next_power_of_two();
    let mut re = vec![0.0; size];
    let mut im = vec![0.0; size];
    let mut re_s = vec![0.0; size];
    let mut im_s = vec![0.0; size];
    re[..n].copy_from_slice(template);
    re_s[..signal.len()].copy_from_slice(signal);
    fft(&mut re, &mut im, false);
    fft(&mut re_s, &mut im_s, false);
    for k in 0..size {
        let pr = re[k] * re_s[k] + im[k] * im_s[k];
        let pi = re[k] * im_s[k] - im[k] * re_s[k];
        re[k] = pr;
        im[k] = pi;
    }
    fft(&mut re, &mut im, true);
    let corr = re;
    // Sliding signal energy via cumulative squares.
    let mut cum = vec![0.0; signal.len() + 1];
    for (i, &v) in signal.iter().enumerate() {
        cum[i + 1] = cum[i] + v * v;
    }
    (0..=max_lag)
        .map(|lag| {
            let s_energy = (cum[lag + n] - cum[lag]).max(0.0);
            if s_energy == 0.0 {
                return 0.0;
            }
            corr[lag] / (t_norm * s_energy.sqrt())
        })
        .collect()
}

/// In-place radix-2 FFT. `re`/`im` lengths must be equal powers of two.
pub fn fft(re: &mut [f64], im: &mut [f64], inverse: bool) {
    let n = re.len();
    assert_eq!(n, im.len());
    assert!(n.is_power_of_two());
    // Bit-reversal permutation.
    for i in 1..n {
        let j = i.reverse_bits() >> (usize::BITS - n.trailing_zeros());
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let sign = if inverse { 1.0 } else { -1.0 };
    let mut len = 2;
    while len <= n {
        let ang = sign * 2.0 * PI / len as f64;
        let (wr, wi) = (ang.cos(), ang.sin());
        for i in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0, 0.0);
            for j in i..i + len / 2 {
                let k = j + len / 2;
                let (tr, ti) = (re[k] * cr - im[k] * ci, re[k] * ci + im[k] * cr);
                re[k] = re[j] - tr;
                im[k] = im[j] - ti;
                re[j] += tr;
                im[j] += ti;
                let ncr = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = ncr;
            }
        }
        len *= 2;
    }
    if inverse {
        for v in re.iter_mut().chain(im.iter_mut()) {
            *v /= n as f64;
        }
    }
}

/// GCC-PHAT generalized cross-correlation: whitened spectrum of
/// `template` against `signal`, returned as a correlation value per
/// lag `0..=max_lag`. The FFT size is the next power of two ≥
/// `signal.len() + template.len() - 1` to avoid circular wrap.
pub fn gcc_phat(template: &[f64], signal: &[f64], max_lag: usize) -> Vec<f64> {
    if template.is_empty() || signal.len() < template.len() {
        return Vec::new();
    }
    let needed = signal.len() + template.len() - 1;
    let n = needed.next_power_of_two();
    let mut re_a = vec![0.0; n];
    let mut re_b = vec![0.0; n];
    let mut im_a = vec![0.0; n];
    let mut im_b = vec![0.0; n];
    re_a[..template.len()].copy_from_slice(template);
    re_b[..signal.len()].copy_from_slice(signal);
    fft(&mut re_a, &mut im_a, false);
    fft(&mut re_b, &mut im_b, false);
    // PHAT cross-spectrum R = A* · B / |A* · B|, A = template.
    let mut cr = vec![0.0; n];
    let mut ci = vec![0.0; n];
    for k in 0..n {
        let pr = re_a[k] * re_b[k] + im_a[k] * im_b[k];
        let pi = re_a[k] * im_b[k] - im_a[k] * re_b[k];
        let mag = (pr * pr + pi * pi).sqrt();
        if mag > 0.0 {
            cr[k] = pr / mag;
            ci[k] = pi / mag;
        }
    }
    fft(&mut cr, &mut ci, true);
    let max_lag = max_lag.min(signal.len() - template.len());
    cr[..=max_lag].to_vec()
}

/// One-pole smoothing spectral envelope (log magnitude) of `samples`.
///
/// Returns `bins` log-magnitude values; used by the spectral-envelope
/// locate method. Absolute-level sensitive — normalize inputs to a
/// common scale before comparing envelopes across inputs.
pub fn spectral_envelope(samples: &[f64], fft_size: usize, bins: usize) -> Vec<f64> {
    let n = fft_size.max(samples.len().next_power_of_two());
    let mut re = vec![0.0; n];
    let mut im = vec![0.0; n];
    re[..samples.len()].copy_from_slice(samples);
    fft(&mut re, &mut im, false);
    let half = n / 2;
    let mut out = Vec::with_capacity(bins);
    for b in 0..bins {
        let lo = b * half / bins;
        let hi = ((b + 1) * half / bins).max(lo + 1);
        let mut acc = 0.0;
        for k in lo..hi.min(half) {
            acc += (re[k] * re[k] + im[k] * im[k]).sqrt();
        }
        let mean = acc / (hi.min(half) - lo) as f64;
        out.push((mean + 1e-12).ln());
    }
    out
}

/// Zero-phase FIR bandpass via windowed ideal filter (spectral mask
/// approach): FFT, zero bins outside `[lo_hz, hi_hz)`, inverse FFT.
pub fn bandpass(samples: &[f64], sample_rate: u32, lo_hz: f64, hi_hz: f64) -> Vec<f64> {
    let n = samples.len().next_power_of_two();
    if n == 0 {
        return Vec::new();
    }
    let mut re = vec![0.0; n];
    let mut im = vec![0.0; n];
    re[..samples.len()].copy_from_slice(samples);
    fft(&mut re, &mut im, false);
    let bin_hz = f64::from(sample_rate) / n as f64;
    for k in 0..n {
        // Mirror-bin frequency for the second half of the spectrum.
        let f = bin_hz * (k.min(n - k)) as f64;
        if f < lo_hz || f >= hi_hz {
            re[k] = 0.0;
            im[k] = 0.0;
        }
    }
    fft(&mut re, &mut im, true);
    re[..samples.len()].to_vec()
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
        let rows: Vec<Vec<f64>> = (0..20)
            .map(|i| vec![i as f64 * 0.1, (i as f64 * 0.37).sin()])
            .collect();
        let rhs: Vec<f64> = rows.iter().map(|r| 2.0 * r[0] - r[1]).collect();
        let (x, rms) = al_math::linalg::least_squares(&rows, &rhs, 0.0).unwrap();
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
