---
id: "TASK-034z889vFy9QDdh0EG6Nj4"
type: "task"
status: "done"
title: "DatasetStage 実装と CLI 接続"
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
    - "docs/tasks/dataset-reader.md"
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
---

# DatasetStage 実装と CLI 接続

## Work

`DatasetStage`(Stage trait 実装)を実装する。`depends_on()` は
`[StageKind::Ingest]`(デフォルトの先行全ステージにしない)。
job manifest の既存成果物(declared inputs + ingest)を読み、
`<job_dir>/analysis/` に dataset を物化する。1 ArtifactRecord =
1 ストアルートで登録する。`al-cli` の `Dataset` スタブを実装に
差し替え、USAGE を更新する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/analysis-dataset.md`
- `docs/designs/dataset-schema.md`

## Dependencies

- `docs/tasks/dataset-reader.md`

## Affected Files

- `crates/al-dataset/src/stage.rs`
- `crates/al-cli/src/main.rs`
- `crates/al-cli/Cargo.toml`
- `Cargo.toml`

## Done When

- [ ] `al run --stages ingest,dataset` で `analysis/` が物化される
- [ ] `depends_on` が `[Ingest]` で部分 plan が受理される
- [ ] 成果物が job manifest にストアルート単位で記録される
- [ ] resume で Completed が維持される

## Verification

- [ ] `cargo test --workspace` が通る
- [ ] CLI で run → resume → inspect が動作する
