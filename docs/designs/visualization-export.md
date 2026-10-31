---
id: "DESIGN-034yYLceYoIOUbV56hSfsg"
type: "design"
status: "superseded"
title: "可視化・エクスポート設計"
created: "2026-10-02"
updated: "2026-10-31"
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
    - "dataset-schema.md"
    - "analysis-stages.md"
  blocks: []
  supersedes: []
  superseded-by:
    - "../impl/ir/issue-106-viz-removal.md"
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/visualization-dataset.md"
    - "../specs/analysis-export.md"
    - "../specs/validation-strategy.md"
    - "../adr/derived-visualization-dataset.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

> **Superseded (Issue #106)**: 表示専用の派生可視化(LOD/tiles/
> timeline/views/summary)と静的 HTML レポートは廃止された。表示は
> `al-gui` + `ruviform-viz` が Analysis Dataset を直接読んで担い、
> 可搬 export のみ `al-dataset::export` として残る。移行判断の記録
> は
> [`../impl/ir/issue-106-viz-removal.md`](../impl/ir/issue-106-viz-removal.md)
> を参照。以下は歴史的記録として残す。

# 可視化・エクスポート設計

## Context

Analysis Dataset から人間がレビュー・共有できる出力を生成する
最終段の設計。spec `../specs/visualization-dataset.md`・
`../specs/analysis-export.md`・`../specs/validation-strategy.md` と
ADR `../adr/derived-visualization-dataset.md` に基づく。

## Scope

### In Scope

- Analysis Dataset から派生する Visualization Dataset
- 波形 LOD / スペクトログラムタイル / ヒートマップ /
  伝達・ステレオ・残差・イベント表示用データ
- 人間可読サマリと成果物パッケージの生成
- 移設可能なパッケージレイアウトと決定的出力
- 再生成可能性と非互換時の明示的検出

### Out of Scope

- 具体的 UI / ビューアの実装
- グラフ描画ライブラリやフロントエンドスタックの選定
- 外部サービス連携

## Components and Boundaries

- `visualization-builder` コンポーネント: dataset から同期表示用の
  LOD・タイル・トレース・イベント参照を生成
- `summary-exporter` コンポーネント: findings・evidence・unknowns を
  人間可読形式に整形
- `package-writer` コンポーネント: サマリ + dataset + 主要図版を
  持ち運び可能な配置に出力
- `validation-exporter` コンポーネント: fixture / replay 用の
  golden データを指定可能な形で切り出す
- 境界: この層は解析の意味付けを追加せず、dataset の計測を
  表示可能な形に変換するだけに留める

## Data and Control Flow

- dataset → visualization-builder が LOD / タイル / 共通時間軸を生成
- dataset + 観測・イベント・確度 → summary-exporter が人間可読
  サマリを生成
- package-writer が移植可能な構造で成果物一式を出力
- validation-exporter が fixture 用のスナップショットを
  バージョン付きで生成

## Non-Functional Constraints

- 再生成可能: Visualization Dataset は常に Analysis Dataset から
  作り直せる
- 意味保存: 確度・validity・元計測への参照を欠落させない
- 可搬性: 絶対パス・環境依存の値を含めない
- 決定性: 同一 dataset から同一出力を得られること

## Risks and Trade-offs

- データ肥大 → LOD / タイル化と参照 id で肥大を抑える
- 可視化が意味を作り込むリスク → 派生層では診断語彙を追加しない
- パッケージのバージョン互換 → 明示的スキーマと非互換検出で対処

## Deferred Design Concerns

- 図版フォーマットの具体選定 | Rust 実装での出力形式に依存 |
  実装フェーズ | 再利用性低下
- fixture 抽出の最小単位 | 検証要件の精緻化が前提 |
  検証設計時 | 過剰な fixture 生成

## Verification Notes

- [ ] Visualization Dataset を Analysis Dataset から再生成できる
- [ ] パッケージが別環境で壊れない(可搬)
- [ ] 表示用データが元計測への参照と確度を保持する
