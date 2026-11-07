//! `DesktopSession` の FakeDriver lifecycle fixture (AL-G1)。
//! driver-neutral 契約のみを検証し、実 OS window は開かない。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use al_gui::desktop::{DesktopPolicy, DesktopSession};
use al_gui::service::AlGuiService;
use ruviform_ui::ipc::{ErrorBody, Intent};
use ruviform_ui::{
    CallOutcome, DataRegistry, DriverEvent, JobSpec, OpInfo, Service, ServiceContext, UiRuntime,
    WindowDriver, WindowLifecycle, WindowSpec,
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
    // job signal が溜まるまで Tick を繰り返す。
    for _ in 0..20 {
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
fn destroy_does_not_cancel_jobs_by_default() {
    // job 実行中の最終 Destroyed: 既定 policy は job も loop も
    // 止めない — `exit()` は job の終端 signal が drain されるまで
    // 遅延する。実 driver では `exit()` = loop 終了 = process 終了
    // で detached worker が未終端 kill されるため、「exit が job
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

    // opt-in: Destroyed で cancel_window_jobs → job 側の
    // check_cancel が cancelled 分岐へ進む(gate は開かない)。
    // cancel は signal に過ぎず、終端 signal の drain までは
    // exit() が遅延する — worker が cancelled 分岐を記録した後に
    // loop が自然に終わる。
    driver.destroy_window(w).unwrap();
    pump(&mut session, &mut driver);
    pump_until_exited(&mut session, &mut driver);
    assert_eq!(
        *outcome.lock().unwrap(),
        Some("cancelled"),
        "opt-in policy must cancel window-owned jobs on destroy"
    );
}

#[test]
fn request_exit_defers_until_running_job_finishes() {
    // 明示 quit(`request_exit`): `exit_on_last_window` policy に
    // 依らず live window を全 destroy して終了要求を立てるが、
    // 実行中 job の終端までは `exit()` を発行しない。
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
    pump(&mut session, &mut driver); // queue された Destroyed を消費
    assert!(driver.destroyed().contains(&w));
    assert!(driver.windows().is_empty(), "quit destroys all windows");
    assert!(
        !driver.exited(),
        "explicit quit must wait for the running job"
    );

    gate.store(true, Ordering::SeqCst);
    pump_until_exited(&mut session, &mut driver);
    assert_eq!(*outcome.lock().unwrap(), Some("finished"));
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
