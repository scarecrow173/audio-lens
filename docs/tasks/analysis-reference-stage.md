---
id: "TASK-034z9nvusj4lsGXWZBzU0k"
type: "task"
status: "done"
title: "ReferenceAnalysisStage: リファレンス比較特徴 + R0-R3"
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
    - "docs/plans/analysis-stages.md"
  implemented-by: []
  depends-on:
    - "docs/plans/analysis-stages.md"
    - "docs/tasks/analysis-cheap-stage.md"
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
    - "docs/test-specs/analysis-stages.md"
  references: []
  defers: []
  deferred-by: []
---

# ReferenceAnalysisStage: リファレンス比較特徴 + R0-R3

## Work

`ReferenceAnalysisStage` を実装する。
`depends_on = [Ingest, Locate, CheapAnalysis]`。

- 入力: source/capture の decoded PCM + locate の受理
  alignment(dataset `locate` role または locate artifact 経由)
- 特徴ファミリ(spec `reference-analysis.md`):
  - ラグ / ドリフト(time-map 由来 + 局所推定)
  - ゲイン差 / 400Hz ゲイン
  - 1/3 オクターブ M2(平均二乗誤差)
  - 帯域相関 / コヒーレンス
  - 複素伝達関数 H(f) / 位相 / 群遅延
  - ch 応答行列(mixer 結果の参照実装)
  - 段階残差 R0-R3(アラインのみ / +ゲイン / +EQ / +チャンネル
    行列)と付随データ(各段の残差エネルギー・除去係数)
- 出力: `features/reference` テーブル + `intermediate`/
  `residual` ロール
- alignment が受理されなかったペアは行 validity で表現し、
  ステージは続行する

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/analysis-stages.md`
- `docs/specs/reference-analysis.md`
- `docs/specs/analysis-dataset.md`

## Dependencies

- `docs/tasks/analysis-cheap-stage.md`

## Affected Files

- `crates/al-analysis/src/reference.rs`(新規)
- `crates/al-analysis/src/residual.rs`(新規)

## Done When

- [ ] R0-R3 の各段階残差がエネルギー単調減少を示す
- [ ] 伝達関数が既知の遅延/ゲインを回復する
- [ ] 未受理 alignment ペアが行 validity で表現される

## Verification

- [ ] `cargo test -p al-analysis`
