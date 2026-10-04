---
id: "TASK-034AaFVy7YRTQZiKi04T2q"
type: "task"
status: "done"
title: "mp4: box パーサ + trak メタデータ"
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

# mp4: box パーサ + trak メタデータ

## Work

ISO-BMFF の box ツリーパーサを実装する。ftyp ブランド・moov/trak/tkhd/mdhd/hdlr/stbl/stsd を解釈し、トラック種別・timescale・duration・codec fourcc を記録する。未知 box はスキップ+記録、必須 box は厳格に検証する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/mp4-ingest.md`
- `docs/test-specs/mp4-ingest.md`

## Dependencies

- none

## Affected Files

- `crates/al-media/src/mp4.rs`
- `crates/al-media/src/format.rs`

## Done When

- [ ] 合成 MP4 のトラック/メタデータが記録される
- [ ] 未知 box があっても処理が続行する

## Verification

- [ ] `cargo test -p al-media`
