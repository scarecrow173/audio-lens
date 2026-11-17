//! `al-gui-real-scenario`: AudioLens 製品 GUI の実 OS E2E lane
//! (Issue #219 — `scarecrow173/ruviform#631` の consumer 側接続)。
//!
//! 製品 `ViewSpec`(`desktop_screen`/`source_screen` — `al-desktop`
//! と同じ `ViewHost`/`ViewMount` 経路)を `RealOsGuiTest::run` の
//! 実 OS window へ in-process mount し、driver-neutral scenario
//! (`tests/common/scenario.rs` の共有 helper)を
//! `run_scenario`/`ScenarioSpec` 経由で `ScenarioDriver::RealOs`
//! として実行する。real OS 入力は `os_*` 系、pixel 証拠は
//! `capture_frame`(`VisualGuiTest`)のみを使い、semantic dispatch
//! を実入力証拠へ昇格しない。
//!
//! - 通常実行: 実 window を開き、全 case の `ScenarioReport` を
//!   JSONL(`run` header + `case` 行)で `--out <path>` または
//!   stdout へ出力する。`Fail` verdict がある場合のみ exit 1。
//!   `NotRun`/`Unsupported`/`Blocked` は正直な記録であり lane
//!   失敗ではない(POLICIES §2.1/§4)。
//! - `--blocked-probe`: window を開かず、catalog 全 case を
//!   `NotRun` + `observed.environment_blocked` で決定的に出力し、
//!   report 形状の自己検査(全件 NotRun・`environment_kind` が
//!   `real-window` を主張しない)に違反があれば exit 1。
//! - 非対話・headless host では `RealOsGuiTest::run` が
//!   `UiError::Host` で失敗し、全 case が同じ fail-closed 形状で
//!   記録される。実走 Pass 証拠は interactive Windows desktop
//!   session 前提の opt-in lane(`mise run test-ui-gpui-real`)で
//!   しか得られない — 本 bin は Issue #219 を close しない。
//!
//! test-only bin — `gpui-real` feature 指定時のみ build され、
//! 製品配布には含めない(POLICIES §2.1)。

// `gpui-test`(simulated adapter)と `gpui-real`(real OS adapter)は
// 同一 session で同居不可(ruviform-ui-gpui の注記: test platform が
// OS surface を stub する)。feature 組合せ自体を compile error で
// 排他する。
#[cfg(all(feature = "gpui-test", feature = "gpui-real"))]
compile_error!(
    "gpui-test and gpui-real are mutually exclusive: the gpui-test \
     platform stubs the OS surface, so a real-window session cannot \
     produce honest pixel/input evidence while it is enabled"
);

#[path = "../../tests/common/mod.rs"]
mod common;

use std::collections::BTreeSet;
use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

use al_gui::command;
use al_gui::view::desktop::DESKTOP_KEY;
use al_gui::view::screen::{SEEK_KEY, SEEK_TARGET};
use common::scenario::{
    CASE_JOB_SELECT_RESUME, CASE_SEEK_COMMIT, SCENARIO_DESKTOP_JOBS, SCENARIO_SOURCE_SEEK, al_view,
    command_actions, desktop_jobs_scenario, desktop_spec, source_seek_scenario, source_spec,
};
use ruviform_ui::behavior::CommandIntent;
use ruviform_ui::view::{AttachEpochAllocator, ComponentKey, ViewHost, ViewSpec};
use ruviform_ui::{WindowId, WindowSpec};
use ruviform_ui_gpui::os_input::{OsInputOutcome, OsKey, PreflightScope};
use ruviform_ui_gpui::{RealOsGuiTest, RealWindowMount, RealWindowOptions, ViewMount};
use ruviform_ui_test::{
    CaptureOutcome, CaseId, RunId, ScenarioCapability, ScenarioDriver, ScenarioId, ScenarioOutcome,
    ScenarioReport, ScenarioSpec, ScenarioVerdict, SemanticGuiTest, TargetQuery, VisualGuiTest,
    run_scenario, settle_until,
};
use serde_json::{Map, Value, json};

/// JSONL `run_id` — AL consumer namespace。
const RUN_ID: &str = "al219-gui-real-e2e";
/// live lane 名(mise task `test-ui-gpui-real` と対応)。
const LANE_REAL: &str = "test-ui-gpui-real";
/// headless blocked-probe lane 名(mise task `test-ui-gpui-real-headless`)。
const LANE_HEADLESS: &str = "test-ui-gpui-real-headless";

