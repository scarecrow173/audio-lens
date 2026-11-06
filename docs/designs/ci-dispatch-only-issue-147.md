---
id: "DESIGN-034c5j6d48zGZdh9KX0KM"
type: "design"
status: "approved"
title: "ci.yml workflow_dispatch 専用化: PR/push trigger 撤去と expected-sha 無条件照合"
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
  implements:
    - "../specs/ci-dispatch-only-issue-147.md"
  implemented-by:
    - "../plans/implement-ci-dispatch-only-issue-147.md"
  depends-on:
    - "../designs/audiolens-independent-ci.md"
    - "../designs/full-ci-ci-summary-issue-158.md"
    - "../designs/ci-tracker-reconcile-issue-159.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "https://github.com/scarecrow173/audio-lens/issues/158"
    - "https://github.com/scarecrow173/audio-lens/issues/159"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/ci-dispatch-only-issue-147.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ci-dispatch-only-issue-147.md"
  references: []
  defers: []
  deferred-by: []
---

# ci.yml workflow_dispatch 専用化: PR/push trigger 撤去と expected-sha 無条件照合

## Context

Issue #158 設計で「`pull_request`/`push` trigger 削除」は最終段階へ延期
された。延期条件は Ruleset Required 解除の Owner 確認と、Tracker 通知
gap の代替手段(reconcile 入口)の main 実受入であった。本設計時点で
両条件は実証済み:

- 3 repo(audio-lens/ruviform/synvelis)の effective rules を実 API で
  再確認: `main-checkstatus` ruleset は disabled、active の
  `main-protect` は `deletion`/`non_fast_forward`/`pull_request`(承認
  必須 0 件)のみを持ち `required_status_checks` を含まない。classic
  branch protection も未設定(404)。
- PR #163 取込後の main manual dispatch run `42180639964` が
  completed/success — `eed590e` 実 checkout・`mode: reconcile`・
  pending-run noop・report artifact upload を log 確認済み。

`ci.yml` の trigger を `workflow_dispatch` のみにすることで、PR 作成/
更新・main push では CI が起動しなくなり、Full CI の起動権は Daily
Scheduler(`decide.mjs` → workflow_dispatch)と人手 dispatch に一元化
される。

## Scope

### In Scope

- `ci.yml` `on:` から `pull_request`/`push` 撤去、`workflow_dispatch`
  (expected_sha required)のみ維持。
- `expected-sha` job: `github.event_name == 'workflow_dispatch'` の
  `if:` 2 箇所と `Record non-dispatch event` step を撤去し、
  checkout + 照合を無条件実行。
- header comment を最終状態に更新(移行中記述・PR/push 維持義務・
  expected-sha no-op 記述・concurrency コメントを整合)。
- `tools/check-ci-workflow.mjs` trigger 契約反転 + expected-sha 無条件
  契約化 + header 契約記述更新。
- doc chain + index 再生成。

### Out of Scope

- `summary`/`ci-summary`/lane job・needs・fail-closed 判定の変更
  (job 集合は完全に不変)。
- 他 3 workflow・他 checker・Ruleset・製品コード・依存・secret・権限。

## Components and Boundaries

### 1. trigger 契約

```yaml
on:
  workflow_dispatch:
    inputs:
      expected_sha:
        description: "Dispatch 判定時の対象 commit SHA(40 hex)。checkout revision と不一致なら expected-sha job が run を失敗にする"
        required: true
        type: string
```

- `pull_request`/`push` を撤去。他の一切の event も持たない。
- expected_sha は Daily Scheduler(shared `decide.mjs`)が dispatch
  時に渡す契約で required のまま。手動 dispatch では 40-hex を
  入力する。

### 2. expected-sha job の無条件照合

現行は `if: github.event_name == 'workflow_dispatch'` で照合を実行し、
非 dispatch では `Record non-dispatch event` が no-op success を返す
設計だった(DAG を event 非依存に保つため)。

dispatch 専用化で `github.event_name` は常に `workflow_dispatch` となる
ため、両 `if:` と no-op step は dead code — 撤去し、checkout(dispatch
guard 用)+ `git rev-parse HEAD` vs `expected_sha` 照合を無条件実行
する。

fail-closed 観点では強化になる: 将来 trigger を誤って再導入しても、
expected_sha を持たない event run は即失敗する(空文字が 40-hex 正規
表現に合わない)。checker が trigger 再導入も拒否するため二重に
閉じる。

### 3. 維持する要素(変更しない)

