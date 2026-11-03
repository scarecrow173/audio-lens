---
id: "TASK-deJYGXM5AEKotdnNNLUHMn"
type: "task"
status: "done"
title: "Ruviform Controls・Gesture・Binding の GUI 統合(AL#131)"
created: "2026-11-03"
updated: "2026-11-03"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/131"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/ruviform-foundation-migration.md"
  implemented-by:
    - "docs/impl/ir/gui-ruviform-controls-issue-131.md"
  depends-on:
    - "docs/tasks/ruviform-gui-app.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
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

# Ruviform Controls・Gesture・Binding の GUI 統合(AL#131)

## Work

Ruviform UI lane(ui/core headless 契約、ui/components Web
Controls + reference skin/theme、ui/binding Command/Query +
receipts + multiwindow)を AudioLens GUI へ導入し、
headless 検証可能な統合層を構築する。Issue
[#131](https://github.com/scarecrow173/audio-lens/issues/131) を正本とする。

- 共有 Seek/Range/Numeric/Slider コントロールをそのまま利用し、
  gesture/value ロジックを AudioLens 側で fork しない
- AudioLens 製品テーマ(操作ロジックと独立に切替可能、
  Reference Skin も theme なしで動作)
- 画面構成 fixture: Job settings / media preview /
  seek-range 選択 / analysis settings
- 明示的な Source/Run revision/Range/Job identity を
  headless Query/Command 経路へ伝達
- Preview/Commit/Cancel gesture と Job run/resume/cancel の分離
- 表示丸めと `f64` dataset 精度の厳格分離
- missing source/stale revision/job failure/partial/unsupported/
  delayed/reordered/cancel の正直な状態表現
- `al-cli` とサービスを共有し、GUI/WebView 依存を解析
  core/CLI へ持ち込まない
- 実 desktop GUI shell(#105)が無い環境では headless 統合層 +
  テストを成果物とし、実 GUI/ウィンドウ検証は未検証として記録

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/tasks/ruviform-gui-app.md`
- `docs/specs/gui-app-mvp.md`
- Issue [#131](https://github.com/scarecrow173/audio-lens/issues/131)

## Dependencies

- `docs/tasks/ruviform-gui-app.md`(al-service / al-gui op 語彙)
- 外部依存: Ruviform ui/core(#258)、ui/components(#259)、
  ui/viz(#260)、ui/binding(#261)、ui/showcase(#263) —
  ruviform main `e9e6abe` でマージ済み

## Affected Files

- `Cargo.toml` / `Cargo.lock`(ruviform pin bump)
- `tools/sync-ruviform-ui.mjs`(vendored UI source 同期)
- `.gitignore` / `.worktreeinclude`(vendor tree の除外/引継ぎ)
- `ui/`(新規 TypeScript パッケージ:
  gateway/theme/port/view-state/screen fixtures/tests)
- `mise.toml`(`sync-ui-deps`/`setup-ui`/`check-ui`/`test-ui`)

## Done When

- [x] 共有 Seek/Range/Numeric/Slider を GUI で利用し、製品
      テーマを操作ロジック fork なしに切替可能
- [x] 明示的 Source/Run/Range 指定と Gesture 終端が headless
      service へ正確に伝達
- [x] GUI 操作で既存 Analysis Dataset を表示用途だけで
      再解析しない
- [x] delayed/reordered/failed/cancel/unavailable の応答で
      確定値や解析完了を偽装しない
- [x] headless 検証 task(`mise run check-ui`/`test-ui`)が green
- [ ] 実 GUI/ウィンドウ環境での検証(実環境未整備のため
      未検証として記録)

## Verification

- [x] `mise run check-ui`(vendor pin check + typecheck)
- [x] `mise run test-ui`(69 tests / 23 suites)
- [x] `mise run lint`(biome ui/ + tools/、cargo clippy)
- [x] `mise run check` / `mise run test`(pin bump 後の既存
      回帰、Rust 側)
- [ ] 実ウィンドウ/2-window/focus/keyboard/high-contrast/touch
      (実 GUI 環境なし、未検証)
