---
id: "IMPL-IgijqBdvx2YO4lzFPDz4cA"
type: "impl"
status: "completed"
title: "Native desktop 初期 slice AL-G1〜G3 実装記録(IR)"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/native-desktop-initial-slices.md"
  implemented-by: []
  depends-on:
    - "docs/designs/native-desktop-initial-slices.md"
    - "docs/plans/native-desktop-initial-slices.md"
    - "docs/tasks/native-desktop-initial-slices.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/native-desktop-initial-slices.md"
    - "docs/test-specs/native-desktop-initial-slices.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/native-desktop-initial-slices.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/native-desktop-initial-slices.md"
  references:
    - "https://github.com/scarecrow173/ruviform/commit/e8adcc09aaa6444b00b9ba62c24fd2d217fb39f0"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Native desktop 初期 slice AL-G1〜G3 実装記録(IR)

## 実装済み

### AL-G1 — DesktopSession + GPUI composition root

- `crates/al-gui/src/desktop.rs`: `DesktopSession`/`DesktopPolicy`。
  `UiRuntime` を内包し `&mut dyn WindowDriver` 経由で `DriverEvent`
  を消費。GPUI/OS 型なし。
  - `Tick` → `drain_jobs_routed` → `post_event` 配送。
  - `Action(Intent)` → `dispatch_window_intent` → 配送。
  - `CloseRequested` → `handle_window(.., CloseRequested)` +
    `respond_close`。decision の優先順位: 製品 veto → `Veto`、
    最終 live window かつ job 実行中 → `Defer` +
    `deferred_close` 登録(opt-in 時は同時に
    `cancel_window_jobs`)、それ以外 → `Approve`
    (review 第 4 ラウンド対応 — 詳細は判断記録)。
  - `Destroyed` → `vetoes`/`deferred_close` から除去。opt-in のみ
    `cancel_window_jobs`。`exit_on_last_window`(既定)かつ
    `driver.windows().is_empty()` で `exit_pending` を立てる。
    live window 集合は `WindowDriver::windows()` を正本とし
    session 側に第二の追跡集合を持たない。
  - `Tick` drain 直後に `resume_deferred_closes`:
    `!has_running_jobs()` なら `deferred_close` を take し
    live な window へ `request_close` を再発行 — 新しい
    `CloseRequested` として policy/veto が最初から再評価
    される通常 flow に乗る。
  - `maybe_exit` — `driver.exit()` の唯一の発行点。
    `exit_pending && windows().is_empty() && !has_running_jobs()`
    のときのみ発行。`exit()` = loop 終了 = process 終了で
    detached job worker が未終端 kill されるため、実行中 job が
    いる間は終了要求を保留し、`Tick` drain が終端 signal を
    回収した直後に発行する(review 第 2 ラウンド P1 対応)。
    `request_exit(driver)` は明示 quit 経路 —
    `exit_on_last_window` に依らず `exit_pending` を立て live
    window 全てへ `request_close` を発行する(veto-able な
    通常 close flow — 強制 destroy は製品 veto と `Defer` を
    越えるため使わない、review 第 4 ラウンド対応)。
    `exit_pending()`/`close_deferred(w)` getter で保留状態を
    観測できる(「job 完了後に終了/閉じる」を UI が表示する点)。
    `exited` latch が発行済み `exit()` の重複発行と exit 後
    event の dispatch/配送/新規 job spawn を閉じる。
  - `Resized/Moved/Focused/ScaleFactorChanged` → 対応 `HostEvent` へ
    写像。`Ready` は `HostEvent::Ready{url}` が WebView 由来のため
    写さない(空文字列を捏造しない)。未知 variant は無視。
  - `RoutedEvent.window=None`(ownerless)は `driver.windows()` の
    全 live window へ `post_event`。
- `crates/al-gui/src/bin/al-desktop.rs`: `feature = "gpui"` 限定の
  composition root。`GuiPreferences` → `theme::resolve` →
  `UiRuntime` + `AlGuiService` → `GpuiDriver` → `create_window` →
  `run`。GPUI 型の出現はこの file のみ。
- `crates/al-gui/tests/desktop.rs`: `FakeDriver` で
  Action→Intent 配送・Tick→ownerless job drain(実 `al.job.run`)・
  CloseRequested→veto/approve・Destroyed→last-window exit を検証。
  job cancel 契約は `BlockingService` fixture(gate + 観測路)で
  決定的に検証する — `has_running_jobs` の一瞬状態ではなく job が
  辿った分岐(finished/cancelled)を観測。exit 遅延は
  `pump_until_exited` が `driver.exited()` まで Tick を回す
  自然な loop 終了で検証 — `exit()` 後に Tick を流す偽受入は
  しない(9 test)。

### AL-G2 — Product Theme / assets / preferences

