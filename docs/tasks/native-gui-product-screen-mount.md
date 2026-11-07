---
id: "TASK-9dUDqYVPHWLYEx2RNUzEzR"
type: "task"
status: "in-progress"
title: "Issue #166 Source 画面 ViewSpec 射影・al-desktop mount・GPUI parity 実装タスク (第3 slice)"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  changes:
    - "../specs/native-gui-product-screen-mount.md"
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

# Issue #166 Source 画面 ViewSpec 射影・al-desktop mount・GPUI parity 実装タスク

## T1: ruviform rev を c31dc67c へ更新

- [x] `node tools/update-ruviform-rev.mjs c31dc67c1d0bed86542c6442843e91c5fafca2dd`
- [x] `mise run check-ruviform-lock` / `check-ruviform-rev` / `sync-ui-deps`

## T2: TimeAxis/Meter の射影

- [x] Fake 層テストを先に追加(Meter の peak 写像・gap・ruler の件数と viewport 内)
- [x] `view/screen.rs` の `source_screen` に `ruler`(TimeAxis)と `meter` を追加
- [x] 丸めで viewport 外に出る tick を frame 空間の最終判定で落とす(テストで検出して修正)

## T3: al-desktop の mount

- [x] `create_window`(fixture)を `create_view_window` + Source/空画面へ置換
- [x] 引数 `<dataset-root> <input-id>` の解析と usage 表示
- [ ] 実 OS window での起動・表示確認(NotRun: この作業環境では窓を開いて確認していない)

## T4: GPUI parity

- [x] `gpui-test` feature を追加し、`GpuiObserver` で mount する consumer test を追加
- [x] Fake/GPUI の共有 fixture を `tests/common` へ集約
- [ ] Meter/TimeAxis の描画 plan の parity(NotRun: `GpuiObserver` に描画 plan の公開が無い)

## T5: 文書と mise task

- [x] spec / design / plan / test-spec / tasks の chain を作成
- [x] `check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui` を `gpui-test` feature へ切替
- [x] impl IR(実装記録)と各 README の索引の最終更新

## T6: 未達項目と上流依頼

- [ ] Job/Run/Analysis の ViewSpec 化(RF の text/table kind 不在、未達 → RF#435)
- [ ] Spectrogram の射影(RF の Spectrogram kind 不在、未達 → RF#434)
- [x] Ruviform へ契約ギャップの Issue 起票と本 Issue への相互参照(RF#434・#435・#436)

## T7: 検証

- [x] `mise run fmt` / `lint-ui-gpui` / `test-ui-gpui`(Fake 12 件 + GPUI parity 3 件)
- [ ] `mise run fmt` / `lint` / `test` / `build` / `check-ui-gpui` / `check-cli-headless` / `check-ruviform-*` / `sync-ui-deps` / `check-ui` / `test-ui`(merge 後 HEAD で最終実施)
- [ ] review-protocol(独立レビュー)の実施と blocker 解消
