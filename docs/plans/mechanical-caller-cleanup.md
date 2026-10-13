---
id: "PLAN-034EO41zt4R9uACMi7eaFH"
type: "plan"
status: "completed"
title: "機械的 caller クリーンアップ: bandpass バッチ化・csd インライン化・pass 集約 (A6-A16)"
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
    - "docs/specs/mechanical-caller-cleanup.md"
  implemented-by:
    - "docs/tasks/mech-cleanup-bench-baseline-characterization.md"
    - "docs/tasks/mech-cleanup-a6-bandpass-batching.md"
    - "docs/tasks/mech-cleanup-a7-a8-csd-inlining.md"
    - "docs/tasks/mech-cleanup-a9-a12-cheap-pass-consolidation.md"
    - "docs/tasks/mech-cleanup-a15-interleave-block-writes.md"
    - "docs/tasks/mech-cleanup-a16-row-major-fast-paths.md"
    - "docs/tasks/mech-cleanup-measurement-ir-closeout.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/caller-side-optimizations.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/mechanical-caller-cleanup.md"
    - "docs/designs/mechanical-caller-cleanup.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/mechanical-caller-cleanup.md"
  references: []
  defers: []
  deferred-by: []
---

# 機械的 caller クリーンアップ: bandpass バッチ化・csd インライン化・pass 集約 (A6-A16) Implementation Plan

## Goal

approved design `docs/designs/mechanical-caller-cleanup.md`
(DESIGN-034ENh1Caa1pUN8HBonU2b) に基づき、spec
`docs/specs/mechanical-caller-cleanup.md`
(SPEC-034EMwMaXGBCfumSazX7zs) が束ねる機械的 caller 側
クリーンアップ 6 項目(A6 bandpass バッチ化・A7/A8 `csd`
インライン化・A9 `power()` 一括化・A12 HashSet 事前計数・
A15 interleave/deinterleave ブロック化・A16a–d 行優先累積 /
`dom()` scratch / video 融合 / tensor fast path)を、al-math
カーネル・公開 API・永続化フォーマットを一切変更せずに
適用する。全項目が **bit-identical** 等価クラス(A16d は
返却 `(out, region_shape)` の完全一致)であり、tolerance
predicate は一度も適用しない。spec のプロセス契約
(characterization テスト先行 + bench baseline/事後計測 +
Implementation Record 記録 + char コミットが opt コミットに
先行する git 履歴)の下で直列に land させる。

## Scope

単一の実装ストリームとして扱い、T1–T7 の7タスクに分割する。
分割は design のコミット順序(char → A6-opt → A7A8-opt →
A9A12-opt → A15-opt → A16-opt → IR 記録)に 1 対 1 で対応
させ、先行チェーン(`caller-side-optimizations`)と同じく全
サイトの characterization と bench 腕・baseline を T1 に
集約する。item ↔ commit は 1:1 ではなく、A9 の
`reference.rs:653-654` 側(psd→power + 行優先累積)は A16a
と同一変更単位のため T6 へ含める(design 確定事項)。
opt 同士を直列にするのは、各 opt の bench delta を単一の
進行する baseline に対して計測し、等価性ゲートの比較軸を
一意に保つため — 各 opt サイトが `reference.rs` / `cheap.rs`
を横断共有することも直列の理由。

- In scope: `crates/al-analysis` の criterion 新規 arm 4 本
  (`band_correlation`・`csd_accum`・`quant_step`・
  `spectral_pass`、`[[bench]]` stanza 同一パターン追加)と
  `benches/` 新設ファイル、`examples/bench_reference.rs` の
  stage 級経路再計測、al-locate `examples/bench_dsp.rs` の
  A15 Instant 区間(流用または追加)、al-dataset
  `tests/integration.rs` 内 Instant micro-timing、全項目の
  characterization テスト(配置は design の表に従う)、
  A6 `bandpass_many` 2 呼出し化 + `bandpass` import prune、
  A7/A8 `csd` インライン化 7 サイト + `csd` import prune、
  A9 cheap.rs `power()` 一括化 + `mean_row` 融合 +
  `half` 重複除去、A12 `HashSet<u64>` 事前計数、A15 の
  staged-row/indexed/channel-block 3 サイト、A16a 行優先
  スカラー累積(`reference.rs`・`expensive/mod.rs`)+
  `reference.rs:653-654` psd→power、A16b `dom()` `mag_sq`
  scratch、A16c video luma/diff 融合、A16d tensor
  `read_range` 行 fast path、baseline/事後実測の
  `docs/impl/ir/` Implementation Record 記録
