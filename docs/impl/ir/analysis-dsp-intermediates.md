---
id: "IMPL-034zjc5CvQwv1X10BXDkKP"
type: "impl"
status: "completed"
title: "analysis-dsp-intermediates"
created: "2026-10-03"
updated: "2026-10-03"
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
    - "docs/tasks/analysis-dsp-intermediates.md"
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

# analysis-dsp-intermediates

## Summary

`al-analysis::stft` に複素 STFT と全スペクトル中間生成物を
実装した(外部 DSP クレート不使用、`al-locate::dsp::fft`
の radix-2 FFT を再利用)。

## Context

task `docs/tasks/analysis-dsp-intermediates.md`、plan
`docs/plans/analysis-stages.md`、spec
`docs/specs/cheap-analysis-features.md` /
`docs/specs/reference-analysis.md` に従う。

## Implementation

### Changes

- `crates/al-analysis/src/stft.rs`: `stft`(Hann 窓 + hop +
  非 2 冪窓は次の 2 冪へゼロパディング)、`StftMatrix`(
  `magnitude` / `power` / `psd` を全 bin で返す)、
  `band_power`、`third_octave_bands`(IEC 1/3 オクターブ)、
  `csd`(conj(x)·y)、`complex_coherence`(位相保持)、
  `envelope`(log 平滑スペクトル包絡)

### Decisions During Implementation

- task 記載の `dsp.rs` ではなく `stft.rs` とした: 時間領域
  プリミティブは `al-locate::dsp` が所有し、al-analysis 側は
  周波数領域生成物のみを持つ方が責務が明確
- PSD は two-sided 正規化(`|X|²/(fs·Σw²)`)。全 `fft_size`
  bin を返すため Parseval で総パワーが復元される
- 短い信号で 0 フレームを返し、validity 判定
  (`InsufficientSupport`)は呼び出し側の責務とした

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Validation

- `cargo test -p al-analysis` 全 pass(既知正弦波の bin 位置、
  非 2 冪窓のゼロパディング、PSD の Parseval 積分、
  1/3 オクターブ帯域カバレッジ、同一信号の coherence=1)
- `cargo clippy --workspace --all-targets -- -D warnings` clean

## Risks

- FFT は radix-2 素朴実装のため大規模 STFT は遅い。perf が問題
  にされた段階で改善する

## Follow-ups

- `intermediate` ロールへの tensor 永続化は各解析ステージ
  (cheap/reference)側で DatasetWriter 経由で行う
