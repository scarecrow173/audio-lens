---
id: "TASK-034Y1A2b3C4d5E6f7G8h9"
type: "task"
status: "in-progress"
title: "ruviform pin abb9f372 追従 + consumer oracle タスク"
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
    - "docs/specs/ruviform-abb9f372-rev-watch.md"
  implemented-by: []
  depends-on:
    - "docs/specs/ruviform-abb9f372-rev-watch.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-127-rf628-consumer-oracle.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ruviform-abb9f372-rev-watch.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform pin abb9f372 追従 + consumer oracle タスク

## Work

rev-watch spec の実施記録。実作業の正本は
`docs/tasks/issue-127-rf628-consumer-oracle.md`（同一 slice の
文書チェーン task）であり、本 task は pin 追従評価としての
採用判断・検証・rollback を追跡する。

| # | 内容 | 状態 |
| --- | --- | --- |
| 1 | ruviform main 再解決・`pipeline/` 存在確認 | 完了 |
| 2 | pin bump + `ruviform-audio-test`/`ruviform-audio-core` dev-dep | 完了 |
| 3 | lock/rev check・sync-ui-deps | 完了 |
| 4 | `rf628_oracle.rs` 実装（AL-1/2/3） | 完了 |
| 5 | 差分影響評価・採用判断記録 | 完了（design doc） |
| 6 | 全検証 lane・index 再生成 | 進行中 |
| 7 | PR（`Refs #127`）・Issue #127 記録 | 未着手 |

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/ruviform-abb9f372-rev-watch.md`
- `docs/specs/issue-127-rf628-consumer-oracle.md`

## Dependencies

- `docs/tasks/issue-127-rf628-consumer-oracle.md`

## Affected Files

- `Cargo.toml` / `Cargo.lock` / `crates/al-analysis/Cargo.toml`
- `crates/al-analysis/tests/rf628_oracle.rs`
- rev-watch 文書 5 点 + index

## Done When

- [ ] 採用 SHA `abb9f372` が lockfile/rev check で確認される
- [ ] AL-1/2/3 テスト Pass、全検証 lane 結果記録
- [ ] 採用/保留・rollback が文書化される
- [ ] PR 作成（`Refs #127`）、Issue #127 に記録（Close しない）

## Verification

- `docs/test-specs/ruviform-abb9f372-rev-watch.md` を正本とする
