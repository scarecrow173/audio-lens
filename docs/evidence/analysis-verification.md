---
id: "EVID-D82T5lWcFNbk5XlZWnI6"
type: "evidence"
status: "approved"
title: "検証記録: 解析パイプライン — 特徴ファミリ・候補イベント・残差契約"
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

# 検証記録: 解析パイプライン

`al-analysis` の 3 層ステージ・特徴ファミリ・候補イベント・残差契約に関する検証結果の事実記録。設計判断は `docs/knowledge/analysis-pipeline.md` を参照。

## ステージ実装の検証

- DSP 中間生成物(複素 STFT/PSD/帯域/CSD/コヒーレンス): 既知正弦波の bin 位置、非 2 冪窓のゼロパディング、PSD の Parseval 積分、1/3 オクターブ帯域カバレッジ、同一信号 coherence=1 を `cargo test -p al-analysis` で確認。PSD は two-sided 正規化で全 bin を返す。
- cheap stage: 合成 WAV で `al run` E2E 完走し **46 特徴 ID / 83 レコード** を確認。silence/clip 検出・stereo ペア特徴・短入力の InsufficientSupport を検証。
- reference stage: `cargo test -p al-analysis` 23 件 pass(既知ゲイン 0.5/400Hz ゲイン回復、既知残差遅延 7 サンプルの群遅延・ラグ回復、R0–R3 エネルギー単調減少、同一信号で残差≈0、未受理 alignment の invalid 行)。
- CLI 配線: `cargo test -p al-cli` で capture-only の `ingest,cheap_analysis,dataset` 完走と、`--stages cheap_analysis` 単独での依存欠落入口拒否を確認。

## 候補イベントの検証

- events 単体テスト 4 件: 閾値超過でリンク付きイベント発火、閾値未満で無発火、invalid 行の validity 伝播、イベント ID 決定性、テーブル平坦化。
- 「クリーン入力でイベント 0 件」はヒューリスティックの性質上成立しない(広帯域信号が noise-floor に正当ヒットする)ため、抑制経路は config オーバーレイで閾値を上げる形で検証した。
- expensive 結合テスト 5 件: clipping 候補の expensive 区間起動と cheap 非上書き、抑制時の events 空テーブル、always_run が reference なしで全入力解析、manual_ranges が `manual` トリガ区間生成、非 Valid 候補のみの区間が全レコード null+非 Valid。
- ケプストラム遅延: 既知エコー遅延 120 サンプル(2.5ms@48kHz)を回復。

## #29(expensive 残存精度、PR #26 レビュー)の修正検証

- `gh issue view 29` で原文 9 項目に差分なしを確認し、13 件の AC-29 回帰テストを追加。`cargo test -p al-analysis` 98 tests pass(unit 68、avsync 2、expensive 6、golden 19、reference 3)。
- 修正した不具合の例: `mix_band_coeff` で condition>1e3 でも係数が Valid のまま(→ LowCoherence+理由に demote)、不均一 ch 長でフィルタ後 index による ch 誤ラベル、非プライマリ ch の slice panic、`splice_first_s`/`codec_block_size`/`src_candidate_rate` の「候補なしを 0+Valid で出力」、early/late 窓の circular wrap 混入、`Channel(0)` への pair summary 誤紐付け、継承した暗黙 skip の NaN+InsufficientSupport 化。

## #33(spec 残差適合)の判断と検証

- 3 項目の承認結果: (a) 候補検出の独立ステージ化は仕様改訂(fused 構成を承認)、(b) FLAC は defer(WAV のみ)、(c) `Resolution.sample_rate` は第一級 u32 実装を承認。
- 変更前 red: `level.trajectory` の sample_rate が Null、`ref.band_gain_m2` の sample_rate が Null。変更後 exit 0。32/44.1kHz cheap、8/32kHz expensive、原入力 44.1→解析 48kHz で正規化後レートが保存されることを確認。
- docs-only 仕様適用(#33 確定)の検証: 一時検証テストを作成・実行・除去して空 `events/candidates` テーブルが `row_count()==0` で実 DatasetReader から読めることと独立 artifact record を確認。計既存 13 件+一時 1 件 pass。恒久 assertion は未追加と明記。

## 帯域別ラグ補償ゲイン(#38)の検証

- `cargo test -p al-analysis -p al-locate -p al-math` **181 passed / 0 failed**(al-analysis 89・golden 19、al-locate 65・golden 16、al-math 27)。
- 検証ケース: 8kHz/48kHz、2 帯域 25ms/15ms 遅延、0.5/1.4 gain、2ch 極性反転。上位半分・時間同点・偶数中央値・無音・短入力・探索端 peak・無相関大振幅雑音を網羅。golden 19 件の期待範囲を維持。

## キャプチャ間経路校正(#41)の検証

- `cargo test -p al-analysis -p al-dataset` **135 passed / 0 failed**(golden 19 含む)。実 Locate→Cheap→Reference→Expensive の integration で 2 倍の経路差回復・job スコープ保存・unsupported 記録・ペア JSON 不変を確認。

## golden 行列の構成

- al-analysis golden は 19 ケース(既知 gain/EQ/位相/固定・jitter 遅延/SRC drift/極性/ch swap/低コヒーレンス/clip/comb/echo/SRC alias/drop/repeat/stuck)。「非コヒーレントな PSD 比による高域リフト偽装」を名前付き失敗モードとして含む。

## 未検証・制約として記録された事項

- 実ゲームキャプチャ規模の性能・スループットは non-goal どおり未検証。
- `dig.quant_step_est` は高ビット深度素材で感度が落ちる既知の限界。
- `ref.group_delay_median_s` は低 coherence 帯域でノイズに左右される(coherence 重み付けは follow-up)。
- STFT/R3 畳み込みは当初 O(n·tap) の素朴実装で長尺ペアが遅い(後の性能施策で `dsp::conv` overlap-save へ移行、約 52–54 倍。`docs/evidence/performance-measurements.md` 参照)。
- 候補のみ再実行の独立 StageKind 提案は残置。
