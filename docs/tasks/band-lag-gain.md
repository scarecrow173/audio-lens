---
id: "TASK-036dBnMqWeRtYuIoPlKj"
type: "task"
status: "todo"
title: "reference: 帯域別ラグ補償ゲイン推定(#38)"
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
    - "https://github.com/scarecrow173/audio-lens/issues/38"
  defers: []
  deferred-by: []
---

# reference: 帯域別ラグ補償ゲイン推定(#38)

## Work

現行 `ref.band_gain_m2` は PSD 比ベースで、元ツールチェーン資料の
注意点「非コヒーレントな PSD 比は周波数特性判断に使うな」に
抵触するリスクがある。本来の手法 — 「帯域ごとの最良ラグ探索
(±40ms)→ LS ゲイン → 相関上位半分の中央値」— を実装する。

- 1/3oct(または等価)帯域ごとに ±40ms 窓で最良ラグを探索
  (現行 `lag_max=512` = ±10.7ms @48k より広い)
- 帯域ラグ補償後に LS ゲインを算出(least_squares は
  QR 化後の al-math 経由でも可)
- 帯域相関上位半分の中央値を代表値として出力
- coherence / support を validity メタデータとして付与
- 既存 `band_gain_m2` との関係(置き換え or 併記)を決める —
  pre-release のため置き換えてよい

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/media-format-analysis-extensions.md`
- `docs/specs/reference-analysis.md`
- issue #38

## Dependencies

なし(#40 のフィルタキャッシュと統合すると速いが必須ではない)

## Affected Files

- `crates/al-analysis/src/reference.rs`

## Done When

- [ ] 帯域ごとの最良ラグ+LS ゲイン+中央値が観測として出力される
- [ ] coherence-based validity メタデータが付く
- [ ] 非コヒーレント帯域で PSD 比由来の過大評価が出ない

## Verification

- [ ] `cargo test -p al-analysis`