- Out of scope(spec/design の Out of Scope・Deferred を
  継承): al-math カーネル変更(`Fft`/`RealFft`/vec
  primitives/filter 数学)、公開 API・永続化形式・record
  意味の変更、新規公開 API(`StftMatrix` メソッド含む)、
  al-analysis への直接 `al-math` 依存(dsp ファサード
  一元化を維持)、新規依存・al-dataset/al-media への新規
  dev-dep、`mean_psd` の vec reduction 化(tolerance
  class;別バンドル)、envelope prefix-sum、
  `matrix_to_tensor` 統合、A2/A5/A10/A13/A14、
  マルチスレッド化、golden 判定意味・Validity 閾値の変更、
  互換層・deprecation 期間(pre-release 方針により置換)

## Upstream Documents

- `docs/specs/mechanical-caller-cleanup.md`(approved,
  SPEC-034EMwMaXGBCfumSazX7zs): 受入基準 — 6 項目のスコープ・
  site 一覧・等価クラス(全 bit-identical)・禁止パターン・
  プロセス契約・計測方式規定
- `docs/adr/simd-reduction-lane-fma.md`(accepted,
  ADR-034EoHxzsvsHp9BOdCK2Cn): 数値不変の権威。本バンドルは
  全項目 bit-identical で tolerance predicate
  `|a-b| <= 1e-12 * max(1,|a|,|b|)` を適用しないが、A16b の
  `mag_sq` lane/tail 丸め依存の根拠となる
- `docs/specs/al-math-vec-primitives.md` +
  `docs/adr/al-math-vec-primitives-slice-boundary.md`:
  `mag_sq`/`scale` の elementwise 意味(A9/A16b の
  bit-identical 根拠)と dsp ファサード境界
- `docs/specs/observability-foundation.md` +
  `docs/adr/stage-timing-manifest-persistence.md`: manifest
  `duration_ms` の stage 級計時証拠
- `docs/discovery/mechanical-caller-cleanup.md`
  (DISC-034EMosQizFcChc9KKeaWS): 現 HEAD file:line で検証
  済みの site 列挙の根拠
- 先行チェーン: `docs/specs/caller-side-optimizations.md`・
  `docs/plans/caller-side-optimizations.md`・
  `docs/test-specs/caller-side-optimizations.md`
  (A1/A3/A4、test+bench-first 手続きと bench 基盤の継承元)

## Design Inputs

- `docs/designs/mechanical-caller-cleanup.md`(approved):
  本計画の正本。characterization 配置表・捕捉粒度決定
  (record/artifact 級 `to_bits`/バイト列完全一致、非永続
  中間量は下流 record で間接捕捉)、bench 構成(4 arm +
  Instant/example/fallback 規定)、コミット順序(7 step)、
  項目別実装 sketch(A6 `bandpass_many` 形状・A7/A8 の 7
  サイト表・A9 `power()` 導出 sketch・A12 HashSet
  sketch・A15 3 サイト sketch・A16a–d)、A15/A16d の
  deferred 決定(staged row + channel-block、行頭完全包含
  判定)、ファイルマップ、8 ゲートのテスト計画、リスク表

## File Map

