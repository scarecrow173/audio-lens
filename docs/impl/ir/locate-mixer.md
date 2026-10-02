---
id: "IMPL-034z8Dfd6xsTuo3nyGla6D"
type: "impl"
status: "completed"
title: "locate-mixer"
created: "2026-10-02"
updated: "2026-10-02"
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
    - "docs/tasks/locate-mixer.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# locate-mixer

## Summary

`mixer::fit_mix` を実装。deinterleave 済みソース/キャプチャ各 ch を
入力に、capture ch ごとの最小二乗で source→capture 混合行列を推定する。
符号付き係数(極性反転は負係数)、リッジ正則化、適合品質
(残差 RMS/キャプチャ RMS)、条件数を記録する。

## Implementation

### Changes

- `crates/al-locate/src/mixer.rs`: `MixerEstimate` + `fit_mix`

### Decisions During Implementation

- 条件数はリッジ適用前の正規行列に対して計算(病態の診断値として
  正則化で隠さない)
- 特異系ではリッジ>0 で解き、条件数は Infinity のまま記録

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Context

task `docs/tasks/locate-mixer.md`、spec
`docs/specs/locate-tracking.md`。commit `1a742a6`。

## Validation

- `cargo test -p al-locate` 全 pass(符号付き行列の回復 /
  病態系の条件数報告)

## Risks

- なし

## Follow-ups

- なし
