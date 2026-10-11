---
id: "PLAN-034DXViZtHIbCiHSxsn1O0"
type: "plan"
status: "approved"
title: "al-math 公開 API 中立化と split re/im FFT 境界"
created: "2026-10-11"
updated: "2026-10-11"
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
    - "docs/specs/al-math-api-boundary.md"
  implemented-by:
    - "docs/tasks/al-math-split-re-im-internals.md"
    - "docs/tasks/al-math-split-re-im-tests.md"
    - "docs/tasks/al-locate-split-re-im-fft.md"
    - "docs/tasks/bench-correlation-split-re-im.md"
    - "docs/tasks/al-math-api-pub-crate.md"
    - "docs/tasks/al-math-api-boundary-verification.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/al-math-api-fft.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/al-math-api-boundary.md"
    - "docs/designs/al-math-api-boundary.md"
    - "docs/adr/al-math-api-fft-split-re-im.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/al-math-api-boundary.md"
  references: []
  defers: []
  deferred-by: []
---

# al-math 公開 API 中立化と split re/im FFT 境界 Implementation Plan

## Goal

`al-math` の公開 API をバックエンド中立の型(f64 slice とクレート
所有の構造体)だけで構成し、pulp 由来の `Simd`/`WithSimd`/`c64`/
`F64s`/`C64s`/`dispatch` を crate 内限定に降格する。あわせて
`Fft`/`RealFft` の複素数境界を split re/im(`&mut [f64]` ペア)に
変更し、FFT カーネル内部を SoA 複素演算に書き換えて、`dsp::fft` が
呼出しごとに行う `Vec<c64>` 確保と往復コピーを除去する。

## Scope

単一の実装ストリームとして扱う。公開シグネチャの変更と全利用箇所
(5 箇所)の追従は同一クリティカルパス上にあり、途中状態で
ワークスペースがコンパイル不能になるため分割しない。特に `fft.rs`
の `pub(crate)` `forward_with_simd`/`inverse_with_simd` を同一
lib ターゲット内の `correlation.rs`/`filter.rs` が `Vec<c64>`
バッファで呼んでいるため、`fft.rs` 単独の split 化では lib が
コンパイル不能になり検証に到達しない。`fft.rs`・`correlation.rs`・
`filter.rs` の内部 split 化は分割不能な単一スライス(S1)として
扱う。

- In scope: `lib.rs` 公開面の `pub(crate)` 化、`fft.rs` の
  split re/im API + SoA カーネル化、`correlation.rs`/`filter.rs` の
  内部スペクトラムバッファの `Vec<f64>` ペア化、`dsp.rs`/
  `bench_dsp.rs`/`bench_correlation.rs`/`tests` 2 件の追従
- Out of scope(design の Out of Scope を継承): pulp の置き換え、
  ベクトル演算プリミティブの公開、`dsp::fft` ラッパの再設計・
  `StftMatrix` の `RealFft` 直結、直接 FIR の FFT 畳み込み化・
  lag 走査の `ncc` 置換、AVX-512・固定幅ベクトル型・ベクトル超越関数

## Upstream Documents

- `docs/specs/al-math-api-boundary.md`(approved): 受入基準 —
  公開 API への pulp 型非露出、`al-math` 以外の pulp 非依存、
  scalar/dispatch 一致維持、独立参照比較の保持、golden 不変
- `docs/adr/al-math-api-fft-split-re-im.md`(accepted): split re/im
  採用の決定と影響範囲(追従 5 箇所)
- `docs/discovery/al-math-api-fft.md`: pulp 露出監査・表現案比較の
  根拠資料

## Design Inputs

- `docs/designs/al-math-api-boundary.md`(approved): 本計画の正本。
  公開シグネチャ、内部表現(twiddles/roots/scratch の pad 付き
  flat `Vec<f64>` 化)、FMA 方針(`mul_add_f64s`/
  `negate_mul_add_f64s`)、数値同一性の制約(誤差閾値
  `1e-10 * (1 + |b|)` 相対の維持)を規定
- 分解式(設計より): `mul(conj(a), b)` = `(ar·br + ai·bi, ar·bi − ai·br)`、
  `mul_c64s((x),(w))` = `(xr·wr − xi·wi, xr·wi + xi·wr)`、
  `abs2` = `re² + im²`、`conj` = 虚部の符号反転

