---
id: "TASK-034AaFzOsSmhBfpjb1kM9F"
type: "task"
status: "done"
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

算出した映像特徴を IngestStage の成果物 `video_features` として永続化する。イベント種別(scene_change/luminance_step/motion_onset)を列挙して記録する。フレーム時刻は `StreamTiming`/`TimingObservation` と同一ストリームクロックで記録する。**部分 ingest 契約を導入する**: デコード失敗を `StageError` にせず、metadata/timing/video_features は decode 可否と独立に記録し PCM のみ `Option` 化する。1 入力の失敗が他入力を巻き込まない。

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

- [ ] `video_features` 成果物がフレーム時刻+イベント種別付きで記録される
- [ ] vids がない/圧縮の入力でも ingest が完結する
- [ ] auds が欠損・デコード不可でも映像観測とメタデータが記録される(decoded_pcm のみ省略)
- [ ] コンテナ中立のタイミング命名(stream_timestamps + 由来ラベル)に一般化される

## Verification

- [ ] `cargo test -p al-media`
