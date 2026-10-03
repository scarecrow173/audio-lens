---
id: "IMPL-034zkPeCQtfiabQEaY78Fq"
type: "impl"
status: "completed"
title: "viz-events-summary"
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
    - "docs/tasks/viz-events-summary.md"
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

# viz-events-summary

## Summary

skipped_products の記録と決定的 `summary.json` を生成する
`summary.rs` を実装した。入力一覧・produced ロール・
イベント/alignment 件数・確度/validity tally・生成製品一覧を
1 つの JSON にまとめる。

## Context

task `docs/tasks/viz-events-summary.md`、plan
`docs/plans/visualization-export.md` に従う。

## Implementation

### Changes

- `crates/al-viz/src/summary.rs`(新規): `Skipped` 型と
  `summary_body(reader, roles_source, produced, skipped)`
- `crates/al-viz/src/stage.rs`: 各 product 群の生成時に
  required role が未 produced の場合は理由つきで skipped に記録し
  `visualization/skipped.json` に保存

### Decisions During Implementation

- `produced_roles` は読み出しスナップショットではなく
  **書き込み中のマニフェスト**(`writer.manifest()`)から取る。
  読みスナップショットは当該実行の visualization ロールを
  含まないため、初回と再実行で summary.json の内容が変わる
  非決定性があった(回帰テストで検出・修正)
- ロール一覧はソート済み、製品一覧は生成順(決定的)で
  serialize する

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Validation

- `cargo test -p al-viz --test viz`:
  `visualization_rerun_replaces_products_deterministically` で
  2 回実行の全出力バイト一致を確認

## Risks

- なし

## Follow-ups

- なし
