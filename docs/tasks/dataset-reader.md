---
id: "TASK-034z8896wbdQ95hRyjJY0y"
type: "task"
status: "done"
title: "dataset-reader(manifest 経由参照)"
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
    - "docs/tasks/dataset-writer-provenance.md"
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

# dataset-reader(manifest 経由参照)

## Work

`dataset-reader` を実装する。dataset manifest を唯一の入口にし、
ロール別成果物の列挙、表の行範囲/カラム選択読み出し、テンソルの
範囲/チャンク読み出し、音声の読み出し、provenance 参照を提供する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/analysis-dataset.md`
- `docs/test-specs/dataset-provenance-validity.md`

## Dependencies

- `docs/tasks/dataset-writer-provenance.md`

## Affected Files

- `crates/al-dataset/src/reader.rs`

## Done When

- [ ] manifest からロール別成果物と生成経緯を再構成できる
- [ ] 表・テンソル・音声を部分読み出しできる
- [ ] 存在しない成果物・壊れた manifest が明示的失敗になる

## Verification

- [ ] `cargo test -p al-dataset` が通る
