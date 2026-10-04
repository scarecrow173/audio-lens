---
id: "TASK-034AaFD0xQrwpuxDxVEjiO"
type: "task"
status: "todo"
title: "exp-ir: IR 候補 + early/late + decay"
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

# exp-ir: IR 候補 + early/late + decay

## Work

IR 候補推定(deconvolution/H(f) ベース)と early/late エネルギー比・decay 傾きを `expensive/ir.rs` に実装する。数値不安定は条件数/support で防御する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/expensive-families.md`
- `docs/test-specs/expensive-families.md`

## Dependencies

- `docs/tasks/expensive-module-split.md`

## Affected Files

- `crates/al-analysis/src/expensive/ir.rs`

## Done When

- [ ] 既知残響で decay が許容誤差内で回復される
- [ ] support 不足で valid にならない

## Verification

- [ ] `cargo test -p al-analysis`