| Path | Responsibility | Task |
|------|----------------|------|
| `crates/al-analysis/Cargo.toml` | `[[bench]] name = "band_correlation" / "csd_accum" / "quant_step" / "spectral_pass" harness = false` stanza ×4(criterion `=0.8.2` dev-dep は先行チェーンで導入済み) | T1 |
| `crates/al-analysis/benches/band_correlation.rs`(新設) | A6 腕: 8×`bandpass` loop vs `bandpass_many` の section 直接計測(既存公開 API のみで A/B 自己完結) | T1 |
| `crates/al-analysis/benches/csd_accum.rs`(新設) | A7/A8 腕: per-frame `csd` 累積と単要素 `csd` パターンの alloc 削減を固定長 synthetic 入力で計測 | T1 |
| `crates/al-analysis/benches/quant_step.rs`(新設) | A12 腕: >1M distinct 決定的入力で現行 double-sort vs HashSet gate を計測 | T1 |
| `crates/al-analysis/benches/spectral_pass.rs`(新設) | A9/A16a/A16b 腕: frames×bins の `mag_sq` pass 数・行優先累積・dom scratch を計測 | T1 |
| `crates/al-analysis/examples/bench_reference.rs` | A6・A9・A16a/b の stage 級代表経路(既存 harness、manifest `duration_ms` 併記) | T1, T2, T4, T6 |
| `crates/al-locate/examples/bench_dsp.rs` | A15 の Instant 計時区間(deinterleave/interleave が既存経路に載れば流用、載らなければ Instant 区間追加) | T1, T5 |
| `crates/al-analysis/tests/reference.rs` | A6(band_correlation tensor `to_bits` + `bands < 8` edge)・A7 下流 record・A8 hft・A15 residual interleave・A16a・A16b の characterization | T1 |
| `crates/al-analysis/tests/expensive.rs` + `src/expensive/mod.rs`/`channel.rs` tests | A7/A16a 下流 record(`mean_power` 系)の characterization | T1 |
| `crates/al-analysis/src/cheap.rs` `mod tests` | A9(mag/power/psd tensor + `spec.slope`)・A12(in-gate + >1M skip 両経路)・A7 stereo の characterization | T1 |
| `crates/al-locate/tests/locate.rs` + `src/dsp.rs` `mod tests` | A15 normalized `.f64le` バイト列・`deinterleave` 返却列の characterization | T1 |
| `crates/al-media/src/video.rs` `mod tests` | A16c luminance/diff 系列 `to_bits`・先頭 `diff=0.0`・`pixels==0` degenerate | T1 |
| `crates/al-dataset/src/tensor.rs` `mod tests` + `tests/integration.rs` | A16d `(out, region_shape)` 完全一致(fast path・fallback・innermost 部分包含 edge)+ Instant micro-timing | T1 |
| `crates/al-analysis/src/reference.rs` | A6 バッチ化(`:742-747`)+ `:13` import prune(T2)、A7×3 サイト + A8 hft + `:18` `csd` prune(T3)、A15 residual interleave(`:1163-1170`)(T5)、A16a(`:655-660`)+ A9 `:653-654` psd→power + A16b `dom()`(`:1071-1084`)(T6) | T2, T3, T5, T6 |
| `crates/al-analysis/src/cheap.rs` | A9 pass 集約(`:632` 付近、`:665` move 前)+ `mean_row` 融合(`:794-796`)+ `:668` `half` 重複除去 + A12 HashSet(`:577-603`)(T4)、A7 stereo `csd`(`:1105-1120`)+ `:16` `csd` prune(T3) | T3, T4 |
| `crates/al-analysis/src/band_gain.rs` | A7 coherence `xy`(`:159`)+ `:16` `csd` prune | T3 |
| `crates/al-analysis/src/expensive/channel.rs` | A7 `sxy`(`:102-118`、`band_bins` のみ積)+ `:9` `csd` prune | T3 |
| `crates/al-analysis/src/expensive/mod.rs` | A16a `mean_power` 行優先(`:648-651`) | T6 |
| `crates/al-locate/src/normalize.rs` | A15 staged-row interleave(`:168-171`) | T5 |
| `crates/al-locate/src/dsp.rs` | A15 channel-block deinterleave(`:17-29`) | T5 |
| `crates/al-media/src/video.rs` | A16c luma/diff 融合(`:402-441`) | T6 |
| `crates/al-dataset/src/tensor.rs` | A16d `read_range` contiguous-row fast path(`:271-289`) | T6 |
| `docs/impl/ir/` | baseline・各 opt 後の before/after 実測記録 | T1–T7 |

変更しない(design file map の「変更しない」欄を踏襲):
`stft.rs` の `csd`/`psd`/`magnitude`/`power`(公開 API、
内部不変)、al-math 全般(`filter.rs`・`vec.rs` 含む)、
`matrix_to_tensor`、永続化形式、record 意味、単一消費者
例外 `cheap.rs:942` `mm.magnitude()`・`reference.rs:1189`
`rm.psd()`。

