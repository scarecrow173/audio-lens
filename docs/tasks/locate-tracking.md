---
id: "TASK-034z8kIgVL2y1o3HNiBmfD"
type: "task"
status: "done"
title: "tracking: anchor/segment/TimeMap + ループ反復識別"
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
    - "docs/plans/locate-and-tracking.md"
  implemented-by: []
  depends-on:
    - "docs/plans/locate-and-tracking.md"
    - "docs/tasks/locate-consensus.md"
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
    - "docs/test-specs/locate-tracking-residual.md"
  references: []
  defers: []
  deferred-by: []
---

# tracking: anchor/segment/TimeMap + ループ反復識別

## Work

`Anchor`/`Segment`/`TimeMap` とループ反復識別(2〜3 連結反復探索)を実装。出現ごとにループ index・出現位置・ソース位相/時間・キャプチャ時間を保存し、ドリフト・局所ジッタ・スキップ・ギャップ・再開・未マッチを区分的に表現する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/locate-and-tracking.md`
- `docs/specs/locate-tracking.md`

## Dependencies

- `docs/tasks/locate-consensus.md`

## Affected Files

- `crates/al-locate/src/tracking.rs`

## Done When

- [ ] ループ出現が個別識別される
- [ ] time map が spec の全現象を表現できる
- [ ] 長尺入力でグローバルオフセットではなく区分的 time map が生成される

## Verification

- [ ] ドリフト/スキップ+再開 fixture で区分表現が検証される
- [ ] `cargo test -p al-locate` パス
