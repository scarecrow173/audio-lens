---
id: "DISC-034AajCHW8VrVBAMwzlOzL"
type: "discovery"
status: "resolved"
title: "高コスト解析の未実装ファミリ"
created: "2026-10-04"
updated: "2026-10-04"
owners: []
relations:
  source:
    - "../specs/expensive-analysis.md"
    - "../../crates/al-analysis/src/expensive.rs"

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
  derives-from: []
  derived-by:
    - "../designs/expensive-families.md"
    - "../plans/expensive-families.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 高コスト解析の未実装ファミリ

## Exploration Goal

issue #9(高コスト詳細解析)の要求ファミリと `al-analysis`
(expensive.rs)の実装を照合した結果、一部ファミリが未実装または
部分的である。残りの要求をどの単位で実装するかを確定する。

## Key Issues and Assumptions

- 実装済み: トリガ機構(event/manual/always-run + 理由永続化)、
  精度カットオフ + 辞書照合、cepstrum echo、SRC 候補レート/
  イメージ相関、THD/極性非対称、dynamics range + attack/release、
  repeat block/gap、codec block/spectral hole、hum/tonal、
  周波数帯別 L/R コヒーレンス
- 未実装または部分(issue #9 要求):
  - impulse-response 候補推定 + early/late energy・decay 記述子
  - click/splice 定位・stuck/repeated buffer 候補の深掘り
  - pre-echo 候補・transient smear(codec 系の残り)
  - compressor/limiter transfer-curve 候補・pumping/ducking 変調
  - intermodulation 証拠(実用的な範囲で)
  - ノイズ分解の stationary/tonal/modulated 分類
  - 周波数依存混合行列(サポート可能な範囲で)
  - level-dependent transfer(非線形の残り)
- 前提: 既存の「下位測定を上書きしない」「トリガ理由を永続化」
  契約は維持する

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| 各ファミリを1タスクずつ expensive.rs に追加 | 既存構造に自然に乗る・検証しやすい | ファイルが大きくなる | yes |
| ファミリごとにモジュール分割して実装 | 見通しが良い | 既存の単一ファイル構成と不整合・移動コスト | maybe |
| 実装しない | — | issue #9 未達のまま | no |

## Tentative Conclusions and Hypotheses

- confirmed: issue の全ファミリは純粋 DSP で実装可能。
  expensive.rs 内に各ファミリの算出を追加し、特徴 id は
  `exp.*` 命名規約に従う
- hypothesis: ファミリ単位の内部関数分割が済んでいれば
  ファイル分割不要で済む(design で確認)

## Open Questions

- intermodulation・周波数依存混合行列は「実用的な範囲」と spec が
  言う範囲の切り分け → design で決定

## Promotion Candidates

- [ ] spec-doc needed: 既存 `docs/specs/expensive-analysis.md` が
  要求を網羅済みのため新規 spec は不要(必要なら補遺)
- [ ] adr-doc needed: 不要(既存アーキテクチャ内の拡張)

## Sources and Relations

- GitHub issue #9 (expensive analysis families)
- `docs/specs/expensive-analysis.md`
- `docs/designs/analysis-stages.md`
- `crates/al-analysis/src/expensive.rs`
