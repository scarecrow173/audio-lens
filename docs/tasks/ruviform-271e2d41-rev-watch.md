---
id: "TASK-034Vc1sd1nC8xoAnTOVPsF"
type: "task"
status: "done"
title: "ruviform 271e2d41 pin 追従評価タスク"
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
    - "docs/plans/ruviform-271e2d41-rev-watch.md"
  implemented-by: []
  depends-on:
    - "docs/plans/ruviform-271e2d41-rev-watch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/ruviform-271e2d41-rev-watch.md"
  derived-by: []
  verifies:
    - "docs/test-specs/ruviform-271e2d41-rev-watch.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform 271e2d41 pin 追従評価タスク

## Work

ruviform pin を `84d21641` → `271e2d4111d3769693445bc5bffca37af9e0650b`
へ lockfile-primary 手順で追従し、対象 revision で正規 mise 検証
lane を実行、`84d21641..271e2d41` の影響評価・Legacy WebView
棚卸し・採用/保留判断を記録するタスク。製品コード変更はなく、
変更は `Cargo.lock`・`ui/vendor/ruviform`（生成物）・docs のみ。

| # | 内容 | 状態 |
| --- | --- | --- |
| 1 | ruviform main HEAD 確認と完全 SHA 確定（`git ls-remote`） | 完了 |
| 2 | `tools/update-ruviform-rev.mjs` で `Cargo.lock` 20 package を `271e2d41` へ更新 | 完了 |
| 3 | `mise run sync-ui-deps` で `ui/vendor/ruviform` 再生成・`REV` 一致確認 | 完了 |
| 4 | `check-ruviform-lock`/`check-ruviform-rev` で lockfile-primary 契約を検証 | 完了 |
| 5 | `fmt`/`check`/`lint`/`test`/`build`/`check-cli-headless` を実行 | 完了 |
| 6 | `check-ui`/`test-ui`（TS headless）を実行 | 完了 |
| 7 | `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui` を実行 | 完了 |
| 8 | `check-ui-webview`/`lint-ui-webview`/`test-ui-webview` を実行 | 完了 |
| 9 | `check-ci`/`lint-ci`/`test-ci`（CI profile）を実行 | 完了 |
| 10 | 差分影響評価を design doc、棚卸し表を discovery doc に記録 | 完了 |
| 11 | `git fetch origin main && git merge origin/main`（Already up to date） | 完了 |
| 12 | commit → push → PR 作成（`Refs #127`）→ Issue #127 comment（本変更の PR として実施） | 完了 |

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/ruviform-271e2d41-rev-watch.md`
- `docs/specs/ruviform-271e2d41-rev-watch.md`
- `docs/designs/ruviform-271e2d41-rev-watch.md`
- `docs/discovery/issue-127-legacy-webview-inventory.md`

## Dependencies

- ruviform private repo への system git 認証（`git-fetch-with-cli`）
- 前提 pin: `84d21641`（PR #188 で採用済み）

## Affected Files

- `Cargo.lock` - ruviform 20 package の解決 SHA
- `ui/vendor/ruviform/**` - vendored UI source + `REV`（生成物）
- `docs/{specs,designs,plans,tasks,test-specs}/
  ruviform-271e2d41-rev-watch.md`、`docs/discovery/
  issue-127-legacy-webview-inventory.md` と各 README index

## Done When

- [x] `Cargo.lock` と `ui/vendor/ruviform/REV` が
  `271e2d4111d3769693445bc5bffca37af9e0650b` に一致
- [x] 全正規検証 lane が対象 revision で実行・記録済み
- [x] 棚卸し表が残存/撤去候補/独立 Issue に分類済み
- [x] 採用判断（採用）と rollback 手順が記録済み
- [x] PR 作成（`Refs #127`）、Issue #127 comment、Issue は open のまま

## Verification

- `mise run check-ruviform-lock` / `check-ruviform-rev`:
  Pass（単一 SHA、vendor 一致、fail-closed 構造維持）
- `mise run sync-ui-deps` → `check-ui` → `test-ui`: Pass（112 tests）
- `mise run fmt`/`check`/`lint`/`test`/`build`/
  `check-cli-headless`: Pass
- `mise run check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`: Pass
- `mise run check-ui-webview`/`lint-ui-webview`/
  `test-ui-webview`: Pass
- `mise run check-ci`/`lint-ci`/`test-ci`: Pass
- 詳細な結果表と NotRun 項目は
  `docs/test-specs/ruviform-271e2d41-rev-watch.md` を正本とする
