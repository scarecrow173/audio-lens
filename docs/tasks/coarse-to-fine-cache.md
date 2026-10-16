---
id: "TASK-034GMMLy26EDCzZmTAUf9y"
type: "task"
status: "done"
title: "coarse-to-fine 境界・cache 監査テスト"
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

# coarse-to-fine 境界・cache 監査テスト

## Work

coarse-to-fine 境界と InputCache の監査テストを追加する。

- coarse 位置のフルレート換算が半径内に真オフセットを含むこと
- 探索範囲端の候補が `boundary-candidate` とマークされること
- hit/miss 計数と release 後の再 miss を検証

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/locate-plan.md`
- `docs/specs/locate-plan-cache.md`

## Dependencies

- `docs/plans/locate-plan.md`

## Affected Files

- `crates/al-locate/tests/prepared.rs`
- `crates/al-locate/src/prepared.rs`

## Done When

- [x] coarse_positions_land_within_radius_of_the_true_offset が pass
- [x] candidates_at_the_search_range_edges_are_marked_boundary が pass
- [x] hit/miss/release テストが pass

## Verification

- [x] `cargo test -p al-locate --test prepared`
