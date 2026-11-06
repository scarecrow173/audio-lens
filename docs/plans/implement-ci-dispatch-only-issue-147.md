---
id: "PLAN-034Lgf6kv61kqUvJK826Z"
type: "plan"
status: "approved"
title: "ci.yml workflow_dispatch 専用化 実装計画(trigger 撤去 + checker 反転 + PR 無起動実検証)"
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
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
  implements:
    - "../specs/ci-dispatch-only-issue-147.md"
    - "../designs/ci-dispatch-only-issue-147.md"
  implemented-by:
    - "../tasks/ci-dispatch-only-issue-147.md"
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
    - "../designs/ci-dispatch-only-issue-147.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ci-dispatch-only-issue-147.md"
  references: []
  defers: []
  deferred-by: []
---

# ci.yml workflow_dispatch 専用化 実装計画

## Steps

1. **前提実検証(済)** — reconcile run `42180639964` の log/artifact で
   `mode: reconcile`・`eed590e` 実 checkout・最小権限・report upload
   を確認。3 repo の rulesets/branch protection を実 API で再確認
   (`main-checkstatus` disabled、`main-protect` active・
   required_status_checks なし)。merge deadlock 要素なし。
2. **新 branch `fix/al-ci-manual-only`** を merge 済み main
   (`bfe7d8f`)から作成(済)。
3. **`ci.yml` 編集** — `on:` から `pull_request`/`push` を削除。
   `expected-sha` job の `if: github.event_name == 'workflow_dispatch'`
   2 箇所と `Record non-dispatch event` step を削除し、checkout +
   照合を無条件化。header comment を最終状態へ更新(trigger 維持
   義務・no-op 記述・concurrency の PR/push 言及を除去・`summary`
   維持理由を残す)。
4. **`tools/check-ci-workflow.mjs` 更新** — header 契約記述を更新、
   trigger 検査を「trigger key 集合 == {workflow_dispatch}」の厳密
   一致へ反転、expected-sha job に `github.event_name` を参照する
   `if:` が無いことを要求。
5. **doc chain** — 本 plan と spec/design/test-spec/task を作成、
   `doc_maintenance apply` で管理 index 再生成。
6. **検証** — `mise run check-ci-workflow`、actionlint、
   `check-ci-tracker-workflows`(無変更で pass のはず)、
   `prek run --all-files`。陰性 fixture 5 本(pull_request 再導入/
   push 再導入/schedule 混入/expected_sha 欠落/event 分岐残存)を
   worktree ローカル一時 dir で exit 1 確認。
7. **review-protocol 独立レビュー** — cooperative + adversarial 両
   pass、指摘を修正して再検証。
8. **commit/push/PR** — logical commit(ci.yml+checker / docs)、
   `git push -u origin fix/al-ci-manual-only`、日本語 PR 作成。
9. **PR 無起動実検証** — head SHA・PR 作成時刻を記録し、
   `gh run list --workflow=<each>`(branch/SHA・createdAt フィルタ)
   と check suite API で CI run 0 件を確認。PR 本文・worker_done に
   証跡を記載。

## Verification

| 検査 | コマンド | 期待 |
| --- | --- | --- |
| 構造契約 | `mise run check-ci-workflow` | exit 0 |
| tracker 契約(不変) | `mise run check-ci-tracker-workflows` | exit 0 |
| 文法 | `mise exec -- actionlint .github/workflows/ci.yml` | exit 0 |
| 衛生 | `mise exec -- prek run --all-files` | exit 0 |
| 陰性 1: pull_request 再導入 | `node tools/check-ci-workflow.mjs <fixture>` | exit 1 |
| 陰性 2: push 再導入 | 同上 | exit 1 |
| 陰性 3: schedule 混入 | 同上 | exit 1 |
| 陰性 4: expected_sha 欠落 | 同上 | exit 1 |
| 陰性 5: event 分岐残存 | 同上 | exit 1 |
| 実 GitHub | `gh run list --workflow ci.yml --branch <head>` + check suite API | 当該 SHA で run 0 件 |

## Done When

- 上表全てが実測で期待通り。
- PR 作成/更新で CI workflow run が当該 head SHA で発生しないことを
  API 証跡で確認済み。
- Required status checks が全 repo で引き続き存在しないことを実 API
  再確認済み。
- 日本語 PR が作成済み(template: Summary + Test plan + 未実施項目)。

## Notes

- `summary` job・全 lane・needs・fail-closed は一切変更しない。
- merge/Issue close/main push/force push/git config/削除は禁止。
- cron 実観測は NotRun のまま(manual 実行と混同しない)。
