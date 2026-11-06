---
id: "PLAN-034SKVQSQ4akoGIp96Tksi"
type: "plan"
status: "approved"
title: "Issue #159 Tracker reconcile 入口実装計画"
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
    - "../specs/ci-tracker-reconcile-issue-159.md"
    - "../designs/ci-tracker-reconcile-issue-159.md"
  implemented-by:
    - "../tasks/ci-tracker-reconcile-issue-159.md"
  depends-on:
    - "../designs/audiolens-independent-ci.md"
    - "../designs/ruviform-dependency-ci.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/368"
    - "https://github.com/scarecrow173/ruviform/pull/380"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/ci-tracker-reconcile-issue-159.md"
    - "../designs/ci-tracker-reconcile-issue-159.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ci-tracker-reconcile-issue-159.md"
  references:
    - "https://github.com/scarecrow173/ruviform/blob/eed590e0a3837e61c943d9de1d3efcfd7df32cb0/.github/workflows/ci-issue-tracker-reconcile.yml"
  defers: []
  deferred-by: []
---

# Issue #159 Tracker reconcile 入口実装計画 Implementation Plan

## Goal

AudioLens の CI failure Issue Tracker に、共有 pin の
`eed590e0a3837e61c943d9de1d3efcfd7df32cb0` 更新と、同一 evaluator
を呼ぶ `schedule` + `workflow_dispatch` の reconcile 入口 workflow
を導入し、構造契約テストで入口境界を fail-closed に固定する。

## Scope

単一の実装ストリーム(workflow wiring + 構造契約テスト + doc 同期)。
複数 plan への分割は不要。

## Upstream Documents

- [docs/specs/ci-tracker-reconcile-issue-159.md](../specs/ci-tracker-reconcile-issue-159.md)
- Ruviform accepted ADR `ADR-vJvyZNtrHi2xnycJxbnSPv`
  (issue-368-tracker-reconcile-schedule-entry.md @ eed590e) —
  別 workflow schedule/manual 入口の決定権。本 plan はその
  consumer 適用であり ADR 本文は変更しない。
- Ruviform spec `issue-368-tracker-reconcile.md` @ eed590e —
  `--reconcile` モード・guard・mode 記録の共有契約正本。

## Design Inputs

- [docs/designs/ci-tracker-reconcile-issue-159.md](../designs/ci-tracker-reconcile-issue-159.md)
  (approved)— workflow 骨格・guard・cron 根拠・check tool 設計。

## File Map

- `.github/workflows/ci-issue-tracker-reconcile.yml` — 新設。schedule +
  workflow_dispatch、reconcile job、共有 checkout + `--reconcile` 実行。
- `.github/workflows/daily-ci.yml` — shared checkout `ref:` を
  `eed590e0a3837e61c943d9de1d3efcfd7df32cb0` へ 1 行更新。
- `.github/workflows/ci-issue-tracker.yml` — 同様に `ref:` 1 行更新
  (trigger・権限・concurrency は不変)。
- `tools/check-ci-tracker-workflows.mjs` — 新設。3 workflow の構造
  契約 fail-closed 検査。
- `mise.toml` — `[tasks.check-ci-tracker-workflows]` 追加。
- `.pre-commit-config.yaml` — local hook
  `check-ci-tracker-workflows` 追加。
- `docs/{specs,designs,plans,test-specs,tasks}/README.md` — 生成
  index 再生成(doc_maintenance)。

## Implementation Sequence

- [ ] `daily-ci.yml`・`ci-issue-tracker.yml` の共有 `ref:` を
      eed590e へ更新(各 1 行)。
- [ ] `.github/workflows/ci-issue-tracker-reconcile.yml` を design
      §2 の骨格どおり作成。
- [ ] `tools/check-ci-tracker-workflows.mjs` を作成(共有 pin 一致・
      reconcile 入口契約・event 入口不変の構造検査)。
- [ ] `mise.toml` に task、`.pre-commit-config.yaml` に hook を登録。
- [ ] 検証実行(actionlint・新 check・check-ci-workflow・prek
      --all-files・共有 suite の node --test)。
- [ ] doc 生成 index 再生成 → 論理コミット → push → 日本語 PR。

## Dependencies and Risks

- 共有 pin eed590e は Ruviform main で実在(merge 済み PR #380 を
  `git ls-remote`/`git fetch` で確認済み)。未取得なら作業停止して
  coordinator へ証拠付きで報告する。
- cron 実発火は merge 後でないと観測できない — offline 検証 pass を
  cron 受入と混同しない。
- prek hook 追加は `files:` パターンで発火対象を限定し、無関係
  ファイルでの pre-commit 失敗を起こさない。

## Task Breakdown

- [docs/tasks/ci-tracker-reconcile-issue-159.md](../tasks/ci-tracker-reconcile-issue-159.md)
  — 単一 task。複数 commit(① workflows+pin、② check tool+登録、
  ③ docs)への分割は実装時の論理境界として許容。

## Verification

- [ ] `mise run check-ci-tracker-workflows` exit 0。
- [ ] `mise exec -- actionlint .github/workflows/` exit 0。
- [ ] `mise run check-ci-workflow` exit 0(ci.yml 無差分の確認)。
- [ ] `mise exec -- prek run --all-files` exit 0。
- [ ] 共有 checkout 相当(pin eed590e の `ci/tracker`・`ci/daily`・
      `ci/summary`)の `node --test` が exit 0。
- [ ] `git diff` で event 入口・daily は pin 1 行のみ、ci.yml 無差分。
- [ ] 生成 index が新 doc を反映(`doc_maintenance.js plan` で差分 0)。

## Open Questions / Gaps

- 実 cron/実 Issue/実 dispatch の受入は本 plan の範囲外(merge 後
  の coordinator 工程)。offline 契約検証までを完了条件とする。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution(本 worker が直接実装)。
- Independent work streams: none(単一 task)。
- User approval required before dispatch: no — worker-guide により
  review-protocol 独立レビューを承認代替とし、本 worker が実装まで
  担う。

## Review Handoff

- reviewer 注目点: (a) job `if:` が schedule で `repository` 欠落時も
  silent skip しないこと、(b) checkout が常に default branch pin で
  あること、(c) concurrency group が event 入口と完全一致すること、
  (d) `TRACKER_EVENT_PATH` を渡さない mode 分離、(e) check tool が
  pin 不一致・trigger 混入・cron 追加を実際に検出すること。
