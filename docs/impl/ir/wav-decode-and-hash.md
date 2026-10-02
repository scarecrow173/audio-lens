---
id: "IMPL-034z0Uhoo3y9fXz1OCXqAl"
type: "impl"
status: "completed"
title: "wav decode and hash"
created: "2026-10-02"
updated: "2026-10-02"
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
    - "docs/tasks/wav-decode.md"
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

# wav decode and hash

## Summary

RIFF/WAV デマックス + PCM/IEEE-float デコード(正規化前 PCM)と
自前 SHA-256 を実装した。

## Context

task `docs/tasks/wav-decode.md`、test-spec `ingest-wav-decode` に従う。

## Implementation

### Changes

- `wav.rs`: fmt/data チャンク走査、PCM u8/s16/s24/s32 と
  float32/64 を f64 へ変換、奇数チャンクの pad byte 対応、
  破損・非対応形式の明示的失敗
- `hash.rs`: 依存ゼロの SHA-256(既知ベクトルで検証)
- `metadata.rs` の `AudioMetadata` / `SampleFormat` を利用

### Decisions During Implementation

- PCM は f64 に変換して保持(ゲイン正規化は行わないという意味での
  「正規化前」を保持)。元のビット深度・フォーマットは別フィールドに記録
- 24bit の符号拡張を明示処理

## Validation

- `cargo test -p al-media`: 9 tests pass
- `cargo clippy -p al-media --all-targets -- -D warnings`: clean
- `cargo fmt --check`: clean

## Follow-ups

- 実コンテナ(MKV/MP4)の対応は別プラン
