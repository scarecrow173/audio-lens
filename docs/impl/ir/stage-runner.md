---
id: "IMPL-034yZNZCVzaiiGljZdAb8Z"
type: "impl"
status: "completed"
title: "stage runner"
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
  implements:
    - "docs/tasks/stage-runner.md"
  implemented-by: []
  depends-on: []
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# stage runner

## Summary

Stage trait(読み取り専用 StageContext)と依存順実行・部分再開を行う StageRunner を実装した。

## Context

task `docs/tasks/stage-runner.md`、test-spec `docs/test-specs/stage-runner-resume.md` に従う。

## Implementation

### Changes

- `stage.rs`: Stage trait と StageContext(読み取り専用 manifest + registry + config)
- `runner.rs`: run/resume。ステージ遷移ごとに manifest を保存し、失敗時は Failed を記録して中断
- al-core に StageKind::PIPELINE_ORDER と dependencies() を追加

### Decisions During Implementation

- ステージは manifest を直接変更せず成果物レコードを返し、runner が登録する
- デフォルト依存は canonical pipeline order の先行ステージ全て

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

<!-- Record the checks that were run. -->

## Risks

<!-- Record remaining risks. -->

## Follow-ups

<!-- Record follow-up tasks or next actions. -->
