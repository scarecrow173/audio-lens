---
id: "TASK-034AaFURCQqe9DBwLB4N0J"
type: "task"
status: "todo"
title: "expensive 追加ファミリの既知ケース検証"
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
    - "docs/tasks/exp-ir-decay.md"
    - "docs/tasks/exp-dynamics-curve.md"
    - "docs/tasks/exp-codec-preecho.md"
    - "docs/tasks/exp-noise-decomp.md"
    - "docs/tasks/exp-channel-matrix.md"
    - "docs/tasks/exp-timing-splice.md"
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

# expensive 追加ファミリの既知ケース検証

## Work

各ファミリの既知合成ケースを整備し、test-spec の Rules/Examples を検証する結合テストを実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/expensive-families.md`
- `docs/test-specs/expensive-families.md`

## Dependencies

- `docs/tasks/exp-ir-decay.md`
- `docs/tasks/exp-dynamics-curve.md`
- `docs/tasks/exp-codec-preecho.md`
- `docs/tasks/exp-noise-decomp.md`
- `docs/tasks/exp-channel-matrix.md`
- `docs/tasks/exp-timing-splice.md`

## Affected Files

- `crates/al-analysis/tests/expensive.rs`
- `crates/al-analysis/tests/`

## Done When

- [ ] test-spec の各 Example が対応テストを持つ
- [ ] 非 Valid トリガで invalid 観測になる

## Verification

- [ ] `cargo test -p al-analysis`
- [ ] `cargo test --workspace`
