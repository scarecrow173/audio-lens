---
id: "TSPEC-034yZAoH14s9ut6X3MkhcD"
type: "test-spec"
status: "proposed"
title: "ジョブ不変性と来歴の再構成"
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
    - "docs/designs/job-orchestration.md"
  derived-by: []
  verifies:
    - "docs/specs/analysis-job-model.md"
    - "docs/adr/immutable-resumable-job.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ジョブ不変性と来歴の再構成

## Purpose

不変ジョブと manifest 系譜が「同じ条件なら同じ来歴を持ち、別条件なら
別ジョブとして記録される」ことを保証するための検証仕様。ジョブ
モデルが安定している間は保持し、モデル自体が廃止された場合のみ
退役する。

## Feature

分析ジョブの不変性と manifest による来歴の完全な再構成。

## Rules

- Rule: ジョブは生成後に変更されず、再実行は常に新しいジョブ id を
  発行する
- Rule: manifest は入力・設定・各成果物 id・生成経緯を含み、それ
  だけで成果物の来歴を再構成できる
- Rule: 成果物 id は安定していて、同じ生成手続きから同じ id が
  参照できる

## Examples

- Example: 同一入力・同一設定で run を 2 回実行すると、2 つの異なる
  ジョブ id とそれぞれ独立した manifest が作られる
- Example: manifest を読み込むだけで、ある成果物がどの入力・
  どのステージ・どの設定から生成されたかを再構成できる

## Guarantees

- ジョブの定義が一度確定したら以後の処理がそれを上書きしない
- manifest から全成果物の生成経緯を一意に辿れる
- 再実行時に成果物が混線しない(id 衝突しない)

## Non-goals

- 各ステージ内部の計算正当性(別の test-spec が担う)
- CLI の使い勝手や出力フォーマットの細部

## Risk

この保証を失うと、解析結果の再現性と監査可能性が崩れ、
「どの条件でこの結果が出たか」を後から説明できなくなる。
