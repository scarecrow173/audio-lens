---
id: "DESIGN-034DlyKvtWhb8oy1WLAsPV"
type: "design"
status: "draft"
title: "残存課題の責務境界と検証設計"
created: "2026-10-10"
updated: "2026-10-10"
owners: ["scarecrow173"]
relations:
  source: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/overview.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/residual-compliance.md"
    - "docs/adr/residual-issue-boundaries.md"
  derived-by:
    - "docs/plans/residual-fixes.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/28"
    - "https://github.com/scarecrow173/audio-lens/issues/29"
    - "https://github.com/scarecrow173/audio-lens/issues/30"
    - "https://github.com/scarecrow173/audio-lens/issues/31"
    - "https://github.com/scarecrow173/audio-lens/issues/32"
    - "https://github.com/scarecrow173/audio-lens/issues/33"
  defers: []
  deferred-by: []
---

# 残存課題の責務境界と検証設計

## Context

#28〜#33 の共通の完了条件は、計測の意味と来歴を守り、仕様との残差を明示することである。spec と ADR の各 1 件を直接の derives-from とし、既存領域仕様は spec 経由で参照する。

## Scope

### In Scope

Issue ごとの修正境界と検証設計。#33 の方式判断を実装修正から分離する。

### Out of Scope

未承認の方式決定、依存追加、高速化・デコーダ拡張・ROI 実装。

## Components and Boundaries

| Issue | 所有境界 | 設計方針 |
| --- | --- | --- |
| #28 | parser/clock、metadata、dataset、viz | 有理時計の余り保持、非デコード対象にも構造検証、unavailable の統一表現。codec の厳密判定・冗長 path・timeline 欠落は各所有者で修正する |
| #29 | expensive family と共通 emit | 元チャネル identity を保持して窓を切り、未支持/不良条件を validity に反映する。IR 窓と summary の意味を fixture で固定する |
| #30 | fixture builder とテスト所有者 | 実コンテナ契約に沿った padding/index/stride、強い assertion、失敗時 cleanup、合成来歴の説明を管理する |
| #31 | config 入力と avsync | マーカーの有限時刻と明示クロックを入力境界で検証し、検出イベントと同じ時刻表現へ一箇所で変換して既存照合へ合流する |
| #32 | residual_search と audio store | 減算済みチャネルを既存 residual ロールで保存し、search JSON と親 source/alignment の lineage を同時に登録する |
| #33 | stage orchestration と dataset schema | 独立候補段/現状の仕様化、FLAC/保存形式の仕様改訂、第一級 sample_rate/既存参照方式を比較して承認に回す |

## Data and Control Flow

1. Issue 原文の項目と AC を task に対応付ける。
2. 所有層で入力と状態を検証し、既存の計測・来歴表現を利用する。
3. 正常・無効・欠落の結果を dataset に保存して consumer から辿れるようにする。
4. #33 の判断が承認された後だけ仕様と実装を更新する。

## Non-Functional Constraints

未指定 marker の既存挙動と部分 ingest を保持する。チャネル identity と時計精度を損なわない。形式変更は再生成する。新依存の選定は承認まで保留する。

## Risks and Trade-offs

#28 の時計修正が #31 のイベント差分に影響するため #31 は #28 の完了後に検証する。#33 の schema 判断が #32 の参照構造に影響する場合は再 review し、互換補償を追加しない。

## Deferred Design Concerns

#33 の三つの方式、marker の入力位置とクロック契約は承認時に明文化する。draft を実装承認と解釈しない。

## Verification Notes

- [ ] spec の AC-28〜AC-33 が plan/task/test-spec へ対応する。
- [ ] derives-from が 1 spec + 1 ADR のみを指す。
- [ ] Graph は関係欠落なしで承認待ちを返す。
