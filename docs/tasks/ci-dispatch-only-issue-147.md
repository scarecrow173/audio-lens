---
id: "TASK-034phtbniZlSYvFqbrIDH"
type: "task"
status: "done"
title: "ci.yml workflow_dispatch 専用化(AL#147/#158 最終段階): PR/push trigger 撤去"
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
    - "../plans/implement-ci-dispatch-only-issue-147.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-ci-dispatch-only-issue-147.md"
    - "../designs/ci-dispatch-only-issue-147.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "https://github.com/scarecrow173/audio-lens/issues/158"
    - "https://github.com/scarecrow173/audio-lens/issues/159"
    - "https://github.com/scarecrow173/audio-lens/pull/163"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-ci-dispatch-only-issue-147.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ci.yml workflow_dispatch 専用化(AL#147/#158 最終段階): PR/push trigger 撤去

## Work

1. `ci.yml` `on:` から `pull_request`/`push` 撤去、`workflow_dispatch`
   (expected_sha required)のみ維持。
2. `expected-sha` job の event 分岐(`if: github.event_name` ×2 +
   `Record non-dispatch event` step)撤去、無条件照合化。
3. header comment 更新(移行中記述→最終状態、concurrency・
   expected_sha・`summary` 維持理由を整合)。
4. `tools/check-ci-workflow.mjs`: trigger 厳密一致反転 + expected-sha
   event 分岐禁止 + header 契約記述更新。
5. doc chain + index 再生成、陰性 fixture 検証、review-protocol、
   commit/push、日本語 PR。
6. PR 無起動を Actions API/check suite で実検証して報告。

## Classification

- 種別: CI migration 最終段階(workflow 構造のみ。製品コード不変)
- branch: `fix/al-ci-manual-only`

## Upstream Documents

- `../specs/ci-dispatch-only-issue-147.md`
- `../designs/ci-dispatch-only-issue-147.md`
- `../plans/implement-ci-dispatch-only-issue-147.md`
- `../test-specs/ci-dispatch-only-issue-147.md`
- `../designs/full-ci-ci-summary-issue-158.md`(変更 5 = 本変更)

## Dependencies

- PR #163 merge 済み(`bfe7d8f`)・reconcile run `42180639964`
  completed/success 確認済み。
- 3 repo rulesets: `main-checkstatus` disabled・`main-protect`
  active(required_status_checks なし)実 API 再確認済み。

## Affected Files

- `.github/workflows/ci.yml`
- `tools/check-ci-workflow.mjs`
- `docs/**`(本 chain + index)

## Done When

- `mise run check-ci-workflow`/`check-ci-tracker-workflows`/
  actionlint/`prek run --all-files` exit 0。
- 陰性 fixture 5 本 exit 1、正 fixture exit 0。
- review-protocol 指摘解消済み。
- PR 作成/更新で当該 head SHA の CI run が 0 件であることを
  Actions API/check suite で確認・報告済み。

## Verification

実施環境: Windows MINGW64 / mise(Node 26.12.1, actionlint 1.7.12,
biome 2.6.2, prek 0.5.7)。

| 検査 | 結果 |
| --- | --- |
| `mise run check-ci-workflow` | exit 0 |
| `mise run check-ci-tracker-workflows` | exit 0(無変更) |
| `actionlint .github/workflows/ci.yml` | exit 0 |
| `prek run --all-files` | exit 0(biome/cargo fmt/actionlint/両 checker 全 pass) |
| 陰性 fixture N1-N5(pull_request/push/schedule/expected_sha 欠落/event 分岐) | 全て exit 1、狙った契約違反メッセージを確認 |
| 追加陰性 N5b(folded if:)/N6(env 内 event_name 参照) | 全て exit 1 |
| `doc_maintenance apply` index 再生成 | exit 0、5 README 更新。新規 findings は 6 件(同型 issue-159 chain と同一の tolerated class: migration id-prefix ×5 + reciprocal-relation ×1) |
| effective rules 再確認(3 repo) | `main-checkstatus` disabled・`main-protect` active(deletion/non_fast_forward/pull_request・required_status_checks なし)・classic branch protection 404 |
| reconcile main 実受入 | run 42180639964 completed/success — `eed590e` 実 checkout・`mode: reconcile`・最小権限・report artifact upload を log 確認 |
| review-protocol | adversarial pass で 2 件指摘(event_name 検査を includes 強化・includes スタイル)修正、再検証 OK。最終判定 ACCEPT WITH NON-BLOCKING NOTES |
| PR 無起動実検証(PR #164, head 6399172, 作成 2026-11-06T07:37:08Z) | `gh pr checks` → checks 0 件。`actions/runs?branch=fix/al-ci-manual-only&created>07:36Z` → total 0。repo 全体 `actions/runs?created>07:36Z` → total 0。check-suites → `devin-ai-integration` app suite 1 件のみ(repo workflow ではない外部 app) |

未実施(worker scope 外/実測不可):
- cron 実観測(reconcile/daily とも NotRun、manual 実行と混同しない)。
- merge 後の daily dispatch 実動作(次回 21:23 UTC 以降)。
- 製品 Rust/UI suite(変更は trigger 層のみ、lane 本体不変)。
