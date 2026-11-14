---
id: "SPEC-034XpWEQMpT2Z7xn8JP2Nh"
type: "spec"
status: "approved"
title: "main CI hygiene/verify 失敗の修正(Issue #210)"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/full-ci-ci-summary-issue-158.md"
    - "docs/designs/ci-tracker-reconcile-issue-159.md"
    - "docs/designs/ci-os-l1-gpui-desktop.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# main CI hygiene/verify 失敗の修正(Issue #210)

## What Should Be Built

main の `ci` workflow で失敗している 2 job の root cause を、責務を
持つレイヤーで修正し、CI verify ubuntu lane 相当のローカル検証が
全て Pass する状態を回復する。具体的には:

- `.pre-commit-config.yaml` の YAML 構文破損(`hygiene` job の
  `prek run --all-files` が parse error で exit 2)を修復する。
- `biome check` が報告する error/warning を、lint を弱めずに
  解消する(`verify (ubuntu-latest)` job の `lint-full` 失敗)。
- 修正の root cause 分析・検証台帳を `evidence/al210/` に残す。

## Why Now

- PR #209(merge commit `0e2bfee`)以降、main の `ci` run
  (run 43540885602)で `hygiene` と `verify (ubuntu-latest)` が
  失敗し、ci-issue-tracker 管理 Issue #210 が起票された。
- `hygiene` は config parse 段階で落ちるため全 hook が未実行で、
  ローカルの pre-commit も同様に死ぬ。main の衛生ゲートが
  実質無効化されている。
- `lint-full` の失敗で verify lane の後続 step(test-full・
  check-cli-headless 等)が当該 run で未実行の可能性があり、
  さらなる破損が潜んでいるかもしれない。lane 全体の再検証が必要。

## Users and Value

- **main の CI 結果を受入証拠に使う開発者/agent**: 必須 job が
  再び green になり、merge 後 SHA の CI 証跡による統合検証が
  可能になる。
- **ci-issue-tracker**: 失敗 revision と現 main tip をカバーする
  新しい ci run の success で Issue #210 を auto-close できる。
- **後続作業の Worker**: pre-commit hook が復旧し、commit 時の
  軽量検証が機能する。

## Scope

### In Scope

- `.pre-commit-config.yaml` の構文修復(閉じられていない
  single-quoted scalar の閉鎖と、混入した重複 hook 定義の除去)。
  hook の id/entry/files/pass_filenames 契約は変更しない。
- `tools/cinematic-sword-cutscene/src/three/vfx/ParticleSystem.tsx`
  の `noNonNullAssertion` を実行時チェックへ書き換える。
- `tools/check-ci-workflow.mjs` の `useTemplate`/`useOptionalChain`/
  `useIndexOf`/formatter drift を biome の推奨形へ整合させる。
- `tools/check-ci-tracker-workflows.mjs` の `useIndexOf` を修正し、
  `${{ }}` GitHub 式を含む照合リテラル 3 箇所を、検査意味を変えない
  形(biome-ignore + 理由)で対処する。
- `evidence/al210/` への root cause 分析・検証台帳の記録。
- 本 spec + design/plan/task/test-spec/ir の doc チェーン作成と、
  影響 dir の管理 index の正規手順での再生成。

### Out of Scope

- lint 設定(`biome.jsonc`)の変更、biome rule の off 化、
  `--error-on-warnings`/`-D warnings` の解除、hook の削除・無効化、
  files フィルタの縮小など、gate を弱める一切の変更。
- `.github/workflows/` の変更、`mise.toml`/`mise.ci.toml` の変更、
  Rust コードの変更。
- ruviform リポジトリの変更(root cause が Ruviform 側と確定した
  場合の Issue 起票は別途)。
- `*-msrv` 系タスク、macOS leg 等ローカル実行不可 lane の検証
  (NotRun として台帳に記録)。
- Issue #210 本文・marker comment の編集、手動 close。

## Requirements

- R1: `.pre-commit-config.yaml` が YAML として parse でき、
  `prek run --all-files` が全 hook を実行して Pass する。
- R2: `mise run lint-full` の `biome check` が error/warning
  なしで exit 0 する(`noNonNullAssertion` は実行時チェックへの
  書き換え、`noTemplateCurlyInString` は検査意味を変えない対処)。
- R3: `check-ci-workflow.mjs`/`check-ci-tracker-workflows.mjs` の
  検査契約(照合対象文字列・判定ロジック)が変わらない。
- R4: `mise run fmt`/`lint-full`/`test-full`/`check-cli-headless`/
  `verify-toolchain`/`check-ruviform-rev`/`check-ruviform-lock`/
  `check-ci-workflow`/`check-ci-tracker-workflows` が Pass する。
- R5: 修正の root cause 分析・検証台帳・NotRun を `evidence/al210/`
  に記録する。

## Acceptance Criteria

- [ ] `mise exec -- prek run --all-files` が exit 0 で全 hook Pass。
- [ ] `mise run lint-full` が exit 0(biome check + clippy
      -D warnings)。
- [ ] `mise run fmt`・`mise run test-full`・
      `mise run check-cli-headless`・`mise run verify-toolchain`・
      `mise run check-ruviform-rev`・`mise run check-ruviform-lock`・
      `mise run check-ci-workflow`・
      `mise run check-ci-tracker-workflows` が全て exit 0。
- [ ] `evidence/al210/` に一次ログ引用・導入 commit・ローカル再現・
      検証台帳・NotRun 表がある。
- [ ] PR 本文が日本語で `Refs #210` を含み、実行した検証の
      コマンド/OS/exit code・NotRun・review-protocol 結果・
      未対応作業を記載する。

## Deferred Design Concerns

- なし。

## Sources and Relations

- Issue #210(ci-issue-tracker 管理 Issue。失敗 run 43540885602、
  sha `0e2bfee57ead58510ead95212a2540540efc430b` = PR #209 merge)。
- 関連設計: `docs/designs/full-ci-ci-summary-issue-158.md`(ci.yml
  構造契約)、`docs/designs/ci-tracker-reconcile-issue-159.md`
  (tracker workflow 契約)、`docs/designs/ci-os-l1-gpui-desktop.md`
  (OS-L1 lane 構成)。
