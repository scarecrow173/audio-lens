---
id: "IMPL-034z8jeiDvJJ5osxk1UhA2"
type: "impl"
status: "completed"
title: "dataset-writer-provenance"
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
    - "docs/tasks/dataset-writer-provenance.md"
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

# dataset-writer-provenance

## Summary

`Provenance`(input_id・algorithm_id/version・params・alignment_id・parents)と `DatasetWriter`(role ごとのストア振り分け + manifest 登録)を実装。宣言入力はパス+SHA-256 参照のみ。

## Implementation

### Changes

- `crates/al-dataset/src/provenance.rs`: `Provenance` / `InputReference`
- `crates/al-dataset/src/writer.rs`: `DatasetWriter`

### Decisions During Implementation

- 外部依存は追加せず独自バイナリフォーマットにした(ADR: implementation-stack)
- durable 書き込みは `al-job::fsutil` を再利用

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Context

task `docs/tasks/dataset-writer-provenance.md`、plan `docs/plans/analysis-dataset.md`、spec `docs/specs/analysis-dataset.md` に従う。

## Validation

- `cargo test --workspace` 全 pass
- `cargo clippy --workspace` clean
- `cargo fmt` 適用済み

## Risks

- なし(pre-release: schema 変更時は再生成)

## Follow-ups

- dataset 取り込みは upstream 解析ステージ実装時に writer 側で行う
