---
id: "TSPEC-034Xq14aOWwCe83ety6DBH"
type: "test-spec"
status: "approved"
title: "CI hygiene/verify 修復の外部契約"
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-210-ci-failure-fix-plan.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-210-ci-failure-fix.md"
    - "docs/designs/issue-210-ci-failure-fix-design.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# CI hygiene/verify 修復の外部契約

## Purpose

Issue #210 が要求する main CI 修復の、外部から観測可能な契約を
固定する。prek hook・biome・構造契約 script の振る舞いは
「修正後も検査が fail-closed であること」と「全検証が Pass する
こと」の 2 軸で判定し、merge 後 main SHA の CI success が最終
証拠となる。本 spec は lint rule の無効化や hook 契約の変更を
「修正」として受理しない境界を明示するために存在し、CI 構成が
変わり契約自体が更新された場合に再評価される。

## Feature

pre-commit 衛生ゲートと verify lane の静的 lint が、main の
現行構成(ci.yml・composite action・tracker workflow・Remotion
tool の現行コード)に対して Pass する。

## Rules

- Rule: `.pre-commit-config.yaml` は valid な YAML であり、
  定義された全 hook が parse 後に実行可能である。
- Rule: `biome check` は repo 全体で error/warning を残さない。
  `noNonNullAssertion` は実行時チェックへ、`noTemplateCurlyInString`
  は検査意味を変えない限定抑制で対処する。
- Rule: `check-ci-workflow.mjs`/`check-ci-tracker-workflows.mjs`
  の照合文字列・判定ロジック・hook の files/pass_filenames 契約は
  不変である。
- Rule: 検査の弱化(lint rule off、警告握りつぶし、files フィルタ
  縮小、hook 削除、`-D warnings` 解除)を修正手段として使わない。

## Examples

- Example: Given 修復済みの `.pre-commit-config.yaml`、
  When `mise exec -- prek run --all-files` を実行、
  Then exit 0 で全 hook が Pass する。
- Example: Given biome 修正済みの 3 ファイル、
  When `mise exec -- biome check` を実行、
  Then 診断 0 件で exit 0。
- Example: Given biome-ignore 付きの照合リテラル、
  When `node tools/check-ci-tracker-workflows.mjs` を実行、
  Then `${{ github.repository }}` 等の文字列照合が従来どおり
  機能し `... OK` を出力する。

## Guarantees

- `prek run --all-files`・`biome check`・`mise run fmt`・
  `lint-full`・`test-full`・`check-cli-headless`・
  `verify-toolchain`・`check-ruviform-rev`・`check-ruviform-lock`・
  `check-ci-workflow`・`check-ci-tracker-workflows` が
  Windows ローカルで exit 0。
- lint 設定・hook 契約・検査対象範囲は一切変更されていない。

## Non-goals

- macOS leg・`gui-real`・3OS matrix の他 OS leg・`*-msrv` 系の
  実行(ローカル実行不可。merge 後 CI 証跡で確認)。
- ubuntu CI 固有の toolchain 差異(clippy の OS 依存 lint 等)の
  先回り修正 — 今回の変更は Rust コードに触れない。
- Remotion scene の描画結果そのものの検証(描画は preview/GUI
  領域であり、本修正は lint 準拠のみ)。

## Risk

- biome-ignore を契約 script の照合文字列変更と混同して
  受理してしまうと、fail-closed 検査が形骸化する。照合文字列の
  diff が空であることをレビューで確認する。
- `getContext("2d")` の throw 化を silent fallback と誤認すると、
  描画不能時の診断性が下がる。throw は context 取得失敗時のみで
  通常経路の挙動は不変。
