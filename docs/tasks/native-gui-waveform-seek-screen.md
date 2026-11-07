---
id: "TASK-NWrMC4dBFWhLLr6YuXhEPF"
type: "task"
status: "in-progress"
title: "Issue #166 Waveform/Seek 画面 ViewSpec 射影 実装タスク"
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
  implements:
    - "../plans/native-gui-waveform-seek-screen.md"
  implemented-by:
    - "../impl/ir/native-gui-waveform-seek-screen.md"
  depends-on:
    - "../plans/native-gui-waveform-seek-screen.md"
    - "../test-specs/native-gui-waveform-seek-screen.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/native-gui-waveform-seek-screen.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #166 Waveform/Seek 画面 ViewSpec 射影 実装タスク

## T1: ruviform rev を a57cd3d3 へ更新

- [x] `node tools/update-ruviform-rev.mjs a57cd3d3bdae2b3dfde02bf4b496132bae7f3235`
- [x] `mise run check-ruviform-lock` / `check-ruviform-rev` / `sync-ui-deps`

## T2: frame_range の抽出

- [x] `WaveformView::frame_range` を追加し `window` から使う
- [x] 既存 `view` テストが通ること

## T3: 射影の実装

- [x] `view/screen.rs` に `waveform_seek` を実装(ViewBin 1 対 1 写像)
- [x] Seek control(`transport.seek`)と Panel 構成

## T4: Fake 層評価

- [x] `tests/screen.rs` の 6 件(bit 同一・gap・範囲・intent・theme 中立・境界)

## T5: 検証

- [x] `mise run fmt` / `lint` / `test`
- [x] `mise run check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui`(GPUI 型が al-gui の composition root 外に出ていないことを rg で確認)
- [ ] GPUI driver 上の parity 評価(上流 #421/#426 待ち、NotRun)
