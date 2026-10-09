---
id: "TASK-035dQwErTyUiOpAsDfGhJk"
type: "task"
status: "todo"
title: "al-math: FFT カーネル(複素/実数)"
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
    - "docs/tasks/al-math-skeleton.md"
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
    - "https://github.com/scarecrow173/audio-lens/issues/42"
  defers: []
  deferred-by: []
---

# al-math: FFT カーネル(複素/実数)

## Work

現行 `al-locate::dsp::fft`(手書き radix-2 複素 FFT、twiddle 逐次
再帰、SIMD なし)を `al-math` に SIMD 化して移行する。

- pulp の `Simd`/`c64xN` でバタフライをベクトル化
- 実数信号用に rfft(2-for-1 または専用実数 FFT)を用意し、
  「実数を複素 FFT に掛ける」現行の 2 倍無駄を解消
- pow2 以外の長さの対応方針を決める(混合基数 or 現行どおり
  次の pow2 パディング — まずは現行互換で良い)
- `al_locate::dsp::fft` の呼び出し元を `al-math` に差し替え
  (dsp.rs の fft は thin wrapper or 直接置き換え)
- `bench_dsp` で NumPy pocketfft(157ms/4M)同等以上を確認

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/simd-dsp-foundation.md`
- `docs/adr/simd-arithmetic-pulp.md`

## Dependencies

- `docs/tasks/al-math-skeleton.md`

## Affected Files

- `crates/al-math/src/fft.rs`(新規)
- `crates/al-locate/src/dsp.rs`(呼び出しを al-math に差替え)

## Done When

- [ ] 現行 radix-2 結果と数値一致(許容誤差内)
- [ ] `bench_dsp` の FFT が現行比 3× 以上高速
- [ ] 既存テスト(fft 往復、gcc_phat 等)が全てパス

## Verification

- [ ] `cargo test -p al-math -p al-locate`
- [ ] `cargo run --release -p al-locate --example bench_dsp`
