---
id: "ADR-034EoHr8hchpVwmk758yL4"
type: "adr"
status: "accepted"
title: "al-math vec プリミティブ層のスライス公開境界と dsp ファサード消費経路"
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
    - "al-math-api-fft-split-re-im.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/al-math-vec-primitives.md"
    - "simd-reduction-lane-fma.md"
    - "spectral-envelope-fft-size-assert.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/al-math-simd-reduction.md"
  derived-by:
    - "../designs/al-math-vec-primitives.md"
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

# al-math vec プリミティブ層のスライス公開境界と dsp ファサード消費経路

## Context and Problem Statement

al-locate・al-analysis に分散する f64 ベクトル演算(`corr0`/`rms`/
`energy`/`dot`/`scale`/re²+im² 合成)を `al-math` の vec
プリミティブに集約するにあたり、(1) どこに置きどう公開するか
(公開境界)、(2) 下流 crate がどの経路で消費するか(消費経路)、
(3) 2入力の長さ不整合をどう扱うか(契約)を決める必要がある。

制約: pulp は al-math 内部依存に封じ込める方針が既決
(`simd-arithmetic-pulp.md`、split re/im 境界で公開面から pulp 型を
除去済み)。al-analysis は al-math に直接依存せず、DSP は
`al_locate::dsp` ファサード経由が現行規約(`ncc`/`gcc_phat`/
`bandpass`/`spectral_envelope` は `pub use al_math::...` の
re-export、`fft` は dsp.rs:31-38 の手書き wrapper fn で
`Fft::new`+`transform` を呼ぶもので re-export ではない)。
al-analysis への al-math 依存追加は規約上「依存の追加・変更」の
確認対象。
長さ不整合は現行 corr0 が `zip` による min-len 黙認で、呼出しバグを
隠す(discovery の棚卸し・確定事項に基づく)。

## Decision Drivers

* 公開 API のバックエンド中立化(`&[f64]` スライスのみ)という
  前段決定(split re/im 境界)の継続 — vec 層でも pulp 型を
  公開面に出さない
* dsp ファサードによる消費経路の一元化を維持し、al-analysis に
  依存変更を発生させない
* 「fail, never guess」: 長さ不整合の呼出しバグを min-len で
  黙って丸めない。`spectral_envelope` の `fft_size` 入口 assert
  の前例と一致させる
* 公開するプリミティブ全てに実呼出し箇所が存在する
  (YAGNI 適合、discovery 棚卸し表)
* `linalg.rs` の内部 `dot`(既存の内部 SIMD 版)と公開
  プリミティブを同一カーネルに統一し、実装の二重化を避ける

## Considered Options

公開境界・消費経路:

* `al_math::vec` 新モジュール + `al_locate::dsp` re-export
* al-analysis に al-math 依存を追加して直接利用
* dsp 内部のみに公開(al-math 公開面を増やさない)

長さ不整合の契約:

* panic(等長 assert) — 入口で契約違反を検出
* min-len(`zip`) — 現行 corr0 と同じ黙認

## Decision Outcome

Chosen option: "`al_math::vec` 新モジュール + `al_locate::dsp`
re-export、2入力は等長 assert で panic", because 既存 dsp
ファサードパターンと一貫し al-analysis は依存追加なしで使え、
公開 API をスライス境界に保つことで pulp が内部に封じ込められた
ままとなり、panic が呼出しバグを原因箇所で検出するため。

契約の確定内容:

* **配置・公開**: `al_math::vec` 新モジュールに
  `dot` / `sum_sq` / `corr0` / `rms` / `scale` / `axpy` /
  `sub_scaled` / `mag_sq` を集約する。`corr0` は Σxy・Σx²・Σy²
  の3アキュムレータを1パスで融合した単一関数とする
* **公開シグネチャ**: `&[f64]`/`&mut [f64]` スライスと f64
  スカラーのみ。`scale`/`axpy`/`sub_scaled` の乗数引数 `a` は
  f64 スカラー乗数(要素ベクトルではない)。`mag_sq` は
  `(re: &[f64], im: &[f64], out: &mut [f64])` の3スライス形。
  pulp 由来の型・トレイト(`Simd`・`WithSimd` 等)は公開面に
  出さない。dispatch は公開入口で1回、カーネルは `Simd`
  ジェネリックな単一ソース(前 spec の規約を継続)
