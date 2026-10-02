---
id: "TASK-034z8821L9FbGf5lvsQjiM"
type: "task"
status: "todo"
title: "チャンクテンソルストア"
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

# チャンクテンソルストア

## Work

配列ごとのディレクトリ(`array.json`: shape/dtype/chunk_shape/order
+ チャンク座標ファイル raw LE)のチャンクテンソルストアを実装する。
書き込み時はチャンク単位で分割保存し、読み出しは範囲指定で該当
チャンクのみを読んで返す(配列全体を展開しない)。chunk_shape は
~4 MiB/チャンク目安で算出・config 上書き可。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/analysis-dataset.md`
- `docs/test-specs/dataset-store-roundtrip.md`

## Dependencies

- `docs/tasks/dataset-skeleton.md`

## Affected Files

- `crates/al-dataset/src/tensor.rs`

## Done When

- [ ] N 次元テンソルをチャンク分割して保存できる
- [ ] 範囲読み出しがチャンク境界をまたいでも正しい値を返す
- [ ] 部分読み出しが配列全体を読まずに済む
- [ ] 書き込みが durable である

## Verification

- [ ] `cargo test -p al-dataset` が通る
