---
id: "TASK-034AaFEfKrZkVSHWqbJo7H"
type: "task"
status: "done"
title: "exp-codec: pre-echo + transient smear"
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

# exp-codec: pre-echo + transient smear

## Work

オンセット前の pre-echo 候補・transient smear・cepstrum 副次ピークによる二次エコー遅延/レベルを `expensive/codec.rs` に実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/expensive-families.md`
- `docs/test-specs/expensive-families.md`

## Dependencies

- `docs/tasks/expensive-module-split.md`

## Affected Files

- `crates/al-analysis/src/expensive/codec.rs`

## Done When

- [ ] pre-echo 的な高域エネルギーが観測される
- [ ] トランジェントの時間的にじみが記録される
- [ ] 既知エコーで二次遅延/レベルが記録される

## Verification

- [ ] `cargo test -p al-analysis`
