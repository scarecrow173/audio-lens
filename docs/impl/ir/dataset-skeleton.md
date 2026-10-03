---
id: "IMPL-034z8jciJUJvAAxyLYMv2o"
type: "impl"
status: "completed"
title: "dataset-skeleton"
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
    - "docs/tasks/dataset-skeleton.md"
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

# dataset-skeleton

## Summary

`al-core::Validity` を spec の 10 状態集合に置き換え、`al-dataset` クレートを新設。`FeatureRecord`(結果形状)と `DatasetManifest`(schema_version・inputs・config・roles・artifacts)を実装。

## Implementation

### Changes

- `crates/al-core/src/types.rs`: `Validity` を 10 状態に置き換え
- `crates/al-dataset/`: クレート新設(`manifest.rs` / `provenance.rs` / `types.rs`)

### Decisions During Implementation

- 外部依存は追加せず独自バイナリフォーマットにした(ADR: implementation-stack)
- durable 書き込みは `al-job::fsutil` を再利用

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Context

task `docs/tasks/dataset-skeleton.md`、plan `docs/plans/analysis-dataset.md`、spec `docs/specs/analysis-dataset.md` に従う。

## Validation

- `cargo test --workspace` 全 pass
- `cargo clippy --workspace` clean
- `cargo fmt` 適用済み

## Risks

- なし(pre-release: schema 変更時は再生成)

## Follow-ups

- dataset 取り込みは upstream 解析ステージ実装時に writer 側で行う
