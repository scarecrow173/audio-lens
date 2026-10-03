---
id: "IMPL-034zkPdMoUyZdzPym1TQ4F"
type: "impl"
status: "completed"
title: "viz-spectrogram-tiles"
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
    - "docs/tasks/viz-spectrogram-tiles.md"
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

# viz-spectrogram-tiles

## Summary

保存済み中間スペクトル tensor から表示域タイルを生成する
`tiles.rs` を実装した。スペクトログラム・帯域ヒートマップ・
差分/コヒーレンス/残差タイルを tensor として出力し、
cheap 特徴の validity からマスクも生成する。

## Context

task `docs/tasks/viz-spectrogram-tiles.md`、plan
`docs/plans/visualization-export.md`、spec
`docs/specs/visualization.md` に従う。

## Implementation

### Changes

- `crates/al-viz/src/tiles.rs`(新規): `build(reader)` で
  intermediate ロールの tensor を走査して表示域タイルを生成、
  `validity_masks(reader, hop_s)` で features/cheap の
  validity を時系列マスク化

### Decisions During Implementation

- タイル化は保存 tensor の再サンプリングを行わず、そのままの
  時間×周波数格子を visualization ロールへ複製する設計。
  UI 側のズームは LOD ピラミッドと同じ考え方で別途行う
- dB 変換等の表示域マッピングはビルド時に行い、値域を
  タイルの params に残す

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Validation

- `cargo test -p al-viz` 全 pass

## Risks

- intermediate ロールに未知の tensor 名が増えた場合、タイル化の
  対象外になる(表示は落ちないがタイルが出ない)

## Follow-ups

- なし
