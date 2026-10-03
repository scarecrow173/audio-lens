---
id: "TASK-034z8kIHPMPeuyRptsxAAF"
type: "task"
status: "done"
title: "mixer-fit: 最小二乗混合行列推定"
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
    - "docs/plans/locate-and-tracking.md"
  implemented-by: []
  depends-on:
    - "docs/plans/locate-and-tracking.md"
    - "docs/tasks/locate-skeleton-dsp.md"
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
    - "docs/test-specs/locate-tracking-residual.md"
  references: []
  defers: []
  deferred-by: []
---

# mixer-fit: 最小二乗混合行列推定

## Work

per-channel 最小二乗 source→capture 混合行列推定を実装。符号付き係数(極性反転は負)、リッジ正則化、適合品質・条件数を記録する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/locate-and-tracking.md`
- `docs/specs/locate-tracking.md`

## Dependencies

- `docs/tasks/locate-skeleton-dsp.md`

## Affected Files

- `crates/al-locate/src/mixer.rs`

## Done When

- [ ] 混合行列が符号付き係数として推定される
- [ ] 適合品質・条件数が記録される
- [ ] 入力スケール差(`sample_format`)を比較前に正規化

## Verification

- [ ] 極性反転 fixture で負係数が推定される
- [ ] `cargo test -p al-locate` パス
