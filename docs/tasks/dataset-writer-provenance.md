---
id: "TASK-034z888EftCjwJOLuS09K0"
type: "task"
status: "in-progress"
title: "provenance レコードと dataset-writer"
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
    - "docs/plans/analysis-dataset.md"
  implemented-by: []
  depends-on:
    - "docs/plans/analysis-dataset.md"
    - "docs/tasks/columnar-table-store.md"
    - "docs/tasks/chunked-tensor-store.md"
    - "docs/tasks/wav-audio-store.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/dataset-provenance-validity.md"
  references: []
  defers: []
  deferred-by: []
---

# provenance レコードと dataset-writer

## Work

provenance レコード(input id・algorithm id/version・params・
alignment id・親/中間物系譜)と、`dataset-writer`(成果物を
ロール別ストアへ振り分け、dataset manifest を durable に保存)
を実装する。宣言入力は source_path + source_sha256 の参照のみ
記録し、ingest の decoded PCM は `analysis/inputs/` に WAV で
物化する。上流未実装ロールは `not_produced` で記録する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/analysis-dataset.md`
- `docs/specs/analysis-dataset.md`
- `docs/test-specs/dataset-provenance-validity.md`

## Dependencies

- `docs/tasks/columnar-table-store.md`
- `docs/tasks/chunked-tensor-store.md`
- `docs/tasks/wav-audio-store.md`

## Affected Files

- `crates/al-dataset/src/provenance.rs`
- `crates/al-dataset/src/writer.rs`
- `crates/al-dataset/src/manifest.rs`

## Done When

- [ ] provenance が成果物ごとに記録され manifest から辿れる
- [ ] ロール別ストアへの振り分けと manifest 集約ができる
- [ ] 宣言入力が参照のみで記録される(複製しない)
- [ ] 空ロールが `not_produced` と明示される

## Verification

- [ ] `cargo test -p al-dataset` が通る
