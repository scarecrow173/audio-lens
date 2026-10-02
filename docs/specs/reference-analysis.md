---
id: "SPEC-034yYluri8PXVLEWU6M3qK"
type: "spec"
status: "proposed"
title: "リファレンスアライン比較解析"
created: "2026-10-02"
updated: "2026-10-02"
owners: ["scarecrow173"]
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
  depends-on:
    - "source-localization.md"
    - "cheap-analysis-features.md"
    - "analysis-dataset.md"
  blocks:
    - "expensive-analysis.md"
    - "candidate-event-model.md"
  supersedes: []
  superseded-by: []
  related: []
  refines:
    - "audiolens-overview.md"
  refined-by: []
  derives-from:
    - "../discovery/initial-requirements-consolidation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# リファレンスアライン比較解析

## What Should Be Built

locate が信頼できるソース / キャプチャ対応を与えたとき、アラインされた
キャプチャがリファレンスとどう異なるかを、単一の類似度スコアに
潰さず計測する。

アラインメント / タイミング特徴:

- 局所最適ラグ τ(t)
- ラグの median / MAD / パーセンタイル
- ラグ微分 / ドリフト
- ラグのステップ / 不連続候補
- 局所時間スケール係数
- ピーク幅 / 曖昧性
- 窓ごとのアラインメント確度

レベル特徴:

- 最小二乗ゲイン
- 400 Hz ゲイン
- 区間 RMS / リファレンス RMS
- 短窓ゲイン軌跡 g(t)
- ソースレベル対ゲイン関係

帯域特徴(最低限):

- 既存 M2 アプローチによる 1/3 オクターブゲイン
- 400 Hz に対する比率
- 帯域別相関
- 帯域別コヒーレンス
- 二次 / 参考証拠としての PSD 比

低相関帯域を同等に信頼できる推定として報告してはならない。
相関 / 確度と、推定器が要求する「下限的」解釈メタデータを保持する。

類似度観測:

- フルバンド正規化相関
- 帯域別相関
- PHAT 系アラインメント証拠
- スペクトル包絡類似度

複素伝達(サポートが十分な場合、セグメント形と時間変動形の両方を
保持する):

- CSD
- 複素伝達 H(f)
- 複素伝達 H(f,t)
- magnitude 応答
- アンラップ位相
- 群遅延
- コヒーレンス / サポート

チャンネル / ステレオ:

- source→capture チャンネル混合行列
- モノソースに対する出力ごとの応答
- L/R 応答差
- 帯域別 L/R 複素コヒーレンス(magnitude と位相の両方を保持)
- 帯域別チャンネル間相関 / サポート
- M/S 変化
- ソースに対する低域 L/R 類似度変化
- 極性インジケータ
- クロストーク / リーク観測
- モノソース素材では source→L と source→R の伝達 / コヒーレンス差が
  第一級の計測であり、ステレオ要約に縮約しない

ダイナミクス / 過渡 / ピッチ:

- ソースレベル→ゲイン関係
- 過渡オンセットオフセット
- 過渡強度比
- F0 比(有効な場合)
- 高調波周波数比
- 持続 / 時間スケール比

段階的残差(生成・保持する):

- R0: 時間アラインメント後
- R1: スカラー / 時間変動ゲイン補正後(設定に従う)
- R2: チャンネル行列補正後
- R3: サポートされた線形伝達補正後

各残差について保持するもの: 波形、STFT、PSD、帯域エネルギー、
RMS / リファレンス比、ソース相関。

残差は説明されていない信号の観測であり、根本原因の診断ではない。

## Why Now

「どこが・どれだけ違うか」を周波数・時間・チャンネルの各軸で
分離して記録しないと、キャプチャ系の劣化とゲーム側の変化を
切り分けられない。単一スコアへの縮約は情報を不可逆に失う。

## Users and Value

- 解析者: 帯域別・位相・チャンネル別の差分と、説明しきれなかった
  残差を階層的に調べられる
- expensive 解析: 候補区間の根拠となる観測を得られる

## Scope

### In Scope

- 上記のアラインメント / レベル / 帯域 / 類似度 / 複素伝達 /
  チャンネル / ダイナミクス特徴
- R0〜R3 の段階的残差とその付随データ

### Out of Scope

- 残差の原因推定
- 高コストな詳細解析(`expensive-analysis.md` に分離)

## Requirements

- 400 Hz、1/3 オクターブ M2、400 Hz 相対ゲイン、帯域相関、
  帯域別 L/R コヒーレンス(magnitude / 位相)が表現される
- magnitude だけの結果が、利用可能な位相 / コヒーレンス情報を
  捨てない
- 段階的残差により、各モデルクラスがどれだけ説明したかを下流解析が
  判断できる

## Acceptance Criteria

- [ ] 400 Hz、1/3 オクターブ M2、400 Hz 相対ゲイン、帯域相関、
  帯域別 L/R コヒーレンス magnitude / 位相の要件が表現される
- [ ] magnitude のみの結果が位相 / コヒーレンス情報を捨てない
- [ ] 段階的残差から各モデルクラスの説明量を下流が判断できる

## Deferred Design Concerns

- M2 推定器の詳細仕様と帯域数 | 既存実装の移植時に確定 |
  実装着手時 | 帯域数不足による分解能低下

## Sources and Relations

`source-localization.md` のアラインメント結果と
`cheap-analysis-features.md` の中間スペクトルを入力とする。
候補駆動の高コスト解析は `expensive-analysis.md`、イベント化は
`candidate-event-model.md` を参照。
