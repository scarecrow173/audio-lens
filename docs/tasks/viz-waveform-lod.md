---
id: "TASK-034z9oByaJjYPsnfEEUy8o"
type: "task"
status: "done"
title: "波形 LOD ピラミッド(min/max/RMS)"
created: "2026-10-02"
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
    - "docs/plans/visualization-export.md"
  implemented-by: []
  depends-on:
    - "docs/plans/visualization-export.md"
    - "docs/tasks/viz-dataset-reads.md"
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

# 波形 LOD ピラミッド(min/max/RMS)

## Work

波形 LOD ピラミッドを生成する。

- 各チャンネルについて複数レベルの LOD を構築
  (レベルごとに min / max / RMS の 3 系列)
- 基底は dataset の audio store から範囲読み出しで構築
  (生 PCM 全体を読まない)
- 出力: `analysis/visualization/waveform/` 配下に
  レベル別 tensor + メタデータ(レベル→サンプル数倍率)
- 決定的(同一 dataset → バイト一致)

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/visualization-export.md`
- `docs/specs/visualization-dataset.md`

## Dependencies

- `docs/tasks/viz-dataset-reads.md`

## Affected Files

- `crates/al-viz/src/waveform.rs`(新規)

## Done When

- [ ] min/max/RMS の 3 系列が全 LOD レベルで生成される
- [ ] 出力が決定的である

## Verification

- [ ] `cargo test -p al-viz`
