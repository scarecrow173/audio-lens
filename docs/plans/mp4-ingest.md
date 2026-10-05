---
id: "PLAN-034AaBsJ00thIonv8bp54N"
type: "plan"
status: "approved"
title: "MP4 demux とタイミング観測の実装"
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
    - "docs/specs/media-ingest.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/media-ingest.md"
    - "docs/designs/mp4-ingest.md"
    - "docs/adr/mp4-demux-decode.md"
    - "docs/discovery/container-coverage-gaps.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/mp4-ingest.md"
  references: []
  defers: []
  deferred-by: []
---

# MP4 demux とタイミング観測の実装 Implementation Plan

## Goal

ISO-BMFF/MP4 の demux・トラックメタデータ・サンプルタイミング
観測を `al-media` に実装し、コーデック decode は明示失敗とする。

## Scope

1 つの実装ストリーム。ADR `docs/adr/mp4-demux-decode.md` の
範囲(demux + タイミング観測、非圧縮 PCM のみ取り出し)に限定。

## Upstream Documents

- `docs/specs/media-ingest.md`
- `docs/adr/mp4-demux-decode.md`
- `docs/discovery/container-coverage-gaps.md`

## Design Inputs

- `docs/designs/mp4-ingest.md`(approved)

## File Map

- `crates/al-media/src/mp4.rs` - box パーサ・trak メタデータ・
  サンプルテーブル解釈
- `crates/al-media/src/format.rs` - ftyp シグネチャ検出追加
- `crates/al-media/src/stage.rs` - MP4 入力の ingest 配線
- `crates/al-media/src/lib.rs` - mp4 モジュール公開
- `crates/al-media/tests/` - 合成 MP4 fixture 検証
- fixture 用 MP4 writer(テスト側コード)

## Implementation Sequence

- [ ] `mp4-box-parser`: box ツリー・ftyp/moov/trak メタデータ
- [ ] `mp4-sample-tables`: stts/ctts/stss/stsz/stsc/stco による
  サンプル時刻列 → `TimingObservation`
- [ ] `mp4-pcm-extract`: PCM サンプルエントリの取り出し +
  圧縮コーデック明示失敗
- [ ] `mp4-ingest-stage`: format 検出・stage 配線・成果物記録
- [ ] `mp4-fixtures-tests`: 合成 MP4 writer + 既知ケース検証

## Dependencies and Risks

- ISO-BMFF box の多様性 → 未対応 box はスキップ+記録、
  必須 box のみ厳格解釈
- stts/ctts の端ケース → 合成 fixture で既知系列を先に固定

## Task Breakdown

- `mp4-box-parser` → `mp4-sample-tables` / `mp4-pcm-extract`
  (並列可) → `mp4-ingest-stage` → `mp4-fixtures-tests`

## Verification

- [ ] `cargo test -p al-media`
- [ ] 合成 MP4 でトラック・タイミングが記録される
- [ ] 圧縮コーデック decode 要求が明示失敗になる

## Open Questions / Gaps

- fMP4(moof)は検出して明示失敗とする。edit list は観測記録のみ

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: subagent-capable implementation
- Independent work streams: `mp4-sample-tables` と
  `mp4-pcm-extract` は `mp4-box-parser` 後に並列化可能
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- PCM 取り出しを「demux 範囲」とみなす境界解釈、
  `TimingObservation` への載せ方をレビュー対象とする
