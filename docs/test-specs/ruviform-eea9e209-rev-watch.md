---
id: "TSPEC-ydJqlePUkMViYuADBejzZi"
type: "test-spec"
status: "approved"
title: "ruviform eea9e209 pin 追従の検証仕様"
created: "2026-11-28"
updated: "2026-11-28"
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
    - "docs/specs/ruviform-eea9e209-rev-watch.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/ruviform-87a7ba8f-rev-watch.md"
  derives-from:
    - "docs/specs/ruviform-eea9e209-rev-watch.md"
  derived-by: []
  verifies:
    - "docs/specs/ruviform-eea9e209-rev-watch.md"
  verified-by: []
---

# ruviform `eea9e209` pin 追従の検証仕様

## Feature: pin 一貫性

- Rule: `Cargo.lock` の全 `ruviform-*` git source が単一 resolved SHA
  `eea9e2090a23e24dd5cd7a0799b25175c91cad9c` に一致する。
- Rule: `ui/vendor/ruviform/REV` が lockfile の resolved SHA と一致する
  （`sync-ui-deps` 生成物、commit 対象外）。
- Rule: manifest（ルート/各 crate の `Cargo.toml`）に `rev`/`branch`/`tag`
  を書かない lockfile-primary 契約を維持する。
- Rule: `Cargo.lock` の差分は 25 個の ruviform source fragment に限られる。
  無関係な package 追加・version bump があれば原因を分類する。
- 検証: `node tools/update-ruviform-rev.mjs <candidate>`、
  `mise run sync-ui-deps`、`mise exec -- cargo fetch --locked`、
  `mise run check-ruviform-lock`、`mise run check-ruviform-rev`、
  `mise exec -- node tools/check-ruviform-rev.mjs --require-vendor`。

## Feature: 差分評価

- Rule: `87a7ba8f..candidate` の `git log`/`git diff --stat`/
  `git diff --name-status`/必要な `git diff` を read-only clone で実測し、
  AL lock の ruviform package と照合する。
- Rule: `ruviform-media-core` の plan-owned musical transport と `SpanSink`
  signature 変更、`ruviform-audio`/plugin bridge/core の transport handoff、
  `ruviform-audio-test` の test runtime 変更、`ruviform-ui` IPC decision
  core、`ruviform-ui-test` EOF 分類、依存木外の archive/structured/
  audio-device/driver/consumer-verify 差分を個別に採用/保留/rollback/
  NotApplicable へ分類する。
- Rule: upstream candidate の covering CI run を確認し、job 別結果と URL を
  記録する。consumer verify の一方の success を AL の独立 Pass へ転用しない。

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
  `Cargo.lock` 差分、shared file 影響を記録する。
- Rule: docs index（`docs/**/README.md`）は更新しない。index 未同期は
  Coordinator が統合時に整備する残件として記録する。
