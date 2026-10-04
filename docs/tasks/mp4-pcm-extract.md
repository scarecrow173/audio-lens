---
id: "TASK-034AaFWPLKohMACaTcKkDv"
type: "task"
status: "todo"
title: "mp4: PCM エントリ取り出し + 圧縮コーデック明示失敗"
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

# mp4: PCM エントリ取り出し + 圧縮コーデック明示失敗

## Work

非圧縮 PCM サンプルエントリ(sowt/twos/lpcm 等)を mdat から取り出す。圧縮コーデックは `decode/input unavailable` で明示失敗する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/mp4-ingest.md`
- `docs/test-specs/mp4-ingest.md`

## Dependencies

- `docs/tasks/mp4-box-parser.md`

## Affected Files

- `crates/al-media/src/mp4.rs`
- `crates/al-media/src/metadata.rs`

## Done When

- [ ] PCM エントリが decoded PCM として取り出せる
- [ ] 圧縮コーデックの decode 要求が明示失敗

## Verification

- [ ] `cargo test -p al-media`
