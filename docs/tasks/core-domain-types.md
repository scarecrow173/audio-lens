---
id: "TASK-wjixVNPct4FFdTgz7uqvU9"
type: "task"
status: "todo"
title: "共有ドメイン型と安定 id の実装"
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
    - "docs/tasks/init-rust-workspace.md"
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

# 共有ドメイン型と安定 id の実装

## Work

`al-core` に共有ドメイン型(JobId, ArtifactId, InputRole, StageKind,
Validity, Confidence 等)と安定 id の生成・検証を実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/analysis-job-model.md`
- `docs/designs/job-orchestration.md`

## Dependencies

- `docs/tasks/init-rust-workspace.md`

## Affected Files

- `crates/al-core/src/lib.rs`
- `crates/al-core/src/ids.rs`
- `crates/al-core/src/types.rs`

## Done When

- [ ] 入力ロール・ステージ種別・validity・confidence が型として
  表現されている
- [ ] id の生成が決定的・衝突しない実装になっている
- [ ] 単体テストが存在する

## Verification

- [ ] `mise exec -- cargo test -p al-core` が通る