## Implementation Sequence

| Task | 内容 | Depends on | Blocks |
|------|------|------------|--------|
| T1 | bench 腕 ×4 + stanza 追加 + 全項目 characterization テスト + al-dataset Instant micro-timing + bench_dsp/bench_reference 計測経路確認 + baseline 採取(`target/bench-baseline/`・`--save-baseline`)+ IR ドラフト。**最適化コードは含まず既存 API のみで compile** | — | T2 |
| T2 | A6-opt: `reference.rs:742-747` を `bands[..bands.len().min(8)]` slice + `bandpass_many` ×2 + `corr0` zip へ置換、`reference.rs:13` `bandpass` import prune(同一コミット)。`to_bits` ゲート + `band_correlation`/`bench_reference` 再計測 + IR 転記 | T1 | T3 |
| T3 | A7A8-opt: 列挙 7 サイトの `csd` 呼出しを inline `conj(x)·y` 積へ(加算順・auto-power 式は現行通り)、consumer file の `csd` import prune(`reference.rs:18`・`cheap.rs:16`・`band_gain.rs:16`・`expensive/channel.rs:9`、同一コミット)。下流 record `to_bits` ゲート + `csd_accum` 再計測 + IR 転記 | T2 | T4 |
| T4 | A9A12-opt: `cheap.rs` `power()` 一括化(`* inv` のみ・`:665` move 前・`half` 重複除去・`mean_row` 融合)+ `HashSet<u64>` 事前計数(gate 外では両 sort・f64 実体化を skip、support = `distinct.len()`)。mag/pow/psd/`spec.slope`/`dig.quant_step_est` ゲート + `quant_step`/`spectral_pass` 再計測 + IR 転記 | T3 | T5 |
| T5 | A15-opt: `normalize.rs:168-171` staged-row + `reference.rs:1163-1170` indexed write + `dsp.rs:17-29` channel-block gather(`chunks_exact`)。byte/element 完全一致ゲート + `bench_dsp` 再計測 + IR 転記 | T4 | T6 |
| T6 | A16-opt: `reference.rs:655-660`・`expensive/mod.rs:648-651` 行優先スカラー累積(`.take(frames)` 防御 bound 保持)+ `reference.rs:653-654` psd→power、`dom()` `mag_sq` scratch(手書き `re*re+im*im` 禁止)、video luma/diff 融合(`p.get(i)` zip 意味保持)、tensor `read_range` 行完全包含 fast path(部分包含は per-element gather fallback)。record/系列/`(out, region_shape)` ゲート + `spectral_pass`/Instant 再計測 + IR 転記 | T5 | T7 |
| T7 | 事後計測・closeout: 全 bench(criterion `--baseline` + Instant)再実行・±5% 回帰判定・IR 完成(計測方式・baseline 名・fixture 定義・before/after)・git 履歴の char→opt 順序確認・workspace 最終検証 | T6 | — |

コミット境界(design の 7 step に対応):

1. **char コミット**(T1): 全 characterization テスト +
   新規 criterion arm 4 本 + stanza(挙動不変、既存 API
   のみで compile)。このコミット後に baseline を採取し
   `target/bench-baseline/` + `target/criterion/`
   (`--save-baseline`)へ記録、IR ドラフトへ転記
2. **A6-opt**(T2)・3. **A7A8-opt**(T3)・
   4. **A9A12-opt**(T4)・5. **A15-opt**(T5)・
   6. **A16-opt**(T6、A9 の `reference.rs:653-654` 側を
   同コミットに含める — A16a と同一変更単位)
7. **IR 記録**: 各 `-opt` コミットまたは直後の docs
   コミットで `docs/impl/ir/` へ before/after を記録
   (各 opt タスクの Done When に組込み、T7 で全体を
   closeout する)

各 `-opt` は独立適用可能で、任意のコミット単位で revert
しても他項目の等価性に影響しない(design 確定)。

## Dependencies and Risks

