//! ruviform#464 Step C / AL#127: driver-neutral `SemanticGuiTest`
//! scenario の Fake adapter 採用検証。
//!
//! AL が Own する scenario(`tests/common/scenario.rs`)を `FakeGuiTest`
//! で実行し、Pass が取れる範囲と、能力未宣言の項目が `ScenarioReport`
//! の Unsupported/NotRun として正直に記録されることを検証する。
//! fake の pass を実 OS 入力・pixel capture の証拠へ昇格させない。

mod common;

use std::collections::BTreeSet;

use al_gui::command;
use al_gui::view::run_screen::JOBS_KEY;
use al_gui::view::screen::SEEK_TARGET;
use common::scenario::{
    CASE_JOB_SELECT_RESUME, CASE_SEEK_COMMIT, SCENARIO_DESKTOP_JOBS, SCENARIO_SOURCE_SEEK,
    al_scenario_report, al_view, command_actions, desktop_jobs_scenario, desktop_spec,
    source_seek_scenario, source_spec,
};
use ruviform_ui::WindowId;
use ruviform_ui::behavior::CommandIntent;
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::ComponentKey;
use ruviform_ui_test::golden::{BaselineStore, GoldenKey, GoldenVerdict};
use ruviform_ui_test::{
    CaptureOutcome, FakeGuiTest, ScenarioCapability, ScenarioDriver, ScenarioError, ScenarioTarget,
    ScenarioVerdict, SemanticGuiTest, TargetQuery, VisualGuiTest,
};
use serde_json::json;

/// desktop 画面 spec を Fake adapter の 1 window に mount する。
/// 戻り値は `(window, view, job_id, _tempdir)` — tempdir は spec が
/// 参照する job 配置を保持するため生存させる。
fn mount_desktop(
    gui: &mut FakeGuiTest,
) -> (
    WindowId,
    ViewInstanceId,
    al_core::JobId,
    al_testkit::temp::TempDir,
) {
    let (dir, spec, job_id) = desktop_spec();
    let window = gui.open_window();
    gui.mount_view(window, al_view(), spec)
        .expect("mount desktop");
    (window, al_view(), job_id, dir)
}

#[test]
fn fake_desktop_jobs_scenario_passes_with_honest_report() {
    let mut gui = FakeGuiTest::new();
    let (window, _view, job_id, _dir) = mount_desktop(&mut gui);

    let outcome = desktop_jobs_scenario(&mut gui, window).expect("desktop jobs scenario");

    assert_eq!(outcome.jobs_selected, Some(0));
    assert_eq!(
        command_actions(&outcome.intents),
        vec![
            (
                command::JOB_TARGET.to_string(),
                command::ACTION_SELECT.to_string(),
                Some(json!({ "row": 0 })),
            ),
            (
                command::DESKTOP_TARGET.to_string(),
                command::ACTION_TAB_SELECT.to_string(),
                Some(json!({ "index": 1 })),
            ),
            (
                command::JOB_TARGET.to_string(),
                command::ACTION_RESUME.to_string(),
                Some(json!({ "job_id": job_id.as_str() })),
            ),
        ],
        "select → tab-select → resume intents"
    );

    let mut report = al_scenario_report(
        &gui,
        "run-fake-desktop-jobs",
        SCENARIO_DESKTOP_JOBS,
        CASE_JOB_SELECT_RESUME,
    );
    report.targets = vec![outcome.target];
    report.expected = Some(json!({
        "actions": [
            { "target": command::JOB_TARGET, "action": command::ACTION_SELECT, "params": { "row": 0 } },
            { "target": command::DESKTOP_TARGET, "action": command::ACTION_TAB_SELECT, "params": { "index": 1 } },
            { "target": command::JOB_TARGET, "action": command::ACTION_RESUME },
        ],
        "jobs_selected": 0,
    }));
    report.observed = Some(json!({
        "actions": command_actions(&outcome.intents),
        "jobs_selected": outcome.jobs_selected,
    }));
    report.evidence = vec!["crates/al-gui/tests/scenario.rs".to_string()];
    report.verdict = ScenarioVerdict::Pass;

    assert_eq!(report.driver, ScenarioDriver::Fake);
    assert_eq!(report.verdict, ScenarioVerdict::Pass);
    assert_eq!(report.verdict.lane_status(), "pass");
    assert_eq!(
        report.capabilities,
        BTreeSet::from([
            ScenarioCapability::SemanticAction,
            ScenarioCapability::MultiWindow,
        ])
    );
    // fake の semantic pass は強い evidence level を主張しない。
    for absent in [
        ScenarioCapability::SimulatedPointerOrKeyboard,
        ScenarioCapability::RealOsPointerOrKeyboard,
        ScenarioCapability::Ime,
        ScenarioCapability::VisualCapture,
        ScenarioCapability::NativePluginDock,
    ] {
        assert!(
            !report.capabilities.contains(&absent),
            "{absent:?} must not be claimed by the fake adapter"
        );
    }
}

