---
id: "TSPEC-034z0HtvsU1qWh7kPAH0ml"
type: "test-spec"
status: "proposed"
title: "AV タイミング観測と辞書注釈"
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
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ingest-av.md"
  derived-by: []
  verifies:
    - "docs/specs/video-av-timing.md"
    - "docs/specs/media-ingest.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# AV タイミング観測と辞書注釈

## Purpose

CFR を仮定しないタイムスタンプ観測と、キャプチャアーティファクト
辞書の「注釈のみ」契約を合成系列で検証する。実コンテナパーサが
追加されても、観測ロジックの規範はこのまま再利用する。

## Feature

タイムスタンプ系列からの間隔・ドリフト・不連続観測と辞書注釈。

## Rules

- タイムスタンプ系列は公称 FPS 演算ではなく一次情報として扱う
- フレーム間隔系列・ドリフト・不連続候補・重複/ドロップ候補が
  観測として出力される
- 音声と映像のクロックは個別にアドレス可能なまま保持される
- 辞書照合は観測への注釈のみを付与し、生の計測値を変更しない
- 辞書の結果には辞書バージョンが紐付く
- タイムスタンプ源が不在でもモジュール自体は明示的に空結果を返せる

## Examples

- Given 等間隔の合成 PTS 系列、Then ドリフト・不連続は観測されない
- Given VFR の合成 PTS 系列(間隔が変動)、Then 間隔系列と
  実測分布が出力され公称 FPS に丸められない
- Given 途中でジャンプする PTS 系列、Then 不連続候補として記録される
- Given 辞書に既知のブロックサイズ項目、When 照合、Then 観測へ
  注釈が付き生値は変わらない

## Guarantees

- 同じタイムスタンプ系列から常に同じ観測結果が得られる(決定性)
- 観測結果と辞書注釈が別フィールドで保持される
- 映像証拠が無くても音声側の記録は完結する

## Non-goals

- 実コンテナからの映像 PTS 抽出(別プラン)
- ゲーム固有の映像イベント検出器
- 音声イベントとの自動同期判定(後段ステージの責務)

## Risk

- CFR 仮定が混入すると VFR キャプチャの異常を作為として誤認し、
  解析の土台そのものが歪む
