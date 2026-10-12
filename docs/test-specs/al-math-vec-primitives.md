---
id: "TSPEC-034EpMxXDl69zm65O1mGBG"
type: "test-spec"
status: "approved"
title: "al-math vec プリミティブ検証仕様"
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
    - "docs/designs/al-math-vec-primitives.md"
  derived-by: []
  verifies:
    - "docs/specs/al-math-vec-primitives.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# al-math vec プリミティブ検証仕様

## Purpose

`docs/specs/al-math-vec-primitives.md` の Acceptance Criteria を
検証可能な形に分解した検証仕様。`al_math::vec` の公開境界
(スライスのみ・pulp 非露出)、等長 panic 契約、値の契約、
scalar 経路のビット一致と dispatch↔Scalar の許容誤差等価、
dsp ファサード消費経路の一元化、重複解消(残存許容リスト付き)、
および計測・記録義務が、spec/ADR の契約が変わらない限り保持
されることを保証する。プリミティブ集合・長さ panic 契約・
ゼロ入力規則の権威は `docs/adr/al-math-vec-primitives-slice-boundary.md`、
reduction 順序と等価性 predicate の権威は
`docs/adr/simd-reduction-lane-fma.md` とし、本文の再掲と ADR が
乖離した場合は ADR を正とする。

## Feature

f64 スライスのベクトル演算プリミティブ(`dot`/`sum_sq`/`corr0`/
`rms`/`scale`/`axpy`/`sub_scaled`/`mag_sq`)を `al_math::vec` に
集約し、pulp dispatch 経由の SIMD 実装として `al_locate::dsp`
ファサード経由でのみ公開する集約・移行。

## Rules

### Rule A: 公開 API 境界とカーネル規約

- Rule: `al_math::vec` の各プリミティブが公開 API として存在し、
  公開シグネチャは `&[f64]`/`&mut [f64]` スライスと f64
  スカラーのみで、pulp 由来の型・トレイト(`Simd`・
  `WithSimd` 等)が出現しない
- Rule: `scale`/`axpy`/`sub_scaled` の乗数引数 `a` は f64
  スカラー乗数であり要素ベクトルではない。`mag_sq` は
  `(re: &[f64], im: &[f64], out: &mut [f64])` の3スライス形
- Rule: dispatch の呼出しは演算の公開入口で1回とし、カーネルは
  `Simd` ジェネリックな単一ソース(`pub(crate)
  xxx_kernel<S: Simd>`)を維持する。`linalg.rs` の内部 `dot`
  は共有カーネル化して公開 `dot` と同一実装とし、`reflect`
  は `dot_kernel` + `sub_scaled_kernel` のカーネルレベル消費者
  となる
- Rule: リダクションカーネルの積和は `mul_add_e_f64s` のみを
  使う。`mul_add_f64s`(Scalar でも融合となりビット一致を破る)
  と `mul_f64s` + `add_f64s` の2命令形(FMA を失う)は禁止。
  要素毎融合形(`axpy`/`sub_scaled`/`mag_sq`)も `_e` 系 op を
  使う。この規則はコードレビューと scalar ビット一致テストで
  検証可能とする

### Rule B: 長さ panic 契約

- Rule: 2入力プリミティブ(`dot`/`corr0`/`axpy`/`sub_scaled`/
  `scale`、入出力スライス含む)は長さ不整合を等長 assert で
  panic とし、`mag_sq` は `re`/`im`/`out` の3スライス全てが
  等長でなければ panic とする
- Rule: 現行 corr0 の min-len(`zip`)黙認は継承しない
  (「fail, never guess」・`fft_size` assert の前例に揃える)

### Rule C: 値の契約

- Rule: `corr0` は `Σx² > 0 かつ Σy² > 0` でなければ 0.0 を
  返す(NaN・縮退エネルギーも 0.0 に写像する現行契約を維持)
  し、中心化しない
- Rule: `rms` は空入力で 0.0 を返す

### Rule D: scalar 経路のビット一致

- Rule: `pulp::Scalar` 強制経路の各プリミティブ結果は、従来の
  逐次 `.iter().sum()` 系参照実装と同一データ上でビット一致
  (`assert_eq!` 厳密比較)する。検証サイズは
  0..=33・1k・64k
- Rule: ビット一致はプリミティブ単位・同一データ上の保証で
  あり、プリミティブを再合成した式(per-channel `sum_sq` の
  和によるマルチch rms 等)は結合順が変わるため許容誤差
  ポリシー内の差異を許容する。永続化値(`ResidualEnergy` 等)
  は ~ulp 級に揺れ得る

### Rule E: dispatch↔Scalar 等価性

