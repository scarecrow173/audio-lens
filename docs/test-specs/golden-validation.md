---
id: "TSPEC-034AaE61nEEdgSDFQNXCjW"
type: "test-spec"
status: "proposed"
title: "ゴールデン fixture マトリクスの検証"
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
    - "docs/plans/golden-validation.md"
  derived-by: []
  verifies:
    - "docs/specs/validation-strategy.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---


# ゴールデン fixture マトリクスの検証

## Purpose

issue #13 の既知ケース行列が、プログラム生成された決定的
fixture と許容誤差付き期待値で回帰可能であることを保証する。

## Feature

共有 fixture 基盤と既知ケースの回帰行列

## Rules

- Rule: fixture はすべてプログラム生成で決定的(シード固定)
- Rule: 期待値は許容誤差付きで、回復すべき関係を検証する
- Rule: spec 列挙ケースが対応するテストとして存在する
- Rule: 偽受理・低相関ケースは期待どおり
  invalid/low confidence になる

## Examples

- Example: gain 変換挿入 → locate が変換を越えて定位する
- Example: 強周期信号 → 偽ピークが受理されない
- Example: 既知 EQ/遅延/極性 → reference がそれぞれ
  該当メトリクスで回復する
- Example: CFR/VFR・既知オフセット/ドリフト →
  A-V 観測がそれを記録する

## Guarantees

- 行列の各ケースが外部アセット非依存で再現できる
- 回帰がしきい値/アルゴリズム変更を検出できる

## Non-goals

- 実機材由来の期待値
- 新規機能の fixture(各 WS の担当)

## Risk

- 行列が未網羅のまま「検証済み」と誤認される
