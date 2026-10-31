---
id: "DESIGN-034yYLbiufAAGV4Pj6UKJc"
type: "design"
status: "approved"
title: "ジョブオーケストレーション設計"
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
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/analysis-job-model.md"
    - "../adr/immutable-resumable-job.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ジョブオーケストレーション設計

## Context

ingest・locate・3 ステージ解析・可視化・エクスポートを束ねる実行
モデルを設計する。spec `../specs/analysis-job-model.md` と ADR
`../adr/immutable-resumable-job.md` に基づき、不変ジョブ・安定 id・
ステージ再開を中核に据える。

## Scope

### In Scope

- ジョブ / ステージ / 成果物のライフサイクルと id 体系
- 入力ロールとペアワイズ解析の構成
- ステージ選択・キャッシュ再利用・部分再開の設計
- manifest と設定のシリアライズ・バージョニング方針
- 再実行比較を可能にする保存の粒度

### Out of Scope

- ジョブスケジューラやキューイング基盤
- 分散実行やクラウドオーケストレーション
- UI / CLI の最終文法

## Components and Boundaries

- `job-model` コンポーネント: ジョブ id、入力ロール、設定、
  ステージ計画を保持する不変な定義
- `stage-runner` コンポーネント: ingest / locate / cheap / reference /
  expensive / dataset / export の実行と再開を管理
- `artifact-registry` コンポーネント: 入力・デコード済み・
  locate 結果・aligned interval・feature などの
  成果物 id を発行・解決
- `manifest` コンポーネント: ジョブ定義・設定・各成果物の系譜を
  記録する正本
- 境界: stage-runner は成果物を直接他ステージへ引き回さず、
  manifest / dataset 経由で参照する

## Data and Control Flow

- job-config を読み込み、不変ジョブを生成
- stage-runner がステージ計画を立て、成果物 id を払い出す
- 各ステージの結果は manifest / dataset に記録され、後続ステージは
  id 参照で入力を解決する
- 失敗時は完了済みステージの成果物を保持し、再開時に未完了部分だけを
  実行する
- 可視化 / エクスポートは解析成果物を再利用する独立ステージとして
  扱う

## Non-Functional Constraints

- 不変性: ジョブは変更せず、再実行は新ジョブとして扱う
- 再現性: manifest だけで全成果物の生成経緯を再構成できる
- 部分再開: 失敗ステージが完了済み成果物を無効化しない
- 決定性: 可能な範囲で決定的な実行を維持する

## Risks and Trade-offs

- 不変ジョブによる記録量増 → 再利用可能な成果物は id で参照し、
  実データは dataset に集約する
- ステージ間の依存が複雑化 → ステージの入出力を manifest で
  明示し、暗黙の結合を避ける
- 設定変更と再実行の切り分け → 設定はジョブに紐付け、差分比較可能に
  する

## Deferred Design Concerns

- 設定ファイルの具体的文法と CLI 表現 | Rust 実装のシリアライズ /
  CLI 構成に依存(`../adr/implementation-stack.md`) |
  実装フェーズ | インターフェースの変更コスト
- キャッシュ粒度の最適化 | 実行時間計測が前提 | 実装フェーズ |
  過剰 / 過少な再実行

## Verification Notes

- [ ] manifest だけで全成果物の生成経緯を再構成できる
- [ ] expensive 失敗後も cheap / reference 成果物が保持される
- [ ] 同一設定の再実行が別ジョブ id として記録される
