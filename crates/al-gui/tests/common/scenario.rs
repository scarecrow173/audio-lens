//! ruviform#464 Step C / AL#127: driver-neutral `SemanticGuiTest`
//! scenario helpers。
//!
//! AL が Own する scenario は通常の Rust 関数としてここに置き、
//! `FakeGuiTest`(headless)と `GpuiGuiTest`(gpui-test)の両 adapter で
//! 同一契約を実行する。scenario・report の語彙は ruviform-ui-test の
//! contract 型だけを使い、GPUI 型はこの module に現れない。

use std::collections::BTreeSet;

use al_core::JobId;
use al_gui::view::desktop::{DESKTOP_KEY, RESUME_KEY, desktop_screen};
use al_gui::view::run_screen::JOBS_KEY;
use al_gui::view::screen::{SEEK_KEY, source_screen};
use al_testkit::temp::TempDir;
use ruviform_ui::WindowId;
use ruviform_ui::behavior::{CommandIntent, ValueInput};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{ComponentKey, ComponentSnapshot, ViewSpec};
use ruviform_ui_test::{
    CaseId, GuiAction, MintedIntent, RunId, ScenarioCapability, ScenarioError, ScenarioId,
    ScenarioReport, ScenarioTarget, SemanticGuiTest, TargetQuery, parity_gesture_scenario,
};
use serde_json::{Value, json};

use super::{fixture_view, make_job};

/// AL product namespace の scenario/case ID。
pub const SCENARIO_DESKTOP_JOBS: &str = "al.desktop.jobs";
pub const CASE_JOB_SELECT_RESUME: &str = "job-select-resume";
pub const SCENARIO_SOURCE_SEEK: &str = "al.source.seek";
pub const CASE_SEEK_COMMIT: &str = "seek-commit";

/// production composition root(`bin/al-desktop.rs` の `mount_host`)と
/// 同じ view instance id。scenario の target 解決はこの id を使う。
pub const AL_VIEW: u64 = 1;

/// `AL_VIEW` の `ViewInstanceId`。
pub fn al_view() -> ViewInstanceId {
    ViewInstanceId::new(AL_VIEW).expect("nonzero")
}

/// job fixture 付きの desktop 画面 spec。`TempDir` は spec 構築後も
/// 呼び出し側で保持する(dataset session の基底 dir)。
pub fn desktop_spec() -> (TempDir, ViewSpec, JobId) {
    let dir = TempDir::new("al-scenario-desktop");
    let base = dir.join("jobs");
    let job_id = make_job(&base, &dir.join("cap.wav"));
    let spec = desktop_screen(Some(&base), Some(&job_id));
    (dir, spec, job_id)
}

/// Source 画面 fixture(`tests/gpui_parity.rs` と同一の waveform)。
pub fn source_spec() -> ViewSpec {
    let view = fixture_view();
    source_screen(&view, 0, 0.0, 0.1, 64).expect("screen spec")
}

/// desktop 画面での job 選択→Run tab→Resume 起動 scenario。
///
/// `SemanticGuiTest` port のみで記述する driver-neutral な流れ:
/// Jobs Table の行 0 を `SelectRow` → desktop root Tabs の Run tab を
/// `SelectTab` → Resume Button を `Invoke`。各操作の間で `settle` し、
/// command intent を drain して返す。Jobs Table の選択状態は
/// read-only `observe` で確認する。
pub struct DesktopJobsOutcome {
    /// pin 済み target(Jobs Table へ narrow)。
    pub target: ScenarioTarget,
    /// 発行された command intent(選択→tab→resume の順)。
    pub intents: Vec<MintedIntent>,
    /// Jobs Table が観測した選択行。
    pub jobs_selected: Option<usize>,
}

