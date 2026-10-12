---
id: "SPEC-034EoHgAtLogjk6xxhiJWm"
type: "spec"
status: "approved"
title: "al-math vec プリミティブ集約と dsp ファサード経由の公開"
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
  implemented-by:
    - "../designs/al-math-vec-primitives.md"
    - "../plans/al-math-vec-primitives.md"
  depends-on:
    - "./observability-foundation.md"
    - "../adr/simd-arithmetic-pulp.md"
    - "../adr/al-math-api-fft-split-re-im.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/al-math-vec-primitives-slice-boundary.md"
    - "../adr/simd-reduction-lane-fma.md"
    - "../adr/spectral-envelope-fft-size-assert.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/al-math-simd-reduction.md"
  derived-by:
    - "../designs/al-math-vec-primitives.md"
    - "../tasks/vec-primitives-verification-bench.md"
  verifies: []
  verified-by:
    - "../test-specs/al-math-vec-primitives.md"
  references: []
  defers: []
  deferred-by: []
---

# al-math vec プリミティブ集約と dsp ファサード経由の公開

## What Should Be Built

`al-math` に `vec` モジュールを新設し、f64 スライスのベクトル演算
プリミティブを公開 API として集約する。公開集合は
`dot`(Σa·b) / `sum_sq`(Σx²) / `corr0`(中心化しない正規化相関、
1パス3アキュムレータ融合) / `rms`(√(Σx²/n)) / `scale`(x·a) /
`axpy`(y+=a·x) / `sub_scaled`(y-=a·x) / `mag_sq`(re²+im²→out)。
公開シグネチャは `&[f64]`/`&mut [f64]` スライスのみとし、pulp は
al-math 内部に封じ込める。消費経路は `al_locate::dsp` ファサードの
re-export に一本化し、al-analysis は al-math に直接依存しない。
al-locate・al-analysis に散在する `corr0`/`rms`/`energy`/`dot`/
`scale`/re²+im² 合成の重複実装をプリミティブ呼出しに置き換える。
SIMD/dispatch 経路と scalar 経路の等価性は明示的な許容誤差
predicate(Requirements 参照)で定義し、scalar 経路は現行
`.iter().sum()` の逐次和と同順序でビット一致を維持する。
criterion ベンチでプリミティブの性能を計測し、実測値を記録する。

> プリミティブ集合・長さ panic 契約・ゼロ入力規則の権威は
> `docs/adr/al-math-vec-primitives-slice-boundary.md`、reduction
> 順序と等価性 predicate の権威は
> `docs/adr/simd-reduction-lane-fma.md` とする。本 spec 内の
> 再掲が ADR と乖離した場合は ADR を正とし、spec 側を追従修正
> する(契約文面の二重管理によるドリフト防止)。

## Why Now

`docs/specs/al-math-api-boundary.md` の Deferred Design Concerns で
次 spec として予告された残課題。同一式の f64 リダクションが複数
crate に3重以上重複しており単一の権威実装がない(discovery の棚卸し
表: `corr0` は `expensive/mod.rs:264`・`reference.rs:134`・
`cheap.rs` 内3箇所のインライン、`rms` は `expensive/mod.rs:257`・
`reference.rs:119`・`residual.rs:133` の3実装、`energy`/`sum_sq`
は `methods.rs:142`・`mixer.rs`・`band_gain.rs` 等に分散)。
加えて全て逐次 `.iter().sum()` のスカラ実装のままで、
`methods.rs` preflight の全長 signal/template energy のように毎回
実行されるホット経路が SIMD 化されていない。主目的は「重複解消」
と「性能向上」を同等に扱う(ユーザー確定)。先行する observability
foundation(criterion 0.8.2 dev-dep + `dsp_smoke` bench、stage
`duration_ms`)がマージ済みで、計測基盤の前提が揃った。

## Users and Value

- al-locate / al-analysis の実装者: 同一式のコピーをメンテナンス
  する必要がなくなり、`dsp::` から一貫した契約(等長 panic・
  ゼロ入力規則)のプリミティブを呼べる
