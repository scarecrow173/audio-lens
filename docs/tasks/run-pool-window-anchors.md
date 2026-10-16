---
id: "TASK-034GMMKt9UyD6jcOVtDgiW"
type: "task"
status: "done"
title: "run_pool ワーカーコンテキスト拡張と window_anchors 移行"
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

# run_pool ワーカーコンテキスト拡張と window_anchors 移行

## Work

`run_pool` をワーカーコンテキスト生成方式に拡張し、全呼出点を移行する。

- `run_pool(n, make_ctx, work)` — 各ワーカーが `make_ctx()` で `CorrelationPool` を所有
- `run_method_tasks` と `window_anchors_in` の両呼出点を `CorrelationPool::new` コンテキストに移行
- `best_window_match` は `pool: &mut CorrelationPool` を受け取り、検索範囲を `Range<usize>` に整理

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/locate-plan.md`
- `docs/specs/locate-plan-cache.md`

## Dependencies

- `docs/plans/locate-plan.md`

## Affected Files

- `crates/al-locate/src/stage.rs`

## Done When

- [x] run_pool 両呼出点がワーカー所有プールを使う
- [x] best_window_match が pool を引数に取る
- [x] clippy too_many_arguments を引かない

## Verification

- [x] `cargo test -p al-locate`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
