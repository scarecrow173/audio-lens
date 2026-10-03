---
id: "TASK-034z9nDhaSqoH2kao6D3Xz"
type: "task"
status: "done"
title: "analysis ステージ CLI 配線 + depends_on"
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
    - "docs/plans/analysis-stages.md"
  implemented-by: []
  depends-on:
    - "docs/plans/analysis-stages.md"
    - "docs/tasks/analysis-expensive-stage.md"
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
    - "docs/test-specs/analysis-stages.md"
  references: []
  defers: []
  deferred-by: []
---

# analysis ステージ CLI 配線 + depends_on

## Work

解析ステージを `al run` に配線する。

- `al-cli` に Cheap/Reference/ExpensiveAnalysisStage を登録
- `--stages` 選択が depends_on と整合することを確認
  (cheap-only / cheap+expensive 等)
- `USAGE` / `--help` / README 相当の使用説明を更新
- ステージ名 ↔ StageKind の整合(kebab-case CLI 名)

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/analysis-stages.md`

## Dependencies

- `docs/tasks/analysis-expensive-stage.md`

## Affected Files

- `crates/al-cli/src/main.rs`
- `crates/al-cli/Cargo.toml`

## Done When

- [ ] `al run --stages ingest,cheap_analysis` が完走する
- [ ] `--stages` に依存欠落がある場合は入口で拒否される

## Verification

- [ ] `cargo test -p al-cli`
