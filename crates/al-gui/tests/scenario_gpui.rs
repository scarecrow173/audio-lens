//! ruviform#464 Step C / AL#127: driver-neutral `SemanticGuiTest`
//! scenario の GPUI adapter(`GpuiGuiTest`)採用検証。
//!
//! `tests/common/scenario.rs` の AL scenario を `GpuiGuiTest`
//! (`GpuiObserver` 経由、`gpui-test` feature)で実行し、Fake adapter
//! と同一契約が動くこと、能力宣言が `GpuiSimulated` として正直で
//! あること、実行不能項目が Unsupported/NotRun のままであることを
//! 検証する。`gpui::TestAppContext` 等の GPUI 型は adapter 内部に
//! 閉じ、scenario は contract 型だけを使う。

#![cfg(feature = "gpui-test")]

mod common;

use std::collections::BTreeSet;

use al_gui::command;
use al_gui::view::run_screen::JOBS_KEY;
use common::scenario::{
    CASE_JOB_SELECT_RESUME, SCENARIO_DESKTOP_JOBS, al_scenario_report, al_view, command_actions,
    desktop_jobs_scenario, desktop_spec, source_seek_scenario, source_spec,
};
use ruviform_ui::behavior::CommandIntent;
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{AttachEpochAllocator, ComponentKey, ViewHost, ViewSpec};
use ruviform_ui::{WindowId, WindowSpec};
use ruviform_ui_gpui::{GpuiGuiTest, ViewMount};
use ruviform_ui_test::golden::{BaselineStore, GoldenKey, GoldenVerdict};
use ruviform_ui_test::{
    FakeGuiTest, ScenarioCapability, ScenarioDriver, ScenarioError, ScenarioTarget,
    ScenarioVerdict, SemanticGuiTest, TargetQuery, VisualGuiTest,
};
use serde_json::json;

/// AL の desktop 画面 spec を GPUI window に mount する
/// (production の `mount_host` + `create_view_window` と同じ形)。
fn mount_gpui(gui: &mut GpuiGuiTest, spec: ViewSpec) -> WindowId {
    let mut host = ViewHost::new(al_view(), AttachEpochAllocator::new().mint());
    host.mount(spec).expect("mount");
    host.drain_events();
    let window = gui
        .observer_mut()
        .driver_mut()
        .create_view_window(&WindowSpec::default(), ViewMount { host, theme: None })
        .expect("create_view_window");
    gui.settle().expect("settle mount");
    window
}

#[test]
fn gpui_desktop_jobs_scenario_passes_with_declared_capabilities() {
    let (_dir, spec, job_id) = desktop_spec();
    let mut gui = GpuiGuiTest::new();
    let window = mount_gpui(&mut gui, spec);

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
        "run-gpui-desktop-jobs",
        SCENARIO_DESKTOP_JOBS,
        CASE_JOB_SELECT_RESUME,
    );
    report.targets = vec![outcome.target];
    report.observed = Some(json!({
        "actions": command_actions(&outcome.intents),
        "jobs_selected": outcome.jobs_selected,
    }));
    report.verdict = ScenarioVerdict::Pass;

    assert_eq!(gui.driver(), ScenarioDriver::GpuiSimulated);
    assert_eq!(report.driver, ScenarioDriver::GpuiSimulated);
    assert_eq!(report.verdict, ScenarioVerdict::Pass);
    assert_eq!(report.verdict.lane_status(), "pass");
    // GPUI adapter が正直に申告する能力は semantic + multi-window のみ。
    // dispatch_input は hit-test を通らない semantic 注入であり、
    // simulated pointer / real OS / pixel の証拠へ昇格しない。
    assert_eq!(
        report.capabilities,
        BTreeSet::from([
            ScenarioCapability::SemanticAction,
            ScenarioCapability::MultiWindow,
        ])
    );
    for absent in [
        ScenarioCapability::SimulatedPointerOrKeyboard,
        ScenarioCapability::RealOsPointerOrKeyboard,
        ScenarioCapability::Ime,
        ScenarioCapability::VisualCapture,
        ScenarioCapability::NativePluginDock,
    ] {
        assert!(
            !report.capabilities.contains(&absent),
            "{absent:?} must not be claimed by the gpui-test adapter"
        );
    }
}

