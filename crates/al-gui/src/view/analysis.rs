//! Analysis view model — events/candidates・locate summary・
//! `FeatureRecord` の typed 写し。
//!
//! 数値は f64/`Confidence`/`Validity`/`Provenance`/`SampleRange` を
//! そのまま保持し、GUI 側で再計算・解析の移動はしない。

use al_core::{Confidence, Validity};
use al_dataset::types::FeatureRecord;
use al_locate::SampleRange;
use serde_json::Value;

use super::ViewError;

/// `events/candidates` テーブル 1 行分の写し
/// (`al-analysis::events_schema` の列構成に対応)。
#[derive(Debug, Clone, PartialEq)]
pub struct EventRow {
    /// event id。
    pub event_id: String,
    /// event kind(tag 文字列)。
    pub kind: String,
    /// 計測対象 input id。
    pub input_id: String,
    /// 開始時刻(秒)。
    pub start_s: f64,
    /// 終了時刻(秒。open-ended は `None`)。
    pub end_s: Option<f64>,
    /// 強度。
    pub strength: f64,
    /// typed confidence。
    pub confidence: Confidence,
    /// typed validity。
    pub validity: Validity,
    /// 由来 feature_id。
    pub source_feature: String,
    /// 周波数範囲 (lo_hz, hi_hz)。無い場合 `None`。
    pub frequency_range_hz: Option<(f64, f64)>,
    /// algorithm id。
    pub algorithm_id: String,
    /// algorithm version。
    pub algorithm_version: String,
}

impl EventRow {
    /// `al.ds.events` の `rows[i]` から写す。
    ///
    /// # Errors
    /// 必須列が欠ける、または tag が未知の場合。
    pub fn from_json(row: &Value) -> Result<Self, ViewError> {
        let frequency_range_hz = match (
            super::f64_or(row, "freq_lo_hz"),
            super::f64_or(row, "freq_hi_hz"),
        ) {
            (Some(lo), Some(hi)) => Some((lo, hi)),
            _ => None,
        };
        Ok(Self {
            event_id: super::required(row, "event_id")?.to_string(),
            kind: super::required(row, "kind")?.to_string(),
            input_id: super::required(row, "input_id")?.to_string(),
            start_s: super::f64_or(row, "start_s")
                .ok_or_else(|| ViewError::Invalid("row lacks start_s".into()))?,
            end_s: super::f64_or(row, "end_s"),
            // `strength` は必須列。`Cell::F64(NaN)` は null として
            // 永続化される(dataset.rs cell_to_json)ため、null は
            // 0.0 ではなく NaN として復元する(f64 意味の保持)。
            strength: match row.get("strength") {
                Some(serde_json::Value::Null) => f64::NAN,
                Some(v) => v
                    .as_f64()
                    .ok_or_else(|| ViewError::Invalid("strength is not a number".into()))?,
                None => return Err(ViewError::Invalid("row lacks strength".into())),
            },
            confidence: super::parse_tag(row, "confidence")?,
            validity: super::parse_tag(row, "validity")?,
            source_feature: super::required(row, "source_feature")?.to_string(),
            frequency_range_hz,
            algorithm_id: super::required(row, "algorithm_id")?.to_string(),
            algorithm_version: super::required(row, "algorithm_version")?.to_string(),
        })
    }
}

/// `locate/summary` テーブル 1 行分の写し。
/// capture 側区間のみ持つ(source 側は `locate/<src>/<cap>.json`
/// detail または `LocateResult` 経路で取得する)。
#[derive(Debug, Clone, PartialEq)]
pub struct AlignmentRow {
    /// source input id。
    pub source_input_id: String,
    /// capture input id。
    pub capture_input_id: String,
    /// alignment id(detail artifact への参照鍵)。
    pub alignment_id: String,
    /// 合議が採用したか。
    pub accepted: bool,
    /// typed validity。
    pub validity: Validity,
    /// typed confidence。
    pub confidence: Confidence,
    /// 一致した独立 method 数。
    pub agreeing_methods: u64,
    /// capture 側の一致区間(sample)。
    pub capture_range: SampleRange,
}

impl AlignmentRow {
    /// `al.ds.locate` の `alignments[i]` から写す。
    ///
    /// # Errors
    /// 必須列が欠ける場合。
    pub fn from_json(row: &Value) -> Result<Self, ViewError> {
        Ok(Self {
            source_input_id: super::required(row, "source_input_id")?.to_string(),
            capture_input_id: super::required(row, "capture_input_id")?.to_string(),
            alignment_id: super::required(row, "alignment_id")?.to_string(),
            accepted: super::required(row, "status")? == "accepted",
            validity: super::parse_tag(row, "validity")?,
            confidence: super::parse_tag(row, "confidence")?,
            // locate/summary は 9 列を常に emit する(al-dataset
            // stage.rs)。欠損を 0 で補うと accepted 行に見える
            // `SampleRange{0,0}` を捏造するため必須とする。
            agreeing_methods: super::u64_or(row, "agreeing_methods")
                .ok_or_else(|| ViewError::Invalid("row lacks agreeing_methods".into()))?,
            capture_range: SampleRange {
                start: super::u64_or(row, "capture_start")
                    .ok_or_else(|| ViewError::Invalid("row lacks capture_start".into()))?,
                end: super::u64_or(row, "capture_end")
                    .ok_or_else(|| ViewError::Invalid("row lacks capture_end".into()))?,
            },
        })
    }
}

/// `al.ds.features` の写し — `FeatureRecord` を typed のまま保持する。
#[derive(Debug, Clone)]
pub struct FeatureTable {
    /// typed feature record(limit まで)。
    pub records: Vec<FeatureRecord>,
    /// limit で打ち切られたか。
    pub truncated: bool,
}

impl FeatureTable {
    /// `al.ds.features` の result JSON `{records, truncated}` から写す。
    /// `_artifact` 等の拡張 field は serde が未知として無視する。
    ///
    /// # Errors
    /// record が `FeatureRecord` として解釈不能な場合。
    pub fn from_json(v: &Value) -> Result<Self, ViewError> {
        let records_raw = v
            .get("records")
            .and_then(Value::as_array)
            .ok_or_else(|| ViewError::Invalid("result lacks records".into()))?;
        let mut records = Vec::with_capacity(records_raw.len());
        for r in records_raw {
            records.push(
                serde_json::from_value::<FeatureRecord>(r.clone()).map_err(|e| {
                    ViewError::Invalid(format!("feature record does not match schema: {e}"))
                })?,
            );
        }
        Ok(Self {
            records,
            truncated: v.get("truncated").and_then(Value::as_bool).unwrap_or(false),
        })
    }
}
