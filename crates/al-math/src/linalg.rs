//! 小列数の Householder QR。求解と正規行列による診断は独立している。

use crate::vec::{dot_kernel, sub_scaled_kernel, sum_sq_kernel};
use crate::{Simd, WithSimd, dispatch};

// `v` と `target` は同じ suffix 長(m+extra-j)を持ち、等長契約は呼出し側
// の構造で保証される。既に `WithSimd` コンテキスト内にあるため公開
// wrapper を経由せずカーネルを直接呼び、二重 dispatch を避ける。
fn reflect<S: Simd>(simd: S, v: &[f64], target: &mut [f64]) {
    let factor = 2.0 * dot_kernel(simd, v, target);
    sub_scaled_kernel(simd, target, factor, v);
}

/// 元の観測列と反射を所有し、複数 rhs に再利用する分解。
pub struct LeastSquares {
    // ridge 適用前の入力列(1 コピーのみ、RSS 用)。
    obs_columns: Vec<Vec<f64>>,
    // ridge augmentation 付き分解後の列(R 情報)。
    columns: Vec<Vec<f64>>,
    reflectors: Vec<Vec<f64>>,
}

impl LeastSquares {
    /// 有限・等長の列、非負有限の ridge、観測数(列長) >= 列数を要求する。
    pub fn from_columns(columns: &[&[f64]], lambda: f64) -> Option<Self> {
        let m = columns.first()?.len();
        if m < columns.len()
            || !lambda.is_finite()
            || lambda < 0.0
            || columns
                .iter()
                .any(|c| c.len() != m || c.iter().any(|x| !x.is_finite()))
        {
            return None;
        }
        dispatch(Factor { columns, lambda })
    }

    /// 元の観測のみから残差 RMS を求める（ridge 項を含めない）。
    pub fn solve(&self, rhs: &[f64]) -> Option<(Vec<f64>, f64)> {
        if rhs.len() != self.obs_columns[0].len() || rhs.iter().any(|x| !x.is_finite()) {
            return None;
        }
        dispatch(Apply { factor: self, rhs })
    }
}

/// 単一 rhs の ridge 最小二乗求解。
pub fn least_squares(columns: &[&[f64]], rhs: &[f64], lambda: f64) -> Option<(Vec<f64>, f64)> {
    LeastSquares::from_columns(columns, lambda)?.solve(rhs)
}

