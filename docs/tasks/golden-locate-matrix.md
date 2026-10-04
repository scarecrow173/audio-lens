---
id: "TASK-034AaFZQVKvQFjEmAbNfog"
type: "task"
status: "todo"
title: "golden: locate ケース行列"
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

# golden: locate ケース行列

## Work

spec の locate ケース行列を testkit fixture で実装する(exact/gain/EQ 挿入、silence 境界、強周期偽ピーク、loop、repeated、長尺 offset/drift、skip+resume、multichannel downmix、BGM マスキング+残差探索)。ピン留め失敗モード「無音/強周期での偽相関ピーク受理」と「低コヒーレンス時の optimal-lag 過大推定」を名前付きケースとして含める。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/golden-validation.md`
- `docs/test-specs/golden-validation.md`

## Dependencies

- `docs/tasks/al-testkit-crate.md`
- `docs/tasks/testkit-expect-helpers.md`

## Affected Files

- `crates/al-locate/tests/golden.rs`

## Done When

- [ ] spec 列挙の locate ケースがテストとして存在する
- [ ] 偽受理ケースが受理されない

## Verification

- [ ] `cargo test -p al-locate`
