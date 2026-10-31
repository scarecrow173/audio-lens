//! Mixer fitting: per-channel least-squares estimation of the
//! source→capture mixing matrix.
//!
//! Signed coefficients (a polarity-inverted channel comes out as a
//! negative coefficient), ridge regularization, fit quality, and the
//! condition number are all recorded — the matrix is what residual
//! subtraction uses, so it is persisted, not just the gain.

use serde::{Deserialize, Serialize};

use ruviform_numeric::linalg::{LeastSquares, condition_number};
use ruviform_numeric::vec;

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
    source_channels: &[&[f64]],
    capture_channels: &[&[f64]],
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

    // Condition number of the raw normal matrix (before ridge),
    // accumulated on the contiguous column buffers.
    let mut ata = vec![vec![0.0; n_src]; n_src];
    for (i, ci) in source_channels.iter().enumerate() {
        for (j, cj) in source_channels.iter().enumerate() {
            ata[i][j] = vec::dot(ci, cj).expect("channel lengths verified above");
        }
    }
    let cond = condition_number(&ata);
    let factor = LeastSquares::from_columns(source_channels, ridge)?;

    let mut matrix = Vec::with_capacity(capture_channels.len());
    let mut quality_acc = 0.0;
    for cap in capture_channels {
        let (coeffs, residual_rms) = factor.solve(cap)?;
        matrix.push(coeffs);
        let cap_rms = vec::rms(cap);
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
        let est = fit_mix(
            &[s0.as_slice(), s1.as_slice()],
            &[cap0.as_slice(), cap1.as_slice()],
            1e-3,
        )
        .unwrap();
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
        let est = fit_mix(&[s0.as_slice(), dup.as_slice()], &[cap.as_slice()], 1e-6).unwrap();
        assert_eq!(est.condition_number, f64::INFINITY);
    }

    // ==== A4 characterization (caller-side optimizations,
    // docs/test-specs/caller-side-optimizations.md Rule D) ====
    //
    // `fit_mix` output (`matrix` / `fit_quality`) must match a
    // self-contained row-based normal-equations oracle (same shape
    // as `al_math::linalg`'s `mod legacy`) within the recomposition
    // tolerance `|a-b| <= 1e-12*max(1,|a|,|b|)`
    // (ADR-034EoHxzsvsHp9BOdCK2Cn). `condition_number` is out of
    // predicate scope: only INFINITY-consistency and order-of-
    // magnitude agreement (sanity bound) are checked. Stays green
    // after the A4 column-oriented `fit_mix` migration.

    /// Element-wise recomposition tolerance.
    fn assert_close(a: f64, b: f64) {
        let tol = 1e-12 * 1.0_f64.max(a.abs()).max(b.abs());
        assert!((a - b).abs() <= tol, "{a} vs {b} (tol {tol})");
    }

    /// Observation rows of source values: `rows[t][j] = sources[j][t]`.
    fn oracle_rows(sources: &[Vec<f64>], obs: usize) -> Vec<Vec<f64>> {
        (0..obs)
            .map(|t| sources.iter().map(|s| s[t]).collect())
            .collect()
    }

    /// Solve `a x = b` by Gaussian elimination with partial
    /// pivoting; `None` when singular (same shape as
    /// `linalg.rs` `mod legacy::solve`).
    #[allow(clippy::needless_range_loop)]
    fn oracle_gauss(a: &[Vec<f64>], b: &[f64]) -> Option<Vec<f64>> {
        let n = a.len();
        if n == 0 || b.len() != n || a.iter().any(|r| r.len() != n) {
            return None;
        }
        let mut m: Vec<Vec<f64>> = a
            .iter()
            .zip(b)
            .map(|(row, &bi)| row.iter().copied().chain([bi]).collect())
            .collect();
        for col in 0..n {
            let pivot = (col..n).max_by(|&i, &j| m[i][col].abs().total_cmp(&m[j][col].abs()))?;
            if m[pivot][col].abs() < 1e-12 {
                return None;
            }
            m.swap(col, pivot);
            let inv = 1.0 / m[col][col];
            for row in col + 1..n {
                let f = m[row][col] * inv;
                for c in col..=n {
                    m[row][c] -= f * m[col][c];
                }
            }
        }
        let mut x = vec![0.0; n];
        for row in (0..n).rev() {
            let s: f64 = (row + 1..n).map(|c| m[row][c] * x[c]).sum();
            x[row] = (m[row][n] - s) / m[row][row];
        }
        Some(x)
    }

    /// Row-based normal-equations least squares: returns
    /// `(coefficients, residual_rms)` — the oracle counterpart of
    /// one `LeastSquares::solve` call.
    #[allow(clippy::needless_range_loop)]
    fn oracle_solve(rows: &[Vec<f64>], rhs: &[f64], lambda: f64) -> Option<(Vec<f64>, f64)> {
        let cols = rows.first()?.len();
        if cols == 0 || rows.len() < cols || rhs.len() != rows.len() {
            return None;
        }
        let mut ata = vec![vec![0.0; cols]; cols];
        let mut atb = vec![0.0; cols];
        for (row, &y) in rows.iter().zip(rhs) {
            if row.len() != cols {
                return None;
            }
            for i in 0..cols {
                atb[i] += row[i] * y;
                for j in 0..cols {
                    ata[i][j] += row[i] * row[j];
                }
            }
        }
        for i in 0..cols {
            ata[i][i] += lambda;
        }
        let x = oracle_gauss(&ata, &atb)?;
        let mut rss = 0.0;
        for (row, &y) in rows.iter().zip(rhs) {
            let pred: f64 = row.iter().zip(&x).map(|(a, c)| a * c).sum();
            rss += (pred - y).powi(2);
        }
        Some((x, (rss / rows.len() as f64).sqrt()))
    }

    /// Plain RMS used by the oracle's fit-quality computation.
    fn oracle_rms(x: &[f64]) -> f64 {
        (x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64).sqrt()
    }

    /// Raw (pre-ridge) normal matrix over observation rows — the
    /// matrix `fit_mix` reports the condition number of.
    #[allow(clippy::needless_range_loop)]
    fn oracle_ata(sources: &[Vec<f64>], obs: usize) -> Vec<Vec<f64>> {
        let cols = sources.len();
        let mut ata = vec![vec![0.0; cols]; cols];
        for t in 0..obs {
            for i in 0..cols {
                for j in 0..cols {
                    ata[i][j] += sources[i][t] * sources[j][t];
                }
            }
        }
        ata
    }

    #[test]
    fn fit_mix_matches_normal_equations_oracle() {
        // (n_src, n_cap, obs, lambda): well-conditioned deterministic
        // fixtures, plus a singular duplicated-source case for the
        // INFINITY-consistency check on `condition_number`.
        let mut s_idx = 0u64;
        for (n_src, n_cap, obs, lambda) in [
            (1usize, 1usize, 4096usize, 0.0f64),
            (2, 2, 4096, 1e-6),
            (2, 1, 2048, 1e-3),
            (3, 2, 8192, 0.0),
        ] {
            let sources: Vec<Vec<f64>> = (0..n_src)
                .map(|_| {
                    s_idx += 1;
                    noise(100 + s_idx, obs)
                })
                .collect();
            // cap_c = Σ_s w[c][s]·src_s with deterministic weights.
            let captures: Vec<Vec<f64>> = (0..n_cap)
                .map(|c| {
                    (0..obs)
                        .map(|t| {
                            (0..n_src)
                                .map(|s| (0.4 + 0.3 * (c + s) as f64) * sources[s][t])
                                .sum()
                        })
                        .collect()
                })
                .collect();
            let src_refs: Vec<&[f64]> = sources.iter().map(Vec::as_slice).collect();
            let cap_refs: Vec<&[f64]> = captures.iter().map(Vec::as_slice).collect();
            let est = fit_mix(&src_refs, &cap_refs, lambda).unwrap();
            let rows = oracle_rows(&sources, obs);
            let mut quality = 0.0;
            for (c, cap) in captures.iter().enumerate() {
                let (x, rms) = oracle_solve(&rows, cap, lambda).unwrap();
                for (&a, &b) in est.matrix[c].iter().zip(&x) {
                    assert_close(a, b);
                }
                let cap_rms = oracle_rms(cap);
                quality += if cap_rms > 0.0 { rms / cap_rms } else { 0.0 };
            }
            assert_close(est.fit_quality, quality / n_cap as f64);
            // condition_number sanity only: INFINITY-consistency and
            // order-of-magnitude agreement with the in-test raw
            // normal matrix.
            let expected = condition_number(&oracle_ata(&sources, obs));
            assert_eq!(
                est.condition_number.is_infinite(),
                expected.is_infinite(),
                "condition_number INFINITY mismatch (est={}, expected={})",
                est.condition_number,
                expected
            );
            if expected.is_finite() && expected > 0.0 && est.condition_number > 0.0 {
                assert!(
                    (est.condition_number.log10() - expected.log10()).abs() <= 0.5,
                    "condition_number magnitude {} vs {}",
                    est.condition_number,
                    expected
                );
            }
        }

        // Ill-conditioned fixture: duplicated source → singular
        // normal matrix. `condition_number` reports INFINITY only
        // when the Gauss-Jordan pivot residual lands below its
        // absolute threshold — an ulp-luck edge — so the sanity
        // bound is "huge or infinite" (well-conditioned fixtures
        // above sit at ~1-10).
        let s0 = noise(200, 2048);
        let est = fit_mix(&[s0.as_slice(), s0.as_slice()], &[s0.as_slice()], 1e-6).unwrap();
        assert!(
            est.condition_number.is_infinite() || est.condition_number >= 1e6,
            "ill-conditioned mix must surface a huge condition number, got {}",
            est.condition_number
        );
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
        let src_refs: Vec<&[f64]> = sources.iter().map(Vec::as_slice).collect();
        let cap_refs: Vec<&[f64]> = captures.iter().map(Vec::as_slice).collect();
        let est = fit_mix(&src_refs, &cap_refs, 0.0).unwrap();
        for (actual, expected) in est.matrix.iter().flatten().zip([2.0, -1.0, -0.5, 3.0]) {
            assert!((actual - expected).abs() < 1e-7);
        }
        assert!(est.fit_quality < 1e-14);
        // 診断は従来と同じ ridge 前の正規行列を使用し、ridge に依存しない。
        let regularized = fit_mix(&src_refs, &cap_refs, 1e-3).unwrap();
        assert_eq!(est.condition_number, regularized.condition_number);
        // 参照側の AᵀA も fit_mix と同じ列 `vec::dot` 累算で組み立てる
        // (行累算とは ulps がずれ得るため assert_eq は同じ累算順が前提)。
        let mut ata = vec![vec![0.0; 2]; 2];
        for (i, ci) in sources.iter().enumerate() {
            for (j, cj) in sources.iter().enumerate() {
                ata[i][j] = vec::dot(ci, cj).expect("channel lengths are equal");
            }
        }
        assert_eq!(est.condition_number, condition_number(&ata));
    }
}