- `crates/al-gui/theme/base.tokens.json`: primitive 層
  (`rvf-dtcg-profile.v1`、modes light/dark)。
- `crates/al-gui/theme/product.tokens.json`: product 層の semantic
  token(`color.bg`/`fg`/`surface`/`accent`/`good`/`warn`/`bad`/
  `waveform`)を base palette へ alias。mode 差は
  `dev.ruviform.modes` 経由。
- `crates/al-gui/src/theme.rs`: `documents()`(埋め込み)+
  `resolve(mode)`。DTCG 解釈は `ThemeResolver` へ委譲。
- `crates/al-gui/src/preferences.rs`: `GuiPreferences`
  (theme_mode/window/base_dir)、JSON load/save、`default_path`
  (env override → OS 別 config dir、std env のみ)、
  `resolved_mode(system)`、`window_spec(title)`。
- `crates/al-gui/tests/theme.rs`: 10 test。

### AL-G3 — view model adapter + fixture

- `crates/al-gui/src/view/mod.rs`: `ViewError` 集約 + 行写像 helper。
- `view/job.rs`: `JobRow`/`InputRow`/`StageRow`/`JobDetail`/
  `RunOutcome`。`JobRow.errors`/`RunOutcome.errors`・
  `durations_ms` は stage index 対応 Vec で保持(失敗理由を
  落とさない)。
- `view/analysis.rs`: `EventRow`(`events/candidates` schema)、
  `AlignmentRow`(`locate/summary`)、`FeatureTable`
  (`FeatureRecord` を typed 保持、`_artifact` 拡張は無視)。
  schema 必須列は欠損で `Invalid` — `strength` の JSON null は
  永続化 NaN の表現として `f64::NAN` へ復元し、
  `agreeing_methods`/`capture_start`/`capture_end` の 0 補完は
  しない(`SampleRange{0,0}` の捏造を防ぐ)。
- `view/source.rs`: `WaveformView`(`WaveformSeries::on_demand` +
  bounded `window(start_s,end_s,bins)` + `SampleTimebase`)、
  `SpectrumView`(`Spectrogram` + `tile(&TileRequest)`)、
  `TimeMap`(capture→source 秒 offset+rate 写像。pre-range は
  signed delta で線形外挿、clamp しない)、
  `StreamTimeView`(bounded `ticks(i..j)` + `seconds_at`)。
  `ticks` は `start`/`end` を両端 `len` へ clamp し、
  `start > len` や空 stream でも panic せず空 slice を返す
  (review 第 2 ラウンド P2 対応)。`WaveformView::window` も
  同じ両端 clamp — `seconds_to_sample` の u64::MAX saturate で
  `start + 1` が overflow する経路を `min(frames)` +
  `saturating_add` で閉じた(review 第 3 ラウンド P2 対応)。
  `from_metadata` は永続化 shape `av_timing` の
  `Vec<StreamTiming>` のみを解釈し `Vec<Self>` を返す。
  `timing()`/`timestamp_origin()`/`observation()` で producer の
  `TimingObservation` を typed 露出。`timebase_hz == 0` の
  `seconds_at` は `None`(inf/NaN を捏造しない)。
- `view/dataset.rs`: `TimelineView`(`al.ds.timeline` の
  inputs/event_count/locate_count typed 写し)。
- `crates/al-gui/tests/view.rs`: 単体 8 test + 実 dataset e2e 3 test
  — `real_dataset_produces_view_models`(実 ingest→locate→
  cheap→dataset → JobRow/WaveformView/SpectrumView tile/
  TimelineView/実 AlignmentRow + LocateResult → TimeMap 写像/
  FeatureTable)、`real_expensive_dataset_produces_event_rows`
  (clip fixture → expensive → 実 EventRow)、
  `real_avi_metadata_produces_stream_time_views`(AVI movi_order
  timing → metadata.av_timing → 2 stream の StreamTimeView)。

### 依存/feature 配線

- `Cargo.toml`(workspace): `ruviform-ui-theme`/`ruviform-ui-gpui`/
  `ruviform-ui-test` を ruviform git 依存へ追加(全て既承認 RF git
  source)。
- `crates/al-gui/Cargo.toml`: `al-media`/`al-locate`/
  `ruviform-ui-theme` 追加、`ruviform-ui-gpui`(optional)+
  `feature = "gpui"`、`[[bin]] al-desktop`(required-features)。
  dev-dep: `ruviform-ui-test`/`al-testkit`。
- `Cargo.lock`: `node tools/update-ruviform-rev.mjs e8adcc09…` で
  全 17 ruviform package を `ad99d181` → `e8adcc09` へ更新
  (正規 tool 経由)。これにより `ruviform-ui` の
  `WindowDriver`/`UiRuntime` 最新契約、`ruviform-ui-theme`/
  `ruviform-ui-gpui`/`ruviform-ui-test` が利用可能になった。
