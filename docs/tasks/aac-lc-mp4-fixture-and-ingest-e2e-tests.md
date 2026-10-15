---
id: "TASK-034Gs7Y9aM823BFJ6lNIuO"
type: "task"
status: "done"
title: "AAC-LC MP4 fixture and ingest e2e tests"
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
    - "docs/tasks/decoder-registry-audio-fallback-merge-and-stage-wiring.md"
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

# AAC-LC MP4 fixture and ingest e2e tests

## Work

ffmpeg(mise 管理下、生成時のみ使用)で小さな AAC-LC MP4 fixture を
生成し `crates/al-media/tests/fixtures/` に静的コミットする。
`DecoderRegistry`(symphonia 登録済み)または `IngestStage` 経由の
e2e テストを追加し、以下を検証する:

- AAC-LC MP4 → `decoded.f64le` 生成、`metadata.json.audio.codec ==
  "aac"`・sample_rate/channels 一致、trim 済み frames 数が期待値。
- PCM (`sowt`) のみの MP4 → attempts 1件・`fallback == false`。
- 破損 MP4 → audio なし・`fallback == false`・attempts に symphonia
  失敗記録・stage は失敗しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-symphonia-aac-lc-audio-decode-fallback-in-mp4.md`
- `docs/test-specs/symphonia-aac-lc-audio-decode-fallback.md`

## Dependencies

- `docs/tasks/decoder-registry-audio-fallback-merge-and-stage-wiring.md`

## Affected Files

- `crates/al-media/tests/fixtures/`(新規 fixture)
- `crates/al-media/tests/`(e2e テスト)

## Done When

- [ ] AAC-LC MP4 fixture がリポジトリにコミットされている
- [ ] e2e テストが `decoded.f64le`・`codec=="aac"`・frames・
      fallback/attempts 契約を検証する
- [ ] PCM-only MP4 と破損 MP4 の回帰テストがある
- [ ] `cargo test -p al-media` がパスする

## Verification

- [ ] `cargo test -p al-media`
- [ ] `cargo test --workspace`
