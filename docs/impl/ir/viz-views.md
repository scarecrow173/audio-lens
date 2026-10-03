---
id: "IMPL-034zkPe9Tu7zeC8JiaQUrV"
type: "impl"
status: "completed"
title: "viz-views"
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
    - "docs/tasks/viz-views.md"
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

# viz-views

## Summary

dataset の tensor/JSON から構造化ビュー JSON を生成する
`views.rs` を実装した。伝達関数・locate・ステレオ・残差・
イベントマーカの各ビューを dataset 相対参照つきで出力する。

## Context

task `docs/tasks/viz-views.md`、plan
`docs/plans/visualization-export.md`、spec
`docs/specs/visualization.md` に従う。

## Implementation

### Changes

- `crates/al-viz/src/views.rs`(新規):
  `transfer_views` / `locate_views` / `stereo_views` /
  `residual_views` / `events_view`。
  イベントビューは events テーブルを行読みして
  kind/時刻範囲/チャンネル/確度を marker 配列に整形

### Decisions During Implementation

- ビューは「読み出し + 整形」に限定し、数値再計算は行わない
  (dataset 由来のみ)
- イベントビューは events ロールが produced でもテーブルが
  無い場合に `None` を返し、呼び出し側で product を作らない
- `TableReader::read_range` は列優先(`cols[col][row]`)を返す。
  初期実装が行として扱いフィールドがずれていたため、サンプル
  データでの動作確認を経て列名(`schema()`)でフィールドを
  解決する実装に修正した

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Validation

- `cargo test -p al-viz` 全 pass

## Risks

- locate ロールの JSON スキーマ変更時はビュー側も追随が必要
  (pre-release なので互換層は作らず同時変更する)

## Follow-ups

- なし