- Rule: dispatch 経路と Scalar 経路の等価性は、スカラー出力
  (`dot`/`sum_sq`/`corr0`/`rms`)で `|a - b| <= 1e-12 *
  max(1.0, |a|, |b|)`、ベクトル出力(`scale`/`axpy`/
  `sub_scaled`/`mag_sq`)で同 predicate を要素毎に満たすこと
  と定義する。経路間比較に float の `assert_eq!` 厳密比較は
  使わない
- Rule: 検証サイズ行列は 0..=33(空・1要素・lane 端数の全
  経過)・1k・64k・~1M(1_048_576)とし、決定的入力(負値・
  混合桁を含む)で lane 順序差を露出させる
- Rule: 本ビルドの pulp features は `["std","x86-v3"]` であり、
  検証される dispatch 経路は x86-v3 と Scalar のみ(v4 等は
  feature 追加時に拡張)

### Rule F: 消費経路の一元化

- Rule: `al_locate::dsp` が vec プリミティブを re-export する
  唯一の消費経路であり、`al_locate` crate root は既存 mirror
  規約(`lib.rs` の `pub use dsp::{...}`)に倣い vec シンボル
  を mirror する(正規経路は `dsp::` のまま)
- Rule: al-analysis は `al_locate::dsp` 経由でのみ消費し、
  `crates/al-analysis/Cargo.toml` に al-math 依存を追加しない。
  `mise exec -- cargo tree -p al-analysis --depth 1` の直接
  依存に al-math が現れないこと(または Cargo.toml の直接確認)
  で判定する

### Rule G: 重複解消と残存許容

- Rule: spec の In Scope で列挙した重複実装(methods.rs
  `energy`・preflight・`normalize_gain`・envelope NCC、
  mixer.rs `rms`、residual.rs マルチch `rms`、cheap.rs の
  `rms`×4・mean-square・`sum_sq`×2・inline corr0×3、
  reference.rs `rms`・`corr0`・`ls_gain`・マルチch `energy`・
  `res_rms`、expensive/mod.rs `rms`・`corr0` と timing.rs・
  dynamics.rs の消費側、ir.rs `mag2`、band_gain.rs `sum_sq`×2・
  window dot、stft.rs `power`/`psd`/`magnitude`)が
  プリミティブ呼出しに置換され、「残存許容」リストに明示した
  箇所を除き同一式の独立実装が残存しない
- Rule: 残存許容は spec の列挙に限る: `expensive/ir.rs`
  タプルスライス上の合成、`residual.rs` 列 gather dot(条件
  付き再構成の非採用時のみ)、`al-viz` `lod.rs` incremental
  `sum_sq`、`Vec<Vec<f64>>`/`Vec<(f64,f64)>` 上の合成
  (residual Frobenius ノルム・reference の magnitude 合成・
  off-diagonal `sum_sq`)、同一式でない複合式(cheap.rs
  mid/side 電力・magnitude 総和・4アキュムレータ回帰 fold・
  `sqrt(mean(mom))`、dynamics.rs 符号別 split・要素毎
  magnitude・回帰アキュムレータ、reference.rs 単要素/バンド
  合成・M/S エネルギー)
- Rule: ガード付き合成は値を落とさない: `ls_gain` は
  `let den = dsp::sum_sq(x); if den > 0.0 { dsp::dot(x,y)/den }
  else { 0.0 }` のゼロ分母ガードを保持し、
  `pair_correlation` は `corr0` + 個別 `sum_sq` で sx/sy を
  保持する(Validity 判定・`stereo.ild_db` で消費)
- Rule: 置換後も既存の分析意味・golden 出力・manifest/feature
  record の形式を変更しない。float `assert_eq!` 厳密比較の既存
  テストで経路間比較に該当するものは許容比較へ修正し、修正一覧
  を Implementation Record に記録する

### Rule H: 計測と記録

- Rule: `mise exec -- cargo bench -p al-math` で vec
  プリミティブの criterion ベンチ(`vec_primitives` target、
  criterion group `vec`、サイズ 32・1_024・65_536・1_048_576)
  が実行でき、reduction は `Throughput::Bytes` を報告する
- Rule: 代表呼出し経路(methods.rs preflight energy、
  band_gain.rs window dot、cheap.rs corr0 lag 走査、
  residual.rs 減算ループ)の置換前後比を既存 examples
  `bench_*` の `Instant` 計測で取り、criterion ノイズ幅
  (目安 ±5%)を超える回帰がないことを確認し、実測値を
  Implementation Record に記録する。小スライスの per-call
  dispatch オーバーヘッド回帰はポリシー上許容する
- Rule: `residual.rs` 減算ループの per-source `sub_scaled`
  再構成は、正しさゲート(既存テスト green + 同一 fixture
  で要素毎 predicate 内)と性能(±5% を超える回帰なし)の
  計測プロトコルで採否を決め、採否の根拠を計測値とともに
  Implementation Record に記録する

