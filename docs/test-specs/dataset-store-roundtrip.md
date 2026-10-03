---
id: "TSPEC-034z87feL8uCPOIagychsT"
type: "test-spec"
status: "proposed"
title: "dataset store 往復と部分読み出し"
created: "2026-10-02"
updated: "2026-10-02"
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
  derives-from:
    - "docs/plans/analysis-dataset.md"
  derived-by: []
  verifies:
    - "docs/specs/analysis-dataset.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# dataset store 往復と部分読み出し

## Purpose

Analysis Dataset の各ストア(表・テンソル・音声)が
「全体を読み込まずに範囲 / チャンクで読め、書いた内容を
そのまま復元できる」ことを合成データで検証する。
ストア形式を変更する際に同じ規範を再利用する。

## Feature

列指向チャンク表・チャンクテンソル・WAV 音声の書き込み・
部分読み出し・往復。

## Rules

- 表は行範囲 / カラム選択で部分読みできる(全チャンクを読まない)
- テンソルはチャンク単位・範囲指定で部分読み出しできる
  (配列全体をメモリに展開しない)
- WAV 音声は書き出し → 読み出しで値が往復する
- 可変長文字列・NULL が表に正しく保持される
- 保存が要求される中間データがスカラー要約に置き換わらない

## Examples

- Given 10 万行超の合成テーブル(数値 + 文字列 + NULL)、
  When 行範囲を読む、Then 該当チャンクのみから正しい行が返る
- Given チャンク分割されたテンソル、When 部分範囲を読む、
  Then 境界をまたぐ値が正確に返る
- Given f64 PCM、When WAV に書き出して読み戻す、
  Then サンプル値が一致する
- Given 文字列・NULL を含む行、Then 往復で同じ値が返る

## Guarantees

- 部分読み出しが常に正しい値を返す(境界・端数チャンク含む)
- 書き込みは durable(途中クラッシュで部分ファイルを残さない)
- ストアの読み出し結果は書き込みパラメータで決定的

## Non-goals

- 真の Parquet / Zarr / FLAC との相互運用(export 側の責務)
- パフォーマンス最適化(実データ計測は運用フェーズ)

## Risk

- 部分読み出しが壊れると大規模テンソル契約が失われ、
  可視化・エクスポートが全体読み込み前提に退化する