- `mise.toml`: `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui` 追加
  (ruviform 側同名 lane と同一構造、`-p al-gui --features gpui`)。

## 検証記録

- `cargo test -p al-gui`: 41 test 全 pass(desktop 14 + service_ops 5 +
  theme 10 + view 12)。review 第 2 ラウンドでは
  `destroy_does_not_cancel_jobs_by_default` が修正前コードで
  `assert!(!driver.exited())` に FAILED することを確認した
  RED → 修正で GREEN。第 3 ラウンド独立 review verdict: ACCEPT
  — P2 sibling(`WaveformView::window` overflow)と P3 3 件
  (`exited` latch・cancel test ordering assert)を同じ修正へ畳んだ。
  第 4 ラウンド(不可視無期限待機の禁止)では
  `close_requested_on_last_window_defers_while_job_runs` 等 4 test が
  修正前コードで `windows().contains(&w)` に FAILED することを
  確認した RED → `Defer` 保留実装で GREEN(自然 driver loop、
  exit 後の Tick 続行・既定強制 cancel・再 close spam 突破なし)。
  第 4 ラウンドの内部独立 review verdict: ACCEPT — P2(保留中に
  立った製品 veto を再開 close が尊重するかの test gap)を
  `deferred_close_resume_respects_new_product_veto` で閉塞、
  P3(強制 destroy 中の deferred 記録掃除・rustdoc 表現ドリフト・
  Defer 副作用の ordering)も同じ修正へ畳んだ。
- `cargo check -p al-gui --all-targets` / `cargo clippy -p al-gui
  --all-targets -- -D warnings`: pass。
- `node tools/check-headless-deps.mjs`: pass。guard の候補名に
  `gpui`/`gpui-`/`ruviform-ui-gpui`/`ruviform-ui-webview` を追加し、
  headless crate への backend 混入を検知可能にした。
- 正規 `mise run` 検証と gpui lane の結果は PR 本文へ記録する。

## 未実施 / Blocked(明記)

- **ViewSpec 実描画接続**: RF#387 の Component/View renderer は別
  Worker 実装中(`4c53e28` 候補 branch あり、main 未 merge)。
  `create_view_window(&WindowSpec, ViewMount{host, theme})` の接続
  は後段統合条件。`DesktopSession::theme()` が受け渡し点。
- **AL-G4**: 実 OS 上の全 lifecycle(Cancelled/Stale/Failure/Save/
  Reload/Source revision/GPU resources/Subscriptions/Job lifetime)
  検証 → 未達。
- **AL-G5**: 実 OS/visual/numeric/CLI 正式受入 → 未達。
- **実 desktop 入力 slot**: RF#389 担当。
- `ruviform-ui-gpui` の `ime`/`native_surface_slot` capability は
  `Unsupported` であり、本件で偽装しない。

## 判断記録

- `cancel_jobs_on_destroy` 既定 `false`: window close/destroy が
  job を暗黙停止しない Issue #166 受入を `DesktopPolicy` の既定値
  として表現。
- `exit_on_last_window` 既定 `true`: single-window desktop の
  自然な終了経路。`Destroyed` でのみ発火(`CloseRequested`
  approve → `Destroyed` → exit)。
- `driver.exit()` は `exit_pending` + `maybe_exit` 経路へ集約
  (review 第 2 ラウンド P1 対応): `exit()` は loop 終了 =
  process 終了を意味し、`UiRuntime` の job worker は JoinHandle を
  保持しない detached thread であるため、実行中 job がある状態で
  exit すると worker が未終端のまま kill される。これは
  「window close/destroy が job を暗黙停止しない」受入を
  process 終了経路で破るため、全ての終了要求(last-window
  policy・`request_exit`)は `exit_pending` に畳み、
  `windows().is_empty() && !has_running_jobs()` が成立した時点で
  のみ `exit()` を発行する。job が走っている限り loop は存続し、
  `Tick` が終端 signal の drain を続ける — 待機は job の実終端に
  束ねられている。`cancel_jobs_on_destroy` opt-in は待機を job
  完走ではなく cancelled 終端へ短縮するだけで、規則は同一。
- 最終 window の `CloseRequested` を job 実行中に `Defer` 保留
  (review 第 4 ラウンド対応 — 前節の「隠れた無期限待機でも
  ない」はこの修正前は過大だった): 第 2 ラウンドの遅延 exit は
  「job を kill しない」を満たしたが、正常 close で全 window が
  消えた後は可視 window・取消導線・表示される pending 状態の
  ない待機が残り、「不可視な無期限待機の禁止」条件を破って
  いた。`CloseDecision::Defer` は契約の設計語彙(window は
  close-pending のまま live — 非同期 cleanup flow 向け)であり、
  window・job 一覧・取消 intent・`close_deferred` の pending
  状態が残るため待機は可視かつ操作可能。`Veto` でなく `Defer`
  を選ぶ理由: `Veto` は close 意思を取消すが、`Defer` は
  close-pending として意思を保持したまま保留する — job 終端で
  close が再開される設計意図と一致する。
