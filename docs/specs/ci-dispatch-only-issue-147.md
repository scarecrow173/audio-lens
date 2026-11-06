---
id: "SPEC-034jbVIejFblXPjaYuN9R"
type: "spec"
status: "approved"
title: "Issue #147/#158 最終段階: ci.yml を workflow_dispatch 専用にし PR/push 起動入口を撤去する"
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
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "tools/check-ci-workflow.mjs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/ci-dispatch-only-issue-147.md"
    - "../plans/implement-ci-dispatch-only-issue-147.md"
  depends-on:
    - "../designs/audiolens-independent-ci.md"
    - "../designs/full-ci-ci-summary-issue-158.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "https://github.com/scarecrow173/audio-lens/issues/158"
    - "https://github.com/scarecrow173/audio-lens/issues/159"
  refines:
    - "../designs/audiolens-independent-ci.md"
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/audio-lens/issues/147"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ci-dispatch-only-issue-147.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #147/#158 最終段階: ci.yml を workflow_dispatch 専用にし PR/push 起動入口を撤去する

## What Should Be Built

`ci.yml` から `pull_request` / `push`(main) trigger を撤去し、起動入口を
`workflow_dispatch` のみとする。これは Issue #158 設計で明示的に最終段階へ
延期された「変更 5: PR/push trigger 削除」であり、次の前提が全て実証済みの
時点でのみ実行する:

- Ruleset `main-checkstatus`(Required status checks 保持側)が全対象 repo
  (audio-lens / ruviform / synvelis)で disabled、active 側
  `main-protect` が required_status_checks を含まないことを作業時点で
  実 API 再確認した。
