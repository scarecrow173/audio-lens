---
id: "TASK-034z9nCqVubu7pBmp7vaOT"
type: "task"
status: "done"
title: "ExpensiveAnalysisStage: トリガ + 深掘り解析"
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
    - "docs/tasks/analysis-candidate-events.md"
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

# ExpensiveAnalysisStage: トリガ + 深掘り解析

## Work

`ExpensiveAnalysisStage` を実装する。
`depends_on = [Ingest, CheapAnalysis]`(reference が無くても
cheap 由来の候補で起動できる)。

- 開始時: cheap/reference 特徴を dataset から読み、候補イベントを
  抽出して `events` ロールへ書き込む(前タスクのモデル利用)
- トリガモード(spec/plan):
  - 自動(候補イベント発生時)
  - 高残差 / 低類似度 / タイミング異常閾値
  - 手動範囲指定(config)
  - 明示的 always-run
- 深掘り解析: トリガされた範囲に対して高分解能の
  再解析(より細かい STFT、追加測定)を行い
  `features/expensive` へ書き込む
- **下位観測は上書きしない**: expensive の結果は別ロール・別行、
  元の cheap/reference 行は不変
- expensive 未選択時に events が `not_produced` であることを
  dataset 上の表現で区別できるようにする

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/analysis-stages.md`
- `docs/specs/expensive-analysis.md`
- `docs/specs/candidate-event-model.md`

## Dependencies

- `docs/tasks/analysis-candidate-events.md`

## Affected Files

- `crates/al-analysis/src/expensive.rs`(新規)

## Done When

- [ ] 全トリガモードが仕様どおり起動/抑制される
- [ ] expensive 出力が cheap/reference を上書きしない
- [ ] reference なしでも cheap 候補で起動できる

## Verification

- [ ] `cargo test -p al-analysis`
