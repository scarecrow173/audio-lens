---
id: "TASK-034z881BJBYiNLa5ozHuiO"
type: "task"
status: "done"
title: "列指向チャンク表ストア"
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
    - "docs/tasks/dataset-skeleton.md"
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
  references: []
  defers: []
  deferred-by: []
---

# 列指向チャンク表ストア

## Work

テーブルごとのディレクトリ(`schema.json` + カラム別チャンク +
チャンクインデックス)の列指向ストアを実装する。物理エンコーディング:
固定幅数値は LE、可変長は `u32 len + payload`、NULL はチャンク先頭の
null ビットマップ。行範囲 / カラム選択で必要チャンクのみ読む
writer / reader API を提供する。デフォルト 65_536 行/チャンク
(config 上書き可)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/analysis-dataset.md`
- `docs/test-specs/dataset-store-roundtrip.md`

## Dependencies

- `docs/tasks/dataset-skeleton.md`

## Affected Files

- `crates/al-dataset/src/table.rs`

## Done When

- [ ] 数値・文字列・NULL カラムの書き込み/往復ができる
- [ ] 行範囲クエリが該当チャンクのみを読んで正しい行を返す
- [ ] カラム選択読み出しができる
- [ ] 書き込みが durable(temp + fsync + rename)である

## Verification

- [ ] `cargo test -p al-dataset` が通る
