---
id: "TSPEC-034WuVDeP2Keiu82igc78e"
type: "test-spec"
status: "approved"
title: "ir 必須セクション契約: missing-section 0 件と本文具体性"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/202"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "doc"
        path: "docs/impl/ir/README.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-199-impl-index-generation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-202-ir-missing-sections-plan.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-202-ir-missing-sections.md"
    - "docs/designs/issue-202-ir-missing-sections-design.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ir 必須セクション契約: missing-section 0 件と本文具体性

## Purpose

Issue #202 が定める `docs/impl/ir` 本文セクション補完の外部
から観測可能な契約を固定する。実装は各 ir 文書の編集と
`impl-doc` の `audit_impl_record.js` 監査が担い、本 spec は
「必須セクションが実内容で存在する」ことの判定基準を残す。
必須セクション定義や監査ロジックが変更される場合に本 spec
を更新する。

## Feature

`docs/impl/ir/` の全 ir 文書は、`audit_impl_record.js` が要求する
7 つの必須 `##` セクションを、対象文書の実装・検証内容に基づく
具体的な本文で保持する。

## Rules

- Rule: 各 ir 文書は `## Summary` / `## Context` /
  `## Implementation` / `## Related Experiments` /
  `## Validation` / `## Risks` / `## Follow-ups` の 7 見出しを
  `^##\s+<canonical>\s*$` 形式(完全一致、case-insensitive)で
  持たなければならない。
- Rule: 各セクションの本文は当該文書が記録した実装変更・検証
  結果・残課題に基づく具体的な記述でなければならず、空見出し、
  TODO のみ、定型文の羅列は禁止。
- Rule: Related Experiments は実在する `docs/impl/exp/` ログと
  front matter `metadata.experiments` のみを根拠とし、存在しない
  実験を参照してはならない。
- Rule: front matter(id/status/relations/updated を含む)と
  ファイル名は本文補完によって変更してはならない。
- Rule: `missing-section` 以外の audit error code
  (missing-experiment-link 等)の件数を増やしてはならない。

## Examples

- Example: `issue-105-gui-app.md` の `## 検証結果(実行済み)` を
  `## Validation` へ改名し、`## 残課題・制限` を `## Follow-ups`
  の内容として再配置する → missing-section から当該文書分が
  消える。
- Example: `ruviform-lockfile-source-of-truth.md` は 6 セクション
  完備で `## Risks` のみ欠落 → 文書記述の pin/lockfile 運用リスクを
  根拠に `## Risks` を追記する。
- Example: `## Validation` 見出しの下に「後で書く」とだけある
  文書 → 具体性ルール違反であり、文書内の検証記録から実内容を
  補うまで受入不可。

## Guarantees

- `audit_impl_record.js --json` の `missing-section` が 0 件に
  なり、ir 文書の必須構造が全件で一貫する。
- 補完は既存本文の再配置と不足分追記に限定され、文書が記録した
  事実関係は変わらない。
- `docs/impl/ir/README.md` の管理 index は front matter 由来の
  ため本文補完では drift せず、`index plan` が `in-sync` を保つ。

## Non-goals

- `missing-experiment-link` 等 front matter 由来の dangling
  参照の修復(編集範囲外、別 Issue)。
- ir 文書本文の内容改訂・最新化(歴史的記録の保存が目的)。
- `docs/impl/exp/`・他 docs dir の本文変更。
- 新規 ir 文書の作成とそのセクション適合(対象は既存文書のみ)。

## Risk

- この契約が無ければ、見出しの形だけを満たす空のセクションや
  根拠のない定型文で監査を通過できてしまい、ir 文書の記録価値が
  失われる。また front matter を不用意に変更すると index drift
  や relation 破壊を招く。