* **消費経路**: `al_locate::dsp` が vec プリミティブを
  re-export する唯一の消費経路。al-analysis は `al_locate::dsp`
  経由でのみ利用し、`crates/al-analysis/Cargo.toml` に al-math
  依存を追加しない。`al_locate` crate root には既存の dsp
  シンボル mirror 規約(`lib.rs:22` の `pub use dsp::{...}`)
  があり、vec シンボルもこれに倣って root で mirror する —
  正規経路は `dsp::` のまま、root mirror は公開面の一貫性の
  ための re-export のみとする
* **長さ不整合**: 2入力プリミティブ(`dot`/`corr0`/`axpy`/
  `sub_scaled`/`scale`)は入出力スライスの等長を assert で
  強制し、`mag_sq` は `re`/`im`/`out` の **3スライス全て** が
  等長でなければ panic。現行 corr0 の min-len(`zip`)黙認は継承しない
* **値の契約**: `corr0` は `Σx² > 0 かつ Σy² > 0` でなければ
  0.0 を返す(NaN・縮退エネルギーも 0.0 に写像する現行契約を
  維持)し中心化しない。`rms` は空入力で 0.0 を返す
* **カーネル統一**: `linalg.rs` の内部 `dot` は共有カーネル化
  して公開プリミティブと同一実装とする。`linalg.rs` の
  `reflect`(:15-26、`*t = t - factor·v` の形)は
  `sub_scaled` の当然のカーネルレベル消費者であり、residual
  再構成を見送っても `sub_scaled` の実呼出し箇所が存在する
* **residual 減算ループ**: `residual.rs` の転置アクセス形
  減算ループは、計測で安全性と利益を確認した上で per-source
  `sub_scaled` 形への再構成を許容する(条件付き、hypothesis)。
  同所の列 gather dot(:166、`s[t]` の strided アクセス)は
  `&[f64]` スライス API では直接供給できず、本再構成の採用時に
  のみ解消される

### Consequences

* Good, because dsp ファサードの一元化が維持され、al-analysis
  は依存追加なしでプリミティブを使える
* Good, because 公開境界がスライスのみのため、vec 層追加でも
  pulp の内部封じ込めが保たれ、将来のバックエンド変更が
  al-math 内に閉じる
* Good, because 等長 assert が呼出し側のスライス切り出しバグを
  隠さず、fft_size assert と同じ panic 契約の系に揃う
* Good, because `linalg` の内部 dot と公開プリミティブが同一
  実装になり、SIMD 改善が両者に自動的に及ぶ
* Bad, because panic 契約のため、長さを動的に決める呼出し側は
  等長を事前に保証する必要がある(既存呼出しは全て等長前提で
  あり影響なし)
* Bad, because al-locate を経由しない将来の消費者は al-math を
  直接使う形になる(現状そのような消費者は存在しない)

### Confirmation

* `al_math::vec` の公開シグネチャに pulp 型が出現しないこと、
  および `al_math::vec` が `al_locate::dsp` から re-export
  されていることをコード検査で確認する
* `mise exec -- cargo tree -p al-analysis --depth 1` の直接依存に
  al-math が現れないこと(または `crates/al-analysis/Cargo.toml`
  の直接確認)を確認する。`cargo tree -p al-analysis` の全表示は
  al-locate 経由の推移的依存として常に al-math を含むため
  判定には使えない
* 等長でない入力で panic する `#[should_panic]` テスト、
  `corr0` が `Σx²>0 かつ Σy²>0` でない場合→0.0・`rms` 空→0.0
  のテストが存在し pass することを確認する

## Implementation Plan

