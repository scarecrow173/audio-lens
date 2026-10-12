---
id: "DESIGN-034DXiPrxAy7If9Jsw6gop"
type: "design"
status: "approved"
title: "al-math 公開 API のバックエンド中立化と split re/im FFT"
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
    - "../specs/al-math-api-boundary.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/al-math-api-boundary.md"
    - "../adr/al-math-api-fft-split-re-im.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers:
    - "../discovery/al-math-simd-reduction.md"
  deferred-by: []
---

# al-math 公開 API のバックエンド中立化と split re/im FFT

## Context

`al-math` のカーネルは pulp dispatch で SIMD 化済みだが、公開面に
pulp 型が露出している(`lib.rs` の `pub use` と `Fft`/`RealFft` の
`c64` シグネチャ)。ADR で「公開 API はバックエンド中立の型のみ、
FFT 境界は split re/im」と決定された。本設計はその具体化である。

現在の内部構造:

- `Fft` は `twiddles: Vec<c64>` を持ち、`transform(&mut [c64])` が
  `butterfly`/`stage`(radix-4 融合、in-place)と bit-reversal
  並び替え、inverse スケールを実行する
- `RealFft` は `roots: Vec<c64>` と `scratch: Vec<c64>` を持ち、
  forward は実数→2点詰め込み→半長 FFT→`recombine_pair` で
  非負周波数 N/2+1 bins を組み立て、inverse はその逆
- `forward_with_simd`/`inverse_with_simd` は `pub(crate)` で、
  `correlation` と `filter` が `Vec<c64>` のスペクトラムバッファを
  共有して `mul_c64s`/`conj_c64s`/`abs2_c64s` で複素演算している

## Scope

### In Scope

- `lib.rs` 公開面から `Simd`/`WithSimd`/`c64`/`F64s`/`C64s`/
  `dispatch` を除去し、crate 内限定(`pub(crate)` または private)に
  する
- `Fft::transform` と `RealFft` 公開メソッドのシグネチャを
  split re/im(`&mut [f64]` ペア)に変更する
- FFT カーネル内部を SoA(re/im 分離)の複素演算に書き換える。
  twiddle/scratch も re/im 副表を持つ pad 付き flat `Vec<f64>`
  格納にする(キャッシュセット衝突回避のため)
- `forward_with_simd`/`inverse_with_simd`(`pub(crate)`)も
  split シグネチャにし、`correlation`/`filter` の内部スペクトラム
  バッファを re/im ペアに追従させる
- `dsp::fft`(`al-locate`)と `bench_dsp.rs` を新 API に追従させ、
  pack/unpack を除去する
- `al-math` 内の公開 API 消費者(`tests/fft.rs`、
  `tests/correlation_filter.rs`、`examples/bench_correlation.rs`)を
  split API に追従させる。`tests/fft.rs` の独立参照(`reference()`
  radix-2 と naive DFT との比較)は削除せず、split API 形に
  書き換えて保持する

### Out of Scope

- pulp の置き換え(内部依存として継続)
- ベクトル演算プリミティブ(dot/sum_sq 等)の追加 — 別 spec
- `dsp::fft` ラッパ自体の再設計・`StftMatrix` の `RealFft` 直結 —
  本変更の後続設計
- 直接 FIR の FFT 畳み込み化・lag 走査の `ncc` 置換
- AVX-512・固定幅ベクトル型・ベクトル超越関数

## Components and Boundaries

- `crates/al-math/src/lib.rs`: `pulp` への `use`/`type` エイリアス/
  `dispatch` を `pub(crate)` に降格する。公開モジュール名
  (`correlation`/`fft`/`filter`/`linalg`/`resample`)と公開関数の
  シグネチャは f64 slice と構造体のまま維持する。private 化に伴い、
  クレート doc の `[`Simd`]`/`[`WithSimd`]`/`[`dispatch`]` の
  intra-doc link はコード表記へ書き換える
- `crates/al-math/src/fft.rs`:
  - `Fft::transform(&self, re: &mut [f64], im: &mut [f64], inverse: bool)`
    — 長さ一致と n 一致を assert
  - `RealFft::forward(&mut self, input: &[f64], out_re: &mut [f64],
    out_im: &mut [f64])` / `inverse(&mut self, in_re: &[f64],
    in_im: &[f64], output: &mut [f64])` — 現行の長さ契約
    (input=n, spectrum=n/2+1)を維持。`in_re`/`in_im` は同一スライスを
    共有してはならない(契約。必要に応じ debug assert)
  - `forward_with_simd`/`inverse_with_simd` は同形の split
    シグネチャで `pub(crate)` のまま。複合カーネルが一度選択した
    ISA を共有する構造は維持する
- `crates/al-math/src/correlation.rs`/`filter.rs`: スペクトラム
  バッファを `Vec<f64>` ペアにし、`mul_c64s`/`conj_c64s`/
  `abs2_c64s` 相当を実部演算で書き換える
  - `mul(conj(a), b)`: `(ar·br + ai·bi, ar·bi − ai·br)`
  - `abs2`: `re² + im²`
  - `bytemuck::cast_slice_mut` による flat 2N 配列化(3箇所)を除去し、
    PHAT の `normalize` は re/im 各配列に同一 energy 配列で適用する。
    `masked`/`magnitudes` バッファも re/im ペア化する
