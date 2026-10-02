---
id: "TASK-J14iCJv5kCKP839iZfaROa"
type: "task"
status: "done"
title: "不変ジョブ・manifest・成果物レジストリの実装"
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
    - "docs/tasks/core-domain-types.md"
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
    - "docs/test-specs/job-immutability-provenance.md"
  references: []
  defers: []
  deferred-by: []
---

# 不変ジョブ・manifest・成果物レジストリの実装

## Work

`al-job` に不変ジョブ定義、manifest の読み書き、成果物 id の発行・
解決を行う artifact-registry を実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/analysis-job-model.md`
- `docs/adr/immutable-resumable-job.md`
- `docs/designs/job-orchestration.md`

## Dependencies

- `docs/tasks/core-domain-types.md`

## Affected Files

- `crates/al-job/src/job.rs`
- `crates/al-job/src/manifest.rs`
- `crates/al-job/src/registry.rs`

## Done When

- [ ] ジョブ定義が不変に表現され、再実行は新しいジョブ id を発行する
- [ ] manifest が入力・設定・成果物 id・系譜を記録できる
- [ ] 成果物 id の発行・解決が registry 経由で行える

## Verification

- [ ] `mise exec -- cargo test -p al-job` が通る
- [ ] manifest だけで成果物の生成経緯を再構成できるテストが通る
