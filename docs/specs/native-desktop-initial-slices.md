---
id: "SPEC-VCx77hBs4d2qAIPreErlEF"
type: "spec"
status: "approved"
title: "Native desktop 初期 slice AL-G1〜G3 (Issue #166 先行実装)"
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
  implements: []
  implemented-by: []
  depends-on:
    - "../discovery/native-desktop-initial-slices.md"
    - "../designs/gui-app-architecture.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/gui-app-mvp.md"
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/385"
    - "https://github.com/scarecrow173/ruviform/issues/386"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/native-desktop-initial-slices.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/native-desktop-initial-slices.md"
  references: []
  defers: []
  deferred-by: []
---

# Native desktop 初期 slice AL-G1〜G3 (Issue #166 先行実装)

## What Should Be Built

Issue [#166](https://github.com/scarecrow173/audio-lens/issues/166)
の確定仕様 v2 に従い、Ruviform の driver-neutral GUI 契約が使える
範囲で AL-G1〜G3 の先行実装を `crates/al-gui` に追加する。
対象 ruviform resolved SHA は `e8adcc09`(Theme 契約・最小
Window/Driver/Slot/Session を含む main)。

### AL-G1 — Desktop composition root と Window lifecycle

- `al-gui` に **driver-neutral な `DesktopSession`** を追加する。
  `UiRuntime` を内包し、`WindowDriver` trait object 経由で
  `DriverEvent` を消費する。GPUI/OS 型を一切持たない。
- イベント配線は ruviform-ui の canonical mapping に従う:
  `Action(Intent)`→`dispatch_window_intent`、`Tick`→
  `drain_jobs_routed`、`CloseRequested`→`closing` lifecycle
  event + close 決定、`Destroyed`→teardown policy。
- `RoutedEvent` の配送: `Some(window)` はその window へ
  `post_event`、`None`(ownerless)は全 live window へ
  `post_event`(app-level 通知の既定意味)。
- Window lifecycle policy:
  - `set_close_veto(window, bool)` による veto 登録を持ち、
    `CloseRequested` は veto window へ `CloseDecision::Veto`、
    それ以外へ `Approve` を返す。
  - `exit_on_last_window`(default true): `Destroyed` で live
    window が尽きたら app 終了を要求する。ただし `driver.exit()`
    自体は実行中 Job が無い時点まで遅延される — `exit()` で
    loop/process が終わると detached job worker が未終端のまま
    kill されるため、終了要求は `exit_pending` に保留し、
    `Tick` drain が終端 signal を回収した後に発行する。
  - 明示的な app 終了要求(将来の Quit 経路)は `request_exit` が
    受け、live window を全 destroy した上で同じ遅延規則に従う。
  - `cancel_jobs_on_destroy`(default false): opt-in のみ
    `cancel_window_jobs`。既定では window close/destroy が
    Job を暗黙停止しない(Issue #166 受入に整合)。opt-in cancel
    後も `exit()` は終端 signal の drain まで遅延する。
- `feature = "gpui"` 限定の composition root binary
  `al-desktop`: `GuiPreferences` → `ResolvedTheme` →
  `GpuiDriver::new()` → `WindowSpec` → `run` → `session.handle`。
  GPUI 型の出現はこの binary 内に閉じる。
- 既存 `al-gui` binary(webview feature)と `al` CLI/headless/
  解析/数値契約は変更しない。

### AL-G2 — Product Theme / assets / preferences

- AudioLens 固有の DTCG token document を `al-gui` が所有する
  (`crates/al-gui/theme/*.tokens.json` を Git 管理・crate 埋め込み)。
  base(primitive)層 + product 層の 2 文書構成とし、alias・
  mode(light/dark)・layer override の実経路を使う。
- DTCG 解釈・単位・alias・override の解決は全て Ruviform #385 の
  `TokenDocument`/`ThemeResolver`/`ResolvedTheme` に委譲する。
  独自の token 型・単位・パースを作らない。
- `GuiPreferences` を `al-gui` が所有する:
  `theme_mode`(system/light/dark)、window 初期サイズ・リサイズ可否、
  既定 `base_dir`。JSON file への load/save、OS 別既定 path
  解決(std env のみ・新外部依存なし)、`resolved_mode(system)`
  と `window_spec(title)` 射影を持つ。
- `ResolvedTheme` は composition root が session へ保持する。
  GPUI style 型への変換は Driver/renderer 側(RF #387 系)の
  責務であり、本件では行わない。

### AL-G3 — Job/Run/Analysis/Source view model adapter + fixture

- driver-neutral view model を `al-gui::view` に追加する。
  表示意味の写しであり、f64/DSP/解析の再計算は行わない。
- Job/Run: `JobSummary`/`JobInspection`/`Manifest` の typed
  mapping(`StageKind`/`StageStatus`/`InputRole` 保持)。
- Analysis: events/candidates 行・`FeatureRecord`・
  locate summary 行を `Confidence`/`Validity`/`Provenance`/
  `SampleRange` の typed 形で保持する mapping。
- Source: `DatasetSession::waveform`/`spectrum` を
  `WaveformSeries`/`Spectrogram` へ束ね、**bounded**
  window(秒範囲 + bins 上限)・tile(`TileRequest`)read を提供。
  長尺 Dataset の全量ロードをしない。
- TimeMap/PTS: alignment(`LocateResult` 由来)の source↔capture
  sample 写像と、`StreamTiming`(start_tick・stream_timestamps・
  `timebase_hz`)の bounded window を提供する。
  `SampleTimebase`/`FrameGrid` で seconds↔samples↔STFT frame を
  換算する。
- これらの view model は RF #387 renderer の消費対象であり、
  実 ViewSpec への接続は本件では行わない(後段統合条件)。

## Acceptance Criteria

- [ ] `cargo check --workspace`(既定 feature)が GPUI crate 非依存で
  通り、headless crates への GUI framework 依存が増えない
  (`mise run check-cli-headless`)。
- [ ] `DesktopSession` の FakeDriver fixture で
  Action→Intent 配送・Tick→job drain・CloseRequested→veto/approve・
  Destroyed→last-window exit・owner routing が検証される。
- [ ] theme resolve が light/dark 両 mode で成功し、product 層の
  override・alias 解決・mode 選択が確認される。
- [ ] view model が実 dataset(ingest+locate+cheap+dataset stage)を
  経て `Confidence`/`Validity`/`Provenance`/`SampleRange`/bounded
  read を保持することが fixture で検証される。
- [ ] GPUI crate 型は `al-gui` の `gpui` feature 配下
  (`bin/al-desktop.rs`)にのみ存在する。
- [ ] `feature = "gpui"` 付きの `al-desktop` が
  `mise run check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui`
  で compile/lint/test 可能であること(実 OS window の視覚 PASS は
  本件の受入条件に**含めない**)。

## Non-goals(本件では達成しない — 未達と明記)

- AL-G4: Window/View/Dock/Save/Reload/Source revision・
  Cancelled/Stale/Failure/GPU resources/Subscriptions/Job lifetime
  の実 OS 検証。
- AL-G5: 実 GUI visual/semantic regression・独立 oracle・
  Consumer Gate(#219/#294)の正式受入。
- RF #387 Component/View renderer への実 ViewSpec 描画接続
  (別 Worker 実装中。本件は後段統合条件として分離する)。
- Focus/Keyboard/Accessibility の ViewSpec 面(RF #387 依存)。
- Native Surface Slot / 実 desktop 入力(RF #389 担当)。
- Plugin Dock UX・新共通 GUI framework・WASM。
- 旧 WebView/Custom Elements 成果(#105/#131/#132)の削除。

## Constraints

- 新外部依存は追加しない。追加する crate は全て既承認の
  ruviform git 依存(`ruviform-ui-theme`/`ruviform-ui-gpui`/
  `ruviform-ui-test`/`ruviform-media-core`)に限定する。
- GPUI/OS 型は composition root(`bin/al-desktop.rs`)に限定し、
  通常の View/Domain/CLI/headless へ漏らさない。
- 仮造した型・stub・schema や renderer の二重実装を完了扱いしない。
- テスト無効化・warning 抑制・受入緩和をしない。
