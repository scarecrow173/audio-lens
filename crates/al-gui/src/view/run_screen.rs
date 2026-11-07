//! Job/Run の `ViewSpec` 射影 (AL-G4)。
//!
//! 状態テキストは [`RunOutcome`] の stage 状態を数えた結果を Label に、
//! stage 表(stage・status・duration・error)は Table に写す。job 一覧は
//! 一列の Table(List)とする。値は typed 写しのまま表示し、stage の
//! 失敗理由と実行時間は欠落させない。

use al_core::StageStatus;
use ruviform_ui::view::{ComponentSpec, ViewSpec};

use super::job::{JobRow, RunOutcome};
use super::{ViewError, label, num_cell, panel, table, tag, text_cell};

/// Run 画面 Panel の key。
pub const RUN_PANEL_KEY: &str = "run-view";
/// 状態テキスト Label の key。
pub const RUN_STATUS_KEY: &str = "run-status";
/// stage 表 Table の key。
pub const STAGES_KEY: &str = "stages";
/// job 一覧 Table(List)の key。
pub const JOBS_KEY: &str = "jobs";

const STAGE_COLUMNS: [&str; 4] = ["stage", "status", "duration_ms", "error"];

/// `RunOutcome` の Run 画面(状態テキスト + stage 表)の `ViewSpec`。
///
/// # Errors
/// stage 数が Table の上限を超える場合。
pub fn run_screen(run: &RunOutcome) -> Result<ViewSpec, ViewError> {
    let total = run.stages.len();
    let count = |s: StageStatus| run.stages.iter().filter(|r| r.status == s).count();
    let status = label(
        RUN_STATUS_KEY,
        format!(
            "job {}: completed {}/{total}, failed {}",
            run.job_id,
            count(StageStatus::Completed),
            count(StageStatus::Failed)
        ),
    );
    let rows = run
        .stages
        .iter()
        .zip(&run.errors)
        .zip(&run.durations_ms)
        .map(|((stage, error), duration)| {
            vec![
                text_cell(&tag(&stage.kind)),
                text_cell(&tag(&stage.status)),
                duration.and_then(|ms| num_cell(ms as f64)),
                error.as_deref().and_then(text_cell),
            ]
        })
        .collect();
    Ok(panel(
        RUN_PANEL_KEY,
        vec![status, table(STAGES_KEY, "stages", &STAGE_COLUMNS, rows)?],
    ))
}

/// job 一覧(List)。一列の Table で job id を並べる。
///
/// # Errors
/// job 数が Table の上限を超える場合。
pub fn job_list(jobs: &[JobRow]) -> Result<ComponentSpec, ViewError> {
    let rows = jobs
        .iter()
        .map(|job| vec![text_cell(&job.job_id)])
        .collect();
    table(JOBS_KEY, "jobs", &["job"], rows)
}
