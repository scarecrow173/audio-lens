//! `DesktopSession` の FakeDriver lifecycle fixture (AL-G1)。
//! driver-neutral 契約のみを検証し、実 OS window は開かない。

mod common;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use al_gui::command;
use al_gui::desktop::{DesktopPolicy, DesktopSession};
use al_gui::service::AlGuiService;
use al_testkit::temp::TempDir;
use ruviform_ui::ipc::{ErrorBody, Intent, codes};
use ruviform_ui::{
    CallOutcome, DataRegistry, DriverEvent, JobSpec, OpInfo, Service, ServiceContext, UiRuntime,
    WindowDriver, WindowId, WindowLifecycle, WindowSpec,
};
use ruviform_ui_test::FakeDriver;
use serde_json::json;

fn session() -> DesktopSession {
    let mut rt = UiRuntime::new(DataRegistry::new());
    rt.register_service(AlGuiService::new())
        .expect("al-gui service");
    DesktopSession::new(rt)
}

fn spec() -> WindowSpec {
    WindowSpec {
        title: "AudioLens".into(),
        ..WindowSpec::default()
    }
}

fn pump(session: &mut DesktopSession, driver: &mut FakeDriver) {
    driver
        .pump(&mut |ev, drv| session.handle(ev, drv))
        .into_iter()
        .for_each(|_| {});
}

// ---- deterministic job fixture ----
// `test.block` op は `gate` が開くまで `check_cancel` で自ら
// spin する JobSpec を返す。worker が到達した分岐
// ("finished"/"cancelled")を `outcome` に書く — window destroy と
// job 終端の順序に依存しない決定的観測路。
struct BlockingService {
    gate: Arc<AtomicBool>,
    outcome: Arc<Mutex<Option<&'static str>>>,
}

const BLOCK_OPS: &[OpInfo] = &[OpInfo {
    name: "test.block",
    summary: "deterministic blocking job for lifecycle tests",
}];

impl Service for BlockingService {
    fn ops(&self) -> &'static [OpInfo] {
        BLOCK_OPS
    }

    fn call(
        &mut self,
        op: &str,
        _params: &serde_json::Value,
        _ctx: &mut ServiceContext<'_>,
    ) -> Result<CallOutcome, ErrorBody> {
        assert_eq!(op, "test.block");
        let gate = self.gate.clone();
        let outcome = self.outcome.clone();
        Ok(CallOutcome::Job(JobSpec::new(
            "blocking-probe",
            move |ctx| {
                loop {
                    ctx.check_cancel().inspect_err(|_| {
                        *outcome.lock().unwrap() = Some("cancelled");
                    })?;
                    if gate.load(Ordering::SeqCst) {
                        *outcome.lock().unwrap() = Some("finished");
                        return Ok(json!({"done": true}));
                    }
                    std::thread::yield_now();
                }
            },
        )))
    }
}

fn blocking_session(
    policy: DesktopPolicy,
) -> (
    DesktopSession,
    Arc<AtomicBool>,
    Arc<Mutex<Option<&'static str>>>,
) {
    let gate = Arc::new(AtomicBool::new(false));
    let outcome = Arc::new(Mutex::new(None));
    let mut rt = UiRuntime::new(DataRegistry::new());
    rt.register_service(BlockingService {
        gate: gate.clone(),
        outcome: outcome.clone(),
    })
    .expect("blocking service");
    (DesktopSession::with_policy(rt, policy), gate, outcome)
}

