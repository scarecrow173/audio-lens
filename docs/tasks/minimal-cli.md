---
id: "TASK-034yZBGR8sboPsFlB6FpMr"
type: "task"
status: "todo"
title: "最小 CLI(run / resume / inspect)の実装"
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
    - "docs/tasks/stage-runner.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 最小 CLI(run / resume / inspect)の実装

## Work

`al-cli` に最小限の CLI を実装し、ジョブの実行(run)、再開(resume)、
manifest の確認(inspect)ができるようにする。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-job-orchestration-foundation.md`
- `docs/specs/analysis-job-model.md`

## Dependencies

- `docs/tasks/stage-runner.md`

## Affected Files

- `crates/al-cli/src/main.rs`

## Done When

- [ ] `run` でジョブを作成してステージ列を実行できる
- [ ] `resume` で未完了ステージを再開できる
- [ ] `inspect` で manifest の内容を確認できる

## Verification

- [ ] `mise exec -- cargo run -p al-cli -- --help` が動作する
- [ ] run → inspect で manifest が読める
