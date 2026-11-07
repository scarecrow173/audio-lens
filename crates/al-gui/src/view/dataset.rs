//! Dataset metadata の view model — `al.ds.timeline`/`overview` の
//! typed 写し。

use serde_json::Value;

use super::ViewError;

/// `al.ds.timeline` の `inputs[i]` 1 行分。
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineInputRow {
    /// input id。
    pub input_id: String,
    /// audio 総フレーム数。
    pub frames: u64,
    /// サンプルレート。
    pub sample_rate: u32,
    /// チャネル数。
    pub channels: u64,
    /// 長さ(秒。producer 計算値をそのまま保持)。
    pub duration_s: f64,
}

/// `al.ds.timeline` 結果 — 入力別 audio 情報 + 件数。
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineView {
    /// 入力行。
    pub inputs: Vec<TimelineInputRow>,
    /// `events/candidates` の行数。
    pub event_count: u64,
    /// `locate/summary` の行数。
    pub locate_count: u64,
}

impl TimelineView {
    /// `DatasetSession::timeline` の result JSON から写す。
    ///
    /// # Errors
    /// `inputs` が無い、または必須列が欠ける場合。
    pub fn from_json(v: &Value) -> Result<Self, ViewError> {
        let inputs_raw = v
            .get("inputs")
            .and_then(Value::as_array)
            .ok_or_else(|| ViewError::Invalid("timeline lacks inputs".into()))?;
        let mut inputs = Vec::with_capacity(inputs_raw.len());
        for row in inputs_raw {
            inputs.push(TimelineInputRow {
                input_id: super::required(row, "input_id")?.to_string(),
                frames: super::u64_or(row, "frames")
                    .ok_or_else(|| ViewError::Invalid("row lacks frames".into()))?,
                sample_rate: super::u64_or(row, "sample_rate")
                    .and_then(|r| u32::try_from(r).ok())
                    .ok_or_else(|| ViewError::Invalid("row lacks sample_rate".into()))?,
                channels: super::u64_or(row, "channels")
                    .ok_or_else(|| ViewError::Invalid("row lacks channels".into()))?,
                duration_s: super::f64_or(row, "duration_s")
                    .ok_or_else(|| ViewError::Invalid("row lacks duration_s".into()))?,
            });
        }
        Ok(Self {
            inputs,
            event_count: super::u64_or(v, "event_count").unwrap_or(0),
            locate_count: super::u64_or(v, "locate_count").unwrap_or(0),
        })
    }
}