## File Map

| Path | Responsibility | Notes |
|------|----------------|-------|
| `crates/al-math/src/lib.rs` | `pub use`/`type`/`dispatch` の `pub(crate)` 化、crate doc の intra-doc link 修正 | 公開面閉じ込めは S5 で実施 |
| `crates/al-math/src/fft.rs` | `Fft`/`RealFft` の split re/im API と SoA カーネル(twiddles/scratch/roots の pad 付き flat `Vec<f64>` 格納、`stage`/`butterfly`/`recombine_pair`/`recombine` の実部演算化、大きな n の thread_local workspace 経路) | 変更の主本体。S1(統合スライス) |
| `crates/al-math/src/correlation.rs` | スペクトラムバッファ `a`/`b` の re/im ペア化、`cross_spectrum` の実部演算化、PHAT `normalize` の同一 energy による re/im 適用、`cast_slice_mut` 2 箇所の除去 | S1(統合スライス)。`forward_with_simd`/`inverse_with_simd` の呼出し追従を含む |
| `crates/al-math/src/filter.rs` | `spectrum`/`masked`/`magnitudes` の re/im ペア化、`cast_slice_mut` 1 箇所の除去、`forward_with_simd`/`inverse_with_simd` 呼出し追従 | S1(統合スライス) |
| `crates/al-locate/src/dsp.rs` | `dsp::fft` 内部の `Vec<c64>` 詰め替え除去、`Fft::transform(re, im, inverse)` 直接呼出し | シグネチャ `fft(re, im, inverse)` は維持。S3 |
| `crates/al-locate/examples/bench_dsp.rs` | `complex fft`/`rfft` ベンチ腕の split API 追従 | S3 |
| `crates/al-math/tests/fft.rs` | 独立参照(`reference()` radix-2、naive DFT)を split 形に書き換えて保持、契約テストの split API 追従 | S2。`al_math::c64` は使わずタプル/re/im 演算で記述 |
| `crates/al-math/tests/correlation_filter.rs` | `complex_spectrum` ヘルパと契約テストの split 化 | S2 |
| `crates/al-math/examples/bench_correlation.rs` | `previous_fft` 比較腕の削除と追従 | S4。c64 dispatch 版は公開 API で表現不能のため削除(設計許容) |

## Implementation Sequence

| Slice | 内容 | Depends on | Blocks |
|-------|------|------------|--------|
| S1 | `al-math` 内部の split re/im 化(`fft.rs` SoA カーネル + `correlation.rs`/`filter.rs` スペクトラムバッファ) | — | S2, S3, S4 |
| S2 | `al-math` tests 2 件の split 化(独立参照の保持を含む) | S1 | S5, S6 |
| S3 | `al-locate` `dsp.rs`/`bench_dsp.rs` 追従 | S1 | S5, S6 |
| S4 | `bench_correlation.rs` 追従 | S1 | S5, S6 |
| S5 | `lib.rs` 公開面の `pub(crate)` 化 | S1〜S4(外部の `al_math::c64` 等参照が全て消えた後) | S6 |
| S6 | 全体検証・性能比較 | S1〜S5 | — |

クリティカルパス: S1 → S2 → S5 → S6。S1 は同一 lib ターゲット内の
`correlation.rs`/`filter.rs` が `fft.rs` の `pub(crate)`
`forward_with_simd`/`inverse_with_simd` を `Vec<c64>` で呼ぶため
分割不能な統合スライスであり、S2・S3・S4 はいずれも S1 のみに
依存する。S2 は統合スライス直後の最優先スライスとし、scalar/dispatch
一致テストでは検出不能な系統誤りの blind window を独立参照比較で
早期に閉じる。S3・S4 は統合スライス完了後に並行可能
(担当ファイルが独立)。

## Task Breakdown

### S1: `al-math` 内部の split re/im 化(`fft.rs` + `correlation.rs` + `filter.rs`)

同一 lib ターゲット内で `fft.rs` の `pub(crate)`
`forward_with_simd`/`inverse_with_simd` を `correlation.rs`/
`filter.rs` が `Vec<c64>` バッファで呼ぶため、3 ファイルを分割せず
1 スライスで書き換える。

