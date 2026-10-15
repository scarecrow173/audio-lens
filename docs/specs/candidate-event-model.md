---
id: "SPEC-034yYlFacHOsi95HzKHN7M"
type: "spec"
status: "proposed"
title: "候補イベントモデルと確度伝播"
created: "2026-10-02"
updated: "2026-10-14"
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
    - "cheap-analysis-features.md"
    - "analysis-dataset.md"
    - "reference-analysis.md"
  blocks:
    - "expensive-analysis.md"
  supersedes: []
  superseded-by: []
  related:
    - "../adr/measurement-over-verdict.md"
  refines:
    - "audiolens-overview.md"
  refined-by: []
  derives-from:
    - "../discovery/initial-requirements-consolidation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 候補イベントモデルと確度伝播

## What Should Be Built

計測値を候補領域 / イベントに変換し、確度を伝播させる横断的ルール。
AudioLens をバグ分類器にしないための規範を定める。

候補 / イベントのセマンティクス:

イベントは「興味深い計測挙動」を意味し、「確認済みバグ」ではない。
例: 不連続候補、強いクリッピング、ラグジャンプ、大きな残差、
低コヒーレンス、反復ブロック候補、カットオフ候補、無音 / ドロップ
候補、伝達変化。

各イベントが持つもの:

- 開始 / 終了または点時刻
- ソース特徴
- 計測強度
- 確度
- validity
- 任意の周波数 / チャンネル範囲
- アナライザ / バージョン
- 裏付けとなる特徴行 / テンソルへのリンク

ゲームバグの「severity」ではなく `strength` のようなフィールドを使う。

確度伝播:

下流の確度は上流の前提を反映しなければならない。例:

- アラインメントが不確かなら伝達 / ゲイン比較の確度を下げる
- 帯域コヒーレンスが低ければ帯域伝達の確度を下げる
- 無音 / 不十分なリファレンスエネルギーはゲインを誤解を招く数値で
  出すのではなく invalid にする

マルチ解像度ポリシー:

標準解像度クラスを定義し、各特徴が自身のクラスを宣言する:

- サンプル / サブ ms
- 約 5 ms
- 約 20 ms
- 約 100 ms
- 約 500 ms
- 約 1〜5 s
- セグメント

クラス名だけでなく実際の window / hop を永続化する。

要約ポリシー(時系列特徴に対し意味がある場合):

- median
- MAD
- min / max
- p05 / p25 / p75 / p95
- 有効率
- サポート数

## Why Now

イベント化と確度伝播は全解析ステージを横断する共通規範であり、
各所で個別実装すると「高確度だが根拠のないイベント」が量産される。

## Users and Value

- 解析者: イベントから原因となった観測へ辿れる
- 下流コンシューマ: 低確度の入力が高確度の主張に化けない

## Scope

### In Scope

- イベントのフィールド体系とセマンティクス
- 確度伝播ルール
- 標準解像度クラスと要約ポリシー

### Out of Scope

- イベントのバグ確定判定
- 個別解析アルゴリズムの内部仕様

## Requirements

- 候補検出が元の連続特徴データを破壊しない
- イベントが原因となった観測にリンクできる
- 低確度入力が暗黙に高確度の下流主張を生まない

## Acceptance Criteria

- [ ] 候補検出が元の連続特徴データを破壊しない
- [ ] イベントが原因となった観測へリンクできる
- [ ] 低確度入力が暗黙に高確度の下流主張を生まない

## Deferred Design Concerns

- strength の標準化スケール | fixture での分布確認が前提 |
  fixture 実装時 | 閾値の恣意的運用

## Sources and Relations

`cheap-analysis-features.md` と `reference-analysis.md` の特徴を
入力として `expensive-analysis.md` のトリガを生成する。データセット
契約は `analysis-dataset.md` を参照。

## #33 の実行境界

候補検出は論理段であり、`ExpensiveAnalysisStage` 内で候補生成・保存を
高コスト解析のトリガ評価より先に実行し、その後 Expensive Analysis を実行する。
独立 StageKind は要求しない。manifest/resume 境界を変更しない。候補データは
解析結果から独立して参照できる構造化成果物であり、stage の独立性とは別の契約。
候補なしでも空の `events/candidates` を保存して未実行と区別し、
高コスト解析が抑制されても候補を保持する。元の連続特徴を変更せず、
候補テーブルの provenance.parents から走査した特徴へ辿れる契約を維持する。
[承認と検証証拠](../tasks/spec-residual-decisions.md) を参照。

2026-10-14 に上記の維持を再確認。候補のみの実行・再実行が必要になれば stage 分離を再検討する。
[今回仕様](spec-residual-contract-finalization.md) を参照。
