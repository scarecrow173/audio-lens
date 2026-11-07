---
id: "DESIGN-WjCNQiQJztajyjGCJIjb0y"
type: "design"
status: "approved"
title: "Native desktop 初期 slice AL-G1〜G3 の設計"
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
    - "../specs/native-desktop-initial-slices.md"
  implemented-by:
    - "../plans/native-desktop-initial-slices.md"
  depends-on:
    - "../discovery/native-desktop-initial-slices.md"
    - "../specs/native-desktop-initial-slices.md"
    - "gui-app-architecture.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/native-desktop-initial-slices.md"
    - "gui-app-architecture.md"
  refines:
    - "gui-app-architecture.md"
  refined-by: []
  derives-from:
    - "../specs/native-desktop-initial-slices.md"
    - "../discovery/native-desktop-initial-slices.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/native-desktop-initial-slices.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/385"
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "https://github.com/scarecrow173/ruviform/issues/387"
    - "https://github.com/scarecrow173/ruviform/commit/e8adcc09aaa6444b00b9ba62c24fd2d217fb39f0"
  defers: []
  deferred-by: []
---

# Native desktop 初期 slice AL-G1〜G3 の設計

## Context

[discovery](../discovery/native-desktop-initial-slices.md) の
棚卸しどおり、Ruviform `e8adcc09` には driver-neutral な
`WindowDriver`/`FakeDriver`/`GpuiDriver`/`ruviform-ui-theme` が
揃っている一方、RF #387 の Component/View renderer は未提供。
本設計は `al-gui` に「renderer が差し込める正しい形」の
session・theme・view model を置く。実描画接続は後段統合条件。

## 依存方向

```text
bin/al-desktop.rs (feature=gpui)         ← 唯一 GPUI 型を持つ層
  → al-gui::desktop::DesktopSession      ← driver-neutral (gpui なし)
  → al-gui::theme / preferences          ← ResolvedTheme/WindowSpec
  → ruviform-ui (WindowDriver/DriverEvent/UiRuntime/ipc)
  → ruviform-ui-gpui (GpuiDriver; feature 内のみ)
al-gui::view                             ← Job/Analysis/Source VM
  → al-service / al-dataset / al-core / al-locate / al-media
  → ruviform-viz / ruviform-media-core
```

headless crates への依存追加はなく、`al-gui` lib の既定 feature は
これまでどおり GUI backend 非依存(gpui crate は feature 内のみ)。

## AL-G1 — `DesktopSession` と composition root

```rust
pub struct DesktopPolicy {
    pub exit_on_last_window: bool,   // default true
    pub cancel_jobs_on_destroy: bool, // default false
}

pub struct DesktopSession {
    runtime: UiRuntime,
    policy: DesktopPolicy,
    vetoes: BTreeSet<WindowId>,
    theme: Option<ResolvedTheme>,
    exit_pending: bool,
}
```

- `handle(&mut self, ev: DriverEvent, driver: &mut dyn WindowDriver)`
  - `Tick` → `deliver(runtime.drain_jobs_routed())`
  - `Windowed{Action(intent)}` → `deliver(dispatch_window_intent(w, intent))`
  - `Windowed{CloseRequested}` → `deliver(handle_window(w, HostEvent::CloseRequested))`
    → `driver.respond_close(w, decision)`。decision は veto window
    なら `Veto`、それ以外は `Approve`。
  - `Windowed{Destroyed}` → `vetoes` から除去。
    `cancel_jobs_on_destroy` 時のみ `cancel_window_jobs`。
    `exit_on_last_window && driver.windows().is_empty()` →
    `exit_pending = true`(app 終了要求の保留)。live window 集合は
    `WindowDriver::windows()` を正本とし、session 側に第二の
    追跡集合を持たない(契約の designated source of truth)。
  - `Windowed{Resized/Moved/Focused/ScaleFactorChanged}` → 対応する
    `HostEvent` へ写して `handle_window`。
    `Ready` は写さない — `HostEvent::Ready` の `url` は WebView
    adapter の関心事であり、空文字列を捏造しない。
  - 未知の `DriverEvent`/`WindowLifecycle` variant(non-exhaustive)
    は無視する — 意味を捏造しない。
  - 各 event 処理の末尾で `maybe_exit` を評価する。
