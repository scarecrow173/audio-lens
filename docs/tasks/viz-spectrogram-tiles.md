---
id: "TASK-034z9oBYoDVIWGp8h2kY9n"
type: "task"
status: "todo"
title: "スペクトログラムタイル + 差分/帯域ヒートマップ"
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
    - "docs/plans/visualization-export.md"
  implemented-by: []
  depends-on:
    - "docs/plans/visualization-export.md"
    - "docs/tasks/viz-waveform-lod.md"
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
    - "docs/test-specs/visualization-export.md"
  references: []
  defers: []
  deferred-by: []
---

# スペクトログラムタイル + 差分/帯域ヒートマップ

## Work

スペクトログラム系の可視化データを生成する。

- スペクトログラムタイル: dataset の STFT/PSD 中間生成物から
  周波数×時間タイルを生成(再 STFT しない)
- dB ドメイン差分スペクトログラム: source/capture の対応
  PSD から差分を計算
- 帯域ヒートマップ: 固定帯域 / 1/3oct パワー時系列
- validity マスク: 無効区間を明示する対応系列
- 出力: `analysis/visualization/spectrogram/` 配下 tensor +
  メタデータ。決定的

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/visualization-export.md`
- `docs/specs/visualization-dataset.md`

## Dependencies

- `docs/tasks/viz-waveform-lod.md`

## Affected Files

- `crates/al-viz/src/spectrogram.rs`(新規)

## Done When

- [ ] 再解析なしで dataset 由来のタイルが生成される
- [ ] 差分スペクトログラムが dB ドメインである
- [ ] validity マスクが特徴行の validity と一致する

## Verification

- [ ] `cargo test -p al-viz`