struct Factor<'a> {
    columns: &'a [&'a [f64]],
    lambda: f64,
}
impl WithSimd for Factor<'_> {
    type Output = Option<LeastSquares>;
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let n = self.columns.len();
        let m = self.columns[0].len();
        let extra = if self.lambda > 0.0 { n } else { 0 };
        // 分解用は入力列のコピーに √λ 対角の extra 要素を末尾へ追加した
        // augmented 列(長さ m+extra)。augmentation は分解内部のみに存在する。
        let mut columns: Vec<Vec<f64>> = self
            .columns
            .iter()
            .enumerate()
            .map(|(j, c)| {
                let mut c = c.to_vec();
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
            let vnorm = dot_kernel(simd, &v, &v).sqrt();
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
            obs_columns: self.columns.iter().map(|c| c.to_vec()).collect(),
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
        // RSS は連続バッファ上の per-column `sub_scaled` パスで計算する
        // (residual.rs の per-source 先例と同形)。ridge 項は含めない。
        let mut resid = self.rhs.to_vec();
        for (j, &xj) in x.iter().enumerate() {
            sub_scaled_kernel(simd, &mut resid, xj, &f.obs_columns[j]);
        }
        let rss = sum_sq_kernel(simd, &resid);
        let rms = (rss / self.rhs.len() as f64).sqrt();
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

    /// `&[&[f64]]` 引数用の列参照を組み立てる。
    fn col_refs(columns: &[Vec<f64>]) -> Vec<&[f64]> {
        columns.iter().map(Vec::as_slice).collect()
    }

    /// 列 fixture を行へ転置して `mod legacy` oracle に渡す。
    fn rows_of(columns: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let m = columns.first().map_or(0, |c| c.len());
        (0..m)
            .map(|i| columns.iter().map(|c| c[i]).collect())
            .collect()
    }

    #[test]
    fn golden_matches_normal_equations_and_scalar_for_ridge_and_tails() {
        for n in 1..=8 {
            for m in [n + 1, 33, 65] {
                let columns: Vec<Vec<f64>> = (0..n)
                    .map(|j| {
                        (0..m)
                            .map(|i| ((i + 1) as f64 * (j + 1) as f64 * 0.71).sin())
                            .collect()
                    })
                    .collect();
                let rhs: Vec<f64> = (0..m)
                    .map(|i| {
                        (0..n)
                            .map(|j| columns[j][i] * (j as f64 - 2.5))
                            .sum::<f64>()
                            + (i as f64 * 0.9).cos() * 0.01
                    })
                    .collect();
                let rows = rows_of(&columns);
                let refs = col_refs(&columns);
                for lambda in [0.0, 1e-3, 2.0] {
                    let expected = legacy::run(&rows, &rhs, lambda).unwrap();
                    let factor = LeastSquares::from_columns(&refs, lambda).unwrap();
                    let native = factor.solve(&rhs).unwrap();
                    let scalar_factor = pulp::Scalar
                        .vectorize(Factor {
                            columns: &refs,
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
        let columns: Vec<Vec<f64>> = vec![
            (0..128).map(|i| (i as f64 * 0.7).sin()).collect(),
            (0..128)
                .map(|i| (i as f64 * 0.7).sin() + 1e-7 * (i as f64 * 1.3).cos())
                .collect(),
        ];
        let rhs: Vec<f64> = (0..128)
            .map(|i| 2.0 * columns[0][i] - columns[1][i])
            .collect();
        let (x, rms) = least_squares(&col_refs(&columns), &rhs, 0.0).unwrap();
        let error = (x[0] - 2.0).abs() + (x[1] + 1.0).abs();
        assert!(error < 1e-7, "error={error}");
        assert!(rms < 1e-14);
        if let Some((old, _)) = legacy::run(&rows_of(&columns), &rhs, 0.0) {
            assert!(error < ((old[0] - 2.0).abs() + (old[1] + 1.0).abs()) * 0.01);
        }
    }

    #[test]
    fn rejects_invalid_and_rank_deficient_inputs_but_ridge_solves_singular() {
        let columns: [&[f64]; 2] = [&[1.0, 2.0], &[1.0, 2.0]];
        assert!(LeastSquares::from_columns(&columns, 0.0).is_none());
        let factor = LeastSquares::from_columns(&columns, 2.0).unwrap();
        let (x, rms) = factor.solve(&[2.0, 4.0]).unwrap();
        close(x[0], 5.0 / 6.0, 1e-14);
        close(x[1], 5.0 / 6.0, 1e-14);
        close(rms, (5.0_f64 / 18.0).sqrt(), 1e-14);
        assert!(factor.solve(&[2.0]).is_none());
        assert!(factor.solve(&[f64::NAN, 4.0]).is_none());
        for lambda in [-1.0, f64::NAN, f64::INFINITY] {
            assert!(LeastSquares::from_columns(&columns, lambda).is_none());
        }
        let empty: [&[f64]; 0] = [];
        assert!(LeastSquares::from_columns(&empty, 0.0).is_none());
        for columns in [
            vec![&[][..]],
            vec![&[1.0][..], &[2.0][..]],
            vec![&[1.0, 2.0][..], &[1.0][..]],
            vec![&[f64::NAN][..]],
            vec![&[f64::INFINITY][..]],
            vec![&[0.0][..]],
        ] {
            assert!(LeastSquares::from_columns(&columns, 0.0).is_none());
        }
    }

    #[test]
    fn reuses_factorization_and_handles_small_scale() {
        let column = vec![1e-100, -2e-100, 3e-100];
        let f = LeastSquares::from_columns(&[column.as_slice()], 0.0).unwrap();
        for gain in [-2.0, 0.0, 3.0] {
            let rhs: Vec<f64> = column.iter().map(|v| v * gain).collect();
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
