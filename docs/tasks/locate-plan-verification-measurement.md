---
id: "TASK-034GMMM1tcW8J95GVhHG9W"
type: "task"
status: "done"
title: "全検証と計測記録"
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

# 全検証と計測記録

## Work

全検証と計測証跡を記録する。

- `cargo test -p al-locate` / `--workspace` / clippy `-D warnings` / fmt check
- `bench_locate_prepared` で pool builds/hits と計測時間を記録

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/locate-plan.md`
- `docs/specs/locate-plan-cache.md`

## Dependencies

- `docs/plans/locate-plan.md`

## Affected Files

- `crates/al-locate/examples/bench_locate_prepared.rs`

## Done When

- [x] 全テスト pass、clippy/fmt clean
- [x] pool builds が呼出数に対して 1-2 に収束することを記録

## Verification

- [x] `cargo test --workspace`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `cargo fmt --all -- --check`
- [x] `cargo build --release --example bench_locate_prepared -p al-locate && 実行`
