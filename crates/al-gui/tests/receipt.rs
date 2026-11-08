//! job 終端 receipt と Tabs 選択 receipt の consumer 側検証(Issue #166 第8・第9 slice)。
//! `DesktopSession` が window へ返す `result`(ack・終端 receipt・tab-select の receipt)を、
//! GPUI driver の inbox と同じく `CommandReceipt::from_result` で decode して ViewHost へ
//! 適用し、Run tab の Resume の phase、pointer 相当の Tabs 選択、再射影後の選択を確かめる。

mod common;

use std::path::Path;
use std::time::Duration;

use al_core::JobId;
use al_gui::command;
use al_gui::desktop::DesktopSession;
use al_gui::service::AlGuiService;
use al_gui::view::desktop::{DESKTOP_KEY, RESUME_KEY, desktop_screen};
use al_testkit::temp::TempDir;
use ruviform_ui::behavior::{CommandPhase, CommandReceipt, ReceiptStatus};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::ipc::Intent;
use ruviform_ui::view::{AttachEpochAllocator, ComponentKey, ViewEvent, ViewHost, ViewSpec};
use ruviform_ui::{
    DataRegistry, DriverEvent, UiRuntime, WindowDriver, WindowId, WindowLifecycle, WindowSpec,
};
use ruviform_ui_test::FakeDriver;
use serde_json::json;

/// Run tab の index。
const RUN_TAB: usize = 1;

fn pump(session: &mut DesktopSession, driver: &mut FakeDriver) {
    driver.pump(&mut |ev, drv| session.handle(ev, drv));
}

/// window が受け取った receipt の status 列。
fn receipt_statuses(h: &Harness) -> Vec<ReceiptStatus> {
    h.driver
        .events_for(h.window)
        .iter()
        .filter_map(|e| CommandReceipt::from_result(e).map(|r| r.status))
        .collect()
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
        Self::start(base, job, true)
    }

    /// `with_service` が false の場合は al-gui service を登録しない(command は未処理になる)。
    fn start(base: &Path, job: &JobId, with_service: bool) -> Self {
        let mut rt = UiRuntime::new(DataRegistry::new());
        if with_service {
            rt.register_service(AlGuiService::with_base_dir(base))
                .expect("al-gui service");
        }
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
        host.mount(desktop_screen(Some(base), Some(job)))
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

    fn selected(&self, key: &str) -> Option<usize> {
        self.host
            .observe(&ComponentKey::from(key))
            .expect("mounted component")
            .selected
    }

    /// pointer の tab header 押下相当: `ViewHost::select_tab` で表示を移し、receipt を待つ。
    fn press_tab(&mut self, index: usize) {
        let generation = self.host.generation();
        self.host
            .select_tab(generation, &ComponentKey::from(DESKTOP_KEY), index)
            .expect("select tab");
        self.flush_intents();
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
    h.press_tab(RUN_TAB);
    assert_eq!(h.selected(DESKTOP_KEY), Some(RUN_TAB));

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

    h.reproject(desktop_screen(Some(&base), Some(&job_id)));
    assert_eq!(
        h.selected(DESKTOP_KEY),
        Some(RUN_TAB),
        "the pointer-selected Run tab survives a re-projection whose active tab is unchanged"
    );
    assert_eq!(h.phase(RESUME_KEY), CommandPhase::Applied);
}

#[test]
fn pointer_tab_selection_is_applied_by_the_service_receipt() {
    let dir = TempDir::new("al-gui-receipt-tab-applied");
    let base = dir.join("jobs");
    let job_id = common::make_job(&base, &dir.join("cap.wav"));
    let mut h = Harness::new(&base, &job_id);

    h.press_tab(RUN_TAB);
    assert_eq!(
        receipt_statuses(&h),
        [ReceiptStatus::Applied],
        "the service replies applied"
    );
    assert_eq!(
        h.selected(DESKTOP_KEY),
        Some(RUN_TAB),
        "applied keeps the tab"
    );
    assert!(
        !h.session.take_projection_dirty(),
        "a tab switch does not change the job projection"
    );
}

#[test]
fn rejected_tab_selection_restores_the_previous_tab() {
    // al-gui service を登録しない runtime では tab-select の command は処理されず
    // rejected の receipt になる。ViewHost は直前の表示 tab(Jobs)へ戻す。
    let dir = TempDir::new("al-gui-receipt-tab-rejected");
    let base = dir.join("jobs");
    let job_id = common::make_job(&base, &dir.join("cap.wav"));
    let mut h = Harness::start(&base, &job_id, false);

    h.press_tab(RUN_TAB);
    assert_eq!(
        receipt_statuses(&h),
        [ReceiptStatus::Rejected],
        "the unhandled command replies rejected"
    );
    assert_eq!(h.selected(DESKTOP_KEY), Some(0), "rejected restores Jobs");
}
