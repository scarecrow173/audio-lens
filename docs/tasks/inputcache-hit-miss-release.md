---
id: "TASK-034GMMK00SyuEB3Ztrq2v7"
type: "task"
status: "done"
title: "InputCache hit/miss 計数と release 検証口"
created: "2026-10-16"
updated: "2026-10-16"
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
    - "docs/plans/locate-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/locate-plan.md"
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
    - "docs/test-specs/locate-plan-pool-inputcache-coarse-to-fine.md"
  references: []
  defers: []
  deferred-by: []
---

# InputCache hit/miss 計数と release 検証口

## Work

`InputCache` に hit/miss 計数と監査口を追加する。

- `hits`/`misses` カウンタ、`CacheStats`(`stats()`)、`len()`/`is_empty()`
- `release(id, version)` が一致エントリのみを除去することをテストで検証

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/locate-plan.md`
- `docs/specs/locate-plan-cache.md`

## Dependencies

- `docs/plans/locate-plan.md`

## Affected Files

- `crates/al-locate/src/prepared.rs`

## Done When

- [x] 再利用呼出が hit、初回/再 prepare が miss として計上される
- [x] release が対象 id/version のみを除去する

## Verification

- [x] `cargo test -p al-locate prepared`
