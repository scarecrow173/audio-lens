---
id: "TASK-034GMMKYWaNUIglxx3wXe3"
type: "task"
status: "done"
title: "LocateMethod 経路を CorrelationPool へ移行"
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

# LocateMethod 経路を CorrelationPool へ移行

## Work

LocateMethod 経路の相関呼出を `CorrelationPool` へ移行する。

- `FullNcc`/`BandNcc`/`GccPhat`/`SpectralEnvelope` の `locate`/`locate_prepared`/`locate_coarse_to_fine` に `pool: &mut CorrelationPool` を追加
- `propose_coarse_to_fine`/`refine_coarse`/`evidence_from_corr` は借用曲線をその場消費
- `SpectralEnvelope` は corr0 経路のため pool を受け取るが使わない
- tests/examples 全呼出点を追従

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/locate-plan.md`
- `docs/specs/locate-plan-cache.md`

## Dependencies

- `docs/plans/locate-plan.md`

## Affected Files

- `crates/al-locate/src/methods.rs`
- `crates/al-locate/tests/locate.rs`
- `crates/al-locate/tests/prepared.rs`
- `crates/al-locate/examples/bench_ncc.rs`
- `crates/al-locate/examples/bench_locate_prepared.rs`

## Done When

- [x] 全メソッド経路が pool 経由で相関する
- [x] 既存証拠・golden テストが全て pass

## Verification

- [x] `cargo test -p al-locate`
- [x] `cargo test --workspace`
