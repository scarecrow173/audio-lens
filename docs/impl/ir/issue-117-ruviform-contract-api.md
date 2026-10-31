# Issue #117 + Ruviform #167–#170: 契約 API 採用置換と rev/precision stamp

## Summary

Ruviform 側で実装済みの #102 契約提案 R1–R4(ruviform #167–#170)へ
AL 側の残留二重実装・暫定実装を置き換え、あわせて #117「解析 stage
artifact への ruviform rev/precision stamp」を実装した。数値カーネルは
ruviform 側の同一評価順実装へ移し AL 側は stage 意味論
(validity/support/confidence・端点除外・FeatureRecord 変換)のみを
保持する。pin は ruviform main `080c228`、bit-exact は golden 回帰で
説明不能差分 0 件、stamp/ResamplePlan ID 形式変更は新規期待クラス
として機械帰属した。

## Context

- ruviform #167 `linalg::complex_solve`(AL `csolve` 同等の複素
  Gaussian 消去。部分 pivot 選択・消去順序・特異判定・
  `piv_max/piv_min` 条件数 proxy を同一評価順で提供)
- ruviform #168 `corr` 直算 NCC API(`overlap_ncc_direct` 等。
  `corr0` と同一の逐次 `sxy`/`ex`/`ey` 蓄積・`ex>0 && ey>0` guard で
  bit-exact)
- ruviform #169 `spec`(`centroid`/`bandwidth`/`rolloff`/`flatness`/
  `entropy`/`flux`/`ols_slope`/`median`/`argmax`。AL measure_* の
  汎用部分を逐語移植し oracle test で bit-exact 検証済み)
- ruviform #170 `ResampleParams` 正準表現 + `stable_id()`/
  `ALGORITHM_ID`/`CONTRACT_VERSION`
- audio-lens #117: 解析 stage の FeatureRecord.params・artifact
  params へ ruviform rev・precision・主要パラメータを記録する。
  rev 導出は #104 で導入した Cargo.lock 解析経路
  (`al_core::RUVIFORM_REV`)の単一経路
- 責務境界の正本: `docs/designs/ruviform-migration-responsibility-map.md`、
  `docs/impl/ir/issue-102-domain-separation.md`(#102 で残留と判断した
  4 項目が今回の置換対象)

## Implementation

### Changes

**1. ruviform pin 更新**(`47b0b11`)
- `Cargo.toml` の全 ruviform git 依存 `rev` を
  `080c228d197ebc645b8f81cd0082be442abaf10c`(origin/main HEAD)へ
  更新し `Cargo.lock` 再生成。

**2. csolve 置換**(`6eedad9`)
- `crates/al-analysis/src/expensive/channel.rs` の局所 `csolve` を
  `ruviform_numeric::linalg::complex_solve` へ置換。旧実装は
  `#[cfg(test)]` oracle として残し、サイズ 1/2/3/5 の決定的行列で
  係数・条件数を `to_bits()` 比較する bit-exact テスト
  (`complex_solve_matches_csolve_bit_exact`)と、形状違反・特異行列
  の同一失敗判定テストを追加。`max_cond` 伝播・`continue` による
  失敗スキップの AL 側意味論は不変更。

**3. 直算 corr0 sweep 置換**(`1faf8d4`)
- `cheap.rs` stereo IPD sweep(`-max_lag..=max_lag` の `corr0` 直算)
  を `overlap_ncc_direct` へ。ruviform API は `lag>=0` で第2引数を
  シフトする契約のため、AL の「第1引数をシフト」する正 lag 意味論を
  保つよう引数を `(ys, xs)` 順で呼び、負 lag は既存どおり
  `corr0` のまま厳密 `c > best_v` 採択を維持。
- `cheap.rs` transient self-similarity(lag 2–8)と `expensive/
  timing.rs` の local time-map sweep、`reference.rs` の F0
  自己相関 sweep も同 API へ。いずれも `corr0` と同一の逐次
  蓄積 + `ex>0 && ey>0` guard で bit-exact。
- 単発 `corr0` 呼出(sweep でない箇所)は `ruviform_numeric::vec::
  corr0` 直呼びへ整理済み(意味論同一)。

**置き換えなかった sweep(根拠付き)**
- `cheap.rs` `ipd_lags`/`ipd_block_lag`・`reference.rs` `local_lag` は
  既に `CorrelationPlan`/`OverlapCorrPlan` 経由。plan の NCC
  正規化は prefix-sum window energy であり逐次 `corr0` と bit
  一致しないため、直接 API への置換は差分を生む — 現行のまま維持。
