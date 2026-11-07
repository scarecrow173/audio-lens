//! `al_gui::view`: Job/Run/Analysis/Source の driver-neutral view
//! model (AL-G3)。
//!
//! ここにある型は `al-service`/`al-dataset`/`al-locate`/`al-media` の
//! typed 契約の**写し**であり、f64/DSP/解析を GUI 側で再計算しない
//! (設計: `docs/designs/native-desktop-initial-slices.md`)。
//! RF#387 Component/View renderer の消費対象として、表示意味だけを
//! 保持する。

pub mod analysis;
pub mod dataset;
pub mod job;
pub mod screen;
pub mod source;

use std::fmt;

/// view model の写像/開放エラー。
#[non_exhaustive]
#[derive(Debug)]
pub enum ViewError {
    /// JSON 行が期待スキーマを満たさない。
    Invalid(String),
    /// 要求した入力/artifact が存在しない。
    NotFound(String),
    /// `al-service` 層のエラー。
    Service(al_service::ServiceError),
}

impl fmt::Display for ViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(m) => write!(f, "invalid view data: {m}"),
            Self::NotFound(m) => write!(f, "not found: {m}"),
            Self::Service(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ViewError {}

impl From<al_service::ServiceError> for ViewError {
    fn from(e: al_service::ServiceError) -> Self {
        Self::Service(e)
    }
}

pub(crate) fn required<'a>(row: &'a serde_json::Value, key: &str) -> Result<&'a str, ViewError> {
    row.get(key)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ViewError::Invalid(format!("row lacks {key}")))
}

pub(crate) fn f64_or(row: &serde_json::Value, key: &str) -> Option<f64> {
    row.get(key).and_then(serde_json::Value::as_f64)
}

pub(crate) fn u64_or(row: &serde_json::Value, key: &str) -> Option<u64> {
    row.get(key).and_then(serde_json::Value::as_u64)
}

/// snake_case 文字列を serde で typed enum へ写す
/// (`Confidence`/`Validity`/`StageStatus` 共通)。
pub(crate) fn parse_tag<T: serde::de::DeserializeOwned>(
    row: &serde_json::Value,
    key: &str,
) -> Result<T, ViewError> {
    let raw = required(row, key)?;
    serde_json::from_value(serde_json::Value::String(raw.to_string()))
        .map_err(|_| ViewError::Invalid(format!("unknown {key} tag {raw:?}")))
}
