---
id: "TSPEC-034Z9kQmDatasetRevWatch04"
type: "test-spec"
status: "approved"
title: "ruviform b6d0cc6a pin 追従の検証仕様"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/ruviform-b6d0cc6a-rev-watch.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/ruviform-cbcc8343-rev-watch.md"
---

# ruviform b6d0cc6a pin 追従の検証仕様

## Feature: pin 一貫性

- Rule: `Cargo.lock` の全 `ruviform-*` git source が単一 resolved SHA
  `b6d0cc6a2e14f47e5324061d8dcac3d905e4abd5` に一致する。
- Rule: `ui/vendor/ruviform/REV` が lockfile の resolved SHA と一致する
  （`sync-ui-deps` 生成物、commit 対象外）。
- 検証: `mise run check-ruviform-lock`、`mise run check-ruviform-rev`、
  `mise run check-ui`。

## Feature: 互換性（OS-L1）

- Rule: 既定 feature の `check`/`lint`/`test al-gui` が非回帰。
- Rule: `gpui-test`/`gpui-real` 両 feature の check/lint/test が非回帰
  （feature 排他契約を維持）。
- Rule: `check-cli-headless` で CLI 依存木の GUI/GPU 非含有を維持。
- Rule: `test al-analysis` で standing oracle 回帰。

## Feature: scenario contract

- Rule: 既存 `SemanticGuiTest` scenario（desktop.jobs / source.seek）が
  新 pin で同一契約を維持する。
- Rule: 新 dataset workspace scenario が Fake/GpuiSimulated 両 adapter で
  同一 intent 列・selected・mounted・typed-cell oracle を満たす。
- Rule: `al-gui-real-scenario --blocked-probe` が catalog 全件
  `NotRun`+`environment_blocked` の fail-closed 形状を維持する
  （`environment_kind` が `real-window` を主張しない）。

## Feature: window-close/DPI 観測面

- Rule: `window_geometry`/`window_events`/`request_close`/
  `arm_close_response`/`respond_close` が `RealOsGuiTest` で利用可能。
- Rule: catalog 追加 case は前提不成立時 `NotRun`/`Unsupported` に写し、
  Fail にしない。実機 Pass は interactive session のみ。