- `al-locate/src/methods.rs` の spectral-envelope scan は lag ごとに
  envelope を再計算する意味論的 scan であり生 NCC sweep ではない。
- `al-locate` full/band NCC は `CorrelationPool` 経由で band 電力
  フィルタ・evidence 意味論を持つため AL 側で維持。

**4. スペクトル統計置換**(`1faf8d4`)
- `cheap.rs` measure_spectral 系の centroid/bandwidth/rolloff/
  flatness/entropy/flux/OLS log-log slope/median/argmax を
  `ruviform_numeric::spec::*` へ。`total>0`/`total<=0` の非対称
  guard・NaN→0.0/伝播・`ln(n)` 正規化・`total_cmp` argmax 最終
  index 採択など AL 非対称セマンティクスは ruviform 側が同一
  評価順で移植済みで、AL 固有の STFT/band 構成・support/
  validity・FeatureRecord 排出は AL に残留。
- onset 局所 median、`ipd_block_lag` argmax、expensive 各所の
  等価 median/argmax も `spec::*` へ。意味論が異なる箇所
  (例: hum ratio・AM depth のドメイン特化計算)は置換対象外。

**5. ResamplePlan 永続化置換**(`55c03d8`)
- `al-locate/src/normalize.rs` の `transform(plan)` を
  `ALGORITHM_ID`(identity/`polyphase-blackman-sinc`)+
  `algorithm_version: "1"` + `stable_id` の正準表現へ。旧の
  内部係数フィールド(up/down/radius/cutoff/…)は永続契約から
  撤去し、`NormalizedParams::validate()` は source/target rate
  から plan を再構成して stable_id 一致を検証。改ざん検知テストは
  `stable_id` 改変へ更新。`prepared.rs` cache 鍵は内部係数表に
  依存していなかったため変更なし。

**6. #117 stamp**(`880a963`)
- `records.rs` に `ruviform_stamp()` / `with_ruviform_stamp()` /
  `ruviform_params()` / `stamp_feature_records()` を追加。stamp 形:
  `{"rev": al_core::RUVIFORM_REV, "precision": "f64",
  "crates": ["ruviform-audio-analysis","ruviform-dsp",
  "ruviform-numeric"]}`。precision は ruviform の f64 scalar
  評価契約、crates は stage が消費する依存列挙(主要パラメータ)。
- FeatureRecord は measure 層が `params` を上書きするため、
  `stage.rs` の各 `write_json` 直前に `stamp_feature_records()` で
  一括 stamp(cheap fallback/通常・reference・expensive region・
  cross_capture comparison)。video ROI・input.decode fallback・
  avsync 由来 record も網羅される。
- artifact params(`ArtifactRecord.params`)と tensor/table
  provenance params も同一 helper で stamp。cross_capture
  provenance は stamped な `comparison.records[0].params` を
  そのまま引き継ぐ。
- records.rs に stamp 形状・既存 params 保持・rev 一致の
  unit test 追加。

**7. golden 分類器 + 証跡**(`e8e22cb`相当)
- `tools/baseline/lib/classify-diffs-117.mjs`: #104 分類機を基盤に
  #117 stamp(`params/ruviform` key 追加)・#170 stable_id
  (`transform` セグメント配下のキー再構成)の2クラスと、新
  capture 側の stamp/stable_id 存在・rev 一致の直接検証を追加。
- 証跡: `docs/impl/ir/issue-117-baseline/`。

### Decisions During Implementation

- **`overlap_ncc_direct` の引数順**: ruviform 契約は
  `corr0(x[..len-lag], y[lag..])`(第2引数シフト)。AL IPD loop は
  第1引数をシフトするため `(y, x)` 順呼出で同一計算を再現する。
  呼出ごとの対応は実装時に `corr0_parts_seq` ソースと突合済み。
- **plan 系 NCC は非置換**: `CorrelationPlan::ncc` は direct モード
  でも window energy を prefix-sum で正規化し逐次 `corr0` と
  bit 一致しない。「直算 sweep の置換」は真の `corr0` 逐次ループに
  限定した。
- **stamp は書き出し直前の一括適用**: `Emit::push` 等が `params`
  を全体上書きするため RecordBuilder 時点の stamp は失われる。
  artifact 永続化境界での一括適用が authoritative な単一点。
- **precision = `"f64"`**: ruviform-numeric は f64 専用・scalar
  逐次・非 FMA が評価順契約であり、実行時モード切替は存在しない。
  追加の "eval" 語彙は YAGNI と判断。

## Related Experiments

### Adopted

