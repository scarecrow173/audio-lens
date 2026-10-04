---
id: "TASK-034AaFSu2D4Mw7dqcQ6qW6"
type: "task"
status: "todo"
title: "exp-noise: stationary/tonal/modulated 分類"
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

# exp-noise: stationary/tonal/modulated 分類

## Work

残差/バンドエネルギー系列を stationary/tonal/modulated に分類する記述子を `expensive/noise.rs` に実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/expensive-families.md`
- `docs/test-specs/expensive-families.md`

## Dependencies

- `docs/tasks/expensive-module-split.md`

## Affected Files

- `crates/al-analysis/src/expensive/noise.rs`

## Done When

- [ ] stationary ノイズが stationary に分類される
- [ ] AM 変調ノイズが modulated に分類される

## Verification

- [ ] `cargo test -p al-analysis`
