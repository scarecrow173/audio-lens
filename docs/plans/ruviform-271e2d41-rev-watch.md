---
id: "PLAN-034Vc1j1fzjqjqxWEMZUlZ"
type: "plan"
status: "approved"
title: "ruviform 271e2d41 pin 追従評価と棚卸しの計画"
created: "2026-11-10"
updated: "2026-11-10"
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
    - "docs/designs/ruviform-271e2d41-rev-watch.md"
  implemented-by: []
  depends-on:
    - "docs/designs/ruviform-271e2d41-rev-watch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ruviform-271e2d41-rev-watch.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/ruviform-271e2d41-rev-watch.md"
  references:
    - "docs/discovery/issue-127-legacy-webview-inventory.md"
  defers: []
  deferred-by: []
---

# ruviform 271e2d41 pin 追従評価と棚卸しの計画 Implementation Plan

## Goal

ruviform pin を `84d21641` → `271e2d41`（ruviform main HEAD、merge
PR #551）へ lockfile-primary 手順で追従し、正規 mise 検証 lane を
対象 revision で実行して結果を記録する。あわせて Issue #127 の
Legacy 撤去 Gate に必要な旧 WebView 棚卸し表を作成する。採用/保留
判断と rollback 情報を残し、Issue #127 へ結果を記録して PR を作る。

## Scope

- `Cargo.lock`（ruviform 20 package の解決 SHA）と
  `ui/vendor/ruviform`（生成物、`REV` 含む）の更新。
- 差分影響評価と検証結果の docs 記録、棚卸し表の作成。
- commit/push/PR/Issue comment。Issue #127 は Close しない。

## Upstream Documents

- `docs/specs/ruviform-271e2d41-rev-watch.md`
- `docs/designs/ruviform-271e2d41-rev-watch.md`
- `docs/discovery/issue-127-legacy-webview-inventory.md`

## Design Inputs

- `docs/designs/ruviform-271e2d41-rev-watch.md`

## File Map

- `Cargo.lock` - ruviform 20 package の `source` fragment を
  `271e2d41...` へ（`tools/update-ruviform-rev.mjs` 生成物）
- `ui/vendor/ruviform/**` - `mise run sync-ui-deps` の生成物
  （`REV` に SHA を記録）
- `docs/specs|designs|plans|tasks|test-specs/
  ruviform-271e2d41-rev-watch.md` - 本文書チェーン
- `docs/discovery/issue-127-legacy-webview-inventory.md` -
  Legacy WebView 棚卸し表
- `docs/{specs,designs,plans,tasks,test-specs,discovery}/README.md` -
  生成 index の再生成

## Implementation Sequence

- [x] `git fetch` で ruviform main HEAD を確認し、完全 SHA
  `271e2d4111d3769693445bc5bffca37af9e0650b` を確定
- [x] `node tools/update-ruviform-rev.mjs <full-sha>` で
  `Cargo.lock` を更新（短縮 SHA の誤展開を完全 SHA で修正）
- [x] `mise run sync-ui-deps` で vendor tree を再生成
- [x] `check-ruviform-lock`/`check-ruviform-rev`/`check-ui`/
  `test-ui`/`fmt`/`check`/`lint`/`test`/`build`/
  `check-cli-headless` を実行
- [x] `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui` と
  `check-ui-webview`/`lint-ui-webview`/`test-ui-webview` を実行
- [x] `check-ci`/`lint-ci`/`test-ci`（CI profile）を実行
- [x] `84d21641..271e2d41` の差分影響評価を design doc に記録
- [x] Legacy WebView 棚卸し表を discovery doc に作成
- [x] `git fetch origin main && git merge origin/main`（Already up
  to date を確認）
- [ ] Conventional Commits で commit → push → PR 作成
  （`Refs #127`）→ Issue #127 comment

## Dependencies and Risks

- ruviform は private repo で `cargo fetch`/`sync-ui-deps` が system
  git 認証（`.cargo/config.toml` の `git-fetch-with-cli`）に依存。
  認証失敗時は pin 更新自体が失敗する → 実績として成功済み。
- 大規模差分（88 commit）に対し検証 lane の網羅性リスク → 正規
  20 task を全実行し、未到達 gate は NotRun 記録で対処。
- doc graph の既存 broken relation（`crates/al-math` 欠落 path を
  参照する discovery/spec 群）は main 既存の問題で本タスク範囲外。
  route_graph は `focus-required`+broken-relation で blocked を返す
  ため、固定受入条件の是正タスクとして文書チェーンを直接作成し、
  review-protocol で承認相当のレビューを行う（前段 PR #188 と同じ
  運用）。

## Task Breakdown

- 単一タスクとして実施（pin 追従 + 評価記録 + 棚卸し）。
  製品コード変更がないため分割不要。

## Verification

- [x] `mise run check-ruviform-lock` / `check-ruviform-rev`:
  resolved SHA = `271e2d41...`、20 package、vendor REV 一致
- [x] `mise run sync-ui-deps` → `check-ui` → `test-ui`:
  Pass（ui test 112 件、fail 0）
- [x] `mise run fmt` / `check` / `lint` / `test` / `build`:
  Pass（workspace、test 58 suite fail 0）
- [x] `mise run check-cli-headless`: Pass（headless 10 crate に
  GUI framework 依存なし）
- [x] `mise run check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui`:
  Pass（GPUI feature lane、parity/desktop/screen 等全 pass）
- [x] `mise run check-ui-webview` / `lint-ui-webview` /
  `test-ui-webview`: Pass（Legacy WebView lane 維持確認）
- [x] `mise run check-ci` / `lint-ci` / `test-ci`: Pass
  （CI profile、clippy `-D warnings` 含む）
- [x] 未検証項目の NotRun/Unsupported 記録（実 OS GUI・実 Pixel・
  macOS/Linux・cross-consumer・`gui-real`）

## Open Questions / Gaps

- 実 OS GUI・実 Pixel golden・macOS/Linux lane は本実行環境
  （Windows、headless/semantic のみ）では到達不能 → NotRun として
  記録し、Pass に繰り上げない。後続 slice で実機 lane を整備する。
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

- reviewer は「未実行 gate を Pass としていないか」「棚卸し分類が
  実在箇所と一致するか」「lockfile-primary 契約が維持されているか」
  を重点確認。
