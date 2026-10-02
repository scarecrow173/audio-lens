---
id: "DESIGN-XhBzf2DS7KALGUBXv2xcuH"
type: "design"
status: "draft"
title: "Design Overview"
created: "2026-10-02"
updated: "2026-10-02"
owners: ["scarecrow173"]
relations:
  source: []
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
    - "../specs/overall-architecture.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Design Overview

## 1. この文書の目的

この文書は、AudioLens のアーキテクチャ全体像を短く把握するための
入口。詳細仕様や詳細設計そのものを定義する場ではなく、目的・
方針・制約・導線をまとめ、実装者と AI エージェントが最初に
参照するコンテキストとして機能する。

## 2. システム概要

AudioLens は、ゲームのキャプチャ映像と任意の比較用入力を
アラインし、音声に何が起きたかを計測して構造化解析データと
可視化用データを export する解析基盤。

概念上の処理は次の流れで構成される。

1. Ingest / Decode: 入力 media の decode と timing / channel
   metadata の正規化
2. Locate: source 素材と capture 区間の対応付け
   (multi-method consensus)
3. Cheap Analysis: 常時計算される低コスト feature と中間
   スペクトルの保持
4. Reference Analysis: アライン済み reference が存在する場合の
   比較 feature
5. Candidate Detection / Expensive Analysis: candidate または
   指定区間への高コスト解析
6. Analysis Dataset への永続化
7. Visualization Builder による render-ready dataset への変換

## 3. アーキテクチャ方針

- 観測ファースト: 合否や類似度ではなく、再解釈可能な低レベル
  observable を広く保持する
- 契約分離: 「何を満たすか(specs)」と「どう作るか(designs)」を
  分離する
- 事実と診断の分離: 観測された事実と hypothesis / diagnosis を
  分離し、core analyzer は診断しない
- 段階的解析: cheap → reference → expensive の3段階で、高コスト
  解析は candidate 駆動に限定する
- provenance 完備: 派生値は algorithm version・パラメータ・
  confidence・validity・source lineage を伴う
- 意思決定の明示: 方針変更は ADR に記録して追跡可能にする

## 4. 主要構成

- Spec(外部/下流契約): `docs/specs/overall-architecture.md`
- Architecture(内部構造・分割): `docs/designs/`(本書が入口)
- ADR(意思決定正本): `docs/adr/`
- 要件の詳細草案: GitHub issues #2〜#14(各 stage 領域。
  canonical 化は本 spec / design 群で行う)

## 5. 主要な設計制約

- 可視化 builder は新しい意味解析を行わない(Analysis Dataset
  からの変換のみ)
- 「異常なし」と「計測不可 / low confidence」を区別する
- 正常系 capture を特別な実行経路にせず reference の一種として
  扱う
- 特定のゲームエンジン / middleware 実装に強く依存しない
- 実装詳細の新規方針は ADR を伴うこと

## 6. 主要な品質目標

- 再現性: 同一入力・同一設定から同一の解析 dataset を得られる
- 検証可能性: golden fixture と数値検証で既知の failure mode を
  回帰確認できる
- 追跡可能性: すべての派生値の provenance を辿れる
- 再解釈可能性: 中間スペクトル等の基礎量から後から別解析を
  導出できる

## 7. アーキテクチャ文書の読み方

1. `docs/specs/overall-architecture.md`(何を満たすか、issue #1
   由来のスコープ)
2. `docs/designs/overview.md`(全体像、本書)
3. `docs/adr/`(意思決定履歴)

## 8. 主要リンク

- Specification: `docs/specs/overall-architecture.md`
- Design index: `docs/designs/README.md`
- ADR index: `docs/adr/README.md`

## 9. 実装時の注意事項

- 詳細要件は GitHub issues が草案であり、確定事項は specs /
  designs へ canonical 文書として記録する
- 仕様変更と実装変更を混在させる場合、specs / designs / tasks
  を同時更新して整合を保つ
- 「内部構造の詳細」を specs 側へ書き戻さない(designs へ
  集約する)

## 10. 未確定事項

- 言語 / 実装 stack 全体(数値解析・media decode・columnar
  storage の要件を踏まえて選定する)
- analysis dataset の具体的 schema(issue #6 が草案)
- locate の consensus 方式の詳細(issue #4・#5 が草案)
- 配布形態(CLI / library / その他)
