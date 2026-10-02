---
id: "TASK-034z0J6W0kR1ncPEkWbQgf"
type: "task"
status: "done"
title: "タイムスタンプ源 trait と間隔・ドリフト・不連続観測"
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
    - "docs/plans/implement-ingest-av.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-ingest-av.md"
    - "docs/tasks/media-skeleton.md"
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
    - "docs/test-specs/av-timing-observation.md"
  references: []
  defers: []
  deferred-by: []
---

# タイムスタンプ源 trait と間隔・ドリフト・不連続観測

## Work

タイムスタンプ源 trait(`TimestampSource`)と、PTS 系列から
フレーム間隔・ドリフト・不連続・重複/ドロップ候補を観測する
ロジックを実装する。CFR / 公称 FPS を仮定しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-ingest-av.md`
- `docs/specs/video-av-timing.md`
- `docs/test-specs/av-timing-observation.md`

## Dependencies

- `docs/tasks/media-skeleton.md`

## Affected Files

- `crates/al-media/src/timing.rs`

## Done When

- [ ] 合成 VFR 系列から間隔系列・実測分布・不連続候補が観測される
- [ ] 等間隔系列でドリフト・不連続が観測されない
- [ ] 音声 / 映像のクロックが別系列として保持される

## Verification

- [ ] `cargo test -p al-media` が通る
