---
id: "DESIGN-034yYLbNoalTLdhNR8vS37"
type: "design"
status: "approved"
title: "解析ステージ・イベント設計"
created: "2026-10-02"
updated: "2026-10-02"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "localization.md"
    - "dataset-schema.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/cheap-analysis-features.md"
    - "../specs/reference-analysis.md"
    - "../specs/expensive-analysis.md"
    - "../specs/candidate-event-model.md"
    - "../adr/measurement-over-verdict.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 解析ステージ・イベント設計

## Context

cheap / reference / expensive の 3 ステージと、それらを束ねる
candidate / event モデルを設計する。spec
`../specs/cheap-analysis-features.md`・`../specs/reference-analysis.md`・
`../specs/expensive-analysis.md`・`../specs/candidate-event-model.md`
と ADR `../adr/measurement-over-verdict.md` に基づく。

## Scope

### In Scope

- 常時実行される cheap 特徴の共通パイプライン
- 定位済みリファレンスがある場合の reference 特徴群
- candidate / manual / always-run の契機で動く expensive 解析
- 複数計測を束ねる candidate / event モデルと確度伝播
- 多解像度出力と validity / 確度 / サポートの統一表現
- staged residual(R0〜R3)の生成・保存・参照

### Out of Scope

- 個々の特徴量・推定器の最終パラメータ詳細
- 閾値の最終決定(固定閾値を避ける方針)
- 可視化表現への最終マッピング

## Components and Boundaries

- `cheap-stage` コンポーネント: STFT / loudness / spectral /
  transient / stereo / timing / anomaly / modulation の常時観測
- `reference-stage` コンポーネント: lag / gain / transfer / coherence /
  phase / group delay / channel response / staged residual
- `candidate-extractor` コンポーネント: cheap / reference 観測から
  candidate を生成し確度を集約
- `expensive-stage` コンポーネント: candidate・手動・常時実行の
  3 契機で高コスト解析を起動
- `event-model` コンポーネント: candidate を event として
  正規化し、範囲・evidence・確度・層を管理
- 境界: 各ステージは dataset に書き込み、後段は dataset / manifest
  経由で参照する

## Data and Control Flow

- aligned inputs → cheap-stage が常時特徴を記録
- aligned reference がある場合 reference-stage が差分・変換・残差を記録
- candidate-extractor が弱い証拠を束ねて candidate を生成
- expensive-stage が candidate / manual / always-run に応じて高コスト
  解析を実行
- event-model が計測を event 化し、確度・validity・サポートを伝播
- すべての観測と判断の中間データは dataset に保存される

## Non-Functional Constraints

- 観測と解釈の分離: 生計測を常に保持し、診断的表現は上書きしない
- 計算コスト制御: expensive を常時実行しない多層構造
- 弱い証拠の保存: 単一スカラーの閾値で情報を落とさない
- 再現性: 特徴ごとに algorithm id / version / params を記録する

## Risks and Trade-offs

- 特徴数が多くデータ量が増大 → cheap で全中間保持、expensive は
  候補駆動で限定
- 候補駆動による見逃し → 低閾値候補生成と validity / support 記録で
  後から検証可能にする
- 多解像度設計の複雑性 → 各出力に分解能と根拠データを明示する

## Deferred Design Concerns

- cheap / reference / expensive の個別特徴実装詳細 | アルゴリズム
  実装時に確定 | 実装フェーズ | 特徴の再設計
- candidate 統合ルールの重み付け | fixture での評価が前提 |
  検証整備時 | 過剰 / 過少なイベント化

## Verification Notes

- [ ] 常時特徴が reference 非依存で実行できる
- [ ] candidate から expensive を起動できる
- [ ] 生観測と event 判定が分離して保存される
