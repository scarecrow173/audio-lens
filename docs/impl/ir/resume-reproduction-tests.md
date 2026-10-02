---
id: "IMPL-034yZPZx2PZbqcLEJtvlsz"
type: "impl"
status: "completed"
title: "resume reproduction tests"
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
    - "docs/tasks/resume-reproduction-tests.md"
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

# resume reproduction tests

## Summary

スタブステージを用いた結合テスト 5 件を crates/al-job/tests/resume.rs に実装した。

## Context

task `docs/tasks/resume-reproduction-tests.md`、test-spec `stage-runner-resume` / `job-immutability-provenance` に従う。

## Implementation

### Changes

- 部分失敗→resume で完了済み成果物再利用と未完了のみ再実行
- 全成功後 resume が no-op であること
- 再実行で新ジョブ id・別 manifest が作られること
- 依存未充足計画の拒否と lineage 再構成

### Decisions During Implementation

- スタブに実行カウンタを持たせ副作用の有無を直接観測する方式を採用

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