#[test]
fn fake_source_seek_scenario_commits_transport_seek() {
    let mut gui = FakeGuiTest::new();
    let spec = source_spec();
    let window = gui.open_window();
    gui.mount_view(window, al_view(), spec)
        .expect("mount source");

    let outcome = source_seek_scenario(&mut gui, window, 0.05).expect("source seek scenario");

    let commits: Vec<_> = outcome
        .intents
        .iter()
        .filter(|m| matches!(m.intent, CommandIntent::Commit { .. }))
        .collect();
    assert_eq!(commits.len(), 1, "one commit intent: {:?}", outcome.intents);
    match &commits[0].intent {
        CommandIntent::Commit { target, .. } => assert_eq!(target, SEEK_TARGET),
        _ => unreachable!(),
    }
    let committed = outcome.snapshot.committed.expect("seek committed value");
    assert!(
        (committed.plain - 0.05).abs() < 1e-12,
        "committed plain value: {}",
        committed.plain
    );

    let report = al_scenario_report(
        &gui,
        "run-fake-source-seek",
        SCENARIO_SOURCE_SEEK,
        CASE_SEEK_COMMIT,
    );
    assert_eq!(report.driver, ScenarioDriver::Fake);
    assert_eq!(report.verdict, ScenarioVerdict::NotRun);
}

#[test]
fn fake_target_resolution_covers_window_view_component_and_pins() {
    let mut gui = FakeGuiTest::new();
    let (window, view, _job_id, _dir) = mount_desktop(&mut gui);
    let spec = source_spec();
    let jobs = ComponentKey::from(JOBS_KEY);

    // WindowId + ViewInstanceId + ComponentKey の解決が AL の view
    // 構造で機能することを確認する。
    let target: ScenarioTarget = gui
        .resolve_target(&TargetQuery::new(window, view).component(jobs.clone()))
        .expect("resolve jobs target");
    assert_eq!(target.window, window);
    assert_eq!(target.view, view);
    assert_eq!(target.component, Some(jobs.clone()));

    // 未 mount の component / view / window は明示 error。
    let err = gui
        .resolve_target(
            &TargetQuery::new(window, view).component(ComponentKey::from("no-such-key")),
        )
        .expect_err("unknown component must fail");
    assert!(matches!(err, ScenarioError::UnknownComponent { .. }));

    let err = gui
        .resolve_target(&TargetQuery::new(
            window,
            ViewInstanceId::new(999).expect("nonzero"),
        ))
        .expect_err("unknown view must fail");
    assert!(matches!(err, ScenarioError::NoSuchView { .. }));

    let err = gui
        .resolve_target(&TargetQuery::new(
            WindowId::new(999).expect("nonzero"),
            view,
        ))
        .expect_err("unknown window must fail");
    assert!(matches!(err, ScenarioError::NoSuchWindow { .. }));

    // query の pin 照合: 誤った generation pin は StaleGeneration。
    let err = gui
        .resolve_target(&TargetQuery::new(window, view).generation(target.generation + 1))
        .expect_err("wrong generation pin must fail");
    assert!(matches!(err, ScenarioError::StaleGeneration { .. }));

    // detach で generation が進み、pin 済み target は stale になる。
    gui.detach_view(window, view, "test-detach")
        .expect("detach");
    let err = gui
        .observe(&target, &jobs)
        .expect_err("detached view must stale the target");
    assert!(
        matches!(err, ScenarioError::StaleGeneration { .. }),
        "expected StaleGeneration, got {err:?}"
    );

    // 同じ view id の再 mount は新しい attach epoch を持ち、
    // 旧 target は StaleEpoch で拒否される。
    gui.mount_view(window, view, spec).expect("remount");
    let err = gui
        .observe(&target, &jobs)
        .expect_err("remounted view must stale the epoch");
    assert!(
        matches!(err, ScenarioError::StaleEpoch { .. }),
        "expected StaleEpoch, got {err:?}"
    );
}

