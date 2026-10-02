---
id: "TASK-034z881eeUpQjjpRLhM0io"
type: "task"
status: "done"
title: "al-core Validity 置き換えと al-dataset 骨格・結果形状・manifest"
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

# al-core Validity 置き換えと al-dataset 骨格・結果形状・manifest

## Work

`al-core::Validity` を spec の 10 区分で置き換える
(既存 `Unavailable`/`Degraded` は廃止、単体テスト更新)。
`crates/al-dataset` を workspace に追加し、結果形状
(`FeatureRecord`: spec の全フィールド)・dataset manifest 型
(schema_version / inputs / config / role 別成果物・生成経緯・
role ごとの `produced`/`not_produced` ステータス)を実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/analysis-dataset.md`
- `docs/specs/analysis-dataset.md`
- `docs/test-specs/dataset-provenance-validity.md`

## Dependencies

- なし(先行タスクなし)

## Affected Files

- `Cargo.toml`
- `crates/al-core/src/types.rs`
- `crates/al-dataset/Cargo.toml`
- `crates/al-dataset/src/lib.rs`
- `crates/al-dataset/src/types.rs`
- `crates/al-dataset/src/manifest.rs`

## Done When

- [ ] Validity が spec の 10 区分で定義され旧 variants がない
- [ ] FeatureRecord が spec 結果形状の全フィールドを持つ
- [ ] dataset manifest が role 別ステータス・schema_version を持つ
- [ ] `al-dataset` が workspace としてビルドされる

## Verification

- [ ] `cargo test -p al-core -p al-dataset` が通る
