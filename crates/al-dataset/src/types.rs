//! Result shape shared by every derived measurement stored in the
//! analysis dataset (`FeatureRecord`).

use al_core::{Confidence, Validity};
use serde::{Deserialize, Serialize};

/// A closed `[start, end)` range in seconds or Hz.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Range {
    /// Range start (inclusive).
    pub start: f64,
    /// Range end (exclusive).
    pub end: f64,
}

/// Channel selector for a measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelSelector {
    /// Single channel index.
    Channel(u16),
    /// Channel pair (for cross-channel measurements).
    ChannelPair(u16, u16),
}

/// Where the measured payload lives.
///
/// JSON has no NaN literal: invalid measurements persist
/// `{"scalar": null}` and deserialize back to `f64::NAN`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Value {
    /// A scalar value held directly in the record.
    Scalar(f64),
    /// A slice of a stored tensor: dataset-relative array path plus
    /// the coordinate range covering the measured region.
    Tensor {
        /// Dataset-relative path of the chunked tensor array.
        array: String,
        /// Per-dimension `[start, end)` coordinates.
        region: Vec<[u64; 2]>,
    },
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case")]
        enum Repr {
            Scalar(Option<f64>),
            Tensor {
                array: String,
                region: Vec<[u64; 2]>,
            },
        }
        match Repr::deserialize(deserializer)? {
            Repr::Scalar(v) => Ok(Value::Scalar(v.unwrap_or(f64::NAN))),
            Repr::Tensor { array, region } => Ok(Value::Tensor { array, region }),
        }
    }
}

/// Analysis resolution parameters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Resolution {
    /// 解析対象 PCM のサンプリングレート (Hz)。
    pub sample_rate: u32,
    /// Analysis window length (samples or Hz bins, per `unit`).
    pub window: u64,
    /// Hop between successive windows.
    pub hop: u64,
}

/// Coarse resolution class a measurement belongs to. The class name
/// and the exact window/hop are persisted together — the class is a
/// stable axis for cross-feature comparison while window/hop carry
/// the actual parameters used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionClass {
    /// Per-sample measurements.
    Sample,
    /// ~5 ms windows.
    Fine5ms,
    /// ~20 ms windows.
    Mid20ms,
    /// ~100 ms windows.
    Broad100ms,
    /// ~500 ms windows.
    Wide500ms,
    /// ~1 s or longer windows.
    Seconds,
    /// Whole-segment summary (no windowing).
    Segment,
}

/// Difference to a reference value, when meaningful.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Delta {
    /// Absolute difference `target - reference`.
    pub difference: Option<f64>,
    /// Ratio `target / reference`.
    pub ratio: Option<f64>,
}

/// One derived measurement, covering the spec's result shape.
///
/// Every field maps 1:1 to the analysis-dataset spec so a consumer can
/// trace what was measured, where, by which algorithm, and why the
/// value may not be trustworthy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureRecord {
    /// Feature name / schema id identifying what was measured.
    pub feature_id: String,
    /// The measured value or tensor position.
    pub value: Value,
    /// Unit of `value` (e.g. `hz`, `db`, `samples`).
    pub unit: String,
    /// Time range the measurement covers, when applicable.
    pub time_range: Option<Range>,
    /// Frequency range the measurement covers, when applicable.
    pub freq_range: Option<Range>,
    /// Channel or channel pair the measurement applies to.
    pub channels: Option<ChannelSelector>,
    /// Window / hop the measurement was computed with.
    pub resolution: Option<Resolution>,
    /// Coarse resolution class (stable axis for cross-feature use).
    pub resolution_class: ResolutionClass,
    /// Absolute measured (target) value.
    pub target: Option<f64>,
    /// Reference value used for comparison, when applicable.
    pub reference: Option<f64>,
    /// Difference and/or ratio to the reference, when meaningful.
    pub delta: Option<Delta>,
    /// Confidence that the observation reflects a real phenomenon.
    pub confidence: Confidence,
    /// Whether the measurement is meaningful for this range.
    pub validity: Validity,
    /// Reason the measurement is invalid or low-confidence.
    pub invalid_reason: Option<String>,
    /// Support count / number of valid windows.
    pub support_count: u64,
    /// Algorithm id that produced the measurement.
    pub algorithm_id: String,
    /// Algorithm version (independent from the schema version).
    pub algorithm_version: String,
    /// Algorithm parameters.
    pub params: serde_json::Value,
    /// Input id the measurement was computed from.
    pub input_id: String,
    /// Alignment id underpinning the measurement, when applicable.
    pub alignment_id: Option<String>,
    /// Parent / intermediate artifact ids in the derivation chain.
    pub lineage: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_record_round_trips() {
        let record = FeatureRecord {
            feature_id: "rms".into(),
            value: Value::Scalar(-12.5),
            unit: "dbfs".into(),
            time_range: Some(Range {
                start: 0.0,
                end: 1.0,
            }),
            freq_range: None,
            channels: Some(ChannelSelector::Channel(0)),
            resolution: Some(Resolution {
                sample_rate: 48000,
                window: 2048,
                hop: 512,
            }),
            resolution_class: ResolutionClass::Mid20ms,
            target: Some(-12.5),
            reference: Some(-10.0),
            delta: Some(Delta {
                difference: Some(-2.5),
                ratio: Some(1.25),
            }),
            confidence: Confidence::High,
            validity: Validity::Valid,
            invalid_reason: None,
            support_count: 42,
            algorithm_id: "rms.v1".into(),
            algorithm_version: "1".into(),
            params: serde_json::json!({}),
            input_id: "IN-x".into(),
            alignment_id: None,
            lineage: vec!["ART-a".into()],
        };
        let json = serde_json::to_string(&record).unwrap();
        let body: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(body["resolution"]["sample_rate"], 48000);
        let back: FeatureRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back, record);
        // 旧形式の暗黙 fallback は設けず、成果物の再生成を要求する。
        let mut missing_rate = body;
        missing_rate["resolution"]
            .as_object_mut()
            .unwrap()
            .remove("sample_rate");
        assert!(serde_json::from_value::<FeatureRecord>(missing_rate).is_err());
    }

    #[test]
    fn resolution_class_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&ResolutionClass::Mid20ms).unwrap(),
            "\"mid20ms\""
        );
        assert_eq!(
            serde_json::to_string(&ResolutionClass::Broad100ms).unwrap(),
            "\"broad100ms\""
        );
        assert_eq!(
            serde_json::to_string(&ResolutionClass::Segment).unwrap(),
            "\"segment\""
        );
    }
}