- `crates/al-locate/src/dsp.rs`: `dsp::fft(re, im, inverse)` の
  シグネチャは維持し、内部で `Fft::transform(re, im, inverse)` を
  直接呼ぶ(`Vec<c64>` 詰め替えを削除)
- `crates/al-locate/examples/bench_dsp.rs`: split API に追従
- `crates/al-math/tests/fft.rs`/`tests/correlation_filter.rs`/
  `examples/bench_correlation.rs`: 公開 API 経由の `c64`/FFT 利用を
  split API へ追従。`bench_correlation.rs` の `previous_fft` 比較腕は
  c64 dispatch 版 FFT の再現であり公開 API では表現不能になるため、
  削除または split API での書き換えを行う

## Data and Control Flow

- `Fft` の内部表現: `twiddles: Vec<c64>` → pad 付きの単一 flat
  `Vec<f64>`。stage ごとに `[w1 re | pad | w1 im | pad | w2 re | pad |
  w2 im | pad]` を連結し、`TW_PAD`/`tw_segment`/`tw_offset` で副表を
  切り出す(段ごとの `roots()` 生成は re/im ペアのまま、格納時に
  flat へ展開)。split 化で同時アクセスが 8 データ + 4 twiddle
  ストリームに増え、4K 境界で揃う quarter stride が同一キャッシュ
  セットへ集中するため、pad で副表を別セットへずらす
- `transform` の並び替え: n < 16384 は従来通り in-place で
  `re.swap(i,j)` + `im.swap(i,j)` のペア交換(実装は 4K
  エイリアシング回避のため両配列のロードを先に発行する
  `swap_pair`)。bit-reversal タイル化のロジックは不変
- n ≥ 16384 は thread_local workspace(`FFT_WORK`、`2n + WS_SKEW`
  要素、im 部を `WS_SKEW` で re からずらす)上で、入力配列から
  直接読む out-of-place 先頭 stage(`stage_out`)+ 残り stage
  (`butterfly_from`)を実行し、DIF 出力の digit-reversed 置換を
  `gather_permute` の sequential write + gather read で呼出し側へ
  書き戻す。呼出し側の re/im が同一ページオフセットに揃うと大きな
  len の quarter stride が 4K の倍数となり 8 本のデータストリームが
  同一 L1/L2 セットへ集中するため、skew した配置を保証できる
  workspace 経路とする
- `stage` の radix-4 複素演算を f64s 演算で分解する。
  `mul_c64s((x),(w))` は `(xr·wr − xi·wi, xr·wi + xi·wr)`、
  `conj_c64s` は虚部の符号反転として実部演算で表現する。
  SIMD 演算は `simd.mul_f64s`/`add`/`sub` と FMA 融合のための
  `mul_add_f64s`/`negate_mul_add_f64s` の組合せになり、
  c64 レーン shuffle は発生しない
- inverse の 1/N スケールは、in-place 経路では re/im 各配列を
  `mul_f64s` で処理し、workspace 経路では `gather_permute` の
  書き戻しへ畳み込む
- `RealFft` の `roots`/`scratch` も pad 付きの単一 flat `Vec<f64>`
  とし、`roots` は組立て係数 `(0.5·im, −0.5·re)` を
  `[re 副表 | pad | im 副表]`、`scratch` は `[re(m) | pad | im(m)]`
  のレイアウトで保持する。`recombine_pair`/`recombine` を実部演算に
  書き換える。2点詰め込み/展開(`c64::new(pair[0], pair[1])` 相当)
  は scratch の re/im 副表への `sre[i]=x[2i]`、`sim[i]=x[2i+1]` の
  ストライドコピーとして実装する
- 公開 API の複素数契約: 全メソッドで `re`/`im` の長さは一致
  (assert)。DC/Nyquist bin の虚部は現行同様に使用しない
  (forward は0を書き、inverse は無視する)

## Non-Functional Constraints

- 数値結果の同一性: 複素演算の分解形は現行の `*_c64s` 演算と
  数学的に同値でなければならない。乗算・加算の結合順序を変えない
  範囲で分解し、scalar/dispatch 一致テストの誤差閾値(現行
  `1e-10 * (1 + |b|)` 相対)を維持する
- FMA 方針: 複素乗算の分解には `mul_add_f64s`/
  `negate_mul_add_f64s`(Scalar 経路では `f64::mul_add` 相当)を
  使い、現行 `mul_c64s` の FMA 融合丸めを可能な範囲で再現する。
  実装の `cmul` は re 成分を `negate_mul_add_f64s(ai, bi,
  mul(ar, br))`(ai·bi 側が融合)とし、pulp `mul_c64s` の
  `re = fma(ar,br,−(ai·bi))`(ar·br 側融合)とは融合側が逆になる
  ため ~1ulp の差が生じ得るが、1 演算削減の性能を優先して意図的に
  採用する。融合側が逆の箇所は ~1ulp 差を許容し、それでも残る
  丸め差は 1e-10 相対閾値内であることを既存テストで確認する
