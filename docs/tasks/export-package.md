---
id: "TASK-034z9oICJD1cJoadJNGUhu"
type: "task"
status: "done"
title: "ExportStage: 再配置可能パッケージ"
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
    - "docs/tasks/viz-stage-cli.md"
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

# ExportStage: 再配置可能パッケージ

## Work

`ExportStage`: 再配置可能な解析パッケージを作る。

- パッケージ内容: `analysis/`(dataset + visualization)の
  コピー + パッケージ manifest
- 大きな原メディアは hash + ファイル名参照のみ(明示設定時のみ
  コピー)。原メディアが無くてもパッケージを読める
- 絶対パス・ソース場所をパッケージ manifest/設定から除去
- package schema version + generator metadata(ツール名・
  バージョン)を明記。スキーマ不一致は明示的失敗
- lineage / hashes / validity / provenance をパッケージ内に保持
- `depends_on = [Dataset, Visualization]`

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/visualization-export.md`
- `docs/specs/analysis-export.md`

## Dependencies

- `docs/tasks/viz-stage-cli.md`

## Affected Files

- `crates/al-viz/src/export.rs`(新規)
- `crates/al-cli/src/main.rs`

## Done When

- [ ] パッケージを別位置へ移しても読める
- [ ] 原メディアなしで開ける
- [ ] manifest に絶対パスが無い

## Verification

- [ ] `cargo test -p al-viz`
