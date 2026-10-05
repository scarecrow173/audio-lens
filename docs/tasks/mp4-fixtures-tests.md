---
id: "TASK-034AaFY7vtgsBOWZeYX4ez"
type: "task"
status: "todo"
title: "mp4: 合成 fixture writer + 既知ケース検証"
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
    - "docs/tasks/mp4-ingest-stage.md"
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

# mp4: 合成 fixture writer + 既知ケース検証

## Work

テスト用の合成 MP4 writer(最小 box セット)を実装し、既知タイミング・コーデック構成での検証を行う。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/mp4-ingest.md`
- `docs/test-specs/mp4-ingest.md`

## Dependencies

- `docs/tasks/mp4-ingest-stage.md`

## Affected Files

- `crates/al-media/tests/`

## Done When

- [ ] test-spec の各 Example が対応テストを持つ
- [ ] 合成 MP4 のタイミングが回復される

## Verification

- [ ] `cargo test -p al-media`