#[test]
fn fake_unrunnable_items_report_unsupported_not_run_not_pass() {
    let mut gui = FakeGuiTest::new();
    let (window, view, _job_id, _dir) = mount_desktop(&mut gui);
    let target = gui
        .resolve_target(&TargetQuery::new(window, view).component(ComponentKey::from(JOBS_KEY)))
        .expect("resolve");

    // pre-flight: fake が申告しない能力は Unsupported error。
    for capability in [
        ScenarioCapability::SimulatedPointerOrKeyboard,
        ScenarioCapability::RealOsPointerOrKeyboard,
        ScenarioCapability::Ime,
        ScenarioCapability::VisualCapture,
        ScenarioCapability::NativePluginDock,
    ] {
        let err = gui
            .require_capabilities(&BTreeSet::from([capability]))
            .expect_err("capability must be unsupported");
        assert!(
            matches!(err, ScenarioError::Unsupported { .. }),
            "{capability:?}: {err:?}"
        );
    }

    // pixel capture は測定不能を Unsupported として返す
    // (draw plan や semantic snapshot を pixel 証拠にしない)。
    let outcome = gui
        .capture_frame(&target)
        .expect("capture_frame returns an honest outcome");
    assert!(
        matches!(outcome, CaptureOutcome::Unsupported { .. }),
        "fake capture must be Unsupported: {outcome:?}"
    );

    // golden step も Unsupported verdict を返し、pass に昇格しない。
    let dir = al_testkit::temp::TempDir::new("al-scenario-golden");
    let store = BaselineStore::new(dir.to_path_buf());
    let key = GoldenKey {
        story_id: "al.desktop.jobs".to_string(),
        theme: "test".to_string(),
        state: "idle".to_string(),
        driver_class: "fake".to_string(),
        os_class: std::env::consts::OS.to_string(),
        gpu_class: None,
        scale_factor: 1.0,
        font_class: None,
        revision: "test".to_string(),
    };
    let comparison = gui
        .golden_check(&target, &key, None, &store)
        .expect("golden_check returns a report");
    assert_eq!(comparison.report.verdict, GoldenVerdict::Unsupported);
    assert_eq!(
        ScenarioVerdict::from(comparison.report.verdict),
        ScenarioVerdict::Unsupported
    );

    // ScenarioReport: driver=Fake・verdict=Unsupported・
    // lane_status=unsupported のまま記録し、実機 pass と偽らない。
    let mut report = al_scenario_report(
        &gui,
        "run-fake-unsupported",
        SCENARIO_DESKTOP_JOBS,
        CASE_JOB_SELECT_RESUME,
    );
    report.required = BTreeSet::from([
        ScenarioCapability::RealOsPointerOrKeyboard,
        ScenarioCapability::VisualCapture,
    ]);
    report.verdict = ScenarioVerdict::Unsupported;
    report.detail =
        Some("real OS input and pixel capture are not provided by the fake adapter".to_string());
    assert_eq!(report.driver, ScenarioDriver::Fake);
    assert_eq!(report.verdict, ScenarioVerdict::Unsupported);
    assert_eq!(report.verdict.lane_status(), "unsupported");
    assert!(
        !report
            .capabilities
            .contains(&ScenarioCapability::VisualCapture)
    );
    assert!(
        !report
            .capabilities
            .contains(&ScenarioCapability::RealOsPointerOrKeyboard)
    );
}
