---
id: "SPEC-srSXzJOEfwrrPbP7Q35wbN"
type: "spec"
status: "draft"
title: "Native GUI Source 画面の ViewSpec 射影・al-desktop mount・GPUI parity (Issue #166 第3 slice)"
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
    - "native-gui-waveform-seek-screen.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/native-gui-product-screen-mount.md"
    - "../test-specs/native-gui-product-screen-mount.md"
    - "https://github.com/scarecrow173/ruviform/issues/421"
    - "https://github.com/scarecrow173/ruviform/issues/429"
    - "https://github.com/scarecrow173/ruviform/issues/434"
    - "https://github.com/scarecrow173/ruviform/issues/435"
    - "https://github.com/scarecrow173/ruviform/issues/436"
  refines: []
  refined-by: []
  derives-from:
    - "native-gui-waveform-seek-screen.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/native-gui-product-screen-mount.md"
  references: []
  defers: []
  deferred-by: []
---

# Native GUI Source 画面の ViewSpec 射影・al-desktop mount・GPUI parity

## What Should Be Built

Issue [#166](https://github.com/scarecrow173/audio-lens/issues/166) の
AL-G3(Job/Run/Analysis/Source の GPUI 投影・TimeAxis/Meter/Spectrogram)
と AL-G1(composition root)の第 3 slice として、次を満たす。

- **Source 画面の射影を拡張**: 第 2 slice の Waveform/Seek 画面に、TimeAxis
  (ruler)と Meter を同じ解決済み bin から 1 対 1 で射影する。統計・count・
  NaN gap は再計算しない。Meter の `peak` は bin の `min`/`max` から
  `max(|min|, |max|)` で導く唯一の写像とし、他の値は写さない。
- **al-desktop への mount**: `al-desktop` の fixture window(view を持たない
  `create_window`)を、Source 画面の `ViewSpec` を載せた view window
  (`create_view_window`)へ置き換える。Source は任意引数
  `<dataset-root> <input-id>` で指定し、省略時は Source を載せない空の画面。
- **GPUI parity の観測**: `ruviform-ui-gpui` の consumer 観測 seam
  (`GpuiObserver`, Ruviform #429)を使い、同一 `ViewSpec` を Fake 層
  (`ViewHost` 直接)と GPUI Driver(mount 済み view)で評価して、
  mount 済み key・各 component の projection(mounted/phase/display/committed)・
  Seek gesture の command intent が一致することを検証する。

## Non-goals(本 slice では扱わない)

- **Job/Run/Analysis の ViewSpec 化**: 現行 Ruviform ViewSpec は Panel/Tabs/
  Fader/Seek/Knob/Toggle/Waveform/Meter/TimeAxis/NativeSlot のみで、テキスト・
  表・リストを表す kind が無い。Job/Run/Analysis の表示は射影できないため
  未達とし、Ruviform 側へ契約の拡張を依頼する(Ruviform Issue 参照)。
- **Spectrogram の射影**: 現行 ViewSpec に Spectrogram kind が無いため未達。
  同じく Ruviform 側の契約拡張を依頼する。
- **AL-G4**(実 OS window・Save/Reload・Source revision・Cancel/Close の実機検証)
  と **AL-G5**(visual regression・性能・Consumer Gate)は本 slice の対象外。
  実 OS での起動は CI で行わない。
- **Meter/TimeAxis の描画 plan の parity**: `GpuiObserver` は component の
  状態は公開するが描画 plan は公開しないため、描画の一致は NotRun。
- Theme/Source/数値契約(f64/NaN/PTS/Validity/Provenance)は変更しない。

## Acceptance

- Source 画面 `ViewSpec` に Waveform・Seek・TimeAxis・Meter が含まれ、
  Waveform の bin と Meter の bin が解決済み `ViewBin` と bit 同一(Meter の
  `peak` は `max(|min|,|max|)` の bit 同一)である。
- gap bin は `NaN`・`count == 0` のまま。TimeAxis の tick は viewport 内で、
  件数は `MAX_TIME_AXIS_TICKS` 以下。
- `al-desktop` は引数の有無に応じて Source 画面または空画面を
  `create_view_window` で mount する。GPUI 型は `bin/al-desktop.rs` と
  `ruviform-ui-gpui` にのみ現れる。
- `GpuiObserver` を使う consumer test で、Fake 層と GPUI 層の mount key・
  projection・Seek intent が一致する(同一 fixture)。
- 既存の数値 oracle(Waveform bit 同一・gap・light/dark の semantic 同一)が非回帰。
