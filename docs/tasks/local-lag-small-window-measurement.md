---
id: "TASK-034GNyF8pIMrWca9EKOhHN"
type: "task"
status: "done"
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

- [x] 計測結果を記録し、閾値の有無を根拠付きで決定

## Verification

- [x] `cargo test --workspace`
- [x] bench/example の実行記録

## Measurement

`bench_overlap_corr`(release、窓あたり µs、direct = 現行 corr0 走査):

| window | max_lag | direct | OverlapCorr | speedup |
| --- | --- | --- | --- | --- |
| 128 | 32 | 1.9 | 1.7 (直接 dispatch) | 1.1x |
| 256 | 64 | 6.7 | 6.3 | 1.1x |
| 512 | 128 | 25.8 | 13.4 | 1.9x |
| 1024 | 256 | 103.0 | 36.0 | 2.9x |
| 2048 | 512 | 414.3 | 65.3 | 6.3x |
| 8192 | 512 | 1848.2 | 283.6 | 6.5x |
| 16384 | 512 | 3727.0 | 535.3 | 7.0x |

交差は ~L=256 付近。`max_lag` にも依存するため、固定閾値ではなく
仕事量モデル `3·(2·max_lag+1)·L < 15·N·log2(N)` で dispatch する
(N=next_pow2(2L−1))。小 `max_lag` でも直接法が選ばれる。

## e2e 観測(samples 実データ run、JOB-8hxnz2d7LtiqzuoEW6diXa)

- reference_analysis: 60.5 s(前回 61.5 s)。この入力では
  local_lag の窓数が少なく(~585 窓 × ~1.8 ms ≈ ~1 s/pair)、
  ステージ全体では軽微 — 支配要因は別(STFT/CSD/FIR 系)にある。
- カーネル単体では窓あたり 6.5x(既定 8192/lag 512)を実測。
- locate は #40 の効果で 344.7 → 272.9 s(−21%)。