#[test]
fn gpui_and_fake_run_the_same_scenarios_with_equal_evidence() {
    let (_dir, desktop, _job_id) = desktop_spec();
    let source = source_spec();

    // Fake adapter: 2 window に desktop / source を mount。
    let mut fake = FakeGuiTest::new();
    let f_desktop = fake.open_window();
    fake.mount_view(f_desktop, al_view(), desktop.clone())
        .expect("mount fake desktop");
    let f_source = fake.open_window();
    fake.mount_view(f_source, al_view(), source.clone())
        .expect("mount fake source");
    let f_jobs = desktop_jobs_scenario(&mut fake, f_desktop).expect("fake desktop scenario");
    let f_seek = source_seek_scenario(&mut fake, f_source, 0.05).expect("fake seek scenario");

    // GPUI adapter: 同じ spec を 2 window に mount(MultiWindow)。
    let mut gpui = GpuiGuiTest::new();
    let g_desktop = mount_gpui(&mut gpui, desktop);
    let g_source = mount_gpui(&mut gpui, source);
    let g_jobs = desktop_jobs_scenario(&mut gpui, g_desktop).expect("gpui desktop scenario");
    let g_seek = source_seek_scenario(&mut gpui, g_source, 0.05).expect("gpui seek scenario");

    // 同一 scenario が両 driver で同じ CommandIntent を発行する。
    let f_actions: Vec<_> = f_jobs.intents.iter().map(|m| &m.intent).collect();
    let g_actions: Vec<_> = g_jobs.intents.iter().map(|m| &m.intent).collect();
    assert_eq!(f_actions, g_actions, "desktop jobs intents must match");
    assert_eq!(
        command_actions(&f_jobs.intents),
        command_actions(&g_jobs.intents)
    );
    assert_eq!(f_jobs.jobs_selected, g_jobs.jobs_selected);

    let f_commits: Vec<&CommandIntent> = f_seek
        .intents
        .iter()
        .filter(|m| matches!(m.intent, CommandIntent::Commit { .. }))
        .map(|m| &m.intent)
        .collect();
    let g_commits: Vec<&CommandIntent> = g_seek
        .intents
        .iter()
        .filter(|m| matches!(m.intent, CommandIntent::Commit { .. }))
        .map(|m| &m.intent)
        .collect();
    assert_eq!(f_commits, g_commits, "seek commit intents must match");
    let f_plain = f_seek.snapshot.committed.expect("fake committed").plain;
    let g_plain = g_seek.snapshot.committed.expect("gpui committed").plain;
    assert!(
        (f_plain - g_plain).abs() < 1e-12,
        "committed plain values diverge: {f_plain} vs {g_plain}"
    );
}

#[test]
fn gpui_target_resolution_and_stale_pins_cover_al_view() {
    let (_dir, spec, _job_id) = desktop_spec();
    let mut gui = GpuiGuiTest::new();
    let window = mount_gpui(&mut gui, spec);
    let jobs = ComponentKey::from(JOBS_KEY);

    // WindowId + ViewInstanceId + ComponentKey の解決。
    let target: ScenarioTarget = gui
        .resolve_target(&TargetQuery::new(window, al_view()).component(jobs.clone()))
        .expect("resolve jobs target");
    assert_eq!(target.window, window);
    assert_eq!(target.view, al_view());
    assert_eq!(target.component, Some(jobs.clone()));

    // 未知 component / view / window は明示 error。
    let err = gui
        .resolve_target(
            &TargetQuery::new(window, al_view()).component(ComponentKey::from("no-such-key")),
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
            al_view(),
        ))
        .expect_err("unknown window must fail");
    assert!(matches!(err, ScenarioError::NoSuchWindow { .. }));

    // detach で generation が進み、pin 済み target は stale。
    gui.detach_view(window, "test-detach").expect("detach");
    let err = gui
        .observe(&target, &jobs)
        .expect_err("detached view must stale the target");
    assert!(
        matches!(err, ScenarioError::StaleGeneration { .. }),
        "expected StaleGeneration, got {err:?}"
    );
}

#[test]
fn gpui_unrunnable_items_report_unsupported_not_pass() {
    let (_dir, spec, _job_id) = desktop_spec();
    let mut gui = GpuiGuiTest::new();
    let window = mount_gpui(&mut gui, spec);
    let target = gui
        .resolve_target(
            &TargetQuery::new(window, al_view()).component(ComponentKey::from(JOBS_KEY)),
        )
        .expect("resolve");

    // pre-flight: GpuiSimulated が申告しない能力は Unsupported error。
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

    // pixel capture は gpui-test platform に readback が無く
    // Unsupported を返す(draw plan を pixel 証拠にしない)。
    let outcome = gui
        .capture_frame(&target)
        .expect("capture_frame returns an honest outcome");
    assert!(
        matches!(
            outcome,
            ruviform_ui_test::CaptureOutcome::Unsupported { .. }
        ),
        "gpui-test capture must be Unsupported: {outcome:?}"
    );

    // golden step も Unsupported verdict を返す。
    let dir = al_testkit::temp::TempDir::new("al-scenario-golden-gpui");
    let store = BaselineStore::new(dir.to_path_buf());
    let key = GoldenKey {
        story_id: "al.desktop.jobs".to_string(),
        theme: "test".to_string(),
        state: "idle".to_string(),
        driver_class: "gpui-test".to_string(),
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

    // ScenarioReport: driver=GpuiSimulated・verdict=Unsupported の
    // まま記録し、semantic pass を実機・pixel pass へ昇格しない。
    let mut report = al_scenario_report(
        &gui,
        "run-gpui-unsupported",
        SCENARIO_DESKTOP_JOBS,
        CASE_JOB_SELECT_RESUME,
    );
    report.required = BTreeSet::from([
        ScenarioCapability::RealOsPointerOrKeyboard,
        ScenarioCapability::VisualCapture,
    ]);
    report.verdict = ScenarioVerdict::Unsupported;
    report.detail = Some(
        "real OS input and pixel capture are not provided by the gpui-test adapter".to_string(),
    );
    assert_eq!(report.driver, ScenarioDriver::GpuiSimulated);
    assert_eq!(report.verdict, ScenarioVerdict::Unsupported);
    assert_eq!(report.verdict.lane_status(), "unsupported");
}