/// [`DesktopJobsOutcome`] を得る scenario 本体。driver 非依存。
///
/// # Errors
/// target 解決・capability pre-flight・各 port 操作の `ScenarioError`。
pub fn desktop_jobs_scenario(
    gui: &mut impl SemanticGuiTest,
    window: WindowId,
) -> Result<DesktopJobsOutcome, ScenarioError> {
    let jobs = ComponentKey::from(JOBS_KEY);
    let target =
        gui.resolve_target(&TargetQuery::new(window, al_view()).component(jobs.clone()))?;
    gui.require_capabilities(&BTreeSet::from([ScenarioCapability::SemanticAction]))?;

    gui.perform(
        &target,
        GuiAction::SelectRow {
            key: jobs.clone(),
            row: 0,
        },
    )?;
    gui.settle()?;
    let mut intents = gui.drain_intents();
    let jobs_selected = gui.observe(&target, &jobs)?.selected;

    gui.perform(
        &target,
        GuiAction::SelectTab {
            key: ComponentKey::from(DESKTOP_KEY),
            index: 1,
        },
    )?;
    gui.settle()?;
    intents.extend(gui.drain_intents());

    gui.perform(
        &target,
        GuiAction::Invoke {
            key: ComponentKey::from(RESUME_KEY),
        },
    )?;
    gui.settle()?;
    intents.extend(gui.drain_intents());

    Ok(DesktopJobsOutcome {
        target,
        intents,
        jobs_selected,
    })
}

/// Source 画面の Seek へ ruviform 共通 `parity_gesture_scenario`
/// (Begin→Preview→Commit)を適用する scenario。
pub struct SourceSeekOutcome {
    /// pin 済み target(Seek へ narrow)。
    pub target: ScenarioTarget,
    /// gesture が発行した intent 列。
    pub intents: Vec<MintedIntent>,
    /// gesture 後の Seek snapshot。
    pub snapshot: ComponentSnapshot,
}

/// [`SourceSeekOutcome`] を得る scenario 本体。`plain` は Seek の
/// plain-domain 値(秒)。driver 非依存。
///
/// # Errors
/// target 解決・capability pre-flight・gesture 操作の `ScenarioError`。
pub fn source_seek_scenario(
    gui: &mut impl SemanticGuiTest,
    window: WindowId,
    plain: f64,
) -> Result<SourceSeekOutcome, ScenarioError> {
    let seek = ComponentKey::from(SEEK_KEY);
    let target =
        gui.resolve_target(&TargetQuery::new(window, al_view()).component(seek.clone()))?;
    gui.require_capabilities(&BTreeSet::from([ScenarioCapability::SemanticAction]))?;
    let (intents, snapshot) =
        parity_gesture_scenario(gui, &target, &seek, "al-seek-1", ValueInput::Plain(plain))?;
    Ok(SourceSeekOutcome {
        target,
        intents,
        snapshot,
    })
}

/// `MintedIntent` の `CommandIntent::Action` を
/// `(target, action, params)` へ射影する(AL product oracle)。
pub fn command_actions(intents: &[MintedIntent]) -> Vec<(String, String, Option<Value>)> {
    intents
        .iter()
        .filter_map(|m| match &m.intent {
            CommandIntent::Action {
                target,
                action,
                params,
                ..
            } => Some((target.clone(), action.clone(), params.clone())),
            _ => None,
        })
        .collect()
}

/// adapter の実測 `driver`/`capabilities` を記録した
/// `ScenarioReport` の骨格。`verdict`/`expected`/`observed` は
/// 証拠に応じて caller が設定する — fake の pass を実機 pass へ
/// 昇格させないため、capabilities は adapter の申告値をそのまま使う。
pub fn al_scenario_report(
    gui: &impl SemanticGuiTest,
    run: &str,
    scenario: &str,
    case: &str,
) -> ScenarioReport {
    let mut report = ScenarioReport::new(
        RunId::new(run),
        ScenarioId::new(scenario),
        Some(CaseId::new(case)),
        gui.driver(),
    );
    report.capabilities = gui.capabilities();
    report.required = BTreeSet::from([ScenarioCapability::SemanticAction]);
    report.environment = Some(json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    }));
    report
}
