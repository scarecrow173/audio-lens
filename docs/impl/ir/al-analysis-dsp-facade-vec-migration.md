---
id: "IMPL-034EqFnpaWTKLnca2cw3Z3"
type: "impl"
status: "completed"
title: "al-analysis dsp facade vec migration"
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
    - "docs/tasks/al-analysis-vec-migration.md"
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
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# al-analysis dsp facade vec migration

## Summary

al-analysis 内の重複した `corr0`/`rms`/`sum_sq`/`dot`/`mag` 実装を、
`al_locate::dsp` ファサード経由の `al_math::vec` プリミティブ
呼出しに置換した(plan T3)。`crates/al-analysis/Cargo.toml` は
変更せず、al-math 直接依存は追加していない
(`cargo tree -p al-analysis --depth 1` の直接依存に al-math なし)。
残存許容リストの箇所(タプルスライス合成・回帰アキュムレータ・
M/S エネルギー等)は一切触れていない。

## Context

task `docs/tasks/al-analysis-vec-migration.md`
(TASK-034EpPeS7I91EbO1Kzjqwr)、design
`docs/designs/al-math-vec-primitives.md`(al-analysis 移行マップ
L262-291・エイリアス規約・ガード付き合成)、ADR
`docs/adr/al-math-vec-primitives-slice-boundary.md`(dsp ファサード
消費経路の権威)に従う plan T3。T2
(IMPL-034EqrFgTWTPu16W3CuG6r)の `al_locate::dsp` re-export を
前提とする。

## Implementation

### Changes

- `crates/al-analysis/src/cheap.rs`
  (`use al_locate::dsp::{corr0, ncc, rms, sum_sq}` に拡張):
  - インライン rms 4 箇所(旧 :180,:254,:358,:414)→ `rms`
  - mean-square(旧 :385、Σv²/n・sqrt なし)→
    `sum_sq(f) / f.len() as f64`(`rms` 非適用 — 平方根を取らない式)
  - `sum_sq` 2 箇所(旧 :912,:921)→ `sum_sq`
  - `stereo.pair_correlation`(旧 :1017-1023)→
    `corr0(xs, ys)` + 個別 `sum_sq(xs)`/`sum_sq(ys)`。
    sx/sy は Validity ゲート(ゼロエネルギー→
    `SilenceInsufficientEnergy`)と `stereo.ild_db` で
    消費するため値を保持(融合 1 パス → 3 パスは受容済み)
  - `stereo.ipd_samples` lag 走査(旧 :1067-1071)・
    `trans.self_similarity`(旧 :1250-1252)のインライン corr →
    `corr0`(ガード `if xa.is_empty() { continue; }` は保持)
- `crates/al-analysis/src/reference.rs`
  (import に `corr0, dot, rms, sum_sq` を追加):
  - ローカル `fn rms`・`fn corr0` を削除し全呼出しを dsp 経由へ
  - `ls_gain` → ゼロ分母ガードを字面どおり保持:
    `let den = sum_sq(x); if den > 0.0 { dot(x, y) / den } else { 0.0 }`
    (素直な `dot/sum_sq` では `sum_sq==0` で NaN になるため)
  - マルチch `energy` クロージャ → per-channel `sum_sq` 合成
  - `res_rms` → `rms(&interleaved)`(interleaved の全要素和
    ÷ `n*channels` と同一集合のため等価)
- `crates/al-analysis/src/expensive/mod.rs`: `pub(crate)` の
  `rms`(:257)・`corr0`(:264)ヘルパを削除
- `crates/al-analysis/src/expensive/timing.rs`・`dynamics.rs`:
  `super::{..., corr0, rms}` import を
  `use al_locate::dsp::{corr0, rms}` / `use al_locate::dsp::rms`
  に変更(`super::{Emit, ExpensiveResult, RegionCtx, ...}` 部分は
  保持)。呼出し 7+4 箇所は名前置換のみ
- `crates/al-analysis/src/expensive/ir.rs`: split 済み re/im 上の
  `mag2` 合成(:178)→ `al_locate::dsp::mag_sq(&re, &im, &mut mag2)`
  (:155-161 のタプルスライス合成は残存許容で未変更)
- `crates/al-analysis/src/expensive/codec.rs`: :363 のブロック
  エネルギー `Σv²` → `al_locate::dsp::sum_sq`
  (file map 列挙外だが `&[f64]` 上の同一式で残存許容にも
  該当しないため、spec AC「残存許容以外に同一式の独立実装を
  残さない」に従い置換)
- `crates/al-analysis/src/band_gain.rs`
  (`dsp::{dot, ncc, sum_sq}` に拡張): :94,:101 の `Σv²` →
  `sum_sq`、:96 の window dot → `dot`
  (`energy > 1e-20`・`<= 1e-20` ガードは保持)
