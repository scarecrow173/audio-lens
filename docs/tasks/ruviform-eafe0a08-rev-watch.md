---
id: "TASK-034YaQRhfjwFoCxtc2kZrh"
type: "task"
status: "done"
title: "ruviform eafe0a08 pin 追従評価タスク"
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
    - "docs/designs/ruviform-eafe0a08-rev-watch.md"
  implemented-by: []
  depends-on:
    - "docs/designs/ruviform-eafe0a08-rev-watch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/ruviform-ac10a2f7-rev-watch.md"
    - "docs/tasks/ruviform-abb9f372-rev-watch.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ruviform-eafe0a08-rev-watch.md"
  derived-by: []
  verifies:
    - "docs/test-specs/ruviform-eafe0a08-rev-watch.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform eafe0a08 pin 追従評価タスク

## Work

ruviform pin を `abb9f372` → `eafe0a08b71bd9a74566ae6678c3e0683fce1421`
へ lockfile-primary 手順で追従し、対象 revision で正規 mise 検証
lane を実行、`abb9f372..eafe0a08` の影響評価・新 API の採用判断
（今回は採用しない）・採用/保留判断を記録するタスク。製品コード
変更はなく、変更は `Cargo.lock`・`ui/vendor/ruviform`（生成物）・
docs のみ。本 slice の文書セットは spec/design/test-spec/task/IR
の 5 点（plans 文書は含めない）。

| # | 内容 | 状態 |
| --- | --- | --- |
| 1 | ruviform main HEAD 確定（`git ls-remote` refs/heads/main = local clone `G:/repositry/audio-products/ruviform` origin/main = `eafe0a08b71bd9a74566ae6678c3e0683fce1421`。Spec 起稿時点と同一、upstream 進行なし） | 完了 |
| 2 | `abb9f372..eafe0a08` の差分精査（6 commit・93 file、crates/ 38 file・+7,442/−259、docs/ 45 file・evidence/ 8 file）と consumer 影響評価 | 完了 |
| 3 | `tools/update-ruviform-rev.mjs` で `Cargo.lock` 26 package を `eafe0a08` へ更新（SHA fragment 26 行 + `ruviform-audio-test` の `serde`/`serde_json` edge 2 行のみ、version 変動・新規 package なし） | 完了 |
| 4 | `mise run sync-ui-deps` で `ui/vendor/ruviform` 再生成・`REV` 一致確認 | 完了 |
| 5 | `check-ruviform-lock`/`check-ruviform-rev` で lockfile-primary 契約を検証 | 完了 |
| 6 | `fmt`/`check al-analysis`/`lint al-analysis`/`test al-analysis`（`rf628_oracle` 含む standing oracle）を実行 | 完了 |
| 7 | `check-cli-headless`/`verify-toolchain`/`cargo fetch --locked` を実行 | 完了 |
| 8 | `setup-ui`（npm ci）→ `check-ui`/`test-ui`（TS headless）を実行 | 完了 |
| 9 | 補助証拠として `check-full`（workspace check）を実行 | 完了 |
| 10 | upstream main CI 証拠の確認（`eafe0a08` に covering CI なし → NotRun 記録） | 完了 |
| 11 | 差分影響評価を design doc に記録 | 完了 |
| 12 | `git fetch origin` で最新 main を確認（worktree 起点 = origin/main HEAD `e8b2aed`、merge 不要） | 完了 |
| 13 | commit → push → PR 作成（`Refs #127`） | 完了 |

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/designs/ruviform-eafe0a08-rev-watch.md`
- `docs/specs/ruviform-eafe0a08-rev-watch.md`
- `docs/test-specs/ruviform-eafe0a08-rev-watch.md`

## Dependencies

- ruviform private repo への system git 認証（`git-fetch-with-cli`）
- 前提 pin: `abb9f372`（PR #215 で採用済み）
- `ui/` の npm devDependencies: 本 worktree では `mise run setup-ui`
  で `npm ci --prefix ui` を実行して復元

## Affected Files

- `Cargo.lock` - ruviform 26 package の解決 SHA +
  `ruviform-audio-test` の `serde`/`serde_json` edge
- `ui/vendor/ruviform/**` - vendored UI source + `REV`（生成物）
- `docs/{specs,designs,tasks,test-specs,impl/ir}/
  ruviform-eafe0a08-rev-watch.md` と各 README index、
  `evidence/al127/eafe0a08-verification.md`

## Done When

- [x] `Cargo.lock` と `ui/vendor/ruviform/REV` が
  `eafe0a08b71bd9a74566ae6678c3e0683fce1421` に一致
- [x] 全正規検証 lane が対象 revision で実行・記録済み
- [x] 新 API/feature（RunMetadata JSON・`StreamFault::
  Cancellation`・plugin state bridge・test_editor/test_stateful
  fixture・plugin-dock real-editor attach）の採用判断（採用しない）
  が記録済み
- [x] 採用判断（採用）と rollback 手順が記録済み
- [x] PR 作成（`Refs #127`）、Issue は open のまま

## Verification

- `mise run check-ruviform-lock` / `check-ruviform-rev`:
  Pass（26 package、単一 SHA、REV 一致、fail-closed 構造維持）
- `mise run sync-ui-deps` → `check-ui` → `test-ui`: Pass
  （typecheck + node:test 113 件 fail 0）
- `mise run fmt` / `check al-analysis` / `lint al-analysis` /
  `test al-analysis`: Pass（`rf628_oracle` 7 tests 含む全 suite
  fail 0。長尺 scenario `rf628_long_scenario_non_regression` 実走）
- `mise run check-cli-headless`: Pass（headless 10 crate に GUI
  framework 依存なし。`serde`/`serde_json` の dev-dep 波及なし）
- `mise run verify-toolchain`: Pass（rustc/cargo 1.99.1）
- `mise exec -- cargo fetch --locked`: Pass
- 補助証拠: `mise run check-full` Pass（workspace check）
- 詳細な結果表と NotRun 項目は
  `docs/test-specs/ruviform-eafe0a08-rev-watch.md` を正本とする
