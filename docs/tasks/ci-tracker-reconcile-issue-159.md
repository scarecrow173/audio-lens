---
id: "TASK-034SKVSwvA141jYCWJ6SYi"
type: "task"
status: "done"
title: "Tracker reconcile 入口(AL#159): 共有 pin eed590e 更新と reconcile workflow 導入"
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
    - "../plans/implement-ci-tracker-reconcile-issue-159.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-ci-tracker-reconcile-issue-159.md"
    - "../designs/ci-tracker-reconcile-issue-159.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/368"
    - "https://github.com/scarecrow173/ruviform/pull/380"
    - "https://github.com/scarecrow173/audio-lens/issues/147"
    - "https://github.com/scarecrow173/audio-lens/issues/158"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-ci-tracker-reconcile-issue-159.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Tracker reconcile 入口(AL#159): 共有 pin eed590e 更新と reconcile workflow 導入

## Work

`daily-ci.yml`・`ci-issue-tracker.yml` の Ruviform 共有実装 checkout
pin を `eed590e0a3837e61c943d9de1d3efcfd7df32cb0` へ更新し、
`ci-issue-tracker-reconcile.yml`(schedule + workflow_dispatch のみ、
同一 concurrency group・最小権限・default-branch pin checkout・
`--reconcile` 実行)を新設する。`tools/check-ci-tracker-workflows.mjs`
で構造契約を fail-closed 検査し、mise task + prek hook に登録する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `../plans/implement-ci-tracker-reconcile-issue-159.md`
- `../designs/ci-tracker-reconcile-issue-159.md`
- `../specs/ci-tracker-reconcile-issue-159.md`
- `../test-specs/ci-tracker-reconcile-issue-159.md`

## Dependencies

- Ruviform main `eed590e0a3837e61c943d9de1d3efcfd7df32cb0`(PR #380
  merge 済み)が共有実装として取得可能であること。
- なし(単一 worktree 内の一連編集)。

## Affected Files

- `.github/workflows/ci-issue-tracker-reconcile.yml`(新規)、
  `.github/workflows/daily-ci.yml`・`.github/workflows/ci-issue-tracker.yml`
  (pin 1 行ずつ)、`tools/check-ci-tracker-workflows.mjs`(新規)、
  `mise.toml`、`.pre-commit-config.yaml`、doc 生成 index。

## Done When

- [x] 3 workflow の共有 pin が eed590e で一致し、
      `mise run check-ci-tracker-workflows` が exit 0。
- [x] reconcile workflow が design §2 の契約(trigger・guard・
      concurrency・permissions・timeout・checkout pin・
      `--reconcile`・`TRACKER_EVENT_PATH` 非付与・self-test 先行)
      を満たし、actionlint が exit 0。
- [x] `ci-issue-tracker.yml` の trigger が `workflow_run` のみのまま
      (cron 追加なし)。
- [x] check tool が負例(pin 不一致・event 入口 trigger 混入・
      guard 欠落・mode 違反)を全件 exit 1 で検出する。
- [x] 共有 pin eed590e の `ci/tracker`・`ci/daily`・`ci/summary`
      suite を `node --test` で実測 exit 0。
- [x] `mise exec -- prek run --all-files`・`mise run check-ci-workflow`
      が exit 0、`ci.yml` 無差分。
- [x] 生成 index 再生成済み・検証記録(実コマンド + exit code)を
      本書に記入済み。

## Verification

実行環境: Windows (MINGW64) / mise 管理 toolchain
(node 26.12.1, actionlint 1.7.12, biome 2.6.2, prek 0.5.7)

- [x] `mise exec -- node tools/check-ci-tracker-workflows.mjs`
      → exit 0(3 workflow の pin 一致・reconcile/event/入口契約 OK)
- [x] `mise exec -- actionlint .github/workflows/ci-issue-tracker-reconcile.yml
      .github/workflows/ci-issue-tracker.yml .github/workflows/daily-ci.yml`
      → exit 0
- [x] `mise exec -- node tools/check-ci-workflow.mjs` → exit 0
      (ci.yml 無差分・構造契約維持)
- [x] `mise exec -- prek run --all-files` → exit 0
      (check-ci-tracker-workflows hook 実施済み・全 hook 通過)
- [x] 共有実装実測: `git archive eed590e ci | tar -x` で展開した
      `ci/` に対し `mise exec -- node --test` を実行 —
      tracker **91/91 pass**・daily **50/50 pass**・summary
      **31/31 pass**(いずれも exit 0)
- [x] 負例 fixture(3 workflow を複製して個別に改変)で非 0 exit を
      確認 → 全 6 負例で exit 1: (1) daily pin を旧 73d51d8 に
      逆戻し(pin 不一致検出)、(2) event 入口へ schedule 追加
      (trigger 違反検出)、(3) reconcile `if:` から
      `github.event_name == 'schedule' ||` 除去(guard 違反検出)、
      (4) `TRACKER_EVENT_PATH` env 追加(mode 分離違反検出)、
      (5) reconcile へ `workflow_run` trigger 追加(入口違反検出)、
      (6) `--reconcile` 除去(実行契約 3 違反検出)
- [x] `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js
      apply` → exit 0(生成 index 4 件再生成、findings 75→71。
      残存は既存の manual/migration findings)
- [x] review-protocol: 独立レビュー(A 適合/B 敵対→Validator→Judge)
      で CONFIRMED 指摘 2 件(cron 複数 entry 未検査・event group
      部分一致)を修正・再検証済み。残余 blocker なし —
      ACCEPT WITH NON-BLOCKING NOTES

未検証項目: 実 GitHub Actions 上の job `if:` 評価・schedule 実発火・
実 Issue 起票/close・共有 checkout の実 runner 動作はローカルでは
実行不能。実 cron/実 Issue/実 dispatch の受入は merge 後の
coordinator 工程に委譲する。製品 Rust/UI suite は未実行(変更は
workflow wiring + node 構造検査のみ)。
