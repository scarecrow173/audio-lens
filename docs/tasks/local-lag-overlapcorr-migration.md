---
id: "TASK-034GNyEFEOZSC93dTivNPe"
type: "task"
status: "todo"
title: "local_lag を OverlapCorr 経由へ移行"
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

# local_lag を OverlapCorr 経由へ移行

## Work

`reference.rs` の `local_lag` を OverlapCorr 経由へ移行する。

- `al_locate::dsp` に新規型を re-export
- `analyze` が窓ループ前に `OverlapCorrPlan::new(lag_window)` + workspace を構築
- `local_lag` は借用曲線から `(best_lag, best_score, half_width)` を算出(ロジック不変)

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/local-lag-fft.md`
- `docs/specs/local-lag-overlap-corr-fft.md`
- `docs/adr/lag-al-math-plan.md`

## Dependencies

- `docs/plans/local-lag-fft.md`

## Affected Files

- `crates/al-locate/src/dsp.rs`
- `crates/al-analysis/src/reference.rs`

## Done When

- [ ] `ref.lag_*` を返す既存テストが不変で pass

## Verification

- [ ] `cargo test -p al-analysis`