1. `fft.rs`: `roots()` の戻り値を `(Vec<f64>, Vec<f64>)` の re/im
   ペアに変更し、`Fft::new` の twiddle 生成ループで stage ごとに
   `[w1 re | pad | w1 im | pad | w2 re | pad | w2 im | pad]` へ展開
   する単一 flat `twiddles: Vec<f64>` フィールドに書き換える
   (`TW_PAD`/`tw_segment`/`tw_offset` で副表を切り出す)
2. `fft.rs`: `Fft::transform` を `(&self, re: &mut [f64],
   im: &mut [f64], inverse: bool)` に変更し、
   `re.len() == im.len() == self.n` を assert する。`Transform`
   カーネル構造体と内部 `transform()` 関数を re/im 引数に追従させる
3. `fft.rs`: `transform()` 内の bit-reversal 並び替え
   (`values.swap` 2 箇所: 単純ループと n>=1024 のタイル化)を
   `re.swap(i,j)` + `im.swap(i,j)` に書き換える(タイル化ロジック
   自体は不変)。inverse の 1/N スケールを re/im 各配列への
   `mul_f64s` に書き換える。n ≥ `WS_MIN`(16384)は thread_local
   `FFT_WORK` 上で out-of-place 先頭 stage(`stage_out`)+ 残り
   stage(`butterfly_from`)を実行し、digit-reversed 置換と 1/N
   スケールを `gather_permute` の書き戻しへ畳み込む workspace 経路
   を新設する
4. `fft.rs`: `butterfly`/`stage` を re/im 引数 + pad 付き flat
   twiddles に書き換える。`mul_c64s` は
   `(xr·wr − xi·wi, xr·wi + xi·wr)`、`conj_c64s` は虚部 negate、
   rotate 乗算は実部演算に分解し、FMA 融合は `mul_add_f64s`/
   `negate_mul_add_f64s` で現行の丸め形を可能な範囲で再現する。
   `cmul` の re 成分は `negate_mul_add_f64s(ai, bi, mul(ar, br))`
   とし、pulp `mul_c64s` の `fma(ar,br,−(ai·bi))` とは融合側が逆で
   ~1ulp 差が生じ得るが、1 演算削減の性能優先で意図的に採用する。
   `len == 2` 経路と scalar tail も実部演算にする
5. `fft.rs`: `RealFft` の `roots`/`scratch` を pad 付き flat
   `Vec<f64>`(`roots` は `[re | pad | im]`、`scratch` は
   `[re(m) | pad | im(m)]`)に変更し、`forward`/`inverse` を split
   シグネチャ(`forward(input, out_re, out_im)` /
   `inverse(in_re, in_im, output)`)に変更する。現行の長さ契約
   (input=n、spectrum=n/2+1)と assert を維持し、
   `in_re`/`in_im` の同一スライス共有禁止を debug assert で
   明示する
6. `fft.rs`: `forward_with_simd`/`inverse_with_simd` を同形の
   split シグネチャに変更(`pub(crate)` と選択済み ISA の複合
   カーネル共有は維持)。`RealBuffer`/`SpectrumBuffer`/
   `RealTransform` を re/im ペア構造に書き換える
7. `fft.rs`: `RealTransform::with_simd` 本体を実部演算に書き換える:
   2点詰め込み/展開は `scratch_re[i]=x[2i]`・`scratch_im[i]=x[2i+1]`
   のストライドコピー、DC/Nyquist 組立てと conj 配置は re/im 演算、
   `recombine_pair`/`recombine` は実部演算化。`low`/`high` は re/im
   両配列で split・prefill(conj 充填)・reverse を対で行い、
   `high.reverse()` は両配列に適用する
8. `fft.rs` 内の `use crate::{...}` から `c64` を除去し、モジュール内
   テスト `scalar_and_dispatch_share_the_kernel` を split API 形に
   書き換える
9. `correlation.rs`: `a`/`b` バッファを `Vec<f64>` の re/im ペアにし、
   `forward_with_simd`/`inverse_with_simd` 呼出しを split 引数に
   追従させる
10. `correlation.rs`: `cross_spectrum` を実部演算に書き換える
    (`mul(conj(a), b)` = `(ar·br + ai·bi, ar·bi − ai·br)`。
    FMA は fft.rs と同じ方針)
