---
id: "TASK-034z889WFC0CLXorGLm3bG"
type: "task"
status: "done"
title: "dataset 結合テスト"
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
    - "docs/tasks/dataset-stage.md"
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
    - "docs/test-specs/dataset-store-roundtrip.md"
    - "docs/test-specs/dataset-provenance-validity.md"
  references: []
  defers: []
  deferred-by: []
---

# dataset 結合テスト

## Work

合成データによる dataset 結合テストを実装する: 大テーブルの
行範囲読み出し、チャンク境界をまたぐテンソル範囲読み出し、
WAV 往復、provenance の manifest 再構成、validity の区別、
`not_produced` ロール、宣言入力の参照のみ記録、`run → resume`
での dataset 物化。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/analysis-dataset.md`
- `docs/test-specs/dataset-store-roundtrip.md`
- `docs/test-specs/dataset-provenance-validity.md`

## Dependencies

- `docs/tasks/dataset-stage.md`

## Affected Files

- `crates/al-dataset/tests/`

## Done When

- [ ] test-spec の Examples をすべてカバーする
- [ ] run → resume → dataset 物化の検証が通る
- [ ] 部分読み出し・来歴・validity の規範が確認される

## Verification

- [ ] `cargo test --workspace` が通る
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` が通る
