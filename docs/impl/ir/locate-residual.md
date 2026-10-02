---
id: "IMPL-034z8DfCfOKTTNak1Zz7jf"
type: "impl"
status: "completed"
title: "locate-residual"
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
    - "docs/tasks/locate-residual.md"
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

# locate-residual

## Summary

2 段階残差探索を `residual.rs` に実装。受理アライメントの混合を
推定・減算し、残差キャプチャで子 locate を再実行する。来歴として
親ソース・親 alignment id・推定ゲイン・減算に使ったチャンネル変換
(混合行列)・減算前後エネルギー・確度・子 LocateResult を保持する。

## Implementation

### Changes

- `crates/al-locate/src/residual.rs`: `ResidualEnergy` /
  `ResidualSearch` / `subtract_fit` / `residual_search`

### Decisions During Implementation

- 確度は「除去できたエネルギー比」と「子結果の受理」の平均で算出
- 子 locate はクロージャ注入にし、ステージが設定を制御できるようにした

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Context

task `docs/tasks/locate-residual.md`、spec
`docs/specs/locate-tracking.md`。commit `1a742a6`。

## Validation

- `cargo test -p al-locate` 全 pass(支配的 BGM 減算後の残差エネルギー
  低下 / 来歴フィールドの保存)

## Risks

- なし

## Follow-ups

- なし