11. `correlation.rs`: PHAT 分岐の `magnitude` を `Vec<f64>` の
    energy 配列(abs2 = `re² + im²`)にし、`bytemuck::cast_slice_mut`
    2 箇所を除去して `normalize` を同一 energy 配列で re/im 各配列に
    適用する
12. `correlation.rs`: `use` から `c64` を除去する
13. `filter.rs`: `spectrum`/`masked` を re/im ペアにし、band ループの
    `fill`/`copy_from_slice` を両配列に対して行う。
    `forward_with_simd`/`inverse_with_simd` 呼出しを追従させる
14. `filter.rs`: `spectral_envelope` の `magnitudes` を `Vec<f64>` にし、
    `abs2_c64s` を `re² + im²` の実部演算に、`cast_slice_mut` 1 箇所を
    除去する(sqrt 適用と区間平均の流れは現行を維持)
15. `filter.rs`: `use` から `c64` を除去する
16. 検証: `mise exec -- cargo test -p al-math --lib` が合格
    (FFT 系と correlation/filter の scalar/dispatch 一致テストを
    含む。この時点では integration test/example/下流 crate の
    ビルドは壊れていてよい。`--lib` で lib ターゲットのみ確認する)

### S2: `al-math` tests 2 件の split 化(統合スライス直後の最優先)

S1 完了直後に実施する最優先スライス。S1 後は scalar/dispatch 一致
テストしか通らず、カーネル書き換えミスの系統誤りを検出する独立参照
比較が blind window になるため、S3・S4 に先行させて閉じる。

1. `tests/fft.rs`: `reference()`(radix-2 独立参照)を re/im ペア
   操作に書き換え、`close()` を re/im ペア比較にする。
   `al_math::c64` は使わず `(f64, f64)` タプルまたは re/im 演算で
   記述する
2. `tests/fft.rs`: 全テスト(`complex_matches_legacy_forward_inverse_
   and_roundtrip`、`real_half_spectrum_matches_complex_and_
   roundtrips`、`rejects_*` 4 件、`matches_independent_dft_and_is_
   repeatable`、`real_inverse_ignores_endpoint_imaginary_parts`、
   `immutable_complex_plan_can_be_shared`)を split API に追従させる。
   naive DFT 比較は削除せず re/im 累積形に書き換えて保持する
3. `tests/correlation_filter.rs`: `complex_spectrum` ヘルパを
   re/im ペア返却に書き換え、`real_kernels_preserve_previous_
   complex_spectrum_contract` 内の複素演算(mask・conj 積・
   norm_sqr)を実部演算に追従させる
4. チェック: `power` ループ範囲(`tests/fft.rs` の `0..=16`、`fft.rs`
   モジュール内テストの `0..=10`)とエッジ条件テスト(n=1、
   DC/Nyquist、`real_inverse_ignores_endpoint_imaginary_parts`)を
   維持する
5. 検証: `mise exec -- cargo test -p al-math --tests` が合格
   (`bench_correlation.rs` の追従は S4 のため、この時点では
   example のビルドは壊れていてよい)

### S3: `al-locate` `dsp.rs`/`bench_dsp.rs` 追従

1. `dsp.rs`: `fft(re, im, inverse)` 内の `Vec<c64>` 詰め替えを削除し、
   `al_math::fft::Fft::new(re.len())` の `transform(re, im, inverse)` を
   直接呼ぶ。`al_math::c64` 参照を除去する
2. `bench_dsp.rs`: `complex fft 4M` 腕を re/im バッファへの直接変換に、
   `rfft 4M` 腕の `spectrum` を re/im ペアに書き換え、
   `al_math::c64` 参照を除去する
3. 検証: `mise exec -- cargo test -p al-locate` が合格し、
   `mise exec -- cargo build -p al-locate --examples` が成功する

### S4: `bench_correlation.rs` 追従

1. `examples/bench_correlation.rs`: `previous_fft` 比較腕を削除する
   (c64 dispatch 版 FFT は公開 API で表現不能になるため。設計で
   削除または書き換えが許容されており、削除を採る)。
   `legacy_bandpass` の `previous_fft` 使用分岐と `previous`/
   `previous_many` ベンチ、speedup 表示を整理し、残る腕
   (`original_fft` 比較・現行 `filter`/`correlation`)を動作させる
