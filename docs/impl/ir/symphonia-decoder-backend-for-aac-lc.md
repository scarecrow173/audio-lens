---
id: "IMPL-034Gsq0ChrN0sWj8RidtUS"
type: "impl"
status: "completed"
title: "symphonia decoder backend for AAC-LC"
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
    - "docs/tasks/symphoniadecoder-backend-for-aac-lc-in-mp4.md"
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

# symphonia decoder backend for AAC-LC

## Summary

`crates/al-media/src/symphonia.rs` に `SymphoniaDecoder` を実装し、
`al-media` に `symphonia 0.5.5`(`aac`+`isomp4` のみ)を追加した。
MP4 検出→最初の decode 可能な AAC track 選択→packet decode →
interleaved f64 `DecodedAudio{codec:"aac", Float32, bits 32}`。

## Context

- Task: `docs/tasks/symphoniadecoder-backend-for-aac-lc-in-mp4.md`
- Design: `docs/designs/audio-fallback-merge-native-container-authority-symphonia-aac-decode.md`

## Implementation

### Changes

- `crates/al-media/Cargo.toml`: `symphonia = { version = "0.5.5",
  default-features = false, features = ["aac", "isomp4"] }`。
- `crates/al-media/src/symphonia.rs`(新規): `MediaDecoder` 実装。
  `detect` は `detect_format` で `MediaFormat::Mp4` のみ。
  `probe` は最小構成(track 列挙のみ)。`decode` は isomp4
  demuxer → `CODEC_TYPE_AAC` の最初の decoder 生成可能 track を
  選択 → `next_packet` ループで interleaved f64 に変換。
  `FormatOptions::enable_gapless = true` を明示。
- `crates/al-media/src/lib.rs`: `pub mod symphonia` 追加。

### Decisions During Implementation

- `Packet::trim()` メソッドは 0.5.5 に存在しない。gapless は
  demuxer が `trim_start`/`trim_end` フィールドを埋め、decoder が
  自動適用する仕組み。実測では isomp4 が elst を trim に反映
  しない(全 packet で trim 0)ため、AAC priming/padding 除去は
  merge 側(`edit_media_time_ticks` + `duration_ticks`)が担う。
- `track.id` は tkhd ではなく trak 順 index(0.5.5 実測)。
  `StreamResult.stream` にこの値を記録し、merge 側で native
  `track_id`/`track_index` に逆引きする。
- decode エラーは packet 単位で skip して続行(`skipped_packets`
  を settings に記録)、0 frames なら `audio: None` + Unavailable。
  container 自体の demux 失敗のみ `Err`。
- `version()` は Cargo.toml の pin に対応する `"0.5.5"` 固定値。

## Related Experiments

### Adopted

### Rejected

## Validation

- `cargo build -p al-media` — OK
- `cargo test -p al-media` — OK(既存含め全パス)
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- ffmpeg 8.1.2(mise)生成 fixture で decode 確認: 48kHz stereo
  AAC-LC → 13312 フレーム decode(elst trim 前)。

## Risks

- symphonia API 変更は pin 0.5.5 で隔離。
- 非 AAC-LC(SBR/PS)は decoder 生成失敗 → `audio: None` 契約。

## Follow-ups

- registry merge + trim 適用(次 task)。
- fixture e2e テスト(次 task)。