- `maybe_exit` — `driver.exit()` の唯一の発行点。`exit_pending &&
  driver.windows().is_empty() && !runtime.has_running_jobs()` の
  ときだけ発行する。`exit()` = loop 終了 = process 終了で
  detached job worker が未終端 kill されるため、実行中 job が
  ある間は終了要求を保留し、`Tick` drain が終端 signal を回収
  (`has_running_jobs` が落ちる)した直後の機会に発行する。
  `cancel_jobs_on_destroy` opt-in も同じ規則 — cancel は
  signal であって終端ではなく、cancelled 報告の drain までは
  exit を遅延する。
- `request_exit(&mut self, driver)` — 明示的な app 終了要求
  (将来の Quit action / OS quit)。`exit_on_last_window` に依らず
  `exit_pending` を立て、live window を全て `destroy_window`
  する(各 `Destroyed` は通常経路で veto 掃除・opt-in cancel を
  通る)。job 実行中は同じく終端まで `exit()` を遅延する。
- `exit_pending()` — 保留中の終了要求を観測する getter
  (「job 完了後に終了」を UI が表示するための点)。
- `deliver`: `RoutedEvent.window = Some(w)` → `post_event(w)`。
  `None` → `driver.windows()` の全 window へ `post_event`
  (ownerless = app-level)。
- `set_close_veto(window, veto)` / `runtime()`/`runtime_mut()` /
  `theme()`/`set_theme()` を公開。

`bin/al-desktop.rs`(required-features = `gpui`):

```text
GuiPreferences::load_or_default(default_path())
→ theme::resolve(resolved_mode(None))     // ResolvedTheme
→ UiRuntime::new(DataRegistry::new()) + AlGuiService
→ GpuiDriver::new(); create_window(prefs.window_spec("AudioLens"))
→ driver.run(tick, |ev, drv| session.handle(ev, drv))
```

`GpuiDriver`/`gpui` 型が現れるのはこの file のみ。`DesktopSession`
は `ruviform-ui-test::FakeDriver` で headless に検証する。

## AL-G2 — Theme / preferences

`crates/al-gui/theme/`:

- `base.tokens.json`: `rvf-dtcg-profile.v1`、layer=primitive、
  `modes=["light","dark"]`。palette(`color.*`)、dimension
  (`space.*`/`radius.*`)、fontFamily/fontWeight/number/duration。
- `product.tokens.json`: layer=product。`color.bg`/`color.fg`/
  `color.accent`/`color.warn`/`color.bad` 等 semantic 名を base
  palette へ alias(`{color.neutral.900}` 等)。`dev.ruviform.modes`
  で light/dark の値差を持つ。`mode.system` 相当は resolver では
  なく preferences が解決する。

`theme.rs`:

```rust
pub const MODES: &[&str] = &["light", "dark"];
pub fn documents() -> Result<Vec<TokenDocument>, TokenError>;
pub fn resolve(mode: &str) -> Result<ResolvedTheme, ResolveError>;
```

`preferences.rs`:

```rust
pub struct GuiPreferences { pub theme_mode: ThemeMode, pub window: WindowPrefs,
                            pub base_dir: Option<PathBuf> }
pub enum ThemeMode { System, Light, Dark }
pub struct WindowPrefs { pub width: u32, pub height: u32, pub resizable: bool }
```

- `default_path()`: `AL_GUI_PREFERENCES` env → Windows
  `%APPDATA%\AudioLens\gui-preferences.json`、macOS
  `~/Library/Application Support/AudioLens/…`、その他 unix は
  `XDG_CONFIG_HOME`/`~/.config/audiolens/…`。std env のみ。
- `load`/`save`(親 dir 作成付き)/`load_or_default`。
- `resolved_mode(system: Option<&str>)`: System 時は `system` が
  `MODES` 内ならそれ、無効/未提供なら `FALLBACK_MODE = "dark"`。
- `window_spec(title)`: `WindowSpec{title, LogicalSize, None, resizable}`。

## AL-G3 — `al-gui::view`

