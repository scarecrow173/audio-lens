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
//!
//! job 終端を「window を消して不可視に待つ」のではなく、最終
//! window の `CloseRequested` を [`CloseDecision::Defer`] で保留
//! する: window は close-pending のまま live で、job 一覧・取消
//! など既存の操作手段と pending 状態(`close_deferred`)が残る。
//! `Tick` drain が job 終端を回収したら保留中の window へ
//! `request_close` を再発行し、close policy/veto を通常 flow で
//! 再評価して `Approve` → `Destroyed` → `exit_pending` → `exit()`
//! へ進む。OS 強制 `Destroyed`(veto 不能)ではこの保留が適用
//! できないため、その経路では job 終端まで window が無い
//! windowless 待機が起こり得る — 既知の残余リスクとして spec /
//! IR に記録する(暗黙 kill より劣位と判断)。

use std::collections::BTreeSet;

use al_core::JobId;
use ruviform_ui::ipc::{ErrorBody, JobState, codes};
use ruviform_ui::{
    CloseDecision, DriverEvent, Event, Intent, RoutedEvent, UiRuntime, WindowDriver, WindowId,
    WindowLifecycle,
};
use ruviform_ui_theme::ResolvedTheme;
use serde_json::json;

use crate::command;

/// Window lifecycle の policy。既定値は Issue #166 の受入
/// (window close/destroy が job を暗黙停止しない)を表現する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopPolicy {
    /// live window が尽きたとき driver へ `exit()` を伝える
    /// (single-window desktop の既定: `true`)。
    pub exit_on_last_window: bool,
    /// window 消去の意思 = job 取消として `cancel_window_jobs` を
    /// 呼ぶか — `Destroyed` 時、および最終 window close の `Defer`
    /// 時点の両方で signal する(後者は「閉じる = その window の
    /// job を取消」の意思)。`UiRuntime` の契約どおり opt-in のみ。
    /// 既定 `false`。
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
    /// job 実行中に [`CloseDecision::Defer`] で保留した
    /// `CloseRequested`。window は close-pending のまま live —
    /// `Tick` drain が job 終端を回収したら `request_close` を
    /// 再発行して通常 close flow で再評価する。
    deferred_close: BTreeSet<WindowId>,
    /// composition root が注入する product theme。renderer
    /// (RF#387 系)への受け渡し点。
    theme: Option<ResolvedTheme>,
    /// app 終了が要求されたが実行中 job の終端待ちで保留中。
    /// `request_exit`(明示 quit)か last-window `Destroyed` で立ち、
    /// `maybe_exit` が条件成立時に降ろす。
    exit_pending: bool,
    /// `driver.exit()` を既に発行したか。exit 後の event 処理・
    /// 再 arm・二度目の `exit()` を全て閉じる latch。
    exited: bool,
    /// 画面の Jobs 選択の結果(起動引数または行選択で決まる job)。
    /// Run・Dataset tab の射影元で、行の index は持たない。
    selected_job: Option<JobId>,
    /// 選択または job の終端で画面の射影が古くなったことを示す。
    /// composition root が [`take_projection_dirty`](Self::take_projection_dirty)
    /// で取り出して再射影する。
    projection_dirty: bool,
}

impl DesktopSession {
    /// `policy` 付きで session を作る。
    pub fn with_policy(runtime: UiRuntime, policy: DesktopPolicy) -> Self {
        Self {
            runtime,
            policy,
            vetoes: BTreeSet::new(),
            deferred_close: BTreeSet::new(),
            theme: None,
            exit_pending: false,
            exited: false,
            selected_job: None,
            projection_dirty: false,
        }
    }

    /// 起動時の job 選択を設定する(`<job-id>` 引数)。
    pub fn set_selected_job(&mut self, job: Option<JobId>) {
        self.selected_job = job;
    }

    /// 現在選択されている job。
    pub fn selected_job(&self) -> Option<&JobId> {
        self.selected_job.as_ref()
    }

