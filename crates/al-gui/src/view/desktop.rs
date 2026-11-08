//! al-desktop の root 画面(Jobs・Run・Dataset の Tabs)の `ViewSpec` 射影 (AL-G4)。
//!
//! 入力は job の置き場 `base_dir` と、起動時に選んだ job の 2 つだけ。
//! Dataset tab は job の `analysis/` を開き、先頭 input を
//! Source・Spectrogram・Analysis の画面へ写す。各 tab は独立に読み、読めない tab は
//! 理由を示す Label に置き換えて画面全体は開く(`workspace` と同じ規則)。
//! job の選択は ViewSpec に選択操作が無いため起動引数で与える
//! (設計: `docs/designs/native-gui-job-run-mount.md`)。

use std::path::Path;

use al_core::JobId;
use al_service::jobs::{JobInspection, JobService};
use ruviform_ui::view::ViewSpec;

use super::job::{JobRow, RunOutcome};
use super::run_screen::{RUN_PANEL_KEY, job_list, run_screen};
use super::workspace::workspace_screen;
use super::{ViewError, panel, tabs, unavailable};

/// 画面全体(Tabs)の key。
pub const DESKTOP_KEY: &str = "desktop";
/// Jobs tab の key。
pub const JOBS_TAB_KEY: &str = "tab-jobs";
/// Run tab の key。
pub const RUN_TAB_KEY: &str = "tab-run";
/// Dataset tab の key。
pub const DATASET_TAB_KEY: &str = "tab-dataset";
const JOBS_PANEL_KEY: &str = "jobs-view";
const DATASET_PANEL_KEY: &str = "dataset-view";

/// `base_dir` 配下の job 一覧(Jobs)と、`job` の Run 結果(Run)・dataset(Dataset)を
/// 3 つの tab に写した `ViewSpec`。`base_dir` が無い場合は全 tab が理由を示す。
#[must_use]
pub fn desktop_screen(base_dir: Option<&Path>, job: Option<&JobId>) -> ViewSpec {
    let service = base_dir.map(JobService::new);
    let s = service.as_ref();
    tabs(
        DESKTOP_KEY,
        vec![
            (
                JOBS_TAB_KEY,
                "Jobs",
                or_unavailable(JOBS_PANEL_KEY, jobs_panel(s)),
            ),
            (
                RUN_TAB_KEY,
                "Run",
                or_unavailable(RUN_PANEL_KEY, run_panel(s, job)),
            ),
            (
                DATASET_TAB_KEY,
                "Dataset",
                or_unavailable(DATASET_PANEL_KEY, dataset_panel(s, job)),
            ),
        ],
    )
}

fn or_unavailable(key: &str, view: Result<ViewSpec, ViewError>) -> ViewSpec {
    view.unwrap_or_else(|e| panel(key, vec![unavailable(key, &e)]))
}

fn base_dir_unset() -> ViewError {
    ViewError::NotFound("base_dir is not set".into())
}

fn jobs_panel(service: Option<&JobService>) -> Result<ViewSpec, ViewError> {
    let service = service.ok_or_else(base_dir_unset)?;
    let rows: Vec<JobRow> = service.list()?.iter().map(JobRow::from_summary).collect();
    Ok(panel(JOBS_PANEL_KEY, vec![job_list(&rows)?]))
}

fn no_job_selected() -> ViewError {
    ViewError::NotFound("no job selected".into())
}

/// 選択 job の永続 manifest。base_dir 未設定・job 未選択は理由を返す。
fn inspect(service: Option<&JobService>, job: Option<&JobId>) -> Result<JobInspection, ViewError> {
    match (service, job) {
        (None, _) => Err(base_dir_unset()),
        (Some(_), None) => Err(no_job_selected()),
        (Some(s), Some(id)) => Ok(s.inspect(id)?),
    }
}

fn run_panel(service: Option<&JobService>, job: Option<&JobId>) -> Result<ViewSpec, ViewError> {
    let inspection = inspect(service, job)?;
    run_screen(&RunOutcome::from_manifest(&inspection.manifest))
}

fn dataset_panel(service: Option<&JobService>, job: Option<&JobId>) -> Result<ViewSpec, ViewError> {
    let service = service.ok_or_else(base_dir_unset)?;
    let inspection = inspect(Some(service), job)?;
    let input = inspection
        .manifest
        .inputs
        .first()
        .ok_or_else(|| ViewError::NotFound("job has no input".into()))?;
    let session = service.open_dataset(job.ok_or_else(no_job_selected)?)?;
    workspace_screen(&session, input.input_id.as_str())
}