## Examples

- Example(公開境界): `dot(a: &[f64], b: &[f64]) -> f64` 等の
  公開シグネチャに `Simd`/`WithSimd`/pulp 型が現れず、
  `mise exec -- cargo doc -p al-math` 相当の公開面検査で確認
  できる
- Example(等長 panic): `dot(&[1.0; 4], &[1.0; 3])` が panic
  し、`mag_sq(&re4, &im3, &mut out4)` および `re != out` の
  組合せも panic する `#[should_panic]` テストが pass する。
  `scale` は `x`/`out` の不整合で panic する
- Example(値の契約): `corr0(&[0.0; 8], &[1.0; 8])` → 0.0、
  NaN 混入入力 → 0.0、`corr0(x, x)` ≈ 1.0、直投入力 ≈ 0.0。
  `rms(&[])` → 0.0、定数入力 `rms(&[c; n])` → |c|
- Example(scalar ビット一致): `pulp::Scalar` 経路の `dot` が
  逐次 `a.iter().zip(b).map(|(x,y)| x*y).sum::<f64>()` 参照と
  サイズ 0..=33・1k・64k で `assert_eq!` 一致する
- Example(dispatch↔Scalar): サイズ 1_048_576 の混合桁・負値
  を含む決定的入力で、`dot` の dispatch 結果と Scalar 結果が
  `|a - b| <= 1e-12 * max(1.0, |a|, |b|)` を満たす。
  `mag_sq` 出力は全要素が同 predicate を満たす
- Example(消費経路): `mise exec -- cargo tree -p al-analysis
  --depth 1` の出力に al-math が直接依存として現れず、
  `crates/al-analysis/Cargo.toml` に al-math 記述がない。
  `al_locate::dsp::corr0` が使え、`al_locate::corr0` も root
  mirror として解決する
- Example(ガード保持): `ls_gain` に `sum_sq(x) == 0.0` の
  入力を与えても NaN を返さず 0.0 になる。
  `pair_correlation` 置換後もゼロエネルギー入力で
  `SilenceInsufficientEnergy` の Validity 判定が維持される
- Example(残存許容): `expensive/ir.rs` のタプルスライス合成
  と `cheap.rs` の mid/side 電力は置換されず残存し、重複解消
  の判定対象外となる
- Example(bench): `mise exec -- cargo bench -p al-math` で
  `vec` group のベンチが 4 サイズで走り、IR に実測値と
  置換前後比が記録される
- Example(残余再構成の採否): 採用時は `cap[t] -= Σ m_s·s[t]`
  が per-source `dsp::sub_scaled(cap, m, s)` 呼出しになり列
  gather dot も解消される。見送り時は現行ループと列 gather
  dot が残存許容として残り、理由が IR に記録される

## Guarantees

- 公開面がスライス+f64 のみで pulp が al-math 内部に封じ込め
  られ、将来のバックエンド変更が al-math に閉じること
- 長さ不整合の呼出しバグが min-len で黙って丸められず、原因
  箇所で panic として検出されること
- `corr0`/`rms` の縮退入力契約(0.0 写像)が現行契約どおり
  維持されること
- Scalar 経路が逐次和とビット一致する基準点が保存され、
  dispatch 経路の差が明示的な許容誤差 predicate で回帰検出
  できること
- al-analysis が `dsp` ファサードのみ経由し、依存追加が発生
  しないこと
- 残存許容リスト以外に同一式の独立実装が残らず、重複解消の
  判定が列挙可能であること
- プリミティブの性能が criterion で再現計測でき、置換前後比・
  residual 再構成の採否根拠が記録として残ること

## Non-goals

- FFT・相関カーネルのアルゴリズム変更(issue #42 の別チェーン)
- `al-viz` `lod.rs` の incremental `sum_sq`(streaming 契約が
  異なる別境界)
- lag 走査そのもののプリミティブ化・`ncc` 置換
- ベクトル/行列の抽象型の導入
- 入出力スライスのエイリアシング(プリミティブは alias を
  許容しない;in-place は呼出し側で `to_vec` 等)
- criterion ベンチの CI 回帰ゲート
- x86-v4 等 `["std","x86-v3"]` 以外の pulp feature 経路の検証
- 小ベクトルへの per-call dispatch コストの改善(観測時に
  呼出し側 batch 化等を別途検討)

## Risk

- この保証を失うと、重複実装が再分散して修正が多重化し、
  min-len 黙認や 0 除算の欠落(`ls_gain` の NaN・
  `pair_correlation` の sx/sy 喪失)が下流の Validity ゲート・
  feature 値に混入し得る。Scalar 基準の喪失は SIMD 回帰の検出
  不能を意味し、残存許容の境界喪失は「置換完了」の判定不能を
  招く
