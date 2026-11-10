---
id: "TASK-034VnVry1uhHUyyAsDeWFx"
type: "task"
status: "done"
title: "ruviform b02942c2 pin 追従評価タスク"
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
    - "docs/plans/ruviform-b02942c2-rev-watch.md"
  implemented-by: []
  depends-on:
    - "docs/plans/ruviform-b02942c2-rev-watch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/ruviform-271e2d41-rev-watch.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/ruviform-b02942c2-rev-watch.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform b02942c2 pin 追従評価タスク

## Work

ruviform pin を `271e2d41` → `b02942c2f9e6e195d0d752803b077866468fe58e`
へ lockfile-primary 手順で追従し、対象 revision で正規 mise 検証
lane を実行、`271e2d41..b02942c2` の影響評価・新 API の採用判断
（今回は採用しない）・採用/保留判断を記録するタスク。製品コード
変更はなく、変更は `Cargo.lock`・`ui/vendor/ruviform`（生成物）・
docs のみ。

| # | 内容 | 状態 |
| --- | --- | --- |
| 1 | ruviform main HEAD 確認と完全 SHA 確定（local clone HEAD = `b02942c2`） | 完了 |
| 2 | `tools/update-ruviform-rev.mjs` で `Cargo.lock` 20 package を `b02942c2` へ更新 | 完了 |
| 3 | `mise run sync-ui-deps` で `ui/vendor/ruviform` 再生成・`REV` 一致確認 | 完了 |
| 4 | `check-ruviform-lock`/`check-ruviform-rev` で lockfile-primary 契約を検証 | 完了 |
| 5 | `fmt`/`check`/`lint`/`test`/`build`/`check-cli-headless` を実行 | 完了 |
| 6 | `check-ui`/`test-ui`（TS headless）を実行 | 完了 |
| 7 | `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui` を実行 | 完了 |
| 8 | `check-ui-webview`/`lint-ui-webview`/`test-ui-webview` を実行 | 完了 |
| 9 | `check-ci`/`lint-ci`/`test-ci`（CI profile）を実行 | 完了 |
| 10 | 差分影響評価を design doc に記録 | 完了 |
| 11 | `git fetch origin main && git merge origin/main`（Already up to date） | 完了 |
| 12 | commit → push → PR 作成（`Refs #127`） | 完了 |

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/ruviform-b02942c2-rev-watch.md`
- `docs/specs/ruviform-b02942c2-rev-watch.md`
- `docs/designs/ruviform-b02942c2-rev-watch.md`

## Dependencies

- ruviform private repo への system git 認証（`git-fetch-with-cli`）
- 前提 pin: `271e2d41`（PR #190 で採用済み）

## Affected Files

- `Cargo.lock` - ruviform 20 package の解決 SHA
- `ui/vendor/ruviform/**` - vendored UI source + `REV`（生成物）
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/
  ruviform-b02942c2-rev-watch.md` と各 README index

## Done When

- [x] `Cargo.lock` と `ui/vendor/ruviform/REV` が
  `b02942c2f9e6e195d0d752803b077866468fe58e` に一致
- [x] 全正規検証 lane が対象 revision で実行・記録済み
- [x] 新 API（real-scenario・SendInput preflight・OS MIDI backend・
  plugin→host report）の採用判断（採用しない）が記録済み
- [x] 採用判断（採用）と rollback 手順が記録済み
- [x] PR 作成（`Refs #127`）、Issue は open のまま

## Verification

- `mise run check-ruviform-lock` / `check-ruviform-rev`:
  Pass（単一 SHA、vendor 一致、fail-closed 構造維持）
- `mise run sync-ui-deps` → `check-ui` → `test-ui`: Pass（113 tests）
- `mise run fmt`/`check`/`lint`/`test`/`build`/
  `check-cli-headless`: Pass
- `mise run check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`: Pass
- `mise run check-ui-webview`/`lint-ui-webview`/
  `test-ui-webview`: Pass
- `mise run check-ci`/`lint-ci`/`test-ci`: Pass
- 詳細な結果表と NotRun 項目は
  `docs/test-specs/ruviform-b02942c2-rev-watch.md` を正本とする
