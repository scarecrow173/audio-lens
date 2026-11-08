//! `al_gui::view`: Job/Run/Analysis/Source の driver-neutral view
//! model (AL-G3)。
//!
//! ここにある型は `al-service`/`al-dataset`/`al-locate`/`al-media` の
//! typed 契約の**写し**であり、f64/DSP/解析を GUI 側で再計算しない
//! (設計: `docs/designs/native-desktop-initial-slices.md`)。
//! RF#387 Component/View renderer の消費対象として、表示意味だけを
//! 保持する。

pub mod analysis;
pub mod analysis_screen;
pub mod dataset;
pub mod desktop;
pub mod job;
pub mod run_screen;
pub mod screen;
pub mod source;
pub mod workspace;

use std::fmt;

use ruviform_ui::view::{
    A11ySpec, ButtonSpec, CellValue, ComponentKey, ComponentSpec, ComponentVisualState, LabelSpec,
    MAX_TABLE_ROWS, PanelSpec, TabSpec, TableSelectionSpec, TableSpec, TabsSelectionSpec, TabsSpec,
    ViewSpec,
};

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

/// typed enum の snake_case tag 文字列(表示用)。
pub(crate) fn tag<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// 文字列セル。
pub(crate) fn text_cell(s: &str) -> Option<CellValue> {
    Some(CellValue::Text(s.to_string()))
}

/// 数値セル。非有限値(NaN・無限大)は欠損セル(`None`)として位置を保つ。
pub(crate) fn num_cell(v: f64) -> Option<CellValue> {
    v.is_finite().then_some(CellValue::Number(v))
}

/// Label component。
pub(crate) fn label(key: &str, text: String) -> ComponentSpec {
    ComponentSpec::Label(LabelSpec {
        key: ComponentKey::from(key),
        text,
        visual: ComponentVisualState::default(),
        a11y: A11ySpec::default(),
    })
}

/// Panel を根とする画面 `ViewSpec`。
pub(crate) fn panel(key: &str, children: Vec<ComponentSpec>) -> ViewSpec {
    ViewSpec {
        root: ComponentSpec::Panel(PanelSpec {
            key: ComponentKey::from(key),
            title: None,
            children,
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        }),
    }
}

/// Tabs を根とする画面 `ViewSpec`。各 tab の画面はその root を子にする。
/// `selection` を与えると tab header から選択でき、その target へ選択 intent を発行する。
pub(crate) fn tabs(
    key: &str,
    selection: Option<&str>,
    tabs: Vec<(&str, &str, ViewSpec)>,
) -> ViewSpec {
    ViewSpec {
        root: ComponentSpec::Tabs(TabsSpec {
            key: ComponentKey::from(key),
            active: 0,
            tabs: tabs
                .into_iter()
                .map(|(key, title, view)| TabSpec {
                    key: ComponentKey::from(key),
                    title: title.to_string(),
                    child: Box::new(view.root),
                })
                .collect(),
            selection: selection.map(|target| TabsSelectionSpec {
                target: target.to_string(),
            }),
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        }),
    }
}

/// 読めなかった節・画面の理由を示す Label。
pub(crate) fn unavailable(key: &str, reason: &impl fmt::Display) -> ComponentSpec {
    label(
        &format!("{key}-unavailable"),
        format!("unavailable: {reason}"),
    )
}

/// Button component(値を伴わない command の起動)。
pub(crate) fn button(
    key: &str,
    label: &str,
    target: &str,
    action: &str,
    params: Option<serde_json::Value>,
) -> ComponentSpec {
    ComponentSpec::Button(ButtonSpec {
        key: ComponentKey::from(key),
        label: label.to_string(),
        target: target.to_string(),
        action: action.to_string(),
        params,
        visual: ComponentVisualState::default(),
        a11y: A11ySpec::default(),
    })
}

/// Table component。`selection` を与えると行選択可能になり、その target へ
/// 選択 intent を発行する。行数・列数は ruviform の上限を超える場合に拒否する
/// (長い Dataset は呼び出し側が bounded query で行を絞る)。
pub(crate) fn table(
    key: &str,
    title: &str,
    columns: &[&str],
    rows: Vec<Vec<Option<CellValue>>>,
    selection: Option<&str>,
) -> Result<ComponentSpec, ViewError> {
    if rows.len() > MAX_TABLE_ROWS {
        return Err(ViewError::Invalid(format!(
            "table {key} has {} rows (> {MAX_TABLE_ROWS})",
            rows.len()
        )));
    }
    Ok(ComponentSpec::Table(TableSpec {
        key: ComponentKey::from(key),
        columns: columns.iter().map(|c| (*c).to_string()).collect(),
        rows,
        selection: selection.map(|target| TableSelectionSpec {
            target: target.to_string(),
        }),
        visual: ComponentVisualState::default(),
        a11y: A11ySpec {
            label: Some(title.to_string()),
            ..A11ySpec::default()
        },
    }))
}
