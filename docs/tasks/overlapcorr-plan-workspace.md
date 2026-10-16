---
id: "TASK-034GNyDJKV8WtTW1TbBP06"
type: "task"
status: "done"
title: "OverlapCorrPlan/OverlapCorrWorkspace を al-math に追加"
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

# OverlapCorrPlan/OverlapCorrWorkspace を al-math に追加

## Work

等長窓・両側 lag・overlap 正規化の相関カーネルを `crates/al-math/src/correlation.rs` に追加する。

- `OverlapCorrPlan::new(window_len)`(内部に `RealFftPlan` N=next_pow2(2L))
- `OverlapCorrWorkspace::new(&plan)`(FFT ws + real/bins/prefix バッファを全確保)
- `plan.curve(&mut ws, x, y, max_lag) -> &[f64]`、`curve[i]` が `lag = i−max_lag`
- `|lag| ≥ L` と `ex≤0 || ey≤0` は 0.0
- owned-curve 入口 `overlap_corr_curve(x, y, max_lag) -> Vec<f64>`

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

- [x] corr0 逐点再現のカーネルがコンパイルされて借用契約を持つ

## Verification

- [x] `cargo check -p al-math`
