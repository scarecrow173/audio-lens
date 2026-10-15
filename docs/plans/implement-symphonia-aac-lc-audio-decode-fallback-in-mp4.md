---
id: "PLAN-034Gs73yLxitjnNfDd2Cz4"
type: "plan"
status: "approved"
title: "implement symphonia AAC-LC audio decode fallback in MP4"
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
    - "docs/specs/symphonia-aac-lc-audio-decode-in-mp4.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/symphonia-aac-lc-audio-decode-in-mp4.md"
    - "docs/designs/audio-fallback-merge-native-container-authority-symphonia-aac-decode.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/symphonia-aac-lc-audio-decode-fallback.md"
  references: []
  defers: []
  deferred-by: []
---

# implement symphonia AAC-LC audio decode fallback in MP4 Implementation Plan

## Goal

MP4/AAC-LC 入力で `decoded.f64le` を生成する audio fallback を
`DecoderRegistry` に導入し、native の container/timing 観測を保持した
まま `metadata.json.audio.codec == "aac"` を実現する。

## Scope

1実装ストリーム。`SymphoniaDecoder` 追加 → registry fallback 合成 →
fixture テストの順で、ステージ・manifest 契約は変更しない。

## Upstream Documents

- `docs/specs/symphonia-aac-lc-audio-decode-in-mp4.md`
- `docs/adr/audio-fallback-pipeline-native-container-facts-symphonia-decode.md`

## Design Inputs

- `docs/designs/audio-fallback-merge-native-container-authority-symphonia-aac-decode.md`

## File Map

- `crates/al-media/Cargo.toml` - symphonia 依存(`0.5.5`、`aac`+`isomp4` のみ)
- `crates/al-media/src/symphonia.rs` - `SymphoniaDecoder` 実装
- `crates/al-media/src/decoder.rs` - `DecoderRegistry::decode` の fallback 合成
- `crates/al-media/src/stage.rs` - registry へ `SymphoniaDecoder` 登録
- `crates/al-media/src/lib.rs` - module 宣言
- `crates/al-media/tests/` - AAC-LC MP4 fixture ベーステスト + fixture ファイル
- `crates/al-testkit/` - fixture 生成補助(必要な場合のみ)

## Implementation Sequence

- [ ] `symphonia` を `al-media` に追加(`default-features = false`,
      `features = ["aac", "isomp4"]`)
- [ ] `SymphoniaDecoder` を `MediaDecoder` trait に実装
      (detect=MP4 のみ、probe=最小、decode=AAC track 選択 + trim +
      interleaved f64 + `provenance.settings["symphonia"]` に
      first_pts/trim 記録)
- [ ] `DecoderRegistry::decode` に audio fallback 合成を追加
      (native 音声なし → detect 順 fallback → `audio` のみ merge、
      track_id→track_index 解決で `StreamResult` 更新)
- [ ] `IngestStage` で `DecoderRegistry::native()` +
      `register(SymphoniaDecoder)` を構成
- [ ] AAC-LC MP4 fixture を生成して静的コミット、
      decode/fallback/provenance テストを追加

## Dependencies and Risks

- symphonia 0.5.5(承認済み、7日ルール充足)。API 変更は pin で隔離。
- ffmpeg 生成 fixture は test asset としてのみ使用(runtime 依存なし)。
- fallback merge の track_id 逆引きが失敗する場合は棄却して native
  Unavailable を維持(誤 decoded 記録を避ける)。

## Task Breakdown

- **T1 backend**: 依存追加 + `SymphoniaDecoder` + unit tests
  (track 選択・trim・interleave・エラー分類)。
- **T2 registry merge**: `DecoderRegistry::decode` fallback 合成 +
  stage 配線 + provenance/attempts 更新テスト。
- **T3 fixture tests**: ffmpeg 生成 AAC-LC MP4 fixture + e2e
  (`decoded.f64le`・`codec=="aac"`・frames・PCM-only MP4 は
  fallback しない・破損 MP4 は失敗しない)。

## Verification

- [ ] `cargo test -p al-media` がパスする
- [ ] `cargo test --workspace` がパスする
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` が clean
- [ ] `cargo fmt --all -- --check` がパスする
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --type all`
      で no findings

## Open Questions / Gaps

- symphonia の `track.id` が `tkhd` track_id に一致することを
  fixture で検証する(一致しない場合は merge 棄却経路を残す)。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- symphonia API(`FormatOptions::enable_gapless`、`Packet::trim`、
  `track.id`/`track_index` 対応)を fixture で検証する。
- merge 棄却時の `StreamResult`/`attempts` 更新を重複しないよう確認。
