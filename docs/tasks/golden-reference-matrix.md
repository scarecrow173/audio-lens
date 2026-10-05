---
id: "TASK-034AaG0wyqS2s6eM05bcTA"
type: "task"
status: "todo"
title: "golden: reference ケース行列"
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
    - "docs/plans/golden-validation.md"
  implemented-by: []
  depends-on:
    - "docs/plans/golden-validation.md"
    - "docs/tasks/al-testkit-crate.md"
    - "docs/tasks/testkit-expect-helpers.md"
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
    - "docs/test-specs/golden-validation.md"
  references: []
  defers: []
  deferred-by: []
---

# golden: reference ケース行列

## Work

spec の reference ケース行列を testkit fixture で実装する(既知 gain/EQ/位相/固定・jitter 遅延/SRC drift/極性/ch swap/低コヒーレンスノイズ/clip/comb/echo/SRC alias/drop/repeat/stuck)。ピン留め失敗モード「非コヒーレントな PSD 比による高域リフト偽装」を名前付きケースとして含める。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/golden-validation.md`
- `docs/test-specs/golden-validation.md`

## Dependencies

- `docs/tasks/al-testkit-crate.md`
- `docs/tasks/testkit-expect-helpers.md`

## Affected Files

- `crates/al-analysis/tests/golden.rs`

## Done When

- [ ] spec 列挙の reference ケースがテストとして存在する
- [ ] 既知パラメータが許容誤差内で回復される

## Verification

- [ ] `cargo test -p al-analysis`