依存: 新規依存なし(`HashSet` は std)。al-analysis の
criterion `=0.8.2` dev-dep は先行チェーンで導入済みのため
stanza 追加のみ。al-dataset/al-media へ新規 dev-dep を
追加しない(Instant micro-timing または code inspection +
回帰ゲートの fallback 規定)。al-analysis へ直接 `al-math`
依存を追加しない(`mag_sq`/`bandpass_many`/`scale` は
`al_locate::dsp` facade 経由のみ、dsp.rs:34-35 で re-export
済み)。

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | A6: `.min(8)` を忘れ `bands < 8` で panic | low | med | char テストに低 sample rate の `bands < 8` edge を含める。単一ブロックなので revert 容易 |
| 2 | A7/A8: inline 式の引数順取り違え(`conj(x)·y` の x/y)・auto-power 式の不用意な統一 | med | med | 下流 record `to_bits` ゲート。現行式を機械的に写すのみ。サイト単位で revert 可 |
| 3 | A9: `* inv` でなく `/ norm` を書く・`m` move 後に m を読む | med | med | `/ norm` リテラル不在・`:665` より前の導出完了のコード検査 + `to_bits` ゲート |
| 4 | A9+A16a: psd 行列非実体化で隠れた消費者を落とす | low | med | `psd_s`/`psd_c` 消費者は `mean_psd_*` のみと grep 確認済み(design)。コンパイルが検出 |
| 5 | A12: HashSet の順序非決定が値へ混入 | low | low | 値へ影響するのは `total_cmp` sort 後の集合のみ。in-gate・>1M skip の両経路 char |
| 6 | A15: staged/indexed write の境界ずれ | med | med | バイト列・要素列完全一致テスト。各サイト独立で revert 可 |
| 7 | A16b: 手書き `re*re+im*im` で lane/tail 丸めずれ | med | high | `mag_sq` 呼出し必須(設計に明記)+ `harmonic_freq_ratio` record 一致 + コード検査 |
| 8 | A16c: `None=>0.0`/`pixels==0` 分岐の欠落、`p[i]` index 化による failure mode 変化 | med | med | 系列 `to_bits` + degenerate fixture + `p.get(i)` 使用のコード検査 |
| 9 | A16d: fast path の境界判定誤り(部分包含を誤って行コピー) | med | med | 完全包含のみ fast path・部分包含は per-element gather fallback。両経路の完全一致テスト |
| 10 | bench baseline の手順ブレ(ノイズ誤判定) | med | low | 先行チェーン同様 `--save-baseline`/`--baseline` と複数回中央値。±5% 幅を超える回帰のみ問題視 |
| 11 | 全体: 測定対象外経路の回帰 | low | med | baseline 比較 ±5% ゲート + workspace bench 回帰ゲート |

### Rollback Strategy

- 対象: 全タスクがコード・文書の追加・置換で不可逆な
  データ変換はない。公開 API・永続化形式の変更もない
- Trigger: 各 opt の等価性ゲート失敗・golden 破壊・
  bench ±5% 超回帰で原因切り分けに時間を要する場合
- Procedure: 当該 opt コミットを `git revert` する。
  opt コミットはそれぞれ独立(A6/A7A8/A9A12/A15/A16 の
  各コミット境界)。T1 の char・bench 基盤は revert 後も
  残してよい(現行実装を捕捉するテストとして green を
  維持 — char は opt 前後両時点で pass する契約)
- Verification: revert 後に `mise exec -- cargo test
  --workspace` が green であること

## Task Breakdown

### T1: characterization・bench 腕・baseline — `docs/tasks/mech-cleanup-bench-baseline-characterization.md`

design の char コミット(step 1)を担う。`[[bench]]` stanza
×4 + `benches/` 4 ファイル(band_correlation・csd_accum・
quant_step・spectral_pass — いずれも既存公開 API のみで
A/B 腕を自己完結)、design 配置表の全 characterization
テスト(tests/reference.rs・tests/expensive.rs・cheap.rs
mod tests・expensive 系 tests・locate.rs・dsp.rs mod
tests・video.rs mod tests・tensor.rs mod tests +
tests/integration.rs)、al-dataset Instant micro-timing、
al-locate `bench_dsp.rs` 計測経路の確認・区間追加、
baseline 採取と IR ドラフト。最適化コードは含まない。

### T2: A6 bandpass_many バッチ化 — `docs/tasks/mech-cleanup-a6-bandpass-batching.md`

