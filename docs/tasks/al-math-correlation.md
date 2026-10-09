---
id: "TASK-035eXcVbNmAsDfGhJkLzQ"
type: "task"
status: "todo"
title: "al-math: 相関/NCC/bandpass カーネル"
created: "2026-10-09"
updated: "2026-10-09"
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
    - "docs/plans/simd-dsp-foundation.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/al-math-fft.md"
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
    - "docs/test-specs/simd-dsp-foundation.md"
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/40"
    - "https://github.com/scarecrow173/audio-lens/issues/42"
  defers: []
  deferred-by: []
---

# al-math: 相関/NCC/bandpass カーネル

## Work

`al-locate::dsp` の相関・フィルタ系カーネルを `al-math` に移行し
SIMD 化する。

- `ncc`(窓相関)、FFT 相関(fft_ncc 相当)、`gcc_phat` を
  al-math に移設
- `bandpass` の再設計: 現行 spectral-mask FFT(689ms/2.4M)を
  IIR(Butterworth 相当)or 1回 FFT→帯域ごと IFFT 分割に変更。
  目標は `sosfiltfilt` 相当(46ms)に近づけること
- `spectral_envelope` 等の補助カーネルも移行対象
- 既存テスト(bandpass_isolates_band 等)の挙動を維持

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/simd-dsp-foundation.md`

## Dependencies

- `docs/tasks/al-math-fft.md`

## Affected Files

- `crates/al-math/src/correlation.rs`(新規)
- `crates/al-math/src/filter.rs`(新規)
- `crates/al-locate/src/dsp.rs`

## Done When

- [ ] bandpass が現行比 10× 以上高速(または sosfiltfilt 同等)
- [ ] 相関・NCC が scalar 結果と一致
- [ ] 既存 golden(long_source, skip_and_resume, loop 系)が全て
      パス

## Verification

- [ ] `cargo test -p al-math -p al-locate`
- [ ] `cargo run --release -p al-locate --example bench_dsp`
