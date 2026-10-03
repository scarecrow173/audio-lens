---
id: "TASK-034z9oHpFZMcAeDWVmSBMF"
type: "task"
status: "done"
title: "伝達/タイムマップ/ステレオ/残差/locate ビュー"
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
    - "docs/tasks/viz-spectrogram-tiles.md"
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

# 伝達/タイムマップ/ステレオ/残差/locate ビュー

## Work

構造系ビューを生成する。

- 伝達関数ビュー: 振幅 / 位相 / 群遅延 / コヒーレンス
  (reference 特徴由来)
- locate / time-map ビュー: セグメント・現象注釈・オカレンス
- ステレオ/ch ビュー: ch 間相関・位相差・レベル差時系列
- 残差ビュー: capture / aligned reference / prediction /
  R0-R3 の残差 PSD と帯域エネルギー
- 共有タイムライン: すべてのビューが共通の時間軸
  (sample → 表示軸)で参照できるインデックス
- confidence / validity / ソース参照を全ビューで保持
- 出力: `analysis/visualization/` 配下。決定的

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/visualization-export.md`
- `docs/specs/visualization-dataset.md`

## Dependencies

- `docs/tasks/viz-spectrogram-tiles.md`

## Affected Files

- `crates/al-viz/src/views.rs`(新規)

## Done When

- [ ] 全ビューが共有タイムラインに乗る
- [ ] R0-R3 と残差 PSD/帯域エネルギーが揃う
- [ ] confidence/validity が素通しで保持される

## Verification

- [ ] `cargo test -p al-viz`
