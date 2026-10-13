---
id: "ADR-034yYy6pBGTFj1fMRdqJdg"
type: "adr"
status: "proposed"
title: "locate 受理に複数手法の合議を要求する"
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
    - "../specs/source-localization.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/locate-tracking.md"
    - "../specs/audiolens-overview.md"
    - "../specs/source-localization.md"
    - "locate-argmax-ambiguity-representation.md"
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

# locate 受理に複数手法の合議を要求する

## Context and Problem Statement

ソース定位は比較解析の前提であり、1 つの誤った受理位置が下流の
全計測を誤った区間で実行させる。単一手法の最高スコアだけで
受理すると、無音境界・周期信号・広いピークで偽受理が起きる。

## Decision Drivers

* 偽相関ピークを単一手法が見抜けない失敗モードを排除したい
* 定位は「評決」ではなくアラインメント証拠であり、不一致そのものも
  後段のデバッグ材料になる
* 受理 / 棄却の閾値は調整可能にして検証 fixture で裏付けたい

## Considered Options

* 単一手法の最高スコア候補を受理する
* 複数手法の独立証拠を保持し、少なくとも 2 手法の位置一致と
  ピーク / 背景閾値で合議受理する
* 複数手法を重み付き融合して単一スコアで受理する

## Decision Outcome

Chosen option: "複数手法の独立証拠を保持し、少なくとも 2 手法の
位置一致とピーク / 背景閾値で合議受理する"。単一スコアへの縮約を
避け、偽受理を構造的に防ぐため。

### Consequences

* Good, because 1 手法の偽ピークが受理マッチにならず、不一致も
  証拠として残る。
* Bad, because 複数手法の実行と証拠保存のコストが増える。

### Confirmation

locate 結果に手法ごとの証拠・一致数・位置ばらつき・棄却理由が
保持され、単一手法のみの候補が受理されないことを確認する。

## Implementation Plan

* Affected paths: locate パイプライン、locate 結果スキーマ、
  alignment id の発行
* Patterns to follow: NCC / band-NCC / PHAT / スペクトル包絡を
  独立証拠として保持し、受理は合議ルールのみで行う
* Patterns to avoid: 最終位置だけを残す縮約、単一スコアへの融合、
  棄却理由の破棄
* Dependencies or configuration: 閾値と位置許容差は設定可能とする

## Verification

* [ ] 1 手法のみの偽ピークが受理マッチとして採用されない
* [ ] 手法ごとの位置と棄却理由が永続化される
* [ ] 受理に 2 手法以上の一致が要求される

## Pros and Cons of the Options

### 単一手法の最高スコア候補を受理する

* Good, because 実装が最も単純で計算も軽い。
* Bad, because 無音境界や周期信号の偽ピークを検出できない。

### 複数手法の独立証拠を保持し、少なくとも 2 手法の位置一致とピーク / 背景閾値で合議受理する

* Good, because 偽受理を構造的に防ぎ、証拠を残したまま不一致を
  解析できる。
* Bad, because 実装・計算・保存のコストが増える。

### 複数手法を重み付き融合して単一スコアで受理する

* Good, because 判定が一見単純になる。
* Bad, because 融合係数に原因モデルを埋め込み、不一致の証拠が
  失われる。

## More Information

基礎手法と既知失敗モードは `../specs/source-localization.md`、
ループ / 長尺への拡張は `../specs/locate-tracking.md` を参照。
