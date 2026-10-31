---
id: "ADR-035pmRjqxvweesfcw4L90A"
type: "adr"
status: "accepted"
title: "独自 SIMD 算術クレートの基盤に pulp を採用する"
created: "2026-10-09"
updated: "2026-10-12"
owners: ["scarecrow173"]
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
  depends-on:
    - "implementation-stack.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by:
    - "ruviform-foundation-migration.md"
  derives-from:
    - "../research/simd-arithmetic-crates/final-report.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# 独自 SIMD 算術クレートの基盤に pulp を採用する

## Context and Problem Statement

AudioLens の DSP カーネル(FFT、相互相関、帯域フィルタ、リサンプル、
STFT 等)は現在手書きのスカラ実装で、NumPy/SciPy との比較ベンチで
FFT 単体 ~2.1×(実質 ~4×)、bandpass ~15× の劣後が計測された
(`crates/al-locate/examples/bench_dsp.rs`)。独自算術クレートを作成し
SIMD 化する方針が決まったため、その基盤となる portable SIMD 抽象化
クレートを選定する必要がある。

## Decision Drivers

* 同一バイナリで SSE2〜AVX2 の CPU をカバーする**実行時 dispatch** が
  内蔵されていること(自前で CPU 検出+振り分けを書きたくない)
* AudioLens の中心型である **f64** のベクトル演算が揃っていること
* FFT 内部で有利な**複素ベクトル型**があること
* stable Rust で動作すること(AVX-512 は不要)
* 決定的な計算結果(golden テスト互換)であること
* 実戦実績・メンテナンス状況が安定していること

## Considered Options

* wide 1.7.1 — 固定幅型のみ、実行時 dispatch なし、超越関数なし
* pulp 0.22.3 — 実行時 dispatch 内蔵、c64xN 複素型、faer の基盤
* thermite 0.2.1 — 超越関数ライブラリ内蔵だが 2026-08 の全面書き換えで採用実績なし

## Decision Outcome

Chosen option: "pulp", because 実行時 dispatch を内蔵し、stable Rust で
AVX2+FMA まで動作し、c64xN 複素ベクトルが FFT 内部に直結し、faer の
基盤として実戦検証済みであるため。

### Consequences

* 独自算術クレートは `pulp::Arch::new()` を保持し、
  `dispatch(WithSimd)` で各カーネルを ISA 別に振り分ける構造とする
* pulp の `Simd` トレイトに対してジェネリックに書くため、
  スカラフォールバックは同一ソースから自動生成される
* AVX-512 は `x86-v4` feature が nightly 依存のため当面対象外
  (AVX2+FMA が実質上限)
* 超越関数(exp/log 等)は当面スカラ libm で十分、ベクトル化が必要に
  なった段階で thermite を再評価する

### Confirmation

* 独自算術クレートで少なくとも1つの DSP カーネル(相関または
  bandpass)が pulp dispatch 経由で動作し、scalar 結果と一致すること
* 既存 golden テストの決定性が維持されること

## Implementation Plan

* 独自算術クレート(仮称 `al-math` 等)を workspace に追加し、
  pulp を唯一の外部 SIMD 依存とする
* 優先移植: FFT(#42)、bandpass(#40)、NCC/相関(#40)
* リサンプル(#37)も同基盤を利用する

## Verification

* scalar 実装と pulp dispatch 実装の出力一致をテストで確認
* ベンチで NumPy/SciPy 同等以上の速度を確認(#42 の基準: FFT 3×↑)

## Pros and Cons of the Options

### wide

* Good: 最もシンプルで成熟(465 commits)、ドキュメント率100%
* Bad: 実行時 dispatch がなく自前実装が必要。超越関数なし。
  固定幅型のみでレーン数抽象化が弱い

### pulp

* Good: 実行時 dispatch 内蔵、c64xN 複素型、stable、faer 実績、
  `#[with_simd]` proc macro で定型コード削減可
* Bad: ドキュメント率 21% で学習コストあり。`WithSimd` の
  クロージャ/trait スタイルは他クレートと異なる慣れが必要

### thermite

* Good: ポリシー設定可能なベクトル化超越関数、gather/scatter、
  compress/expand、自動微分など機能が最も豊富
* Bad: 0.2 系が 2026-08 の全面リライトで採用実績がほぼ無い
  (36 DL/月、被依存4件)。AVX-512 未実装。SPIR-V バックエンドは
  壊れていると明記

## More Information

* 比較調査の詳細は `docs/research/simd-arithmetic-crates/` を参照
* GitHub issue #42(DSPカーネル SIMD化)が本決定の実装対象
