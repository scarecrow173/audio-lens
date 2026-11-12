---
id: "IMPL-avXvztBqYhq2RTsytk92JZ"
type: "impl"
status: "completed"
title: "Native GUI Source 画面 ViewSpec 射影・al-desktop mount・GPUI parity 実装記録(IR)"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/native-gui-product-screen-mount.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../specs/native-gui-product-screen-mount.md"
  refines: []
  refined-by: []
  derives-from:
    - "../../tasks/native-gui-product-screen-mount.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Native GUI Source 画面 ViewSpec 射影・al-desktop mount・GPUI parity 実装記録(IR)

## Summary

Native GUI Source 画面の ViewSpec 射影・al-desktop mount・
GPUI parity の実装記録。`view::screen::source_screen` で
Waveform/Meter/TimeAxis を射影し、fixture window を
`create_view_window` へ置換、Fake(`ViewHost`)と GPUI
(`GpuiObserver`)の parity test を追加した。

## Context

Issue #166(`relations.source`)の Native GUI Source 画面
slice。spec `docs/specs/native-gui-product-screen-mount.md`
(implements)と task
`docs/tasks/native-gui-product-screen-mount.md`(derives-from)
の chain に従う。ruviform rev を `c31dc67c` へ更新し、
`create_view_window`/`ViewMount`/`GpuiObserver` の上流契約を
利用する。

## Implementation

- **Ruviform rev 更新**: `node tools/update-ruviform-rev.mjs c31dc67c…` で `Cargo.lock` の
  ruviform 20 package を `a57cd3d3` から `c31dc67c` へ(`Cargo.lock` の差分は source SHA のみ)。
- **Source 画面の射影**(`view::screen::source_screen`、旧 `waveform_seek` を改名):
  - Waveform / Meter は同じ `window` 結果を一度だけ読んで写す。
  - Meter の `peak` は `max(|min|,|max|)`、gap(`count == 0`)は `NaN` のまま。
  - TimeAxis(`ruler`)は viewport の frame 範囲から刻みを決め、区間数 10 以下・tick は
    viewport 内に限る。刻みは 1 ms から 1 日まで(最大 10 日の span)。
  - 画面の bin 上限は `MAX_METER_BINS`(8192)。
- **al-desktop**: fixture window(`create_window`)を、Source 画面を載せた
  `create_view_window` へ置換。引数 `<dataset-root> <input-id>`、省略時は空の Panel。
  `ViewHost` の epoch は `AttachEpochAllocator` から mint。theme は `DesktopSession` の
  `ResolvedTheme` を `ViewMount` へ渡す。
- **GPUI parity**: feature `gpui-test`(`ruviform-ui-gpui/gpui-test`)を追加し、
  `tests/gpui_parity.rs` で同一 fixture(`tests/common`)を Fake(`ViewHost`)と GPUI
  (`GpuiObserver`)で評価。mount key・各 component の projection(mounted/phase/
  display/committed、f64 は bit 比較)・Seek gesture の command intent JSON を比較。
- **mise task**: `check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui` を `gpui-test` feature へ切替。
- **文書 chain**: spec / design / plan / test-spec / tasks と、各 README の索引。

## Related Experiments

### Adopted

- なし — `docs/impl/exp/` に本 slice に対応する実験ログは
  存在しない(既存ログは CI 系 issue-139〜153 のみ)。
  `metadata.experiments` も空。

### Rejected

- なし(同上)。

## Validation

### 検証結果(この作業ツリーで実行)

| コマンド | 結果 |
| --- | --- |
| `mise run fmt` | pass |
| `mise run check` | pass |
| `mise run lint`(biome + clippy `-D warnings`) | pass(biome の info 表示のみ) |
| `mise run test`(workspace) | pass(Rust 668 件、failed 0、ignored 2 は既存の `al-locate` の `#[ignore]`) |
| `mise run build` | pass |
| `mise run check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui` | pass(Fake 層 `screen` 12 件・GPUI parity 3 件・unit 2 件を含む gpui-test 全体) |
| `mise run check-cli-headless` | pass |
| `mise run check-ruviform-lock` / `check-ruviform-rev` / `sync-ui-deps` | pass(`c31dc67c`, 20 packages) |
| `mise run check-ui` / `test-ui` | pass(`npm --prefix ui run typecheck` / `npm --prefix ui test` 112 件) |

- 検証の数値 oracle(Waveform bit 同一、gap の NaN・count 0、light/dark の semantic 同一)は既存 test が非回帰で通る。

### 独立レビュー(review-protocol)の結果

- 独立した新規コンテキストのサブエージェントを read-only で起動し、diff を検査。
- 指摘: P2 長尺 source(約 100 分超)で ruler が Err になり `al-desktop` が終了する問題 →
  刻みの候補を 10 日まで拡張し、長尺・上限超の unit test を追加して修正。
  P3 の doc 不一致(`MAX_WAVEFORM_BINS` → `MAX_METER_BINS`)を修正。
  P3 の部分 NaN の peak を、gap 判定を `count == 0` で明示する形に修正。
- 最終判定: blocker なし。

## Risks

- Job/Run/Analysis の ViewSpec 化は Ruviform ViewSpec に
  text / table 系の kind が無いため未達 — Panel title への
  詰め込みは契約の迂回になるため不採用し、上流 contract の
  追加(scarecrow173/ruviform#435)待ち。AL 側だけでは解消
  できない。
- Spectrogram の射影は ViewSpec に Spectrogram kind が無く
  未達(scarecrow173/ruviform#434)。
- Meter/TimeAxis の描画 parity は `GpuiObserver` が描画 plan を
  公開していない(scarecrow173/ruviform#436)ため検証不能 —
  component の状態 parity のみ検証済み。
- al-desktop の実 OS window 起動は NotRun — この作業環境では
  窓を開いて表示確認していない。

## Follow-ups

- **Job/Run/Analysis の ViewSpec 化は未達**: Ruviform の ViewSpec に text / table 系の kind が無い。
  Panel title への詰め込みは契約の迂回になるため採らず、scarecrow173/ruviform#435 の解消後に射影する。
- **Spectrogram の射影は未達**: ViewSpec に Spectrogram kind が無い(scarecrow173/ruviform#434)。
- **Meter/TimeAxis の描画 parity は NotRun**: `GpuiObserver` が描画 plan を公開していない
  (scarecrow173/ruviform#436)。component の状態 parity のみ検証。
- **al-desktop の実 OS window 起動は NotRun**: この作業環境では窓を開いて表示確認していない。
- **AL-G4 / AL-G5 は対象外**: Save/Reload・Source revision・Cancel/Close の実機検証、visual
  regression、性能、Consumer Gate は別 slice。
- ブランチは `scarecrow173/al166-screen-mount` が前 slice で使用済みのため、`-s3` を付けて新規に切った。
