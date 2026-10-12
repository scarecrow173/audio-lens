//! Mixer fitting: per-channel least-squares estimation of the
//! source→capture mixing matrix.
//!
//! Signed coefficients (a polarity-inverted channel comes out as a
//! negative coefficient), ridge regularization, fit quality, and the
//! condition number are all recorded — the matrix is what residual
//! subtraction uses, so it is persisted, not just the gain.

use serde::{Deserialize, Serialize};

use al_math::linalg::{LeastSquares, condition_number};

use crate::dsp;

/// Estimated mixing matrix plus fit diagnostics.
///
/// `matrix[capture_ch][source_ch]` — `capture_ch ≈ Σ_s matrix[c][s] *
/// source_s`. Coefficients are signed: polarity inversion appears as
/// a negative entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MixerEstimate {
    /// `matrix[capture][source]` signed coefficients.
    pub matrix: Vec<Vec<f64>>,
    /// Ridge parameter actually used.
    pub ridge: f64,
    /// Residual RMS relative to capture RMS (`0` = perfect fit).
    pub fit_quality: f64,
    /// Condition number of the normal matrix (high = ill-posed).
    pub condition_number: f64,
}

/// Estimate the mixing matrix mapping `source_channels` onto each of
/// `capture_channels` over an aligned window.
///
/// `source_channels`/`capture_channels` are deinterleaved; each
/// channel must share the same length. Returns `None` when there is
/// not enough data to solve (fewer observations than sources).
pub fn fit_mix(
    source_channels: &[Vec<f64>],
    capture_channels: &[Vec<f64>],
    ridge: f64,
) -> Option<MixerEstimate> {
    let n_src = source_channels.len();
    if n_src == 0 || capture_channels.is_empty() {
        return None;
    }
    let len = source_channels[0].len();
    if source_channels.iter().any(|c| c.len() != len)
        || capture_channels.iter().any(|c| c.len() != len)
        || len < n_src
    {
        return None;
    }
    // Observations: each sample index is a row of source values.
    let rows: Vec<Vec<f64>> = (0..len)
        .map(|t| (0..n_src).map(|s| source_channels[s][t]).collect())
        .collect();

    // Condition number of the raw normal matrix (before ridge).
    let mut ata = vec![vec![0.0; n_src]; n_src];
    for row in &rows {
        for (i, &ri) in row.iter().enumerate() {
            for (j, &rj) in row.iter().enumerate() {
                ata[i][j] += ri * rj;
            }
        }
    }
    let cond = condition_number(&ata);
    let factor = LeastSquares::new(&rows, ridge)?;

    let mut matrix = Vec::with_capacity(capture_channels.len());
    let mut quality_acc = 0.0;
    for cap in capture_channels {
        let (coeffs, residual_rms) = factor.solve(cap)?;
        matrix.push(coeffs);
        let cap_rms = dsp::rms(cap);
        quality_acc += if cap_rms > 0.0 {
            residual_rms / cap_rms
        } else {
            0.0
        };
    }
    Some(MixerEstimate {
        matrix,
        ridge,
        fit_quality: quality_acc / capture_channels.len() as f64,
        condition_number: cond,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noise(seed: u64, n: usize) -> Vec<f64> {
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

    #[test]
    fn recovers_signed_mixing_matrix() {
        let s0 = noise(1, 4096);
        let s1 = noise(2, 4096);
        // cap0 = 0.8 s0 - 0.5 s1 (polarity inversion), cap1 = 0.3 s1.
        let cap0: Vec<f64> = s0
            .iter()
            .zip(&s1)
            .map(|(&a, &b)| 0.8 * a - 0.5 * b)
            .collect();
        let cap1: Vec<f64> = s1.iter().map(|&b| 0.3 * b).collect();
        let est = fit_mix(&[s0, s1], &[cap0, cap1], 1e-3).unwrap();
        assert!((est.matrix[0][0] - 0.8).abs() < 0.05);
        assert!((est.matrix[0][1] + 0.5).abs() < 0.05); // negative = inverted
        assert!(est.matrix[1][0].abs() < 0.05);
        assert!((est.matrix[1][1] - 0.3).abs() < 0.05);
        assert!(est.fit_quality < 0.1);
    }

    #[test]
    fn reports_ill_conditioned_mix() {
        let s0 = noise(3, 2048);
        let dup = s0.clone(); // two identical source channels → singular
        let cap = s0.clone();
        let est = fit_mix(&[s0, dup], &[cap], 1e-6).unwrap();
        assert_eq!(est.condition_number, f64::INFINITY);
    }

    #[test]
    fn correlated_mix_recovers_coefficients_and_preserves_raw_diagnostic() {
        let a = noise(11, 1024);
        let b: Vec<f64> = a
            .iter()
            .zip(noise(12, 1024))
            .map(|(&x, y)| x + 1e-7 * y)
            .collect();
        let cap0: Vec<f64> = a.iter().zip(&b).map(|(a, b)| 2.0 * a - b).collect();
        let cap1: Vec<f64> = a.iter().zip(&b).map(|(a, b)| -0.5 * a + 3.0 * b).collect();
        let sources = [a, b];
        let captures = [cap0, cap1];
        let est = fit_mix(&sources, &captures, 0.0).unwrap();
        for (actual, expected) in est.matrix.iter().flatten().zip([2.0, -1.0, -0.5, 3.0]) {
            assert!((actual - expected).abs() < 1e-7);
        }
        assert!(est.fit_quality < 1e-14);
        // 診断は従来と同じ ridge 前の正規行列を使用し、ridge に依存しない。
        let regularized = fit_mix(&sources, &captures, 1e-3).unwrap();
        assert_eq!(est.condition_number, regularized.condition_number);
        let mut ata = vec![vec![0.0; 2]; 2];
        for (&a, &b) in sources[0].iter().zip(&sources[1]) {
            let row = [a, b];
            for (i, &ri) in row.iter().enumerate() {
                for (j, &rj) in row.iter().enumerate() {
                    ata[i][j] += ri * rj;
                }
            }
        }
        assert_eq!(est.condition_number, condition_number(&ata));
    }
}