- 決定性: 同一入力・同一 ISA で同一結果。reduction を新設しない
- 性能: `dsp::fft` 経路の `Vec<c64>` 確保と往復コピーを除去する。
  実使用経路(`dsp::fft` 経由の ncc/bandpass/welch psd/
  stft-equivalent)は旧版と同等以上を維持する。実測で直接 plan
  利用の FFT マイクロベンチ(complex fft/rfft、それを内側で使う
  gcc_phat)に SoA 固有のメモリ uops 増による ~+10% 劣後が残る
  ことを確認済みであり、AoS 復帰なしには解消困難な構造的要因として
  既知トレードオフで受け入れる(bench_dsp で比較)
- メモリ: 大きな n(≥16384)で使う thread_local `FFT_WORK` は
  確保した容量をスレッド生存中保持する。ベンチ・反復呼出しでの
  再確保を避ける意図的設計であり、リークではない
- stable Rust・f64 共通表現を維持する

## Risks and Trade-offs

- SoA 化によるカーネル書き換え量が大きい → `stage`/
  `recombine_pair`/`recombine` の3関数に演算が集中しているため、
  関数単位で置き換えて都度テストする
- bit-reversal の swap が2ストアに増える → 全体コストのごく一部で
  あり、許容する。n ≥ 16384 は workspace 経路で `gather_permute`
  の sequential write + gather read に置き換わり、in-place swap 自体
  が発生しない
- 複素演算の分解で加算順が変わると誤差特性が変わる → 分解形を
  現行演算と同じ乗算/加算の組合せに揃え、round trip・
  DC/Nyquist・短尺の境界ケースで scalar 経路と突き合わせる
- `pub use` 除去は破壊的変更だが、外部利用は `dsp::fft` と
  bench の2箇所に限定されることを調査済み(pre-release 方針)

## Deferred Design Concerns

- ベクトル演算プリミティブ(dot/sum_sq/scale/axpy)の公開 —
  `corr0`/`rms`/`energy` 重複の集約として次 spec で扱う。
  reduction 順序ポリシー(lane 単位 reduce の許容可否)はその時点で
  ADR が必要
- `StftMatrix` を `RealFft` 直結にする変更 — 本設計の split 境界が
  前提。`stft.rs` の re/im 保持形式と合わせて別設計で扱う
- `c64` を内部表現から完全に排除するかの判断 — 本設計では
  公開面の除去が目的であり、内部利用の継続は許容する。
- 直接 plan 利用の FFT マイクロベンチに残る ~10% 回帰の追跡 —
  SoA 固有のメモリ uops 増が構造的要因であり、将来 AoS
  ハイブリッド等で回収する余地を残す(既知トレードオフとして
  Non-Functional Constraints に記録済み)。
  回収策の具体形(解説資料化作業で確認): ws 経路(n ≥ 16384)は
  `stage_out`(入力 → workspace コピー)と
  `gather_permute`(workspace → 出力書き戻し)が既に全要素コピー
  を行うため、workspace の中身を split から AoS(交互配置)に
  変えてもコピー回数は増えない。つまり「内部 stage カーネルを
  AoS 版にするだけ」で 12 → 6 ストリームに戻せ、公開境界は
  split のまま維持できる。さらに `gather_permute` の逆順読みが
  「re/im ペアを1キャッシュラインで読む」形になり効率化する。
  コストは内部 stage の split/AoS 2実装と twiddle の2形式化、
  および検証対象の増加。n < 16384 の in-place 経路は split の
  ままでよい(小さな変換はコピー経路を新設する意味がない)
- `spectral_envelope` の `fft_size: 0` が
  `samples.len().next_power_of_two()` で黙って置き換わる —
  `fft_size.max(...)` の既存動作であり、本変更のスコープ外

## Verification Notes

- [ ] `pulp`/`al_math::c64`/`Simd`/`WithSimd`/`dispatch` への参照が
  `al-math` 内部に限定される(grep 確認)
- [ ] FFT の scalar 強制経路と dispatch 経路が n=1..1024 の全
  べき乗長・forward/inverse で一致する(既存テストを split API で
  維持)
- [ ] RealFft の forward/inverse round trip が入力を 1e-10 精度で
  復元する(既存テスト維持)
- [ ] 複素 FFT/RealFft の出力が独立参照実装(radix-2 reference /
  naive DFT)と直接比較で一致する既存テストが split API でも保持
  される(共有コードの系統誤りは scalar/dispatch 一致では検出
  できないため)
- [ ] `mise exec -- cargo test -p al-math -p al-locate -p al-analysis`
  合格
- [ ] `bench_dsp` で実使用経路(`dsp::fft` 経由の ncc/bandpass/
  gcc_phat/welch psd/stft-equivalent)が旧版と同等以上の性能。
  「旧版」は変更前コミットの c64 dispatch 版とし、旧コミットを
  別ビルドした `bench_dsp` と同一入力で比較する。直接 plan 利用の
  FFT マイクロベンチ(complex fft/rfft)の ~10% 劣後は SoA 構造
  要因の既知トレードオフとして記録する
