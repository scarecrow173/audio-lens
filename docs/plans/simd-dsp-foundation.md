---
id: "PLAN-035aRtYvBnMxQwErTyUiOp"
type: "plan"
status: "approved"
title: "SIMD 算術クレート基盤と DSP カーネル高速化"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/adr/simd-arithmetic-pulp.md"
    - "docs/adr/implementation-stack.md"
    - "docs/specs/source-localization.md"
    - "docs/designs/simd-dsp-foundation.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/simd-dsp-foundation.md"
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/37"
    - "https://github.com/scarecrow173/audio-lens/issues/40"
    - "https://github.com/scarecrow173/audio-lens/issues/42"
    - "https://github.com/scarecrow173/audio-lens/issues/43"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# SIMD 算術クレート基盤と DSP カーネル高速化

## Goal

独自算術クレート `al-math` を pulp 基盤で新設し、NumPy/SciPy
ツールチェーンとのベンチ差(FFT ~2.1×/実質 ~4×、bandpass ~15×)を
解消する。あわせて sample-rate 正規化(#37)、coarse-to-fine 探索と
帯域フィルタキャッシュ(#40)、least_squares の QR 化(#43)を実装
する。

対応 issue: #37, #40, #42, #43

## Scope

`al-math` クレート新設(pulp のみを外部 SIMD 依存として集約)+
`al-locate`/`al-analysis` の既存 DSP カーネルの `al-math` 移行。
GPU は対象外(転送コスト支配・決定性リスク、issue #42 参照)。

## Design Inputs

- `docs/adr/simd-arithmetic-pulp.md`(accepted): pulp 採用、
  `Arch::dispatch` で実行時 ISA 振り分け、c64xN 複素型
- `docs/research/simd-arithmetic-crates/final-report.md`:
  wide / pulp / thermite の比較根拠
- ベンチ基準: `crates/al-locate/examples/bench_dsp.rs`
  (NumPy NCC 328ms / 現行 891ms、bandpass 46ms / 689ms)

## Task Decomposition

1. `al-math` クレート骨格 — pulp 依存追加、`Arch` 保持・
   dispatch 集約、f64xN/c64xN の型再エクスポート
2. FFT カーネル — 複素/実数 FFT を al-math に実装
   (radix-2 互換 + 混合基数検討)、simd 化
3. 相関/NCC/bandpass カーネル — al-math 経由に移行、
   bandpass は IIR または 1 回 FFT→帯域分割へ再設計
4. `least_squares` を Householder QR 化(#43)
5. locate: 入力単位の bandpass キャッシュ + 間引き
   coarse-to-fine 探索(#40)
6. locate: sample-rate 正規化層(リジェクトを廃止し
   polyphase/FFT リサンプルで統一、#37)

## Verification

- `docs/test-specs/simd-dsp-foundation.md` の契約
- 全 golden テスト一致(または許容誤差内)を維持
- `bench_dsp` で NumPy/SciPy 同等以上(FFT 3×↑、bandpass 10×↑)
