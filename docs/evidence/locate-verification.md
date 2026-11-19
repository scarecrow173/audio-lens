---
id: "EVID-68j7u7VkskFCAIDxfbrC"
type: "evidence"
status: "approved"
title: "検証記録: ソース定位 — 曖昧性閾値較正・リサンプル・探索性能"
created: "2026-11-19"
updated: "2026-11-19"
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
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 検証記録: ソース定位 — 曖昧性閾値較正・リサンプル・探索性能

本書は `al-locate` とその呼び出し側(`al-analysis`)に関する実行検証・計測の事実記録である。設計判断の根拠は `docs/knowledge/locate-and-tracking.md` にまとめてある。記録するのは観測された事実のみであり、未実施の検証はその旨を明記する。

## argmax 同点検出の閾値較正(Issue #59)

### 検証したこと

相関曲線 argmax の同点判定パラメータ `TIE_REL_EPS` / `TIE_ABS_FLOOR` を決めるため、二つの計測を行った。(a) scalar 実装と SIMD dispatch 経路の出力偏差の実測、(b) 受理系・拒否系 fixture における argmax マージン(`M − max_{i≠argmax}|corr[i]|`)の分布。本番コードは変更せず、計測用テストのみを追加した。

### 観測された結果

**dispatch 偏差**(dispatch カーネル vs `pulp::Scalar.vectorize`、AVX 系):

- Correlation n=65536(signal 40000 + template 4096)は PHAT 有無とも 0(bitwise 一致)。
- n=32768(signal 24576 + template 8192)は PHAT なし 3.47e-18、PHAT あり 1.74e-18。
- Envelope fft_n=2048/8192 は 0(bitwise)、fft_n=4096 は 4.44e-16。
- 最大 dispatch 偏差は実測約 5e-16、伝播込み保守値で約 1e-14。

**受理系マージンの最小値**(fixture × 手法):

- 全局最小は 1.65e-8(`case_loop_material` × `spectral_envelope`)。ループ素材の正当な複数出現が近同点の包絡スコアを持つため。
- ほかに pin-periodic-control SE 1.07e-7、case_exact_insert SE 3.73e-7、polarity_inverted SE 6.70e-7 など。
- ショルダーデルタの最大曝露は case_long_source_linear_drift × full_ncc で +3.38e-4(第二順位が argmax−1 のショルダー)。

**拒否系の同点構造**:

- pin-periodic で full_ncc の margin は 8.88e-16、|T|=28(周期 1024 の倍数位置にある分離ピークの厳密同点。plateau ではない)。
- spectral_envelope は margin 0.0、|T|=219(hop 評価 lag が厳密 1.0)。
- band_ncc(margin 2.58e-4)と gcc_phat(margin 1.09e-1)は |T|=1 で、厳密周期が崩れるために同点にならない。

### 下された判断

`TIE_REL_EPS = 1e-11`、`TIE_ABS_FLOOR = 1e-11` を採用。spec 候補レンジ(相対 1e-9〜1e-6、floor 約 1e-9)から意図的に下方へ較正した。下側は dispatch 偏差(~5e-16 実測)に対し約 2 万倍、上側は最小受理マージン(1.65e-8)に対し約 1.7×10³ 倍のクリアランスがある。判定が反転し得る不可避の不安定帯は絶対幅約 1e-15 に留まる。閾値較正のためのエスカレーションは不要だった。

### 導入後の回帰計測(T4)

`argmax_tie_count` 追加の性能影響を bench_ncc / bench_locate_prepared で計測した(負荷環境で参照系が 176〜831ms と 4.7 倍変動)。best-of-4 で FullNcc +6.2%、BandNcc +5.1%、GccPhat −4.4%、SpecEnv +12.4%(同コミットの `corr0` 移行を含み方向が逆のためノイズと判断)。判定: 証拠評価あたり O(n) 走査1回の追加で median ±7% 以内に収まり、有意な劣化なし。`cargo test --workspace` は全件 green(golden 19 件含む)、clippy/fmt/build green。

## IPD lag 検索窓・signed 原点の修正(#52 Slice1)

### 検証したこと

cheap stage の `ipd_lags` が同長 block を NCC に渡し合法 clamp 0 で固定偽 lag 0 を返していた不具合を修正し、両 channel 長の min と block+2margin の完全窓のみを分析する形に改めた。修正前後の回帰と、独立計測 baseline の取得。

### 観測された結果

- 修正前の red 確認: `ipd_lag_drift_reports_negative_to_positive_change_for_first_pair` が Some(0)(期待 Some(8))、`ipd_lag_zero_width_exact_block_is_observed` で `mod.lag_drift_samples` record 欠落。
- 修正後 `cargo test -p al-analysis`: 145 passed(unit 99 + avsync 7 + expensive 8 + golden 19 + reference 12)。clippy -D warnings、fmt とも clean。
- 計測 baseline(commit `f12e920`、6 case × raw 3000 call): band-ncc p50 0.3212ms/p95 0.6423ms、band-phat 0.3218/0.5898、ipd-ncc 0.3205/0.6329、ipd-phat 0.3195/0.6038、large-ncc 10.4499/13.0423、large-phat 10.7528/16.2469。fixture SHA256 は channel0 `207422e9…1eaa5`、channel1 `bfc787fd…e11e4`。計測 artifact は `target/ncc-reuse-evidence/` に保存(ignored、長期証拠は実装記録の hash 要約)。

