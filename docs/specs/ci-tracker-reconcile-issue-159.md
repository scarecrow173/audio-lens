---
id: "SPEC-034SKTICUsXIT198jBTZXz"
type: "spec"
status: "approved"
title: "Issue #159 Tracker reconcile 入口: GITHUB_TOKEN dispatch 通知 gap を別 workflow 定期/手動 reconcile で埋める"
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
  implements: []
  implemented-by:
    - "../designs/ci-tracker-reconcile-issue-159.md"
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
  refines:
    - "../designs/audiolens-independent-ci.md"
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/audio-lens/issues/159"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ci-tracker-reconcile-issue-159.md"
  references:
    - "https://github.com/scarecrow173/ruviform/blob/eed590e0a3837e61c943d9de1d3efcfd7df32cb0/docs/adr/issue-368-tracker-reconcile-schedule-entry.md"
    - "https://github.com/scarecrow173/ruviform/blob/eed590e0a3837e61c943d9de1d3efcfd7df32cb0/docs/specs/issue-368-tracker-reconcile.md"
    - "https://github.com/scarecrow173/ruviform/pull/380"
  defers: []
  deferred-by: []
---

# Issue #159 Tracker reconcile 入口: GITHUB_TOKEN dispatch 通知 gap を別 workflow 定期/手動 reconcile で埋める

## What Should Be Built

AudioLens の CI failure Issue Tracker に、Ruviform で実装・統合済みの
**reconcile 入口**を consumer として導入する。具体的には:

1. `daily-ci.yml`・`ci-issue-tracker.yml` が固定 checkout する
   Ruviform 共有実装の pin を `73d51d875e2876a75fb2077c471ea8d04478c082`
   から `eed590e0a3837e61c943d9de1d3efcfd7df32cb0`(Ruviform PR #380
   統合後の main、`--reconcile` モードを含む)へ更新する。
2. 別 workflow `.github/workflows/ci-issue-tracker-reconcile.yml`
   (`schedule` + `workflow_dispatch` のみ)を新設し、共有 checkout した
   `tracker.mjs --reconcile` を呼ぶ。
3. 上記の構造契約(共有 pin 一致・reconcile 入口の trigger/guard/
   権限/concurrency・event 入口の不変)を fail-closed で検査する
   `tools/check-ci-tracker-workflows.mjs` を追加し、mise task と
   prek hook へ登録する。

## Why Now

- `daily-ci.yml`(Scheduler)が `GITHUB_TOKEN` で `workflow_dispatch`
  した `ci` run は実行されるが、GitHub の anti-recursion 規約により
  その完了の `workflow_run` event は発火しない。event 入口
  (`ci-issue-tracker.yml`)は bot dispatch の Full CI を**観測できず**、
  失敗は起票されず成功でも Issue が閉じない(gap は対称)。
  Ruviform では bot 成功後に manual Replay まで Issue #375 が open の
  ままだった実績がある(Ruviform spec issue-368-tracker-reconcile)。
- Ruviform PR #380(merged、main `eed590e`)が `tracker.mjs` の
  `--reconcile` モードと同型の reconcile workflow を提供済み。
  accepted ADR `ADR-vJvyZNtrHi2xnycJxbnSPv` は、既存 event 入口
  ファイルへの trigger 追加禁止を維持したまま、**別 workflow** の
  schedule/manual 入口を正式に承認している。AudioLens は同一
  evaluator を固定 commit から共有 checkout する consumer として
  同じ gap 解消を導入できる。
- ユーザー指示により CI 移行の reconcile 展開が先行タスクであり、
  PR/push trigger 撤去は本 workflow の main 実受入後の後段工程。

## Users and Value

- **AudioLens Owner/開発者**: Scheduler が dispatch した Full CI の
  失敗・回復が人の手動 Replay なしに失敗 Issue へ反映される
  (起票漏れ・open 残りの両方を防ぐ)。
