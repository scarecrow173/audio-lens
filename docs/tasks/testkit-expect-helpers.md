---
id: "TASK-034AaFZgLvWhJ4wJdv5pXt"
type: "task"
status: "done"
title: "testkit: 期待メトリクス検証ヘルパ"
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

# testkit: 期待メトリクス検証ヘルパ

## Work

期待メトリクス(値+許容誤差/関係)を検証する `expect` 系ヘルパを `al-testkit` に実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/golden-validation.md`
- `docs/test-specs/golden-validation.md`

## Dependencies

- `docs/tasks/al-testkit-crate.md`

## Affected Files

- `crates/al-testkit/src/expect.rs`

## Done When

- [ ] 許容誤差付きの照合ができる
- [ ] 失敗時に期待/実測が出る

## Verification

- [ ] `cargo test -p al-testkit`
