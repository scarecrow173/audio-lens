//! Analysis の `ViewSpec` 射影 (AL-G4)。
//!
//! `events/candidates` の行と `locate` の alignment を Table に 1 対 1 で
//! 写す。strength などの数値は typed 値のまま表示し、非有限値は欠損セルに
//! する(GUI 側で 0 や推定値を補わない)。行数は呼び出し側が bounded query
//! で絞り、上限を超える場合は拒否する。
//!
//! 節(events・alignment)は別々に作れる。dataset に一方しか無い場合でも、
//! 読めた節だけを表示できるよう節単位で組み立てる。

use ruviform_ui::view::{CellValue, ComponentSpec, ViewSpec};

use super::analysis::{AlignmentRow, EventRow};
use super::{ViewError, label, num_cell, panel, table, tag, text_cell};

/// Analysis 画面 Panel の key。
pub const ANALYSIS_PANEL_KEY: &str = "analysis-view";
/// events 件数 Label の key。
pub const EVENTS_NOTE_KEY: &str = "events-note";
/// events Table の key。
pub const EVENTS_KEY: &str = "events";
/// alignment Table の key。
pub const ALIGNMENTS_KEY: &str = "alignments";

const EVENT_COLUMNS: [&str; 8] = [
    "event",
    "kind",
    "input",
    "start_s",
    "end_s",
    "strength",
    "confidence",
    "validity",
];
const ALIGNMENT_COLUMNS: [&str; 9] = [
    "source",
    "capture",
    "alignment",
    "status",
    "validity",
    "confidence",
    "agreeing_methods",
    "capture_start",
    "capture_end",
];

/// events 節(件数 Label と events Table)。`event_total` は dataset 全体の
/// events 件数で、`events` は表示対象の部分集合。
///
/// # Errors
/// events の行数が Table の上限を超える場合。
pub fn events_section(
    events: &[EventRow],
    event_total: u64,
) -> Result<Vec<ComponentSpec>, ViewError> {
    let rows = events.iter().map(event_cells).collect();
    Ok(vec![
        label(
            EVENTS_NOTE_KEY,
            format!("events {} of {event_total}", events.len()),
        ),
        table(EVENTS_KEY, "events", &EVENT_COLUMNS, rows, None)?,
    ])
}

/// alignment 節(alignment Table)。
///
/// # Errors
/// alignment の行数が Table の上限を超える場合。
pub fn alignments_section(alignments: &[AlignmentRow]) -> Result<ComponentSpec, ViewError> {
    let rows = alignments.iter().map(alignment_cells).collect();
    table(ALIGNMENTS_KEY, "alignments", &ALIGNMENT_COLUMNS, rows, None)
}

/// 節を並べた Analysis 画面の `ViewSpec`。
#[must_use]
pub fn analysis_screen(sections: Vec<ComponentSpec>) -> ViewSpec {
    panel(ANALYSIS_PANEL_KEY, sections)
}

fn event_cells(e: &EventRow) -> Vec<Option<CellValue>> {
    vec![
        text_cell(&e.event_id),
        text_cell(&e.kind),
        text_cell(&e.input_id),
        num_cell(e.start_s),
        e.end_s.and_then(num_cell),
        num_cell(e.strength),
        text_cell(&tag(&e.confidence)),
        text_cell(&tag(&e.validity)),
    ]
}

fn alignment_cells(a: &AlignmentRow) -> Vec<Option<CellValue>> {
    vec![
        text_cell(&a.source_input_id),
        text_cell(&a.capture_input_id),
        text_cell(&a.alignment_id),
        text_cell(if a.accepted { "accepted" } else { "rejected" }),
        text_cell(&tag(&a.validity)),
        text_cell(&tag(&a.confidence)),
        num_cell(a.agreeing_methods as f64),
        num_cell(a.capture_range.start as f64),
        num_cell(a.capture_range.end as f64),
    ]
}
