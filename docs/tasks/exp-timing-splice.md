---
id: "TASK-034AaFTNVqwkFuTpDA07oM"
type: "task"
status: "todo"
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

click/splice の定位と stuck/repeated buffer 候補の深掘りを `expensive/timing.rs` に実装する。

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

## Verification

- [ ] `cargo test -p al-analysis`
