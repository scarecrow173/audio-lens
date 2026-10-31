---
id: "TASK-034P3P0cd1tJUcKKBnEB7b"
type: "task"
status: "todo"
title: "旧可視化廃止と可搬 export 分離(AL#106)"
created: "2026-10-30"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/106"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/ruviform-foundation-migration.md"
  implemented-by: []
  depends-on:
    - "docs/plans/ruviform-foundation-migration.md"
    - "docs/tasks/ruviform-job-dataset-regression.md"
    - "docs/tasks/ruviform-gui-app.md"
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

# 旧可視化廃止と可搬 export 分離(AL#106)

## Work

`al-viz` の表示専用部(`VisualizationStage`・LOD/tiles/
timeline/views/summary・静的 `report.html`・Visualization
Dataset・viz golden)を廃止し、可搬な解析データ export
(旧 `al-viz::package`/`ExportStage`)を `al-dataset` 側へ分離
する。Issue
[#106](https://github.com/scarecrow173/audio-lens/issues/106) を正本とする。

- 新 GUI(#105)が稼働・検証済みであることを前提に撤去する
- `StageKind::Visualization`・pipeline 順序・`--stages` 表記・
  CLI コマンド・ドキュメントを整合させる
- `StageKind::Export` の enum variant は維持し、実装を
  `al_viz::stage::ExportStage` から al-dataset 実装へ再配線する
  (責務マップ「境界判定記録」の決定。`al run --stages export`
  の意味と manifest/provenance の stage 記録は維持し、新
  `al export` コマンドは同じ headless service 経由で同実装を
  呼ぶ薄い入口とする)
- 可搬 export は `al-dataset` 側の専用モジュールへ再配置し、
  `al-cli` に export コマンドを設ける(静的 HTML 添付は削除)
- 撤去前に比較用の参照資料(旧静的出力・golden)を保存

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/ruviform-foundation-migration.md`
- `docs/designs/ruviform-migration-responsibility-map.md` §6/§7
- Issue [#106](https://github.com/scarecrow173/audio-lens/issues/106)

## Dependencies

- `docs/tasks/ruviform-job-dataset-regression.md`
- `docs/tasks/ruviform-gui-app.md`(検証済みであること)
- 外部依存: Ruviform [#156](https://github.com/scarecrow173/ruviform/issues/156)
  /[#157](https://github.com/scarecrow173/ruviform/issues/157)
  の利用側実績

## Affected Files

- `crates/al-viz`(表示専用部の撤去・`package` の export 機能分離)
- `crates/al-dataset`(可搬 export モジュールの受入)
- `crates/al-core`(`StageKind::Visualization` の撤去。
  `StageKind::Export` variant は維持)
- `crates/al-cli`(export コマンド・`--stages` 表記の整合)

## Done When

- [ ] 旧 viz 表示専用部が撤去され、可搬 export が GUI/静的
      report なしで独立動作する
- [ ] StageKind・CLI・ドキュメントが整合する
- [ ] #106 の完了条件を満たす

## Verification

- [ ] export 成果物の portability/security 検証
- [ ] `mise run test` 相当が通る
