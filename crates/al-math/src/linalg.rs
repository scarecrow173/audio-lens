//! 小列数の Householder QR。求解と正規行列による診断は独立している。

use crate::{Simd, WithSimd, dispatch};

fn dot<S: Simd>(simd: S, a: &[f64], b: &[f64]) -> f64 {
    let (av, at) = S::as_simd_f64s(a);
    let (bv, bt) = S::as_simd_f64s(b);
    let mut sum = simd.splat_f64s(0.0);
    for (&a, &b) in av.iter().zip(bv) {
        sum = simd.add_f64s(sum, simd.mul_f64s(a, b));
    }
    simd.reduce_sum_f64s(sum) + at.iter().zip(bt).map(|(a, b)| a * b).sum::<f64>()
}

fn reflect<S: Simd>(simd: S, v: &[f64], target: &mut [f64]) {
    let factor = 2.0 * dot(simd, v, target);
    let (vv, vt) = S::as_simd_f64s(v);
    let (tv, tt) = S::as_mut_simd_f64s(target);
    let scale = simd.splat_f64s(factor);
    for (t, &v) in tv.iter_mut().zip(vv) {
        *t = simd.sub_f64s(*t, simd.mul_f64s(scale, v));
    }
    for (t, &v) in tt.iter_mut().zip(vt) {
        *t -= factor * v;
    }
}

/// 元の観測と反射を所有し、複数 rhs に再利用する分解。
pub struct LeastSquares {
    rows: Vec<Vec<f64>>,
    columns: Vec<Vec<f64>>,
    reflectors: Vec<Vec<f64>>,
}

impl LeastSquares {
    /// 有限の行列、非負有限の ridge、観測数 >= 列数を要求する。
    pub fn new(rows: &[Vec<f64>], lambda: f64) -> Option<Self> {
        let cols = rows.first()?.len();
        if cols == 0
            || rows.len() < cols
            || !lambda.is_finite()
            || lambda < 0.0
            || rows
                .iter()
                .any(|r| r.len() != cols || r.iter().any(|x| !x.is_finite()))
        {
            return None;
        }
        dispatch(Factor { rows, lambda })
    }

    /// 元の観測のみから残差 RMS を求める（ridge 項を含めない）。
    pub fn solve(&self, rhs: &[f64]) -> Option<(Vec<f64>, f64)> {
        if rhs.len() != self.rows.len() || rhs.iter().any(|x| !x.is_finite()) {
            return None;
        }
        dispatch(Apply { factor: self, rhs })
    }
}

/// 単一 rhs の ridge 最小二乗求解。
pub fn least_squares(rows: &[Vec<f64>], rhs: &[f64], lambda: f64) -> Option<(Vec<f64>, f64)> {
    LeastSquares::new(rows, lambda)?.solve(rhs)
}

struct Factor<'a> {
    rows: &'a [Vec<f64>],
    lambda: f64,
}
impl WithSimd for Factor<'_> {
    type Output = Option<LeastSquares>;
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let n = self.rows[0].len();
        let m = self.rows.len();
        let extra = if self.lambda > 0.0 { n } else { 0 };
        let mut columns: Vec<Vec<f64>> = (0..n)
            .map(|j| {
                let mut c: Vec<f64> = self.rows.iter().map(|r| r[j]).collect();
                c.extend((0..extra).map(|i| if i == j { self.lambda.sqrt() } else { 0.0 }));
                c
            })
            .collect();
        let scale = columns
            .iter()
            .flat_map(|c| c.iter())
            .fold(0.0_f64, |s, x| s.max(x.abs()));
        let tolerance = f64::EPSILON * (m + extra).max(n) as f64 * scale;
        let mut reflectors = Vec::with_capacity(n);
        for j in 0..n {
            // hypot でノルム計算時の二乗による overflow/underflow を避ける。
            let norm = columns[j][j..].iter().fold(0.0_f64, |s, &x| s.hypot(x));
            if !norm.is_finite() || norm <= tolerance {
                return None;
            }
            let sign = if columns[j][j] >= 0.0 { 1.0 } else { -1.0 };
            let mut v: Vec<f64> = columns[j][j..].iter().map(|x| x / norm).collect();
            v[0] += sign;
            let vnorm = dot(simd, &v, &v).sqrt();
            for x in &mut v {
                *x /= vnorm;
            }
            for c in &mut columns[j..] {
                reflect(simd, &v, &mut c[j..]);
            }
            columns[j][j] = -sign * norm;
            columns[j][j + 1..].fill(0.0);
            reflectors.push(v);
        }
        Some(LeastSquares {
            rows: self.rows.to_vec(),
            columns,
            reflectors,
        })
    }
}

