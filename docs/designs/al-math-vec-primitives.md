---
id: "DESIGN-034Epg45jvGomamsJiywFK"
type: "design"
status: "approved"
title: "al-math vec プリミティブのモジュール構造と呼出し移行設計"
created: "2026-10-12"
updated: "2026-10-12"
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
    - "../specs/al-math-vec-primitives.md"
  implemented-by: []
  depends-on:
    - "./al-math-api-boundary.md"
    - "./observability-foundation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./al-math-api-boundary.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/al-math-vec-primitives.md"
    - "../adr/al-math-vec-primitives-slice-boundary.md"
    - "../adr/simd-reduction-lane-fma.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# al-math vec プリミティブのモジュール構造と呼出し移行設計

## Context

spec `../specs/al-math-vec-primitives.md`(approved)と 2 本の ADR
(`../adr/al-math-vec-primitives-slice-boundary.md` —
公開境界・消費経路・長さ panic 契約、
`../adr/simd-reduction-lane-fma.md` — reduction 順序・FMA・
等価性 predicate)で確定した方針を、`al_math::vec` の実装可能な
モジュール構造・カーネル記述・呼出し移行マップに具体化する。
棚卸しの実測根拠は `../discovery/al-math-simd-reduction.md`。

契約文面の権威は ADR にある。本設計内の再掲(プリミティブ集合・
panic 契約・ゼロ入力規則・等価性 predicate)が ADR と乖離した
場合は ADR を正とし、本設計を追従修正する。

先行する `al-math` の dispatch 規約(本設計が継承する足場):

- `lib.rs` の `static ARCH: LazyLock<pulp::Arch>` と
  `pub(crate) fn dispatch<K: WithSimd>(kernel: K)` —
  CPU 検出はプロセス内 1 回、dispatch は演算の公開入口で 1 回
- `pub(crate) use pulp::{Simd, WithSimd}` と
  `pub(crate) type F64s<S>` — pulp 型は crate 内部に封じ込め
- `linalg.rs` の `dot`(lane アキュムレータ + scalar tail) —
  今回共有カーネル化する元の内部 SIMD 実装

## Scope

### In Scope

- `crates/al-math/src/vec.rs` のモジュール構造(カーネル /
  `WithSimd` アダプタ / 公開 wrapper の3層)と 8 プリミティブの
  シグネチャ・カーネル記述の確定
- `linalg.rs` の内部 `dot` の共有カーネル化と `reflect` の
  `sub_scaled` カーネル消費への切替
- `al_locate::dsp` への re-export と `al_locate` crate root の
  既存 mirror 規約(`lib.rs:22` `pub use dsp::{...}`)に倣った
  mirror 追加
- spec In Scope が列挙した全呼出し箇所の移行マップ(ファイル・
  現行式 → プリミティブ/再構成の対応表)。残存許容リストの箇所は
  変更しない
- `residual.rs` 減算ループの per-source `sub_scaled` 条件付き
  再構成の計測プロトコル(方法・採否基準・記録先)
- vec プリミティブの criterion ベンチ計画とテスト計画
  (scalar ビット一致 / dispatch 等価 / panic・契約テスト)

### Out of Scope

