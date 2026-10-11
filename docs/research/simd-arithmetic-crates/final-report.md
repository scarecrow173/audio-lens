# Research Report: 独自算術クレートの基盤比較 — wide vs pulp vs thermite

## Scope and Assumptions

- AudioLens 独自の DSP 算術クレートの基盤として、portable SIMD クレート 3 者を比較
- 前提: stable Rust、f64 主体、実行時 CPU 検出による自動 dispatch があると望ましい、決定的動作(golden テスト)、依存追加は承認制
- 主要用途: FFT 内部、NCC/相関、bandpass(代替IIR)、リサンプル、STFT、PSD/包絡

## Executive Summary

| | wide 1.7.1 | pulp 0.22.3 | thermite 0.2.1 |
|---|---|---|---|
| **採用推奨度** | △(シンプル用途のみ) | **◎(推奨)** | ○(将来の選択肢) |
| stable で動作 | ○ | ○(AVX-512のみnightly) | ○(AVX-512未実装) |
| 実行時 dispatch | **なし**(コンパイル時のみ) | **あり** `Arch::dispatch` | **あり** `dispatch_dyn!` |
| f64 ベクトル | f64x2/4/8 | f64x2/4/8(+c64xN) | f64x2/4/8(ポリシー設定付き数学関数) |
| AVX-512 | 型のみ(intrinsicは対象外) | feature `x86-v4`(nightly) | 未実装 |
| NEON | safe_arch経由 | あり | Complete |
| 超越関数(exp/log等) | なし | libm経由(スカラ) | **ポリシー設定可能なベクトル化実装あり** |
| 複素数 | なし | **c32xN/c64xN** | なし(FloatVectorのみ) |
| メンテナンス | 活発(465 commits) | 活発(250 commits) | **0.2系は2026-08リライトで新規** |
| 実績 | 558★, 61 forks | 369★, faer の基盤 | 36DL/月, 被依存4件 |
| ライセンス | Zlib/Apache/MIT | MIT | MIT/Apache |

## Key Findings

### wide — シンプルだが dispatch なし

固定幅型(f64x4 等)を直接書く最も直感的なクレート。safe_arch 経由で x86_64/wasm/NEON を intrinsic 化。ただし**実行時 CPU 検出・dispatch は自前**(`multiversion` or 自前検出が別途必要)。超越関数なし。ドキュメント率100%、最成熟。

向いている用途: 固定幅(4または8レーン)で十分、コンパイルターゲットを分けるか scalar フォールバックで妥協する場合。

### pulp — faer の実績ある dispatch 基盤

`Arch::new()` で実行時検出 → `dispatch(WithSimd)` でジェネリック関数を最適な ISA に振り分ける。`Simd` トレイトに対して書き、`S::as_mut_simd_f64s` 等で head/tail を処理。**c64xN(複素)が標準装備**で FFT 内部に有利。faer で大規模に利用されている実績。AVX-512 は `x86-v4` feature = nightly。

向いている用途: **独自算術クレートの基盤として最も適合**。実行時 dispatch を自分で書く必要がなく、stable で AVX2+FMA まで使える。

### thermite — 機能最豊富だが新しい

2026-08 の全面リライトで復活。trait ラダー + `dispatch_dyn!` + **ポリシー設定可能な超越関数**(exp/log/sin/cos を UltraPerformance〜Reference で選択、gather/scatter、compress/expand、interleave、自動微分 Dual、Compensated double-double)。機能は圧倒的だが **0.2.x が2ヶ月前**、被依存4件のみ、SPIR-V は壊れていると明記。AVX-512 も未実装。

向いている用途: 超越関数をベクトル化したい場合(ただし現行の解析パスではスカラ libm で十分)。長期的に育てば最有力。

## Evidence Matrix

| Claim | Support | Source | Confidence |
|---|---|---|---|
| pulp が実行時 dispatch を持つ | S2, S8 | docs.rs + GitHub | High |
| pulp の AVX-512 は nightly | S3 | docs.rs features | High |
| thermite が AVX-512 未実装 | S4, S9 | docs.rs | High |
| thermite がポリシー設定可能な超越関数を持つ | S4 | docs.rs | High |
| wide が dispatch 機構を持たない | S1, S7 | docs.rs + GitHub README | High |
| thermite の採用実績が低い | S5, S6 | lib.rs | Medium(lib.rs メタデータ) |
| 3 者とも f64xN を持つ | S1, S2, S4 | docs.rs | High |

## Analysis

### AudioLens 用途との適合

独自算術クレートの基盤として必要なのは:

1. **実行時 dispatch** — 同一バイナリで AVX2 機/SSE2 機をサポート。wide は自前実装が必要。pulp/thermite は内蔵
2. **f64 ベクトル** — 3 者ともあり
3. **FMA / mul_add** — 3 者とも使用可能
4. **複素数** — FFT 内部で有利。pulp のみ c64xN 標準
5. **超越関数** — 包絡の log、PSD の dB 化など。thermite のみベクトル化実装あり(ただし現行パスではそれほど熱くない)
6. **安定性・実績** — pulp(faer)> wide(成熟)> thermite(新規)

### リスク評価

| リスク | wide | pulp | thermite |
|---|---|---|---|
| dispatch 実装コスト | 高(自前) | 低 | 低 |
| API 学習コスト | 低 | 中(WithSimd trait) | 中(dispatch 属性必須) |
| 将来性 | 安定 | faer に牽引され安定 | 未知数(復活直後) |
| 機能不足リスク | 中(math なし) | 低 | 低 |

## Gaps / Uncertainties

- thermite の stable Rust MSRV(Rust 2024 edition = 比較的新しい toolchain 必要)
- 各クレートの実性能差 — 抽象化レイヤーの違いが熱いループで差になるかは実測が必要
- pulp のドキュメント率 21% — 内部実装の把握にソース読みが必要な可能性

## Recommendations

**第一候補: pulp**
- 実行時 dispatch が内蔵、stable、c64xN で FFT に有利、faer の実績
- AudioLens の用途(F64 SIMD カーネル)に構造が最適
- 独自算術クレート内で `Arch::new()` を保持し `dispatch` で各カーネルを振り分ける形が自然

**第二候補: thermite**
- 超越関数のベクトル化が真に必要になった段階で再評価
- 現時点では成熟リスクが高い(0.2.1 が 2026-08、採用実績最小)

**wide は「SIMD化された一関数だけ欲しい」用途に限定**
- dispatch 機構がないため独自算術クレートの基盤としては追加作業が多い

## Source List

- S1: https://docs.rs/wide/latest/wide/ (wide 1.7.1)
- S2: https://docs.rs/pulp/latest/pulp/ (pulp 0.22.3)
- S3: https://docs.rs/crate/pulp/latest/features (pulp feature flags)
- S4: https://docs.rs/thermite/latest/thermite/ (thermite 0.2.1)
- S5: https://docs.rs/crate/thermite/latest (crate metadata)
- S6: https://lib.rs/crates/thermite (adoption metrics)
- S7: https://github.com/Lokathor/wide (repo activity)
- S8: https://github.com/sarah-quinones/pulp (repo activity)
- S9: thermite docs.rs Backends 表 (S4 内)
