---
id: "DESIGN-034SKTJUE6Wsd8z241uUcC"
type: "design"
status: "approved"
title: "Issue #159 Tracker reconcile 入口(consumer 展開): 共有 pin 更新と別 workflow schedule/manual 入口"
created: "2026-11-06"
updated: "2026-11-06"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/159"
  changes:
    added:
      - type: "file"
        path: ".github/workflows/ci-issue-tracker-reconcile.yml"
      - type: "file"
        path: "tools/check-ci-tracker-workflows.mjs"
    modified:
      - type: "file"
        path: ".github/workflows/daily-ci.yml"
      - type: "file"
        path: ".github/workflows/ci-issue-tracker.yml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".pre-commit-config.yaml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/ci-tracker-reconcile-issue-159.md"
  implemented-by:
    - "../plans/implement-ci-tracker-reconcile-issue-159.md"
  depends-on:
    - "../designs/audiolens-independent-ci.md"
    - "../designs/ruviform-dependency-ci.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "https://github.com/scarecrow173/ruviform/issues/367"
    - "https://github.com/scarecrow173/ruviform/issues/368"
    - "https://github.com/scarecrow173/audio-lens/issues/147"
    - "https://github.com/scarecrow173/audio-lens/issues/158"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/ci-tracker-reconcile-issue-159.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ci-tracker-reconcile-issue-159.md"
  references:
    - "https://github.com/scarecrow173/ruviform/blob/eed590e0a3837e61c943d9de1d3efcfd7df32cb0/docs/adr/issue-368-tracker-reconcile-schedule-entry.md"
    - "https://github.com/scarecrow173/ruviform/blob/eed590e0a3837e61c943d9de1d3efcfd7df32cb0/.github/workflows/ci-issue-tracker-reconcile.yml"
    - "https://github.com/scarecrow173/ruviform/pull/380"
  defers: []
  deferred-by: []
---

# Issue #159 Tracker reconcile 入口(consumer 展開): 共有 pin 更新と別 workflow schedule/manual 入口

## Context

