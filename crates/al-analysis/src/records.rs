//! `FeatureRecord` construction helpers: robust window statistics,
//! the confidence/validity propagation rules, and resolution-class
//! tagging required by the analysis plan.

use al_core::{Confidence, Validity};
use al_dataset::types::{FeatureRecord, Range, Resolution, ResolutionClass, Value};

/// Robust summary over a windowed statistic: median-centered and
/// distribution-aware rather than mean/stddev only.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RobustSummary {
    /// Median value.
    pub median: f64,
    /// Median absolute deviation around the median.
    pub mad: f64,
    /// Minimum observed value.
    pub min: f64,
    /// Maximum observed value.
    pub max: f64,
    /// 5th percentile.
    pub p05: f64,
    /// 25th percentile.
    pub p25: f64,
    /// 75th percentile.
    pub p75: f64,
    /// 95th percentile.
    pub p95: f64,
    /// Mean value.
    pub mean: f64,
    /// Number of finite samples that contributed.
    pub support: u64,
    /// Fraction of input samples that were finite.
    pub valid_fraction: f64,
}

/// Compute a [`RobustSummary`] over `values` (non-finite values are
/// excluded but counted in `valid_fraction`). Empty/all-invalid input
/// returns `None`.
pub fn robust_summary(values: &[f64]) -> Option<RobustSummary> {
    if values.is_empty() {
        return None;
    }
    let mut finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if finite.is_empty() {
        return None;
    }
    finite.sort_by(|a, b| a.total_cmp(b));
    let n = finite.len();
    let pct = |p: f64| -> f64 {
        let idx = (p * (n - 1) as f64).round() as usize;
        finite[idx.min(n - 1)]
    };
    let median = pct(0.5);
    let mut abs_dev: Vec<f64> = finite.iter().map(|v| (v - median).abs()).collect();
    abs_dev.sort_by(|a, b| a.total_cmp(b));
    let mad = abs_dev[n / 2];
    let mean = finite.iter().sum::<f64>() / n as f64;
    Some(RobustSummary {
        median,
        mad,
        min: finite[0],
        max: finite[n - 1],
        p05: pct(0.05),
        p25: pct(0.25),
        p75: pct(0.75),
        p95: pct(0.95),
        mean,
        support: n as u64,
        valid_fraction: n as f64 / values.len() as f64,
    })
}

/// Map an upstream invalidity to the validity a downstream
/// measurement must carry — the dependency mapping from the plan.
/// `Valid` passes through unchanged.
pub fn propagate_validity(upstream: Validity) -> Validity {
    match upstream {
        Validity::Valid => Validity::Valid,
        Validity::AmbiguousAlignment | Validity::LowCorrelation => Validity::AmbiguousAlignment,
        Validity::LowCoherence => Validity::LowCoherence,
        Validity::SilenceInsufficientEnergy | Validity::InsufficientSupport => {
            Validity::SilenceInsufficientEnergy
        }
        Validity::DecodeInputUnavailable => Validity::DecodeInputUnavailable,
        _ => Validity::ComputationFailed,
    }
}

/// Downstream confidence: the weaker of the upstream confidence and
/// the stage's own estimate.
pub fn propagate_confidence(upstream: Confidence, own: Confidence) -> Confidence {
    upstream.min(own)
}

/// 解析 stage が依存する ruviform 群の識別 stamp(#117)。
/// rev の導出は `al_core::RUVIFORM_REV`(Cargo.lock 解析)の単一経路のみを
/// 使う。precision は ruviform の f64 scalar 評価契約を表し、crates は
/// stage が利用する ruviform 依存を列挙した主要パラメータである。
pub fn ruviform_stamp() -> serde_json::Value {
    serde_json::json!({
        "rev": al_core::RUVIFORM_REV,
        "precision": "f64",
        "crates": [
            "ruviform-audio-analysis",
            "ruviform-dsp",
            "ruviform-numeric",
        ],
    })
}

/// params object へ `ruviform` stamp を挿入して返す。
/// params は常に JSON object でなければならない(空 object も可)。
pub fn with_ruviform_stamp(mut params: serde_json::Value) -> serde_json::Value {
    params
        .as_object_mut()
        .expect("artifact params must be a JSON object")
        .insert("ruviform".into(), ruviform_stamp());
    params
}

/// `ruviform` stamp のみを持つ params object。
pub fn ruviform_params() -> serde_json::Value {
    with_ruviform_stamp(serde_json::json!({}))
}

/// artifact 書き出し直前に全 `FeatureRecord` の params へ `ruviform`
/// stamp を記録する。measure 層が params を上書きするため record 構築
/// 時点ではなくここで一括適用する。
pub fn stamp_feature_records(records: &mut [FeatureRecord]) {
    for r in records {
        r.params["ruviform"] = ruviform_stamp();
    }
}

/// Builder for `FeatureRecord`s with the shared fields pre-filled.
pub struct RecordBuilder {
    input_id: String,
    algorithm_id: String,
    algorithm_version: String,
    resolution_class: ResolutionClass,
    resolution: Option<Resolution>,
    lineage: Vec<String>,
}

impl RecordBuilder {
    /// Start a builder for one producer (algorithm + input) and one
    /// resolution class.
    pub fn new(
        input_id: impl Into<String>,
        algorithm_id: impl Into<String>,
        resolution_class: ResolutionClass,
    ) -> Self {
        Self {
            input_id: input_id.into(),
            algorithm_id: algorithm_id.into(),
            algorithm_version: "1".into(),
            resolution_class,
            resolution: None,
            lineage: Vec::new(),
        }
    }

