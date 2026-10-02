---
id: "SPEC-034yYlu5QuhtAFuUQWBBvQ"
type: "spec"
status: "proposed"
title: "常時実行の低コスト特徴量"
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
    - "media-ingest.md"
    - "analysis-dataset.md"
  blocks:
    - "reference-analysis.md"
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

# 常時実行の低コスト特徴量

## What Should Be Built

ソース素材を要求せず、解析可能なすべての音声ストリームに対して
常時実行する、広いキャプチャ単体の観測。

最低限保持する中間データ:

- デコード済み PCM
- 複素 STFT
- STFT magnitude
- STFT power
- PSD
- 固定帯域パワー
- 1/3 オクターブパワー
- スペクトル包絡
- L/R またはチャンネルペアの CSD(該当する場合)
- チャンネルペアのコヒーレンス(該当する場合)

複素 STFT は、後段の位相 / 群遅延 / 代替伝達解析を可能にするため
必須。

レベル / ラウドネス:

- サンプルピーク
- トゥルーピーク推定
- RMS
- DC オフセット
- クレストファクタ
- モーメンタリ / ショートターム / 統合ラウドネス(意味がある場合)
- ラウドネスレンジ
- 短窓レベル軌跡

スペクトル形状:

- セントロイド
- 帯域幅
- ロールオフ
- 傾斜
- 平坦度
- エントロピー
- フラックス
- 狭帯域ピーク / ノッチ候補

チャンネル / ステレオ観測:

- チャンネルごとのレベル
- L/R またはペアワイズ相関
- 複素コヒーレンス
- ステレオの Mid/Side パワー
- 帯域別 M/S パワー
- ILD / IPD(該当する場合)

過渡 / タイミング観測:

- オンセット強度 / 時刻
- 過渡ピーク / 持続の記述子
- 短窓の自己相似 / 反復ヒント

デジタル異常観測:

- クリッピング率
- 連続クリッピング持続
- ゼロラン
- スタックサンプルラン
- サンプル不連続 / クリックスコア
- 完全一致ブロック候補
- 量子化ステップ推定(信頼できる場合)

ノイズ / 変調:

- ノイズフロア推定
- トーン性ノイズピーク
- 50/60 Hz + 高調波候補
- 振幅変調スペクトル
- 粗いラグ / チャンネル変調(該当する場合)

マルチ解像度:

- 各特徴は明示的で適切な時間解像度を持つこと(例: サンプル /
  サブ ms、約 5 ms、約 20 ms、約 100 ms、約 500 ms、秒、
  セグメント全体の要約)
- すべての特徴を単一の 100 ms 窓に押し込めない

セグメント要約:

- 意味がある場合、平均 / 標準偏差だけでなく median、MAD、
  パーセンタイル、有効率を含む頑健な要約を生成する

## Why Now

cheap ステージは全解析の基底層であり、後段の reference / expensive /
イベント検出が再利用する中間スペクトルをここで揃える必要がある。

## Users and Value

- 下流ステージ: STFT / PSD を再計算せず再利用できる
- 解析者: 参照音源なしでもキャプチャ単体の異常兆候を把握できる

## Scope

### In Scope

- 上記の中間データ保持と各カテゴリの特徴量
- マルチ解像度の方針と頑健なセグメント要約

### Out of Scope

- リファレンスとの比較(`reference-analysis.md` に分離)
- 高コストな候補解析(`expensive-analysis.md` に分離)

## Requirements

- cheap ステージはリファレンス素材なしで完走する
- 中間スペクトルは後段ステージが再計算なしに再利用できる
- 特徴出力に分解能、window/hop、validity メタデータが含まれる

## Acceptance Criteria

- [ ] cheap ステージがリファレンス素材なしで完走する
- [ ] 中間スペクトルを後段ステージが STFT / PSD の再計算なしに
  再利用できる
- [ ] 特徴出力に分解能、window/hop、validity メタデータが含まれる

## Deferred Design Concerns

- 各特徴の具体的な窓長・帯域分割デフォルト | 計算コストと検証
  fixture とのバランス | fixture 実装時 | 解像度不足による見落とし

## Sources and Relations

全体仕様の Stage 1 を细化。データセット契約は
`analysis-dataset.md`、リファレンス比較は `reference-analysis.md`、
候補 / イベントは `candidate-event-model.md` を参照。