- job 集合 12 個・needs・lane `if:`・`ci-summary`/`summary` の
  always()+fail-closed・`gui-real` not_run 証跡・`hygiene` の
  `prek run --all-files`・`ruviform-access` mode 解決。
- `concurrency`(group=workflow-ref、main では cancel しない):
  dispatch は任意 ref に打てるため現行式のまま意味を持つ。
  コメントのみ「PR 重複 cancel」「main push」表現を dispatch 実態
  に合わせる。
- `summary` job: Required context `summary` 互換 gate として残す。
  Required 空の現在は redundant だが、撤去条件(#158 design:
  `ci-summary` Required 化)は未達成で、残すことで owner が将来
  Required を再有効化しても deadlock にならない。
- `ci-issue-tracker.yml`(workflow_run): ユーザー起因の dispatch run
  完了を引き続き観測する(bot dispatch は従来通り workflow_run を
  発火させないが、それは reconcile 入口が補う設計済みの gap)。

### 4. `check-ci-workflow.mjs` 契約更新

- trigger: `on:` の 2-indent trigger key を列挙し、集合が
  `{workflow_dispatch}` と厳密一致することを要求
  (pull_request/push/schedule/workflow_run 等の混入を全て拒否)。
- `workflow_dispatch.inputs.expected_sha(required: true)` 必須は維持。
- expected-sha job: `git rev-parse HEAD` + `expected_sha` 参照は維持
  しつつ、`if:` が `github.event_name` を参照しないこと(dispatch
  専用下で照合の無条件実行を保証)を要求。
- その他の契約(job 集合・needs・if:・fail-closed・残滓禁止・
  run 本文入力展開禁止)は不変。

## Data and Control Flow

起動経路(変更後):

- `daily-ci.yml`(cron 21:23 UTC / manual)→ shared `decide.mjs` →
  `workflow_dispatch` of `ci.yml` with expected_sha → Full CI 全 lane。
- 手動 `gh workflow run ci.yml -f expected_sha=<40hex>` → 同上。
- `ci-issue-tracker.yml`: `workflow_run` completed(main)で event 型
  評価(ユーザー dispatch run のみ届く)。
- `ci-issue-tracker-reconcile.yml`: 6h schedule/manual で権威履歴を
  reconcile(bot dispatch run 含む全履歴対象)。
- PR 作成/更新・main push: CI 無起動。`main-protect` が PR 必須を
  課すため merge 経路は PR 経由のまま、Required 空で gate 滞留なし。

## Non-Functional Constraints

- deadlock 非発生: required_status_checks なし × PR 必須 ruleset の
  組合せは merge 可能(実 API 確認済み、作業時点でも再確認)。
- fail-closed 強化: expected_sha 照合の無条件化・trigger 集合の
  厳密一致 checker。
- 検証コスト: PR/push 全 lane 起動が消える反面、daily cadence 以外の
  検証は手動 dispatch になる(owner の意図した trade-off、#365)。
- cron 実観測は引き続き未検証(NotRun、manual 実行と混同しない)。

## Risks and Trade-offs

- PR で CI が走らないため、PR レビュー時点での検証証跡はない →
  受入は merge 後の daily/reconcile 系に委ねる設計(#365 横断決定)。
  `main-protect` の PR 必須 + 0 承認で merge 可能なため、検証強制は
  ruleset ではなく運用(Daily Full CI)で担う。
- 誤って trigger を再導入した場合: checker が fail、かつ expected_sha
  無条件照合で event run も失敗する二重防御。
- fork からの PR: 元々 secrets 非依存の構成。trigger 不在のため
  fork PR は一切 CI を起動しない(安全側)。

## Deferred Design Concerns

- `summary` job 撤去(条件: `ci-summary` Required 化)。
- reconcile cron 実観測。
- 他 repo の trigger 撤去。

## Verification Notes

- 構造契約: `mise run check-ci-workflow` exit 0 + 陰性 fixture
  (trigger 再導入・schedule 混入・expected_sha 欠落・event 分岐残存)
  全て exit 1。
- 文法: actionlint exit 0。衛生: `prek run --all-files` exit 0。
- 実 GitHub 検証(worker 実施): PR 作成/更新で `ci`/`daily-ci`/
  `ci-issue-tracker`/`ci-issue-tracker-reconcile` の run が当該 head
  SHA で発生しないことを `gh run list --workflow` + check suite API
  で確認し、PR 作成時刻との時系列証跡を報告する。
- 未検証のまま残るもの: cron 実発火、merge 後の daily dispatch 実
  動作(次の 21:23 UTC 以降)、summary check の実効(Required で
  ないため観測のみ)。
