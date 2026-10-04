---
id: "TASK-034AaFCh7tNIivN73GA2cF"
type: "task"
status: "done"
title: "expensive: サブモジュール分割(振る舞い不変)"
created: "2026-10-04"
updated: "2026-10-04"
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
    - "docs/plans/expensive-families.md"
  implemented-by: []
  depends-on:
    - "docs/plans/expensive-families.md"
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
    - "docs/test-specs/expensive-families.md"
  references: []
  defers: []
  deferred-by: []
---

# expensive: サブモジュール分割(振る舞い不変)

## Work

`src/expensive.rs` をルート mod(config/trigger/plan_regions/analyze_region)+`src/expensive/` サブモジュール(ir/codec/dynamics/noise/channel/timing/transfer)に分割する。既存のスペクトル系(cutoff/narrowband/THD)は transfer.rs、`exp.src_*` は timing.rs へ。振る舞いは変えず、非 Valid トリガの invalid 観測リストも全ファミリ分を網羅して移す。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/expensive-families.md`
- `docs/test-specs/expensive-families.md`

## Dependencies

- none

## Affected Files

- `crates/al-analysis/src/expensive.rs`
- `crates/al-analysis/src/expensive/`

## Done When

- [ ] 分割後も `cargo test -p al-analysis` が全パスする
- [ ] 公開 API・特徴 id が不変

## Verification

- [ ] `cargo test -p al-analysis`
