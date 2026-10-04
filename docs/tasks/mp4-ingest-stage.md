---
id: "TASK-034AaFXpJRZWcxVPOfKCdh"
type: "task"
status: "done"
title: "mp4: ingest stage 配線 + 成果物記録"
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
    - "docs/tasks/mp4-sample-tables.md"
    - "docs/tasks/mp4-pcm-extract.md"
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

# mp4: ingest stage 配線 + 成果物記録

## Work

format 検出(ftyp 由来のみ)・IngestStage への MP4 配線・メタデータ/タイミング成果物の記録を実装する。部分 ingest 契約に従い、全トラック圧縮でも ingest は metadata/timing を記録して完了し decoded_pcm を省略する。永続化タイミングのフィールド名はコンテナ中立名に一般化する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/mp4-ingest.md`
- `docs/test-specs/mp4-ingest.md`

## Dependencies

- `docs/tasks/mp4-sample-tables.md`
- `docs/tasks/mp4-pcm-extract.md`

## Affected Files

- `crates/al-media/src/stage.rs`
- `crates/al-media/src/format.rs`
- `crates/al-media/src/lib.rs`

## Done When

- [ ] MP4 入力で metadata/timing 成果物が記録される
- [ ] 全トラック圧縮の MP4 でも ingest が失敗せず decode/input unavailable が integrity 観測として記録される
- [ ] resume でも一貫して動作する

## Verification

- [ ] `cargo test -p al-media`
- [ ] `cargo test -p al-cli --test cli`
