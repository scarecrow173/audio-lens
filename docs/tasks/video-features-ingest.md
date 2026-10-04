---
id: "TASK-034AaFzOsSmhBfpjb1kM9F"
type: "task"
status: "todo"
title: "video_features 成果物を ingest で永続化"
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
    - "docs/plans/video-av-features.md"
  implemented-by: []
  depends-on:
    - "docs/plans/video-av-features.md"
    - "docs/tasks/video-frame-decode.md"
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
    - "docs/test-specs/video-av-features.md"
  references: []
  defers: []
  deferred-by: []
---

# video_features 成果物を ingest で永続化

## Work

算出した映像特徴を IngestStage の成果物 `video_features` として永続化する。フレーム時刻は `StreamTiming`/`TimingObservation` と同一ストリームクロックで記録する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/video-av-features.md`
- `docs/test-specs/video-av-features.md`

## Dependencies

- `docs/tasks/video-frame-decode.md`

## Affected Files

- `crates/al-media/src/stage.rs`
- `crates/al-media/src/metadata.rs`

## Done When

- [ ] `video_features` 成果物がフレーム時刻付きで記録される
- [ ] vids がない/圧縮の入力でも ingest が完結する

## Verification

- [ ] `cargo test -p al-media`
