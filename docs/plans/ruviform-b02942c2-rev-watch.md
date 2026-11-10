---
id: "PLAN-034VnV6uzeSDaCLr2eIx2N"
type: "plan"
status: "approved"
title: "ruviform b02942c2 pin 追従評価の計画"
created: "2026-11-11"
updated: "2026-11-11"
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
    - "docs/designs/ruviform-b02942c2-rev-watch.md"
  implemented-by: []
  depends-on:
    - "docs/designs/ruviform-b02942c2-rev-watch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/ruviform-271e2d41-rev-watch.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ruviform-b02942c2-rev-watch.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/ruviform-b02942c2-rev-watch.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform b02942c2 pin 追従評価の計画 Implementation Plan

## Goal

ruviform pin を `271e2d41` → `b02942c2f9e6e195d0d752803b077866468fe58e`
（ruviform main HEAD）へ lockfile-primary 手順で追従し、正規 mise
検証 lane を対象 revision で実行して結果を記録する。`271e2d41..
b02942c2` の影響評価・新 API の採用判断（今回は採用しない）・
採用/保留判断と rollback 情報を残し、PR を作る。

## Scope

- `Cargo.lock`（ruviform 20 package の解決 SHA）と
  `ui/vendor/ruviform`（生成物、`REV` 含む）の更新。
- 差分影響評価と検証結果の docs 記録。
- commit/push/PR。Issue #127 は Close しない（結果記録は PR 本文で
  代行し、Issue comment は Coordinator が確認する）。

## Upstream Documents

- `docs/specs/ruviform-b02942c2-rev-watch.md`
- `docs/designs/ruviform-b02942c2-rev-watch.md`

## Design Inputs

- `docs/designs/ruviform-b02942c2-rev-watch.md`

## File Map

- `Cargo.lock` - ruviform 20 package の `source` fragment を
  `b02942c2...` へ（`tools/update-ruviform-rev.mjs` 生成物）
- `ui/vendor/ruviform/**` - `mise run sync-ui-deps` の生成物
  （`REV` に SHA を記録。gitignore 対象）
- `docs/specs|designs|plans|tasks|test-specs|impl/ir/
  ruviform-b02942c2-rev-watch.md` - 本文書チェーン
- `docs/{specs,designs,plans,tasks,test-specs}/README.md` -
  生成 index の再生成、`docs/impl/ir/README.md` - 手動 index への
  追記

## Implementation Sequence

- [x] ruviform main HEAD を local clone で確認し、完全 SHA
  `b02942c2f9e6e195d0d752803b077866468fe58e` を確定
- [x] `git fetch origin main && git merge origin/main`
  （Already up to date を確認）
- [x] `node tools/update-ruviform-rev.mjs <full-sha>` で
  `Cargo.lock` を更新（差分は SHA fragment 20 行のみ）
- [x] `mise run sync-ui-deps` で vendor tree を再生成
- [x] `check-ruviform-lock`/`check-ruviform-rev`/`check-ui`/
  `test-ui`/`fmt`/`check`/`lint`/`test`/`build`/
  `check-cli-headless` を実行
- [x] `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui` と
  `check-ui-webview`/`lint-ui-webview`/`test-ui-webview` を実行
- [x] `check-ci`/`lint-ci`/`test-ci`（CI profile）を実行
- [x] `271e2d41..b02942c2` の差分影響評価を design doc に記録
- [ ] Conventional Commits で commit → push → PR 作成
  （`Refs #127`）

## Dependencies and Risks

- ruviform は private repo で `cargo fetch`/`sync-ui-deps` が system
  git 認証（`.cargo/config.toml` の `git-fetch-with-cli`）に依存。
  認証失敗時は pin 更新自体が失敗する → 実績として成功済み。
- 差分は 6 commit と小規模だが upstream に real OS window/pixel・
  OS input・MIDI・plugin report の新機能が含まれる → AL lane で
  未踏の領域は NotRun 記録で対処し、採用は別 slice とする。
- doc graph の既存 broken relation（`crates/al-math` 欠落 path を
  参照する discovery/spec 群）は main 既存の問題で本タスク範囲外。
  route_graph は `focus-required`+broken-relation で blocked を返す
  ため、固定受入条件の是正タスクとして文書チェーンを直接作成し、
  review-protocol で承認相当のレビューを行う（前段 PR #190 と同じ
  運用）。

## Task Breakdown

- 単一タスクとして実施（pin 追従 + 評価記録）。製品コード変更が
  ないため分割不要。

## Verification

- [x] `mise run check-ruviform-lock` / `check-ruviform-rev`:
  resolved SHA = `b02942c2f9...`、20 package、vendor REV 一致
- [x] `mise run sync-ui-deps` → `check-ui` → `test-ui`:
  Pass（ui test 113 件、fail 0）
- [x] `mise run fmt` / `check` / `lint` / `test` / `build`:
  Pass（workspace、test 58 suite fail 0）
- [x] `mise run check-cli-headless`: Pass（headless 10 crate に
  GUI framework 依存なし）
- [x] `mise run check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui`:
  Pass（GPUI feature lane）
- [x] `mise run check-ui-webview` / `lint-ui-webview` /
  `test-ui-webview`: Pass（Legacy WebView lane 維持確認）
- [x] `mise run check-ci` / `lint-ci` / `test-ci`: Pass
  （CI profile、clippy `-D warnings` 含む）
- [x] 未検証項目の NotRun/Unsupported 記録（実 OS GUI・実 Pixel・
  macOS/Linux・cross-consumer・`gui-real`・ruviform
  `test-ui-real-scenario` lane）

## Open Questions / Gaps

- 実 OS GUI・実 Pixel golden・macOS/Linux lane・ruviform 側
  `test-ui-real-scenario`（実 desktop session 前提の manual lane）
  は本実行環境では到達不能または対象外 → NotRun として記録し、
  Pass に繰り上げない。
- `gui-real` CI job は `not_run` placeholder であり native GUI
  acceptance の証拠ではない（既存仕様どおり）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（本タスクで実施済み）
- Independent work streams: none
- User approval required before dispatch: yes → review-protocol で
  blocker なしを確認済みとして扱う

## Review Handoff

- reviewer は「未実行 gate を Pass としていないか」「lockfile-
  primary 契約が維持されているか」「新 API の採用判断が明示
  されているか」を重点確認。
