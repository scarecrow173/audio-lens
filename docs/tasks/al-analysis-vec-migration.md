---
id: "TASK-034EpPeS7I91EbO1Kzjqwr"
type: "task"
status: "todo"
title: "al-analysis: dsp ファサード経由の vec プリミティブ移行"
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
    - "docs/plans/al-math-vec-primitives.md"
  implemented-by: []
  depends-on:
    - "docs/plans/al-math-vec-primitives.md"
    - "docs/tasks/al-locate-vec-migration.md"
  blocks:
    - "docs/tasks/vec-primitives-verification-bench.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/al-math-vec-primitives.md"
    - "docs/adr/al-math-vec-primitives-slice-boundary.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/al-math-vec-primitives.md"
  references: []
  defers: []
  deferred-by: []
---

# al-analysis: dsp ファサード経由の vec プリミティブ移行

## Work

al-analysis 内の重複実装を `al_locate::dsp` ファサード経由の
プリミティブ呼出しに置き換える(plan T3)。al-math 依存は
追加しない — `crates/al-analysis/Cargo.toml` は変更しない。

- `crates/al-analysis/src/cheap.rs`:
  - インライン `rms`(:180,:254,:358,:414)→ `dsp::rms`
  - mean-square(:385、Σv²/n・sqrt なし)→ `dsp::sum_sq(x)/n`
    (`rms` ではない — sqrt 無しの式のため)
  - `sum_sq`(:912,:921)→ `dsp::sum_sq`
  - `stereo.pair_correlation`(:1017-1023)のインライン
    3 アキュムレータ → `corr = dsp::corr0(xs, ys)` +
    `sx = dsp::sum_sq(xs)`、`sy = dsp::sum_sq(ys)`。sx/sy は
    :1030 の Validity 判定(ゼロエネルギー →
    `SilenceInsufficientEnergy`)と :1042 `stereo.ild_db` で
    消費するため **必ず保持** する。融合 1 パス → 3 パスの
    コスト増は spec/design で受容済み
  - `stereo.ipd_samples` lag 走査(:1067-1071)・
    `trans.self_similarity`(:1250-1252)のインライン corr →
    `dsp::corr0`
- `crates/al-analysis/src/reference.rs`:
  - `rms`(:119)→ `dsp::rms`、`corr0`(:134)→ `dsp::corr0`
  - `ls_gain`(:127-131)→ ゼロ分母ガードを厳密に保持:
    `let den = dsp::sum_sq(x); if den > 0.0 { dsp::dot(x,y)/den }
    else { 0.0 }`(素直な `dot/sum_sq` だと分母 0 で NaN になる)
  - マルチch `energy` クロージャ(:1066)→ per-channel
    `dsp::sum_sq` 合成、`res_rms`(:1164)→ `dsp::rms`
- `crates/al-analysis/src/expensive/mod.rs`: `pub(crate)`
  `rms`(:257)・`corr0`(:264)を削除し、消費側は
  `al_locate::dsp::{corr0, rms}` を import
- `crates/al-analysis/src/expensive/timing.rs`: `:11` の
  `use super::{corr0, rms}` → `use al_locate::dsp::{corr0, rms}`。
  呼出し `corr0`(:92,:148,:397)・`rms`(:172,:216,:276)は
  名前のみ追従
- `crates/al-analysis/src/expensive/dynamics.rs`: `:11` の
  import 同様に `dsp::rms` 経由へ。呼出し(:86,:149,:350,:351)
  は名前のみ追従
- `crates/al-analysis/src/expensive/ir.rs`: split 済み re/im 上の
  `mag2` 合成(:178)→ `dsp::mag_sq(&re, &im, &mut mag2)`。
  `:155-161` のタプルスライス上合成は残存許容で触らない
- `crates/al-analysis/src/band_gain.rs`: `:94,:101` の Σv² →
  `dsp::sum_sq`、`:96` の window dot → `dsp::dot`
- `crates/al-analysis/src/stft.rs`:
  - `power()`(:78-84)→ フレーム毎
    `dsp::mag_sq(re, im, &mut out)`
  - `psd()`(:47-61)→ `dsp::mag_sq` で power バッファを作り
    `dsp::scale(&power, 1.0/norm, &mut out)` の 2 パス合成
    (余分な要素 sweep は受容済み)
  - `magnitude()`(:64-76)→ `dsp::mag_sq` + 要素毎
    `f64::sqrt`(sqrt プリミティブは提供しない)
- プリミティブは入出力スライスのエイリアシングを許容しない。
  論理 in-place(例 `scale(x, a, x)`)が必要な箇所は `to_vec`
  等で別バッファを使う(design エイリアス規約)

### 残存許容(触らない)

`expensive/ir.rs` タプルスライス合成(:155-161)、`cheap.rs`
mid/side 電力(:1096-1101)・magnitude 総和(:1148)・
4 アキュムレータ回帰 fold(:811)・`sqrt(mean(mom))`(:390)、
`dynamics.rs` 符号別エネルギー split(:57-61)・要素毎
magnitude(:199)・回帰アキュムレータ(:373-389)、
`reference.rs` 単要素/バンド合成(:535,:839)・M/S エネルギー
(:872-881)・`Vec<(f64,f64)>` magnitude 合成(:550,:621)・
off-diagonal `sum_sq`(:908)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/al-math-vec-primitives.md`
- `docs/designs/al-math-vec-primitives.md`(al-analysis 移行表
  L262-291・エイリアス規約・ガード付き合成)
- `docs/adr/al-math-vec-primitives-slice-boundary.md`(dsp
  ファサード消費経路の権威)

## Dependencies

- `docs/tasks/al-locate-vec-migration.md`(T2): `al_locate::dsp`
  の vec re-export が前提

## Affected Files

- `crates/al-analysis/src/cheap.rs`
- `crates/al-analysis/src/reference.rs`
- `crates/al-analysis/src/expensive/mod.rs`・`expensive/timing.rs`・
  `expensive/dynamics.rs`・`expensive/ir.rs`
- `crates/al-analysis/src/band_gain.rs`
- `crates/al-analysis/src/stft.rs`
- `crates/al-analysis/Cargo.toml` は変更しない(al-math 依存
  を追加しない)

## Done When

- [ ] 列挙した al-analysis サイトが `al_locate::dsp` 経由の
  プリミティブ呼出しに置換され、残存許容リスト以外に同一式の
  独立実装が残らない
- [ ] `ls_gain` のゼロ分母ガードと `pair_correlation` の
  sx/sy(Validity・ild_db 消費)が保持されている
- [ ] `crates/al-analysis/Cargo.toml` に al-math 依存が追加
  されていない
- [ ] golden・Validity ゲートを含む既存テストが非回帰であり、
  許容比較へ修正した float 厳密比較テストがあれば一覧を IR に
  記録する

## Verification

- [ ] `mise exec -- cargo test -p al-analysis` が green
- [ ] `mise exec -- cargo tree -p al-analysis --depth 1` の直接
  依存に al-math が現れない(または `crates/al-analysis/
  Cargo.toml` の直接確認)
- [ ] `mise exec -- cargo build --workspace` が green
- [ ] `mise exec -- cargo clippy`・`cargo fmt --check` が green
