---
id: "IMPL-034zkPdjQ6GHTg2DAo8BcH"
type: "impl"
status: "completed"
title: "viz-waveform-lod"
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
    - "docs/tasks/viz-waveform-lod.md"
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

# viz-waveform-lod

## Summary

入力音声ストアから min/max/RMS の LOD ピラミッドを生成する
`lod.rs` を実装した。基底バケット 256 サンプル × 5 段
(256/512/1024/2048/4096)で、各段は `read_audio_range` による
チャンク読み出しで構築する。

## Context

task `docs/tasks/viz-waveform-lod.md`、plan
`docs/plans/visualization-export.md`、共有タイムライン仕様
`docs/specs/visualization.md` に従う。

## Implementation

### Changes

- `crates/al-viz/src/lod.rs`(新規): `Bucket` /
  `Pyramid` / `build(reader, input_id)` /
  `products(input_id, pyramid)`
- `crates/al-viz/src/timeline.rs`(新規): `SeriesDecl` /
  `Timeline` / `build(reader, capture_id, config)`。
  全系列の window/hop・LOD の samples_per_bucket を宣言する
  共有タイムラインを生成

### Decisions During Implementation

- 各 LOD 段は下位段の集約ではなく原波形から独立に集計する。
  部分範囲の再集計が壊れにくく、実装も単純
- バケット値は `(min, max, rms)` の 3 列 tensor
  (`[buckets, 3]`)として visualization ロールに保存

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Validation

- `cargo test -p al-viz` 全 pass(結合テストで LOD 出力の
  決定性を確認)

## Risks

- 入力が非常に短い場合は上位段が空 tensor になり得る。
  書き込み側(`write_tensor`)は空データを skip する

## Follow-ups

- なし
