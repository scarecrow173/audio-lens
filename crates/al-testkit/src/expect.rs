//! 期待メトリクス検証ヘルパ。
//!
//! 「期待値 ± 許容誤差」をメトリクス名と対にして検証し、失敗時は
//! 期待/実測/誤差を表示する。回帰テストで「値ではなく回復される
//! べき関係」を読みやすく書くための薄い層。

/// 絶対 + 相対の許容誤差。`|actual - expected| <= abs + rel * |expected|`
/// を満たせば合格。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tolerance {
    /// 絶対許容誤差。
    pub abs: f64,
    /// 相対許容誤差(期待値に対する比率)。
    pub rel: f64,
}

impl Tolerance {
    /// 絶対許容誤差のみ。
    pub const fn abs(abs: f64) -> Self {
        Self { abs, rel: 0.0 }
    }

    /// 相対許容誤差のみ。
    pub const fn rel(rel: f64) -> Self {
        Self { abs: 0.0, rel }
    }

    /// 絶対 + 相対。
    pub const fn abs_rel(abs: f64, rel: f64) -> Self {
        Self { abs, rel }
    }

    /// `actual` が `expected` の許容範囲内か。
    pub fn allows(&self, expected: f64, actual: f64) -> bool {
        if !expected.is_finite() || !actual.is_finite() {
            // NaN/inf は許容誤差では救えない — 完全一致のみ許す
            return expected == actual || (expected.is_nan() && actual.is_nan());
        }
        (actual - expected).abs() <= self.abs + self.rel * expected.abs()
    }
}

/// `metric`(識別名)の実測値 `actual` が `expected ± tol` に収まる
/// ことを検証する。外れた場合は panic で期待/実測を表示する。
#[track_caller]
pub fn expect_close(metric: &str, expected: f64, actual: f64, tol: Tolerance) {
    assert!(
        tol.allows(expected, actual),
        "{metric}: expected {expected} (abs {} + rel {}), got {actual} (diff {})",
        tol.abs,
        tol.rel,
        if actual.is_finite() && expected.is_finite() {
            format!("{:.6}", (actual - expected).abs())
        } else {
            "non-finite".to_string()
        }
    );
}

/// 整数メトリクスの `expected ± tol` 検証(サンプル位置等)。
#[track_caller]
pub fn expect_close_u64(metric: &str, expected: u64, actual: u64, tol: u64) {
    let diff = expected.abs_diff(actual);
    assert!(
        diff <= tol,
        "{metric}: expected {expected} (+/-{tol}), got {actual} (diff {diff})"
    );
}

/// 区間包含検証: `actual` が `[lo, hi]` に収まること。
#[track_caller]
pub fn expect_between(metric: &str, lo: f64, hi: f64, actual: f64) {
    assert!(
        actual >= lo && actual <= hi,
        "{metric}: expected [{lo}, {hi}], got {actual}"
    );
}

/// 条件が真であること。失敗時は `detail` を表示する。
#[track_caller]
pub fn expect_true(metric: &str, ok: bool, detail: impl std::fmt::Display) {
    assert!(ok, "{metric}: expected true — {detail}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abs_and_rel_tolerances() {
        expect_close("g", 0.5, 0.51, Tolerance::abs(0.02));
        expect_close("g", 100.0, 105.0, Tolerance::rel(0.1));
        expect_close("g", 100.0, 101.0, Tolerance::abs_rel(0.5, 0.01));
        expect_close_u64("pos", 5_000, 5_030, 64);
        expect_between("sim", 0.8, 0.99, 0.9);
    }

    #[test]
    #[should_panic(expected = "g: expected 0.5")]
    fn reports_expected_vs_actual() {
        expect_close("g", 0.5, 0.9, Tolerance::abs(0.02));
    }

    #[test]
    #[should_panic(expected = "pos: expected 5000")]
    fn reports_position_mismatch() {
        expect_close_u64("pos", 5_000, 5_200, 64);
    }

    #[test]
    fn nan_only_matches_nan() {
        expect_close("nan", f64::NAN, f64::NAN, Tolerance::abs(0.0));
        assert!(!Tolerance::abs(1.0).allows(f64::NAN, 0.5));
    }
}
