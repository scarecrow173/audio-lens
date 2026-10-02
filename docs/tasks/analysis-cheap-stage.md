---
id: "TASK-034z9nv6tpsfrUU5JDU9cD"
type: "task"
status: "todo"
title: "CheapAnalysisStage: 全特徴ファミリ実装"
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
    - "docs/tasks/analysis-dsp-intermediates.md"
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

# CheapAnalysisStage: 全特徴ファミリ実装

## Work

`CheapAnalysisStage` を実装する。`depends_on = [Ingest]` のみ。
リファレンス素材なし(capture-only)で完走すること。

- 入力: `capture_*` ロールの decoded PCM(共有ローダー経由)
- 特徴ファミリ(spec `cheap-analysis-features.md` 全項目):
  - レベル系: ピーク / RMS / LRA / クレストファクタ / DC
  - スペクトル形状: 帯域パワー分布 / 1/3 オクターブ / 包絡 /
    スペクトル重心・ロールオフ・フラットネス
  - ステレオ/ch 間: 相関 / 位相差 / レベル差
  - トランジェント / タイミング: 立ち上がり / 検出 / 粗ラグ変調
  - デジタル異常: クリッピング / ゼロラン / 飽和 / 量子化痕
  - ノイズ / 変調: ノイズフロア / AM/FM / ハム(50/60Hz 高調波比)
  - マルチ分解能サマリ(fine/medium/coarse の FeatureRecord)
- 出力: `features/cheap` テーブルに FeatureRecord、中間
  スペクトルを `intermediate` ロールへ
- validity/confidence 伝播: 入力 validity が低い行は
  confidence を減衰し validity をそのまま保持
- ステージ失敗ではなく行 validity で表現する

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/analysis-stages.md`
- `docs/specs/cheap-analysis-features.md`
- `docs/specs/candidate-event-model.md`

## Dependencies

- `docs/tasks/analysis-dsp-intermediates.md`

## Affected Files

- `crates/al-analysis/src/cheap.rs`(新規)
- `crates/al-analysis/src/features.rs`(新規)

## Done When

- [ ] capture-only ジョブで ingest → cheap が完走する
- [ ] 全特徴ファミリの FeatureRecord が `features/cheap` に
  永続化される
- [ ] validity が入力から伝播する

## Verification

- [ ] `cargo test -p al-analysis`
- [ ] `al run --stages ingest,cheap_analysis` が capture-only で成功
