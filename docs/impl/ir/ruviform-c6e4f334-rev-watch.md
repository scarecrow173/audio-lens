---
id: "IMPL-034XCW3SvHJVdWjyHNmcwb"
type: "impl"
status: "completed"
title: "ruviform c6e4f334 pin 追従評価の実施記録"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "code"
        path: "docs/specs/ruviform-c6e4f334-rev-watch.md"
      - type: "code"
        path: "docs/designs/ruviform-c6e4f334-rev-watch.md"
      - type: "code"
        path: "docs/tasks/ruviform-c6e4f334-rev-watch.md"
      - type: "code"
        path: "docs/test-specs/ruviform-c6e4f334-rev-watch.md"
      - type: "code"
        path: "docs/impl/ir/ruviform-c6e4f334-rev-watch.md"
    modified:
      - type: "code"
        path: "Cargo.lock"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "code"
        path: "ui/vendor/ruviform"
  implements:
    - "docs/tasks/ruviform-c6e4f334-rev-watch.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/ruviform-fd1d4d5f-rev-watch.md"
    - "docs/impl/ir/ruviform-b02942c2-rev-watch.md"
    - "docs/impl/ir/ruviform-pin-refresh-073a3ff-consumer-regression-issue-127.md"
    - "docs/impl/ir/ruviform-rev-bump-consumer-regression-verification-issue-127.md"
  refines: []
  refined-by: []
  derives-from: []
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

# ruviform c6e4f334 pin 追従評価の実施記録

## Summary

ruviform pin を `fd1d4d5f4de57092b96ad1cd309b85fe47cbef38` から
`c6e4f334242567dbd0e0d8b7713d9d480a6aab66`（ruviform main HEAD）
へ lockfile-primary 手順で追従し、対象 revision で正規 mise 検証
lane を全実行した。`Cargo.lock` の差分は ruviform 20 package の
SHA fragment 更新 20 行のみで、推移的依存の変動・新規 package
追加はなかった。`ui/vendor/ruviform` は `sync-ui-deps` で再生成
（`ui/*` TS source に差分がないため REV 以外不変）。製品コード
変更はなく、全 lane Pass を確認。upstream 新 API/feature
（`ruviform-audio-test`・`uitest` windowed TestIo・`fault-injection`・
CLAP editor-attach・ui-test Window/Job 拡張）は採用しない判断を
記録。

## Context

Issue #127（Ruviform 更新の常設追跡 Issue）の rev-watch 定型
slice。前段は PR #194（pin `fd1d4d5f` 追従評価）。本 slice の
対象範囲は pin `fd1d4d5f` → `c6e4f334`（114 commit・892 file、
`crates/` 配下は 75 file・+9,161/−475 行）。

## Implementation

### Changes

- `Cargo.lock`: `tools/update-ruviform-rev.mjs
  c6e4f334242567dbd0e0d8b7713d9d480a6aab66` で ruviform 20
  package の `source` fragment を一括更新。差分は 20 行の SHA
  fragment のみ（他 crate の version 変動・新規 package 追加・
  推移的依存の変動なし）。
- `ui/vendor/ruviform`: `mise run sync-ui-deps` で lockfile SHA
  から再生成。`REV` = `c6e4f334242567dbd0e0d8b7713d9d480a6aab66`。
  ruviform 差分に `ui/*`（core/components/binding/viz）の変更が
  ないため vendor 内容は REV 以外前 pin と同一。
- docs: spec→design→test-spec→task→IR の文書チェーンを
  `ruviform-c6e4f334-rev-watch` 名で新規作成し、生成 index
  （docs/{specs,designs,tasks,test-specs}/README.md）を再生成、
  手動 index（docs/impl/ir/README.md）に追記。本 slice の文書
  セットは lease どおり 5 点で plans 文書は含めない。

### Decisions During Implementation

- pin 更新は完全 SHA 指定（短縮 SHA の誤展開を避ける前段の教訓に
  従う）。`git ls-remote` で refs/heads/main =
  `c6e4f334242567dbd0e0d8b7713d9d480a6aab66` を確認し、local
  clone `G:/repositry/audio-products/ruviform` の `origin/main`
  HEAD と一致を確認して確定。
