---
id: "DISC-TUR7vhR0dFSFD8CjiMNvgM"
type: "discovery"
status: "resolved"
title: "Issue #166 Native 初期 slice (AL-G1〜G3) の現状棚卸しと実装可能範囲"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/166"
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/385"
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "https://github.com/scarecrow173/ruviform/issues/387"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/390"
    - "https://github.com/scarecrow173/synvelis/issues/23"
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
    - "../designs/gui-app-architecture.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/gui-app-mvp.md"
    - "../designs/gui-app-architecture.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/commit/e8adcc09aaa6444b00b9ba62c24fd2d217fb39f0"
  defers: []
  deferred-by: []
---

# Issue #166 Native 初期 slice (AL-G1〜G3) の現状棚卸しと実装可能範囲

## Scope

Issue [#166](https://github.com/scarecrow173/audio-lens/issues/166)
の 2026-11-07 確定仕様 v2 は AL-G1〜G5 の実装単位を定義する。本
discovery は先行可能な **AL-G1〜G3** に限定し、AL-G4(全 lifecycle)
と AL-G5(実 OS/visual/数値/CLI 正式受入)は**未達**とする。
Issue #166 の Close は本件の目的ではない。

## 既存の正本(診断結果)

### AudioLens 側(main `99a7c03`)

- `al-service`: UI 非依存の Job/Dataset サービス層。
  `JobService`(run/resume/export/inspect/list/open_dataset)、
  `DatasetSession`(overview/timeline/events/locate/features/
  artifacts/table_rows/tensor_region/waveform/spectrum)。
- `al-gui`: thin presentation 層。`AlGuiService` が `rvf.ipc.v1`
  op を `al-service` へ橋渡し。`webview` feature で
  `ruviform-ui-webview` host binary `al-gui`。デフォルトは
  headless(lib のみ)。
- 数値正本: `al-core` の `Confidence`/`Validity`/`StageKind`/
  `StageStatus`/`InputRole`、`al-dataset` の `FeatureRecord`/
  `Provenance`/`Range`/`Resolution`/`ResolutionClass`、
  `al-locate` の `LocateResult`/`SampleRange`、
  `al-media` の `StreamTiming`(start_tick・stream_timestamps・
  `TimingObservation`)。
- `al-testkit`: WAV/AVI/MP4 fixture 生成(`wav_s16_bytes`/
  `write_avi`/`write_mp4`/`sine`/`silence`)。

### Ruviform 側(main `e8adcc0`)

- `ruviform-ui::driver`(Issue #386): `WindowDriver` trait、
  `WindowSpec`/`LogicalSize`/`LogicalPosition`、`WindowLifecycle`
  (Created/Ready/Resized/Moved/Focused/ScaleFactorChanged/
  CloseRequested/Destroyed/Action)、`DriverEvent`(Windowed/Tick)、
  `WindowCapabilities`/`Capability`、`UiError::Unsupported`。
- `ruviform-ui::UiRuntime`: `dispatch_window_intent`、
  `drain_jobs_routed`、`handle_window`、
  `cancel_window_jobs`(opt-in)、`has_running_jobs`。
- `ruviform-ui-gpui`(feature `gpui`、default off): `GpuiDriver`
  (gpui 0.2.2)、`IntentSink`。`ime`/`native_surface_slot` は
  OS 別の正直な Supported/Unsupported。
- `ruviform-ui-test`: `FakeDriver` + semantic fixtures
  (inject/pump、close veto/approve/defer/unanswered、bounded
  backlog、capability gating、IPC disconnect)。
- `ruviform-ui-theme`(Issue #385): `TokenDocument`
  (`rvf-dtcg-profile.v1`)、`ThemeResolver::resolve(&[doc], mode)`
  → 不変 `ResolvedTheme`。layer は
  primitive<semantic<component<product。
- `ruviform-viz`: `WaveformSource`/`WaveformSeries`/`ViewBin`/
  `WaveformLod`、`SpectrumSource`/`Spectrogram`/`Tile`/
  `TileRequest`/`RowSpec`、`SampleTimebase`/`FrameGrid`/`Viewport`。
- `ruviform-media-core`: `TimeBase`/`MediaTime`/`MediaRange`/
  `TimingObservation`/`Discontinuity`。

### GPUI/境界の制約

- `tools/check-headless-deps.mjs` は headless crates
  (`al-cli`/`al-service` 等、`al-gui` を除く)への GUI framework
  依存を禁じる。`gpui` crate は現リストに無いが、al-gui 以外へ
  依存を追加しない限り逸脱しない。
- `ruviform-ui-gpui` は default features=[] で headless check を
  維持する設計(ruviform の `check-ui-gpui`/`lint-ui-gpui`/
  `test-ui-gpui` lane と同じ構造を AL 側でも使える)。

## Gap / 未達(明示)

- **ViewSpec 描画接続**: RF #387 Component/View renderer は別
  Worker が実装中で、al-gui が消費できる公開 API は現時点で存在
  しない。実画面の ViewSpec→GPUI 描画は本件の**後段統合条件**と
  する(仮造した ViewSpec 型や renderer の二重実装はしない)。
- **AL-G4**: Window/View/Dock/Save/Reload/Source revision、
  Cancelled/Stale/Failure/Close、GPU resources/Subscriptions/
  Job lifetime の実 OS 検証 → 未達。
- **AL-G5**: 実 GUI visual/semantic regression、独立 oracle、
  Consumer Gate → 未達。
- **実 desktop 入力 slot**: RF #389 担当。触れない。
- IME/focus/keyboard/accessibility の ViewSpec 面も RF #387 依存。

## 先行可能な実装(採用する設計)

1. **AL-G1**: `al-gui` に driver-neutral な `DesktopSession`
   (UiRuntime 配線・RoutedEvent→post_event 配送・close veto・
   last-window exit・cancel-on-destroy policy)と、feature `gpui`
   限定の composition root binary `al-desktop`。FakeDriver で
   headless に lifecycle fixture を検証。
2. **AL-G2**: `al-gui` が所有する product DTCG token
   (base+product 層、light/dark mode)を `ThemeResolver` で
   `ResolvedTheme` へ解決。`GuiPreferences`(theme_mode/window/
   base_dir)の load/save と `WindowSpec` 射影。
3. **AL-G3**: Job/Run/Analysis/Source を driver-neutral view
   model へ写す adapter。既存 typed 契約(`FeatureRecord`/
   `Confidence`/`Validity`/`LocateResult`/`SampleRange`/
   `StreamTiming`/`WaveformSeries`/`Spectrogram`/`SampleTimebase`/
   `FrameGrid`)を再利用し、bounded read(LOD/Seek/window)と
   alignment 由来の TimeMap を提供。GUI 側で解析を再計算しない。

## References

- Issue #166 本文(AL-G1〜G5 定義・GPUI 隠蔽合意・受入)
- RF #384(Driver-neutral 方針・依存方向)、#385(DTCG)、
  #386(Window/Host 契約)、#387(Component/View renderer)、
  #389(Plugin Editor Session)、#390(Native Component Lab)
- `docs/designs/gui-app-architecture.md`(al-service/al-gui 境界)