`reference.rs:742-747` を `&bands[..bands.len().min(8)]` +
`bandpass_many` 2 呼出し + `corr0` zip へ置換(16→2 plan/
forward)、`:13` `bandpass` import 同一コミット prune、
`band_correlation` tensor `to_bits` ゲート、`bands < 8`
edge、band_correlation arm + `bench_reference` 再計測。

### T3: A7/A8 csd インライン化 — `docs/tasks/mech-cleanup-a7-a8-csd-inlining.md`

列挙 7 サイト(`reference.rs:183`・`:539-545`・`:872-877`・
`band_gain.rs:159`・`cheap.rs:1105-1120`・
`expensive/channel.rs:102-118`・A8 `reference.rs:587-593`)
の `csd` 呼出しを inline `conj(x)·y` 積へ置換(加算順・
auto-power 式・`min(64)` cap・xs guard 保持)、consumer
file 4 箇所の `csd` import prune、下流 record `to_bits`
ゲート、`csd_accum` 再計測。`csd` 関数自体は public の
まま残す(`stft.rs:390` の `#[cfg(test)]` が使用)。

### T4: A9+A12 cheap 側 pass 集約 — `docs/tasks/mech-cleanup-a9-a12-cheap-pass-consolidation.md`

`cheap.rs:632` 付近で `power()` 一回から mag(`sqrt` +
`mean_row` 融合・k<half・÷ `mag.len()`)と psd(`* inv`
のみ、`/ norm` 禁止)を導出(`:665` move 前に完了)、
`:668` `half` 重複除去。`cheap.rs:577-603` を
`HashSet<u64>`(to_bits)事前計数 + in-gate のみ
`total_cmp` sort → min positive diff、support =
`distinct.len()`。mag/pow/psd/`spec.slope`/`dig.quant_step_est`
(両経路)ゲート + `quant_step`/`spectral_pass` 再計測。

### T5: A15 interleave/deinterleave ブロック化 — `docs/tasks/mech-cleanup-a15-interleave-block-writes.md`

`normalize.rs:168-171` staged row + frame 毎
`extend_from_slice`(`with_capacity`)、
`reference.rs:1163-1170` `n*c` 事前確保 + indexed write、
`dsp.rs:17-29` `chunks_exact` channel-block strided gather
(trailing partial frame drop 同一)。`.f64le` バイト列・
deinterleave 列・residual interleave 列の完全一致ゲート、
`bench_dsp` Instant 再計測。

### T6: A16 行優先・dom scratch・video 融合・tensor fast path — `docs/tasks/mech-cleanup-a16-row-major-fast-paths.md`

`reference.rs:655-660`(`.take(frames)` bound 保持)+
`expensive/mod.rs:648-651` 行優先スカラー累積、
`reference.rs:653-654` psd→power(A9 item の reference 側、
A16a と同一コミット)、`dom()` を `mag_sq` 再利用
scratch + `sqrt` 累積へ(手書き `re*re+im*im` 禁止・
`power()` 再利用禁止・facade 経由)、`video.rs:402-441`
luma/diff 1 pass 融合(`p.get(i)`、先頭 `0.0`・
`pixels==0` 保持)、`tensor.rs:271-289` 行頭完全包含
`copy_from_slice` fast path(部分包含 fallback)。各
record/系列/`(out, region_shape)` ゲート +
`spectral_pass`/Instant 再計測。

### T7: 事後計測・IR 記録・最終検証 — `docs/tasks/mech-cleanup-measurement-ir-closeout.md`

全 criterion bench の `--baseline` 比較、Instant 経路の
`target/bench-baseline/` 比較、±5% 回帰判定、項目ごとの
計測方式と before/after を `docs/impl/ir/` へ確定記録、
git 履歴の char→opt 順序確認、workspace 最終検証
(test・clippy・fmt・golden・`cargo tree` 依存検査)。

## Verification

