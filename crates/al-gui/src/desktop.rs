//! `DesktopSession`: driver-neutral な AudioLens desktop session。
//!
//! `UiRuntime` を内包し、[`WindowDriver`] trait object 経由で
//! [`DriverEvent`] を消費する。GPUI/OS 型は一切持たず、backend 固有の
//! コードは composition root (`bin/al-desktop.rs`, feature `gpui`)に
//! 閉じる(設計: `docs/designs/native-desktop-initial-slices.md`)。
//!
//! イベント配線は ruviform-ui の canonical mapping:
//! `Action(Intent)` → `dispatch_window_intent`、`Tick` →
//! `drain_jobs_routed`、`CloseRequested` → `closing` lifecycle +
//! [`CloseDecision`]、`Destroyed` → teardown policy。
//! live window 集合・last-window 判定は契約どおり
//! `WindowDriver::windows()` を正本とし、session 側に第二の
//! truth を持たない。
//!
//! `driver.exit()` は「app 終了要求が出ていて、live window が無く、
//! 実行中 job も無い」ときにだけ発行する。`exit()` で driver loop
//! (≒process) が終わると `UiRuntime` の detached job worker は
//! 未終端のまま kill されるため、job 実行中は終了要求を
//! `exit_pending` として保留し、`Tick` の drain が終端 signal を
//! 回収した後の最初の機会に `exit()` する — window close/destroy
//! が job を暗黙停止しない契約は process 終了にも及ぶ。

use std::collections::BTreeSet;

use ruviform_ui::{
    CloseDecision, DriverEvent, RoutedEvent, UiRuntime, WindowDriver, WindowId, WindowLifecycle,
};
use ruviform_ui_theme::ResolvedTheme;

/// Window lifecycle の policy。既定値は Issue #166 の受入
/// (window close/destroy が job を暗黙停止しない)を表現する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopPolicy {
    /// live window が尽きたとき driver へ `exit()` を伝える
    /// (single-window desktop の既定: `true`)。
    pub exit_on_last_window: bool,
    /// `Destroyed` で `cancel_window_jobs` を呼ぶか。`UiRuntime` の
    /// 契約どおり opt-in のみ。既定 `false`。
    pub cancel_jobs_on_destroy: bool,
}

impl Default for DesktopPolicy {
    fn default() -> Self {
        Self {
            exit_on_last_window: true,
            cancel_jobs_on_destroy: false,
        }
    }
}

/// AudioLens desktop session。close veto と runtime event の
/// window への配送を担当する。
pub struct DesktopSession {
    runtime: UiRuntime,
    policy: DesktopPolicy,
    /// `CloseRequested` へ `Veto` を返す window 集合
    /// (`set_close_veto` で登録)。
    vetoes: BTreeSet<WindowId>,
    /// composition root が注入する product theme。renderer
    /// (RF#387 系)への受け渡し点。
    theme: Option<ResolvedTheme>,
    /// app 終了が要求されたが実行中 job の終端待ちで保留中。
    /// `request_exit`(明示 quit)か last-window `Destroyed` で立ち、
    /// `maybe_exit` が条件成立時に降ろす。
    exit_pending: bool,
}

impl DesktopSession {
    /// `policy` 付きで session を作る。
    pub fn with_policy(runtime: UiRuntime, policy: DesktopPolicy) -> Self {
        Self {
            runtime,
            policy,
            vetoes: BTreeSet::new(),
            theme: None,
            exit_pending: false,
        }
    }

    /// 既定 policy で session を作る。
    pub fn new(runtime: UiRuntime) -> Self {
        Self::with_policy(runtime, DesktopPolicy::default())
    }

    /// UI runtime への参照(service 登録・headless dispatch 用)。
    pub fn runtime(&self) -> &UiRuntime {
        &self.runtime
    }

    /// UI runtime への可変参照。
    pub fn runtime_mut(&mut self) -> &mut UiRuntime {
        &mut self.runtime
    }

    /// `window` の `CloseRequested` への応答を `Approve`/`Veto` に
    /// 切り替える。veto 登録は `Destroyed` 時に掃除される。
    pub fn set_close_veto(&mut self, window: WindowId, veto: bool) {
        if veto {
            self.vetoes.insert(window);
        } else {
            self.vetoes.remove(&window);
        }
    }

    /// composition root が注入する product theme
    /// (AL-G2 の `theme::resolve` 結果)。
    pub fn set_theme(&mut self, theme: ResolvedTheme) {
        self.theme = Some(theme);
    }

    /// 注入済み product theme。
    pub fn theme(&self) -> Option<&ResolvedTheme> {
        self.theme.as_ref()
    }

    /// app 終了要求が保留中か(実行中 job の終端待ち)。
    /// UI が「job 完了後に終了」を表示するための観測点。
    pub fn exit_pending(&self) -> bool {
        self.exit_pending
    }

