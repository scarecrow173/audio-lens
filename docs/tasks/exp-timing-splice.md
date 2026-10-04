---
id: "TASK-034AaFTNVqwkFuTpDA07oM"
type: "task"
status: "done"
title: "exp-timing: click/splice + stuck buffer 深掘り"
created: "2026-10-04"
updated: "2026-10-04"
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
    - "docs/plans/expensive-families.md"
  implemented-by: []
  depends-on:
    - "docs/plans/expensive-families.md"
    - "docs/tasks/expensive-module-split.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/expensive-families.md"
  references: []
  defers: []
  deferred-by: []
---

# exp-timing: click/splice + stuck buffer 深掘り

## Work

click/splice の定位・stuck/repeated buffer 候補の深掘り・リージョン内局所 time map(局所 lag 軌跡)を `expensive/timing.rs` に実装する。既存 `exp.src_*` もこのモジュールに移設する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/expensive-families.md`
- `docs/test-specs/expensive-families.md`

## Dependencies

- `docs/tasks/expensive-module-split.md`

## Affected Files

- `crates/al-analysis/src/expensive/timing.rs`

## Done When

- [ ] 既知 splice 位置が記録される
- [ ] stuck/repeat ブロック候補が記録される
- [ ] 局所 time map が系列として記録される

## Verification

- [ ] `cargo test -p al-analysis`
