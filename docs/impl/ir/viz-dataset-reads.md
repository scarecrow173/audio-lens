---
id: "IMPL-034zkPcVFjg8rllQ3Pmwvl"
type: "impl"
status: "completed"
title: "viz-dataset-reads"
created: "2026-10-03"
updated: "2026-10-03"
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
    - "docs/tasks/viz-dataset-reads.md"
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

# viz-dataset-reads

## Summary

`al-viz` クレートを新設し、可視化が保存済み dataset だけを読む
ための範囲/チャンク読み出し基盤を追加した。`al-media` の
WAV レイアウト走査を共有化して範囲読みを実装し、
`DatasetReader` に `read_audio_range` / `audio_frames` を
追加した。

## Context

task `docs/tasks/viz-dataset-reads.md`、plan
`docs/plans/visualization-export.md` に従う。可視化は
decode や再解析を行わず dataset の保存物のみを読む規約の
足場。

## Implementation

### Changes

- `Cargo.toml`: workspace members に `crates/al-viz` を追加
- `crates/al-viz/`: 新規クレート(lib + モジュール骨格)。
  共有出力型 `TensorProduct` / `JsonProduct` を `lib.rs` に定義
- `crates/al-media/src/wav.rs`: RIFF チャンク走査を共有
  レイアウト関数に分離し、`read_range`(frame 範囲指定の
  部分 decode)を追加
- `crates/al-dataset/src/reader.rs`:
  `read_audio_range(role, name, start, frames)` と
  `audio_frames(role, name)` を追加。Audio store の WAV を
  全体 decode せず必要区間だけ読む

### Decisions During Implementation

- 範囲読みはサンプルフォーマット非依存にせず、dataset が
  物化する PCM WAV(16-bit LE)のレイアウトに限定。新フォーマットが
  増えた時点で一般化する(YAGNI)
- `TensorReader` 側の範囲読みは既存の chunk 読み出しで足りる
  ため新設せず、必要なタイル分だけ読む呼び出し側の責務とした

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Validation

- `cargo test -p al-media -p al-dataset`: 範囲読みの単体テストを
  含め全 pass
- `cargo test --workspace` 全 pass
- `cargo clippy --workspace --all-targets -- -D warnings` clean

## Risks

- Audio store 以外の store kind で `read_audio_range` を呼ぶと
  明示エラー。呼び出し側は artifact の store kind で絞る必要がある
  (stage 側で対応済み)

## Follow-ups

- なし
