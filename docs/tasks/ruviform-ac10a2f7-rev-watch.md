---
id: "TASK-034XLUpL4KslyVSItyRR9g"
type: "task"
status: "done"
title: "ruviform ac10a2f7 pin 追従評価タスク"
created: "2026-11-15"
updated: "2026-11-15"
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
    - "docs/designs/ruviform-ac10a2f7-rev-watch.md"
  implemented-by: []
  depends-on:
    - "docs/designs/ruviform-ac10a2f7-rev-watch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/ruviform-c6e4f334-rev-watch.md"
    - "docs/tasks/ruviform-fd1d4d5f-rev-watch.md"
    - "docs/tasks/ruviform-b02942c2-rev-watch.md"
    - "docs/tasks/ruviform-271e2d41-rev-watch.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ruviform-ac10a2f7-rev-watch.md"
  derived-by: []
  verifies:
    - "docs/test-specs/ruviform-ac10a2f7-rev-watch.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform ac10a2f7 pin 追従評価タスク

## Work

ruviform pin を `c6e4f334` → `ac10a2f7761c08db73aebd861c0a5c1f53f269b7`
へ lockfile-primary 手順で追従し、対象 revision で正規 mise 検証
lane を実行、`c6e4f334..ac10a2f7` の影響評価・新 API/feature の採用判断（今回は採用しない）・
採用/保留判断を記録するタスク。製品コード変更はなく、変更は
`Cargo.lock`・`ui/vendor/ruviform`（生成物）・docs のみ。
本 slice の文書セットは spec/design/test-spec/task/IR の 5 点
（plans 文書は含めない）。

| # | 内容 | 状態 |
| --- | --- | --- |
| 1 | ruviform main HEAD 確定（`git ls-remote` refs/heads/main = local clone `G:/repositry/audio-products/ruviform` origin/main = `ac10a2f7761c08db73aebd861c0a5c1f53f269b7`） | 完了 |
| 2 | `c6e4f334..ac10a2f7` の差分精査（14 commit・148 file、`crates/` 40 file・+9,049/−184、docs/ 87 file・evidence/ 18 file）と consumer 影響評価 | 完了 |
| 3 | `tools/update-ruviform-rev.mjs` で `Cargo.lock` 20 package を `ac10a2f7` へ更新（SHA fragment 20 行のみ、推移的依存変動なし） | 完了 |
| 4 | `mise run sync-ui-deps` で `ui/vendor/ruviform` 再生成・`REV` 一致確認 | 完了 |
| 5 | `check-ruviform-lock`/`check-ruviform-rev` で lockfile-primary 契約を検証 | 完了 |
| 6 | `fmt`/`check-full`/`lint-full`/`test-full`/`build-full` を実行 | 完了 |
| 7 | `check-cli-headless`/`check-ci-workflow`/`check-ci-tracker-workflows`/`verify-toolchain`/`cargo fetch --locked` を実行 | 完了 |
| 8 | `setup-ui`（npm ci）→ `check-ui`/`test-ui`（TS headless）を実行 | 完了 |
| 9 | `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`/`build-ui-gpui` を実行 | 完了 |
| 10 | `check-ui-webview`/`lint-ui-webview`/`test-ui-webview` を実行 | 完了 |
| 11 | 差分影響評価を design doc に記録 | 完了 |
| 12 | `git fetch origin` で最新 main を確認（worktree 起点 = origin/main HEAD `ea101c5`、merge 不要） | 完了 |
| 13 | commit → push → PR 作成（`Refs #127`） | 完了 |

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/designs/ruviform-ac10a2f7-rev-watch.md`
- `docs/specs/ruviform-ac10a2f7-rev-watch.md`
- `docs/test-specs/ruviform-ac10a2f7-rev-watch.md`

## Dependencies

- ruviform private repo への system git 認証（`git-fetch-with-cli`）
- 前提 pin: `c6e4f334`（PR #213 で採用済み）
- `ui/` の npm devDependencies: 本 worktree では `mise run setup-ui`
  で `npm ci --prefix ui` を実行して復元

## Affected Files

- `Cargo.lock` - ruviform 20 package の解決 SHA
- `ui/vendor/ruviform/**` - vendored UI source + `REV`（生成物）
- `docs/{specs,designs,tasks,test-specs,impl/ir}/
  ruviform-ac10a2f7-rev-watch.md` と各 README index

## Done When

- [x] `Cargo.lock` と `ui/vendor/ruviform/REV` が
  `ac10a2f7761c08db73aebd861c0a5c1f53f269b7` に一致
- [x] 全正規検証 lane が対象 revision で実行・記録済み
- [x] 新 API/feature（`ruviform-plugin-dock-test`・
  `ruviform-plugin-test-vst3`・ui-test plugin_dock scenario/
  `PluginHostProbe`・endpoint probe examples・io-e2e opt-in lanes・
  plugin-core MIDI event contract）の採用判断（採用しない）が記録済み
- [x] 採用判断（採用）と rollback 手順が記録済み
- [x] PR 作成（`Refs #127`）、Issue は open のまま

## Verification

- `mise run check-ruviform-lock` / `check-ruviform-rev`:
  Pass（単一 SHA、vendor 一致、fail-closed 構造維持）
- `mise run sync-ui-deps` → `check-ui` → `test-ui`: Pass（113 tests）
- `mise run fmt` / `check-full` / `lint-full` / `test-full` /
  `build-full`: Pass（workspace、test 58 suite fail 0）
- `mise run check-cli-headless`: Pass（headless 10 crate に GUI
  framework 依存なし）
- `mise run check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui` /
  `build-ui-gpui`: Pass
- `mise run check-ui-webview` / `lint-ui-webview` /
  `test-ui-webview`: Pass
- `mise run check-ci-workflow` / `check-ci-tracker-workflows` /
  `verify-toolchain`: Pass
- `mise exec -- cargo fetch --locked`: Pass
- 詳細な結果表と NotRun 項目は
  `docs/test-specs/ruviform-ac10a2f7-rev-watch.md` を正本とする
