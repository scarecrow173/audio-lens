---
id: "TASK-034GsRM5JI2xVj38GXTd9s"
type: "task"
status: "done"
title: "fsutil-atomic-write-primitives"
created: "2026-10-15"
updated: "2026-10-15"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-artifact-durability-tiers.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-artifact-durability-tiers.md"
  blocks:
    - "docs/tasks/relaxed-tier-callsite-migration.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/durability-tiers-resume-verification.md"
  references: []
  defers: []
  deferred-by: []
---

# fsutil-atomic-write-primitives

## Work

fsutil に `write_atomic`(tmp+write+rename、fsync なし)と `rename_atomic`(fs::rename のみ)を追加する。`write_durable` と tmp パス・エラー時 cleanup 規律を共有する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-artifact-durability-tiers.md`

## Dependencies

- なし(最初のタスク)

## Affected Files

- `crates/al-job/src/fsutil.rs`

## Done When

- [ ] `write_atomic`/`rename_atomic` が公開 API として存在し、
      内容を正しく書き tmp を残さず既存ファイルを置き換える
- [ ] 単体テストが `write_durable` と同じ正常系・置換規律を確認する

## Verification

- [ ] `cargo test -p al-job` がパスする
- [ ] 両関数が `sync_all`/dir sync を呼ばないことをコードで確認
