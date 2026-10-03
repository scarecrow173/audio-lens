---
id: "IMPL-034z0Pt9YaZUrkmw1QS5qn"
type: "impl"
status: "completed"
title: "al-media skeleton"
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
    - "docs/tasks/media-skeleton.md"
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

# al-media skeleton

## Summary

`crates/al-media` クレートをワークスペースに追加し、magic bytes ベースの
フォーマット検出と `decode/input unavailable` の明示的失敗を実装した。

## Context

task `docs/tasks/media-skeleton.md`、plan `implement-ingest-av` に従う。

## Implementation

### Changes

- `Cargo.toml`: `al-media` を member と workspace 依存に追加
- `format.rs`: RIFF/WAVE シグネチャ検出(`detect_format`)、未知形式は
  `None`(呼び出し側が明示的失敗に変換)
- `metadata.rs`: `MediaError`(InputUnavailable / Malformed / Io)、
  `AudioMetadata` / `SampleFormat` / `IntegrityObservation` の型定義

### Decisions During Implementation

- 拡張子ではなく magic bytes のみで判定し、誤検出を防ぐ
- エラーは InputUnavailable / Malformed / Io の3種に集約し、
  「失敗したが何か分からない」状態を作らない

## Validation

- `cargo test -p al-media`: 3 tests pass

## Follow-ups

- WAV デコーダ / timing / dictionary / IngestStage は後続タスクで実装

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Risks

- なし