- `crates/al-analysis/src/stft.rs`
  (`use al_locate::dsp::{fft, mag_sq, scale}` に拡張):
  - `power()` → フレーム毎 `mag_sq(re, im, &mut out)`
  - `psd()` → `mag_sq` で power バッファを作り
    `scale(&power, 1.0/norm, &mut out)` の 2 パス合成
    (エイリアス不可規約により別バッファ)
  - `magnitude()` → `mag_sq` + 要素毎 `f64::sqrt`
    (sqrt プリミティブは提供しない)

### Decisions During Implementation

- **`res_rms` の再表現**: design は「`res_rms` → `dsp::rms`」と
  だけ指示する。直上で構築済みの `interleaved: Vec<f64>` が
  `n * residual.len()` 要素を持つため `rms(&interleaved)` が
  最小の再表現。旧式 `(energy(residual) / (n*len).max(1)).sqrt()`
  と同一被加算集合・同一分母で、空入力時も両者 0.0 で等価
- **mod.rs での再 export を避けた**: `pub(crate) use
  al_locate::dsp::{corr0, rms}` を mod.rs に置けば
  `use super::{corr0, rms}` を維持できたが、ADR の dsp ファサード
  消費経路と task の指示に従い、消費側(timing/dynamics)が
  `al_locate::dsp` を直接 import する形にした
- **`cheap.rs` のローカル変数 `rms` との衝突**: :1190 の
  `let rms = &channel_rms[0]` は `rms` プリミティブ import を
  シャドウするが、当該スコープ内で `rms()` 呼出しはなく
  (self_similarity は `corr0` のみ)、合法かつ実害なしのため
  変数名は変更しなかった
- **等長契約の確認**: プリミティブは非等長で panic(旧 zip 黙認
  を継承しない)。全 corr0/dot 呼出しサイトを検査し、
  `Aligned.src`/`cap` の共有長制約・`[..n]` 同長スライス・
  `chunks_exact` ブロック対により構成的に等長であることを確認。
  panic 経路に落ちる呼出しは存在しない

## Related Experiments

### Adopted

<!-- No experiment logs were created for this task. -->

### Rejected

<!-- No experiment logs were created for this task. -->

## Validation

- `mise exec -- cargo test -p al-analysis`: 117 tests pass
  (unit 81・avsync 7・expensive 6・golden 19・reference 4、
  golden・Validity ゲート非回帰)
- `mise exec -- cargo test -p al-locate`: 66 tests pass(非回帰)
- `mise exec -- cargo build --workspace`: green
- `mise exec -- cargo fmt --all -- --check`: green
- `mise exec -- cargo clippy -p al-analysis --all-targets --
  -D warnings`: green
- `cargo tree -p al-analysis --depth 1` の直接依存に al-math
  なし(`Cargo.toml` 未変更と併せて確認)
- 残存同一式 sweep(`v*v`/`r*r`/`zip…sum` 走査): 残存は
  全て spec 残存許容箇所 — `ir.rs` タプル合成(:157,:161)、
  `dynamics.rs` 符号別 split(:60,:62)・要素毎 magnitude(:200)、
  `reference.rs` 単要素/バンド合成(:534,:605,:823)・
  off-diagonal(:892)、`cheap.rs` mid/side(:1080,:1085)・
  magnitude 総和(:1132)・回帰 fold(:808)・`sqrt(mean(mom))`
  (:387)、`band_gain.rs` タプル帯域合成(:178)、
  `ir.rs`/`reference.rs`/`dynamics.rs` 回帰アキュムレータ、
  `methods.rs` envelope NCC(T2 残存許容逸脱)、
  `residual.rs` 列 gather dot・Frobenius、`dsp.rs` テストコード

### 許容比較へ修正した float テスト一覧

なし — 既存テストの厳密/許容比較は一切変更していない
(golden 19 件を含む全テストが非回帰)。

## Risks

- `pair_correlation`・`energy` クロージャ等の融合 1 パス →
  複数パス再合成は結合順差により永続化値が ~ulp 級に揺れ得る
  (再合成許容ポリシーで許容済み。今回の既存テストは全て
  差分を検出しなかった)
- `psd()` は `mag_sq` + `scale` の 2 パス化により 1 回の余分な
  要素走査と中間バッファを持つ(エイリアス規約上不可欠、
  受容済み)

## Follow-ups

- `docs/tasks/vec-primitives-verification-bench.md`(T4):
  residual 減算ループの `sub_scaled` 再構成採否を計測
  プロトコルで判定し、vec プリミティブの criterion bench を
  追加する