- Tracker reconcile 入口(PR #163 → merge commit `bfe7d8f`)が main で実受入
  済み: manual dispatch run `42180639964` が completed/success、共有 pin
  `eed590e` の実 checkout・最小権限・`mode: reconcile` 評価・report
  artifact upload を log で確認済み。

PR 作成/更新・main push では CI は起動しなくなる。検証入口は Daily
Scheduler(`daily-ci.yml` → shared `decide.mjs` → `workflow_dispatch`)+
手動 dispatch に一元化され、失敗追跡は event 型 Tracker(`workflow_run`)
と reconcile 入口(schedule/manual)の両系統で担う。

## Why Now

- #158 変更 5 の実行条件が全て成立した: Required context が空
  (`main-checkstatus` disabled、3 repo とも `required_status_checks` なし)、
  reconcile 入口が main で実受入済み、共有 pin が 3 workflow で
  `eed590e` に整合。
- trigger を残したままでは PR ごとに全 lane(macOS legs 含む)の Full CI
  が起動し続ける — 移行目的(daily 集中 + Required gate 依存の解消)と
  矛盾する。
- これ以上延期しても得るものがない: Required 解除は実確認済みで、
  「PR が CI を起動しなければ merge できない」という deadlock を生む
  構成要素は存在しない。

## Users and Value

- Owner: PR/push のたびの全 lane 起動が止まり、検証証跡が daily/manual
  dispatch run に集約される。Required 空でも PR は即 merge 可能
  (main-protect の PR 必須 + review 0 件運用のまま)。
- 将来の運用者: `ci.yml` の header 記述が「移行中」を自称しなくなり、
  workflow の実態と文書が一致する。

## Scope

### In Scope

- `.github/workflows/ci.yml` の `on:` から `pull_request` / `push` を
  撤去し `workflow_dispatch`(inputs.expected_sha required)のみにする。
- dispatch 専用化に伴い `expected-sha` job の event 分岐
  (`if: github.event_name == 'workflow_dispatch'` の両 step と
  `Record non-dispatch event` no-op step)を撤去し、全 run で
  expected_sha 照合を無条件実行する fail-closed 化。
- header comment の更新(trigger 維持が義務だった移行中記述を最終状態
  へ書き換え。`summary` job 維持理由・concurrency 記述も整合)。
- `tools/check-ci-workflow.mjs` の trigger 契約を反転:
  `workflow_dispatch` 唯一を要求し、`pull_request` / `push` /
  `schedule` / `workflow_run` 等他 trigger を拒否。expected-sha job は
  event 分岐の不在を要求。
- 構造契約 checker の陰性 fixture 検証(trigger 再導入・schedule 混入・
  expected_sha 欠落・event 分岐残存を exit 1 で拒否)。
- doc chain 追加と管理 index 再生成。

### Out of Scope

- `summary` / `ci-summary` job・全 lane・needs・fail-closed 判定の変更
  (job 集合は不変。`summary` 撤去は `ci-summary` Required 化後という
  別条件の延期項目で本変更では触れない)。
- `daily-ci.yml` / `ci-issue-tracker.yml` /
  `ci-issue-tracker-reconcile.yml` / `tools/check-ci-tracker-workflows.mjs`
  の変更(入口分離契約は既に checker が固定済み)。
- Ruleset / branch protection の変更(owner 管理、作業時点で既に
  Required なし実確認済み)。
- 製品 Rust / UI / SDK 実装、依存定義、secret、権限の変更。
- reconcile cron の実観測(未観測は NotRun として扱い manual 実行と
  混同しない)。

## Requirements

- R1: `ci.yml` の `on:` は `workflow_dispatch` のみを持つ。
  `pull_request` / `push` / `schedule` / `workflow_run` /
  `pull_request_target` / `issue_comment` 等他 trigger を持たない。
- R2: `workflow_dispatch.inputs.expected_sha`(required: true)を維持する
  (Daily Scheduler `decide.mjs` の dispatch 契約)。
- R3: `expected-sha` job は全 run で `git rev-parse HEAD` と
  `expected_sha` を無条件照合する(event 分岐・no-op step なし)。
- R4: job 集合・needs・`if:` 条件・fail-closed 集約
  (`ci-summary`/`summary`/`gui-real` not_run 証跡含む)は現行契約の
  まま維持する。macOS legs を含む全 lane が dispatch run で実行する。
- R5: `.github/workflows/` 配下に他の PR/push 起動入口が存在しないこと
  (現状確認: PR event 参照は ci.yml のみ)。
- R6: `tools/check-ci-workflow.mjs` が上記を fail-closed で検査し、
  mise task + prek hook 経路は不変(hygiene job が全 run で実施)。
- R7: merge 後の実検証として、本 PR の作成/更新で Actions run が
  起動しないことを head SHA・check suite・workflow run API で確認する
  (worker は PR 作成まで、API 確認結果を証跡として報告する)。
- R8: `main-protect` の PR 必須ルールと Required 空の組合せが merge を
  deadlock させないことを作業時点で再確認する。

## Acceptance Criteria

- `mise run check-ci-workflow`・actionlint・`prek run --all-files` が
  exit 0。
- 陰性 fixture(ci.yml に pull_request/push/schedule 再導入、
  expected_sha 欠落、event 分岐残存)が全て exit 1。
- PR 作成後、当該 head SHA に対して `ci`/`daily-ci`/
  `ci-issue-tracker`/`ci-issue-tracker-reconcile` の workflow run が
  新規発生しないことを Actions API で確認(run 一覧の createdAt が
  PR 作成以後の当該 SHA で空、check suite が空または PR 非連動)。
- Required status checks が引き続き全 repo で存在しない。

## Deferred Design Concerns

- `summary` job 撤去: `ci-summary` が Required context 化された後の
  別条件延期項目(issue-158 design「先送り」表)。Required 空の現在は
  redundant check として残るが、撤去は本変更の scope 外。
- reconcile cron 実観測: schedule の実発火は GitHub 側の時刻依存であり、
  worker 観測可能範囲では NotRun のまま記録する。
- 他 repo(ruviform/synvelis)の trigger 撤去: 各 repo の作業として扱う。

## Sources and Relations

- Source: AudioLens Issue #147(Ruleset Required 撤去→trigger 撤去順序)、
  #158 変更 5(最終段階として延期)、#159(Tracker 系)、Ruviform #365
  (cross-consumer 移行)。
- Upstream: `docs/designs/audiolens-independent-ci.md`、
  `docs/designs/full-ci-ci-summary-issue-158.md`、
  `docs/designs/ci-tracker-reconcile-issue-159.md`。
