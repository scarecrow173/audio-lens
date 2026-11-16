---
id: "TASK-034YnH7YfuK1bhY85g3gRg"
type: "task"
status: "done"
title: "ruviform cbcc8343 pin 追従評価タスク"
created: "2026-11-16"
updated: "2026-11-16"
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
    - "docs/designs/ruviform-cbcc8343-rev-watch.md"
  implemented-by: []
  depends-on:
    - "docs/designs/ruviform-cbcc8343-rev-watch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/ruviform-eafe0a08-rev-watch.md"
    - "docs/tasks/ruviform-ac10a2f7-rev-watch.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ruviform-cbcc8343-rev-watch.md"
  derived-by: []
  verifies:
    - "docs/test-specs/ruviform-cbcc8343-rev-watch.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform cbcc8343 pin 追従評価タスク

## Work

ruviform pin を `eafe0a08` → `cbcc83437616fdd7418da37183c6be66b6cfe225`
へ lockfile-primary 手順で追従し、対象 revision で正規 mise 検証
lane を実行、`eafe0a08..cbcc8343` の影響評価・新 API の採用判断
（今回は採用しない）・採用/保留判断を記録するタスク。製品コード
変更はなく、変更は `Cargo.lock`・`ui/vendor/ruviform`（生成物）・
docs のみ。本 slice の文書セットは spec/design/test-spec/task/IR
の 5 点（plans 文書は含めない）。

| # | 内容 | 状態 |
| --- | --- | --- |
| 1 | ruviform main HEAD 確定（`git ls-remote` refs/heads/main = local clone `G:/repositry/audio-products/ruviform` origin/main = `cbcc83437616fdd7418da37183c6be66b6cfe225`。Spec 起稿時点と同一、upstream 進行なし） | 完了 |
| 2 | `eafe0a08..cbcc8343` の差分精査（6 commit・97 file、crates/ 9 crate 変更、docs/evidence/mise.toml/root Cargo.lock）と consumer 影響評価 | 完了 |
| 3 | `tools/update-ruviform-rev.mjs` で `Cargo.lock` 26 package を `cbcc8343` へ更新（差分は SHA fragment 26 行のみ、version 変動・新規 package・依存 edge 変動なし） | 完了 |
| 4 | `mise run sync-ui-deps` で `ui/vendor/ruviform` 再生成・`REV` 一致確認 | 完了 |
| 5 | `check-ruviform-lock`/`check-ruviform-rev` で lockfile-primary 契約を検証 | 完了 |
| 6 | `fmt`/`check al-analysis`/`lint al-analysis`/`test al-analysis`（`rf628_oracle` 含む standing oracle）を実行 | 完了 |
| 7 | `check-cli-headless`/`verify-toolchain`/`cargo fetch --locked` を実行 | 完了 |
| 8 | `setup-ui`（npm ci）→ `check-ui`/`test-ui`（TS headless）を実行 | 完了 |
| 9 | 補助証拠として `check-full`（workspace check）を実行 | 完了 |
| 10 | upstream main CI 証拠の確認（`cbcc8343`: `consumer-verify`/`daily-ci` success、`ci` failure — 依存木外 crate のみ、切り分け記録） | 完了 |
| 11 | 差分影響評価を design doc に記録 | 完了 |
| 12 | `git fetch origin` で最新 main を確認（worktree 起点 = origin/main HEAD `62d4761`、merge 不要） | 完了 |
| 13 | commit → push → PR 作成（`Refs #127`） | 完了 |

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/designs/ruviform-cbcc8343-rev-watch.md`
- `docs/specs/ruviform-cbcc8343-rev-watch.md`
- `docs/test-specs/ruviform-cbcc8343-rev-watch.md`

## Dependencies

- ruviform private repo への system git 認証（`git-fetch-with-cli`）
- 前提 pin: `eafe0a08`（PR #216 で採用済み）
- `ui/` の npm devDependencies: 本 worktree では `mise run setup-ui`
  で `npm ci --prefix ui` を実行して復元

## Affected Files

- `Cargo.lock` - ruviform 26 package の解決 SHA
- `ui/vendor/ruviform/**` - vendored UI source + `REV`（生成物）
- `docs/{specs,designs,tasks,test-specs,impl/ir}/
  ruviform-cbcc8343-rev-watch.md` と
  `evidence/al127/cbcc8343-verification.md`
- 注: docs index（`docs/*/README.md`）は本 slice では再生成しない
  （統合時に Coordinator が再生成する運用のため）

## Done When

- [x] `Cargo.lock` と `ui/vendor/ruviform/REV` が
  `cbcc83437616fdd7418da37183c6be66b6cfe225` に一致
- [x] 全正規検証 lane が対象 revision で実行・記録済み
- [x] 新 API/feature（record codec・plugin test_instrument・
  audio-device endpoint probe・Test Lab GUI・DComp pixel
  regression）の採用判断（採用しない）が記録済み
- [x] 採用判断（採用）と rollback 手順が記録済み
- [x] upstream CI 証拠（`ci` failure の依存木外切り分けを含む）
  が記録済み
- [x] PR 作成（`Refs #127`）、Issue は open のまま

## Verification

- `mise run check-ruviform-lock`: Pass（26 package、単一 SHA
  `cbcc8343`）
- `mise run check-ruviform-rev`: Pass（REV = lockfile SHA、
  lockfile-primary、fail-closed 構造維持）
- `mise run sync-ui-deps` → `check-ui` → `test-ui`: Pass
  （typecheck + node:test 113 件 fail 0）
- `mise run fmt` / `check al-analysis` / `lint al-analysis` /
  `test al-analysis`: Pass（`rf628_oracle` 7 tests・長尺
  scenario `rf628_long_scenario_non_regression` 約 132 秒実走を
  含む全 suite fail 0）
- `mise run check-cli-headless`: Pass（headless 10 crate に GUI
  framework 依存なし）
- `mise run verify-toolchain`: Pass（rustc/cargo 1.99.1）
- `mise exec -- cargo fetch --locked`: Pass
- 補助証拠: `mise run check-full` Pass（workspace check）
- 詳細な結果表と NotRun 項目は
  `docs/test-specs/ruviform-cbcc8343-rev-watch.md` を正本とする
