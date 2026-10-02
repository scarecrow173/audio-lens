---
id: "IMPL-034yZJuBCBeGfg8BqwCRWO"
type: "impl"
status: "completed"
title: "core domain types"
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
    - "docs/tasks/core-domain-types.md"
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

# core domain types

## Summary

al-core に JobId/ArtifactId/InputId の安定 id と、InputRole/StageKind/Validity/Confidence/StageStatus のドメイン型を実装した。

## Context

task `docs/tasks/core-domain-types.md`、spec `docs/specs/analysis-job-model.md`、design `docs/designs/job-orchestration.md` に従う。

## Implementation

### Changes

- `crates/al-core/src/ids.rs`: xorshift64* ベースの base62 id 生成と prefix 検証
- `crates/al-core/src/types.rs`: 入力ロール・ステージ種別・validity・confidence・ステージ状態の serde 型
- snake_case の serde 表現で manifest 互換を確保

### Decisions During Implementation

- id 生成は外部クレートを使わず xorshift64* + シーケンス + 時刻/プロセスシードで一意性を確保(暗号強度不要のため)

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