- `Defer` の再開は `request_close` 再発行(強制 `destroy_window`
  ではない): 契約上どちらも許容されるが、再発行は新しい
  `CloseRequested` として製品 veto・job 再チェックを最初から
  通るため「close policy/veto の再評価」要件を満たす。待機中に
  製品 veto が立てば再び Veto、新規 job が走れば再び Defer で
  保留へ戻る。`deferred_close` は session 側の再開対象記録で、
  `Destroyed`(強制経路含む)で除去する。
- `request_exit` は `destroy_window` ではなく `request_close` を
  全 window へ発行: 明示 quit も製品 veto(未保存データ等)と
  job 実行中の `Defer` を越えない — これらを踏み越える
  `destroy_window` 直呼びは veto 契約の意味を壊す。quit が
  veto に阻まれた場合 `exit_pending` は armed のまま残り、
  veto 解除後の通常 close で完結する。veto 尊重が必須で、
  未配線の future API 拡張は行わない。
- **既知の残余リスク(保証不能範囲)**: OS 強制 `Destroyed`
  (`destroy_window` / backend 強制 teardown)は veto/`Defer`
  不能であり、最終 window がこの経路で消えた場合は job 終端
  まで window の無い windowless 待機が起こり得る。代替案の
  暗黙 job kill は受入を破るため棄却、job cancel 強制は既定値
  変更を要求するため棄却。軽減策の候補(強制終了 gesture や
  タイムアウト/watchdog、upstream driver 側の pre-teardown 通知)
  は AL-G4 実 OS lifecycle 検証スコープまたは Ruviform 側の
  検討事項として残す — 本範囲で成功を偽装しない。
- `ticks(start, end)` は両端を `len` へ clamp(review 第 2 ラウンド
  P2 対応): 旧実装は `start > len` のとき `end` が `start` に
  引き上げられ `[start..start]` が範囲外で panic した
  (空 stream `ticks(1, 2)`)。
- `WaveformView::window` も同じ clamp 規則(review 第 3 ラウンド
  P2 対応): `seconds_to_sample` が巨大秒を u64::MAX へ saturate
  し、未 clamp の `start` 経由で `start + 1` が overflow panic
  した。`start.min(frames)` + `end.max(start.saturating_add(1))`。
- `exited` latch(review 第 3 ラウンド P3 対応): `exit()` 発行後に
  後続 `Destroyed` が `exit_pending` を再 arm して `exit()` が
  重複する経路と、exit 後 `Action` が終端へ辿り着けない job を
  spawn する latent 経路を、`handle`/`request_exit`/`maybe_exit`
  の早期 return で閉じた。
- `RoutedEvent{None}` の全 window 配送: `UiRuntime` の unrouted
  契約を app-level 通知として正規化。配送先集合は
  `WindowDriver::windows()` を正本とする(契約 designated source
  of truth — session 側の shadow set は廃止)。
- `Ready` → `HostEvent` 非写像: `HostEvent::Ready{url}` の url は
  WebView adapter の関心事であり native session に由来が無い。
  `url: ""` を捏造しない(driver.rs 契約)。
- `StreamTimeView::from_metadata` は `av_timing` の
  `Vec<StreamTiming>` のみを解釈: producer(al-media stage)は
  stream 毎の配列で永続化しており、単一 object や別 key 名は実在
  しない。架空の shape を受理すると実 metadata で常に失敗する
  デッドコードになる。
- EventRow/AlignmentRow の schema 必須列は厳格化: 欠損を 0 補完
  すると `SampleRange{0,0}` のような意味を捏造する。JSON null
  strength は永続化 NaN の表現として `f64::NAN` へ復元する
  (dataset 側 `cell_to_json` の NaN→null 対称)。
- `RunOutcome.errors`/`durations_ms`: service が emit する
  `{stage,status,duration_ms,error}` の失敗理由を保持するため
  index 対応 Vec とした。
- FeatureRecord/LocateResult/StreamTiming の再定義をせず typed
  保持: 表現ドリフトを作らない。
- `tempfile` crate は新外部依存のため不採用 — test は
  `al-testkit::temp::TempDir`/`std::env::temp_dir` で代替。
- `ruviform-media-core` 直接依存は削除: `ruviform-viz` 経由で
  届くため不要だった(review 指摘対応)。
- job cancel fixture は `BlockingService`(gate + outcome 観測路):
  `has_running_jobs` のタイミング依存スナップショットではなく
  job が実際に辿った終端分岐を観測する。
