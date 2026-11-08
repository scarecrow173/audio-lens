//! job 終端 receipt の consumer 側検証(Issue #166 第8 slice)。
//! `DesktopSession` が window へ返す `result`(ack と終端 receipt)を、GPUI driver の
//! inbox と同じく `CommandReceipt::from_result` で decode して ViewHost へ適用し、
//! Run tab の Resume Button の phase と、再射影後の Tabs 選択を確かめる。
//! Tabs は `selection` を持たないため(本 slice の非スコープ)、Run tab は spec の
//! `active` を 1 にして mount する。

mod common;

use std::path::Path;
use std::time::Duration;

use al_core::JobId;
use al_gui::command;
use al_gui::desktop::DesktopSession;
use al_gui::service::AlGuiService;
use al_gui::view::desktop::{DESKTOP_KEY, RESUME_KEY, desktop_screen};
use al_testkit::temp::TempDir;
use ruviform_ui::behavior::{CommandPhase, CommandReceipt};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::ipc::Intent;
use ruviform_ui::view::{
    AttachEpochAllocator, ComponentKey, ComponentSpec, ViewEvent, ViewHost, ViewSpec,
};
use ruviform_ui::{
    DataRegistry, DriverEvent, UiRuntime, WindowDriver, WindowId, WindowLifecycle, WindowSpec,
};
use ruviform_ui_test::FakeDriver;
use serde_json::json;

/// Run tab(index 1)を表示した画面。Tabs の表示選択は spec の `active` で決まる。
fn run_active(mut view: ViewSpec) -> ViewSpec {
    let ComponentSpec::Tabs(tabs) = &mut view.root else {
        panic!("root must be tabs");
    };
    tabs.active = 1;
    view
}

fn pump(session: &mut DesktopSession, driver: &mut FakeDriver) {
    driver.pump(&mut |ev, drv| session.handle(ev, drv));
}

/// GPUI driver の `flush_events`・`drain_inbox` に相当する配線を持つ Fake 層。
struct Harness {
    session: DesktopSession,
    driver: FakeDriver,
    window: WindowId,
    host: ViewHost,
    delivered: usize,
}

impl Harness {
    fn new(base: &Path, job: &JobId) -> Self {
        let mut rt = UiRuntime::new(DataRegistry::new());
        rt.register_service(AlGuiService::with_base_dir(base))
            .expect("al-gui service");
        let mut session = DesktopSession::new(rt);
        let mut driver = FakeDriver::new();
        let window = driver
            .create_window(&WindowSpec {
                title: "AudioLens".into(),
                ..WindowSpec::default()
            })
            .expect("window");
        driver.inject(DriverEvent::Windowed {
            window,
            event: WindowLifecycle::Ready,
        });
        pump(&mut session, &mut driver);
        let mut host = ViewHost::new(
            ViewInstanceId::new(1).expect("nonzero"),
            AttachEpochAllocator::new().mint(),
        );
        host.mount(run_active(desktop_screen(Some(base), Some(job))))
            .expect("mount");
        host.drain_events();
        Self {
            session,
            driver,
            window,
            host,
            delivered: 0,
        }
    }

    fn phase(&self, key: &str) -> CommandPhase {
        self.host
            .observe(&ComponentKey::from(key))
            .expect("mounted component")
            .phase
    }

    /// ViewHost が発行した command intent を window へ届け、session を進める。
    fn flush_intents(&mut self) {
        for event in self.host.drain_events() {
            if let ViewEvent::Intent {
                intent,
                command_id,
                expect_receipt,
            } = event
            {
                let params = json!({
                    "command_id": command_id,
                    "expect_receipt": expect_receipt,
                    "intent": intent,
                });
                self.driver.inject(DriverEvent::Windowed {
                    window: self.window,
                    event: WindowLifecycle::Action(Intent::new(
                        command_id,
                        command::COMMAND_INTENT_OP,
                        params,
                    )),
                });
            }
        }
        pump(&mut self.session, &mut self.driver);
        self.deliver_receipts();
    }

    /// window へ届いた未適用の `result` を receipt として ViewHost へ適用する。
    fn deliver_receipts(&mut self) {
        let events = self.driver.events_for(self.window);
        for event in &events[self.delivered..] {
            if let Some(receipt) = CommandReceipt::from_result(event) {
                self.host.apply_receipt(&receipt);
            }
        }
        self.delivered = events.len();
    }

    /// job が終端に達するまで Tick を流し、届いた receipt を適用する。
    fn drive_until_idle(&mut self) {
        for _ in 0..500 {
            if !self.session.runtime().has_running_jobs() {
                break;
            }
            self.driver.inject(DriverEvent::Tick);
            pump(&mut self.session, &mut self.driver);
            self.deliver_receipts();
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    /// AL の `update_view` 相当の再射影(同一 key の reconcile)。
    fn reproject(&mut self, view: ViewSpec) {
        let generation = self.host.generation();
        self.host.update(view, generation).expect("update");
    }
}

#[test]
fn resume_receipt_moves_the_run_button_from_pending_to_applied() {
    let dir = TempDir::new("al-gui-receipt-applied");
    let base = dir.join("jobs");
    let job_id = common::make_job(&base, &dir.join("cap.wav"));
    let mut h = Harness::new(&base, &job_id);

    h.host
        .invoke(h.host.generation(), &ComponentKey::from(RESUME_KEY))
        .expect("invoke resume");
    assert_eq!(h.phase(RESUME_KEY), CommandPhase::Pending);

    h.flush_intents();
    assert_eq!(h.phase(RESUME_KEY), CommandPhase::Accepted, "ack");

    h.drive_until_idle();
    assert_eq!(
        h.phase(RESUME_KEY),
        CommandPhase::Applied,
        "terminal receipt"
    );
    assert!(h.session.take_projection_dirty());

    h.reproject(run_active(desktop_screen(Some(&base), Some(&job_id))));
    assert_eq!(
        h.host
            .observe(&ComponentKey::from(DESKTOP_KEY))
            .expect("tabs")
            .selected,
        Some(1),
        "the displayed Run tab survives a same-spec re-projection"
    );
    assert_eq!(h.phase(RESUME_KEY), CommandPhase::Applied);
}

#[test]
fn spec_driven_reprojection_returns_the_displayed_tab_to_jobs() {
    // AL の再射影は Tabs の active を spec の値(Jobs = 0)で作る。spec の
    // active が変わるため、表示中の Run tab は Jobs に戻る(第7 slice の既知の挙動)。
    let dir = TempDir::new("al-gui-receipt-tabs");
    let base = dir.join("jobs");
    let job_id = common::make_job(&base, &dir.join("cap.wav"));
    let mut h = Harness::new(&base, &job_id);
    assert_eq!(
        h.host
            .observe(&ComponentKey::from(DESKTOP_KEY))
            .expect("tabs")
            .selected,
        Some(1)
    );

    h.reproject(desktop_screen(Some(&base), Some(&job_id)));
    assert_eq!(
        h.host
            .observe(&ComponentKey::from(DESKTOP_KEY))
            .expect("tabs")
            .selected,
        Some(0)
    );
}
