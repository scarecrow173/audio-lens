---
id: "PLAN-034AaBrlpxZEKpyEGpUmmk"
type: "plan"
status: "completed"
title: "映像フレーム特徴と A/V 同期の実装"
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
    - "docs/specs/video-av-timing.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/video-av-timing.md"
    - "docs/designs/video-features.md"
    - "docs/adr/avi.md"
    - "docs/discovery/video-av-feature-gaps.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/video-av-features.md"
  references: []
  defers: []
  deferred-by: []
---

# 映像フレーム特徴と A/V 同期の実装 Implementation Plan

## Goal

非圧縮 AVI のフレームデコードによる映像特徴系列と、音声↔映像
イベントの汎用照合(avsync)を実装する。

## Scope

1 つの実装ストリーム。`al-media` の映像フレーム/特徴生成と
`al-analysis` の avsync 照合を含む。圧縮コーデックは
ADR `docs/adr/avi.md` どおり対象外。

## Upstream Documents

- `docs/specs/video-av-timing.md`
- `docs/adr/avi.md`
- `docs/discovery/video-av-feature-gaps.md`

## Design Inputs

- `docs/designs/video-features.md`(approved)

## File Map

- `crates/al-media/src/video.rs` - DIB/rawvideo フレームの走査・
  輝度/差分/シーンチェンジ算出
- `crates/al-media/src/avi.rs` - vids チャンクを video デコードへ接続
- `crates/al-media/src/stage.rs` - `video_features` 成果物の永続化
- `crates/al-media/src/lib.rs` - video モジュール公開
- `crates/al-analysis/src/avsync.rs` - 音声イベント↔映像イベント照合
- `crates/al-analysis/src/stage.rs` - cheap stage への avsync 配線
- `crates/al-media/tests/ingest.rs` / 新規テスト - 合成 AVI 検証
- `crates/al-analysis/tests/` - avsync 既知オフセット検証

## Implementation Sequence

- [ ] `video-frame-decode`: DIB/rawvideo フレームデコード +
  輝度/差分/シーンチェンジ系列
- [ ] `video-features-ingest`: `video_features` を
  `TimingObservation` と同一クロックで永続化
- [ ] `avsync-analysis`: 音声オンセット×映像イベントの照合・
  差分/確度/分布/ドリフト記録
- [ ] `video-av-integration-tests`: 合成 AVI fixture で E2E 検証

## Dependencies and Risks

- 非圧縮 AVI フレーム形式の想定違い → 合成 writer で既知
  フレームを検証
- `al-media` の ingest 成果物を `al-analysis` が読む経路は
  既存の stage 成果物参照パターンに従う

## Task Breakdown

- `video-frame-decode` → `video-features-ingest` →
  `avsync-analysis` → `video-av-integration-tests`

## Verification

- [ ] `cargo test -p al-media`
- [ ] `cargo test -p al-analysis`
- [ ] シーンチェンジを含む合成 AVI で特徴系列が記録される
- [ ] 既知 A/V オフセットが差分として回復される

## Open Questions / Gaps

- 24bpp 以外の rawvideo 表現(16bpp 等)は fixture が必要になった
  時点で拡張する

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: subagent-capable implementation
- Independent work streams: `video-frame-decode` 先行、
  `avsync-analysis` は `video-features-ingest` の成果物形状確定後
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- `al-media` でフレームを保持せず特徴のみ永続化する点、
  avsync を cheap stage 内に置く点をレビュー対象とする
