---
id: "ADR-034EoHxzsvsHp9BOdCK2Cn"
type: "adr"
status: "accepted"
title: "SIMD reduction 順序は lane アキュムレートと FMA を許容し等価性を許容誤差で定義する"
created: "2026-10-12"
updated: "2026-10-12"
owners:
  - "scarecrow173"
relations:
  source:
    - "../discovery/al-math-simd-reduction.md"
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
    - "simd-arithmetic-pulp.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/al-math-vec-primitives.md"
    - "al-math-vec-primitives-slice-boundary.md"
    - "al-math-api-fft-split-re-im.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/al-math-simd-reduction.md"
  derived-by:
    - "../designs/al-math-vec-primitives.md"
    - "../tasks/al-math-vec-primitives.md"
    - "../tasks/vec-primitives-verification-bench.md"
    - "../tasks/caller-opt-bench-baseline-characterization.md"
    - "../tasks/caller-opt-least-squares-column-api.md"
    - "../tasks/caller-opt-dsp-conv-overlap-save.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers:
      - "scarecrow173"
    consulted: []
    informed: []
---

# SIMD reduction 順序は lane アキュムレートと FMA を許容し等価性を許容誤差で定義する

## Context and Problem Statement

`al_math::vec` のベクトルプリミティブを SIMD 化するにあたり、
reduction(Σx²、Σa·b 等の総和)の加算順序ポリシーを決める必要が
ある。これは `docs/specs/al-math-api-boundary.md` の Deferred
Design Concerns で「reduction 順序ポリシー(lane 単位 reduce の
許容)を決定する必要がある」と未決のまま明示された論点。

逐次の `.iter().sum()` と SIMD の lane アキュムレート+FMA は加算順が
異なり ~ulp(10⁻¹⁶)級の差が出得る。順序を固定すれば新旧の差分は
ゼロになるが、直列依存を維持するため SIMD 化の効果がほぼ消失し、
「性能向上」を重複解消と同等の主目的とする方針と矛盾する。一方で
ビットを問わない緩い基準では、scalar 基準の回帰検出が失われる。
下流の耐性は確認済み: golden テストは位置比較(±64 samples)で
float 厳密比較ではなく、Validity ゲート閾値(corr 0.3、
`MIN_ENERGY = 1e-9`・methods.rs:16 等)は想定差分より6〜12桁
大きい(timing.rs:216,:276 の 1e-4 リテラルは `MIN_ENERGY`
ではなく別のローカル閾値)。

## Decision Drivers

* 主目的が「重複解消」と「性能向上」の両方であること — 順序固定
  は直列依存の維持を意味し SIMD 効果をほぼ殺す
* scalar 経路(`pulp::Scalar`、1レーン)は逐次和と同順序にできる
  ため、現行値とのビット一致を「スカラ基準」として保存できる
* dispatch 経路の最終ビットは ISA/環境に依存し得るが、下流の
  永続化値・ゲートは ~ulp 級の揺れを許容できる(閾値との桁差が
  確認済み)
* 複素乗算(cmul)で既に FMA を許容する判断をしており、丸め方針の
  一貫性を保つ

## Considered Options

* 案A: lane アキュムレート + FMA 許容、等価性は許容誤差で定義
* 案B: lane アキュムレートのみ許容(FMA 不使用)
* 案C: bitwise 一致要求(順序固定)

## Decision Outcome

Chosen option: "案A", because レーン並列+積和融合で最大の性能が
得られ、丸め回数が減るため精度も改善方向であり、cmul の FMA
判断と一貫するため。scalar 経路が逐次和とビット一致を保つため
回帰検出の基準点も失われない。

ポリシーの確定内容:

* **reduction 順序**: lane アキュムレータによる並列累算と FMA
  (fused multiply-add)を許容する。レーン数・端数処理の順序は
  dispatch された ISA に依存してよい
* **カーネルで使う pulp op**: リダクションカーネルの積和は
  `mul_add_e_f64s` を使うこと。`pulp::Scalar` では非融合の
  `a * b + c` に下り逐次和のビット一致を保ち、x86-v3/v4 では
  真の FMA(`_mm256_fmadd_pd`)に下りる
* **等価性の定義**: dispatch 経路と scalar 経路の結果が一致
  するとは、スカラー出力(`dot`/`sum_sq`/`corr0`/`rms`)で
  `|a - b| <= 1e-12 * max(1.0, |a|, |b|)` を満たし、ベクトル
  出力(`scale`/`axpy`/`sub_scaled`/`mag_sq`)では全要素が同
  predicate を満たすことと定義する。float の `assert_eq!`
  厳密比較は経路間比較には使わない
* **scalar 基準の保存**: `pulp::Scalar` 経路は逐次和と同じ
  順序で計算し、現行 `.iter().sum()` 系コードの結果と
  ビット一致を維持する。このビット一致は **プリミティブ単位・
  同一データ上** の保証であり、プリミティブを再合成した式
  (例: per-channel `sum_sq` の和によるマルチch rms)では
  結合順が変わるため上記許容誤差ポリシー内での差異を許容する
  — 永続化値(ResidualEnergy 等)は ~ulp 級に揺れ得る
* **下流の許容**: 下流の永続化値(manifest・feature record・
  解析 dataset)と Validity ゲートは「環境非依存でない最終
  ビット」を許容する。閾値付近で挙動が変わる可能性は想定差分と
  閾値の桁差(6〜12桁)により実害なしと判断する