- 汎用カーネル 4 群の ruviform API 直採用(complex_solve・
  overlap_ncc_direct・spec::*・stable_id)。
- `al_core::RUVIFORM_REV` 単一経路による stamp(#104 の
  Cargo.lock 導出を再利用・複製なし)。

### Rejected

- `CorrelationPlan`/`OverlapCorrPlan` 由来 sweep の直接 API 化
  (prefix-sum 正規化で bit 不一致になる)。
- `methods.rs` spectral-envelope scan の NCC API 化(lag ごとの
  envelope 再計算という意味論差異)。
- FeatureRecord 構築時点での stamp 注入(measure 層の params
  上書きで失われるため)。

## Validation

### Golden 回帰(#98 凍結 golden、必須ゲート)

`git fetch origin main && git merge origin/main` は already up to
date。`mise exec -- cargo build --release -p al-cli` 後、
`baseline/runs/<run>` を `<run>-golden98` へ退避し
`node tools/baseline/capture.mjs`(全 run 実行+収集)、
`compare.mjs <run> <run>-golden98` で差分収集、
`classify-diffs-117.mjs` で機械分類した。

| run | total | #101 | #104 | #100 | #106 | #117 | #170 | unexplained |
|-----|-------|------|------|------|------|------|------|-------------|
| aa            | 2437 |   45 | 12 | 18 | 1372 |  900 |  90 | **0** |
| canonical     | 2437 |   45 | 12 | 18 | 1372 |  900 |  90 | **0** |
| resume        | 2437 |   45 | 12 | 18 | 1372 |  900 |  90 | **0** |
| small         | 3124 | 1149 | 16 | 24 |  644 | 1081 | 210 | **0** |
| variant-desync| 1053 |   20 | 52 | 78 |  140 |  688 |  75 | **0** |
| variant-clipping|1299|   42 | 52 | 78 |  170 |  867 |  90 | **0** |
| variant-reverb| 1291 |   31 | 52 | 78 |  170 |  870 |  90 | **0** |
| variant-filter| 1318 |   37 | 52 | 78 |  170 |  891 |  90 | **0** |
| variant-clicks| 1338 |   51 | 52 | 78 |  170 |  897 |  90 | **0** |

- #117/#170/#104/#100/#106/#101 の全差分が機械分類に帰属し
  **unexplained 0 件**。numeric 置換(2–4)由来の新規数値差分は
  0 — 全 tolerance 差分は #101 で既知だった同一 surface/path/kind。
- `aa` vs `canonical`、`resume` vs `canonical` は **0 diff**
  (決定性確認)。
- plan 検査(stage/artifact 集合差が viz 撤去のみ)・decoder rev
  検査・stamp 存在検査・stable_id 形式検査は全 run ok。
- 証跡: `docs/impl/ir/issue-117-baseline/<run>-vs-golden98.json`、
  `aa-vs-canonical.json`、`resume-vs-canonical.json`。

### 全検証(最終ゲート)

| gate | 結果 |
|------|------|
| `mise run fmt`(cargo fmt --all -- --check) | pass |
| `mise run check`(cargo check --workspace --all-targets) | pass |
| `mise run lint`(cargo clippy --workspace --all-targets -D warnings) | pass |
| `mise run test`(cargo test --workspace) | pass(46 suite 全 ok・0 failed) |
| `mise run build`(cargo build --workspace) | pass |
| `mise run verify-toolchain` | pass(rustc/cargo 1.99.0 + rustfmt/clippy) |
| `mise run check-cli-headless` | pass(10 headless crate GUI 依存なし) |

- 単体テスト追加: `channel.rs::complex_solve_matches_csolve_bit_exact`
  (bit-exact oracle・shape/特異失敗一致)、`records.rs` stamp 形状・
  既存 params 保持・rev 一致、`cheap.rs` IPD curve / self_sim の
  `corr0` oracle 一致(`to_bits` 比較)、normalize stable_id 改ざん検知。

## Risks

- `stamp` は全 FeatureRecord.params に一律で付くため、params を
  構造的に読む下流 consumer は新キー `ruviform` を受け取る。
  parse_feature_records は透過的で現状 consumer は golden/dataset
  読込のみであり問題観測なし。
- normalized_pcm 永続形式は破壊的変更(pre-release 方針で互換層
  なし)。既存 job は再生成が必要。
- tolerance 差分は全件 #101 既知クラスだが、#101 自体の根拠
  (FFT/SIMD 順序感度)は #98 時点の記録であり本変更で新規には
  検証していない。

## Follow-ups

- なし(本 Issue 範囲の置換・stamp・golden 回帰・最終ゲートは全て完了)。
