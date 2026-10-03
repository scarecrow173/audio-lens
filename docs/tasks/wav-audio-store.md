---
id: "TASK-034z882rSUvcx6IerdcaPD"
type: "task"
status: "done"
title: "WAV エンコーダと音声ストア"
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
    - "docs/plans/analysis-dataset.md"
  implemented-by: []
  depends-on:
    - "docs/plans/analysis-dataset.md"
    - "docs/tasks/dataset-skeleton.md"
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
    - "docs/test-specs/dataset-store-roundtrip.md"
  references: []
  defers: []
  deferred-by: []
---

# WAV エンコーダと音声ストア

## Work

`al-media::wav` に WAV エンコーダを追加する(f64 PCM →
PCM s16/s24/s32/float)。全バッファを一度に作らず、サイズ未確定の
RIFF ヘッダ → サンプル逐次書き出し → サイズのシークパッチ →
fsync + rename で durable に書く。`al-dataset::audio` から
dataset 音声ロール(decoded/aligned/residual)として利用する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/analysis-dataset.md`
- `docs/test-specs/dataset-store-roundtrip.md`

## Dependencies

- `docs/tasks/dataset-skeleton.md`

## Affected Files

- `crates/al-media/src/wav.rs`
- `crates/al-dataset/src/audio.rs`

## Done When

- [ ] f64 PCM を WAV(PCM)へ書き出せる
- [ ] 書き出した WAV が既存デコーダで往復する
- [ ] 大入力でも全バッファを一度に確保しない
- [ ] 書き込みが durable である

## Verification

- [ ] `cargo test -p al-media -p al-dataset` が通る
