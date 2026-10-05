---
id: "TASK-034AaFDINCaNF95aiWa0oQ"
type: "task"
status: "done"
title: "exp-dynamics: transfer-curve + pumping + level依存"
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

# exp-dynamics: transfer-curve + pumping + level依存

## Work

compressor/limiter の transfer-curve 候補(入出力ゲイン散布の折れ)、pumping/ducking の低周波変調、level-dependent transfer を `expensive/dynamics.rs` に実装する。level-dependent はペア証拠が必要なため、リファレンスペア不在時は invalid 観測とする。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/expensive-families.md`
- `docs/test-specs/expensive-families.md`

## Dependencies

- `docs/tasks/expensive-module-split.md`

## Affected Files

- `crates/al-analysis/src/expensive/dynamics.rs`

## Done When

- [ ] クリップ/リミット済み信号で折れが観測される
- [ ] レベル依存性が記録される

## Verification

- [ ] `cargo test -p al-analysis`
