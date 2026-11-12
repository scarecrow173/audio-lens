---
id: "IMPL-c5supRRZiao8OL194tdwTo"
type: "impl"
status: "completed"
title: "Issue #102 解析意味論と汎用カーネルの責務分離リファクタリング記録"
created: "2026-10-31"
updated: "2026-10-31"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/102"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/ruviform-domain-kernel-separation.md"
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

# Issue #102 解析意味論と汎用カーネルの責務分離リファクタリング記録

- 対象 Issue: audio-lens #102
- 基準 baseline: Issue #98 成果物(`baseline/`)
- Ruviform rev: `f3f474b1734af05ca2435a322f935805010ba8b6`(#101 と同一・変更なし)
- 境界の正本: `docs/designs/ruviform-migration-responsibility-map.md`
  「境界判定記録 → AL#102 判定」節
- 本記録の差分出力: `docs/impl/ir/issue-102-baseline/`(`tools/baseline/compare.mjs` の 9 ラン分)

## Summary

#101 で汎用 f64 カーネルを `ruviform-numeric` へ移行済みの状態を前提に、
残存コードの「意味を持たない演算」と「AudioLens 固有の解析意味論」の境界を
確定し、演算呼出と意味論付与(FeatureRecord 変換)の層を分離する
リファクタリングを実施した。挙動不変(コード移動 + 同一 FP 演算順序の
kernel 寄せ)を条件とし、Issue #98 凍結 golden との 9 ラン比較で
#102 起因の新規差分は 0 件であった。

## Context

- 対象 Issue: audio-lens #102
- task: `docs/tasks/ruviform-domain-kernel-separation.md`(front matter `implements`)
- 前提: #101(汎用 f64 カーネルの Ruviform 切替)完了済み、同一 Ruviform
  rev `f3f474b1734af05ca2435a322f935805010ba8b6`
- 境界の正本: `docs/designs/ruviform-migration-responsibility-map.md`
  「境界判定記録 → AL#102 判定」節
- 基準 baseline: Issue #98 成果物(`baseline/` の凍結 golden)

## Implementation

### Scope and Judgment Rules

#101 で汎用 f64 カーネルは `ruviform-numeric` へ移行済み。#102 は残存コードの
「意味を持たない演算」と「AudioLens 固有の解析意味論」の境界を確定し、
演算呼出と意味論付与(FeatureRecord 変換)の層を分離する。判定規則は責務
マップの正本と同一: 別用途ツールがそのまま使える処理は Ruviform、
AudioLens の解析判断を含む処理は AudioLens。リファクタリングは挙動不変と
する(コード移動 + 同一 FP 演算順序の kernel 寄せのみ)。

### Changes

| 箇所 | 旧 | 新 | 等価根拠 |
| --- | --- | --- | --- |
| `al-analysis/src/reference.rs::fir_ir` | inline の `cxx`/`cxy` per-bin 蓄積 + `cxx>0` ガード付き H1 除算 | `ruviform_numeric::xspec::CrossSpectrum::accumulate` + `transfer_h1_into` | per-bin 加算・`conj(x)·y` 規約・`pxx>0.0` ゼロガードが Ruviform 実装と一致(Ruviform 側 doc が AL `fir_ir` 規則を明示)。`powi(2)` ≡ `x*x` で bit-exact |
| `al-analysis/src/expensive/channel.rs` L/R coherence 蓄積 | band_bins 限定の `sxy`/`sxx`/`syy` 手動蓄積 | `CrossSpectrum::new(bins)` による全 bin 蓄積 | 蓄積は per-bin 独立のため `band_bins` 参照値は不変。第三オクターブ帯選択・`|coh|` 平均・support 判定は AL 側に残置 |

### Separated Layers

- `al-analysis/src/cheap.rs::analyze_input`(約 1,150 行): 演算を 5 つの
  `measure_*` 関数へ抽出し、本体は FeatureRecord 変換(validity/confidence/
  support 写像・tensor 出力・channels/lineage 付与)のみを行う層構造へ。
  - `measure_level`: peak/true-peak/DC/RMS/trajectory/loudness/LRA
  - `measure_digital`: clip/zero/stuck/discontinuity/click・duplicate-block・quant
  - `measure_spectral`: STFT 派生テンソル・per-frame スペクトル統計・slope・
    narrowband・tonal・hum・AM
  - `measure_stereo`: corr0/sum_sq/ILD/IPD sweep/M-S・CrossSpectrum coherence
  - `measure_transient`: onset 検出・self-similarity・lag drift(`ipd_lags` 呼出)
- `al-locate`: `best_window_match` は `pool.ncc` 呼出・`argmax_tie_count`/
  `peaks_from_correlation` の意味論フィルタ・consensus 閾値判定が既に分離
  済みのため変更なし。`methods.rs` の `evidence_from_corr`/`preflight`/
  `band_*` も同様に演算呼出と証拠変換が分離済み。

### Remaining in AudioLens

| 処理 | 判定 | 根拠 |
| --- | --- | --- |
| `al-locate::dsp` facade | 暫定 adapter として維持 | Ruviform `Result` → 旧 `al-math` panic 契約の写像を一箇所に集約。撤去条件は AL#104(全呼出側が ruviform 直呼びへ移行後に panic 契約を再評価) |
| `dsp::deinterleave` / `decode.rs::PcmView::deinterleave` | 恒久残留 | AL の入力境界処理。`PcmView` 版は scale 除算 fused で別経路・別責務 |
| `expensive/channel.rs::csolve` | 残留 | 複素線形ソルバは `ruviform-numeric` 公開 API に存在しない(→ Ruviform 契約提案 R1) |
| `cheap.rs` IPD lag sweep・`local_lag`/tracking 系 `corr0` スキャン | 残留 | `OverlapCorrPlan` は FFT 分子経路で直算 `corr0` と FP 差分が出るため挙動不変条件を満たさない(→ R2) |
| スペクトル統計・OLS・argmax/同率・median/peak 選択・状態機械走査(cheap/reference/expensive/avsync/calibration) | 残留 | 「基礎スペクトル統計」としては汎用だが Ruviform 側に対応 API がなく、AL の観測仕様(端点除外・support・窓分割・閾値)と一体。`measure_*` 層として可視化済み(→ R3) |
| `reference.rs::analyze_pair` / `expensive` 各 family `run` の計算+emit ブロック | 現行構造維持 | 各ブロックの演算は帯域選択・validity gating・median/argmax の意味論集約であり AL 側と判定。`analyze_pair` 同型の measure 分離は AL#104 の監査対象として記録 |
| `residual.rs::rms` | 残留 | 複数チャンネル合成エネルギー(Σsum_sq/Σlen)は `dsp::rms` とは別意味論の AL ローカル helper |
| `normalize.rs`/`prepared.rs` の `ResamplePlan` 内部パラメータの永続化/cache 鍵 | 現行維持 | `normalized_pcm` 永続契約は AL 責務(#98 baseline 互換必須)。Ruviform 側に安定 ID/バージョン化表現があれば移行(→ R4) |
| Locate 合議・曖昧性・anchor/window・loop/skip/gap/drift・mixer/residual・candidate 採否 | 残留(変更なし) | AudioLens 固有意味論(責務マップ §3 全行) |
| Cheap/Reference/Expensive の適用制御・`FeatureRecord`/`Validity`/`Confidence`/Support/Provenance | 残留(変更なし) | AudioLens 固有意味論(責務マップ §4 全行) |

### Ruviform Type Audit

- `ruviform_numeric` 呼出の引数は `&[f64]`・sample rate・plan/workspace の
  みで、`InputRole`/`StageKind`/`CandidateEvent`/`Validity`/`Confidence`/
  `FeatureRecord`/診断語彙は一切渡っていない(全 `use ruviform_numeric`
  箇所を監査)。Ruviform 側型への AL 固有フィールド混入もなし。
- `ResamplePlan` の永続化(上表)は「AL 側の永続契約が Ruviform 型の内部
  表現へ依存している」逆方向の結合であり、AL 固有意味論を Ruviform 型へ
  混入させるものではない。

## Related Experiments

### Adopted

- なし — 境界判定は責務マップ正本(`docs/designs/
  ruviform-migration-responsibility-map.md`)で確定済みであり、方式選択の
  ための実験的比較は行わなかった。関連する `docs/impl/exp/*.jsonl` の
  実験記録は存在しない。

### Rejected

- なし

## Validation

### Golden Comparison (tools/baseline)

`baseline/runs/<name>`(#98 凍結)を `base98-<name>` へ退避し、9 ランを
#102 変更後コード(`cargo build --release`)で capture、`compare.mjs`
(rel 1e-9 / abs 1e-12)で比較後に凍結 golden を復元。差分の実体は
`issue-102-baseline/*-diff.json|txt`。

全 9 ランの全差分を次の 2 クラスに機械分類した結果、**説明不能な
新規差分は 0 件**(#102 起因の挙動差分なし):

1. **#101 既知クラス** — `issue-101-baseline/*-diff.json` と
   surface/path/kind が完全に一致する差分のみ(`aa` 45、`small` 1149、
   他 20–51 件)。SIMD 累積順序差による `tolerance`/`value` 差であり、
   #102 で再 capture しても同一差分が再現(deterministic)。
2. **#100 decoder provenance** — `artifacts/*/params/decoder/*` および
   `decoder/*` パスだけに現れる差分(各ラン 130–160 件)。
   `backend: "native"→"ruviform"`、`version: "0.1.0"→<git rev>`、
   `settings/ruviform` キー追加(`key` kind 各ラン 26–32 件)。
   ruviform media-I/O 移行(#100、6cc8f3e)による manifest/metadata
   provenance の正当な変更。#101 capture は当該キー導入前に行われた
   ため #101 差分セットには含まれない。

なお #101 IR は「added 差分 0」と記録するが、これは #101 capture が
`settings/ruviform` 導入前のバイナリで行われたことによる。#102 では
凍結 #98 golden を基準に再比較し、上記の通り全差分を #100/#101 へ
帰属させた。

### Verification Results

- `mise run fmt` / `check` / `lint`: pass
- `mise run test`: pass(`cargo test --workspace` 全スイート green)
- `mise run build` + `cargo build --release`: pass(baseline 用
  `target/release/al` を生成)
- `mise run verify-toolchain` / `check-cli-headless`: pass(9 headless
  crate に GUI 依存なし)
- `cargo test -p al-analysis -p al-locate`: 全件 pass(bit-exact 特性テスト
  `a7_freq_lr_coherence_is_bit_exact`・`a9_spec_slope_record_is_bit_exact`・
  `a7_stereo_coherence_mean_is_bit_exact`・golden 19+16 件を含む)
- 付随修正: `crates/al-media/tests/ruviform_deps.rs` の未使用 import
  `Fact` を除去(HEAD で `mise run lint` が既に失敗していたため)

## Risks

- 挙動不変条件の制約で AudioLens 残留が確定した処理がある:
  `cheap.rs` の IPD lag sweep・`local_lag`/tracking 系 `corr0` スキャンは
  `OverlapCorrPlan` の FFT 分子経路が直算 `corr0` と FP 差分を生むため
  移行できず(R2)、`expensive/channel.rs::csolve` は ruviform-numeric に
  対応 API がない(R1)。
- `analyze_pair`・`expensive` 各 family の measure/emit 分離は未実施で
  AL#104 の監査対象として残る。
- `ResamplePlan` の永続化・cache 鍵は Ruviform 型の内部表現へ依存する
  逆方向結合(#98 baseline 互換のため現行維持。R4 で安定表現を提案)。
- golden 差分は全て #100 decoder provenance・#101 SIMD 既知クラスへ帰属し
  説明不能な新規差分は 0 件だが、#101 IR の「added 差分 0」は capture
  時点の差(`settings/ruviform` 導入前)による記録であり、本記録では
  #98 凍結 golden 基準で再比較している。

## Follow-ups

- `analyze_pair`・`expensive` 各 family の measure/emit 分離(AL#104 監査対象)
- `dsp` facade 撤去(AL#104、全呼出側の直呼び移行後)
- R1–R4 の Ruviform 契約提案の Issue 起票(本 Issue では報告のみ)

### Ruviform Contract Proposals (Issue filing candidates)

本 Issue では変更しない — Issue 起票案:

- **R1**: 複素線形ソルバ(`expensive/channel.rs::csolve` 相当: 複素
  Gaussian 消去 + pivot + 条件数 proxy)。`linalg::LeastSquares` の複素版。
- **R2**: scalar 直算 overlap-NCC 曲線(`cheap.rs` IPD sweep・
  `local_lag`/band NCC の直算 `corr0` sweep 置換用)。FFT 分子経路では
  bit-exact を満たせないため、直算経路の API 化を提案。
- **R3**: 基礎スペクトル統計プリミティブ(centroid/flatness/rolloff/
  entropy/flux/bandwidth・OLS slope・median/argmax 選択)の `xspec`/
  `stats` API。#102 では `measure_*` 層に集約済みのため移行先が明確。
- **R4**: `ResamplePlan` の安定したパラメータ表現/ID(永続スキーマ・
  cache 鍵の forward-compat 用)。
