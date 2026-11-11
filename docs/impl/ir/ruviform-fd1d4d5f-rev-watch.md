---
id: "IMPL-034VFTb0yP663bAtG9lOHq"
type: "impl"
status: "completed"
title: "ruviform fd1d4d5f pin 追従評価の実施記録"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "code"
        path: "docs/specs/ruviform-fd1d4d5f-rev-watch.md"
      - type: "code"
        path: "docs/designs/ruviform-fd1d4d5f-rev-watch.md"
      - type: "code"
        path: "docs/plans/ruviform-fd1d4d5f-rev-watch.md"
      - type: "code"
        path: "docs/tasks/ruviform-fd1d4d5f-rev-watch.md"
      - type: "code"
        path: "docs/test-specs/ruviform-fd1d4d5f-rev-watch.md"
      - type: "code"
        path: "docs/impl/ir/ruviform-fd1d4d5f-rev-watch.md"
    modified:
      - type: "code"
        path: "Cargo.lock"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
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
    - "docs/tasks/ruviform-fd1d4d5f-rev-watch.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
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

# ruviform fd1d4d5f pin 追従評価の実施記録

## Summary

ruviform pin を `b02942c2f9e6e195d0d752803b077866468fe58e` から
`fd1d4d5f4de57092b96ad1cd309b85fe47cbef38`（ruviform main HEAD）
へ lockfile-primary 手順で追従し、対象 revision で正規 mise 検証
lane を全実行した。`Cargo.lock` の差分は ruviform 20 package の
SHA fragment 更新＋`ruviform-ui-gpui` の依存への `x11rb` 追加
1 行（linux-target optional dep、既存 lockfile 内 package の
参照追加のみで新規 package なし）に留まり、`ui/vendor/ruviform`
は `sync-ui-deps` で再生成（`ui/*` TS source に差分がないため
REV 以外不変）。製品コード変更はなく、全 lane Pass を確認。
upstream 新 API（`real-scenario` 配下の OS input ops・
`OsInputOutcome`・SendInput preflight session cache・Linux
offscreen golden・L1 wire report verb・WASAPI-native input・
bridge unsupported 透過）は採用しない判断を記録。

## Context

Issue #127（Ruviform 更新の常設追跡 Issue）の rev-watch 定型
slice。前段は PR #193（pin `b02942c2` 追従評価）。本 slice の
対象範囲は pin `b02942c2` → `fd1d4d5f`（7 commit: ruviform PR
#564/#567/#566/#568/#569/#570/#571）。

## Implementation

### Changes

- `Cargo.lock`: `tools/update-ruviform-rev.mjs
  fd1d4d5f4de57092b96ad1cd309b85fe47cbef38` で ruviform 20
  package の `source` fragment を一括更新。差分は 20 行の SHA
  fragment＋`ruviform-ui-gpui` 依存への `x11rb` 追加 1 行のみ
  （他 crate の version 変動・新規 package 追加なし。`x11rb` は
  `gpui`/`winit` 由来で既に lockfile に存在）。
- `ui/vendor/ruviform`: `mise run sync-ui-deps` で lockfile SHA
  から再生成。`REV` = `fd1d4d5f4de57092b96ad1cd309b85fe47cbef38`。
  ruviform 差分に `ui/*`（core/components/binding/viz）の変更が
  ないため vendor 内容は REV 以外前 pin と同一。
- docs: spec→design→plan→task→test-spec→IR の文書チェーンを
  `ruviform-fd1d4d5f-rev-watch` 名で新規作成し、生成 index
  （docs/{specs,designs,plans,tasks,test-specs}/README.md）を再生成、
  手動 index（docs/impl/ir/README.md）に追記。

### Decisions During Implementation

- pin 更新は完全 SHA 指定（短縮 SHA の誤展開を避ける前段の教訓に
  従う）。`git ls-remote` で refs/heads/main・HEAD =
  `fd1d4d5f4de57092b96ad1cd309b85fe47cbef38` を確認し、local
  clone `F:/repositry/audio-products/ruviform` の `origin/main`
  HEAD と一致を確認して確定（fd1d4d5f 以降の新 commit なし）。