    /// 射影が古くなっていたか。読み出すと降ろす。
    pub fn take_projection_dirty(&mut self) -> bool {
        std::mem::take(&mut self.projection_dirty)
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

    /// `window` の close が job 終端待ちで `Defer` 保留中か。
    /// window 自体は live で job 一覧・取消など既存の操作手段が
    /// 使える — UI が「job 完了後に閉じる」状態を表示するための
    /// 観測点。
    pub fn close_deferred(&self, window: WindowId) -> bool {
        self.deferred_close.contains(&window)
    }

    /// 明示的な app 終了要求(将来の Quit action / OS quit 等)。
    /// `exit_on_last_window` policy ではなく consumer の終了意思
    /// だが、window の閉じ方は通常経路と同じ veto-able close
    /// (`request_close` → `CloseRequested`)を通る — 強制
    /// `destroy_window` は製品 veto と job 実行中の `Defer` を
    /// 踏み越えるため使わない。job 実行中は通常 close と同じく
    /// 最終 window が `Defer` で残り、終端 drain で close が
    /// 再開されてから `maybe_exit` の規則どおり `exit()` する。
    /// 製品 veto が残っている window は quit でも越えない —
    /// veto の解除は製品側の所有。
    pub fn request_exit(&mut self, driver: &mut dyn WindowDriver) {
        if self.exited {
            return;
        }
        self.exit_pending = true;
        for w in driver.windows() {
            driver.request_close(w);
        }
        self.maybe_exit(driver);
    }

    /// Driver から届いた event を消費する。`run` callback から
    /// `&mut dyn WindowDriver` と一緒に呼ぶ。
    pub fn handle(&mut self, ev: DriverEvent, driver: &mut dyn WindowDriver) {
        // `exit()` 発行後の event は配送も job spawn もしない —
        // loop は終了済みで、ここで産んだ job は終端へ辿り着けない。
        if self.exited {
            return;
        }
        match ev {
            DriverEvent::Tick => {
                let routed = self.runtime.drain_jobs_routed();
                self.projection_dirty |= routed.iter().any(|r| is_job_end(&r.event));
                deliver(routed, driver);
                // job が全て終端へ達したなら `Defer` で保留した
                // close を通常 flow(request_close → CloseRequested
                // → policy/veto 再評価 → Approve → Destroyed →
                // maybe_exit)で再開する。job は Tick の drain で
                // しか終端へ達しないため再評価点はここだけ。
                self.resume_deferred_closes(driver);
            }
            DriverEvent::Windowed { window, event } => match event {
                WindowLifecycle::Action(intent) => {
                    let selecting = is_action(&intent, command::ACTION_SELECT);
                    let routed = if is_action(&intent, command::ACTION_CANCEL) {
                        self.cancel_window_job(window, intent.id)
                    } else {
                        self.runtime.dispatch_window_intent(window, intent)
                    };
                    if selecting {
                        self.adopt_selection(&routed);
                    }
                    deliver(routed, driver);
                }
                WindowLifecycle::CloseRequested => {
                    let routed = self
                        .runtime
                        .handle_window(window, ruviform_ui::HostEvent::CloseRequested);
                    deliver(routed, driver);
                    let decision = if self.vetoes.contains(&window) {
                        // 製品 veto(未保存データ等)が最優先 —
                        // 判断は製品側の所有で、session は
                        // deferred_close に記録せず自動再開もしない。
                        CloseDecision::Veto
                    } else if driver.windows().len() == 1 && self.runtime.has_running_jobs() {
                        // 最終 window + job 実行中: Approve すると
                        // 全 window が消え、job 終端を可視 window
                        // も取消導線も無いまま不可視に待つことに
                        // なる。`Defer` で close を保留し、window・
                        // job 一覧・取消 intent・pending 状態を残す。
                        // `cancel_jobs_on_destroy` opt-in は
                        // 「閉じる = その window の job を取消」の
                        // 意思として signal を送るが、window は
                        // 終端 drain まで残る点は変わらない。
                        CloseDecision::Defer
                    } else {
                        CloseDecision::Approve
                    };
                    // approve 後の Destroyed は backend が発行する。
                    // respond_close 自体の失敗は adapter が記録する。
                    let _ = driver.respond_close(window, decision);
                    if matches!(decision, CloseDecision::Defer) {
                        self.deferred_close.insert(window);
                        if self.policy.cancel_jobs_on_destroy {
                            self.runtime.cancel_window_jobs(window);
                        }
                    }
                }
                WindowLifecycle::Destroyed => {
                    self.vetoes.remove(&window);
                    self.deferred_close.remove(&window);
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
    /// 黙示停止しない。`exited` latch で二度目の発行を閉じる
    /// (queue に残った後続 `Destroyed` が pending を再 arm しても
    /// `exit()` は重複しない)。
    fn maybe_exit(&mut self, driver: &mut dyn WindowDriver) {
        if self.exited {
            return;
        }
        if self.exit_pending && driver.windows().is_empty() && !self.runtime.has_running_jobs() {
            self.exited = true;
            self.exit_pending = false;
            driver.exit();
        }
    }

    /// window が所有する実行中 job の取消。取消は job の終端で非同期に完了するため、
    /// receipt は取消の受理(`accepted`)か、取消対象が無いことの `rejected` とする。
    fn cancel_window_job(&mut self, window: WindowId, id: String) -> Vec<RoutedEvent> {
        let event = if self.runtime.cancel_window_jobs(window) > 0 {
            Event::result_ok(id, json!({ "status": "accepted" }))
        } else {
            Event::result_err(
                id,
                ErrorBody::new(codes::UNKNOWN_JOB, "no running job to cancel"),
            )
        };
        vec![RoutedEvent {
            window: Some(window),
            event,
        }]
    }

    /// 選択 intent の receipt(`applied`)が返した job を選択として採用する。
    fn adopt_selection(&mut self, routed: &[RoutedEvent]) {
        let job = routed.iter().find_map(|r| match &r.event {
            Event::Result {
                ok: true,
                data: Some(data),
                ..
            } => data
                .get("job_id")?
                .as_str()
                .and_then(|s| JobId::parse(s).ok()),
            _ => None,
        });
        if let Some(job) = job
            && self.selected_job.as_ref() != Some(&job)
        {
            self.selected_job = Some(job);
            self.projection_dirty = true;
        }
    }

    /// `Defer` で保留した close を再開する。job が全て終端へ
    /// 達した(Tick drain 直後の呼び出し前提)ときだけ動き、
    /// 保留中の window へ `request_close` を再発行する — 新しい
    /// `CloseRequested` として policy/veto が最初から再評価
    /// されるため、待機中に立った製品 veto や新規 job でも
    /// 正しく保留へ戻る。途中で強制 destroy された window は
    /// `windows()` に残っていないため skip する。
    fn resume_deferred_closes(&mut self, driver: &mut dyn WindowDriver) {
        if self.runtime.has_running_jobs() || self.deferred_close.is_empty() {
            return;
        }
        let pending = std::mem::take(&mut self.deferred_close);
        for w in pending {
            if driver.windows().contains(&w) {
                driver.request_close(w);
            }
        }
    }
}

fn is_action(intent: &Intent, name: &str) -> bool {
    intent.op == command::COMMAND_INTENT_OP
        && command::decode_action(&intent.params).is_some_and(|a| a.action == name)
}

fn is_job_end(event: &Event) -> bool {
    matches!(
        event,
        Event::Job {
            state: JobState::Finished | JobState::Failed | JobState::Cancelled,
            ..
        }
    )
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