| spec AC(要旨) | 検証方法 | 担当 |
|---|---|---|
| 各最適化で characterization テストが変更前に存在し green、opt 前後で green | `cargo test` + git 履歴(char コミットが opt コミットに先行) | T1 + 各 opt + T7 |
| A6: `band_corrs` 要素 `to_bits` 一致、`bandpass_many` 2 回・`.min(8)` bound・`bandpass(` 不在 | `cargo test -p al-analysis --test reference` + コード検査 | T1, T2 |
| A7/A8: 下流 record(h・coherence・band gain・`stereo.coherence_mean`)・hft tensor `to_bits`、列挙 7 サイトに `csd(` 不在 | `cargo test -p al-analysis` + `rg 'csd\('` コード検査 | T1, T3 |
| A9: mag/pow/psd tensor・`spec.slope` `to_bits`、`/ norm` 不在・`:665` 前導出・`mean_row` 融合・`StftMatrix` 新規メソッドなし | `cargo test -p al-analysis cheap` + コード検査 | T1, T4 |
| A12: `dig.quant_step_est` value/support/`Validity` が in-gate・>1M skip 両経路で一致 | `cargo test -p al-analysis cheap` | T1, T4 |
| A15: `.f64le` バイト列・deinterleave 列・residual interleave 列が完全一致 | `cargo test -p al-locate`・`cargo test -p al-analysis --test reference` | T1, T5 |
| A16a/b: `mean_psd_*`/`mean_power`/`total_power`/`one_sided_power`/`harmonic_freq_ratio` `to_bits`・scalar row-ascending・`mag_sq` scratch・手書き `re*re+im*im` 不在 | `cargo test -p al-analysis` + コード検査 | T1, T6 |
| A16c: luminance/diff 系列 `to_bits`・先頭 `diff=0.0`・`pixels==0` degenerate | `cargo test -p al-media` | T1, T6 |
| A16d: `(out, region_shape)` が fast path・fallback・部分包含 edge で完全一致 | `cargo test -p al-dataset` | T1, T6 |
| bench delta 記録・非対象経路 ±5% 以内 | `--save-baseline`/`--baseline` 比較 + Instant/example + IR 記録 | T1, T2–T6, T7 |
| workspace test・build・clippy・fmt・golden green | `mise exec -- cargo test --workspace` 等 | 各タスク + T7 |
| `cargo tree -p al-analysis --depth 1` に al-math 直接依存なし | `mise exec -- cargo tree -p al-analysis --depth 1` | T1–T7 全体 |
| `audit_docs`・IR `audit_impl_record` が clean | doc-status スクリプト | T7 |

## Open Questions / Gaps

- criterion baseline 名(`--save-baseline <name>`)は実施時に
  タスク内で統一し IR に記録する(先行チェーン同様)
- 各新規 bench ファイル内の A/B 腕の構成(入力サイズ・
  `sample_size`・`measurement_time`)は T1 実施時に既存
  `benches/stft.rs`/`conv.rs` の pattern に合わせて決め、
  fixture 定義を IR に記録する
- al-media(A16c)の計測方式は T6 時点で確定する —
  既存 example/test に合致する Instant 区間があればそれ、
  なければ code inspection + workspace bench 回帰ゲート +
  manifest `duration_ms`(spec fallback)。採用方式を IR に
  明記する
- al-locate `bench_dsp.rs` は deinterleave/interleave が既存
  経路に載るかを T1 で確認し、載らなければ Instant 区間
  追加に留める
- `docs/impl/ir/` の記録ファイル名・分割は T1 で IR
  ドラフトを起こす際に既存 IR 規約(先行チェーンの
  `caller-side-optimizations-*.md`)に倣う

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution(T1→T2→T3→T4→T5→T6→T7
  の直列依存。各 opt の bench delta は進行する baseline
  に対して計測し、`reference.rs`/`cheap.rs` を複数タスクが
  共有するため並列化しない)
- Independent work streams: なし(char テスト群は論理的に
  並行可能だが T1 の単一コミット境界に集約済み)
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- design の characterization 配置表・bench 構成・コミット
  順序・項目別 sketch・禁止パターンを正本として転記済み。
  レビュアは (a) T1 への char 集約と T2–T6 の task↔commit
  対応が spec プロセス契約を損なわないこと、(b) A9 の
  `reference.rs:653-654` 側が T6 に配置される item↔commit
  非 1:1 の扱いが design 通りであること、(c) Verification
  matrix が spec Acceptance Criteria を網羅していることを
  確認すること
- 計測方式の未決部分(al-media fallback・bench 腕の具体的
  入力構成・baseline 名・IR 分割)は Open Questions に明記
  し、決定結果を IR に記録する方針