- 性能最適化の担当者: スカラリダクションのホット経路が pulp
  dispatch 経由の SIMD 実装に置き換わり、criterion で改善量を
  計測できる。#42(FFT/相関カーネル)等の後続 SIMD 施策の
  ベンチ足場にもなる
- 将来のバックエンド変更担当者: vec 境界でも pulp 型が公開面に
  出ないため、バックエンド置き換えが al-math 内部に閉じる

## Scope

### In Scope

- `crates/al-math/src/vec.rs` 新モジュールと公開プリミティブ:
  `dot`・`sum_sq`・`corr0`・`rms`・`scale`・`axpy`・
  `sub_scaled`・`mag_sq`。`corr0` は Σxy・Σx²・Σy² の
  3アキュムレータを1パスで融合する
- `al_locate::dsp` への re-export 追加(既存の
  `pub use al_math::...` パターン)。加えて `al_locate` crate
  root の既存 mirror 規約(`lib.rs` の `pub use dsp::{...}`、
  :22)に倣い、vec シンボルも crate root で mirror する —
  正規の消費経路は `al_locate::dsp` のままとし、root mirror は
  dsp 公開面との一貫性のための re-export のみ。al-analysis への
  al-math 依存追加は行わない
- 重複実装の置換(discovery 棚卸し対象。file:line は現行
  コード基準の目安):
  - `crates/al-locate/src/methods.rs`: `energy`(:142)、
    preflight の全長 signal/template `energy` 呼出し
    (:175,:183)、`normalize_gain`(:500、`rms`→`scale` 合成)、
    envelope NCC のインライン corr 合成(:541-544)
  - `crates/al-locate/src/mixer.rs`: インライン `rms`(:74)
  - `crates/al-locate/src/residual.rs`: `&[Vec<f64>]` マルチch
    `rms`(:133、per-channel `sum_sq` の合成で再表現 —
    プリミティブ自体は `&[f64]` 単一ch のみ)
  - `crates/al-analysis/src/cheap.rs`: インライン `rms`
    (:180,:254,:358,:414)、
    mean-square(:385)、`sum_sq`(:912,:921)、インライン
    corr0 ×3(`stereo.pair_correlation` :1017-1023、
    `stereo.ipd_samples` lag 走査 :1067-1071、
    `trans.self_similarity` :1250-1252)。
    `pair_correlation` は sx/sy アキュムレータを Validity
    判定(:1030)と `stereo.ild_db`(:1042)でも消費するため
    融合 `corr0` 単独では供給できない — `corr0` + 個別
    `sum_sq` 呼出し、または生アキュムレータ保持で対応し、
    必要な値を落とさないこと
  - `crates/al-analysis/src/reference.rs`: `rms`(:119)・
    `corr0`(:134)・`ls_gain` の dot/sum_sq 合成(:127-131)、
    マルチch `energy` クロージャ(:1066、per-channel `sum_sq`
    合成)と `res_rms` 合成(:1164)
  - `crates/al-analysis/src/expensive/mod.rs`: `rms`(:257)・
    `corr0`(:264)
  - `crates/al-analysis/src/expensive/timing.rs`:
    `super::{corr0, rms}` を import(:11)し `corr0` を
    :92,:148,:397(lag 走査内)の3箇所、`rms` を
    :172,:216,:276 の3箇所で使用 — 置換対象
  - `crates/al-analysis/src/expensive/dynamics.rs`: `rms`
    呼出し(:86,:149,:350,:351)
  - `crates/al-analysis/src/expensive/ir.rs`: split 済み
    re/im 上の `mag2` 合成(:172)→ `mag_sq`
  - `crates/al-analysis/src/band_gain.rs`: `sum_sq`(:94,:101)
    と window dot(:96)
  - `crates/al-analysis/src/stft.rs`: `power()`(:78-84)・
    `psd()`(:47-61、`mag_sq` + `scale` 合成)・`magnitude()`
    (:64-76、`mag_sq` + 要素毎 sqrt)の同一 re²+im² 合成
