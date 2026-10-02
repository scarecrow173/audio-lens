---
id: "IMPL-034z8jdgoIcLVsYkAPgbrn"
type: "impl"
status: "completed"
title: "chunked-tensor-store"
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
    - "docs/tasks/chunked-tensor-store.md"
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

# chunked-tensor-store

## Summary

array.json + 生 LE f64 チャンク格子の N 次元テンソルストアを実装。領域読みは重なりチャンクのみ走査し全配列を展開しない。

## Implementation

### Changes

- `crates/al-dataset/src/tensor.rs`: `TensorWriter` / `TensorReader`、row-major LE f64 チャンク格子

### Decisions During Implementation

- 外部依存は追加せず独自バイナリフォーマットにした(ADR: implementation-stack)
- durable 書き込みは `al-job::fsutil` を再利用

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Context

task `docs/tasks/chunked-tensor-store.md`、plan `docs/plans/analysis-dataset.md`、spec `docs/specs/analysis-dataset.md` に従う。

## Validation

- `cargo test --workspace` 全 pass
- `cargo clippy --workspace` clean
- `cargo fmt` 適用済み

## Risks

- なし(pre-release: schema 変更時は再生成)

## Follow-ups

- dataset 取り込みは upstream 解析ステージ実装時に writer 側で行う