    /// 明示的な app 終了要求(将来の Quit action / OS quit 等)。
    /// `exit_on_last_window` policy ではなく consumer の終了意思
    /// なので live window は全て destroy する(各 `Destroyed` は
    /// 通常の policy 経路 — veto 掃除と `cancel_jobs_on_destroy`
    /// opt-in を通る)。job 実行中は `maybe_exit` の規則どおり
    /// 終端まで `exit()` を遅延する。
    pub fn request_exit(&mut self, driver: &mut dyn WindowDriver) {
        self.exit_pending = true;
        for w in driver.windows() {
            let _ = driver.destroy_window(w);
        }
        self.maybe_exit(driver);
    }

    /// Driver から届いた event を消費する。`run` callback から
    /// `&mut dyn WindowDriver` と一緒に呼ぶ。
    pub fn handle(&mut self, ev: DriverEvent, driver: &mut dyn WindowDriver) {
        match ev {
            DriverEvent::Tick => {
                let routed = self.runtime.drain_jobs_routed();
                deliver(routed, driver);
            }
            DriverEvent::Windowed { window, event } => match event {
                WindowLifecycle::Action(intent) => {
                    let routed = self.runtime.dispatch_window_intent(window, intent);
                    deliver(routed, driver);
                }
                WindowLifecycle::CloseRequested => {
                    let routed = self
                        .runtime
                        .handle_window(window, ruviform_ui::HostEvent::CloseRequested);
                    deliver(routed, driver);
                    let decision = if self.vetoes.contains(&window) {
                        CloseDecision::Veto
                    } else {
                        CloseDecision::Approve
                    };
                    // approve 後の Destroyed は backend が発行する。
                    // respond_close 自体の失敗は adapter が記録する。
                    let _ = driver.respond_close(window, decision);
                }
                WindowLifecycle::Destroyed => {
                    self.vetoes.remove(&window);
                    if self.policy.cancel_jobs_on_destroy {
                        self.runtime.cancel_window_jobs(window);
                    }
                    // `windows()` は backend の live window 正本 —
                    // Destroyed 配送時点で既に除去済み。最終 window の
                    // teardown は app 終了要求として `exit_pending` を
                    // 立てるだけ — `exit()` 自体は `maybe_exit` が
                    // 実行中 job の有無を見て発行する。
                    if self.policy.exit_on_last_window && driver.windows().is_empty() {
                        self.exit_pending = true;
                    }
                }
                event => {
                    if let Some(host) = map_lifecycle(event) {
                        let routed = self.runtime.handle_window(window, host);
                        deliver(routed, driver);
                    }
                }
            },
            // DriverEvent is non_exhaustive — unknown future events
            // are ignored rather than faked.
            _ => {}
        }
        self.maybe_exit(driver);
    }

    /// `exit()` を発行できる状態なら発行する。全ての app 終了経路
    /// (last-window policy / `request_exit`)は `exit_pending` へ
    /// 畳まれ、ここで唯一の規則に従う: live window が無く、
    /// `runtime` が実行中 job を報告しないときだけ `exit()`。
    /// job が走っている限り driver loop は存続し、`Tick` drain が
    /// 終端 signal を回収(`has_running_jobs` が落ちる)した後の
    /// 次の機会に発火する — 実行中 job を process 終了で
    /// 黙示停止しない。
    fn maybe_exit(&mut self, driver: &mut dyn WindowDriver) {
        if self.exit_pending && driver.windows().is_empty() && !self.runtime.has_running_jobs() {
            self.exit_pending = false;
            driver.exit();
        }
    }
}

/// `RoutedEvent` を window へ配送する。`Some(w)` はその window へ、
/// `None`(ownerless)は `driver.windows()` の全 window へ
/// `post_event` する(destroyed/未知 window への post_event は
/// driver 契約上 silent drop)。
fn deliver(routed: Vec<RoutedEvent>, driver: &mut dyn WindowDriver) {
    for RoutedEvent { window, event } in routed {
        match window {
            Some(w) => driver.post_event(w, &event),
            None => {
                for w in driver.windows() {
                    driver.post_event(w, &event);
                }
            }
        }
    }
}

/// `WindowLifecycle` を runtime の `HostEvent` 語彙へ写す。
/// `CloseRequested`/`Destroyed`/`Action`/`Tick` は session が先に
/// 処理するためここへは来ない。
///
/// `Ready` は写さない — `HostEvent::Ready` の `url` は WebView
/// adapter 側の関心事で neutral contract には由来が無く、空文字列を
/// 捏造しない(driver.rs 契約)。runtime が未対応の variant(将来追加
/// 含む)にも `None` を返す。
fn map_lifecycle(ev: WindowLifecycle) -> Option<ruviform_ui::HostEvent> {
    use ruviform_ui::HostEvent;
    match ev {
        WindowLifecycle::Resized { width, height } => Some(HostEvent::Resized { width, height }),
        WindowLifecycle::Moved { x, y } => Some(HostEvent::Moved { x, y }),
        WindowLifecycle::Focused(f) => Some(HostEvent::Focused(f)),
        WindowLifecycle::ScaleFactorChanged { factor } => {
            Some(HostEvent::ScaleFactorChanged { factor })
        }
        _ => None,
    }
}