2. 検証: `mise exec -- cargo test -p al-math` が合格し、
   `mise exec -- cargo build -p al-math --examples` が成功する

### S5: `lib.rs` 公開面の `pub(crate)` 化

1. `pub use pulp::{Simd, WithSimd, c64}` を `pub(crate) use` に、
   `pub type F64s`/`C64s` を `pub(crate) type` に、`pub fn dispatch`
   を `pub(crate) fn` に降格する
2. クレート doc の `[`Simd`]`/`[`WithSimd`]`/`[`dispatch`]`
   intra-doc link をコード表記に書き換える
3. 検証: `mise exec -- cargo build --workspace` が成功することを
   第一ゲートとし(brace import や複数行 `use` など grep が取りこぼす
   参照もコンパイルで検出する)、`mise exec -- cargo doc -p al-math`
   が warning なく成功し、
   `rg 'al_math::[\w:{}, ]*\b(c64|Simd|WithSimd|F64s|C64s|dispatch)\b'
   crates/` と `rg 'pulp' crates/ --glob '!al-math/**'` のヒットが
   0 件(al-math 内部の `pulp`/`crate::Simd` 等の参照は許容)

### S6: 全体検証

1. `mise exec -- cargo test -p al-math -p al-locate -p al-analysis`
   が合格することを確認する
2. `cargo test -p al-analysis`(golden 系テストを含む)で分析出力が
   不変であることを確認する
3. 変更前コミットを別 worktree/別ビルドした `bench_dsp` と同一入力で
   FFT 系カーネル(`fft 4M`、`complex fft 4M`、`rfft 4M`、
   `stft-equivalent`)を比較する。比較リストに `bandpass`/`ncc`/
   `gcc_phat` 腕を加え、旧コミット別ビルドの `bench_correlation`
   binary でも同一入力比較する。実使用経路(`dsp::fft` 経由の
   ncc/bandpass/welch psd/stft-equivalent)は同等以上であることを
   確認し、直接 plan 利用のマイクロベンチ(complex fft/rfft、
   gcc_phat)に残る ~10% 劣後は SoA 構造要因の既知トレードオフ
   として記録する
4. `al-math` 以外のクレートの `Cargo.toml` に `pulp` 依存がないことを
   再確認する(現状確認済み、変更なしを維持)

## Dependencies and Risks

依存関係は Implementation Sequence の表の通り。外部依存の追加はない
(`pulp` は `al-math` の内部依存として継続)。

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | 複素演算の分解で丸め特性が変わり scalar/dispatch 一致閾値(1e-10 相対)を超える | med | med | `mul_add_f64s`/`negate_mul_add_f64s` で現行 `mul_c64s` の FMA 融合形を再現。関数単位で置き換えて都度 `--lib` テスト |
| 2 | `stage`/`recombine_pair`/`recombine` の書き換えミスが scalar/dispatch 両経路に共通する系統誤りになる(一致テストでは検出不能) | med | high | `tests/fft.rs` の独立参照(radix-2 `reference()`・naive DFT)との直接比較を split API で維持し、tests 書き換えスライス(S2)を統合スライス直後の最優先スライスとして、スライス直後に独立参照比較を通して blind window を最小化する |
| 3 | 公開 API の破壊的変更による利用側の連鎖修正 | high | low | 外部利用は `dsp.rs`/`bench_dsp.rs`/tests 2 件/`bench_correlation.rs` の 5 箇所と調査済み。pre-release 方針で互換層は設けない |
| 4 | bit-reversal swap の 2 ストア化・SoA 化による性能回帰 | low | med | `bench_dsp`/`bench_correlation` で旧コミット別ビルドと同一入力比較。回帰時はカーネル分解を見直す(実測で直接 plan 利用のマイクロベンチに ~10% 劣後を確認済み。SoA 構造要因の既知トレードオフとして受け入れ) |
| 5 | `RealFft::inverse` の `in_re`/`in_im` 同一スライス共有という契約違反 | low | med | debug assert + ドキュメントで契約を明示(設計準拠) |
| 6 | データ損失リスク | low | low | 本変更はコードのみでデータ変換・永続形式の変更を含まない |

