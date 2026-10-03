---
id: "TSPEC-034z87fIonHcU07IHGb1dE"
type: "test-spec"
status: "proposed"
title: "dataset 来歴と validity セマンティクス"
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

# dataset 来歴と validity セマンティクス

## Purpose

Analysis Dataset の来歴(provenance)記録と validity 区分が
spec の契約を守ることを検証する:「どの計測も生成経緯を一意に
辿れる」「計測不能と異常なしを混同しない」。
新しい成果物ロールを追加するたびに同じ規範を再利用する。

## Feature

dataset manifest の来歴再構成と validity セマンティクス。

## Rules

- 任意の成果物について、input id・algorithm id/version・
  パラメータ・alignment id・親/中間物の系譜を manifest から
  一意に辿れる
- スキーマバージョンとアルゴリズムバージョンは別フィールドで
  管理される
- 「異常なし(valid)」と「計測不能」は異なる validity で表現される
- validity 区分は spec の一式をすべて区別できる
- 上流未実装のロールは `not_produced` と明示され、
  「空の正結果」と区別される
- 宣言入力は内容のコピーではなく source_path + ハッシュの参照で
  記録される

## Examples

- Given manifest を読む、Then 各成果物の生成経緯を再構成できる
- Given 計測不能な特徴と異常なしの特徴、Then 両者の validity が
  異なる値で記録される
- Given 上流ステージ未実行の dataset、Then 対応ロールが
  `not_produced` で記録される
- Given 宣言入力、Then ソースファイルが dataset 内に複製されず
  参照のみ記録される

## Guarantees

- manifest だけで成果物の生成経緯を一意に特定できる
- schema_version と algorithm version が独立に読み取れる
- 「情報がない」ことと「正常」を混同する表現が存在しない

## Non-goals

- alignment id を実際に発行する locate ステージの動作
  (この時点ではフィールドの存在と構造のみを保証する)
- 特徴値そのものの正しさ(各解析ステージの責務)

## Risk

- 来歴が辿れないと後段の解析・エクスポートが「どう作られたか
  分からない値」を扱い、信頼性検証の前提が崩れる