- `residual.rs` の減算ループ(転置アクセス形 :163-169)の
  per-source `sub_scaled` 再構成 — 計測で安全性と利益を確認
  した上で実施する条件付きスコープ。同所の列 gather dot
  (:166、`s[t]` の strided アクセス)は `&[f64]` スライス API
  では直接供給できず、この再構成を採用した場合にのみ
  per-source `sub_scaled` 呼出しとして解消される。採否の根拠は
  計測値とともに記録する
- `linalg.rs` の内部 `dot` を共有カーネル化し、公開プリミティブ
  と同一実装に統一する。`reflect`(:15-26、`*t = t - factor·v`
  の形)は residual 再構成の採否に依らず `sub_scaled` の当然の
  カーネルレベル消費者となる(YAGNI の懸念はこれで解消)
- scalar(`pulp::Scalar`)経路の逐次和ビット一致維持と、
  dispatch 経路との許容誤差等価の検証(等価性の定義は
  `docs/adr/simd-reduction-lane-fma.md` が権威)
- vec プリミティブの criterion ベンチ追加(既存 `dsp_smoke`
  target の拡張または新 bench target)。対象・サイズ分布は
  design で確定し、実測値を Implementation Record に記録する

### 残存許容(residual allowance)

`&[f64]` スライス API では供給できない、または同一式でない
複合式として、重複解消の対象外とし残存を明示的に許容する箇所。
以下は AC の「独立実装が残存しない」判定から除く:

- `expensive/ir.rs` のタプルスライス上の合成(:155-161)—
  `pair.transfer_h` が `Option<Vec<(f64, f64)>>`(re/im タプル)
  であり、re/im 分離前のため `&[f64]` API に乗らない
- `residual.rs` の列 gather dot(:166)— 上記条件付き再構成を
  採用した場合のみ解消される
- `al-viz` `lod.rs` の incremental `sum_sq` — Out of Scope
  (streaming 契約が異なる)
- `Vec<Vec<f64>>` / `Vec<(f64,f64)>` 上の合成で、呼出し側が
  データを再構成しない限り残るもの: `residual.rs` の
  Frobenius ノルム(:221-225、`matrix` 行の flat_map)、
  `reference.rs` の `Vec<(f64,f64)>` 上の magnitude 合成
  (:550,:621)と off-diagonal `sum_sq`(:908)
- 同一式でない複合式(置換対象外): `cheap.rs` の mid/side
  電力(:1096-1101)、magnitude 総和(:1148)、スペクトル傾斜の
  4アキュムレータ回帰 fold(:811)、`sqrt(mean(mom))`(:390 —
  `mom` はフレーム毎 mean-square で `rms` 式でない、
  mean プリミティブが不在)、`dynamics.rs` の符号別
  エネルギー split(:57-61)・要素毎 magnitude(:199)・
  回帰アキュムレータ(:373-389)、`reference.rs` の単要素/
  バンド合成(:535,:839)と M/S エネルギー(:872-881 —
  `Σ((a±b)/2)²`、materialization を要する複合式)

### Out of Scope

