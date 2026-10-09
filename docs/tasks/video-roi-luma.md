---
id: "TASK-036cAsDfGhJkLzQwErTy"
type: "task"
status: "todo"
title: "video: ROI 輝度追跡(#39)"
created: "2026-10-09"
updated: "2026-10-09"
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
    - "docs/plans/media-format-analysis-extensions.md"
  implemented-by: []
  depends-on:
    - "docs/plans/media-format-analysis-extensions.md"
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
    - "docs/test-specs/media-format-analysis-extensions.md"
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/39"
  defers: []
  deferred-by: []
---

# video: ROI 輝度追跡(#39)

## Work

字幕・UI 表示などの**矩形領域**の平均輝度時系列を観測できる
ようにし、A/V 同期の精度を上げる。現行 `video.rs` は全フレーム
平均輝度のみで領域指定がない。

- `VideoFeatureConfig` に `rois`(名前 + 矩形)を追加
- ROI ごとの輝度時系列を FeatureRecord/tensor として出力
- ROI 輝度ステップをイベント化して avsync の video event に
  接続(字幕表示↔セリフ音声の照合が可能になる)
- marker 取り込み(issue #31)との統合は後続でもよいが、
  ROI 名と marker の紐付けを想定したデータ構造にする

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/media-format-analysis-extensions.md`
- `docs/designs/video-features.md`
- issue #39

## Dependencies

なし(#36 のデコーダ移行とは独立)

## Affected Files

- `crates/al-analysis/src/`(video feature / avsync 周辺)
- `crates/al-media/src/video.rs`(ROI 抽出)

## Done When

- [ ] ROI 指定で輝度時系列が出力される
- [ ] ROI 輝度ステップが avsync イベントとして照合される
- [ ] ROI 未指定時の既存挙動が変わらない

## Verification

- [ ] `cargo test -p al-analysis -p al-media`
