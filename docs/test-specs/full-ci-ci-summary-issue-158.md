---
id: "TSPEC-034SmNbPOS3xJ9KpmCgL4o"
type: "test-spec"
status: "approved"
title: "Full CI 構造契約(ci.yml): 全実行・ci-summary・expected_sha"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/158"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "tools/check-ci-workflow.mjs"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../plans/implement-full-ci-ci-summary-issue-158.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-full-ci-ci-summary-issue-158.md"
  derived-by: []
  verifies:
    - "../designs/full-ci-ci-summary-issue-158.md"
    - "../specs/full-ci-ci-summary-issue-158.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Full CI 構造契約(ci.yml): 全実行・ci-summary・expected_sha

## Purpose

Issue #158 の安全な第一段階が規定する ci.yml の構造契約を、
選択実行機構の再導入・Required context `summary` の消滅・
`ci-summary` の needs 欠落に対して fail-closed で保つ。
実装は `tools/check-ci-workflow.mjs`(prek hook → hygiene job で
全 run 実施、`mise run check-ci-workflow` が検証入口)。
`ci-summary` が Ruleset Required 化され `summary` が撤去される
最終段階で本 spec の job 集合・needs 契約を更新する。

## Feature

ci.yml が宣言する Full CI 構造契約 — trigger・job 集合・needs・
fail-closed 集約分離・選択実行機構の不在。

## Rules

- `on:` は `pull_request`(branches: [main])・`push`(branches:
  [main])・`workflow_dispatch` の 3 trigger を持つ。
- `workflow_dispatch.inputs.expected_sha` が `required: true` で
  宣言される。
- job 集合は固定の 12 job: `expected-sha`, `ruviform-access`,
  `hygiene`, `verify`, `verify-macos`, `ui-verify`,
  `ui-verify-macos`, `webview-verify`, `webview-verify-macos`,
  `gui-real`, `ci-summary`, `summary`。
- 6 lane job(verify 系 × u/w + macos)は `needs:
  [ruviform-access, expected-sha]` と両 preflight success を課す
  `if:` を持つ(常時実行)。
- `ci-summary` は 10 の全必要 job(macOS legs 含む)を needs に
  持ち `if: always()`。`summary` は macOS legs を含まない
  needs + `if: always()` + `name: summary`。
- コード行中に `impact` job 参照・`classify-impact`・
  `CI_IMPACT_SELECTIVE`・`macos-summary`・`not_applicable`・
  `steps.classify`・`SHADOW` が存在しない。
- `github.event.inputs.*`(未検証ユーザー入力)を `run:` の
  ブロック/インライン本文へ式展開しない — always() な集約 job
  は入力拒否後も実行されるため、inputs は `env:` 値や `if:` 式
  など値としてのみ評価される文脈で扱う。

## Examples

- ci-summary の needs から `*-macos` を外す → 検査 exit 1
  (負例: needs 欠落検出)。
- `expected_sha` を `required: false` にする → 検査 exit 1。
- lane の needs を `impact` へ差し替える → 検査 exit 1。
- `summary` job の `name:` を改名する → 検査 exit 1
  (Required context 消滅検出)。
- `ci-summary` の run 本文に `${{ github.event.inputs.
  expected_sha }}` を埋め込む → 検査 exit 1(未検証入力の
  shell 実行を防止)。`env:` 値への同式の記述は違反としない。

## Guarantees

- actionlint が検出しない構造契約違反(needs 欠落・job 削除・
  required context 消滅・選択実行機構の再導入)を非ゼロ exit で
  検出する。
- コメント中の機構名の言及(設計経緯の記録)は違反としない
  (YAML コメント部を除いた実装部を検査)。

## Non-goals

- GitHub Actions 実行時の `needs.<id>.result` 評価や実 runner
  上の job 成功/失敗そのものは検証しない(本 spec は workflow
  宣言の構造契約のみ)。
- lane 内部の step 内容(Golden/CLI/UI/WebView 各検証の詳細)は
  composite action 側の責務。

## Risk

- 本 spec を落とすと、Required context `summary` の偶発的消失
  (全 PR 滞留)や `ci-summary` への macOS legs 混入(Required
  gate の runner 逼迫化)・逆に `ci-summary` からの legs 欠落
  (Full CI verdict の偽緑)が、人のレビューだけに依存する。