/// session が `driver.exit()` を発行するまで Tick を回す
/// (自然な driver-loop 終了)。Fake の `exited` は `exit()` 呼出を
/// 記録するだけで pump 自体は止まらないため、exit 後に余分な
/// Tick を流してはいけない — 実 driver では `exit()` = loop
/// 終了 = process 終了で、それ以降の Tick は存在しない。
fn pump_until_exited(session: &mut DesktopSession, driver: &mut FakeDriver) {
    for _ in 0..200 {
        if driver.exited() {
            return;
        }
        driver.inject(DriverEvent::Tick);
        pump(session, driver);
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("driver loop never exited — session must issue exit() once jobs drain");
}

#[test]
fn action_intent_reaches_service_and_replies_on_owner() {
    let mut session = session();
    let mut driver = FakeDriver::new();
    let w = driver.create_window(&spec()).unwrap();
    driver.inject(DriverEvent::Windowed {
        window: w,
        event: WindowLifecycle::Ready,
    });
    driver.inject(DriverEvent::Windowed {
        window: w,
        event: WindowLifecycle::Action(Intent::new("i1", "al.app.info", json!({}))),
    });
    pump(&mut session, &mut driver);
    let delivered = driver.events_for(w);
    assert!(
        delivered
            .iter()
            .any(|e| matches!(e, ruviform_ui::ipc::Event::Result { id, .. } if id == "i1")),
        "expected Result event for intent i1, got {delivered:?}"
    );
}

#[test]
fn tick_drains_ownerless_events_to_all_live_windows() {
    // ownerless job: headless dispatch(owner=None)で spawn し、
    // Tick drain が `RoutedEvent{window: None}` を生む実経路を使う。
    let dir = al_testkit::temp::TempDir::new("al-gui-desktop-tick");
    let wav = dir.join("cap.wav");
    std::fs::write(
        &wav,
        al_testkit::media::wav_s16_bytes(1, 48_000, &[0i16; 960]),
    )
    .unwrap();

    let mut session = session();
    let mut driver = FakeDriver::new();
    let w1 = driver.create_window(&spec()).unwrap();
    let w2 = driver.create_window(&spec()).unwrap();
    for w in [w1, w2] {
        driver.inject(DriverEvent::Windowed {
            window: w,
            event: WindowLifecycle::Ready,
        });
    }
    session.runtime_mut().dispatch_op(
        "job1",
        "al.job.run",
        json!({
            "base_dir": dir.join("jobs").display().to_string(),
            "inputs": [{"role": "capture_bug", "path": wav.display().to_string()}],
            "stages": ["ingest"]
        }),
    );
    // job signal が溜まるまで Tick を繰り返す。実 `al.job.run`
    // (ingest)は負荷下で ~200ms を超え得るため余裕を持たせる。
    for _ in 0..500 {
        driver.inject(DriverEvent::Tick);
        pump(&mut session, &mut driver);
        if !driver.events_for(w1).is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    for w in [w1, w2] {
        let job_events: Vec<_> = driver
            .events_for(w)
            .into_iter()
            .filter(|e| matches!(e, ruviform_ui::ipc::Event::Job { .. }))
            .collect();
        assert!(
            !job_events.is_empty(),
            "ownerless job event should reach live window {w}"
        );
    }
}

#[test]
fn close_requested_approves_and_destroyed_exits_last_window() {
    let mut session = session();
    let mut driver = FakeDriver::new();
    let w = driver.create_window(&spec()).unwrap();
    driver.inject(DriverEvent::Windowed {
        window: w,
        event: WindowLifecycle::Ready,
    });
    driver.request_close(w);
    pump(&mut session, &mut driver);
    // Approve → Destroyed → windows() 空 → exit_on_last_window。
    assert!(driver.destroyed().contains(&w));
    assert!(driver.exited());
    assert!(driver.windows().is_empty());
}

#[test]
fn close_veto_keeps_window_live() {
    let mut session = session();
    let mut driver = FakeDriver::new();
    let w = driver.create_window(&spec()).unwrap();
    driver.inject(DriverEvent::Windowed {
        window: w,
        event: WindowLifecycle::Ready,
    });
    pump(&mut session, &mut driver);
    session.set_close_veto(w, true);
    driver.request_close(w);
    pump(&mut session, &mut driver);
    assert!(!driver.destroyed().contains(&w));
    assert!(driver.windows().contains(&w));
    assert!(!driver.exited());
}

#[test]
fn remaining_live_window_prevents_exit() {
    let mut session = session();
    let mut driver = FakeDriver::new();
    let w1 = driver.create_window(&spec()).unwrap();
    let w2 = driver.create_window(&spec()).unwrap();
    pump(&mut session, &mut driver);
    driver.destroy_window(w1).unwrap();
    pump(&mut session, &mut driver);
    assert!(!driver.exited());
    assert!(driver.windows().contains(&w2));
}

#[test]
fn close_requested_on_last_window_defers_while_job_runs() {
    // 正常 close(request_close → CloseRequested)で最終 window を
    // 閉じようとしても job 実行中は `Defer` で保留: window は
    // close-pending のまま live で、job 一覧・取消 intent・
    // pending 状態が残る(window を消して不可視に待たない)。
    // job 終端の drain 後に close が再開されて自然終了する。
    let (mut session, gate, outcome) = blocking_session(DesktopPolicy::default());
    let mut driver = FakeDriver::new();
    let w = driver.create_window(&spec()).unwrap();
    driver.inject(DriverEvent::Windowed {
        window: w,
        event: WindowLifecycle::Action(Intent::new("j1", "test.block", json!({}))),
    });
    pump(&mut session, &mut driver);
    assert!(
        session.runtime().has_running_jobs(),
        "job should be running"
    );

    driver.request_close(w);
    pump(&mut session, &mut driver);
    assert!(
        driver.windows().contains(&w),
        "last-window close must be held (Defer) while a job runs"
    );
    assert!(!driver.destroyed().contains(&w));
    assert!(
        session.close_deferred(w),
        "deferred close intent must be observable to the UI"
    );
    assert!(!driver.exited());
    assert!(!session.exit_pending());
    assert_eq!(
        *outcome.lock().unwrap(),
        None,
        "job must still be in flight — close deferral is not cancellation"
    );

    // 実行中の再度 close も同じ判定で保留に戻るだけ(状態は
    // 冪等、spam で強行突破できない)。
    driver.request_close(w);
    pump(&mut session, &mut driver);
    assert!(driver.windows().contains(&w));
    assert!(session.close_deferred(w));

    // job が終端へ達すると Tick drain が保留 close を再開 —
    // 通常 flow を再評価して Approve → Destroyed → exit。
    gate.store(true, Ordering::SeqCst);
    pump_until_exited(&mut session, &mut driver);
    assert_eq!(*outcome.lock().unwrap(), Some("finished"));
    assert!(
        driver.destroyed().contains(&w),
        "deferred close must resume and destroy the window"
    );
}

#[test]
fn deferred_close_resume_respects_new_product_veto() {
    // 保留中に製品 veto(未保存データ等)が立った場合、job 終端で
    // 再開される close は強行 destroy ではなく veto-able な
    // `CloseRequested` として再評価される — veto が有効な間は
    // window が残り、解除後の通常 close で完結する。
    let (mut session, gate, _outcome) = blocking_session(DesktopPolicy::default());
    let mut driver = FakeDriver::new();
    let w = driver.create_window(&spec()).unwrap();
    driver.inject(DriverEvent::Windowed {
        window: w,
        event: WindowLifecycle::Action(Intent::new("j1", "test.block", json!({}))),
    });
    pump(&mut session, &mut driver);
    driver.request_close(w);
    pump(&mut session, &mut driver);
    assert!(session.close_deferred(w));

    // 待機中に製品 veto を立てる → job 終端・close 再開後も
    // window は残る(再開が veto を越える destroy 直叩きではない
    // ことの検証)。
    session.set_close_veto(w, true);
    gate.store(true, Ordering::SeqCst);
    for _ in 0..200 {
        driver.inject(DriverEvent::Tick);
        pump(&mut session, &mut driver);
        if !session.runtime().has_running_jobs() {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(!session.runtime().has_running_jobs());
    pump(&mut session, &mut driver); // 再発行された CloseRequested を消費
    assert!(
        driver.windows().contains(&w),
        "resumed close must honor a product veto set during the wait"
    );
    assert!(!session.close_deferred(w));
    assert!(!driver.exited());

    session.set_close_veto(w, false);
    driver.request_close(w);
    pump_until_exited(&mut session, &mut driver);
    assert!(driver.destroyed().contains(&w));
}

#[test]
fn force_destroy_during_deferral_cleans_state() {
    // `Defer` 保留中の window が OS 強制 destroy で消えた経路:
    // `deferred_close` の記録は `Destroyed` で掃除され、job 終端
    // 後も stale な再 close を残さず loop が自然終了する
    // (windowless 待機が起こり得る既知の残余経路)。
    let (mut session, gate, outcome) = blocking_session(DesktopPolicy::default());
    let mut driver = FakeDriver::new();
    let w = driver.create_window(&spec()).unwrap();
    driver.inject(DriverEvent::Windowed {
        window: w,
        event: WindowLifecycle::Action(Intent::new("j1", "test.block", json!({}))),
    });
    pump(&mut session, &mut driver);
    driver.request_close(w);
    pump(&mut session, &mut driver);
    assert!(session.close_deferred(w));

    driver.destroy_window(w).unwrap();
    pump(&mut session, &mut driver);
    assert!(
        !session.close_deferred(w),
        "Destroyed must clean the deferred-close record"
    );
    assert!(driver.windows().is_empty());
    assert!(!driver.exited(), "job still running — exit stays deferred");

    gate.store(true, Ordering::SeqCst);
    pump_until_exited(&mut session, &mut driver);
    assert_eq!(*outcome.lock().unwrap(), Some("finished"));
}

#[test]
fn destroy_does_not_cancel_jobs_by_default() {
    // OS 強制 destroy(veto 不能)経路: 既定 policy は job も
    // loop も止めない — `exit()` は job の終端 signal が drain
    // されるまで遅延する。この経路では job 終端まで window の
    // 無い windowless 待機が起こり得る(正常 close の `Defer`
    // 保留は強制 destroy には適用できない — 既知の残余)。
    // 実 driver では `exit()` = loop 終了 = process 終了で
    // detached worker が未終端 kill されるため、「exit が job
    // 終端より後に来る」ことを loop が自然に終わるまで回して検証。
    let (mut session, gate, outcome) = blocking_session(DesktopPolicy::default());
    let mut driver = FakeDriver::new();
    let w = driver.create_window(&spec()).unwrap();
    driver.inject(DriverEvent::Windowed {
        window: w,
        event: WindowLifecycle::Action(Intent::new("j1", "test.block", json!({}))),
    });
    pump(&mut session, &mut driver);
    assert!(
        session.runtime().has_running_jobs(),
        "job should be running"
    );

    // owner window を destroy — 既定では job は cancel されない
    // (UiRuntime 契約: window close is not job cancellation)。
    driver.destroy_window(w).unwrap();
    pump(&mut session, &mut driver);
    assert!(
        session.runtime().has_running_jobs(),
        "job survives owner destruction by default"
    );
    assert!(
        !driver.exited(),
        "exit must be deferred while a job is running"
    );
    assert_eq!(
        *outcome.lock().unwrap(),
        None,
        "job must still be in flight"
    );

    // gate を開けると job は完走し、終端 drain 後に session が
    // exit() を発行して loop が自然に終わる。
    gate.store(true, Ordering::SeqCst);
    pump_until_exited(&mut session, &mut driver);
    assert_eq!(
        *outcome.lock().unwrap(),
        Some("finished"),
        "default policy must let the orphaned job finish before exit"
    );
}

#[test]
fn cancel_on_destroy_policy_is_opt_in() {
    // 強制 destroy 経路の opt-in: Destroyed で cancel_window_jobs
    // → job 側の check_cancel が cancelled 分岐へ進む(gate は
    // 開かない)。cancel は signal に過ぎず、終端 signal の drain
    // までは exit() が遅延する — worker が cancelled 分岐を
    // 記録した後に loop が自然に終わる。
    let (mut session, _gate, outcome) = blocking_session(DesktopPolicy {
        exit_on_last_window: true,
        cancel_jobs_on_destroy: true,
    });
    let mut driver = FakeDriver::new();
    let w = driver.create_window(&spec()).unwrap();
    driver.inject(DriverEvent::Windowed {
        window: w,
        event: WindowLifecycle::Action(Intent::new("j1", "test.block", json!({}))),
    });
    pump(&mut session, &mut driver);
    assert!(
        session.runtime().has_running_jobs(),
        "job should be running"
    );

    driver.destroy_window(w).unwrap();
    pump(&mut session, &mut driver);
    assert!(
        !driver.exited(),
        "exit must be deferred until the cancelled job terminates"
    );
    pump_until_exited(&mut session, &mut driver);
    assert_eq!(
        *outcome.lock().unwrap(),
        Some("cancelled"),
        "opt-in policy must cancel window-owned jobs on destroy"
    );
}

#[test]
fn close_requested_with_cancel_opt_in_cancels_and_defers() {
    // 正常 close + `cancel_jobs_on_destroy` opt-in: close 要求を
    // 「その window の job を取消」意思として signal し、window
    // 自体は `Defer` で残す。cancelled 終端の drain で close が
    // 再開されて自然終了する。
    let (mut session, _gate, outcome) = blocking_session(DesktopPolicy {
        exit_on_last_window: true,
        cancel_jobs_on_destroy: true,
    });
    let mut driver = FakeDriver::new();
    let w = driver.create_window(&spec()).unwrap();
    driver.inject(DriverEvent::Windowed {
        window: w,
        event: WindowLifecycle::Action(Intent::new("j1", "test.block", json!({}))),
    });
    pump(&mut session, &mut driver);
    assert!(session.runtime().has_running_jobs());

    driver.request_close(w);
    pump(&mut session, &mut driver);
    assert!(
        driver.windows().contains(&w),
        "window stays (Defer) until the cancelled job terminates"
    );
    assert!(session.close_deferred(w));
    assert!(!driver.exited());

    pump_until_exited(&mut session, &mut driver);
    assert_eq!(
        *outcome.lock().unwrap(),
        Some("cancelled"),
        "opt-in must signal cancellation at close time and still wait for the terminal drain"
    );
    assert!(driver.destroyed().contains(&w));
}

#[test]
fn request_exit_defers_until_running_job_finishes() {
    // 明示 quit(`request_exit`): `exit_on_last_window` policy に
    // 依らず終了要求を立てるが、window の閉じ方は veto-able な
    // 通常 close flow を通る — job 実行中の最終 window は
    // `Defer` で残り、終端 drain で close が再開されてから
    // `exit()` する(強制 destroy は veto と job 保留を越えるため
    // 使わない)。
    let (mut session, gate, outcome) = blocking_session(DesktopPolicy {
        exit_on_last_window: false,
        cancel_jobs_on_destroy: false,
    });
    let mut driver = FakeDriver::new();
    let w = driver.create_window(&spec()).unwrap();
    driver.inject(DriverEvent::Windowed {
        window: w,
        event: WindowLifecycle::Action(Intent::new("j1", "test.block", json!({}))),
    });
    pump(&mut session, &mut driver);
    assert!(session.runtime().has_running_jobs());

    session.request_exit(&mut driver);
    pump(&mut session, &mut driver); // queue された CloseRequested を消費
    assert!(
        driver.windows().contains(&w),
        "quit must not destroy the last window while a job runs — it defers the close"
    );
    assert!(session.close_deferred(w));
    assert!(
        session.exit_pending(),
        "explicit quit must arm the exit request"
    );
    assert!(
        !driver.exited(),
        "explicit quit must wait for the running job"
    );

    gate.store(true, Ordering::SeqCst);
    pump_until_exited(&mut session, &mut driver);
    assert_eq!(*outcome.lock().unwrap(), Some("finished"));
    assert!(driver.destroyed().contains(&w));
}

#[test]
fn request_exit_respects_product_veto() {
    // 明示 quit でも製品 veto(未保存データ等)は越えない —
    // veto 中の window は残り、exit 要求は armed のまま待つ。
    // veto 解除後の通常 close で flow が完結する。
    let (mut session, _gate, _outcome) = blocking_session(DesktopPolicy {
        exit_on_last_window: true,
        cancel_jobs_on_destroy: false,
    });
    let mut driver = FakeDriver::new();
    let w = driver.create_window(&spec()).unwrap();
    session.set_close_veto(w, true);

    session.request_exit(&mut driver);
    pump(&mut session, &mut driver);
    assert!(
        driver.windows().contains(&w),
        "product veto must hold even against explicit quit"
    );
    assert!(!driver.exited());

    // veto 解除 → 通常 close → Destroyed → armed 済みの exit。
    session.set_close_veto(w, false);
    driver.request_close(w);
    pump(&mut session, &mut driver);
    assert!(driver.destroyed().contains(&w));
    assert!(driver.exited());
}

#[test]
fn exit_on_last_window_false_keeps_loop_running() {
    // last-window 終了を無効化した policy: 最終 Destroyed でも
    // 終了要求は立たず、loop は存続する(exit は consumer の
    // 明示要求のみ)。
    let (mut session, _gate, _outcome) = blocking_session(DesktopPolicy {
        exit_on_last_window: false,
        cancel_jobs_on_destroy: false,
    });
    let mut driver = FakeDriver::new();
    let w = driver.create_window(&spec()).unwrap();
    pump(&mut session, &mut driver);
    driver.destroy_window(w).unwrap();
    pump(&mut session, &mut driver);
    assert!(driver.windows().is_empty());
    assert!(!driver.exited());
    driver.inject(DriverEvent::Tick);
    pump(&mut session, &mut driver);
    assert!(!driver.exited(), "no exit request was ever armed");
}

fn command_intent(id: &str, action: &str, params: serde_json::Value) -> Intent {
    Intent::new(
        id,
        command::COMMAND_INTENT_OP,
        json!({
            "command_id": id,
            "expect_receipt": true,
            "intent": {
                "kind": "action",
                "target": command::JOB_TARGET,
                "control_id": "job-table",
                "sequence": 1,
                "action": action,
                "params": params,
            },
        }),
    )
}

fn action_on(window: WindowId, intent: Intent) -> DriverEvent {
    DriverEvent::Windowed {
        window,
        event: WindowLifecycle::Action(intent),
    }
}

fn reply(
    driver: &FakeDriver,
    window: WindowId,
    id: &str,
) -> Option<(bool, Option<serde_json::Value>)> {
    driver.events_for(window).into_iter().find_map(|e| match e {
        ruviform_ui::ipc::Event::Result {
            id: rid, ok, data, ..
        } if rid.as_str() == id => Some((*ok, data.clone())),
        _ => None,
    })
}

#[test]
fn select_intent_adopts_the_job_and_marks_the_projection_dirty() {
    let dir = TempDir::new("al-gui-desktop-select");
    let base = dir.join("jobs");
    let job_id = common::make_job(&base, &dir.join("cap.wav"));
    let mut rt = UiRuntime::new(DataRegistry::new());
    rt.register_service(AlGuiService::with_base_dir(&base))
        .expect("al-gui service");
    let mut session = DesktopSession::new(rt);
    let mut driver = FakeDriver::new();
    let w = ready_window(&mut session, &mut driver);
    driver.inject(action_on(
        w,
        command_intent("s1", command::ACTION_SELECT, json!({"row": 0})),
    ));
    pump(&mut session, &mut driver);

    assert_eq!(session.selected_job(), Some(&job_id));
    assert_eq!(reply(&driver, w, "s1").map(|(ok, _)| ok), Some(true));
    assert!(session.take_projection_dirty());
    assert!(!session.take_projection_dirty());
}

#[test]
fn rejected_selection_keeps_the_previous_job() {
    let dir = TempDir::new("al-gui-desktop-reject");
    let base = dir.join("jobs");
    common::make_job(&base, &dir.join("cap.wav"));
    let mut rt = UiRuntime::new(DataRegistry::new());
    rt.register_service(AlGuiService::with_base_dir(&base))
        .expect("al-gui service");
    let mut session = DesktopSession::new(rt);
    let mut driver = FakeDriver::new();
    let w = ready_window(&mut session, &mut driver);
    driver.inject(action_on(
        w,
        command_intent("s2", command::ACTION_SELECT, json!({"row": 7})),
    ));
    pump(&mut session, &mut driver);

    assert_eq!(session.selected_job(), None);
    assert!(!session.take_projection_dirty());
    assert_eq!(reply(&driver, w, "s2").map(|(ok, _)| ok), Some(false));
}

#[test]
fn cancel_without_a_window_job_replies_rejected() {
    let mut session = session();
    let mut driver = FakeDriver::new();
    let w = ready_window(&mut session, &mut driver);
    driver.inject(action_on(
        w,
        command_intent("c1", command::ACTION_CANCEL, json!(null)),
    ));
    pump(&mut session, &mut driver);

    let (ok, _) = reply(&driver, w, "c1").expect("receipt for c1");
    assert!(!ok);
    let delivered = driver.events_for(w);
    assert!(delivered.iter().any(|e| matches!(
        e,
        ruviform_ui::ipc::Event::Result { id, error: Some(err), .. }
            if id == "c1" && err.code == codes::UNKNOWN_JOB
    )));
}

#[test]
fn cancel_intent_cancels_the_window_owned_job_and_replies_accepted() {
    let gate = Arc::new(AtomicBool::new(false));
    let outcome = Arc::new(Mutex::new(None));
    let mut rt = UiRuntime::new(DataRegistry::new());
    rt.register_service(BlockingService {
        gate: gate.clone(),
        outcome: outcome.clone(),
    })
    .expect("blocking service");
    rt.register_service(AlGuiService::new())
        .expect("al-gui service");
    let mut session = DesktopSession::new(rt);
    let mut driver = FakeDriver::new();
    let w = ready_window(&mut session, &mut driver);
    driver.inject(action_on(w, Intent::new("j1", "test.block", json!({}))));
    pump(&mut session, &mut driver);
    assert!(session.runtime().has_running_jobs());

    driver.inject(action_on(
        w,
        command_intent("c1", command::ACTION_CANCEL, json!(null)),
    ));
    pump(&mut session, &mut driver);
    let (ok, data) = reply(&driver, w, "c1").expect("receipt for c1");
    assert!(ok);
    assert_eq!(
        data.as_ref().and_then(|d| d["status"].as_str()),
        Some("accepted")
    );

    for _ in 0..200 {
        if !session.runtime().has_running_jobs() {
            break;
        }
        driver.inject(DriverEvent::Tick);
        pump(&mut session, &mut driver);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        !session.runtime().has_running_jobs(),
        "cancelled job must terminate"
    );
    assert_eq!(*outcome.lock().unwrap(), Some("cancelled"));
    assert!(
        session.take_projection_dirty(),
        "job end must re-project the screen"
    );
}

fn ready_window(session: &mut DesktopSession, driver: &mut FakeDriver) -> WindowId {
    let w = driver.create_window(&spec()).unwrap();
    driver.inject(DriverEvent::Windowed {
        window: w,
        event: WindowLifecycle::Ready,
    });
    pump(session, driver);
    w
}
