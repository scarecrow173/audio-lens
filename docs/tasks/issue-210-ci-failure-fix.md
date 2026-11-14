---
id: "TASK-034Xq1IjYBr7OnhswOHW6s"
type: "task"
status: "done"
title: "main CI hygiene/verify 失敗修正(AL#210)"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/210"
    - "https://github.com/scarecrow173/audio-lens/actions/runs/43540885602"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-210-ci-failure-fix.md"
      - type: "doc"
        path: "docs/designs/issue-210-ci-failure-fix-design.md"
      - type: "doc"
        path: "docs/plans/issue-210-ci-failure-fix-plan.md"
      - type: "doc"
        path: "docs/tasks/issue-210-ci-failure-fix.md"
      - type: "doc"
        path: "docs/test-specs/issue-210-ci-failure-fix.md"
      - type: "doc"
        path: "docs/impl/ir/main-ci-hygiene-verify-issue-210.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-210-ci-failure-fix-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-210-ci-failure-fix-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-210-ci-failure-fix.md"
  references: []
  defers: []
  deferred-by: []
---

# main CI hygiene/verify 失敗修正(AL#210)

## Work

main CI run 43540885602 で失敗した `hygiene` と
`verify (ubuntu-latest)` の root cause を修正する。
`.pre-commit-config.yaml` の YAML 構文破損(閉じられていない
single-quoted scalar と重複 hook ブロック)を修復し、
`biome check` の診断(error 1・warning 5・info 13)を
lint を弱めずに解消する。`noNonNullAssertion` は実行時
チェックへ、`${{ }}` 照合リテラルは理由付き biome-ignore で
対処し、`useTemplate`/`useOptionalChain`/`useIndexOf`/
formatter drift は biome 推奨形へ整合させる。
あわせて `evidence/al210/` に root cause 分析と検証台帳を残す。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-210-ci-failure-fix-plan.md`
- `docs/designs/issue-210-ci-failure-fix-design.md`
- `docs/specs/issue-210-ci-failure-fix.md`
- `docs/test-specs/issue-210-ci-failure-fix.md`

## Dependencies

- `prek`/`biome`/`actionlint`/`node` は mise 管理
  (repo pin: prek 0.5.7, biome 2.6.2, actionlint 1.7.12,
  node 26.12.1)。rust は環境導入版を使用(repo は pin しない)。
- なし(単一 worktree 内の一連編集)。

## Affected Files

- `.pre-commit-config.yaml`(構文修復のみ。hook 契約不変)。
- `tools/check-ci-workflow.mjs`、
  `tools/check-ci-tracker-workflows.mjs`(biome 診断解消。
  照合文字列・判定ロジック不変)。
- `tools/cinematic-sword-cutscene/src/three/vfx/
  ParticleSystem.tsx`(non-null assertion → 実行時チェック)。
- `evidence/al210/`(新規: root cause 分析・検証台帳)。
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/
  issue-210-ci-failure-fix*.md`(doc チェーン)。
- 各 docs dir の管理 index(再生成物)。

## Done When

- [x] `mise exec -- prek run --all-files` が exit 0 で全 hook Pass。
- [x] `mise exec -- biome check` が診断 0 件で exit 0。
- [x] `node tools/check-ci-workflow.mjs` /
      `node tools/check-ci-tracker-workflows.mjs` が OK を維持
      (照合契約の semantics 保存)。
- [x] `mise run lint-full`/`test-full`/`fmt`/`check-cli-headless`/
      `verify-toolchain`/`check-ruviform-rev`/
      `check-ruviform-lock`/`check-ci-workflow`/
      `check-ci-tracker-workflows` が exit 0。
- [x] `evidence/al210/` に一次ログ引用・導入 commit・ローカル
      再現・検証台帳・NotRun 表がある。
- [x] review-protocol レビューで blocker なし。
- [ ] PR 作成(本文は日本語・`Refs #210`)— commit/push 後に実施。

## Verification

実行環境: Windows 11 (ORCA worktree) / mise 管理 tool
(prek 0.5.7, biome 2.6.2, actionlint 1.7.12, node 26.12.1) /
rustc 1.99.1(環境導入)。`MISE_ENV` 未設定 = ローカル標準
profile。

- [x] `mise exec -- prek run --all-files` → exit 0。
- [x] `mise exec -- biome check` → exit 0(修正前は exit 1、
      error 1/warning 5/info 13 を再現)。
- [x] `mise run fmt` → exit 0。
- [x] `mise run check-cli-headless` → exit 0。
- [x] `mise run verify-toolchain` → exit 0。
- [x] `mise run check-ruviform-lock` → exit 0。
- [x] `mise run check-ruviform-rev` → exit 0。
- [x] `mise run check-ci-workflow` → exit 0。
- [x] `mise run check-ci-tracker-workflows` → exit 0。
- [x] `mise run lint-full` → exit 0。
- [x] `mise run test-full` → exit 0。
- [ ] NotRun: macOS leg 群・`gui-real`・他 OS leg・`*-msrv`
      (ローカル実行不可。merge 後 CI 証跡で確認)。
