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
  float32/64 を f64 へ変換(整数 PCM は整数値のまま保持し
  [-1,1] スケーリングしない。u8 は offset binary を符号値に
  変換)、奇数チャンクの pad byte 対応、末尾不完全フレーム・
  破損・非対応形式の明示的失敗
- `hash.rs`: 依存ゼロの SHA-256(既知ベクトルで検証)
- `metadata.rs` の `AudioMetadata` / `SampleFormat` を利用

### Decisions During Implementation

- PCM はソースのネイティブスケールで f64 に保持(整数 PCM は
  整数値のまま、f64 の精度内で無損失)。[-1,1] へのスケーリングは
  正規化に該当するため行わない。元のビット深度・フォーマットは
  `sample_format` に記録
- 8bit PCM は unsigned offset binary のため `raw - 128` の符号値に
  変換(u8 の wrapping 演算で波形が反転する不具合を修正)
- data チャンクがフレーム境界で終わらない場合は `malformed` で
  明示的失敗(暗黙切り捨てしない)
- 24bit の符号拡張を明示処理

## Validation

- `cargo test -p al-media`: 9 tests pass
- `cargo clippy -p al-media --all-targets -- -D warnings`: clean
- `cargo fmt --check`: clean

## Follow-ups

- 実コンテナ(MKV/MP4)の対応は別プラン
