---
id: "SPEC-Gq3Lm8Xv2RtN6pWzK4hYcA"
type: "spec"
status: "draft"
title: "Native GUI Spectrogram・Job/Run/Analysis の ViewSpec 射影と drawn parity (Issue #166 第4 slice)"
created: "2026-11-08"
updated: "2026-11-08"
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
    - "native-gui-product-screen-mount.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/native-gui-spectrogram-analysis-projection.md"
    - "../test-specs/native-gui-spectrogram-analysis-projection.md"
    - "https://github.com/scarecrow173/ruviform/issues/434"
    - "https://github.com/scarecrow173/ruviform/issues/435"
    - "https://github.com/scarecrow173/ruviform/issues/436"
    - "https://github.com/scarecrow173/ruviform/issues/442"
    - "https://github.com/scarecrow173/ruviform/issues/443"
  refines: []
  refined-by: []
  derives-from:
    - "native-gui-product-screen-mount.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/native-gui-spectrogram-analysis-projection.md"
  references: []
  defers: []
  deferred-by: []
---

# Native GUI Spectrogram・Job/Run/Analysis の ViewSpec 射影と drawn parity

## What Should Be Built

Issue [#166](https://github.com/scarecrow173/audio-lens/issues/166) の AL-G3 の残課題
(Spectrogram・Job/Run/Analysis の射影と描画 parity)を、第 4 slice として次の範囲で満たす。

- **Spectrogram 射影**: `ruviform-viz` の解決済み `Tile` を `ComponentSpec::Spectrogram` へ
  列ごとに 1 対 1 で写す。値は再計算せず、gap(`count == 0` の列、`NaN` のセル)を保持する。
  表示範囲 `magnitude_range` は consumer が宣言する。
- **Job/Run/Analysis の射影**: `Label`(状態テキスト)と `Table`(stage 表・events 表・
  alignment 表・job 一覧)へ写す。List は一列の Table とする。Panel の title や a11y label
  へ文字列を詰める迂回はしない。
- **al-desktop への mount**: dataset 引数を指定したとき、Source・Spectrogram・Analysis を
  1 つの Tabs 画面として `create_view_window` へ mount する。Spectrogram と Analysis の
  読み出しに失敗しても Source は開き、該当 tab に理由を示す Label を置く。
- **drawn parity**: `GpuiObserver::drawn(window)` が返す Meter bar・TimeAxis mark・Waveform
  bin を、mount した spec の payload から導いた Fake 層の期待値と照合する。
- **Ruviform rev の追従**: `627e4bc2`(#434 `c63108c8`・#435 `627e4bc2`・#436 `f875d305` を含む)
  へ `Cargo.lock` を更新し、`check-ruviform-lock` / `check-ruviform-rev` / `sync-ui-deps` を通す。

## Non-goals(本 slice では扱わない)

- **Job/Run の mount**: al-desktop は job の選択・実行結果の入力源を持たないため、
  Job 一覧と Run の stage 表は射影関数までとし、画面への mount は NotRun。AL-G4 の
  Job 操作(実行・再開・選択)と合わせて扱う。
- **Spectrogram・Label・Table の描画 parity**: `DrawnPlan` に該当 variant が無く観測できない。
  NotRun とし、scarecrow173/ruviform#442 に依存する。
- **Tile の列区間・count の上流契約**: 本 slice は上流に規則が無いため、AudioLens 側で
  `Spectrogram::tile` と同じ等分割を複製する。上流の規則固定は scarecrow173/ruviform#443 に依存する。
- **Spectrogram の theme token**: 色は既存の viz 色を使う。専用 token は follow-up
  (ruviform#434 の F3)。
- **実 OS window での表示確認・platform a11y 接続・AL-G5(性能・visual regression)**: 本 slice の対象外。
- 数値 oracle(f64・NaN・gap・PTS・timebase・Validity・Provenance)と Theme/Source の既存契約は変更しない。
  GPUI 型は `bin/al-desktop.rs` と `ruviform-ui-gpui` の外に出さない。

## Acceptance

- `view::screen::spectrogram_screen` は `Tile` の値と gap を bit 同一で保持し、列 `c` の
  frame 区間は `Spectrogram::tile` の等分割と一致する(割り切れない span を含む)。
- 全 `NaN` の列は `count == 0` になり、その列のセルは `NaN` のまま残る。
- Job/Run/Analysis の Table は、非有限の数値を欠損セル(位置を保つ)として表し、行数が
  `MAX_TABLE_ROWS` を超える場合は拒否する。stage の失敗理由と実行時間を欠落させない。
- `workspace_screen` は Source・Spectrogram・Analysis の 3 tab を持つ Tabs 画面を返し、
  各 tab の画面が Fake 層(`ViewHost`)に mount できる。
- `gpui_parity` の drawn 照合で、Meter bar・TimeAxis mark・Waveform bin が Fake 期待値と一致し、
  gap bin は描かれない。
- `GpuiObserver` 経由の描画 plan の観測は Meter・TimeAxis・Waveform に限られ、Spectrogram・Label・Table は
  NotRun として PR と IR に明記する。
- 既存の数値 oracle が非回帰(`mise run test` 全件 pass)。
