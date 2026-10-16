---
id: "TASK-034GNyF8pIMrWca9EKOhHN"
type: "task"
status: "todo"
title: "小窓交差の実測と閾値判定・検証記録"
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

# 小窓交差の実測と閾値判定・検証記録

## Work

窓サイズ別に直接法と FFT 法を実測し、小窓 dispatch 閾値の要否を決める。

- 例: L ∈ {256, 512, 1024, 2048, 4096, 8192} で Instant 計測
- 交差があれば閾値を導入して根拠を記録、なければ FFT 一本とする
- 全検証(cargo test workspace, clippy, fmt)と計測結果を記録

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/local-lag-fft.md`
- `docs/specs/local-lag-overlap-corr-fft.md`
- `docs/adr/lag-al-math-plan.md`

## Dependencies

- `docs/plans/local-lag-fft.md`

## Affected Files

- `crates/al-analysis/examples/`
- `crates/al-analysis/src/reference.rs`

## Done When

- [ ] 計測結果を記録し、閾値の有無を根拠付きで決定

## Verification

- [ ] `cargo test --workspace`
- [ ] bench/example の実行記録