/// lane catalog の 1 行 — scenario/case id と required capability。
/// `blocked_reports` と live path が同じ表を読むため、blocked 記録が
/// live catalog から drift しない。
struct LaneCase {
    scenario: &'static str,
    case: &'static str,
    required: &'static [ScenarioCapability],
}

const CASE_DESKTOP_JOBS: LaneCase = LaneCase {
    scenario: SCENARIO_DESKTOP_JOBS,
    case: CASE_JOB_SELECT_RESUME,
    required: &[ScenarioCapability::SemanticAction],
};
const CASE_SOURCE_SEEK: LaneCase = LaneCase {
    scenario: SCENARIO_SOURCE_SEEK,
    case: CASE_SEEK_COMMIT,
    required: &[ScenarioCapability::SemanticAction],
};
const CASE_PREFLIGHT: LaneCase = LaneCase {
    scenario: "al.real-os.input",
    case: "os-input-preflight",
    required: &[ScenarioCapability::RealOsPointerOrKeyboard],
};
const CASE_POINTER_SEEK: LaneCase = LaneCase {
    scenario: "al.real-os.pointer",
    case: "seek-pointer-click",
    required: &[ScenarioCapability::RealOsPointerOrKeyboard],
};
const CASE_KEYBOARD_SEEK: LaneCase = LaneCase {
    scenario: "al.real-os.keyboard",
    case: "seek-key-end",
    required: &[ScenarioCapability::RealOsPointerOrKeyboard],
};
const CASE_CAPTURE_DESKTOP: LaneCase = LaneCase {
    scenario: "al.real-os.visual",
    case: "capture-frame-desktop",
    required: &[ScenarioCapability::VisualCapture],
};
const CASE_CAPTURE_SOURCE: LaneCase = LaneCase {
    scenario: "al.real-os.visual",
    case: "capture-frame-source",
    required: &[ScenarioCapability::VisualCapture],
};

/// 実行順の catalog。blocked-probe の件数・id 検査もこの表が正本。
const CASES: &[&LaneCase] = &[
    &CASE_DESKTOP_JOBS,
    &CASE_SOURCE_SEEK,
    &CASE_PREFLIGHT,
    &CASE_POINTER_SEEK,
    &CASE_KEYBOARD_SEEK,
    &CASE_CAPTURE_DESKTOP,
    &CASE_CAPTURE_SOURCE,
];