* **テスト方針**: 既存テストで float を `assert_eq!` 厳密比較
  している箇所は planning で棚卸しし、経路間比較に該当するもの
  は許容比較へ修正する。経路間比較テストのサイズ行列は
  0..=33(空・1要素・lane 端数の全経過)・1k・64k・~1M とする。
  本ビルドの pulp features は `["std","x86-v3"]`
  (crates/al-math/Cargo.toml)であり、検証される dispatch 経路は
  x86-v3 と Scalar のみ。golden 出力の意味・閾値は変更しない

### Consequences

* Good, because lane 並列と FMA によりホット経路(全長
  signal/template の energy、lag 走査内の相関等)の性能が最大化
  され、丸め回数の削減で精度も改善方向になる
* Good, because scalar 経路が現行値とビット一致を保ち、
  「スカラ基準との差が許容誤差以内」という検証可能な等価性の
  定義が得られる
* Good, because 順序ポリシーが vec 層に閉じ、FFT/相関カーネル
  のアルゴリズム変更(issue #42)とは独立に適用できる
* Bad, because dispatch 経路の最終ビットが ISA/環境に依存し、
  永続化値が ~ulp 級に揺れ得る(確認済みの桁差で実害なし)
* Bad, because 経路間を厳密比較する既存テストは許容比較への
  修正が必要になり、棚卸しコストが発生する

### Confirmation

* scalar 強制経路の結果が従来の逐次和とビット一致するテストが
  pass すること
* dispatch 経路と scalar 経路の差が許容誤差 predicate
  (`|a - b| <= 1e-12 * max(1.0, |a|, |b|)`、ベクトル出力は
  要素毎)を満たすことを、サイズ行列(0..=33・1k・64k・~1M)
  で検証するテストが pass すること
* 既存の golden テスト・Validity ゲートを含む
  `mise exec -- cargo test --workspace` が pass すること

## Implementation Plan

* Affected paths: `crates/al-math/src/vec.rs`(リダクション
  カーネル)、`crates/al-math/src/linalg.rs`(共有カーネル)、
  al-math 内の経路間比較テスト、厳密比較している float テスト
  (棚卸し後に許容比較へ修正)
* Patterns to follow: `linalg.rs` 既存の内部 `dot` と同じ
  lane アキュムレータ + scalar tail の構造、
  `pulp::Arch::new()` + `dispatch(WithSimd)` による公開入口1回の
  dispatch、scalar 強制経路と dispatch 経路の一致検証モデル
  (一致の定義を厳密から許容誤差へ拡張して適用)
* Patterns to avoid: 順序固定による bitwise 一致の強制(SIMD
  効果の消失)、経路間比較での `assert_eq!`、許容誤差の暗黙の
  拡大(1e-12 スケールを超える許容は再決定事項)、下流での
  差分吸収のための二重丸め・閾値調整。カーネル記述では
  `mul_add_f64s` を使わないこと(Scalar でも常に融合 FMA となり
  scalar 逐次和のビット一致契約を破る)。また `mul_f64s` +
  `add_f64s` の素直な2命令も避けること(FMA を失い速度・精度
  とも損なう)— 積和は必ず `mul_add_e_f64s` で書く
* Dependencies or configuration: 新規依存なし

## Verification

* [ ] scalar 強制経路の各プリミティブが従来の `.iter().sum()`
  系とビット一致するテストが存在し pass する
* [ ] dispatch 経路と scalar 経路の差が許容誤差 predicate
  (`|a - b| <= 1e-12 * max(1.0, |a|, |b|)`、ベクトル出力は
  要素毎)を満たすことを検証するテストが、サイズ行列
  (0..=33・1k・64k・~1M)で pass する
* [ ] `mise exec -- cargo test --workspace` が golden・
  Validity ゲートを含めて pass する
* [ ] 厳密比較から許容比較への修正が必要だった float テストの
  一覧が Implementation Record に記録される

## Pros and Cons of the Options

### 案A: lane アキュムレート + FMA 許容

* Good, because 最大性能(レーン並列+積和融合)で丸め回数が減り
  精度も改善方向。cmul の FMA 判断と一貫
* Bad, because dispatch 経路の最終ビットが環境依存(~ulp 級)
  となり、等価性を許容誤差で定義する必要がある

### 案B: lane アキュムレートのみ(FMA 不使用)

* Good, because lane 順序差のみで差分の出所が1つ少ない
* Bad, because FMA を捨て速度・精度とも僅かに損し、それでも
  lane 順序差は残るため許容誤差は結局必須

### 案C: bitwise 一致要求

* Good, because 新旧の差分が定義上ゼロ
* Bad, because 順序固定は直列依存の維持を意味し SIMD 効果が
  ほぼ消失し、性能向上の主目的と矛盾する

## More Information

* 確定経緯・数値影響の確認(ulp 級差、golden は位置比較、
  ゲート閾値との桁差): `docs/discovery/al-math-simd-reduction.md`
* 実施スコープ・受入基準: `docs/specs/al-math-vec-primitives.md`
* 公開境界・消費経路:
  `docs/adr/al-math-vec-primitives-slice-boundary.md`
* 上位決定: `docs/adr/simd-arithmetic-pulp.md`(pulp の dispatch
  基盤)、`docs/adr/al-math-api-fft-split-re-im.md`(scalar/dispatch
  一致検証モデルの前段)
* reduction 順序の解説資料 `target/explain-reduction-order.html`
  はユーザー確認用の一時資料でコミット対象外
