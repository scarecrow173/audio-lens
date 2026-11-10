---
id: "IMPL-034VnVtzhBuw5urVoJzgnw"
type: "impl"
status: "completed"
title: "ruviform b02942c2 pin 追従評価の実施記録"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "code"
        path: "docs/specs/ruviform-b02942c2-rev-watch.md"
      - type: "code"
        path: "docs/designs/ruviform-b02942c2-rev-watch.md"
      - type: "code"
        path: "docs/plans/ruviform-b02942c2-rev-watch.md"
      - type: "code"
        path: "docs/tasks/ruviform-b02942c2-rev-watch.md"
      - type: "code"
        path: "docs/test-specs/ruviform-b02942c2-rev-watch.md"
      - type: "code"
        path: "docs/impl/ir/ruviform-b02942c2-rev-watch.md"
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
    - "docs/tasks/ruviform-b02942c2-rev-watch.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
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

# ruviform b02942c2 pin 追従評価の実施記録

## Summary

ruviform pin を `271e2d4111d3769693445bc5bffca37af9e0650b` から
`b02942c2f9e6e195d0d752803b077866468fe58e`（ruviform main HEAD）
へ lockfile-primary 手順で追従し、対象 revision で正規 mise 検証
lane を全実行した。`Cargo.lock` の差分は ruviform 20 package の
SHA fragment 更新のみ（推移的依存の変動なし）、`ui/vendor/ruviform`
は `sync-ui-deps` で再生成（`ui/*` TS source に差分がないため
REV 以外不変）。製品コード変更はなく、全 lane Pass を確認。
upstream 新 API（`real-scenario`・SendInput preflight・OS MIDI
backend・plugin→host report・VST3 MIDI）は採用しない判断を記録。

## Context

Issue #127（Ruviform 更新の常設追跡 Issue）の rev-watch 定型
slice。前段は PR #190（pin `271e2d41` 追従評価 + Legacy WebView
棚卸し）。本 slice の対象範囲は pin `271e2d41` → `b02942c2`
（6 commit: ruviform PR #554/#555/#556/#557/#561/#563）。

## Implementation

### Changes

- `Cargo.lock`: `tools/update-ruviform-rev.mjs
  b02942c2f9e6e195d0d752803b077866468fe58e` で ruviform 20
  package の `source` fragment を一括更新。差分は 20 行の SHA
  fragment のみで、推移的依存・他 crate の version 変動なし。
- `ui/vendor/ruviform`: `mise run sync-ui-deps` で lockfile SHA
  から再生成。`REV` = `b02942c2f9e6e195d0d752803b077866468fe58e`。
  ruviform 差分に `ui/*`（core/components/binding/viz）の変更が
  ないため vendor 内容は REV 以外前 pin と同一。
- docs: spec→design→plan→task→test-spec→IR の文書チェーンを
  `ruviform-b02942c2-rev-watch` 名で新規作成し、生成 index
  （docs/{specs,designs,plans,tasks,test-specs}/README.md）を再生成、
  手動 index（docs/impl/ir/README.md）に追記。

### Decisions During Implementation

- pin 更新は完全 SHA 指定（短縮 SHA の誤展開を避ける前段の教訓に
  従う）。`git ls-remote` ではなく local clone
  `F:/repositry/audio-products/ruviform` の HEAD と一致を確認して
  `b02942c2f9e6e195d0d752803b077866468fe58e` を確定。
- `271e2d41..b02942c2` の影響評価: AL の依存木に変更があるのは
  `ruviform-ui-gpui`（opt-in `real-scenario` feature と
  `os_input` preflight の additive、内部 refactor のみ）、
  `ruviform-ui-test`（driver-lab fixture 更新）、
  `ruviform-audio-core`（`midi_input` の additive。AL は MIDI
  非使用）。`ruviform-midi-device`/`plugin-*`/`audio-plugin-bridge`
  は AL 依存木外。破壊的変更なし・adapter 修正不要。
- 新 API の採用判断: `real-scenario`（real OS window adapter）、
  SendInput preflight、OS MIDI backend（WinMM）、plugin→host
  report、VST3 MIDI はいずれも opt-in/別ドメインであり AL 側の
  採用 gate・需要が未定義のため本 slice では採用しない。
- doc graph route は main 既存の broken relation
  （`crates/al-math` 参照切れ）で blocked のため、前段 PR #190 と
  同じく文書チェーンを直接作成し review-protocol で承認相当の
  レビューを行う運用とした。

## Related Experiments

### Adopted

<!-- なし -->

### Rejected

<!-- なし -->

## Validation

- `mise run check-ruviform-lock`: Pass（20 package、単一 SHA
  `b02942c2f9...`）
- `mise run check-ruviform-rev`: Pass（REV = lockfile SHA、
  lockfile-primary、fail-closed 構造維持）
- `mise run sync-ui-deps` → `check-ui` → `test-ui`: Pass
  （typecheck + node:test 113 件 fail 0）
- `mise run fmt` / `check` / `lint` / `test` / `build`: Pass
  （workspace、test 58 suite fail 0）
- `mise run check-cli-headless`: Pass（headless 10 crate に GUI
  framework 依存なし）
- `mise run check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui`: Pass
  （`gpui-test` feature lane）
- `mise run check-ui-webview` / `lint-ui-webview` /
  `test-ui-webview`: Pass（Legacy WebView lane 維持）
- `mise run check-ci` / `lint-ci` / `test-ci`: Pass（CI profile、
  clippy `-D warnings` 含む）
- NotRun/Unsupported: 実 OS GUI・実 Pixel・macOS/Linux・
  cross-consumer・`gui-real`・ruviform `test-ui-real-scenario`
  （実 desktop session 前提の manual lane）。詳細は
  `docs/test-specs/ruviform-b02942c2-rev-watch.md` を正本とする。

## Risks

- upstream の新機能領域（real OS window/pixel・OS input・MIDI・
  plugin report）は AL lane 未踏。candidate tuple としての継続
  評価で担保し、Issue #127 は open のまま。
- `proc-macro-error2 v2.0.1` の future-incompatibility warning
  （依存由来、記録のみ）。

## Follow-ups

- `real-scenario` / SendInput preflight / OS MIDI / plugin→host
  report の AL 採用判断は別 slice の Issue で行う（本 slice では
  採用しない）。
- 実機 lane（実 OS GUI・実 Pixel・macOS/Linux）の整備は既存の
  NotRun 追跡に従う。