- spec の Out of Scope を全て継承する: FFT・相関カーネルの
  アルゴリズム変更(issue #42)、`al-viz` `lod.rs` の
  incremental `sum_sq`、lag 走査自体のプリミティブ化、
  ベクトル/行列抽象型、al-analysis への al-math 直接依存、
  pulp 型の公開面露出、golden 意味・Validity 閾値の変更
- spec「残存許容」リストの箇所(`expensive/ir.rs` タプルスライス
  合成、residual 列 gather dot(再構成非採用時)、
  `Vec<Vec<f64>>`/`Vec<(f64,f64)>` 上の合成、同一式でない
  複合式)には一切触れない。file map の棚卸しで残存許容に
  分類した追加箇所:
  - `cheap.rs:390` — `mom`(フレーム毎 mean-square)への
    `sqrt(mean(mom))` は `rms` 式でない(同一式でない複合式 —
    mean プリミティブが不在)。`rms(&mom)` に写すと
    `sqrt(mean(mom²))` となり `level.integrated_dbfs` が実質
    変化するため、インラインのまま残す
  - `reference.rs:872-881` M/S エネルギー — `Σ((a±b)/2)²` は
    materialization を要する複合式 — 対象外

## Components and Boundaries

### `al_math::vec` モジュール構成

`crates/al-math/src/vec.rs` に単一ファイルモジュールとして新設し、
`lib.rs` に `pub mod vec;` を追加する。8 プリミティブは全て同一の
3層パターンを共有するためファイル分割しない(KISS)。

```text
vec.rs
  ├─ fn check_same_len(...)            … 等長 assert の共有ヘルパ
  ├─ pub(crate) fn dot_kernel<S: Simd>(simd, a, b) -> f64
  ├─ …各プリミティブの内部ジェネリックカーネル(pub(crate))
  ├─ struct Dot<'a> { a, b } impl WithSimd … 各プリミティブ1つ
  └─ pub fn dot(a: &[f64], b: &[f64]) -> f64 { dispatch(Dot{a,b}) }
     …各プリミティブの公開 wrapper
```

- **内部カーネル** `fn xxx_kernel<S: Simd>(simd: S, ...) -> ...`:
  `Simd` ジェネリックな単一ソース。`WithSimd` コンテキスト内
  (linalg の `reflect` 等)からも直接呼べるよう `pub(crate)`。
  等長 assert はカーネル先頭の `check_same_len` で行う —
  公開経路・カーネルレベル再利用経路の両方で同一契約を
  1箇所に保持するため
- **`WithSimd` アダプタ**: スライス・スカラー引数をフィールドに
  持ち、`with_simd` がカーネルを呼ぶだけの薄い構造体
- **公開 wrapper**: `dispatch(Xxx { ... })` を 1 回呼ぶだけ。
  pulp 型は公開シグネチャに出ない

既存規約との対応: `linalg.rs` の `dot` と同じ lane アキュムレータ
+ scalar tail 構造、`lib.rs` の `dispatch` を公開入口で 1 回呼ぶ
規約、FFT 系の入口 assert/`should_panic` テスト方針を継承する。

### 公開シグネチャ(8 プリミティブ)

```rust
pub fn dot(a: &[f64], b: &[f64]) -> f64;
pub fn sum_sq(x: &[f64]) -> f64;
pub fn corr0(x: &[f64], y: &[f64]) -> f64;
pub fn rms(x: &[f64]) -> f64;
pub fn scale(x: &[f64], a: f64, out: &mut [f64]);
pub fn axpy(y: &mut [f64], a: f64, x: &[f64]);
pub fn sub_scaled(y: &mut [f64], a: f64, x: &[f64]);
pub fn mag_sq(re: &[f64], im: &[f64], out: &mut [f64]);
```

- `scale`/`axpy`/`sub_scaled` の `a` は f64 スカラー乗数
  (ベクトルではない)
- **等長契約**: `dot`/`corr0`/`scale`/`axpy`/`sub_scaled` は
  関連する2スライス、`mag_sq` は `re`/`im`/`out` の3スライス全てが
  等長でなければ panic。`scale` の `x`/`out` 等長も ADR・spec の
  panic 列挙に含まれる(`zip` での黙認は「fail, never guess」に
  反するため同一ポリシー)。現行 corr0 の min-len 黙認は
  継承しない
- **値契約**: `corr0` は `Σx² > 0 && Σy² > 0` でなければ 0.0
  (NaN 入力は `NaN > 0.0 == false` により自然に 0.0 へ写像)。
  中心化しない。`rms` は空入力で 0.0

### カーネル記述(プリミティブ毎)

reduction 系の積和は **`mul_add_e_f64s` 専用** とする
(`pulp::Scalar` では非融合の `a*b+c` に下り逐次和とビット一致、
x86-v3/v4 では真の FMA に下る)。reduction カーネルで
`mul_add_f64s` は禁止(Scalar でも融合し逐次和ビット一致を破る)、
`mul_f64s` + `add_f64s` の2命令も禁止(FMA を失う)。
elementwise 系の積和式(`axpy`/`sub_scaled`/`mag_sq`)も同じ
`_e` 系 op を使い、Scalar でのビット一致を保つ。

| primitive | カーネル構造 | 使用 op / 補足 |
| --- | --- | --- |
| `dot` | lane acc `s = splat(0)`、vector 部 `s = mul_add_e(a,b,s)`、tail は `a*b` の逐次加算、`reduce_sum(s) + tail` | linalg の既存構造を `mul_add_e` 化した形。linalg `dot` はこれを呼ぶ薄い委譲に置換 |
| `sum_sq` | `dot` と同形で `s = mul_add_e(v,v,s)` | |
| `corr0` | **1パス3アキュムレータ融合**: `sxy`/`sxx`/`syy` の3レーン acc を同一ループで `mul_add_e` 更新し、vector/tail 各処理後に `ex>0.0 && ey>0.0 → sxy/(ex*ey).sqrt() else 0.0` | 現行の「2パス + 別走査」を1パス化。Scalar(1レーン)でも各 acc が要素順に累算されるため逐次和とビット一致 |
| `rms` | `n==0 → 0.0`、else `(sum_sq_kernel(simd,x)/n).sqrt()` | カーネル内で `sum_sq_kernel` を再利用(追加 dispatch なし) |
| `scale` | vector 部 `*o = mul_f64s(x, splat(a))`、tail `*o = x*a` | 純粋な乗算(積和でない)のため `mul_f64s` |
| `axpy` | vector 部 `*y = mul_add_e(av, xv, *y)`、tail `*y += a*x` | `y += a*x` |
| `sub_scaled` | vector 部 `*y = negate_mul_add_e(av, xv, *y)`(`= y - a*x`)、tail `*y -= a*x` | `negate_mul_add_e_f64s` は Scalar で `c - a*b` に下り `*y -= a*x` とビット一致 |
| `mag_sq` | vector 部 `*o = mul_add_e(r, r, mul_f64s(i,i))`、tail `*o = r*r + i*i` | `out` へ書き込む3スライス形。Scalar では `r*r + i*i` とビット一致 |

`sum_sq`/`rms`/`corr0` のスカラー出力は
`simd.reduce_sum_f64s(acc) + tail逐次和`。tail の逐次和は
`iter().map(..).sum()` と同順序に保つ。

### linalg 共有カーネル化

- `linalg.rs` の `fn dot<S: Simd>(simd, a, b)` は削除し、呼出し側
  (`Factor`/`Apply` 内)を `crate::vec::dot_kernel` 呼出しに変更
- `reflect`(`linalg.rs:15-26`、`*t = t - factor·v` 形)は:
  ```rust
  fn reflect<S: Simd>(simd: S, v: &[f64], target: &mut [f64]) {
      let factor = 2.0 * crate::vec::dot_kernel(simd, v, target);
      crate::vec::sub_scaled_kernel(simd, target, factor, v);
  }
  ```
  `reflect` は `WithSimd` コンテキスト内で既に `simd` を持つため
  公開 wrapper を経由せずカーネル直接呼出しとし、二重 dispatch を
  避ける。`v.len() == target.len()`(どちらも `m+extra-j`)が既存
  構造から保証されるため等長 assert に適合する
- これにより `sub_scaled` は residual 再構成の採否に依らず実
  呼出し箇所を持つ(ADR の YAGNI 解消)

### dsp re-export と crate root mirror

- `crates/al-locate/src/dsp.rs`: 既存の
  `pub use al_math::correlation::{...}` パターンに倣い
  `pub use al_math::vec::{axpy, corr0, dot, mag_sq, rms, scale, sub_scaled, sum_sq};`
  を追加
- `crates/al-locate/src/lib.rs:22` の既存 mirror
  `pub use dsp::{bandpass, deinterleave, fft, gcc_phat, ncc, spectral_envelope};`
  を同じ `pub use dsp::{...}` 行に vec シンボルを追加して拡張。
  正規の消費経路は `al_locate::dsp` のまま、root mirror は公開面の
  一貫性のための re-export のみ。既存 export 名との衝突なし
- `crates/al-analysis/Cargo.toml` には al-math 依存を追加しない
  (al-analysis は `al_locate::dsp` 経由のみ)

### 呼出し移行マップ(file map)

spec In Scope の列挙全件をカバーする。行番号は現行コード基準の
目安。`dsp::` は `al_locate::dsp` 経由の呼出しを表す。

**crates/al-locate**

| 箇所 | 現行 | 移行 |
| --- | --- | --- |
| `methods.rs:142` | `fn energy` 自前 `Σv²` | 削除し `dsp::sum_sq` に置換(呼出し :175,:183,:363,:408 と :501。:363 は `evidence_from_bands`、:408 は `BandNcc::locate` の MIN_ENERGY フィルタ) |
| `methods.rs:500-506` `normalize_gain` | `energy/len` の sqrt → 要素除算 | `dsp::rms(x)` + `dsp::scale(x, r.recip(), &mut out)`。`v / rms` が `v * r.recip()` になるため ulp 級の差異(除算→逆数乗算)を生む — 再構成の許容誤差ポリシーで許容済み。Implementation Record に記録する |
| `methods.rs:541-544` envelope NCC | インライン dot/√(tn·sn) | `dsp::corr0(&t_env, &s_env)`(`tn*sn>0` と `ex>0&&ey>0` は非負和で同値 — NaN を含めガード同値性は保持)。`dot/(tn*sn)` と `sxy/(ex*ey).sqrt()` は `sqrt(a)*sqrt(b)` vs `sqrt(a*b)` で ulp 級に差異し得る |
| `mixer.rs:74` | インライン `rms` | `dsp::rms(cap)` |
| `residual.rs:133` | `&[Vec<f64>]` マルチch `rms` | per-channel `dsp::sum_sq` の合成に再表現(下記参照) |
| `residual.rs:163-169` | 転置アクセス減算ループ | **条件付き** per-source `sub_scaled` 再構成(計測プロトコル節) |

`residual.rs:133` の再表現:
```rust
fn rms(channels: &[Vec<f64>]) -> f64 {
    let (sum, n) = channels.iter().fold((0.0, 0usize), |(s, n), c| {
        (s + dsp::sum_sq(c), n + c.len())
    });
    if n == 0 { 0.0 } else { (sum / n as f64).sqrt() }
}
```
プリミティブを再合成した式のため結合順が変わり、
`ResidualEnergy` 等の永続化値は ~ulp 級に揺れ得る
(ADR で許容済み)。

**crates/al-math**

| 箇所 | 現行 | 移行 |
| --- | --- | --- |
| `linalg.rs` `dot` | 内部 SIMD 実装 | `vec::dot_kernel` 共有(本体削除) |
| `linalg.rs` `reflect` | 手書き `*t -= factor*v` ループ | `dot_kernel` + `sub_scaled_kernel` |

**crates/al-analysis**(全て `al_locate::dsp` 経由、al-math 直接
依存なし)

| 箇所 | 現行 | 移行 |
| --- | --- | --- |
| `cheap.rs:180,:254,:358,:414` | インライン `rms` 系式 | `dsp::rms` |
| `cheap.rs:385` | mean-square(`Σv²/n`、sqrt なし) | `dsp::sum_sq(x) / n`(`rms` 非適用、平方根を取らない式のため) |
| `cheap.rs:912,:921` | インライン `sum_sq` | `dsp::sum_sq` |
| `cheap.rs:1017-1023` `stereo.pair_correlation` | sx/sy/sxy のインライン3アキュムレータ | `corr = dsp::corr0(xs,ys)` + `sx = dsp::sum_sq(xs)`、`sy = dsp::sum_sq(ys)`。sx/sy は :1030 の Validity 判定(ゼロエネルギー→`SilenceInsufficientEnergy`)と :1042 の `stereo.ild_db` で再利用するため値を落とさない |
| `cheap.rs:1067-1071` `stereo.ipd_samples` | lag 走査内インライン corr | `dsp::corr0` |
| `cheap.rs:1250-1252` `trans.self_similarity` | インライン corr | `dsp::corr0` |
| `reference.rs:119` | `rms` | `dsp::rms` |
| `reference.rs:134` | `corr0` | `dsp::corr0` |
| `reference.rs:127-131` `ls_gain` | dot/sum_sq 合成 | ゼロ分母ガード `den > 0.0 → num/den else 0.0` を保持:`let den = dsp::sum_sq(x); if den > 0.0 { dsp::dot(x,y) / den } else { 0.0 }`(字面どおり `dsp::dot / dsp::sum_sq` に写すと `sum_sq==0` で NaN を生む) |
| `reference.rs:1066` | マルチch `energy` クロージャ | per-channel `dsp::sum_sq` 合成 |
| `reference.rs:1164` | `res_rms` 合成 | `dsp::rms` |
| `expensive/mod.rs:257,:264` | `pub(crate) rms`/`corr0` | 削除。呼出し側は `al_locate::dsp::{corr0, rms}` を import |
| `expensive/timing.rs:11` | `use super::{corr0, rms}` | `use al_locate::dsp::{corr0, rms}` に変更。:92,:148,:397 の `corr0`・:172,:216,:276 の `rms` 呼出しは名前置換のみ |
| `expensive/dynamics.rs:11` | `use super::{..., rms}` | timing.rs と同様に `super` から `rms` を外し `use al_locate::dsp::rms` に変更。:86,:149,:350,:351 の `rms` 呼出しは名前置換のみ |
| `expensive/ir.rs:178` | split re/im 上の `mag2` 合成 | `dsp::mag_sq(&re, &im, &mut mag2)`(:155-161 のタプルスライス合成は残存許容、触れない) |
| `band_gain.rs:94,:101` | インライン `Σv²` | `dsp::sum_sq` |
| `band_gain.rs:96` | window dot | `dsp::dot` |
| `stft.rs:78-84` `power()` | re²+im² 合成 | フレーム毎 `dsp::mag_sq(re, im, &mut out)` |
| `stft.rs:47-61` `psd()` | `(re²+im²)/norm` | フレーム毎 `dsp::mag_sq` で power バッファを算出後、`dsp::scale(&power, 1.0/norm, &mut out)` で正規化(2段、下記「エイリアス方針」参照) |
| `stft.rs:64-76` `magnitude()` | `sqrt(re²+im²)` | `mag_sq` + 要素毎 `f64::sqrt` 合成(sqrt のプリミティブは提供しない) |

**エイリアス方針**: 全プリミティブは入力と出力のスライス
エイリアスを許容しない(借用規則上 `&mut [f64]` と `&[f64]` の
同時保持は不可能だが、`scale(x, a, x_mut)` のような論理 in-place
は呼出し側で `x` を `to_vec` するか別バッファを使う)。`psd()` は
`mag_sq` で power バッファを作った後 `scale` で正規化する2段と
し、1回の余分な要素走査は許容する。

### `residual.rs` 条件付き再構成の計測プロトコル

対象: `residual.rs:163-169` の転置アクセス減算ループ
(`*v -= Σ_s m·s[t]`、`s[t]` が strided gather)。

再構成案(採用時):
```rust
for (cap, row) in residual.iter_mut().zip(&est.matrix) {
    for (s, &m) in src.iter().zip(row) {
        dsp::sub_scaled(cap, m, s);
    }
}
```
数学的には `cap[t] -= Σ_s m_s·src_s[t]` と同値で、アクセスが
strided gather から連続ストリームに変わる。累算順が変わるため
結果は許容誤差 predicate 内の差異を許容し、同所の列 gather
dot(:166)もこの再構成でのみ解消される。

**測定方法**:

1. 正しさゲート: `subtract_fit`/`prepare_residual` の既存テストが
   green、かつ同一 fixture 上で再構成前後の `residual` 各要素が
   `|a-b| <= 1e-12 * max(1.0,|a|,|b|)`(要素毎)を満たすことを
   一時比較コードまたはテストで確認する
2. 性能測定: `crates/al-locate/examples/bench_locate_prepared`
   の `Instant` 計測で再構成前後を同一条件で実行し、残差減算を
   含む経路の実測値を比較する。代表入力は実際のチャンネル構成
   (capture channels × source channels)に従う
3. **採否基準**: 正しさゲートを満たし、かつ再構成後が criterion
   のノイズ幅(目安 ±5%)を超える回帰を示さなければ採用。
   回帰または正しさ不成立なら現行ループを保持し、列 gather dot
   は残存許容のまま残す
4. **記録**: 採否の根拠と実測値を Implementation Record に記録
   する(採用・見送りのいずれでも記録必須、spec AC 準拠)

### criterion ベンチ計画

`dsp_smoke` は smoke 配線確認の役割に留めるため、**新 bench
target `crates/al-math/benches/vec_primitives.rs`** を追加し、
`Cargo.toml` に `[[bench]] name = "vec_primitives"` `harness = false`
を登録する。

- 各プリミティブを criterion group `vec` 配下に
  `BenchmarkId` で測定。reduction 系は `Throughput::Bytes` で
  GB/s も併記できるようにする
- サイズ分布: `32`(spectral_envelope bins 等の小ベクトル・
  per-call dispatch コスト観測)、`1_024`、`65_536`、
  `1_048_576`(methods.rs preflight の全長 energy に相当する
  大きさ)。テストのサイズ行列と対応させる
- 入力は deterministic 生成(sin/LCG)、`black_box` で最適化除去
  を防ぐ
- 置換前比は既存 `examples/bench_*`(bench_dsp・bench_ncc・
  bench_locate_prepared)の `Instant` 計測で取り、spec AC の
  代表経路(methods preflight energy・band_gain window dot・
  cheap corr0 lag 走査・residual 減算ループ)について ±5% を
  超える回帰がないことを確認。実測値は Implementation Record に
  記録する

### テスト計画

`vec.rs` 内 `#[cfg(test)] mod tests` に配置(既存ファイル内
テストの規約に一致)。参照実装はテスト内に逐次 `.iter()` 版を
書き、scalar 経路は `pulp::Scalar.vectorize(Kernel{...})`、
dispatch 経路は `dispatch(Kernel{...})` で得る(lib.rs テストの
確立パターン)。

1. **契約テスト**
   - `corr0`: 零ベクトル・片側零・NaN 含有入力で 0.0、
     `corr0(x,x)==1.0`、直交入力で ~0.0
   - `rms`: 空入力で 0.0、定数入力で `|c|`
   - panic: `#[should_panic]` で `dot`/`corr0`/`scale`/`axpy`/
     `sub_scaled` の不等長、`mag_sq` の `re≠im`・`re≠out`
   - 公開シグネチャに pulp 型が出ないことはコード検査
2. **Scalar ビット一致**: 各プリミティブの Scalar 経路結果が
   逐次和参照実装と `assert_eq!`(f64 厳密比較、ビット一致)で
   一致することを、0..=33・1k・64k で確認。プリミティブ単位・
   同一データ上の保証であり、再合成式(マルチch rms 等)は
   対象外
3. **dispatch ↔ Scalar 等価**: predicate
   `|a-b| <= 1e-12*max(1.0,|a|,|b|)`(ベクトル出力は要素毎)を
   満たすことを、サイズ行列 **0..=33・1k・64k・~1M(1_048_576)**
   で確認。本ビルドの pulp features `["std","x86-v3"]` で
   検証される経路は x86-v3 と Scalar。入力は deterministic
   (負値・大きさの異なる要素を含め lane 順序差が出る分布)
4. **呼出し側**: 既存の `cargo test -p al-locate -p al-analysis`
   と `cargo test --workspace`(golden・Validity ゲート含む)が
   置換後も pass すること。float `assert_eq!` 厳密比較の既存
   テストは planning で棚卸しし、経路間比較に該当するものは
   許容比較へ修正(修正一覧は Implementation Record に記録)

## Data and Control Flow

```text
呼出し側 (al-locate / al-analysis)
  → al_locate::dsp::xxx        … re-export(唯一の消費経路)
  → al_math::vec::xxx          … 公開 wrapper
  → dispatch(WithSimd)         … lib.rs ARCH LazyLock、入口で1回
  → xxx_kernel<S: Simd>        … check_same_len → SIMD+tail 単一ソース
                                   ↑ linalg::reflect 等はここへ直接
```

- reduction 系は lane アキュムレータで累算し
  `reduce_sum + tail逐次和`、elementwise 系は `as_simd_f64s` /
  `as_mut_simd_f64s` で vector + tail
- `pulp::Scalar`(1レーン)ではベクトル部が要素順に逐次実行され、
  `mul_add_e` 系が非融合に下るため逐次和と同順序 → ビット一致

## Non-Functional Constraints

- dispatch は公開入口で 1 回、カーネルは `Simd` ジェネリック
  単一ソース(前 spec 規約の継続)
- 公開シグネチャは `&[f64]`/`&mut [f64]` と f64 のみ。pulp 型・
  `Simd`/`WithSimd` トレイトは公開面に出さない
- scalar 経路はプリミティブ単位で逐次和とビット一致。再合成式は
  結合順差を許容誤差ポリシー内で許容
- 置換後の分析意味・golden 出力・manifest/feature record 形式は
  変更しない
- 性能: `corr0` は1パス3アキュムレータ、reduction は
  lane+FMA。小ベクトルの per-call dispatch オーバーヘッドは
  ポリシー上許容(Deferred)

## Risks and Trade-offs

- `residual.rs` 再構成が回帰するリスク → 計測プロトコルで採否を
  判定し、非採用時は現行維持(残存許容に吸収)
- float 厳密比較テストの棚卸し漏れ → planning で洗い出し、
  経路間比較のみ許容比較化
- `pair_correlation` で `corr0`+個別 `sum_sq` にすると sx/sy を
  取るため3パスになる(融合時は1パス+2パス)。n は先頭プレフィクス
  で小さく、生アキュムレータ保持よりプリミティブ消費を優先する
  判断を spec が既に許容しているため採用。ベンチで問題が観測
  された場合のみ再検討
- `scale`/`psd` の2段合成は1回分の要素走査が増える — 契約の
  明確さを優先し、ベンチ対象外の中規模データで影響は限定的
- dispatch 経路の最終ビットが ISA 依存で永続化値が ~ulp 揺れ →
  ADR で許容済み(ゲート閾値との桁差が確認済み)

## Deferred Design Concerns

spec の Deferred Design Concerns をそのまま継承する
(再掲は変更せず、spec 側を正とする):

- lag 走査自体のプリミティブ化・`ncc` 置換 — issue #42 系の
  別チェーン
- `al-viz` `lod.rs` の incremental `sum_sq` — streaming 契約が
  異なる別境界
- criterion ベンチの CI 回帰ゲート — CI 基盤未整備、整備時に
  baseline 比較を再検討
- 小ベクトルへの per-call dispatch コスト — bench で問題が観測
  された時点で呼出し側 batch 化等を再検討

## Verification Notes

- [ ] `al_math::vec` の8プリミティブがスライス/f64 のみの公開
  シグネチャを持ち、pulp 型が公開面に出ない
- [ ] `al_locate::dsp` re-export と crate root mirror が既存規約
  どおり配線され、`cargo tree -p al-analysis --depth 1` の直接
  依存に al-math がない
- [ ] file map の全列挙箇所が置換され、残存許容箇所以外に
  corr0/rms/sum_sq の独立実装が残らない
- [ ] panic・`corr0`/`rms` 値契約のテスト、scalar ビット一致、
  dispatch 等価(0..=33・1k・64k・~1M、predicate 準拠)の
  テストが存在し pass する
- [ ] `linalg.rs` の内部 `dot` が `vec::dot_kernel` を共有し、
  `reflect` が `sub_scaled_kernel` を消費している
- [ ] `cargo bench -p al-math` で vec ベンチが実行でき、代表経路
  の置換前比・実測値が Implementation Record に記録される
- [ ] `residual.rs` 再構成の採否が計測プロトコルに基づき記録
  される
- [ ] `cargo test -p al-math -p al-locate -p al-analysis`・
  `--workspace`・`clippy`・`fmt --check` が green
