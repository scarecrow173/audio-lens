---
id: "TSPEC-bC2OgXciic2j1BpNGCRngr"
type: "test-spec"
status: "approved"
title: "ruviform 87a7ba8f pin 追従の検証仕様"
created: "2026-11-26"
updated: "2026-11-26"
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
    - "docs/specs/ruviform-87a7ba8f-rev-watch.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/ruviform-e4c9f6f0-rev-watch.md"
  derives-from:
    - "docs/specs/ruviform-87a7ba8f-rev-watch.md"
  derived-by: []
  verifies:
    - "docs/specs/ruviform-87a7ba8f-rev-watch.md"
  verified-by: []
---

# ruviform `87a7ba8f` pin 追従の検証仕様

## Feature: pin 一貫性

- Rule: `Cargo.lock` の全 `ruviform-*` git source が単一 resolved SHA
  `87a7ba8ff3fbe6d4502bea27d152d7f74c105899` に一致する。
- Rule: `ui/vendor/ruviform/REV` が lockfile の resolved SHA と一致する
  （`sync-ui-deps` 生成物、commit 対象外）。
- Rule: manifest（ルート/各 crate の `Cargo.toml`）に `rev`/`branch`/`tag`
  を書かない lockfile-primary 契約を維持する。
- Rule: `Cargo.lock` の差分は、25 個の ruviform source fragment と
  upstream 由来の `gpui` registry→fork source switch に限られる。
  無関係な package 追加・version bump があれば原因を分類する。
- 検証: `node tools/update-ruviform-rev.mjs <candidate>`、
  `mise run sync-ui-deps`、`mise exec -- cargo fetch --locked`、
  `mise run check-ruviform-lock`、`mise run check-ruviform-rev`、
  `mise exec -- node tools/check-ruviform-rev.mjs --require-vendor`。

## Feature: 差分評価

- Rule: `e4c9f6f0..candidate` の `git log`/`git diff --stat`/`git diff` を
  read-only clone で実測し、AL lock の ruviform package と照合する。
- Rule: `ruviform-audio` capture push bugfix、`ruviform-math` comment-only
  差分、plugin-core/plugin-host の `Plugin::buses` 契約変更と wire v5→v9、
  `ruviform-ui` Input/CommandParams、`ruviform-ui-gpui` IME/native surface
  変更、`gpui` source switch を個別に採用/見送り/NotApplicable/Deferred
  へ分類する。
- Rule: upstream candidate の covering CI run を確認し、存在しなければ
  NotRun と記録する。直近 ancestor の run を引用する場合は candidate と
  ancestor の間に production code 差分がないことを実測する。

## Feature: 互換性（OS-L1/headless）

- Rule: `mise run verify-toolchain` と `mise run fmt` が合格する。
- Rule: `mise run check al-analysis`、`mise run test al-analysis`、
  `mise run check al-service`、`mise run test al-service`、
  `mise run check al-cli`、`mise run test al-cli` が非回帰。
- Rule: `test al-analysis` で rf628 oracle を含む f64/長尺/PTS/gap/
  Validity の standing oracle が非回帰。
- Rule: `mise run check-cli-headless` で CLI 依存木に GUI/GPU/WebView
  framework crate が入らないこと。
- Rule: `mise run check al-gui`、`mise run lint al-gui`、
  `mise run test al-gui` が非回帰。
- Rule: `mise run check-ui`・`mise run test-ui` が非回帰。
- Rule: `mise run check-ui-gpui`・`mise run lint-ui-gpui`・
  `mise run test-ui-gpui` が非回帰。
- Rule: `mise run build-ui-gpui` で gpui fork 経由の `al-desktop`
  Native Build/Link が合格する。
- Rule: `mise exec -- prek run --all-files` が合格する。

## Feature: scenario contract

- Rule: 既存 `SemanticGuiTest`/`ScenarioReport` scenario
  （desktop.jobs / source.seek / dataset.workspace）が新 pin で同一契約を
  維持する。
- Rule: `mise run test-ui-gpui-real-headless` の blocked-probe が catalog
  全件 `not_run`+`environment_blocked` の fail-closed 形状を維持し、
  `environment_kind` が `real-window` を主張しない。
- Rule: headless/semantic lane の Pass を real window、pixel、OS input、
  IME の Pass へ昇格しない。interactive session の実測がない項目は
  NotRun/Deferred と記録する。

## Feature: 記録

- Rule: 検証証拠にはコマンド・OS・toolchain・exit code・結果を記録する。
- Rule: upstream covering CI、rollback 条件、NotRun/Unsupported/Deferred、
  `Cargo.lock` の gpui source switch、shared file 影響を記録する。
- Rule: docs index（`docs/**/README.md`）は更新しない。index 未同期は
  Coordinator が統合時に整備する残件として記録する。