- FFT・相関カーネルのアルゴリズム変更(issue #42 の別チェーン)
- `al-viz` `lod.rs` の incremental `sum_sq` — streaming 契約が
  異なり本プリミティブ境界に乗らない
- lag 走査そのもののプリミティブ化(ncc 等への置換を含む)
- ベクトル/行列の抽象型の導入
- al-analysis への al-math 直接依存の追加
- pulp 型・`Simd` トレイトの公開面への露出
- golden テストの判定意味・Validity ゲート閾値の変更

## Requirements

- `al_math::vec` の公開 API は `&[f64]`/`&mut [f64]` スライスと
  f64 スカラーのみとし、pulp 由来の型・トレイトが出現しないこと。
  公開シグネチャの形:
  `dot(a: &[f64], b: &[f64]) -> f64` /
  `sum_sq(x: &[f64]) -> f64` / `corr0(x: &[f64], y: &[f64]) -> f64` /
  `rms(x: &[f64]) -> f64` /
  `scale(x: &[f64], a: f64, out: &mut [f64])` /
  `axpy(y: &mut [f64], a: f64, x: &[f64])` /
  `sub_scaled(y: &mut [f64], a: f64, x: &[f64])` /
  `mag_sq(re: &[f64], im: &[f64], out: &mut [f64])`。
  `scale`/`axpy`/`sub_scaled` の `a` は f64 スカラー乗数であり、
  ベクトルではない
- 2入力プリミティブ(`dot`/`corr0`/`axpy`/`sub_scaled`/
  `scale`)は入出力スライスの長さ不整合を等長 assert で
  panic とし、`mag_sq` は `re`/`im`/`out` の **3スライス
  全て** が等長でなければ panic とする。「fail, never guess」の方針と
  `fft_size` assert の前例に揃える。現行 corr0 の
  min-len(zip)黙認は継承しない
- `corr0` は `Σx² > 0 かつ Σy² > 0` でなければ 0.0 を返す
  (NaN・縮退エネルギーも 0.0 に写像する現行契約を維持)し、
  中心化しない(zero-mean でない正規化相関)。`rms` は空入力で
  0.0 を返す。いずれも現行契約の維持
- scalar 経路は現行 `.iter().sum()` 系と同じ逐次和順序で結果を
  ビット一致させる。このビット一致は **プリミティブ単位・
  同一データ上** の保証であり、プリミティブを再合成した式
  (例: per-channel `sum_sq` の和で求めるマルチch rms)は
  結合順が変わるため、許容誤差ポリシー内での差異を許容する
  (永続化値 `ResidualEnergy` 等は ~ulp 級に揺れ得る)。
  dispatch 経路は lane アキュムレータ+FMA を許容し、等価性は
  下記の許容誤差 predicate で定義する(詳細は
  `docs/adr/simd-reduction-lane-fma.md` が権威):
  - スカラー出力(`dot`/`sum_sq`/`corr0`/`rms`):
    `|a - b| <= 1e-12 * max(1.0, |a|, |b|)`
  - ベクトル出力(`scale`/`axpy`/`sub_scaled`/`mag_sq`):
    上記 predicate を要素毎に適用
  - 検証サイズ行列: 0..=33(空・1要素・lane 端数の全経過)、
    1k、64k、~1M。本ビルドの pulp features は
    `["std","x86-v3"]` であり、検証される dispatch 経路は
    x86-v3 と Scalar のみ(v4 等は feature 追加時に拡張)
- dispatch の呼出しは演算の公開入口で1回とし、カーネルは
  `Simd` ジェネリックな単一ソースを維持する(前 spec の規約を
  vec 層にも適用)
- al-analysis は `al_locate::dsp` 経由でのみプリミティブを消費
  し、`crates/al-analysis/Cargo.toml` に al-math 依存を追加し
  ないこと
- 置換後も既存の分析意味・golden 出力・manifest/feature record
  の形式を変更しない。float の `assert_eq!` 厳密比較をしている
  既存テストは許容比較への修正範囲を planning で棚卸しする
- criterion ベンチでプリミティブ単体の計測が可能であり、導入
  前後の代表経路の実測値を Implementation Record に記録する
  こと(性能向上は重複解消と同等の主目的)

## Acceptance Criteria

- [ ] `al_math::vec` の各プリミティブ(`dot`/`sum_sq`/`corr0`/
  `rms`/`scale`/`axpy`/`sub_scaled`/`mag_sq`)が公開 API として
  存在し、公開シグネチャに pulp 型が出現しない
- [ ] `al_locate::dsp` から vec プリミティブが re-export され、
  `mise exec -- cargo tree -p al-analysis --depth 1` の直接依存に
  al-math が現れない(または `crates/al-analysis/Cargo.toml` の
  直接確認で代替可。`cargo tree -p al-analysis` の全表示は
  al-locate 経由の推移的依存として常に al-math を含むため
  判定に使えない)
- [ ] In Scope の箇条書きで列挙した重複実装がプリミティブ
  呼出しに置換され、「残存許容」リストに明示した箇所を除き
  同一の corr0/rms/sum_sq 式の独立実装が残存しない
- [ ] 2入力プリミティブおよび `mag_sq`(`re`/`im`/`out` の
  3スライス)に等長でないスライスを渡した場合に panic する
  `#[should_panic]` テストが存在する
- [ ] `corr0` が `Σx²>0 かつ Σy²>0` でない場合(NaN・縮退
  エネルギーを含む)に 0.0、`rms` が空入力で 0.0 を返すテスト
  が存在する
- [ ] scalar 強制経路の結果が従来の逐次和とビット一致し、
  dispatch 経路との差が許容誤差 predicate(スカラー出力は
  `|a - b| <= 1e-12 * max(1.0, |a|, |b|)`、ベクトル出力は
  要素毎に同 predicate)を満たすことを検証するテストが存在
  する。テストは 0..=33(空・lane 端数)・1k・64k・~1M の
  サイズ行列を含み、本ビルドの pulp features
  `["std","x86-v3"]` 下で V3-dispatch 経路と Scalar 経路を
  網羅する
- [ ] `mise exec -- cargo test -p al-math -p al-locate -p al-analysis`
  および `mise exec -- cargo test --workspace`、
  `mise exec -- cargo clippy`、`mise exec -- cargo fmt --check`
  が green である
- [ ] `mise exec -- cargo bench -p al-math` で vec プリミティブの
  criterion ベンチが実行でき、実測値が Implementation Record に
  記録される。代表呼出し経路 — `methods.rs` preflight の全長
  energy(:175,:183)、`band_gain.rs` の window dot(:94-96)、
  `cheap.rs` の corr0 呼出し(:1067-1071 lag 走査等)、
  `residual.rs` 減算ループ(:163-169)— は既存の
  `crates/al-locate/examples/bench_*`(bench_dsp・bench_ncc・
  bench_locate_prepared)の `Instant` 計測で置換前比を取り、
  criterion のノイズ幅(目安 ±5% または統計的有意差の閾値)を
  超える回帰がないことを確認する。小スライス呼出しにおける
  per-call dispatch オーバーヘッドの回帰はポリシー上許容する
  (Deferred Design Concerns の通り対応を先送り済み)
- [ ] `residual.rs` の減算ループを per-source `sub_scaled` 形に
  再構成した場合は安全性・利益の計測根拠が記録され、見送った
  場合はその理由が記録される。列 gather dot(:166)は採用時に
  限りプリミティブへ移行する

## Deferred Design Concerns

- lag 走査自体のプリミティブ化・`ncc` 置換 — アルゴリズム
  置き換えであり issue #42 系の別チェーンで扱う
- `al-viz` `lod.rs` の incremental `sum_sq` — streaming 契約が
  異なるため別境界。利用側の要件が固まった時点で再検討する
- criterion ベンチの CI 回帰ゲート — observability foundation
  spec と同様、CI 基盤未整備のため見送り。CI 整備時に baseline
  比較の導入を再検討する
- 小ベクトル(spectral_envelope の 32bin 等)への per-call
  dispatch コスト — bench 結果で問題が観測された時点で呼出し
  側の batch 化等を再検討する

## Sources and Relations

- 調査: `docs/discovery/al-math-simd-reduction.md`(重複実装の
  棚卸し表、reduction 順序・公開境界・長さ契約・計測方法の
  選択肢比較と確定事項)
- 決定: `docs/adr/al-math-vec-primitives-slice-boundary.md`
  (公開境界・消費経路・長さ panic 契約)、
  `docs/adr/simd-reduction-lane-fma.md`(reduction 順序ポリシーと
  許容誤差等価)
- 前提: `docs/specs/observability-foundation.md`(criterion
  dev-dep と `dsp_smoke` bench の先行導入)、
  `docs/adr/simd-arithmetic-pulp.md`(pulp は内部依存として継続)、
  `docs/adr/al-math-api-fft-split-re-im.md`(公開面の
  バックエンド中立化 — vec 層のスライス公開境界の前段決定。
  `mag_sq` の split re/im 入出力形はこの決定に依存)
- 関連: `docs/adr/spectral-envelope-fft-size-assert.md`(入口
  assert / panic 契約の前例)
- 起点: `docs/specs/al-math-api-boundary.md` の Deferred Design
  Concerns(本集約を次 spec として予告、`defers` で連結)
