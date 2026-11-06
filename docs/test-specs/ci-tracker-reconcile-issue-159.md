---
id: "TSPEC-034SKW5DYSMKrwyMXiVFqe"
type: "test-spec"
status: "approved"
title: "Tracker workflow 構造契約: reconcile 入口・共有 pin・event 入口不変"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/159"
  changes:
    added:
      - type: "file"
        path: "tools/check-ci-tracker-workflows.mjs"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "tools/check-ci-tracker-workflows.mjs"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../test-specs/full-ci-ci-summary-issue-158.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-ci-tracker-reconcile-issue-159.md"
  derived-by: []
  verifies:
    - "../specs/ci-tracker-reconcile-issue-159.md"
    - "../designs/ci-tracker-reconcile-issue-159.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/blob/eed590e0a3837e61c943d9de1d3efcfd7df32cb0/docs/specs/issue-368-tracker-reconcile.md"
  defers: []
  deferred-by: []
---

# Tracker workflow 構造契約: reconcile 入口・共有 pin・event 入口不変

## Purpose

Issue #159 reconcile 展開が定める tracker 系 workflow 群の構造契約を、
共有 pin の版ずれ・event 入口への trigger 混入・reconcile 入口の
guard/権限/mode 逸脱に対して fail-closed で保つ。
実装は `tools/check-ci-tracker-workflows.mjs`(prek hook → hygiene
job で全 run が実施、`mise run check-ci-tracker-workflows` が検証
入口)。reconcile 運用が廃止・集約される場合に本 spec を更新する。

## Feature

Tracker 系 workflow(daily-ci・ci-issue-tracker・
ci-issue-tracker-reconcile)は、固定 SHA の Ruviform 共有実装を同一
pin で checkout し、入口ごとの trigger・guard・権限・concurrency・
実行モードの契約を満たす。

## Rules

- Rule: 全対象 workflow の `repository: scarecrow173/ruviform`
  checkout `ref:` は同一の完全 SHA(本 script が保持する正本 pin と
  一致)でなければならない。
- Rule: `ci-issue-tracker-reconcile.yml` の trigger は `schedule` と
  `workflow_dispatch` のみ。`workflow_run`・`pull_request`・`push` は
  存在してはならない。
- Rule: `ci-issue-tracker.yml` の trigger は `workflow_run` のみ。
  `schedule`/`workflow_dispatch`/`pull_request`/`push` の追加は
  event 入口専用契約(RF ADR-vJvyZNtrHi2xnycJxbnSPv の trigger 禁止
  補足と同一境界)に違反する。
- Rule: reconcile job は
  `github.event_name == 'schedule' || github.ref_name == github.event.repository.default_branch`
  の `if:` を持ち、schedule を無条件通過させる(payload の
  `repository` 欠落で cron が silent skip しない)。
- Rule: reconcile workflow の `concurrency.group` は
  `ci-issue-tracker-${{ github.repository }}`(event 入口と同一)で
  `cancel-in-progress: false`。
- Rule: reconcile job の permissions は
  `contents:read`/`actions:read`/`issues:write` のみ(それ以外の key
  を持たない)で、`timeout-minutes: 15` を持つ。
- Rule: AudioLens repo checkout は
  `github.event.repository.default_branch || 'main'` に pin し、
  共有実装 checkout は `secrets.RUVIFORM_DEPLOY_KEY` を使い
  `persist-credentials: false`。
- Rule: tracker 実行は `tracker.mjs --reconcile` を明示し、当該
  step の env に `TRACKER_EVENT_PATH` を含まない。self-test
  (`node --test .../ci/tracker/`)が実行 step より前に存在する。
- Rule: cron は UTC で分 != 0。

## Examples

- Example: `daily-ci.yml` の pin が eed590e、
  `ci-issue-tracker.yml` の pinが旧 73d51d8 のまま → check は
  pin 不一致を列挙して exit 1。
- Example: `ci-issue-tracker.yml` の `on:` に `schedule` を追加 →
  check は event 入口 trigger 違反を列挙して exit 1。
- Example: reconcile workflow の job `if:` から
  `github.event_name == 'schedule' ||` を除去 → check は guard
  契約違反を列挙して exit 1(schedule silent skip 防止)。
- Example: 実行 step から `--reconcile` を除去、または env に
  `TRACKER_EVENT_PATH` を追加 → check は mode 契約違反で exit 1。

## Guarantees

- pin は全対象ファイルで 1 値のみ。ずれは全件列挙され exit 1。
- trigger・guard・concurrency・permissions・timeout・checkout pin・
  実行モードの各契約は独立に列挙される(1 違反で他違反の報告を
  隠さない)。
- script は YAML parser に依存せず indentation 構造で検査する
  (check-ci-workflow.mjs と同方式)。
- 契約違反の報告は日本語メッセージで全件列挙し、検査対象 path を
  出力する。

## Non-goals

- evaluator 内部の正しさ(run 履歴 reconcile・compare guard・Issue
  冪等)は共有 suite(`node --test ci/tracker/`)の責務であり、本
  spec は検証しない。
- workflow の YAML 文法(actionlint が担う)・runtime の GitHub API
  挙動・cron の実発火は検証しない。
- `ci.yml` の構造契約は `check-ci-workflow.mjs`/既存 test-spec の
  責務であり、本 spec では触れない。
- `daily-ci.yml` の decide 契約(trigger・inputs・permissions)の
  詳細検査は含めず、共有 pin 一致のみを対象とする。

## Risk

- この契約が無ければ、pin の片方更新で Scheduler と Tracker が異版の
  evaluator を実行し得る(判定ドリフト)。event 入口への cron 追加は
  GitHub auto-disable の blast radius を event 入口へ広げ、reconcile
  分離の RF ADR 決定を無効化する。guard 欠落は cron 全 tick の
  silent skip、または非 default ref での権限付き実行を招く。
