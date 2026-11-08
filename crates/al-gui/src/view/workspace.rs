//! Dataset の Source・Spectrogram・Analysis を 1 つの Tabs 画面へ束ねる (AL-G4)。
//!
//! 3 画面は時間軸の単位が異なる(Source は audio frame、Spectrogram は
//! STFT frame)ため、同じ Panel に並べず Tabs の子に分ける。Source 以外は
//! dataset に無いことが普通に起こりうるため、読めない場合はその tab に
//! 理由を示す Label を置き、画面全体は開く。
//! Tabs は pointer で選択でき、選択は `command::WORKSPACE_TARGET` の
//! `tab-select` intent として発行する(設計: `docs/designs/native-gui-workspace-tabs-selection.md`)。

use std::ops::Range;

use al_service::dataset::DatasetSession;
use ruviform_ui::view::{MAX_TABLE_ROWS, ViewSpec};
use ruviform_viz::{Aggregate, RowSpec, TileRequest};
use serde_json::Value;

use super::analysis::{AlignmentRow, EventRow};
use super::analysis_screen::{
    ALIGNMENTS_KEY, EVENTS_KEY, alignments_section, analysis_screen, events_section,
};
use super::screen::{SPECTROGRAM_PANEL_KEY, source_screen, spectrogram_screen};
use super::source::{SpectrumView, WaveformView};
use super::{ViewError, panel, tabs, unavailable};
use crate::command;

/// 画面全体(Tabs)の key。
pub const WORKSPACE_KEY: &str = "workspace";
/// workspace の Tabs の tab 数(Source・Spectrogram・Analysis)。`tab-select` の index 上限。
pub const WORKSPACE_TAB_COUNT: usize = 3;
/// Source 画面の表示 bin 数(全長を一度に表示する既定値)。
const SCREEN_BINS: usize = 1024;
/// Spectrogram の出力解像度。`rows × cols` は `MAX_SPECTROGRAM_CELLS` 以下。
const SPECTROGRAM_ROWS: usize = 128;
const SPECTROGRAM_COLS: usize = 512;
/// Spectrogram が読む STFT frame の上限(先頭から。全量の一括読み出しを避ける)。
const SPECTROGRAM_MAX_FRAMES: u64 = 4096;

/// `input_id` の Source を主画面とし、Spectrogram と Analysis を tab に加えた `ViewSpec`。
///
/// # Errors
/// Source の入力が開けない場合。Spectrogram と Analysis の失敗は各 tab に示す。
pub fn workspace_screen(session: &DatasetSession, input_id: &str) -> Result<ViewSpec, ViewError> {
    let source = source_panel(session, input_id)?;
    let spectrogram = spectrogram_panel(session, input_id).unwrap_or_else(|e| {
        panel(
            SPECTROGRAM_PANEL_KEY,
            vec![unavailable(SPECTROGRAM_PANEL_KEY, &e)],
        )
    });
    Ok(tabs(
        WORKSPACE_KEY,
        Some(command::WORKSPACE_TARGET),
        vec![
            ("tab-source", "Source", source),
            ("tab-spectrogram", "Spectrogram", spectrogram),
            ("tab-analysis", "Analysis", analysis_panel(session)),
        ],
    ))
}

fn source_panel(session: &DatasetSession, input_id: &str) -> Result<ViewSpec, ViewError> {
    let view = WaveformView::open(session, input_id)?;
    let duration_s = view.frames as f64 / f64::from(view.sample_rate);
    source_screen(&view, 0, 0.0, duration_s, SCREEN_BINS)
}

fn spectrogram_panel(session: &DatasetSession, input_id: &str) -> Result<ViewSpec, ViewError> {
    let view = SpectrumView::open(session, input_id, 0)?;
    let tile = view.tile(&TileRequest {
        frames: 0..view.info.frames.min(SPECTROGRAM_MAX_FRAMES),
        rows: RowSpec::uniform(view.info.bins, SPECTROGRAM_ROWS),
        cols: SPECTROGRAM_COLS,
        aggregate: Aggregate::Max,
    })?;
    spectrogram_screen(&tile, display_range(&tile.values), view.info.frames)
}

/// 表示する magnitude の範囲(consumer が決める): 0 から表示 tile の有限最大値まで。
/// 有限値が無い(全 gap)か最大が 0 の場合は `0..1`。
fn display_range(values: &[f64]) -> Range<f64> {
    let max = values
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .fold(0.0, f64::max);
    0.0..if max > 0.0 { max } else { 1.0 }
}

/// Analysis tab。節ごとに読み、読めない節は理由の Label に置き換える。
fn analysis_panel(session: &DatasetSession) -> ViewSpec {
    let events = events_of(session)
        .and_then(|(rows, total)| events_section(&rows, total))
        .unwrap_or_else(|e| vec![unavailable(EVENTS_KEY, &e)]);
    let alignments = alignments_of(session)
        .and_then(|rows| alignments_section(&rows))
        .unwrap_or_else(|e| unavailable(ALIGNMENTS_KEY, &e));
    analysis_screen([events, vec![alignments]].concat())
}

fn events_of(session: &DatasetSession) -> Result<(Vec<EventRow>, u64), ViewError> {
    let events = session.events(0, MAX_TABLE_ROWS as u64, None, None)?;
    let rows = rows_of(&events, "rows")?
        .iter()
        .map(EventRow::from_json)
        .collect::<Result<Vec<_>, _>>()?;
    Ok((rows, events["row_count"].as_u64().unwrap_or(0)))
}

fn alignments_of(session: &DatasetSession) -> Result<Vec<AlignmentRow>, ViewError> {
    let locate = session.locate()?;
    rows_of(&locate, "alignments")?
        .iter()
        .map(AlignmentRow::from_json)
        .collect()
}

fn rows_of<'a>(v: &'a Value, key: &str) -> Result<&'a [Value], ViewError> {
    v.get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| ViewError::Invalid(format!("result lacks {key}")))
}
