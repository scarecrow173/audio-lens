---
id: "IMPL-034z8DeMcrHrl7aJu1XypN"
type: "impl"
status: "completed"
title: "locate-tracking"
created: "2026-10-02"
updated: "2026-10-04"
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
    - "docs/tasks/locate-tracking.md"
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

# locate-tracking

## Summary

`Anchor` / `Segment` / `TimeMap` とループ反復識別を実装。
`SegmentKind`(Continuous/Drift/Jitter/Skip/Gap/Resume/Unmatched)で
ドリフト・局所ジッタ・スキップ・ギャップ・再開・未マッチを区間表現し、
`identify_loop_occurrences` が各出現を loop_index / 出現位置 /
ソース位相・時間 / キャプチャ時間で個別識別する。

## Implementation

### Changes

- `crates/al-locate/src/tracking.rs`: 型 + `build_time_map` +
  `identify_loop_occurrences` + `capture_to_source` 補間

### Decisions During Implementation

- `build_time_map` は gap/rewind で区間を分割し、run 内のオフレート
  窓でさらに分割。単位速度から外れた run は Drift、`min_run` 未満の
  オフレート run は Jitter。単一 anchor の区間は傾きを観測できない
  ため Continuous とする(レビュー修正: 以前は無条件に Jitter だった)
- `identify_loop_occurrences` は第 3 引数の `anchor`(受理位置)を
  パス 0 / 位相 0 の基準とし、`loop_index` は anchor 起点の符号付き
  通し番号(anchor より前のパスは負)。`occurrence_position` は
  検出位置をそのまま保持する(レビュー修正: 以前は capture 原点の
  グリッドに吸着させ、実際の検出位置を失っていた)
- ループ出現は fuzzy match にまとめず出現ごとにレコード化

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Context

task `docs/tasks/locate-tracking.md`、spec
`docs/specs/locate-tracking.md`。commit `1a742a6`。

## Validation

- `cargo test -p al-locate` 全 pass(ループ個別識別 / 連続区間の
  線形写像 / drift+gap の区間化)

## Risks

- anchor 生成は現状メソッド候補位置ベース。長尺ソースの密な
  anchor 抽出(窓分割定位)は今後の精度改善余地

## Follow-ups

- なし
