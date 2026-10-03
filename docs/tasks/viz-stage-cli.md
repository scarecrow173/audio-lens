---
id: "TASK-034z9oIf37uS3sViutmBaX"
type: "task"
status: "done"
title: "VisualizationStage + CLI 配線"
created: "2026-10-02"
updated: "2026-10-03"
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
    - "docs/plans/visualization-export.md"
  implemented-by: []
  depends-on:
    - "docs/plans/visualization-export.md"
    - "docs/tasks/viz-events-summary.md"
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
    - "docs/test-specs/visualization-export.md"
  references: []
  defers: []
  deferred-by: []
---

# VisualizationStage + CLI 配線

## Work

`VisualizationStage` を `al run` に配線する。

- `depends_on = [Dataset]`(plan 規約どおり明示)
- 出力先は `analysis/visualization/`(dataset の visualization
  ロール)
- 再実行時は visualization 出力のみ置換(他ロール不変)
- CLI 登録と `--stages …,dataset,visualization` の通し確認
- 使用説明更新

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/visualization-export.md`

## Dependencies

- `docs/tasks/viz-events-summary.md`

## Affected Files

- `crates/al-viz/src/stage.rs`(新規)
- `crates/al-cli/src/main.rs`

## Done When

- [ ] dataset 済みジョブで visualization のみ再生成できる
- [ ] 他ロールの成果物が不変である

## Verification

- [ ] `cargo test -p al-viz -p al-cli`
