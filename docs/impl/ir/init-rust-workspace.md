---
id: "IMPL-034yZGjPN32llXkJsytw2G"
type: "impl"
status: "completed"
title: "init rust workspace"
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
    - "docs/tasks/init-rust-workspace.md"
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

# init rust workspace

## Summary

Rust ワークスペース(mise toolchain + al-core/al-job/al-cli 空 crate)を初期化した。

## Context

task `docs/tasks/init-rust-workspace.md` / plan `docs/plans/implement-job-orchestration-foundation.md` / ADR `docs/adr/implementation-stack.md` に従う。

## Implementation

### Changes

- `mise.toml` に rust 1.98.1 と build/test/lint/fmt task を追加
- workspace `Cargo.toml` と 3 crate の雛形を追加
- `.gitignore` に `target/` を追加(Cargo.lock は再現性のためコミット対象)

### Decisions During Implementation

- Cargo.lock はツール系プロジェクトの再現性のためコミットする方針

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
