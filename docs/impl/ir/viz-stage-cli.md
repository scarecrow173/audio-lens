---
id: "IMPL-034zkPf7WpyoiSq0zuRkCS"
type: "impl"
status: "completed"
title: "viz-stage-cli"
created: "2026-10-03"
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
    - "docs/tasks/viz-stage-cli.md"
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

# viz-stage-cli

## Summary

`VisualizationStage` / `ExportStage` を実装し、`al run` に
配線した。produced ロールのストア契約検証・置換セマンティクス・
スキップ記録を備え、USAGE のステージ一覧も実装済み表記に更新。

## Context

task `docs/tasks/viz-stage-cli.md`、plan
`docs/plans/visualization-export.md` に従う。

## Implementation

### Changes

- `crates/al-viz/src/stage.rs`(新規):
  - `VisualizationStage`: `depends_on = [Dataset]`。
    produced ロールに期待する store kind が無い場合は明示エラー
    (例: `locate` が produced だが JSON artifact を持たない)。
    timeline → LOD → tiles → validity masks → views →
    skipped.json → summary.json の順に visualization ロールへ
    書き込み、再実行は同ロール内で置換
  - `ExportStage`: `depends_on = [Dataset, Visualization]`。
    `package::build` + golden スナップショットを実行し、
    manifest/golden/package の ArtifactRecord を返す
  - inputs ロールの波形 LOD は Audio store の artifact のみ対象
    (同ロールの declared-input JSON を音声として decode しない)
- `crates/al-cli/src/main.rs`: 両ステージを配線し、USAGE の
  ステージ一覧を更新
- `crates/al-cli/Cargo.toml`: `al-viz` 依存を追加

### Decisions During Implementation

- ロール/store 契約は「produced なのに期待 store が 0 件」の
  みをエラーにし、余剰 artifact の存在は許容する。
  未 produced ロールはエラーではなく skipped 記録
- export は dataset とは別物(job 直下の `export/`)なので
  DatasetWriter は使わず `package::build` が直接ファイルを書く

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Validation

- `cargo test -p al-viz --test viz`: 4 件全 pass
  (`produced_role_without_expected_store_fails_explicitly` を含む)
- `cargo test -p al-cli --test cli`:
  `full_pipeline_reaches_visualization_and_export` で
  `ingest,cheap_analysis,dataset,visualization,export` 完走を確認
- `cargo test --workspace` 全 pass、clippy clean

## Risks

- なし

## Follow-ups

- なし
