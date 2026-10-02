---
id: "TASK-034yZBFMasR7zBTzyrlevq"
type: "task"
status: "todo"
title: "ステージ trait と stage-runner の実装"
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
    - "docs/plans/implement-job-orchestration-foundation.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/job-manifest-registry.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/stage-runner-resume.md"
  references: []
  defers: []
  deferred-by: []
---

# ステージ trait と stage-runner の実装

## Work

ステージの共通 trait と入出力契約を定義し、依存順実行・成果物再利用・
部分再開を行う stage-runner を `al-job` に実装する。ステージ間の
受け渡しは成果物 id / manifest 参照に限定する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/analysis-job-model.md`
- `docs/adr/immutable-resumable-job.md`
- `docs/designs/job-orchestration.md`

## Dependencies

- `docs/tasks/job-manifest-registry.md`

## Affected Files

- `crates/al-job/src/stage.rs`
- `crates/al-job/src/runner.rs`

## Done When

- [ ] ステージが trait で表現され、依存順に実行される
- [ ] 失敗ステージ以降は実行されず、完了済み成果物が保持される
- [ ] resume が未完了ステージだけを実行し、完了済み成果物を
  manifest の記録から再利用する

## Verification

- [ ] `mise exec -- cargo test -p al-job` が通る
- [ ] 依存順違反が検出できるテストが存在する
