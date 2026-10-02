---
id: "TSPEC-034yZAoH14s9ut6X3MkhcE"
type: "test-spec"
status: "proposed"
title: "ステージランナーの部分再開"
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

# ステージランナーの部分再開

## Purpose

stage-runner が依存順でステージを実行し、途中失敗しても完了済み
成果物を壊さず未完了部分だけを再開できることを保証するための
検証仕様。ランナーの実行セマンティクスが変わらない限り保持する。

## Feature

依存順実行と部分再開を備えたステージ実行。

## Rules

- Rule: ステージは宣言された依存関係の順序で実行される
- Rule: 失敗したステージ以降は実行せず、完了済み成果物は保持される
- Rule: 再開時は未完了のステージだけが実行され、完了済み成果物は
  manifest の記録に基づいて再利用される
- Rule: ステージ間の受け渡しは成果物 id / manifest 参照で行い、
  暗黙の共有状態に依存しない

## Examples

- Example: A → B → C のステージ列で B が失敗した場合、A の成果物は
  保持され、resume すると B と C だけが再実行される
- Example: すべてのステージが成功した後に resume しても、副作用の
  ある再実行は発生しない

## Guarantees

- 部分失敗が完了済み成果物を無効化しない
- 再開がべき等で、余計な副作用を生じさせない
- 実行順が依存関係を破らない

## Non-goals

- 各ステージの計算正しさ
- 並列実行や分散実行の性能

## Risk

この保証を失うと、失敗時の部分再開が信頼できなくなり、
長時間解析の再実行コストが致命的になる。
