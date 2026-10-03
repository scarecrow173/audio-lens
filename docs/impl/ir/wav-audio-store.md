---
id: "IMPL-034z8jdN9kIjPiFpcp0mgM"
type: "impl"
status: "completed"
title: "wav-audio-store"
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
    - "docs/tasks/wav-audio-store.md"
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

# wav-audio-store

## Summary

`al-media` に `WavEncoding`/`WavWriter`(ストリーミング符号化・RIFF サイズ後付け・奇数 pad)を追加し、`al-dataset::audio` で tmp+fsync+rename の durable 書き込みに接続。

## Implementation

### Changes

- `crates/al-media/src/wav.rs`: `WavEncoding` / `WavWriter`
- `crates/al-job/src/fsutil.rs`: `rename_durable` / `tmp_path_for`
- `crates/al-dataset/src/audio.rs`: durable WAV 書き込み/読み込み

### Decisions During Implementation

- 外部依存は追加せず独自バイナリフォーマットにした(ADR: implementation-stack)
- durable 書き込みは `al-job::fsutil` を再利用

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Context

task `docs/tasks/wav-audio-store.md`、plan `docs/plans/analysis-dataset.md`、spec `docs/specs/analysis-dataset.md` に従う。

## Validation

- `cargo test --workspace` 全 pass
- `cargo clippy --workspace` clean
- `cargo fmt` 適用済み

## Risks

- なし(pre-release: schema 変更時は再生成)

## Follow-ups

- dataset 取り込みは upstream 解析ステージ実装時に writer 側で行う
