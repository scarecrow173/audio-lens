---
id: "TASK-034AaFWfHy1QUJblugAdk5"
type: "task"
status: "todo"
title: "mp4: stts/ctts/stss/stsz/stsc/stco サンプル時刻列"
created: "2026-10-04"
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
    - "docs/plans/mp4-ingest.md"
  implemented-by: []
  depends-on:
    - "docs/plans/mp4-ingest.md"
    - "docs/tasks/mp4-box-parser.md"
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
    - "docs/test-specs/mp4-ingest.md"
  references: []
  defers: []
  deferred-by: []
---

# mp4: stts/ctts/stss/stsz/stsc/stco サンプル時刻列

## Work

stts/ctts/stss/stsz/stsc/stco(co64)からサンプル時刻列を構築する。`observe_timing` には decode order の DTS 系列(単調)を適用し、ctts 由来 PTS 系列は別ラベルで記録する。B フレーム再並びを含む fixture で偽 discontinuity が出ないことを検証する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/mp4-ingest.md`
- `docs/test-specs/mp4-ingest.md`

## Dependencies

- `docs/tasks/mp4-box-parser.md`

## Affected Files

- `crates/al-media/src/mp4.rs`
- `crates/al-media/src/timing.rs`

## Done When

- [ ] 既知 stts/ctts の時刻列が回復される
- [ ] VFR 的な時刻列が nominal FPS 演算なしで観測される
- [ ] ctts で再並びする fixture で DTS 系列に偽 discontinuity が記録されない

## Verification

- [ ] `cargo test -p al-media`
