---
id: "IMPL-034z8jflmxl3ZTDBOlZmIQ"
type: "impl"
status: "completed"
title: "dataset-stage"
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
    - "docs/tasks/dataset-stage.md"
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

# dataset-stage

## Summary

`DatasetStage` を実装し `al-cli` に接続。`<job_dir>/analysis/` を生成し、ストアルートごとに ArtifactRecord を登録。resume は完了を維持。

## Implementation

### Changes

- `crates/al-dataset/src/stage.rs`: `DatasetStage`(`depends_on=[Ingest]`)
- `crates/al-cli/`: `Dataset` 配線 + USAGE 更新

### Decisions During Implementation

- 外部依存は追加せず独自バイナリフォーマットにした(ADR: implementation-stack)
- durable 書き込みは `al-job::fsutil` を再利用

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Context

task `docs/tasks/dataset-stage.md`、plan `docs/plans/analysis-dataset.md`、spec `docs/specs/analysis-dataset.md` に従う。

## Validation

- `cargo test --workspace` 全 pass
- `cargo clippy --workspace` clean
- `cargo fmt` 適用済み

## Risks

- なし(pre-release: schema 変更時は再生成)

## Follow-ups

- dataset 取り込みは upstream 解析ステージ実装時に writer 側で行う
