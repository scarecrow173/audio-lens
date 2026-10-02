---
id: "ADR-034yYyq7wrVYD7X3NvjeX0"
type: "adr"
status: "proposed"
title: "不変ジョブと再開可能なステージ実行を採用する"
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
  implements:
    - "../specs/analysis-job-model.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "analysis-dataset-storage.md"
    - "derived-visualization-dataset.md"
    - "../specs/audiolens-overview.md"
    - "../specs/analysis-job-model.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/initial-requirements-consolidation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 不変ジョブと再開可能なステージ実行を採用する

## Context and Problem Statement

解析は複数ステージで構成され、入力・設定・アルゴリズム・バージョンが
結果と不可分に結びつく必要がある。部分失敗や可視化だけの再構築を
前提にすると、ジョブを後から書き換える設計では来歴と再現性が
崩壊する。

## Decision Drivers

* ラン manifest だけで全出力の生成経緯を再構成したい
- 高コストステージの失敗が完了済みの下流 / 上流成果物を壊さない
  ようにしたい
- 可視化は解析済みデータから再生成できる必要がある
- 同一設定の再実行でバージョン間比較を可能にしたい

## Considered Options

* ジョブを可変セッションとして扱い、成果物を逐次上書きする
* ジョブを不変とし、成果物 id とステージ結果を immutable に
  記録して再開・再実行を別ジョブとして扱う
* ジョブは不変だがキャッシュ / 再開はサポートしない

## Decision Outcome

Chosen option: "ジョブを不変とし、成果物 id とステージ結果を
immutable に記録して再開・再実行を別ジョブとして扱う"。
再現可能性と部分失敗からの回復を両立させるため。

### Consequences

* Good, because 来歴が崩れず、キャッシュ再利用・部分再開・
  バージョン比較が可能になる。
* Bad, because ステージ状態と成果物 id の管理が必要で、
  実装は単純な一括実行より複雑になる。

### Confirmation

manifest から入力・アルゴリズム・バージョン・パラメータを再構成
でき、失敗後の再開が完了済み成果物を上書きしないことを確認する。

## Implementation Plan

* Affected paths: ジョブ定義、ステージ実行器、manifest、
  dataset / visualization / export の再入り口
* Patterns to follow: ジョブ・入力・成果物に安定 id を割り当て、
  ステージ結果を immutable に記録する
* Patterns to avoid: セッション内での成果物上書き、設定と結果の
  分離不能な状態管理
* Dependencies or configuration: 設定のシリアライズ形式は開発環境
  決定時に選定する

## Verification

* [ ] manifest だけで全出力の生成経緯を再構成できる
* [ ] expensive 失敗後に cheap / reference 成果物が保持される
* [ ] 可視化再構築に解析再実行が不要
* [ ] 同一設定の再実行が新しいジョブ id を持つ

## Pros and Cons of the Options

### ジョブを可変セッションとして扱い、成果物を逐次上書きする

* Good, because 実装が最も単純で状態も少ない。
* Bad, because 失敗時の復旧が難しく、来歴と再現性が失われる。

### ジョブを不変とし、成果物 id とステージ結果を immutable に記録して再開・再実行を別ジョブとして扱う

* Good, because 再現性・再開・バージョン比較をすべて満たせる。
* Bad, because ステージ管理と id 体系が必要になる。

### ジョブは不変だがキャッシュ / 再開はサポートしない

* Good, because 不変性だけは担保できる。
* Bad, because 高コストステージの再実行が毎回全行程になり、
  実用上の障害になる。

## More Information

入力ロール・ステージ選択・CLI 形状は
`../specs/analysis-job-model.md`、データセット契約は
`../specs/analysis-dataset.md` を参照。
