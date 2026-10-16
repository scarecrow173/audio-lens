---
id: "TASK-034GNyEfl8Rs2lzZbeJNoc"
type: "task"
status: "done"
title: "OverlapCorr 逐点比較と dispatch 同等性テスト"
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
    - "docs/plans/local-lag-fft.md"
  implemented-by: []
  depends-on:
    - "docs/plans/local-lag-fft.md"
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
    - "docs/test-specs/local-lag-overlap-corr.md"
  references: []
  defers: []
  deferred-by: []
---

# OverlapCorr 逐点比較と dispatch 同等性テスト

## Work

`OverlapCorr` が `corr_at` の逐点再現であることを検証するテストを追加する。

- 合成信号で全 lag の curve vs `corr_at` 走査を比較(peak 一致、相対誤差 ≤1e-9)
- `pulp::Scalar` との逐点偏差 ≤1e-12
- 無音窓・`max_lag ≥ L` のガード挙動

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/local-lag-fft.md`
- `docs/specs/local-lag-overlap-corr-fft.md`
- `docs/adr/lag-al-math-plan.md`

## Dependencies

- `docs/plans/local-lag-fft.md`

## Affected Files

- `crates/al-math/src/correlation.rs`

## Done When

- [x] peak 一致とスコア許容誤差を pin するテストが pass

## Verification

- [x] `cargo test -p al-math`