struct Apply<'a> {
    factor: &'a LeastSquares,
    rhs: &'a [f64],
}
impl WithSimd for Apply<'_> {
    type Output = Option<(Vec<f64>, f64)>;
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let f = self.factor;
        let n = f.columns.len();
        let mut y = self.rhs.to_vec();
        y.resize(f.columns[0].len(), 0.0);
        for (j, v) in f.reflectors.iter().enumerate() {
            reflect(simd, v, &mut y[j..]);
        }
        let mut x = vec![0.0; n];
        for j in (0..n).rev() {
            let sum: f64 = (j + 1..n).map(|k| f.columns[k][j] * x[k]).sum();
            x[j] = (y[j] - sum) / f.columns[j][j];
        }
        let rss: f64 = f
            .rows
            .iter()
            .zip(self.rhs)
            .map(|(row, &b)| (dot(simd, row, &x) - b).powi(2))
            .sum();
        let rms = (rss / f.rows.len() as f64).sqrt();
        if x.iter().any(|v| !v.is_finite()) || !rms.is_finite() {
            return None;
        }
        Some((x, rms))
    }
}

/// 従来と同じ正方行列のスペクトル条件数推定。特異行列は INFINITY。
/// mixer は ridge 前の AᵀA を渡す。求解のランク判定には使用しない。
pub fn condition_number(a: &[Vec<f64>]) -> f64 {
    dispatch(Condition(a))
}
struct Condition<'a>(&'a [Vec<f64>]);
impl WithSimd for Condition<'_> {
    type Output = f64;
    fn with_simd<S: Simd>(self, _simd: S) -> f64 {
        condition_number_scalar(self.0)
    }
}
/// Spectral condition-number estimate of a square matrix via the
/// ratio of largest to smallest singular value approximated by
/// power iteration on `AᵀA` and `(AᵀA)⁻¹`. Returns `f64::INFINITY`
/// for singular input.
#[allow(clippy::needless_range_loop)]
fn condition_number_scalar(a: &[Vec<f64>]) -> f64 {
    let n = a.len();
    if n == 0 || a.iter().any(|r| r.len() != n) {
        return f64::INFINITY;
    }
    let ata: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| (0..n).map(|k| a[k][i] * a[k][j]).sum())
                .collect()
        })
        .collect();
    let power = |m: &[Vec<f64>]| -> f64 {
        let mut v = vec![1.0 / (n as f64).sqrt(); n];
        let mut lambda = 0.0;
        for _ in 0..64 {
            let mut w = vec![0.0; n];
            for (i, row) in m.iter().enumerate() {
                w[i] = row.iter().zip(&v).map(|(a, b)| a * b).sum();
            }
            let norm = w.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm == 0.0 {
                return 0.0;
            }
            for x in &mut w {
                *x /= norm;
            }
            lambda = norm;
            v = w;
        }
        lambda
    };
    let max_sv2 = power(&ata);
    if max_sv2 <= 0.0 {
        return f64::INFINITY;
    }
    let Some(inv) = invert(&ata) else {
        return f64::INFINITY;
    };
    let min_sv2_inv = power(&inv);
    if min_sv2_inv <= 0.0 {
        return f64::INFINITY;
    }
    (max_sv2 * min_sv2_inv).sqrt()
}

