---
id: "TASK-036bZxCvBnMqWeRtYuIo"
type: "task"
status: "todo"
title: "既存 WAV/AVI/MP4 を MediaDecoder native バックエンドへ移行(#36 フェーズ1)"
created: "2026-10-09"
updated: "2026-10-09"
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
    - "docs/plans/media-format-analysis-extensions.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/media-decoder-adr.md"
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
    - "docs/test-specs/media-format-analysis-extensions.md"
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/36"
  defers: []
  deferred-by: []
---

# 既存 WAV/AVI/MP4 を MediaDecoder native バックエンドへ移行(#36 フェーズ1)

## Work

ADR で決定した `MediaDecoder` トレイトに既存実装を移し、
呼び出し側をレジストリ経由に統一する。

- `wav.rs` / `avi.rs` / `mp4.rs` のデコードパスを
  `MediaDecoder` 実装(native backend)として再編
- `detect` / `stage` の分岐を拡張子ベースの direct dispatch から
  レジストリ照合へ変更
- provenance に decoder/backend 種別を記録
- コーデック追加(AAC/FLAC/MP3/H.264 デコード等)はこのタスクの
  範囲外 — 後続計画に分離

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/media-format-analysis-extensions.md`
- フェーズ0 で作成する ADR

## Dependencies

- `docs/tasks/media-decoder-adr.md`

## Affected Files

- `crates/al-media/src/`(decoder.rs 新規想定、wav/avi/mp4 再編)

## Done When

- [ ] 既存の WAV/AVI/MP4 ingest テストが変わらずパス
- [ ] decode 経路がレジストリ経由になる
- [ ] provenance に decoder/backend が記録される

## Verification

- [ ] `cargo test -p al-media`
