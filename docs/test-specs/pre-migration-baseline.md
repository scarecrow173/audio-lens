---
id: "TSPEC-034P50olYIsFgSOHmyg3kB"
type: "test-spec"
status: "proposed"
title: "pre-migration baseline の検証仕様"
created: "2026-10-30"
updated: "2026-10-31"
owners:
  - "scarecrow173"
relations:
  source:
    - "../specs/pre-migration-baseline-golden.md"
    - "../adr/pre-migration-baseline.md"
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
  related:
    - "../designs/pre-migration-baseline.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "../plans/implement-pre-migration-baseline.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# pre-migration baseline の検証仕様

## Purpose

Issue #98 の baseline が「同じ手順で再計測可能で、契約上
意味が変わらない」ことを担保する検証面を定義する。baseline
自体は記録であって判定器ではないため、ここではツール
(capture/normalize/compare)と成果物の成立条件を定める。

## Feature

`tools/baseline` の capture・normalize・compare が、ADR
`docs/adr/pre-migration-baseline.md` の正規化契約・比較
意味論・run matrix に従って baseline を生成・検証できる。

## Rules

- Rule: ID 正規化は run グローバルに一貫し、`_` 埋め込み・
  文字列内・ファイル名中の ID をすべて同一 canonical 名に
  写像する
- Rule: 絶対パスは field 名ではなく記録した abs prefix で
  判定して placeholder に置換する
- Rule: bit-exact 面は整数・enum・判定・PCM・path 集合の
  厳密一致、tolerance 面は要素ごとの
  `|a-b| <= max(abs_tol, rel_tol*max(|a|,|b|))` で判定する
- Rule: `.bin` payload は sha256 が異なる tolerance class のみ
  decode して要素比較する
- Rule: resume run の normalized golden は canonical と一致
  する
- Rule: A/A(同一入力 2 回)で bit-exact 面が一致しない場合は
  determinism.json に非決定面として記録される
- Rule: coverage.json は Issue #98 列挙の全項目に担保手段を
  割り当てる(未実施は理由つき)

## Examples

- `normalize` が `residual/IN-…_ART-….wav` の両 ID を
  canonical 名に置換する(`\b` 不使用で検証)
- `compare` が tolerance 内の f64 差は差分にせず、閾値超過
  は差分として報告する
- `compare --self-test` が意図差分(bit-exact 差・tolerance
  超過・path 欠損)を正しく報告する
- `store.mjs` が table の `i64`/`f64`/`utf8`/`null` cell を
  decode し、BigInt で 2^53 超えを検出する

## Guarantees

- run matrix の全 run が `baseline/runs/<name>/` に記録される
- `baseline/environment.json`・`inputs.json`・`config.json` が
  再現に必要な情報を持つ
- 実測未実施項目(stage 別 busy/CPU/RSS/IO)が README と
  performance.json に明記される

## Non-goals

- 比較の自動合否判定・CI 組み込み
- al-viz HTML 内容の golden 化
- 異環境での bit-exact 一致

## Risk

- これらの保証がないと baseline が「比較不能な記録」になり、
  Issue #95 の before/after 評価が印象に依存する
