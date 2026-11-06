---
id: "TSPEC-034FdwKCwLDfKlaVU7JcO"
type: "test-spec"
status: "approved"
title: "ci.yml 構造契約(最終段階): workflow_dispatch 唯一 trigger・expected-sha 無条件照合"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/147"
  changes:
    added: []
    modified:
      - type: "file"
        path: "tools/check-ci-workflow.mjs"
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
    - "../test-specs/full-ci-ci-summary-issue-158.md"
    - "../test-specs/ci-tracker-reconcile-issue-159.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-ci-dispatch-only-issue-147.md"
  derived-by: []
  verifies:
    - "../specs/ci-dispatch-only-issue-147.md"
    - "../designs/ci-dispatch-only-issue-147.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ci.yml 構造契約(最終段階): workflow_dispatch 唯一 trigger・expected-sha 無条件照合

## Purpose

`tools/check-ci-workflow.mjs` が検査する構造契約のうち、本変更で
反転/追加される分を定義する。不変部分(job 集合・needs・
fail-closed・残滓禁止・入力展開禁止)は issue-158 test-spec の
契約を継続する。

## Contract Under Test

### T1: trigger 集合の厳密一致

`on:` ブロックの trigger key(2-indent)集合が
`{workflow_dispatch}` と厳密一致する。`pull_request`/`push`/
`schedule`/`workflow_run`/`pull_request_target`/`issue_comment`/
`merge_group`/`workflow_call` 等の混入は全て違反。

### T2: expected_sha input 契約(不変)

`workflow_dispatch.inputs.expected_sha` が `required: true` で宣言
されている。

### T3: expected-sha 無条件照合(反転)

`expected-sha` job が `git rev-parse HEAD` と `expected_sha` 参照を
持ち、かつ `github.event_name` を参照する `if:` を持たない
(dispatch 専用下で全 run が照合を実行する)。

### T4: 不変契約(再確認)

- job 集合 12 個の厳密一致(expected-sha, ruviform-access, hygiene,
  verify, verify-macos, ui-verify, ui-verify-macos, webview-verify,
  webview-verify-macos, gui-real, ci-summary, summary)。
- lane needs `[ruviform-access, expected-sha]` + 両 preflight success
  の `if:`。
- `ci-summary`/`summary` の always()+needs scope・check 名。
- 選択実行機構の残滓禁止・`run:` 本文への
  `github.event.inputs.*` 式展開禁止。

### T5: 実 GitHub 受入(worker 実行、ローカル再現不可)

- PR 作成/更新で `ci`/`daily-ci`/`ci-issue-tracker`/
  `ci-issue-tracker-reconcile` の run が当該 head SHA で発生しない
  (Actions API・check suite で時系列証跡を記録)。
- 全 repo で required_status_checks が存在しない(再確認)。

## Negative Fixtures(exit 1 を期待)

| ID | 変異 | 検出契約 |
| --- | --- | --- |
| N1 | `pull_request:` trigger 再導入 | T1 |
| N2 | `push:` trigger 再導入 | T1 |
| N3 | `schedule:` trigger 混入 | T1 |
| N4 | `inputs.expected_sha` 撤去 | T2 |
| N5 | `if: github.event_name == 'workflow_dispatch'` 復活 | T3 |

正 fixture(変更後の実 `ci.yml`)は exit 0。

## Out of Scope

- 共有 evaluator の単体試験(Ruviform 側責務、pin `eed590e` で
  91/50/31 pass は別工程の証跡)。
- GitHub runner 上の lane 実実行(本変更は trigger 層のみ、lane
  本体は不変)。
- cron 実発火(NotRun として扱う)。
