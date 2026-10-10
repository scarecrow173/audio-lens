---
id: "ADR-034DlyK8LKvSfTztL5Bfu5"
type: "adr"
status: "proposed"
title: "残存課題を責務境界で修正し仕様判断を分離する"
created: "2026-10-10"
updated: "2026-10-11"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/28"
    - "https://github.com/scarecrow173/audio-lens/issues/29"
    - "https://github.com/scarecrow173/audio-lens/issues/30"
    - "https://github.com/scarecrow173/audio-lens/issues/31"
    - "https://github.com/scarecrow173/audio-lens/issues/32"
    - "https://github.com/scarecrow173/audio-lens/issues/33"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/residual-compliance.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/specs/residual-compliance.md"
    - "docs/designs/residual-fixes.md"
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

# 残存課題を責務境界で修正し仕様判断を分離する

## Context and Problem Statement

#28〜#33 はレビュー修正、欠落入力/成果物、仕様との差の判断が混在する。横断的な downstream fallback は時刻・妥当性・来歴の意味をさらに分岐させる。既存の部分 ingest と観測主義を保持し、未承認変更を実装タスクとして実行させない管理が必要。

## Decision Drivers

- ルールの所有者で修正し、同じ概念の表記を集約する。
- 既存仕様を実装に合わせて無断で弱めない。
- #28〜#33 の完了証拠を個別に追跡する。

## Considered Options

* 責務境界での修正と仕様判断の分離: 各 Issue の所有層で修正し、#33 の判断を承認待ちにする。
* 下流の一括補償: 利用者ごとに時刻、無効値、欠落成果物を補う。
* 現状への一括仕様緩和: 指摘を仕様変更だけで閉じる。

## Decision Outcome

Chosen option: "責務境界での修正と仕様判断の分離"。提案であり採択待ち。共通の残件棚卸し plan に独立 stream を明記し、各 Issue を別 task で追跡する。design/plan は draft、全 task は blocked に保持する。

### Consequences

修正場所と検証の対応が明確になる一方、#33 の選択には別の承認が必要になる。新依存、FLAC 方式、独立 StageKind、sample_rate 型の選定は本 ADR で決定しない。

### Confirmation

Artifact Graph が一意の spec/ADR/design/plan を選び、全 Issue の task を選択する。承認前は runnable task がない。

## Pros and Cons of the Options

- 境界修正: 契約の正本を維持する。Issue 単位の確認コストがある。
- 下流補償: 局所修正が容易だが、情報損失と表現ドリフトを隠す。
- 仕様緩和: 実装負担は小さいが、残差再聴取など利用者価値を失う。

## Implementation Plan

#28 は al-media の parser/clock と dataset/viz の欠落情報境界、#29 は al-analysis の expensive family、#30 は al-testkit と golden fixture、#31 は avsync/config 入力境界、#32 は al-locate と既存 audio store に割り当てる。#33 は既存 spec 対応表と選択肢を先に review し、承認後に必要な実装・仕様更新を行う。依存の変更は別途許可を求める。形式変更時は成果物を再生成し互換層を追加しない。

## Verification

- [ ] spec focus の inspect_graph の Artifact Graph issues が空で briefing が pass、build_task_graph の issues が空。
- [ ] draft design/plan が実行 chain の候補から除外され、承認前は design/planning が通過しない。
- [ ] 対応 task の検証と関連 mise task が承認後の実装で成功する。

## More Information

対象は #28 #29 #30 #31 #32 #33。方式を選定する段階で本提案を再検討する。既存 SIMD/decoder 拡張の作業を混ぜない。

## #28 の実施確認(2026-10-11)

#28 の作業境界は parser/clock、native registry、metadata、dataset/viz と
時計消費者 avsync。coordinator の question `msg_7c43909b855e` reply により、
独立 task 分割は省略して項目別証拠と論理別コミットで追跡すること、
packet-slot 文書化、dwStart 実相対 offset の保持、rec LIST 未対応の明示、
i64 overflow の構造不正境界を承認した。
[実施 task](../tasks/container-review-edge-cases.md) に全項目の証拠を記録する。
本確認は #28 の実施判断であり、#33 等の未承認方式を採択しない。

## #33 の方式承認 (2026-10-11)

coordinator は task の事前質問に対し、候補検出の fused 実行を論理段として
仕様化、FLAC 保存の defer、`Resolution.sample_rate: u32` の第一級化を承認した。
候補のみ再実行を提供する独立 StageKind は採らず、順序・候補保存・非破壊を
維持する。FLAC の未実装を適合済みとせず、#32 の Float64 WAV と来歴を維持する。
分解能は解析対象 PCM のレートを保存し、旧成果物は再生成する。
独立実装修正は分解能のみのため task 分割不要も承認された。
この承認は #33 の方式判断に限り、本 ADR 全体の status は変更しない。
[選択肢・理由・再検討条件・証拠](../tasks/spec-residual-decisions.md) を参照。
