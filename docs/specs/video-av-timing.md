---
id: "SPEC-034yYlFytDPfSSaZovUrRo"
type: "spec"
status: "proposed"
title: "映像タイミングとAV同期"
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
  blocks: []
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

# 映像タイミングとAV同期

## What Should Be Built

固定フレームレートや共有クロックを仮定せずに映像タイミングを保存・
解析し、音声異常を映像イベントと比較できるようにする。

低コスト映像特徴:

- フレーム PTS
- フレーム間隔系列
- 重複 / ドロップフレーム候補
- シーンチェンジタイムスタンプ
- 粗い輝度変化系列
- 粗いモーション / アクティビティ系列

A/V タイミング(永続化するもの):

- オーディオストリームクロック / PTS
- ビデオストリームクロック / PTS
- コンテナレベルのオフセット
- クロック間ドリフト
- 不連続候補

イベント同期(音声オンセット / イベントと映像イベントタイムスタンプを
比較する一般機構):

例: フラッシュ / 衝撃 / モーションオンセット ↔ 音声オンセット、
ユーザー提供の映像マーカー ↔ 音声イベント。

出力:

- 音声イベント時刻
- 映像イベント時刻
- 差分
- マッチング確度
- 反復イベント間の分布
- 時間をまたぐドリフト / 傾き

コアスキーマにゲーム固有の映像検出器をハードコードしない。

## Why Now

音声異常と映像イベントの対応づけはデバッグの主要な手がかりだが、
VFR・クロック分離を前提にしないと比較そのものが誤る。

## Users and Value

- 解析者: 音声イベントと映像イベントの時間差・分布・ドリフトを
  独立した証拠として参照できる

## Scope

### In Scope

- 上記の映像タイミング特徴と A/V タイミング記録
- 汎用のイベント同期機構

### Out of Scope

- ゲーム固有の映像検出器の実装
- 音声アラインメントと映像解析の統合判定

## Requirements

- VFR 入力は公称 FPS 演算ではなくタイムスタンプで扱う
- A/V 同期出力はソース音声アラインメント出力と分離されたままである
- 映像証拠が利用不能でも音声解析をブロックしない

## Acceptance Criteria

- [ ] VFR 入力が公称 FPS 演算ではなくタイムスタンプで処理される
- [ ] A/V 同期出力がソース音声アラインメント出力と分離される
- [ ] 映像証拠が利用不能でも音声解析がブロックされない

## Deferred Design Concerns

- 映像イベント検出の具体手法 | 実装技術と対象ゲームに依存 |
  実装着手時 | 汎用機構のないまま個別検出器が増殖するリスク

## Sources and Relations

`media-ingest.md` の映像メタデータを入力とする。イベント同期は
`candidate-event-model.md` のイベントモデルを参照する。
