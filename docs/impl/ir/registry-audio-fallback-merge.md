---
id: "IMPL-034Gsq1D0B9oXYkMM4W8pP"
type: "impl"
status: "completed"
title: "registry audio fallback merge"
created: "2026-10-15"
updated: "2026-10-15"
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
    - "docs/tasks/decoder-registry-audio-fallback-merge-and-stage-wiring.md"
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

# registry audio fallback merge

## Summary

`DecoderRegistry::decode` に audio fallback 合成を実装し、
`IngestStage` の registry に `SymphoniaDecoder` を登録した。
MP4 の `Mp4Track` に `edit_media_time_ticks` を追加し、merge 時に
AAC encoder delay(priming)を除去する。

## Context

- Task: `docs/tasks/decoder-registry-audio-fallback-merge-and-stage-wiring.md`
- Design: `docs/designs/audio-fallback-merge-native-container-authority-symphonia-aac-decode.md`

## Implementation

### Changes

- `crates/al-media/src/decoder.rs`: `DecoderRegistry::decode` に
  fallback 分岐を追加。`request.audio && result.audio.is_none()`
  のとき、resolved backend 以外の登録 backend を detect 順に試行。
  採用は `audio` のみ、merge 時に native `Mp4Track` の
  `edit_media_time_ticks`(media timescale → sample_rate 換算)で
  priming を除去し `duration_ticks - media_time` に truncate。
- `crates/al-media/src/mp4.rs`: `Mp4Track` に
  `edit_media_time_ticks: Option<u64>` を追加し `elst` の最初の
  非空 entry の `media_time` を記録(`parse_elst_media_time`)。
- `crates/al-media/src/stage.rs`: `DecoderRegistry::native()` +
  `register(SymphoniaDecoder)` に変更。

### Decisions During Implementation

- fallback backend の track ID → native track の対応は
  `track_id`(tkhd)優先、なければ `track_index`(moov 順 index、
  symphonia の `track.id` と一致)にフォールバック。いずれも
  `handler == "soun"` 限定。対応不能なら merge 棄却。
- AAC encoder delay は native の `edit_media_time_ticks` で
  merge 時に除去。symphonia/isomp4 は elst を packet trim に
  反映しないため、`packet.trim()` ではなく merge 側が担当する
  (design doc に反映済み)。
- fallback の失敗は `attempts` に `outcome: "unavailable"` または
  `"error"` + `reason` で記録し、`fallback` は立てない。
  stage は失敗しない(native の部分 ingest 契約維持)。

## Related Experiments

### Adopted

### Rejected

## Validation

- `cargo test -p al-media` — OK
- `cargo clippy -p al-media --all-targets -- -D warnings` — clean
- AAC-LC MP4 fixture で `fallback == true`・attempts 2件・
  audio `StreamResult.decoded == true` を確認。
- PCM MP4(`sowt`)は attempts 1件・`fallback == false` を維持。

## Risks

- `track.id` ↔ `track_index` の対応は symphonia の trak 順に依存。
  tkhd 優先 + index fallback で安定させた。

## Follow-ups

- fixture e2e テスト(次 task)。