/// Invert a square matrix via Gauss-Jordan; `None` when singular.
#[allow(clippy::needless_range_loop)]
fn invert(a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a.len();
    if n == 0 || a.iter().any(|r| r.len() != n) {
        return None;
    }
    let mut m: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            a[i].iter()
                .copied()
                .chain((0..n).map(|j| (i == j) as u8 as f64))
                .collect()
        })
        .collect();
    for col in 0..n {
        let pivot = (col..n).max_by(|&i, &j| m[i][col].abs().total_cmp(&m[j][col].abs()))?;
        if m[pivot][col].abs() < 1e-12 {
            return None;
        }
        m.swap(col, pivot);
        let inv = 1.0 / m[col][col];
        for c in 0..2 * n {
            m[col][c] *= inv;
        }
        for row in 0..n {
            if row == col {
                continue;
            }
            let f = m[row][col];
            for c in 0..2 * n {
                m[row][c] -= f * m[col][c];
            }
        }
    }
    Some(m.into_iter().map(|row| row[n..].to_vec()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tolerance: f64) {
        assert!((a - b).abs() <= tolerance, "{a} != {b}");
    }

    #[test]
    fn golden_matches_normal_equations_and_scalar_for_ridge_and_tails() {
        for n in 1..=8 {
            for m in [n + 1, 33, 65] {
                let rows: Vec<Vec<f64>> = (0..m)
                    .map(|i| {
                        (0..n)
                            .map(|j| ((i + 1) as f64 * (j + 1) as f64 * 0.71).sin())
                            .collect()
                    })
                    .collect();
                let rhs: Vec<f64> = rows
                    .iter()
                    .enumerate()
                    .map(|(i, row)| {
                        row.iter()
                            .enumerate()
                            .map(|(j, v)| v * (j as f64 - 2.5))
                            .sum::<f64>()
                            + (i as f64 * 0.9).cos() * 0.01
                    })
                    .collect();
                for lambda in [0.0, 1e-3, 2.0] {
                    let expected = legacy::run(&rows, &rhs, lambda).unwrap();
                    let factor = LeastSquares::new(&rows, lambda).unwrap();
                    let native = factor.solve(&rhs).unwrap();
                    let scalar_factor = pulp::Scalar
                        .vectorize(Factor {
                            rows: &rows,
                            lambda,
                        })
                        .unwrap();
                    let scalar = pulp::Scalar
                        .vectorize(Apply {
                            factor: &scalar_factor,
                            rhs: &rhs,
                        })
                        .unwrap();
                    for ((a, b), c) in native.0.iter().zip(&expected.0).zip(&scalar.0) {
                        close(*a, *b, 1e-8);
                        close(*a, *c, 1e-10);
                    }
                    close(native.1, expected.1, 1e-10);
                    close(native.1, scalar.1, 1e-10);
                }
            }
        }
    }

    #[test]
    fn highly_correlated_columns_recover_truth_independently_of_diagnostics() {
        let rows: Vec<Vec<f64>> = (0..128)
            .map(|i| {
                let a = (i as f64 * 0.7).sin();
                vec![a, a + 1e-7 * (i as f64 * 1.3).cos()]
            })
            .collect();
        let rhs: Vec<f64> = rows.iter().map(|r| 2.0 * r[0] - r[1]).collect();
        let (x, rms) = least_squares(&rows, &rhs, 0.0).unwrap();
        let error = (x[0] - 2.0).abs() + (x[1] + 1.0).abs();
        assert!(error < 1e-7, "error={error}");
        assert!(rms < 1e-14);
        if let Some((old, _)) = legacy::run(&rows, &rhs, 0.0) {
            assert!(error < ((old[0] - 2.0).abs() + (old[1] + 1.0).abs()) * 0.01);
        }
    }

    #[test]
    fn rejects_invalid_and_rank_deficient_inputs_but_ridge_solves_singular() {
        let rows = vec![vec![1.0, 1.0], vec![2.0, 2.0]];
        assert!(LeastSquares::new(&rows, 0.0).is_none());
        let factor = LeastSquares::new(&rows, 2.0).unwrap();
        let (x, rms) = factor.solve(&[2.0, 4.0]).unwrap();
        close(x[0], 5.0 / 6.0, 1e-14);
        close(x[1], 5.0 / 6.0, 1e-14);
        close(rms, (5.0_f64 / 18.0).sqrt(), 1e-14);
        assert!(factor.solve(&[2.0]).is_none());
        assert!(factor.solve(&[f64::NAN, 4.0]).is_none());
        for lambda in [-1.0, f64::NAN, f64::INFINITY] {
            assert!(LeastSquares::new(&rows, lambda).is_none());
        }
        for rows in [
            vec![],
            vec![vec![]],
            vec![vec![1.0, 2.0]],
            vec![vec![1.0], vec![1.0, 2.0]],
            vec![vec![f64::NAN]],
            vec![vec![f64::INFINITY]],
            vec![vec![0.0]],
        ] {
            assert!(LeastSquares::new(&rows, 0.0).is_none());
        }
    }

    #[test]
    fn reuses_factorization_and_handles_small_scale() {
        let rows = vec![vec![1e-100], vec![-2e-100], vec![3e-100]];
        let f = LeastSquares::new(&rows, 0.0).unwrap();
        for gain in [-2.0, 0.0, 3.0] {
            let rhs: Vec<f64> = rows.iter().map(|r| r[0] * gain).collect();
            let (x, rms) = f.solve(&rhs).unwrap();
            close(x[0], gain, 1e-14);
            assert!(rms < 1e-113);
        }
    }

    #[test]
    fn preserves_normal_matrix_condition_number() {
        close(
            condition_number(&[vec![4.0, 0.0], vec![0.0, 1.0]]),
            4.0,
            1e-10,
        );
        assert_eq!(
            condition_number(&[vec![1.0, 2.0], vec![2.0, 4.0]]),
            f64::INFINITY
        );
        assert_eq!(condition_number(&[]), f64::INFINITY);
    }
    mod legacy {
        /// Ordinary least squares with ridge regularization, solved via
        /// Gaussian elimination on the normal equations.
        ///
        /// `rows` are observation rows (`cols` wide), `rhs` the right-hand
        /// side. Returns `(coefficients, residual_rms)`. `lambda` is the
        /// ridge coefficient; pass 0 for unregularized solve.
        #[allow(clippy::needless_range_loop)]
        fn least_squares(rows: &[Vec<f64>], rhs: &[f64], lambda: f64) -> Option<(Vec<f64>, f64)> {
            let cols = rows.first()?.len();
            if cols == 0 || rows.len() < cols || rhs.len() != rows.len() {
                return None;
            }
            // Normal equations: (AᵀA + λI) x = Aᵀb.
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
            let x = solve(&ata, &atb)?;
            let mut rss = 0.0;
            for (row, &y) in rows.iter().zip(rhs) {
                let pred: f64 = row.iter().zip(&x).map(|(a, c)| a * c).sum();
                rss += (pred - y).powi(2);
            }
            Some((x, (rss / rows.len() as f64).sqrt()))
        }

        /// Solve `a x = b` by Gaussian elimination with partial pivoting.
        /// Returns `None` when the system is singular.
        #[allow(clippy::needless_range_loop)]
        fn solve(a: &[Vec<f64>], b: &[f64]) -> Option<Vec<f64>> {
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
                let pivot =
                    (col..n).max_by(|&i, &j| m[i][col].abs().total_cmp(&m[j][col].abs()))?;
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

        pub(super) fn run(rows: &[Vec<f64>], rhs: &[f64], lambda: f64) -> Option<(Vec<f64>, f64)> {
            least_squares(rows, rhs, lambda)
        }
    }
}