## InputCache + coarse-to-fine 間引き探索(#40 系)

### 検証したこと

長尺ソース向けに、(a) BandNcc のペアごと bandpass 重複を `InputCache` で再利用、(b) rate/4 間引きの粗提案 + 原レート近傍精密化、を導入した際の実測性能。

### 観測された結果(2026-10-10、Windows / i9-9900K / Rust 1.98.1)

48kHz・35秒 mono capture(1,680,000 frames)・20秒 source 2本(seed 901)、warmup2 + 測定7:

- cold 準備(2素材+capture): median 780.8ms / p95 841.5ms。
- 未キャッシュ BandNcc: 3006.5 / 3112.2ms。準備済み BandNcc: 2519.8 / 3036.7ms(**中央値で約16.2%短縮**)。
- 1/4 粗提案のみ: 100.9 / 102.2ms。全探索 FullNcc: 588.8 / 607.3ms。粗候補+精密化+全域 FullNcc: 715.3 / 746.7ms(全域単独比で約21.5%増加)。
- 2ペア cold 経路全体: 4112.0 / 4489.6ms(未キャッシュ BandNcc + FullNcc の 3662.9 / 3791.9ms に対し約12.3%増加)。

### 下された判断

準備済み BandNcc の再利用効果(約16%)は確認できたが、F3 契約(最終証拠は全メソッドで全探索域から生成)がある限り全探索の相関計算は省略できず、初回高速化の目標は未達だった。粗提案は FullNcc 全探索を二重計算にしない設計のまま受け入れ、性能改善の中心はフィルタ済み信号再利用と結論した。テストは `cargo test -p al-math -p al-locate -p al-analysis` 176 件 pass(locate golden 16 件を含む)。

## サンプルレート正規化(#37 系)

### 検証したこと

`sample_rate_mismatch` リジェクトを「polyphase FIR でリサンプル統一」に転換する変更の検証。

### 観測された結果

`cargo test -p al-locate -p al-math -p al-analysis` 146 件 pass(locate golden 16・analysis golden 19 不変)。確認項目: 44.1k/48k 独立 chirp の定位で偽 drift なし、scalar/SIMD 等価性、alias/image 抑制、遅延補償、overshoot 保持、整数 PCM への一度だけのスケール適用、manifest 復元後の reference/expensive で同一 PCM・区間・lineage、保存 PCM 欠落時の両 stage エラー。`cargo fmt --check` は既存 `examples/bench_dsp.rs:22` のみ失敗(範囲外として保持)。

## 残差 PCM 保存(Issue #32 残件)

### 検証したこと

two-source 残差探索で減算後の残差 PCM を WAV として永続化し、child `capture_pcm` が元キャプチャではなく残差を明示参照する契約の検証。

### 観測された結果

回帰テストは修正前に child 使用 PCM が `normalized_pcm` を指し失敗、修正後成功。対象テスト計93件 pass、`mise run fmt`・`mise run lint` 成功。保存前後のサンプル・RMS は完全一致で、manifest 再読出し後に減算信号と一致・元キャプチャと異なることを確認した。

## time_map の dataset materialize(AL#191)

### 検証したこと

`time_map.json` を `DatasetStage::materialize_locate` が `locate/<src>/<cap>/time_map.json` へ複写し、`al.ds.read` kind=`json` → `SyncTimeline` への実データ経路。

### 観測された結果

`cargo test -p al-dataset` PASS(14 integration tests、f64 score の verbatim read-back・rejected pair の非存在を検証)。`node --test viz-timeline.test.ts` 9 tests、`npm --prefix ui test` 113 tests、`tsc --noEmit` すべて PASS。wire 経路テストでは piecewise 補間値 0.25s が detail の定数オフセット写像 0.5s と異なることを assert した。実 GUI(al-gui/al-desktop)での描画配線は対象外(headless のみ検証)。

## locate 全体の回帰状況

- `cargo test -p al-locate` は各段階で全 pass(golden 16 件を含む)。golden 行列は exact/gain/EQ 挿入・silence 境界・強周期偽ピーク・loop・repeated・長尺 offset/drift・skip+resume・multichannel downmix・BGM マスキング+残差探索を名前付きケースとして持つ。
- CLI 結合テストは5件で `al run --stages ingest,locate` による locate_result artifact 登録、time_map/residual artifact 生成、窓 anchor 追跡、drift 区間、skip+resume 区間を検証する。

## 未検証・残課題として記録された事項

- `best_window_match` 第3段の全ペア同点退化は回帰で未観測(発生時は occurrence 毎アンカー退化の受諾済み挙動)。
- 短窓(anchor_window=512)の NCC 曲線は T1 計測対象外で、完了判定の記録がない。
- coarse-to-fine の小窓閾値採否に関する実測記録が残っていない。
- 計測 artifact(`target/ncc-reuse-evidence/`)は ignored 領域で `cargo clean` 等で失われ得る。長期証拠は実装記録の hash 要約のみ。
- 独立 reviewer による review は環境制約で未実施だった段階がある(各実装記録の記述どおり)。
