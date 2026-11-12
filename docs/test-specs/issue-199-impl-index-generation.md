---
id: "TSPEC-034WsMpecRKShroKP0mrqH"
type: "test-spec"
status: "approved"
title: "impl index 生成契約: front matter schema・managed marker・決定的出力"
created: "2026-11-12"
updated: "2026-11-12"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/199"
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
        path: "docs/impl/exp/README.md"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../test-specs/ci-tracker-reconcile-issue-159.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/issue-199-impl-index-generation-plan.md"
  derived-by: []
  verifies:
    - "../specs/issue-199-impl-index-generation.md"
    - "../designs/issue-199-impl-index-generation-design.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# impl index 生成契約: front matter schema・managed marker・決定的出力

## Purpose

Issue #199 が定める `docs/impl/ir`・`docs/impl/exp` 管理 index の
生成契約を、source schema 不正・marker 欠落・index ドリフトに対して
fail-closed で保つ。実装は repo 同梱の `doc-maintenance` /
`impl-doc` script が担い、本 spec はその外部から観測可能な契約を
固定する。index 生成方式や marker 仕様が変更される場合に本 spec
を更新する。

## Feature

`docs/impl/ir`・`docs/impl/exp` の `README.md` は、source 文書
(front matter / jsonl file list)から `doc-maintenance index` で
決定的に再生成される managed index であり、source が schema を
満たさない・index が marker を持たない場合は `rejected` で
安全に失敗する。

## Rules

- Rule: `docs/impl/ir` の全 `.md` ソースは実装記録 schema
  (`id`/`type: "impl"`/`status`/`title`/`created`/`updated`/
  `owners`/`relations`/`metadata.experiments`)を満たさなければ
  ならない。`relations.changes` の各エントリは `{type, ...}`
  object であり、文字列は不可。
- Rule: `docs/impl/ir/README.md` は generated-index marker を
  持ち、4 列 `ID | Title | Status | File` テーブルを active /
  `## Settled <!-- doc-suite:settled-index -->` の 2 tier で
  保持する。settled status は completed/abandoned/superseded。
- Rule: `docs/impl/exp/README.md` は generated-index marker を
  持ち、`.jsonl` file list を filename 順の 1 列 `File` 表で
  保持する。
- Rule: index の行順は filename 順で決定的であり、plan と apply の
  間で source hash・file list・index bytes が変化した場合 apply は
  拒否されなければならない。
- Rule: `index apply --expect <planHash>` は `index plan` の
  トップレベル planHash と一致する場合のみ書き込む。
- Rule: marker を持たない hand-curated index は `rejected` と
  なり、`promote_index`/force 経路で救済してはならない。
- Rule: 既存文書の本文・ファイル名・id は index 生成のために
  変更してはならない(front matter 形式修正のみ許可)。

## Examples

- Example: `docs/impl/ir` のいずれかの `.md` が
  `metadata.experiments` を欠く → `index plan` は当該ファイルを
  違反として列挙し `rejected`。
- Example: `docs/impl/exp/README.md` から generated-index marker
  を除去 → `index plan` は `hand-curated or ambiguous index` で
  `rejected`。
- Example: `index plan` 後に source 文書を変更してから古い
  planHash で `index apply` → hash 不一致で拒否(exit 非 0)。
- Example: 全 dir の index が valid/in-sync の状態で引数なし
  `index apply` → exit 0 かつ書き込みなし。

## Guarantees

- 両 index は source から一意に決定され、手編集された行は次回
  apply で除去・正規化される。
- 違反はファイル単位で列挙され、1 件の不正で他の検査結果を
  隠さない。
- `in-sync` 判定は plan 再計算との一致に基づき、外部状態に
  依存しない。

## Non-goals

- 実装記録本文の必須セクション(Summary/Context/…)充足は
  `audit_impl_record.js` の責務であり、本 spec は検証しない。
- `docs/impl` 以外の dir の index 形式・他 doc type の schema は
  対象外。
- 実験ログ JSONL の内容検証は `audit_experiment_log.js` の責務。

## Risk

- この契約が無ければ、front matter 不正や marker 欠落が index
  生成を静かに壊すか、逆に安全機構を迂回する手編集が常態化する。
  managed index が成立しないと文書追加・status 変更のたびに
  index ドリフトが手作業で修復される。