fn main() -> ExitCode {
    let mut out: Option<PathBuf> = None;
    let mut blocked_probe = false;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => {
                let Some(path) = args.next() else {
                    eprintln!("al-gui-real-scenario: --out requires a path");
                    return ExitCode::from(2);
                };
                out = Some(PathBuf::from(path));
            }
            "--blocked-probe" => blocked_probe = true,
            other => {
                eprintln!("al-gui-real-scenario: unknown argument `{other}`");
                return ExitCode::from(2);
            }
        }
    }

    let repo_sha = repo_sha();
    let (reports, lane) = if blocked_probe {
        (
            blocked_reports(
                &repo_sha,
                "blocked-probe: deterministic self-check — no real session opened",
                LANE_HEADLESS,
                json!({ "real_window_session": "not_started" }),
            ),
            LANE_HEADLESS,
        )
    } else {
        (live_reports(&repo_sha), LANE_REAL)
    };
    let jsonl = render_jsonl(RUN_ID, lane, repo_sha.as_deref(), &reports);

    let write_result = if let Some(path) = &out {
        std::fs::write(path, &jsonl).map_err(|e| format!("{}: {e}", path.display()))
    } else {
        print!("{jsonl}");
        std::io::stdout().flush().map_err(|e| e.to_string())
    };
    if let Err(error) = write_result {
        eprintln!("al-gui-real-scenario: cannot write output: {error}");
        return ExitCode::FAILURE;
    }

    if blocked_probe {
        let violations = blocked_probe_violations(&reports);
        if violations.is_empty() {
            return ExitCode::SUCCESS;
        }
        for violation in &violations {
            eprintln!("al-gui-real-scenario: blocked-probe violation: {violation}");
        }
        return ExitCode::FAILURE;
    }

    // `NotRun`/`Unsupported`/`Blocked` は正直な記録であり lane 失敗
    // ではない。`Fail` verdict(契約違反・実測 oracle 不成立)のみ exit 1。
    if reports
        .iter()
        .any(|report| report.verdict == ScenarioVerdict::Fail)
    {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

// ---------------------------------------------------------------------
// live session
// ---------------------------------------------------------------------

/// desktop / source 両画面の `ViewSpec` を 1 session・2 window に
/// mount して全 case を実行する。session 開始失敗(headless host 等)
/// は全 case `NotRun` + `environment_blocked` の fail-closed 記録へ
/// 写す — 実行していないものを Pass にしない。
fn live_reports(repo_sha: &Option<String>) -> Vec<ScenarioReport> {
    // `TempDir` は desktop spec の dataset session 基底 dir を保持
    // するため、session 終了まで生存させる。
    let (_desktop_dir, desktop_view, job_id) = desktop_spec();
    let source_view = source_spec();
    let environment = environment_json("real-window", repo_sha, LANE_REAL);

    let mounts = vec![
        RealWindowMount {
            window: WindowSpec::default(),
            view: view_mount(desktop_view),
        },
        RealWindowMount {
            window: WindowSpec::default(),
            view: view_mount(source_view),
        },
    ];

    let outcome = RealOsGuiTest::run(mounts, RealWindowOptions::default(), |gui| {
        run_live_cases(gui, &environment, &job_id, repo_sha)
    });

    match outcome {
        Ok(mut reports) => {
            for report in &mut reports {
                if report.cleanup.is_none() {
                    report.cleanup = Some(json!({
                        "real_window_session": "closed",
                        "windows": 2,
                    }));
                }
            }
            reports
        }
        Err(error) => blocked_reports(
            repo_sha,
            &format!("real-window session could not start: {error}"),
            LANE_REAL,
            json!({ "real_window_session": "not_started" }),
        ),
    }
}

/// `al-desktop` の `mount_host` / `scenario_gpui` の `mount_gpui` と
/// 同じ形: `ViewHost` を production view instance id(`AL_VIEW = 1`)
/// で組み立てて `ViewMount` にする。
fn view_mount(spec: ViewSpec) -> ViewMount {
    let mut host = ViewHost::new(al_view(), AttachEpochAllocator::new().mint());
    host.mount(spec).expect("al view spec mounts");
    host.drain_events();
    ViewMount { host, theme: None }
}

/// session 内で catalog 順に全 case を実行する。window は mount 順
/// (= `WindowId` 昇順、driver の `BTreeMap`)に desktop → source。
fn run_live_cases(
    gui: &mut RealOsGuiTest,
    environment: &Value,
    job_id: &al_core::JobId,
    repo_sha: &Option<String>,
) -> Vec<ScenarioReport> {
    let windows = gui.windows();
    if windows.len() != 2 {
        return blocked_reports(
            repo_sha,
            &format!(
                "real-window session mounted {} windows, expected 2",
                windows.len()
            ),
            LANE_REAL,
            json!({
                "real_window_session": "closed",
                "windows": windows.len(),
            }),
        );
    }
    let desktop_window = windows[0];
    let source_window = windows[1];

    let mut reports = Vec::with_capacity(CASES.len());
    reports.push(desktop_jobs_case(
        gui,
        desktop_window,
        &spec_for(&CASE_DESKTOP_JOBS, environment),
        job_id,
    ));
    reports.push(source_seek_case(
        gui,
        source_window,
        &spec_for(&CASE_SOURCE_SEEK, environment),
    ));
    reports.push(preflight_case(gui, &spec_for(&CASE_PREFLIGHT, environment)));

    // OS 入力 case の前提: source window を OS foreground にする。
    // 要求失敗時も続行し、`os_*` op が `NotRun`/`EnvironmentBlocked`
    // を正直に返すのに任せる(focus は case の被検査対象ではない)。
    let _ = gui.request_focus(source_window, true);
    let _ = settle_until(gui, |g| g.window_focused(source_window), 20);

    reports.push(pointer_seek_case(
        gui,
        source_window,
        &spec_for(&CASE_POINTER_SEEK, environment),
    ));
    reports.push(keyboard_seek_case(
        gui,
        source_window,
        &spec_for(&CASE_KEYBOARD_SEEK, environment),
    ));
    reports.push(capture_case(
        gui,
        desktop_window,
        DESKTOP_KEY,
        &spec_for(&CASE_CAPTURE_DESKTOP, environment),
    ));
    reports.push(capture_case(
        gui,
        source_window,
        SEEK_KEY,
        &spec_for(&CASE_CAPTURE_SOURCE, environment),
    ));
    reports
}

fn spec_for(case: &LaneCase, environment: &Value) -> ScenarioSpec {
    ScenarioSpec::new(RunId::new(RUN_ID), ScenarioId::new(case.scenario))
        .case(CaseId::new(case.case))
        .required(case.required.iter().copied().collect::<BTreeSet<_>>())
        .environment(environment.clone())
        .timeout_ms(60_000)
}

// ---------------------------------------------------------------------
// cases
// ---------------------------------------------------------------------

/// `al.desktop.jobs`/`job-select-resume`: 共有 scenario を semantic
/// port で実行し、発行 intent 列が製品契約(select → tab → resume)
/// と一致するかを oracle とする。
fn desktop_jobs_case(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    spec: &ScenarioSpec,
    job_id: &al_core::JobId,
) -> ScenarioReport {
    run_scenario(gui, spec, |gui| {
        let outcome = desktop_jobs_scenario(&mut *gui, window)?;
        let actions = command_actions(&outcome.intents);
        let expected = vec![
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
        ];
        let ok = outcome.jobs_selected == Some(0) && actions == expected;
        let mut result = ScenarioOutcome::new(if ok {
            ScenarioVerdict::Pass
        } else {
            ScenarioVerdict::Fail
        });
        result.expected = Some(json!({ "actions": expected }));
        result.observed = Some(json!({
            "actions": actions,
            "jobs_selected": outcome.jobs_selected,
        }));
        result.targets = vec![outcome.target];
        if !ok {
            result.detail = Some(
                "desktop jobs intents diverged from the select→tab→resume contract".to_string(),
            );
        }
        Ok(result)
    })
}

/// `al.source.seek`/`seek-commit`: 共有 `parity_gesture_scenario`
/// (Begin→Preview→Commit)を実行し、Seek への Commit intent と
/// committed plain 値を oracle とする。
fn source_seek_case(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    spec: &ScenarioSpec,
) -> ScenarioReport {
    run_scenario(gui, spec, |gui| {
        let outcome = source_seek_scenario(&mut *gui, window, 0.05)?;
        let committed = outcome.snapshot.committed;
        let commit_found = outcome.intents.iter().any(
            |m| matches!(&m.intent, CommandIntent::Commit { target, .. } if target == SEEK_TARGET),
        );
        let plain_ok = committed.is_some_and(|v| (v.plain - 0.05).abs() < 1e-6);
        let ok = commit_found && plain_ok;
        let mut result = ScenarioOutcome::new(if ok {
            ScenarioVerdict::Pass
        } else {
            ScenarioVerdict::Fail
        });
        result.expected = Some(json!({
            "commit_target": SEEK_TARGET,
            "plain": 0.05,
        }));
        result.observed = Some(json!({
            "committed": committed.map(|v| json!({"plain": v.plain, "normalized": v.normalized})),
            "commit_found": commit_found,
        }));
        result.targets = vec![outcome.target];
        if !ok {
            result.detail =
                Some("seek gesture did not commit the expected transport.seek value".to_string());
        }
        Ok(result)
    })
}

/// `al.real-os.input`/`os-input-preflight`: pointer+keyboard の
/// session preflight を実測し、`effective == false` は
/// `environment_blocked` の `NotRun`(GUI 不具合ではない)。
fn preflight_case(gui: &mut RealOsGuiTest, spec: &ScenarioSpec) -> ScenarioReport {
    run_scenario(gui, spec, |gui| {
        let report = gui.os_input_preflight(PreflightScope::PointerAndKeyboard);
        let checks: Vec<Value> = report
            .checks
            .iter()
            .map(|check| {
                json!({
                    "family": check.family,
                    "effective": check.effective,
                    "detail": check.detail,
                })
            })
            .collect();
        let mut result = ScenarioOutcome::new(if report.effective {
            ScenarioVerdict::Pass
        } else {
            ScenarioVerdict::NotRun
        });
        result.observed = Some(json!({
            "preflight": { "effective": report.effective, "checks": checks },
            "environment_blocked": !report.effective,
        }));
        if !report.effective {
            result.detail = Some(
                "os input preflight ineffective — environment blocked, not a GUI failure"
                    .to_string(),
            );
        }
        Ok(result)
    })
}

/// `al.real-os.pointer`/`seek-pointer-click`: 実 OS pointer で Seek
/// をクリックし、Commit intent + committed 値変化を oracle とする。
/// `OsInputOutcome` の `EnvironmentBlocked`/`NotRun`/`Unsupported`
/// は verdict にそのまま写し、`Injected` のみ oracle 判定へ進む。
fn pointer_seek_case(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    spec: &ScenarioSpec,
) -> ScenarioReport {
    run_scenario(gui, spec, |gui| {
        let seek = ComponentKey::from(SEEK_KEY);
        let target =
            gui.resolve_target(&TargetQuery::new(window, al_view()).component(seek.clone()))?;
        let before = gui.observe(&target, &seek)?.committed.map(|v| v.plain);

        let ops = [
            (
                "pointer-move",
                gui.os_pointer_move_to(&target, &seek, (0.75, 0.5))?,
            ),
            ("mouse-down", gui.os_mouse_down(&target)?),
            ("mouse-up", gui.os_mouse_up(&target)?),
        ];
        let ops_json = ops_json(&ops);
        if let Some(outcome) = first_non_injected(&ops) {
            return Ok(gate_outcome(&ops_json, outcome));
        }

        gui.settle()?;
        let intents = gui.drain_intents();
        let after = gui.observe(&target, &seek)?;
        let commit_found = intents.iter().any(
            |m| matches!(&m.intent, CommandIntent::Commit { target, .. } if target == SEEK_TARGET),
        );
        let after_plain = after.committed.map(|v| v.plain);
        let moved = after_plain.is_some_and(|p| before.is_none_or(|b| (p - b).abs() > 1e-9));
        let ok = commit_found && moved;
        let mut result = ScenarioOutcome::new(if ok {
            ScenarioVerdict::Pass
        } else {
            ScenarioVerdict::Fail
        });
        result.expected = Some(json!({
            "commit_target": SEEK_TARGET,
            "pointer": "click at 75% of seek bounds",
        }));
        result.observed = Some(json!({
            "ops": ops_json,
            "before_plain": before,
            "after": after.committed.map(|v| json!({"plain": v.plain, "normalized": v.normalized})),
            "commit_found": commit_found,
        }));
        result.targets = vec![target];
        if !ok {
            result.detail = Some(
                "real pointer click did not produce the expected transport.seek commit".to_string(),
            );
        }
        Ok(result)
    })
}

/// `al.real-os.keyboard`/`seek-key-end`: 実 OS pointer click で Seek
/// に focus を置いた後、実 OS keyboard `End` で最大値へ Set し、
/// committed normalized ≈ 1.0 を oracle とする。focus 確立
/// (pointer click)が `Injected` でない場合は前提不成立として
/// `NotRun`/`Unsupported` — keyboard を押して効果不明のまま
/// Fail にしない。
fn keyboard_seek_case(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    spec: &ScenarioSpec,
) -> ScenarioReport {
    run_scenario(gui, spec, |gui| {
        let seek = ComponentKey::from(SEEK_KEY);
        let target =
            gui.resolve_target(&TargetQuery::new(window, al_view()).component(seek.clone()))?;

        // component focus は実 click でのみ得られる(GPUI の
        // pointer→focus 経路)。click 自体も証拠として記録する。
        let focus_ops = [
            (
                "pointer-move",
                gui.os_pointer_move_to(&target, &seek, (0.5, 0.5))?,
            ),
            ("mouse-down", gui.os_mouse_down(&target)?),
            ("mouse-up", gui.os_mouse_up(&target)?),
        ];
        let mut ops_json = ops_json(&focus_ops);
        if let Some(outcome) = first_non_injected(&focus_ops) {
            return Ok(gate_outcome(&ops_json, outcome));
        }
        gui.settle()?;
        let _ = gui.drain_intents();

        let key_outcome = gui.os_press_key(&target, OsKey::End)?;
        ops_json
            .as_object_mut()
            .expect("ops object")
            .insert("key-end".to_string(), op_json(&key_outcome));
        if !matches!(key_outcome, OsInputOutcome::Injected { .. }) {
            return Ok(gate_outcome(&ops_json, &key_outcome));
        }

        gui.settle()?;
        let intents = gui.drain_intents();
        let after = gui.observe(&target, &seek)?;
        let normalized = after.committed.map(|v| v.normalized);
        let ok = normalized.is_some_and(|n| n > 0.99);
        let mut result = ScenarioOutcome::new(if ok {
            ScenarioVerdict::Pass
        } else {
            ScenarioVerdict::Fail
        });
        result.expected = Some(json!({
            "key": "End",
            "committed_normalized_min": 0.99,
        }));
        result.observed = Some(json!({
            "ops": ops_json,
            "committed": after.committed.map(|v| json!({"plain": v.plain, "normalized": v.normalized})),
            "intents": intents.len(),
        }));
        result.targets = vec![target];
        if !ok {
            result.detail =
                Some("real End key did not move the focused seek to its maximum".to_string());
        }
        Ok(result)
    })
}

/// `al.real-os.visual`/`capture-frame-*`: `VisualGuiTest::capture_frame`
/// の実 OS pixel capture。`Captured` は `last_capture_meta` の
/// backend/寸法/scale を証拠に記録し、`Unsupported`/`NotRun` は
/// そのまま verdict へ写す(backend 不在を Fail にしない)。
fn capture_case(
    gui: &mut RealOsGuiTest,
    window: WindowId,
    component: &str,
    spec: &ScenarioSpec,
) -> ScenarioReport {
    run_scenario(gui, spec, |gui| {
        let key = ComponentKey::from(component);
        let target = gui.resolve_target(&TargetQuery::new(window, al_view()).component(key))?;
        match gui.capture_frame(&target)? {
            CaptureOutcome::Captured(image) => {
                let meta = gui.last_capture_meta();
                let pixels_ok = image.pixel_count() > 0;
                let mut result = ScenarioOutcome::new(if pixels_ok {
                    ScenarioVerdict::Pass
                } else {
                    ScenarioVerdict::Fail
                });
                result.observed = Some(json!({
                    "capture": {
                        "backend": meta.map(|m| m.backend),
                        "width": meta.map(|m| m.width),
                        "height": meta.map(|m| m.height),
                        "scale_factor": meta.map(|m| m.scale_factor),
                        "pixel_count": image.pixel_count(),
                    },
                }));
                result.targets = vec![target];
                if !pixels_ok {
                    result.detail = Some("captured frame carried no pixels".to_string());
                }
                Ok(result)
            }
            CaptureOutcome::Unsupported { reason } => {
                let mut result = ScenarioOutcome::new(ScenarioVerdict::Unsupported);
                result.observed = Some(json!({"capture": "unsupported", "reason": reason}));
                result.detail = Some(format!("pixel capture unsupported: {reason}"));
                result.targets = vec![target];
                Ok(result)
            }
            CaptureOutcome::NotRun { reason } => {
                let mut result = ScenarioOutcome::new(ScenarioVerdict::NotRun);
                result.observed = Some(json!({"capture": "not_run", "reason": reason}));
                result.detail = Some(format!("pixel capture not run: {reason}"));
                result.targets = vec![target];
                Ok(result)
            }
            _ => {
                let mut result = ScenarioOutcome::new(ScenarioVerdict::NotRun);
                result.detail = Some("unrecognized CaptureOutcome variant".to_string());
                result.targets = vec![target];
                Ok(result)
            }
        }
    })
}

// ---------------------------------------------------------------------
// OS input outcome helpers
// ---------------------------------------------------------------------

/// `OsInputOutcome` を observed JSON 用の `(tag, detail)` へ写す。
/// enum は `non_exhaustive` — 未知 variant は `"unknown"` として
/// 正直に記録し、`Injected` 扱いしない。
fn op_json(outcome: &OsInputOutcome) -> Value {
    match outcome {
        OsInputOutcome::Injected { action, detail, .. } => json!({
            "tag": "injected",
            "action": action,
            "detail": detail,
        }),
        OsInputOutcome::Unsupported { reason } => json!({
            "tag": "unsupported",
            "detail": reason,
        }),
        OsInputOutcome::EnvironmentBlocked { detail } => json!({
            "tag": "environment_blocked",
            "detail": detail,
        }),
        OsInputOutcome::NotRun { reason } => json!({
            "tag": "not_run",
            "detail": reason,
        }),
        _ => json!({ "tag": "unknown" }),
    }
}

/// `(name, outcome)` 列を `{name: op_json}` の object へ。
fn ops_json(ops: &[(&str, OsInputOutcome)]) -> Value {
    ops.iter()
        .map(|(name, outcome)| ((*name).to_string(), op_json(outcome)))
        .collect::<Map<String, Value>>()
        .into()
}

/// 最初の非 `Injected` outcome。全て `Injected` なら `None`。
fn first_non_injected<'a>(ops: &'a [(&'static str, OsInputOutcome)]) -> Option<&'a OsInputOutcome> {
    ops.iter()
        .map(|(_, outcome)| outcome)
        .find(|outcome| !matches!(outcome, OsInputOutcome::Injected { .. }))
}

/// 非 `Injected` outcome を verdict へ写した `ScenarioOutcome`。
/// `EnvironmentBlocked` は `observed.environment_blocked` を立てた
/// `NotRun`(環境起因を GUI fail にしない)、`Unsupported` は
/// `Unsupported`、`NotRun`/未知は `NotRun`。
fn gate_outcome(ops: &Value, outcome: &OsInputOutcome) -> ScenarioOutcome {
    let mut result = match outcome {
        OsInputOutcome::Unsupported { reason } => {
            let mut result = ScenarioOutcome::new(ScenarioVerdict::Unsupported);
            result.detail = Some(format!("os input unsupported: {reason}"));
            result
        }
        OsInputOutcome::EnvironmentBlocked { detail } => {
            let mut result = ScenarioOutcome::new(ScenarioVerdict::NotRun);
            result.detail = Some(format!("os input environment blocked: {detail}"));
            result
        }
        OsInputOutcome::NotRun { reason } => {
            let mut result = ScenarioOutcome::new(ScenarioVerdict::NotRun);
            result.detail = Some(format!("os input precondition unmet: {reason}"));
            result
        }
        _ => {
            let mut result = ScenarioOutcome::new(ScenarioVerdict::NotRun);
            result.detail = Some("unrecognized OsInputOutcome variant".to_string());
            result
        }
    };
    let blocked = matches!(outcome, OsInputOutcome::EnvironmentBlocked { .. });
    result.observed = Some(json!({
        "ops": ops,
        "environment_blocked": blocked,
    }));
    result
}

// ---------------------------------------------------------------------
// blocked / probe path
// ---------------------------------------------------------------------

/// session が開始できなかった場合の fail-closed reports — catalog
/// 全 case を `NotRun` + `observed.environment_blocked` で記録する。
/// `environment_kind` は "no real environment produced this" の
/// 正直な bucket(`deterministic-backend`)で、`real-window` は
/// 絶対に主張しない。
fn blocked_reports(
    repo_sha: &Option<String>,
    detail: &str,
    lane: &str,
    cleanup: Value,
) -> Vec<ScenarioReport> {
    let run_id = RunId::new(RUN_ID);
    CASES
        .iter()
        .map(|case| {
            let mut report = ScenarioReport::new(
                run_id.clone(),
                ScenarioId::new(case.scenario),
                Some(CaseId::new(case.case)),
                ScenarioDriver::RealOs,
            );
            report.required = case.required.iter().copied().collect();
            report.verdict = ScenarioVerdict::NotRun;
            report.detail = Some(detail.to_string());
            report.observed = Some(json!({
                "observed": "environment_blocked",
                "detail": detail,
            }));
            report.environment = Some(environment_json("deterministic-backend", repo_sha, lane));
            report.cleanup = Some(cleanup.clone());
            report
        })
        .collect()
}

/// `--blocked-probe` 出力の形状自己検査。catalog との id/required
/// 一致、全件 `NotRun` + `RealOs` driver + `observed.environment_
/// blocked`、`environment_kind` が `real-window` を主張しないこと
/// を検査し、違反を列挙する(空 = 合格)。
fn blocked_probe_violations(reports: &[ScenarioReport]) -> Vec<String> {
    let mut violations = Vec::new();
    if reports.len() != CASES.len() {
        violations.push(format!(
            "report count {} does not match catalog size {}",
            reports.len(),
            CASES.len()
        ));
    }
    for (case, report) in CASES.iter().zip(reports.iter()) {
        let label = format!("{}/{}", case.scenario, case.case);
        if report.scenario.as_str() != case.scenario {
            violations.push(format!("{label}: scenario id drift"));
        }
        if report.case.as_ref().map(CaseId::as_str) != Some(case.case) {
            violations.push(format!("{label}: case id drift"));
        }
        if report.driver != ScenarioDriver::RealOs {
            violations.push(format!("{label}: driver is not RealOs"));
        }
        if report.verdict != ScenarioVerdict::NotRun {
            violations.push(format!(
                "{label}: verdict {:?} is not NotRun",
                report.verdict
            ));
        }
        let observed_ok = report
            .observed
            .as_ref()
            .and_then(|observed| observed["observed"].as_str())
            == Some("environment_blocked");
        if !observed_ok {
            violations.push(format!("{label}: observed.environment_blocked missing"));
        }
        let kind = report
            .environment
            .as_ref()
            .and_then(|env| env["environment_kind"].as_str());
        match kind {
            Some("real-window") => {
                violations.push(format!("{label}: claims environment_kind=real-window"));
            }
            Some(_) => {}
            None => violations.push(format!("{label}: environment_kind missing")),
        }
        let required: BTreeSet<ScenarioCapability> = case.required.iter().copied().collect();
        if report.required != required {
            violations.push(format!("{label}: required capabilities drifted"));
        }
    }
    violations
}

// ---------------------------------------------------------------------
// JSONL rendering (ruviform-plugin-dock-test `jsonl.rs` と同形)
// ---------------------------------------------------------------------

/// `run` header 1 行 + report 毎の `case` 行。`targets` は adapter
/// 内部 id のため JSONL 証拠面へ出さない(plugin-dock 先例どおり)。
fn render_jsonl(
    run_id: &str,
    lane: &str,
    repo_sha: Option<&str>,
    reports: &[ScenarioReport],
) -> String {
    let header = json!({
        "type": "run",
        "run_id": run_id,
        "lane": lane,
        "repo": { "sha": repo_sha },
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    });
    let mut out = format!(
        "{}\n",
        serde_json::to_string(&header).unwrap_or_else(|_| "{}".to_string())
    );
    for report in reports {
        out.push_str(&case_line(report));
        out.push('\n');
    }
    out
}

/// `ScenarioReport` の公開 field を 1 `case` 行へ投影する。
fn case_line(report: &ScenarioReport) -> String {
    let object = Map::from_iter([
        ("type".to_string(), json!("case")),
        ("run_id".to_string(), json!(report.run_id)),
        ("scenario".to_string(), json!(report.scenario)),
        (
            "case".to_string(),
            report.case.as_ref().map_or(Value::Null, |case| json!(case)),
        ),
        ("driver".to_string(), json!(report.driver)),
        (
            "capabilities".to_string(),
            json!(report.capabilities.iter().collect::<Vec<_>>()),
        ),
        (
            "required".to_string(),
            json!(report.required.iter().collect::<Vec<_>>()),
        ),
        ("verdict".to_string(), json!(report.verdict)),
        (
            "expected".to_string(),
            report.expected.clone().unwrap_or(Value::Null),
        ),
        (
            "observed".to_string(),
            report.observed.clone().unwrap_or(Value::Null),
        ),
        ("evidence".to_string(), json!(report.evidence)),
        (
            "environment".to_string(),
            report.environment.clone().unwrap_or(Value::Null),
        ),
        (
            "detail".to_string(),
            report.detail.clone().map_or(Value::Null, Value::String),
        ),
        (
            "timeout".to_string(),
            report.timeout.map_or(Value::Null, |timeout| {
                json!({
                    "budget_ms": timeout.budget_ms,
                    "elapsed_ms": timeout.elapsed_ms,
                    "timed_out": timeout.timed_out,
                })
            }),
        ),
        (
            "cleanup".to_string(),
            report.cleanup.clone().unwrap_or(Value::Null),
        ),
    ]);
    serde_json::to_string(&Value::Object(object)).unwrap_or_else(|_| "{}".to_string())
}

/// case/environment 共通の JSON。`environment_kind` は
/// `real-window`(live session が実際に走った場合)か
/// `deterministic-backend`(blocked — 実環境がこの証拠を生まなかった
/// 正直な bucket)のいずれか。`repo_sha` は `GITHUB_SHA` →
/// `git rev-parse HEAD` → `null` の順で、推測しない。
fn environment_json(kind: &str, repo_sha: &Option<String>, lane: &str) -> Value {
    json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "environment_kind": kind,
        "repo_sha": repo_sha,
        "lane": lane,
    })
}

/// repo commit SHA: `GITHUB_SHA`(CI) → `git rev-parse HEAD` → `None`
/// (honest `null`、plugin-dock `repo_sha` と同じ順序)。
fn repo_sha() -> Option<String> {
    if let Ok(sha) = env::var("GITHUB_SHA") {
        return Some(sha);
    }
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|sha| sha.trim().to_string())
        .filter(|sha| !sha.is_empty())
}
