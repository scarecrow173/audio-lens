---
id: "IMPL-034z8DdcUFqoh2YQSbPk6w"
type: "impl"
status: "completed"
title: "locate-skeleton-dsp"
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
    - "docs/tasks/locate-skeleton-dsp.md"
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

# locate-skeleton-dsp

## Summary

`crates/al-locate` をワークスペースに追加し、再利用可能な DSP
プリミティブを `dsp.rs` に実装した。

## Implementation

### Changes

- `Cargo.toml`: workspace members / dependencies に `al-locate` を追加
- `crates/al-locate/Cargo.toml`: 新規クレート(al-core / al-job /
  al-media / serde / serde_json)
- `crates/al-locate/src/dsp.rs`: `deinterleave`, `ncc`, `fft`,
  `bandpass`, `gcc_phat`, `spectral_envelope`, `least_squares`,
  `condition_number`

### Decisions During Implementation

- 外部 DSP クレートは使わず FFT も自前実装(plan の依存制約どおり)
- 合成信号 fixture は疑似乱数でも周期性で偽ピークを持ち得るため、
  LCG ホワイトノイズに統一(初期 fixture がアルゴリズムを誤認させた)

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Context

task `docs/tasks/locate-skeleton-dsp.md`、plan
`docs/plans/locate-and-tracking.md`、spec
`docs/specs/source-localization.md` に従う。commit `d10a0b5`。

## Validation

- `cargo test -p al-locate` 全 pass
- `cargo clippy -p al-locate` clean / `cargo fmt` 適用済み
- `cargo metadata` で workspace member として解決される

## Risks

- FFT は素朴な radix-2 実装。大入力の性能は実測で要検証

## Follow-ups

- なし
