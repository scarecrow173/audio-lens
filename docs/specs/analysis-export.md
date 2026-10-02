---
id: "SPEC-034yYlGGF9K5h2tGmQTOvS"
type: "spec"
status: "proposed"
title: "解析結果エクスポート"
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
    - "visualization-dataset.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/derived-visualization-dataset.md"
    - "../adr/analysis-dataset-storage.md"
    - "../adr/measurement-over-verdict.md"
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

# 解析結果エクスポート

## What Should Be Built

解析をアーカイブし、バグレポートと共に共有し、元のランタイム環境に
依存せず後から別ツール / AI が消費できるエクスポート / パッケージ
挙動を定義する。

パッケージ要件(エクスポートされる解析が含む / 参照するもの):

- manifest
- 入力ハッシュ / メタデータ
- locate 結果 / time map
- cheap / reference / expensive 特徴
- 保持された中間スペクトル / テンソル
- 残差音声
- イベントストリーム
- 可視化データセット
- アルゴリズム / バージョン / 来歴情報

人間可読サマリー(決定的かつ非 AI で以下を記述する):

- 入力
- 実行されたステージ
- アラインされたリファレンス
- valid / invalid のカバレッジ
- 主要な計測範囲
- 候補 / イベント数
- 出力パス

根本原因を推定してはならない。

可搬性:

- パッケージ内パスは再配置可能
- 大きな元入力メディアはエクスポートポリシーで設定可能な形で、
  複製ではなくハッシュ / パス参照にできる
- パッケージのバージョン / スキーマは明示的
- 任意のソースメディアが欠落しても、エクスポート済みの数値 /
  テンソルデータは読み取り不能にならない

## Why Now

解析結果をバグレポートや後続ツールに安全に引き渡すには、再配置
可能かつ来歴を保ったパッケージ形式が必要。

## Users and Value

- バグレポート作成者: 別環境で開ける自己完結パッケージを共有できる
- 後続コンシューマ: ハッシュで入力の同一性を検証できる

## Scope

### In Scope

- パッケージ内容、サマリー生成、可搬性・バージョニング要件

### Out of Scope

- パッケージを解釈する外部ツール / AI の実装

## Requirements

- パッケージは移動でき、独立した可視化 / 解析コンシューマで開ける
- コンシューマがハッシュから入力の同一性を検証できる
- エクスポートが解析の来歴と validity メタデータを保持する

## Acceptance Criteria

- [ ] パッケージが移動可能で独立した可視化 / 解析コンシューマで
  開ける
- [ ] コンシューマがハッシュから入力の同一性を検証できる
- [ ] エクスポートが解析の来歴と validity メタデータを保持する

## Deferred Design Concerns

- パッケージの物理形式(単一アーカイブ / ディレクトリ) |
  データサイズと運用要件に依存 | エクスポート実装時 | 大容量
  メディアの取り扱い方針未決によるパッケージ形式の変更コスト

## Sources and Relations

`analysis-dataset.md` と `visualization-dataset.md` の成果物を
束ねて可搬パッケージ化する。