    /// Set the algorithm version.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.algorithm_version = version.into();
        self
    }

    /// 解析対象 PCM のレートと window/hop を設定する。
    pub fn resolution(mut self, window: u64, hop: u64, sample_rate: u32) -> Self {
        self.resolution = Some(Resolution {
            window,
            hop,
            sample_rate,
        });
        self
    }

    /// Set the derivation lineage (dataset artifact ids/paths).
    pub fn lineage(mut self, lineage: impl IntoIterator<Item = String>) -> Self {
        self.lineage = lineage.into_iter().collect();
        self
    }

    /// A scalar record.
    #[allow(clippy::too_many_arguments)]
    pub fn scalar(
        &self,
        feature_id: impl Into<String>,
        value: f64,
        unit: impl Into<String>,
        time_range: Option<Range>,
        confidence: Confidence,
        validity: Validity,
        invalid_reason: Option<String>,
        support_count: u64,
    ) -> FeatureRecord {
        FeatureRecord {
            feature_id: feature_id.into(),
            value: Value::Scalar(value),
            unit: unit.into(),
            time_range,
            freq_range: None,
            channels: None,
            resolution: self.resolution,
            resolution_class: self.resolution_class,
            target: Some(value),
            reference: None,
            delta: None,
            confidence,
            validity,
            invalid_reason,
            support_count,
            algorithm_id: self.algorithm_id.clone(),
            algorithm_version: self.algorithm_version.clone(),
            params: serde_json::json!({}),
            input_id: self.input_id.clone(),
            alignment_id: None,
            lineage: self.lineage.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_is_robust_to_outliers() {
        let mut v = vec![1.0; 19];
        v.push(1000.0);
        let s = robust_summary(&v).unwrap();
        assert_eq!(s.median, 1.0);
        assert_eq!(s.max, 1000.0);
        assert_eq!(s.support, 20);
    }

    #[test]
    fn summary_counts_invalid_input() {
        let v = vec![1.0, f64::NAN, 3.0];
        let s = robust_summary(&v).unwrap();
        assert_eq!(s.support, 2);
        assert!((s.valid_fraction - 2.0 / 3.0).abs() < 1e-9);
        assert!(robust_summary(&[]).is_none());
        assert!(robust_summary(&[f64::NAN]).is_none());
    }

    #[test]
    fn validity_maps_by_dependency() {
        assert_eq!(propagate_validity(Validity::Valid), Validity::Valid);
        assert_eq!(
            propagate_validity(Validity::AmbiguousAlignment),
            Validity::AmbiguousAlignment
        );
        assert_eq!(
            propagate_validity(Validity::LowCoherence),
            Validity::LowCoherence
        );
        assert_eq!(
            propagate_validity(Validity::InsufficientSupport),
            Validity::SilenceInsufficientEnergy
        );
        assert_eq!(
            propagate_validity(Validity::NotApplicable),
            Validity::ComputationFailed
        );
    }

    #[test]
    fn confidence_propagates_as_min() {
        assert_eq!(
            propagate_confidence(Confidence::High, Confidence::Low),
            Confidence::Low
        );
        assert_eq!(
            propagate_confidence(Confidence::Medium, Confidence::High),
            Confidence::Medium
        );
    }

    #[test]
    fn ruviform_stamp_records_dependency_identity() {
        let stamp = ruviform_stamp();
        assert_eq!(stamp["rev"], al_core::RUVIFORM_REV);
        assert_eq!(stamp["precision"], "f64");
        assert!(
            stamp["crates"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("ruviform-numeric"))
        );
        assert_eq!(stamp, ruviform_stamp());
    }

    #[test]
    fn stamp_feature_records_preserves_existing_params() {
        let mut r = RecordBuilder::new("IN-1", "cheap.v1", ResolutionClass::Segment).scalar(
            "rms",
            -12.0,
            "dbfs",
            None,
            Confidence::High,
            Validity::Valid,
            None,
            10,
        );
        r.params = serde_json::json!({"region_s": [0.0, 1.0]});
        let mut records = vec![r];
        stamp_feature_records(&mut records);
        assert_eq!(records[0].params["ruviform"]["rev"], al_core::RUVIFORM_REV);
        assert_eq!(records[0].params["region_s"], serde_json::json!([0.0, 1.0]));
    }

    #[test]
    fn with_ruviform_stamp_requires_object_params() {
        let params = with_ruviform_stamp(serde_json::json!({"event_count": 3}));
        assert_eq!(params["event_count"], 3);
        assert_eq!(params["ruviform"]["precision"], "f64");
    }

    #[test]
    fn record_builder_stamps_resolution() {
        let b = RecordBuilder::new("IN-1", "rms.v1", ResolutionClass::Mid20ms)
            .resolution(960, 480, 48000)
            .lineage(["ART-1".to_string()]);
        let r = b.scalar(
            "rms",
            -12.0,
            "dbfs",
            None,
            Confidence::High,
            Validity::Valid,
            None,
            10,
        );
        assert_eq!(r.resolution_class, ResolutionClass::Mid20ms);
        assert_eq!(r.resolution.unwrap().window, 960);
        assert_eq!(r.resolution.unwrap().sample_rate, 48000);
        assert_eq!(r.lineage, vec!["ART-1"]);
    }
}