* Affected paths: `crates/al-math/src/vec.rs`(新設)、
  `crates/al-math/src/lib.rs`(モジュール公開)、
  `crates/al-math/src/linalg.rs`(内部 dot の共有カーネル化)、
  `crates/al-locate/src/dsp.rs`(re-export)、
  `crates/al-locate/src/methods.rs`、`residual.rs`、`mixer.rs`、
  `crates/al-locate/src/lib.rs`(crate root の dsp mirror 規約に
  倣った vec シンボル mirror)、
  `crates/al-analysis/src/cheap.rs`、`reference.rs`、
  `band_gain.rs`、`stft.rs`、`expensive/mod.rs`、
  `expensive/timing.rs`(`super::{corr0, rms}` の消費側)、
  `expensive/ir.rs`、`expensive/dynamics.rs`(重複実装の置換。
  棚卸しの網羅表は discovery、置換対象/残存許容の確定列挙は
  spec を参照)
* Patterns to follow: `dsp.rs` 既存の `pub use al_math::...`
  re-export パターン、FFT 系 API の入口 assert と `rejects_*` /
  `#[should_panic]` テスト方針、dispatch を公開入口1回に集約し
  カーネルを `Simd` ジェネリックで書く既存規約(linalg dot の
  lane acc + scalar tail パターン)
* Patterns to avoid: al-analysis への al-math 依存追加、
  min-len(`zip`)による黙認の継承、vec プリミティブと同式を
  消費側で再実装すること、pulp 型を公開シグネチャに出すこと、
  ベクトル/行列の抽象型の導入(YAGNI)
* Dependencies or configuration: 新規依存なし(criterion は
  observability foundation で al-math dev-dep として導入済み。
  ベンチは `dsp_smoke` 拡張または新 bench target)

## Verification

* [ ] `al_math::vec` の各プリミティブが `&[f64]`/`&mut [f64]`
  スライスと f64 スカラーのみの公開シグネチャを持ち、pulp 型が
  公開面に出ない
* [ ] `al_locate::dsp` から vec プリミティブが re-export され、
  al-analysis が `al_locate::dsp` 経由で利用している
* [ ] `mise exec -- cargo tree -p al-analysis --depth 1` の直接
  依存に al-math がない(または Cargo.toml の直接確認)
* [ ] 等長でない入力の `#[should_panic]` テストと、
  ゼロエネルギー/空入力の契約テストが pass する
* [ ] `linalg.rs` の内部 `dot` と公開 `dot` が同一カーネルを
  共有している
* [ ] `mise exec -- cargo test -p al-math -p al-locate -p al-analysis`
  が合格する

## Pros and Cons of the Options

### `al_math::vec` + `al_locate::dsp` re-export

* Good, because 既存 dsp ファサードパターンと一貫し、
  al-analysis は依存追加なしで使える。公開境界が1箇所で
  管理される
* Bad, because al-locate を経由しない将来の消費者は al-math を
  直接使う形になる(現状問題なし)

### al-analysis に al-math 依存追加

* Good, because 直接利用で素直
* Bad, because 規約上の「依存の追加・変更」確認対象が発生し、
  dsp ファサードの一元化が崩れて消費経路が2系統になる

### dsp 内部のみ公開

* Good, because al-math 公開面を増やさない
* Bad, because al-math 内の再利用(linalg dot 等)と公開 API が
  分離し一貫性に欠け、al-locate 内の重複(methods.rs 等)を
  集約できない

### panic(等長 assert)

* Good, because 「fail, never guess」方針・fft_size assert の
  前例と一致し、スライス切り出し忘れを隠さない
* Bad, because 契約違反が実行時 panic であり、動的長の呼出しは
  事前に等長を保証する必要がある

### min-len(`zip`)継続

* Good, because 現行 corr0 と同じで互換
* Bad, because 呼出しバグを黙って丸め、誤った相関値が下流の
  ゲート・指標に混入し得る

## More Information

* 確定経緯・棚卸し表・代替案比較:
  `docs/discovery/al-math-simd-reduction.md`
* 実施スコープ・受入基準: `docs/specs/al-math-vec-primitives.md`
* 上位決定: `docs/adr/simd-arithmetic-pulp.md`(pulp は内部依存)、
  `docs/adr/al-math-api-fft-split-re-im.md`(公開面の
  バックエンド中立化)
* panic 契約の前例: `docs/adr/spectral-envelope-fft-size-assert.md`
* reduction 順序の数値ポリシー:
  `docs/adr/simd-reduction-lane-fma.md`