- **Daily Scheduler(#159 既導入)**: 「dispatch した run の完了は
  tracker に届かない」構造的制約を、別入口の定期 reconcile が埋める。
- **運用者**: `workflow_dispatch` による即時 reconcile が診断・
  手動復旧経路として使える。

## Scope

### In Scope

- 共有 pin 更新: `daily-ci.yml`・`ci-issue-tracker.yml` の
  `scarecrow173/ruviform` checkout ref を
  `eed590e0a3837e61c943d9de1d3efcfd7df32cb0` へ揃える。
- `ci-issue-tracker-reconcile.yml` 新設:
  - trigger は `schedule`(UTC・分 != 0)と `workflow_dispatch` のみ。
    `pull_request`/`push`/`workflow_run` は持たない。
  - 既存 event 入口と**同一 concurrency group**
    (`ci-issue-tracker-${{ github.repository }}`・
    `cancel-in-progress: false`)で Issue 書き込みを直列化する。
  - reconcile job のみ `contents:read`/`actions:read`/`issues:write`
    (既存 track job と同一最小権限)。workflow 直下 `permissions: {}`。
  - job `if`:
    `github.event_name == 'schedule' || github.ref_name == github.event.repository.default_branch`。
    schedule は payload に `repository` が欠落し得るため無条件通過
    (ref 比較が `main == ''` に解決して cron 全体を silent skip
    させない)。`workflow_dispatch` の非 default ref 起動は job を
    skip する防御としてのみ効く。
  - AudioLens repo の checkout は
    `ref: ${{ github.event.repository.default_branch || 'main' }}`
    で default branch code に pin し、
    `persist-credentials: false`(trusted default checkout)。
  - 共有実装は `scarecrow173/ruviform` を pin SHA
    `eed590e0a3837e61c943d9de1d3efcfd7df32cb0` で
    `.ci-shared/ruviform` へ `ssh-key: secrets.RUVIFORM_DEPLOY_KEY`
    checkout(既存経路・新規 secret なし)。
  - `timeout-minutes: 15`。実行前に
    `node --test .ci-shared/ruviform/ci/tracker/` で共有判定器を
    自己検証し、`GH_TOKEN` のみ渡して
    `tracker.mjs --reconcile --report ci-issue-tracker-report.json`
    を実行(`TRACKER_EVENT_PATH` は渡さない)。report artifact を
    `if: always()` で upload。
- `tools/check-ci-tracker-workflows.mjs`: 共有 pin の全 workflow
  一致・reconcile 入口契約・event 入口不変を静的に検査し、mise
  task `check-ci-tracker-workflows` と prek hook で全 run が実施
  する形に登録する。
- 本 spec + design/plan/test-spec/task チェーンと生成 index の同期。

### Out of Scope

- `ci-issue-tracker.yml` への trigger 追加・構造変更(`workflow_run`
  入口は pin 更新のみで維持。cron は持たせない)。
- `ci.yml` の `pull_request`/`push` trigger 撤去(main での実受入後の
  後段工程)、`daily-ci.yml` の trigger・判定変更。
- Ruviform 共有実装(`tracker.mjs` 等)自体の変更・vendor 複製・
  RF Rust/Frontend build 依存の追加。
- 新規 secret・権限増量・`RUVIFORM_DEPLOY_KEY` 以外の credential。
- Ruleset `main-checkstatus`/`main-protect` の変更(#147、Owner 作業)。
- 実 cron・実 Issue・実 dispatch の起動受入(merge 後の運用段階。
  本 spec は offline 検証まで)。
- Synvelis への展開(別 repo・別 worker)。

## Requirements

- R1: 全 3 workflow(`daily-ci`・`ci-issue-tracker`・
  `ci-issue-tracker-reconcile`)の Ruviform 共有実装 pin が同一 SHA
  `eed590e0a3837e61c943d9de1d3efcfd7df32cb0` で一致する。
- R2: reconcile workflow は `schedule` + `workflow_dispatch` のみを
  持ち、event 入口 `ci-issue-tracker.yml` の trigger・job・権限・
  concurrency は変更しない(pin の1行のみ変更)。
- R3: reconcile job は schedule を無条件通過しつつ dispatch の非
  default ref を skip する `if:` を持ち、checkout は常に default
  branch code に pin される(schedule payload の `repository` 欠落で
  silent skip しない)。
- R4: 権限は既存 track job と同一最小権限。新規 secret・権限・
  外部サービスを作らない。共有 checkout は既存
  `RUVIFORM_DEPLOY_KEY` を継続利用する。
- R5: `concurrency` は event 入口と同一 group で直列化し、
  `cancel-in-progress: false`。reconcile は冪等なので pending 置換で
  落ちても次回 tick/event が同じ権威履歴に収束する。
- R6: 実行は `--reconcile` 明示 + `TRACKER_EVENT_PATH` 非付与。
  report/Job Summary の `mode` が `reconcile` と記録される入口契約を
  維持する(共有 evaluator が mode を出力する)。
- R7: `timeout-minutes: 15` で hang が共有 concurrency group を
  占有し続けるのを防ぐ。
- R8: cron は UTC・分 != 0 で、daily-ci の dispatch 時刻(21:23 UTC)
  に続く検知を 6 時間周期で行う。決定根拠は design に記録する。
- R9: 構造契約テストは既存 `check-ci-workflow.mjs` と同じく依存
  ゼロ・indentation 構造検査で、負例(fixture)検出を含む。
  既存テストを削らない。
- R10: 全 CI lane・`ci-summary`/`summary` の fail-closed 契約・
  選択実行機構の不在は不変(既存 check が担保)。

## Acceptance Criteria

- [ ] `daily-ci.yml`・`ci-issue-tracker.yml`・
      `ci-issue-tracker-reconcile.yml` の pin が eed590e で一致し、
      `mise run check-ci-tracker-workflows` が pass する。
- [ ] `ci-issue-tracker-reconcile.yml` が schedule +
      workflow_dispatch のみ・同一 concurrency group・job guard・
      default-branch pin checkout・timeout 15・`--reconcile` 実行を
      持ち、`mise exec -- actionlint` が pass する。
- [ ] `ci-issue-tracker.yml` の trigger が `workflow_run` のみのまま
      (cron 追加なし)であり、check tool が違反を検出する。
- [ ] `mise exec -- node --test`(共有 checkout 相当の tracker/daily/
      summary suite)が pass する(共有 pin の evaluator 実測)。
- [ ] `mise exec -- prek run --all-files`・`mise run check-ci-workflow`
      が pass し、ci.yml に差分がない。
- [ ] 実 GitHub API・実 Issue・実 dispatch・実 cron を起こさない
      (offline 検証のみ。実受入は merge 後・別工程)。

## Deferred Design Concerns

- reconcile cron の実観測 | schedule は merge 後に初めて発火する。
  未観測の cron を成功としない | main 実受入工程(coordinator)で
  実 run を確認 | 未観測のまま受入完了扱いにすると gap 再発を
  見落とす
- schedule auto-disable(60 日無活動)の監視 | reconcile workflow が
  止まっても event 入口は残る設計。`gh workflow enable` による
  再有効化手順は共有 README に記録済み | 運用観測で判断 | 変更不要
- PR/push trigger 撤去 | Required 解除確認 + 本入口の実受入が前提。
  #147/#159 の後段工程 | coordinator 判断 | 先行撤去は検証経路を
  失わせる

## Sources and Relations

- Issue [#159](https://github.com/scarecrow173/audio-lens/issues/159):
  Scheduler/Tracker 導入 Issue。本変更は Tracker の第二入口追加。
- Issue [#158](https://github.com/scarecrow173/audio-lens/issues/158)・
  [#147](https://github.com/scarecrow173/audio-lens/issues/147):
  Full CI 契約と Ruleset 移行(trigger 撤去は後段)。
- Ruviform 横断計画 [#365](https://github.com/scarecrow173/ruviform/issues/365)、
  Scheduler [#367](https://github.com/scarecrow173/ruviform/issues/367)、
  Tracker [#368](https://github.com/scarecrow173/ruviform/issues/368)。
- Ruviform accepted ADR `ADR-vJvyZNtrHi2xnycJxbnSPv`
  (issue-368-tracker-reconcile-schedule-entry.md): event 入口 trigger
  禁止の補足として別 workflow schedule/manual 入口を承認した決定権。
  本 spec はその consumer 側適用であり、ADR 本文を書き換えない。
- Ruviform spec `issue-368-tracker-reconcile.md`・PR #380
  (main `eed590e`): 共有 evaluator の `--reconcile` 契約・guard・
  workflow 骨格の正本。
- [audiolens-independent-ci](../designs/audiolens-independent-ci.md):
  権限最小・secret 取り扱い・fail-closed の不変条件を継承する。
- [ruviform-dependency-ci](../designs/ruviform-dependency-ci.md):
  共有 checkout・SHA 固定・Deploy Key 経路の設計権。
