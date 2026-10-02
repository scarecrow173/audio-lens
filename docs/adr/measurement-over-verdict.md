---
id: "ADR-034yYy5vgxc67xWxy1A4kH"
type: "adr"
status: "proposed"
title: "観測出力を評決より優先する"
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
  implements:
    - "../specs/audiolens-overview.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/candidate-event-model.md"
    - "../specs/analysis-export.md"
    - "../specs/audiolens-overview.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/initial-requirements-consolidation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 観測出力を評決より優先する

## Context and Problem Statement

AudioLens のコア解析器は「音声に何が起きたか」を計測する基盤であり、
根本原因の断定や合否判定を行う場所ではない。しかし、イベントや
エクスポートのサマリーを実装する際に、便利さのために「severity」や
「合否」のような評決的フィールドを導入したくなる誘惑がある。
これを一度入れると、計測値の解釈がツール内部に固定され、後から
別の仮説で再解釈することが不可能になる。

## Decision Drivers

* 未知のバグを前提とするため、ツールが原因モデルを内蔵すると
  想定外の失敗を説明できない
* 観測事実と後段の仮説 / 診断を分離する必要がある
* 「異常が観測されなかった」ことと「計測不能 / 低確度」を
  区別しなければならない
* 将来の AI 診断はエクスポート済みデータのコンシューマであり、
  コア解析の成功条件ではない

## Considered Options

* コアが評決(合否 / severity / 推定原因)を出力する
* コアは観測のみを出力し、評決は下流コンシューマに委ねる
* コアは観測に加えて任意の「総合類似度スコア」を併記する

## Decision Outcome

Chosen option: "コアは観測のみを出力し、評決は下流コンシューマに
委ねる"。計測値の解釈をツール内部に固定しないことが、未知の
失敗モードへの再解釈可能性を保全するため。

### Consequences

* Good, because 観測は仮説変更に対して再利用可能であり、イベント /
  サマリー / エクスポートはすべて「計測された事実」の表現に留まる。
* Bad, because ユーザーが欲しい「答え」をツールが直接返さず、
  解釈は可視化や人間 / 後続ツールの責務になる。

### Confirmation

データセットスキーマ、イベントモデル、エクスポートサマリーのいずれにも
severity / 合否 / 総合類似度 / 原因推定フィールドが存在しないことを
確認する。

## Implementation Plan

* Affected paths: 解析結果スキーマ、イベントモデル、エクスポート
  サマリー生成、将来の可視化・診断コンシューマ
* Patterns to follow: すべての派生値に確度・validity・理由・来歴を
  付与し、「計測不能」を「異常なし」と区別する
* Patterns to avoid: severity / verdict / score フィールドの追加、
  イベントへの原因ラベルの埋め込み、評決を返す API
* Dependencies or configuration: なし

## Verification

* [ ] スキーマとイベントモデルに severity / verdict / score がない
* [ ] エクスポートサマリーが根本原因を推定しない
* [ ] 「異常なし」と「計測不能」が異なる状態で表現される

## Pros and Cons of the Options

### コアが評決(合否 / severity / 推定原因)を出力する

* Good, because ユーザーへの即答性が高く、レポートが単純になる。
* Bad, because 未知のバグを想定外の原因モデルで誤判定し、計測値の
  再解釈余地を失う。

### コアは観測のみを出力し、評決は下流コンシューマに委ねる

* Good, because 観測は仮説変更に再利用可能で、確度・validity・
  来歴を保全できる。
* Bad, because 評決を求める利用者には別レイヤが必要になる。

### コアは観測に加えて任意の「総合類似度スコア」を併記する

* Good, because 一覧性が上がる。
* Bad, because 単一スコアが誤った安心感を生み、スコアの意味が
  コンテキスト依存になる。

## More Information

イベントは「確認済みバグ」ではなく「興味深い計測挙動」であり、
`strength` を用いる規範は `../specs/candidate-event-model.md` を
参照。エクスポートサマリーの非推定要件は
`../specs/analysis-export.md` を参照。
