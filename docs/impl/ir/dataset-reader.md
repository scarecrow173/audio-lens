---
id: "IMPL-034z8jePughZgKVzHu5zP4"
type: "impl"
status: "completed"
title: "dataset-reader"
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
    - "docs/tasks/dataset-reader.md"
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

# dataset-reader

## Summary

`DatasetReader` を実装。manifest が唯一の入口で、ロール列挙・表範囲/列読み・テンソル領域・音声・JSON・provenance アクセスを提供。schema 不一致は明示失敗。

## Implementation

### Changes

- `crates/al-dataset/src/reader.rs`: `DatasetReader`

### Decisions During Implementation

- 外部依存は追加せず独自バイナリフォーマットにした(ADR: implementation-stack)
- durable 書き込みは `al-job::fsutil` を再利用

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Context

task `docs/tasks/dataset-reader.md`、plan `docs/plans/analysis-dataset.md`、spec `docs/specs/analysis-dataset.md` に従う。

## Validation

- `cargo test --workspace` 全 pass
- `cargo clippy --workspace` clean
- `cargo fmt` 適用済み

## Risks

- なし(pre-release: schema 変更時は再生成)

## Follow-ups

- dataset 取り込みは upstream 解析ステージ実装時に writer 側で行う
