---
id: "TASK-034Gs7XEgQOjkzbXJwYUU5"
type: "task"
status: "todo"
title: "decoder registry audio fallback merge and stage wiring"
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
    - "docs/plans/implement-symphonia-aac-lc-audio-decode-fallback-in-mp4.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-symphonia-aac-lc-audio-decode-fallback-in-mp4.md"
    - "docs/tasks/symphoniadecoder-backend-for-aac-lc-in-mp4.md"
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
    - "docs/test-specs/symphonia-aac-lc-audio-decode-fallback.md"
  references: []
  defers: []
  deferred-by: []
---

# decoder registry audio fallback merge and stage wiring

## Work

`DecoderRegistry::decode` に audio fallback 合成を追加する。
resolved backend の `decode` 後、`request.audio &&
result.audio.is_none()` なら残り backend を `detect` 順に試行し、
`audio` のみ merge する。merge 時は `result.mp4.facts.tracks` で
fallback が報告した `track_id` を `track_index` に逆引きし、該当
audio `StreamResult` を `decoded: true, unavailable: None` に更新、
`provenance.fallback = true` + `attempts` 追記。逆引き不能または
fallback 失敗では attempts に失敗を記録して native の状態を維持。
`IngestStage` の registry 構成に `SymphoniaDecoder` を `register` する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-symphonia-aac-lc-audio-decode-fallback-in-mp4.md`
- `docs/designs/audio-fallback-merge-native-container-authority-symphonia-aac-decode.md`

## Dependencies

- `docs/tasks/symphoniadecoder-backend-for-aac-lc-in-mp4.md`

## Affected Files

- `crates/al-media/src/decoder.rs`
- `crates/al-media/src/stage.rs`

## Done When

- [ ] native が音声を生成しない場合のみ fallback が試行される
      (PCM MP4/WAV/AVI では symphonia が試行されない)
- [ ] merge 成功時に `fallback == true`・attempts 2件・
      audio `StreamResult.decoded == true` が記録される
- [ ] merge 失敗時に `fallback == false`・attempts に失敗理由が
      記録され、native の Unavailable/integrity が維持される
- [ ] `cargo test -p al-media` がパスする

## Verification

- [ ] `cargo test -p al-media`
- [ ] `cargo clippy -p al-media --all-targets -- -D warnings`