- `b02942c2..fd1d4d5f` の影響評価: AL の依存木に変更があるのは
  `ruviform-ui-gpui`（`capture.rs` OS-window pixel capture の
  Windows `PrintWindow`/`BitBlt` + Linux X11 backend・
  `os_input.rs` の `OsInputOutcome` additive・`real_window.rs`
  の OS input ops は `real-scenario` feature 配下・`surface.rs`
  `pub(super)` helper・`Cargo.toml` に linux-target optional dep
  `x11rb` 追加。RF#566/#569）と `ruviform-audio-core`
  （`midi_input` mock の `inject_unsupported` additive。AL は
  MIDI 非使用）。`ruviform-ui-test` は本範囲で変更なし。
  `ruviform-audio-device`（WASAPI-native input frame_position、
  RF#565/#251）/`ruviform-plugin-*`（L1 wire report verb、
  RF#253）/`ruviform-audio-plugin-bridge`（unsupported 計数
  透過、RF#253 residual）/`ruviform-midi-device`（test のみ）
  は AL 依存木外。破壊的変更なし・adapter 修正不要。
- 新 API の採用判断: `real-scenario` 配下の OS input ops
  （RealOsPointerOrKeyboard）・`OsInputOutcome`・SendInput
  preflight session cache・Linux offscreen golden・L1 wire
  report verb・WASAPI-native input・bridge unsupported 透過は
  いずれも opt-in/別ドメインであり AL 側の採用 gate・需要が
  未定義のため本 slice では採用しない。
- spec 指定の `fmt-check` task は mise.toml に存在しない
  （`fmt` task 自体が `cargo fmt --all -- --check`）。
  スキップして記録した。
- 本 worktree は `ui/node_modules` 未復元のため、UI lane の前に
  `mise run setup-ui`（`npm ci --prefix ui`）を実行した。
- doc graph route は main 既存の broken relation
  （`crates/al-math` 参照切れ）で blocked のため、前段 PR #190/
  #193 と同じく文書チェーンを直接作成し review-protocol で
  承認相当のレビューを行う運用とした。

## Related Experiments

### Adopted

<!-- なし -->

### Rejected

<!-- なし -->

## Validation

- `mise run check-ruviform-lock`: Pass（20 package、単一 SHA
  `fd1d4d5f4d...`）
- `mise run check-ruviform-rev`: Pass（REV = lockfile SHA、
  lockfile-primary、fail-closed 構造維持）
- `mise run sync-ui-deps` → `check-ui` → `test-ui`: Pass
  （typecheck + node:test 113 件 fail 0）
- `mise run fmt` / `check` / `lint` / `test` / `build`: Pass
  （workspace、test 58 suite fail 0）
- `mise run check-cli-headless`: Pass（headless 10 crate に GUI
  framework 依存なし）
- `mise run check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui`: Pass
  （`gpui-test` feature lane。`ruviform-ui-gpui@fd1d4d5f` が
  compile/test を通過）
- `mise run check-ui-webview` / `lint-ui-webview` /
  `test-ui-webview`: Pass（Legacy WebView lane 維持）
- `mise run check-ci` / `lint-ci` / `test-ci`: Pass（CI profile、
  clippy `-D warnings` 含む）
- `mise run fmt-check`: Skipped（task 未定義。`fmt` task が
  `cargo fmt --all -- --check` を担う）
- NotRun/Unsupported: 実 OS GUI・実 Pixel・macOS/Linux・
  cross-consumer・`gui-real`・ruviform `test-ui-real-scenario`
  （実 desktop session 前提の manual lane）。詳細は
  `docs/test-specs/ruviform-fd1d4d5f-rev-watch.md` を正本とする。

## Risks

- upstream の新機能領域（real OS window/pixel・OS input・WASAPI
  input・plugin report・MIDI bridge）は AL lane 未踏。candidate
  tuple としての継続評価で担保し、Issue #127 は open のまま。
- `proc-macro-error2 v2.0.1` の future-incompatibility warning
  （依存由来、記録のみ）。

## Follow-ups

- `real-scenario` / OS input ops / SendInput preflight session
  cache / Linux offscreen golden / L1 wire report verb /
  WASAPI-native input / bridge unsupported 透過の AL 採用判断は
  別 slice の Issue で行う（本 slice では採用しない）。
- 実機 lane（実 OS GUI・実 Pixel・macOS/Linux）の整備は既存の
  NotRun 追跡に従う。
