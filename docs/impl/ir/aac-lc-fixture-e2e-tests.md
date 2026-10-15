---
id: "IMPL-034Gsq2E1CKaZxLNn5X9qQ"
type: "impl"
status: "completed"
title: "AAC-LC fixture e2e tests"
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
    - "docs/tasks/aac-lc-mp4-fixture-and-ingest-e2e-tests.md"
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

# AAC-LC fixture e2e tests

## Summary

ffmpeg 8.1.2(mise)で AAC-LC MP4 fixture を3件生成し、
`crates/al-media/tests/fixtures/` に静的コミットした。
`DecoderRegistry`(symphonia 登録済み)と `IngestStage` の e2e
テストを `crates/al-media/tests/aac.rs` に追加した。

## Context

- Task: `docs/tasks/aac-lc-mp4-fixture-and-ingest-e2e-tests.md`
- Test spec: `docs/test-specs/symphonia-aac-lc-audio-decode-fallback.md`

## Implementation

### Changes

- `crates/al-media/tests/fixtures/`(新規):
  - `aac_lc_48k_stereo.mp4` — 48kHz stereo AAC-LC 0.25s。
  - `aac_lc_48k_mono.mp4` — 48kHz mono AAC-LC 0.25s。
  - `aac_lc_two_tracks.mp4` — 48kHz + 44.1kHz の2音声 track。
- `crates/al-media/tests/aac.rs`(新規): 6テスト。
  - AAC-LC stereo → `decoded.f64le` 生成、`codec=="aac"`、
    `sample_rate==48000`・`channels==2`・`frames==12000`(trim 済み)。
  - AAC-LC mono → `codec=="aac"`・`channels==1`・`frames==12000`。
  - 2音声 track → 最初の decode 可能 track(48kHz mono)を選択、
    他 track は `unavailable`。
  - PCM MP4(`sowt`)→ attempts 1件・`fallback == false`。
  - decode 不能 MP4(`mp4a` だが AAC データなし)→ attempts 2件・
    `fallback == false`・audio None・stage 非失敗。
  - `IngestStage` e2e → `metadata.json.audio.codec=="aac"`・
    `decoder.fallback==true`・`decoder.attempts` が native+
    symphonia の2件・`decoded_pcm` artifact 生成・mp4 facts 保持。

### Decisions During Implementation

- fixture は runtime 依存なしで静的コミット — 生成は ffmpeg のみ
  必要でテスト実行には不要。
- 破損 MP4 テストは `mp4a` sample entry を持つが AAC payload が
  ない構造を採用 — native probe が成功する範囲で fallback が
  「試行されるが失敗する」経路を検証するため。

## Related Experiments

### Adopted

### Rejected

## Validation

- `cargo test -p al-media --test aac` — 6/6 パス
- `cargo test -p al-media` — 全既存テスト含めパス
- `cargo test --workspace` — 全パス
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo fmt --all -- --check` — パス

## Risks

- fixture は実 AAC-LC ビットストリームを含むため、symphonia 内部の
  decode 結果に依存する(bit-exact はスコープ外 — test spec の
  Non-goals 参照)。

## Follow-ups

- #36 PR 作成。
