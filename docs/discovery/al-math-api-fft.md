---
id: "DISC-034DX9JFBV7X3bMuehK48V"
type: "discovery"
status: "resolved"
title: "al-math 公開APIのバックエンド非依存化とFFT複素数表現"
created: "2026-10-11"
updated: "2026-10-11"
owners: []
relations:
  source:
    - "../../crates/al-math/src/lib.rs"
    - "../../crates/al-math/src/fft.rs"
    - "../../crates/al-locate/src/dsp.rs"
    - "../../crates/al-locate/examples/bench_dsp.rs"
    - "../../crates/al-math/tests/fft.rs"
    - "../../crates/al-math/tests/correlation_filter.rs"
    - "../../crates/al-math/examples/bench_correlation.rs"
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
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../designs/simd-dsp-foundation.md"
    - "../adr/simd-arithmetic-pulp.md"
  defers: []
  deferred-by: []
---

# al-math 公開APIのバックエンド非依存化とFFT複素数表現

## Exploration Goal

SIMD 基盤導入後の残作業調査で、`al-math` の公開 API に `pulp` の型
(`Simd`/`WithSimd`/`c64`/`F64s`/`C64s`、および `dispatch` のトレイト
境界)が露出していることが判明した。将来 pulp 以外のバックエンドへ
移行する可能性を残すため、公開面からバックエンド固有型を排除し、
FFT 境界の複素数表現をどう定義するかを確定する。

## Key Issues and Assumptions

- 確認済み(訂正): pulp 型に実際に触れる `al-math` 外のコードは
  `al-locate/src/dsp.rs`(c64 詰め替え)と
  `al-locate/examples/bench_dsp.rs` の2箇所。ただし `al-math` 自身の
  `crates/al-math/tests/fft.rs`、
  `crates/al-math/tests/correlation_filter.rs`、
  `crates/al-math/examples/bench_correlation.rs` も `al_math::c64` と
  `Fft::transform(&mut [c64])` を利用する公開 API 消費者であり、
  追従対象は実質5箇所。correlation/filter/linalg/resample の
  公開関数は既に `&[f64]` のみで完結しており、
  `al_math::dispatch`/`Simd`/`WithSimd`/`F64s`/`C64s` を参照する
  外部コードはゼロ
- 確認済み: `pulp` 依存を宣言するのは `al-math` のみ
- 確認済み: `pulp::c64` は bytemuck Pod として使える
  (`filter.rs` が `cast_slice_mut::<c64, f64>` を利用)
- 確認済み: `dsp::fft` ラッパは呼出しごとに `Vec<c64>` 確保+
  re/im 往復コピーを行い、STFT がフレームごとに通過する
- 前提: 共通表現は f64(設計方針)。固定幅ベクトル型(f32x4/f32x8 等)
  は不要とユーザー判断済み
- 前提: 単一ソース `WithSimd` カーネル + scalar 強制経路との比較という
  検証モデルを維持する

## Alternatives and Comparison

FFT 境界の複素数表現:

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| split re/im 配列(`&mut [f64]` ×2) | 呼出し側の既存データ配置と一致し pack/unpack が消える。SoA 複素演算は shuffle 不要で AoS と同等以上。バックエンド中立の純データ | カーネルの複素演算を実部演算へ分解する書き換え量が最大。bit-reversal swap が2要素分 | yes |
| クレート所有 `Complex` 型(AoS、`#[repr(C)]`) | 内部 `&mut [c64]` へのゼロコストキャストが可能。カーネル変更不要 | 呼出し側の詰め替えコストが残る。`StftMatrix` と永続テンソル(stft_re/stft_im)が re/im 分離で AoS と噛み合わない | no |
| 不透明 `Spectrum` バッファ | 内部完全隠蔽 | 値アクセスに変換経由が必要。外部利用が bench のみで旨味が薄い | no |
| 現状維持(pulp `c64` 露出のまま) | 変更不要 | バックエンド移行時に全利用箇所が連鎖変更になる | no |

ベクトル演算の幅(参考、本題の前提):

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| pulp ジェネリック `S::f64s` (現行) | 実行時 ISA 選択 + scalar 同一ソース fallback。残る未SIMD化処理(dot/energy/sub 等)は全て表現可能 | レーン数非依存で書けない shuffle/gather は向かない | yes |
| 固定幅ベクトル(wide 等) | PCM 変換・deinterleave の shuffle に効く | 別抽象の同居で表現ドリフト。対象はメモリ律速の数箇所のみ | no |

## Tentative Conclusions and Hypotheses

- confirmed: 公開 API の pulp 露出は `lib.rs` の `pub use`/`dispatch` と
  FFT 系シグネチャの `c64` に限定される。`al-locate` 2箇所と
  `al-math` 内の tests/例3箇所、実質5箇所の追従で閉じられる規模
- hypothesis: split re/im は境界コスト(詰め替え Vec 確保)を消せる
  唯一の案で、`StftMatrix`・永続テンソル・`dsp::fft` シグネチャの
  全てと整合する(ADR で確定)
- hypothesis: `dispatch`/`Simd`/`WithSimd`/`F64s`/`C64s` は外部利用が
  ないため crate 内限定化で機械的に閉じられる
- open: `RealFft` の公開要否(外部利用が bench のみ)と、split 化で
  `Fft`/`RealFft` 内部も SoA に揃える範囲 → design で確定
- open: 検証モデル(scalar 強制経路との比較)を split カーネルにどう
  引き継ぐか → design/test-spec で確定
- confirmed(実装後追記): 実装後の実測で、実使用経路
  (`dsp::fft` 経由、ncc、bandpass、welch psd、stft-equivalent)は
  pack/unpack 除去の効果で旧版と同等以上に改善した。一方、直接
  plan 利用の FFT マイクロベンチ(complex fft/rfft、それを内側で
  使う gcc_phat)は SoA 固有のメモリ uops 増により ~10% 劣後を
  確認。構造的要因のため AoS 復帰なしには解消困難な既知
  トレードオフとして記録する

## Open Questions

- `RealFft` を外部に公開し続ける必要があるか(現状の外部利用は
  bench だけ) → design 入力
- split 化を `Fft`/`RealFft` の内部表現まで広げるか、公開面の変換
  で済ませるか(パフォーマンスの主論点) → design 入力
- `bench_dsp.rs` のベンチ契約(旧実装との同一入力比較)を split API
  でも維持できるか → implementation で検証

## Promotion Candidates

- [x] spec-doc needed: 「al-math 公開 API はバックエンド固有型を含まない」
  契約と受入基準
- [x] adr-doc needed: FFT 境界の複素数表現として split re/im を採用する
  決定(AoS/不透明バッファ/現状との比較付き)

## Sources and Relations

- `docs/designs/simd-dsp-foundation.md`(pulp 型の公開集約方針の起点)
- `docs/adr/simd-arithmetic-pulp.md`(pulp 採用 ADR。本件はその公開面を
  制約する補強決定であり、採用自体の見直しではない)
- `crates/al-math/src/lib.rs`(pulp 再公開)
- `crates/al-math/src/fft.rs`(c64 を含む公開シグネチャ)
- `crates/al-locate/src/dsp.rs`、`crates/al-locate/examples/bench_dsp.rs`
  (外部の pulp 利用箇所)
- `crates/al-math/tests/fft.rs`、`crates/al-math/tests/correlation_filter.rs`、
  `crates/al-math/examples/bench_correlation.rs`(公開 API 経由の
  `c64`/FFT 利用箇所)
- セッション内調査: 未SIMD化音声処理の棚卸し、pulp 露出監査、
  固定幅ベクトルの有利性評価
