---
id: "TASK-034z9oHPHHVPCNKwoMJp5c"
type: "task"
status: "todo"
title: "イベントマーカ + 決定的サマリ + skipped.json"
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
    - "docs/plans/visualization-export.md"
  implemented-by: []
  depends-on:
    - "docs/plans/visualization-export.md"
    - "docs/tasks/viz-views.md"
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

# イベントマーカ + 決定的サマリ + skipped.json

## Work

イベントマーカと決定的サマリを生成する。

- イベントマーカ: events ロールの候補イベントを
  stable ID つきでタイムラインへ配置(ビューから相互参照可能)
- サマリ `summary.json`: 候補数・受理/棄却の alignment 数・
  生成された製品一覧・confidence/validity 集計。
  すべてのパスは dataset 相対(絶対パス禁止)、決定的
- `visualization/skipped.json`: 上流ロール欠如等で生成を
  スキップした製品と理由を明示
- 期待される role が存在するのに製品が無い場合はエラー

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/visualization-export.md`
- `docs/specs/visualization-dataset.md`
- `docs/specs/candidate-event-model.md`

## Dependencies

- `docs/tasks/viz-views.md`

## Affected Files

- `crates/al-viz/src/summary.rs`(新規)

## Done When

- [ ] サマリに絶対パスが含まれず、再実行で一致する
- [ ] スキップ製品が理由つきで記録される
- [ ] イベントが stable ID で参照できる

## Verification

- [ ] `cargo test -p al-viz`
