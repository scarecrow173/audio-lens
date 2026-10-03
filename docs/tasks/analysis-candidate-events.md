---
id: "TASK-034z9nBEhJkL7a1GaPjUjG"
type: "task"
status: "done"
title: "候補イベントモデル + 抽出/永続化"
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
    - "docs/tasks/analysis-reference-stage.md"
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

# 候補イベントモデル + 抽出/永続化

## Work

候補イベントモデル(spec `candidate-event-model.md`)を実装し、
`ExpensiveAnalysisStage` 冒頭の候補抽出に使う永続化経路を提供する。

- CandidateEvent スキーマ: stable content-derived ID
  (入力ハッシュ + 特徴種別 + 時間範囲 + パラメータから導出)、
  kind、時間範囲、confidence、validity、証拠リンク
- 証拠リンク: イベント → 特徴行 / tensor / alignment の参照を
  FeatureRecord ID ベースで保持
- 抽出ルール: cheap/reference 特徴から spec 定義の閾値越えを
  候補として抽出(閾値は config でオーバーレイ可能)
- 永続化: `events` ロールのテーブルへ。ExpensiveAnalysisStage が
  選択されない場合 `not_produced` 状態を明示できるよう、
  出力責務は expensive ステージ側に置く
- イベントは「確認済みバグ」ではなく測定上の注目点として
  表現する(measurement-over-verdict ADR)

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/analysis-stages.md`
- `docs/specs/candidate-event-model.md`
- `docs/adr/measurement-over-verdict.md`

## Dependencies

- `docs/tasks/analysis-reference-stage.md`

## Affected Files

- `crates/al-analysis/src/events.rs`(新規)

## Done When

- [ ] 同一入力から常に同一のイベント ID が導出される
- [ ] イベントが証拠リンク(特徴行/tensor 参照)を保持する
- [ ] events ロールに永続化できる

## Verification

- [ ] `cargo test -p al-analysis`