- `view/job.rs`: `JobRow{job_id, inputs: Vec<InputRow{input_id, role:
  String, path: String}>, stages: Vec<StageRow{kind: StageKind, status:
  StageStatus}>, errors: Vec<Option<String>>, dataset_ready}` ←
  `JobSummary`。`JobDetail` は `JobInspection` から
  manifest/dataset_ready/artifact 数を写す。`RunOutcome` は
  `al.job.run` result の `{stage,status,duration_ms,error}` 行を
  `stages`/`errors`/`durations_ms` の index 対応 Vec で保持する
  (失敗理由を落とさない)。
- `view/analysis.rs`: `EventRow`(events/candidates schema の typed
  写し: `start_s`/`end_s`/`strength`/`freq_lo_hz`/`freq_hi_hz` は
  f64、`confidence`/`validity` は snake_case 文字列を
  `serde_json` 経由で `Confidence`/`Validity` へ)、
  `AlignmentRow`(locate/summary: `accepted`・`capture_start`/
  `capture_end` u64・`agreeing_methods` u64)、
  `FeatureTable{records: Vec<FeatureRecord>, truncated}`(`_artifact`
  拡張 field は serde が未知として無視し FeatureRecord に写す)。
  各行 `from_json(&Value) -> Result<Self, ViewError>`。
- `view/source.rs`:
  - `WaveformView`: `Arc<DatasetWaveform>` を
    `WaveformSeries::on_demand` で包み、`window(start_s,end_s,bins)`
    → `Vec<ViewBin>`(bins 上限の bounded read)。
    `SampleTimebase` を `info().sample_rate` から保持し
    seconds↔samples を提供。
  - `SpectrumView`: `Spectrogram` + `SpectrumInfo` を保持し
    `tile(&TileRequest)` → `Tile`。`FrameGrid`/`BinAxis` で
    frame↔sample/bin↔Hz を提供。
  - `TimeMap`: `LocateResult` + source/capture `sample_rate` から
    `capture_sample → source 秒` 写像(`source_range.start +
    (s - capture_range.start) * src_rate / cap_rate` の offset+rate
    写像)を提供。`capture_range` 以前の sample は線形外挿し、
    clamp しない(負の delta を signed で保つ)。
  - `StreamTimeView`: `StreamTiming` の bounded 写し
    (stream・`stream_type`・`timebase_hz`・`start_tick`・全
    `stream_timestamps` ではなく window read `ticks(i..j)`)と
    `seconds_at(i) = ts[i] / timebase_hz`(`timebase_hz == 0` の
    malformed stream には `None`、inf/NaN を捏造しない)。
    `from_metadata(meta)` は永続化 shape どおり `av_timing` の
    `Vec<StreamTiming>` のみを解釈し `Vec<Self>` で返す。
    `timing()`/`timestamp_origin()`/`observation()` で producer が
    計算した `TimingObservation`(discontinuities・duplicates・
    drop_candidates・drift_ppm)を typed のまま露出する。
- `view/dataset.rs`: `TimelineView`(`al.ds.timeline` の
  `{inputs:[{input_id,frames,sample_rate,channels,duration_s}],
  event_count,locate_count}` の typed 写し)。必須列欠損は
  `Invalid` とし、0 補完しない。

`ViewError` は `al_gui::view::ViewError` 1 型に集約
(`invalid`/`not_found`/`service` 由来を区別)。

## 決定と理由

- **全て `al-gui` に置く**: view model/theme/preferences は
  presentation 層の責務(gui-app-architecture と一致)。新 crate は
  workspace members/lock/index の共有 file を増やし、G1〜G3 の
  先行 slice では層分離の実益が小さい(KISS/YAGNI)。
- **`DesktopPolicy` は data として公開**: 既定値が Issue #166 の
  受入(close で job を暗黙停止しない)を直に表現する。
  `cancel_jobs_on_destroy` は opt-in 経路としてのみ残す。
- **`None` routed event は全 live window へ**: ownerless job は
  app-level 通知として扱う。これは `UiRuntime::drain_jobs_routed`
  の「owner の無い event も破棄しない」契約と整合する。
- **`ResolvedTheme` → GPUI style 変換は作らない**: Driver/renderer
  側(RF #386/#387 系)の責務。al-gui は `ResolvedTheme` を
  保持・注入する点までが境界。
- **`FeatureRecord`/`LocateResult`/`StreamTiming` を再定義しない**:
  typed 契約をそのまま保持し、写し替えで表現ドリフトを作らない。
- **新 crate を作らない**: shared members/index への余計な
  union ポイントを避ける。
