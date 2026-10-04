---
id: "TSPEC-034AaE4Ihkxe6GMzxNwRog"
type: "test-spec"
status: "proposed"
title: "Expensive 追加ファミリの検証"
created: "2026-10-04"
updated: "2026-10-04"
owners: []
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
    - "docs/plans/expensive-families.md"
  derived-by: []
  verifies:
    - "docs/specs/expensive-analysis.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---


# Expensive 追加ファミリの検証

## Purpose

issue #9 の未実装 expensive ファミリが、既存契約
(トリガ信頼度上限・下位測定不変・invalid 記録)を守りつつ
観測値を記録することを保証する。

## Feature

expensive stage の追加解析ファミリ

## Rules

- Rule: 各ファミリは `exp.*` の記録規約で
  confidence/validity/support/resolution を持つ
- Rule: 非 Valid トリガで選ばれた領域は invalid 観測になる
- Rule: 新ファミリの追加で既存ファミリの出力が変わらない
- Rule: 数値不安定(deconvolution 等)は条件数/support で防御され、
  根拠の弱い値は valid にならない

## Examples

- Example: 既知残響付き信号 → IR 候補の decay が
  既知値を許容誤差内で回復する
- Example: クリップ/リミット済み信号 → transfer-curve の
  折れが観測される
- Example: stationary ノイズ → stationary に分類される
- Example: 既知 splice を持つ信号 → click/splice 位置が記録される

## Guarantees

- トリガの信頼度を超える記録が出ない
- 下位測定が上書きされない

## Non-goals

- 各観測の物理的真値証明(観測として妥当な範囲)

## Risk

- 数値不安定な推定が「確かな値」として誤読される
