---
id: "IMPL-034z8jcNVY97ghCe3Apzms"
type: "impl"
status: "completed"
title: "columnar-table-store"
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
    - "docs/tasks/columnar-table-store.md"
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

# columnar-table-store

## Summary

列ごとのチャンクファイル(NULL ビットマップ + LE 固定幅 + u32 長接頭 UTF-8)の列指向表ストアを実装。行範囲・列選択読みをサポート。

## Implementation

### Changes

- `crates/al-dataset/src/table.rs`: `TableWriter` / `TableReader`、NULL ビットマップ + LE 値 + u32 長接頭 UTF-8 のチャンクファイル

### Decisions During Implementation

- 外部依存は追加せず独自バイナリフォーマットにした(ADR: implementation-stack)
- durable 書き込みは `al-job::fsutil` を再利用

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Context

task `docs/tasks/columnar-table-store.md`、plan `docs/plans/analysis-dataset.md`、spec `docs/specs/analysis-dataset.md` に従う。

## Validation

- `cargo test --workspace` 全 pass
- `cargo clippy --workspace` clean
- `cargo fmt` 適用済み

## Risks

- なし(pre-release: schema 変更時は再生成)

## Follow-ups

- dataset 取り込みは upstream 解析ステージ実装時に writer 側で行う