- `fd1d4d5f..c6e4f334` の影響評価: AL の依存木（lockfile の
  20 package）に変更があるのは `ruviform-ui-gpui`（`uitest`
  feature 新設: windowed `TestIo`/`UiSide`/`WindowedIo` export・
  `windowed_test_app` bin・`windowed_io.rs`。`UiJob` cfg を
  `any(real-scenario, uitest)` へ拡張。capture/surface は comment、
  render は test cfg のみ。RF#313/#572）、`ruviform-ui-test`
  （driver/headless/io/journal/fixture の内部再構成と Window/Job
  invariant test 追加。公開 export は不変。RF#243/#625/#388）、
  `ruviform-io-file`（opt-in `fault-injection` feature で `mock`
  module を `#[doc(hidden)] pub` 化。RF#306）、`ruviform-ui-webview`
  （legacy/preview 分類の doc 追記のみ。RF#465）、
  `ruviform-observability`（test のみの timing 修正）の 5 crate。
  依存木外では新規 `ruviform-audio-test`（RF#628/#638）、
  `ruviform-audio-device`/`ruviform-audio`/`ruviform-io-e2e`/
  `ruviform-midi-device`/`ruviform-plugin-*`/`ruviform-poc-native-dock`/
  `ruviform-video-proc` が変更。破壊的変更なし・adapter 修正不要。
- 新 API/feature の採用判断: `ruviform-audio-test`・`uitest`
  windowed TestIo・`real-scenario` OS input ops・`fault-injection`・
  CLAP editor-attach・ui-test Window/Job 拡張はすべて opt-in/別
  ドメインであり AL 側の採用 gate・需要が未定義のため本 slice
  では採用しない。
- Issue 記載の `check-ci`/`lint-ci`/`test-ci`/`fmt-check`/
  `verify-ci` task 名は現行 mise.toml に存在しないため、
  `check-full`/`lint-full`/`test-full`/`build-full`（workspace
  全体）と個別 feature lane で代替実行した。
- 本 worktree は `ui/node_modules`・`ui/vendor` 未復元のため、
  UI lane の前に `mise run setup-ui`（`npm ci --prefix ui`）を
  実行し、`mise run sync-ui-deps` で vendor を再生成した。
- doc graph route は main 既存の broken relation
  （`crates/al-math` 参照切れ）で blocked のため、前段 PR #190/
  #193/#194 と同じく文書チェーンを直接作成し review-protocol で
  承認相当のレビューを行う運用とした。

## Related Experiments

### Adopted

<!-- なし -->

### Rejected

<!-- なし -->

## Validation

- `mise run check-ruviform-lock`: Pass（20 package、単一 SHA
  `c6e4f33424...`）
- `mise run check-ruviform-rev`: Pass（REV = lockfile SHA、
  lockfile-primary、fail-closed 構造維持）
- `mise run sync-ui-deps` → `check-ui` → `test-ui`: Pass
  （typecheck + node:test 113 件 fail 0）
- `mise run fmt` / `check-full` / `lint-full` / `test-full` /
  `build-full`: Pass（workspace、test 58 suite fail 0）
- `mise run check-cli-headless`: Pass（headless 10 crate に GUI
  framework 依存なし）
- `mise run check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui` /
  `build-ui-gpui`: Pass（`gpui`/`gpui-test` feature lane。
  `ruviform-ui-gpui@c6e4f334` が compile/lint/test/build を通過）
- `mise run check-ui-webview` / `lint-ui-webview` /
  `test-ui-webview`: Pass（Legacy WebView lane 維持）
- `mise run check-ci-workflow` / `check-ci-tracker-workflows` /
  `verify-toolchain`: Pass
- `mise exec -- cargo fetch --locked`: Pass
- NotRun/Unsupported: 実 OS GUI・実 Pixel・macOS/Linux・
  cross-consumer・`gui-real`・ruviform `test-ui-real-scenario`・
  windowed `uitest` lane（実 desktop session 前提の manual lane）。
  詳細は `docs/test-specs/ruviform-c6e4f334-rev-watch.md` を正本と
  する。

## Risks

- upstream の新機能領域（deterministic test runtime・windowed
  TestIo・fault-injection・CLAP editor・plugin ParentLease 移行・
  WASAPI input・plugin report・MIDI bridge）は AL lane 未踏。
  candidate tuple としての継続評価で担保し、Issue #127 は open の
  まま。
- `proc-macro-error2 v2.0.1` の future-incompatibility warning
  （依存由来、記録のみ）。

## Follow-ups

- `ruviform-audio-test`・`uitest`/`real-scenario`・
  `fault-injection`・CLAP editor-attach・ui-test Window/Job 拡張の
  AL 採用判断は別 slice の Issue で行う（本 slice では採用しない）。
- 実機 lane（実 OS GUI・実 Pixel・macOS/Linux）の整備は既存の
  NotRun 追跡に従う。
