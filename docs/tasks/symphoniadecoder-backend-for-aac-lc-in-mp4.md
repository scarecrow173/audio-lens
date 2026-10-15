---
id: "TASK-034Gs7XaxwczdCjzEW33CL"
type: "task"
status: "todo"
title: "SymphoniaDecoder backend for AAC-LC in MP4"
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

# SymphoniaDecoder backend for AAC-LC in MP4

## Work

`crates/al-media/src/symphonia.rs` に `MediaDecoder` trait 適合の
`SymphoniaDecoder` を実装する。`detect` は `detect_format` で
`MediaFormat::Mp4` のみ。`decode` は isomp4 demuxer で最初の
decode 可能な AAC track を選択し、`Packet::trim()`(gapless で
priming/delay/padding を除去)→ interleaved `f64` の
`DecodedAudio{codec: "aac", sample_format: Float32,
bits_per_sample: 32}` を生成する。選択 track の `track.id` を
`StreamResult.stream` に記録し、first_pts と trim 適用量を
`provenance.settings["symphonia"]` に残す。

依存追加: `symphonia = { version = "0.5.5", default-features =
false, features = ["aac", "isomp4"] }`。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-symphonia-aac-lc-audio-decode-fallback-in-mp4.md`
- `docs/designs/audio-fallback-merge-native-container-authority-symphonia-aac-decode.md`

## Dependencies

- なし(plan 承認済み)

## Affected Files

- `crates/al-media/Cargo.toml`
- `crates/al-media/src/symphonia.rs`(新規)
- `crates/al-media/src/lib.rs`

## Done When

- [ ] `SymphoniaDecoder` が `MediaDecoder` trait に適合する
- [ ] AAC-LC MP4 バイト列から `DecodedAudio` が生成され、
      priming/delay が trim 済みで `codec == "aac"` になる
- [ ] decode 不能(track なし/codec 非対応/破損)は
      `audio: None` + stream 単位 `Unavailable` で返り fatal にならない
- [ ] `cargo test -p al-media` のユニットテストがパスする

## Verification

- [ ] `cargo test -p al-media`
- [ ] `cargo clippy -p al-media --all-targets -- -D warnings`