AudioLens は Ruviform の共通 CI 実装(`ci/daily`・`ci/tracker`・
`ci/summary`)を、固定 SHA の `actions/checkout` で `.ci-shared/ruviform`
に取得して再利用する consumer(ruviform-dependency-ci・#161/#162)。
判定ロジックは複製せず、workflow 側は checkout・toolchain 復元・
自己検証・実行・report upload の軽量骨格のみを持つ。

Ruviform PR #380(main `eed590e0a3837e61c943d9de1d3efcfd7df32cb0`)で
`tracker.mjs --reconcile` モードと RF 側 reconcile workflow が統合
され、consumer 展開が可能になった。本設計はその consumer 側適用で
あり、**アーキテクチャ決定権は RF の accepted ADR
`ADR-vJvyZNtrHi2xnycJxbnSPv`**(event 入口 trigger 禁止を維持した
まま別 workflow の schedule/manual 入口を承認)にある。本設計は
ADR を書き換えず、consumer の既存 checkout パターンへの適合形だけを
定める。

差分(RF → AL): RF は自 repo に `ci/tracker/` を持つため checkout が
1 段・self-test は `mise run test-ci-tracker`。AL は共有 checkout が
必要で、self-test は `node --test .ci-shared/ruviform/ci/tracker/` を
直接呼ぶ既存形を踏襲する。判定器・権限・guard・concurrency の契約は
同一。

## Scope

### In Scope

- `daily-ci.yml`・`ci-issue-tracker.yml` の共有 pin を
  `eed590e0a3837e61c943d9de1d3efcfd7df32cb0` へ更新(各 1 行)。
- `.github/workflows/ci-issue-tracker-reconcile.yml` 新設
  (schedule + workflow_dispatch のみ)。
- `tools/check-ci-tracker-workflows.mjs` 新設 + mise task
  `check-ci-tracker-workflows` + prek hook 登録。
- doc チェーン・生成 index の同期。

### Out of Scope

- `ci-issue-tracker.yml` の trigger/構造変更、`ci.yml` の trigger
  変更、`daily-ci.yml` の判定変更。
- 共有実装・製品コード・依存の変更。新規 secret/権限。
- 実 cron/実 Issue/実 dispatch 受入(merge 後)。

## Components and Boundaries

### 1. 共有 pin の整合

- 3 ファイル(`daily-ci.yml`・`ci-issue-tracker.yml`・
  `ci-issue-tracker-reconcile.yml`)の
  `repository: scarecrow173/ruviform` checkout `ref:` は常に**同一の
  完全 SHA**。評価器と Scheduler/Tracker の版ずれを構造契約テストで
  防ぐ。pin の更新は 3 箇所同時が規範であり、単独更新は check tool が
  検出して失敗する。

### 2. `ci-issue-tracker-reconcile.yml` の構造

骨格は RF 版に一致させ、checkout 段のみ consumer 形に適合させる:

```yaml
name: ci-issue-tracker-reconcile
on:
  schedule:
    - cron: "23 */6 * * *"
  workflow_dispatch:
permissions: {}
concurrency:
  group: ci-issue-tracker-${{ github.repository }}
  cancel-in-progress: false
env:
  MISE_ENABLE_TOOLS: node
jobs:
  reconcile:
    runs-on: ubuntu-latest
    if: github.event_name == 'schedule' || github.ref_name == github.event.repository.default_branch
    timeout-minutes: 15
    permissions:
      contents: read
      actions: read
      issues: write
    steps:
      - Checkout repository (default branch tracker code)
        ref: ${{ github.event.repository.default_branch || 'main' }}
        persist-credentials: false
      - Checkout shared CI implementation
        repository: scarecrow173/ruviform
        ref: eed590e0a3837e61c943d9de1d3efcfd7df32cb0
        path: .ci-shared/ruviform
        ssh-key: ${{ secrets.RUVIFORM_DEPLOY_KEY }}
        persist-credentials: false
      - Set up mise / Restore node toolchain from mise.lock
      - Record shared CI implementation revision
      - Self-test shared tracker evaluator
        run: mise exec -- node --test .ci-shared/ruviform/ci/tracker/
      - Reconcile authoritative run history and update failure issue
        env: GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: mise exec -- node .ci-shared/ruviform/ci/tracker/tracker.mjs --reconcile --report ci-issue-tracker-report.json
      - Upload tracker report (if: always())
        name: ci-issue-tracker-reconcile-report-attempt-${{ github.run_attempt }}
```

設計上の契約点:

- **trigger 分離**: `workflow_run`/`pull_request`/`push` を持たない。
  cron は本ファイルに閉じ込め、GitHub の workflow 単位 auto-disable
  (public repo 60 日無活動・disabled は全 trigger 不応答)の blast
  radius を event 入口から分離する(RF ADR の Decision Drivers と同一)。
- **guard `if:`**: schedule は payload に `repository` が欠落し得る
  ため `github.event_name == 'schedule'` で無条件通過させ、ref 比較を
  評価させない(`github.ref_name == github.event.repository.default_branch`
  が `main == ''` に解決され、cron 全 tick を silent skip させる
  事故を防ぐ — RF ADR 確定契約)。dispatch の非 default ref 起動のみ
  job skip する。
- **trusted default checkout**: AudioLens repo の checkout は
  `github.event.repository.default_branch || 'main'` に pin する。
  空式解決時の triggering-ref fallback を `'main'` 既定値で防ぎ、
  dispatch 選択 ref・完了 run の head・PR code を実行しない信頼
  モデルを維持する。`persist-credentials: false` は既存形どおり。
- **最小権限**: job は `contents:read`/`actions:read`/`issues:write`
  のみ(event track job と同一)。workflow 直下 `permissions: {}` で
  default 権限を持たせない(AL 既存 convention)。
- **concurrency**: event 入口と同一 group
  (`ci-issue-tracker-${{ github.repository }}`)で Issue 書き込みを
  直列化。`cancel-in-progress: false` — tracker 書き込み途中の cancel
  を避ける。pending 置換で reconcile が落ちても evaluator は冪等で
  あり、次 tick/event が同一権威履歴に収束する。
- **timeout 15**: hang が共有 concurrency group を占有して event 入口
  の pending 置換 window を広げるのを防ぐ(RF 版と同一値)。
- **mode 分離**: `--reconcile` を明示し、`TRACKER_EVENT_PATH` を env
  に渡さない(evaluator は reconcile モードで ambient
  `GITHUB_EVENT_PATH` も無視する。`--event` 同時指定は usage error)。
  report JSON/Summary の `mode: reconcile` が入口識別の証跡。
- **self-test 先行**: 共有 evaluator の mock client contract test を
  job 内で先に走らせ、壊れた pin が Issue を誤操作するのを防ぐ
  (event 入口と同一手順・`if-no-files-found: warn` の report upload
  も同一形)。

### 3. cron 決定根拠

`"23 */6 * * *"`(00:23/06:23/12:23/18:23 UTC、6 時間周期)を採用:

- `daily-ci.yml` の cron は `"23 21 * * *"`(21:23 UTC)。bot Full CI
  は dispatch 後おおよそ 1–2 時間で完了するため、直後の reconcile
  tick(00:23)で典型ケースは dispatch から約 1–3 時間以内に回収
  される。最悪でも 1 間隔(6 時間)以内。
- RF 側は daily `17 21 * * *` に対し同じ `23 */6 * * *` を採用して
  おり、AL の 21:23 dispatch に対しても同一の post-dispatch 窓
  関係が成立する。分 = 23 は daily-ci(21:23)の実行と重ならず、
  毎時 0 分も避ける(GitHub cron の集中回避慣習)。
- RF と tick を**あえてずらす必要はない**: reconcile は各 repo の
  runner と REST metadata のみを使い、repo をまたぐ共有資源・同時刻
  負荷の競合がない。同一 cadence は consumer 間で運用理解を共通化
  でき、判定器と同じく「契約を複製しない」方針に整合する。頻度は
  契約ではなく運用パラメータであり、観測後に調整可能。

### 4. `tools/check-ci-tracker-workflows.mjs`

`check-ci-workflow.mjs` と同方式(依存ゼロ・YAML indentation 構造
検査・任意文字列 grep ではなく block 単位照合)で、3 つの tracker 系
workflow の構造契約を fail-closed 検査する:

- 共有 pin 一致: 全対象ファイルの `repository: scarecrow173/ruviform`
  checkout `ref:` が同一 40hex であること(値そのものは現在 pin と
  照合 — pin はこのファイルで正本を持ち、workflow 側のずれを検出)。
- reconcile 入口契約: trigger は `schedule`+`workflow_dispatch` のみ
  (workflow_run/pull_request/push 不在)、cron が UTC 分!=0、
  job `if:` の schedule 無条件 + dispatch ref guard、concurrency
  group が event 入口と同一、permissions が同一最小権限、
  `timeout-minutes`、default-branch pin checkout、
  `--reconcile` 実行 + `TRACKER_EVENT_PATH` 非付与、self-test 先行。
- event 入口不変: `ci-issue-tracker.yml` の trigger が
  `workflow_run` のみ(cron/workflow_dispatch/schedule 不在)で
  あることを検査し、「event 入口に cron を足す」回帰を防ぐ。
- 負例検出: `--expect-fail <path>` 的な負例 fixture 経路は取らず、
  同ファイル内の自己検証(tests 節)は持たない — 契約違反は全件
  列挙して exit 1(check-ci-workflow.mjs と同じ形式)。

### 5. 登録点

- `mise.toml`: `[tasks.check-ci-tracker-workflows]` を追加
  (`node tools/check-ci-tracker-workflows.mjs`)。
- `.pre-commit-config.yaml`: local hook `check-ci-tracker-workflows`
  を追加。`files:` は 3 workflow + 本 script、`pass_filenames: false`
  (check-ci-workflow hook と同形)。hygiene job(`prek run
  --all-files`)経由で全 run が実施する。

## Data and Control Flow

- schedule tick(default branch の workflow)または main ref への手動
  dispatch → `reconcile` job → default branch checkout → 共有実装
  checkout(pin) → mise/node 復元 → 共有 tracker suite 自己検証 →
  `tracker.mjs --reconcile` が `actions/workflows/ci.yml/runs?branch=
  main` の権威履歴 + marker 状態を reconcile → fail-closed guard 通過時
  のみ失敗 Issue を create/update/close → report artifact upload。
- evaluator 内部(履歴取得・3 分類・compare 2 重 guard・Issue 冪等)は
  Ruviform 側の責務であり、本設計は入口の結線のみを持つ。

## Non-Functional Constraints

- 秘密情報: `RUVIFORM_DEPLOY_KEY` は `actions/checkout` の `ssh-key`
  input としてのみ渡す(既存経路)。`GH_TOKEN` は env のみ・ログへ出さ
  ない。third-party action は `actions/checkout`・`jdx/mise-action`・
  `actions/upload-artifact` のみ。
- 信頼境界: schedule/dispatch いずれでも実行コードは default branch
  + 固定 SHA の共有実装のみ。完了 run のコード・artifact・PR head は
  実行しない。
- fail-closed: 履歴 API 失敗・unknown/pending evidence は evaluator
  側で close を止める(共有 guard)。workflow 側は silent skip しない
  (guard で明示 skip か、実行して失敗)。
- 冪等性: reconcile は繰り返し安全(同一権威履歴→同一 plan)。
  pending 置換による run 喪失は次回 tick が補う。

## Risks and Trade-offs

- cron 未観測リスク: schedule の実発火は merge 後でないと確かめ
  られない。Job Summary/artifact と Actions 一覧で main 実受入時に
  確認する(本設計の受入は offline まで)。
- auto-disable 残存リスク: 60 日無活動で reconcile workflow が止まり
  得るが、event 入口は分離済みで残る。再有効化手順は共有 README
  (`ci/tracker/README.md`)に記録済みのため AL 側に運用 doc 複製は
  作らない。
- 権限は issues:write を持つが、実行対象は trusted code のみ・
  untrusted 値は evaluator が sanitize 済み(RF 側契約)。
- 別 workflow が増えることで Actions 一覧は 1 件増えるが、命名で
  event 入口と区別できる(`ci-issue-tracker-reconcile`)。

## Deferred Design Concerns

- reconcile cron の実観測 | merge 後に初めて発火 | main 実受入工程
  (coordinator)で実 run 確認 | 未観測を成功扱いしない
- PR/push trigger 撤去・Required 整合 | #147/#159 後段工程 |
  Required 解除確認 + 本入口実受入後 | 先行すると検証経路を失う
- schedule auto-disable の監視自動化 | 外部監視は現状不要 |
  運用観測で要否判断 | reconcile 停止時は手動再有効化

## Verification Notes

- [ ] `mise run check-ci-tracker-workflows` pass(新規構造契約)。
- [ ] `mise exec -- actionlint` 全 workflow pass。
- [ ] `mise exec -- prek run --all-files` pass(新 hook 実施を含む)。
- [ ] `mise run check-ci-workflow` pass(ci.yml 不変の確認)。
- [ ] 共有 pin eed590e の `ci/tracker`・`ci/daily`・`ci/summary`
      suite を `node --test` で実測 pass(共有 evaluator の実測)。
- [ ] `git diff` で `ci-issue-tracker.yml` は pin 1 行のみ・
      `daily-ci.yml` も pin 1 行のみ・`ci.yml` は無差分。
