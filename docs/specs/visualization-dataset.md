---
id: "SPEC-034yYlFXqQaDagzQr4eXWW"
type: "spec"
status: "proposed"
title: "可視化データセット"
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
    - "analysis-dataset.md"
  blocks:
    - "analysis-export.md"
  supersedes: []
  superseded-by: []
  related:
    - "../adr/derived-visualization-dataset.md"
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

# 可視化データセット

## What Should Be Built

Analysis Dataset から、意味解析を再実行せずにレンダリング可能な
可視化データを生成する。可視化データは構造化解析データとは別の
派生成果物である。

共有タイムライン(すべての可視化系列が共通のキャプチャタイムラインに
アライン可能であること):

- 映像位置
- 波形
- ソース出現
- locate 確度
- ラグ / time map
- ゲイン
- 帯域メトリクス
- 残差エネルギー
- イベント / 候補

波形 LOD ピラミッド(対象・リファレンス・残差向けに複数レベルの
バケットを生成し、少なくとも以下を含む):

- min
- max
- RMS

長いキャプチャを全ズームレベルで生 PCM を読まずに描画できるように
するため。

スペクトログラム製品(レンダリング可能な時間-周波数タイル / レベル):

- 対象
- アライン済みリファレンス
- 差分
- 残差ステージ
- コヒーレンス(意味がある場合)

差分スペクトログラムは dB 域の対象-リファレンス可視化をサポートし、
validity マスクを保持する。

帯域ヒートマップ(時間 × 帯域のレンダーデータ):

- ゲイン
- 相関
- コヒーレンス
- 残差エネルギー
- その他の帯域メトリクス

伝達ビューデータ(同期化された以下のデータ):

- magnitude
- 位相
- 群遅延
- コヒーレンス

選択時間範囲に対して median やパーセンタイル包絡のような頑健な
要約を可能にする。

locate / time-map ビュー(レンダーデータが以下を公開):

- ソース時間 vs キャプチャ時間
- アンカー
- 区分セグメント
- ラグ
- 局所時間スケール
- 確度
- スキップ / 再開 / 未マッチ観測

ステレオ / チャンネルビュー:

- 時間経過の L/R またはペアワイズ相関
- M/S 比
- 帯域別幅
- IPD / ILD
- 複素コヒーレンス
- source→output 応答比較
- 選択した短範囲への任意のベクトルスコープ / ゴニオメータデータ

残差ビュー(並列 / 同期アクセスを許可):

- キャプチャ
- アライン済みリファレンス
- 予測 / リファレンス補正信号
- R0 / R1 / R2 / R3
- 波形 / スペクトログラム / PSD / 帯域エネルギービュー

イベントマーカー(構造化イベント id にリンクしたコンパクトな
レンダリング可能マーカーを出力)。

ルール: visualization builder は保存済み Analysis Dataset だけから
再現可能でなければならず、新しい診断セマンティクスを発明しない。

## Why Now

可視化を解析と分離しないと、表示のたびに再解析が必要になり、
スクリーンショット的な使い捨て出力に劣化する。

## Users and Value

- 可視化コンシューマ: メディアデコードや特徴抽出なしに再構築できる
- 長尺キャプチャの利用者: LOD / チャンク化データで高速な
  ズーム / パンが可能

## Scope

### In Scope

- 上記の共有タイムライン、LOD ピラミッド、スペクトログラム、
  ヒートマップ、伝達 / time-map / ステレオ / 残差ビュー、
  イベントマーカー
- confidence / validity マスクの保持

### Out of Scope

- UI 実装そのもの
- 新しい意味解析の追加

## Requirements

- 可視化はメディアデコードや特徴抽出なしに再構築できる
- 長尺ファイルが LOD / チャンク化データで効率的にズーム / パンできる
- confidence / validity マスクが焼き込まれず可視化に供給される

## Acceptance Criteria

- [ ] 可視化がメディアデコードや特徴抽出なしに再構築できる
- [ ] 長尺ファイルが LOD / チャンク化データで効率的に
  ズーム / パンできる
- [ ] confidence / validity マスクが可視化に利用可能な形で残る

## Deferred Design Concerns

- タイル / LOD の具体的粒度と保存フォーマット | フロントエンド選定と
  データ量の計測に依存 | 可視化コンシューマ実装時 | 過度に大きな
  派生データの生成

## Sources and Relations

`analysis-dataset.md` の成果物のみから再現可能であること。
派生データ方針の決定は ADR `../adr/derived-visualization-dataset.md`
を参照。