### Rollback Strategy

- 対象: 全スライスがコード変更のみで、不可逆なデータ変換はない。
  公開 API 変更は pre-release 方針により互換層・deprecation を設けない
- Trigger: S6 の検証失敗(テスト不合格・参照比較不一致・性能回帰が
  許容範囲を超える)で原因切り分けに時間を要する場合
- Procedure: 本変更を構成するコミット群を `git revert` で取り消す
- Verification: revert 後に `mise exec -- cargo test -p al-math
  -p al-locate -p al-analysis` が合格すること

## Verification

| Step | Verification command or criteria | Pass condition |
|------|----------------------------------|----------------|
| S1 | `mise exec -- cargo test -p al-math --lib` | fft/correlation/filter 含む全 lib テスト合格 |
| S2 | `mise exec -- cargo test -p al-math --tests` | 独立参照(radix-2/naive DFT)比較・round trip・DC/Nyquist・panic 契約テスト全合格 |
| S3 | `mise exec -- cargo test -p al-locate` + `cargo build -p al-locate --examples` | 合格・ビルド成功 |
| S4 | `mise exec -- cargo test -p al-math` + `cargo build -p al-math --examples` | `bench_correlation` を含む全ターゲット合格 |
| S5 | `cargo build --workspace` 成功(第一ゲート) + `cargo doc -p al-math` warning なし + grep 監査 | `al_math::c64` 等の外部参照・al-math 外の `pulp` 参照 0 件 |
| S6 | `mise exec -- cargo test -p al-math -p al-locate -p al-analysis` | 合格。golden 出力不変 |
| S6 | `bench_dsp`/`bench_correlation` 旧コミット別ビルドとの同一入力比較 | 実使用経路(`dsp::fft`/`ncc`/`bandpass`/`gcc_phat`/`welch psd`/`stft-equivalent`)が旧版と同等以上。直接 plan 利用の FFT マイクロベンチの ~10% 劣後は既知トレードオフとして記録 |

plan レベルの検証は上表の通り。振る舞いレベルの契約(誤差閾値・
境界条件・不変量)は後続フェーズで `test-spec` 文書として切り出し、
`relations.verified-by` で本 plan にリンクする。

## Open Questions / Gaps

- `RealFft::inverse` の `in_re`/`in_im` 同一スライス検査は
  debug assert で実施する(設計の「必要に応じ debug assert」に
  対する採用判断)。release ビルドでは検査しない
- `bench_dsp`/`bench_correlation` の旧版比較は「変更前コミットを
  別ビルド」する運用とし、比較手順は実装時に記録する
  (設計・ADR の確認条件そのまま)

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: delegated / subagent-capable implementation
  (S3・S4 は統合スライス完了後に並行実行可能)
- Independent work streams: S3(`al-locate` 側追従)と
  S4(`bench_correlation.rs` 追従)は統合スライス(S1)完了後に
  並行可能。S2(`al-math` tests 追従)は S1 直後の最優先スライスと
  して独立参照比較を復帰させる。S1 → S2 → S5 → S6 は直列
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints at each slice boundary (S1〜S6).
- If declined or unavailable: execute inline using the same dependency
  graph and verification matrix.

## Review Handoff

- 前提: 既存テストの誤差閾値(`1e-10 * (1 + |b|)` 相対)を変更しない。
  FMA 分解で閾値内に収まらない場合は分解形の見直しが必要であり、
  閾値緩和は本 plan のスコープ外
- 前提: `bench_correlation.rs` の `previous_fft` 腕は「削除」を
  採用した(設計は削除または書き換えを許容)。レビューで書き換え
  判断に変える場合は S4-1 を修正する
- 未解決依存なし。上流は spec(approved)・design(approved)・
  ADR(accepted)が揃っている
- test-spec / task 文書は本 plan のレビュー後に別フェーズで作成し、
  `relations.verified-by` / `implemented-by` をその時点で更新する
- レビュー参照先: `docs/designs/al-math-api-boundary.md`(正本)、
  `docs/specs/al-math-api-boundary.md`(受入基準)、
  `docs/adr/al-math-api-fft-split-re-im.md`(決定根拠)
