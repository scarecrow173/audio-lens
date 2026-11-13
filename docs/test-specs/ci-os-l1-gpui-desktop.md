---
id: "TSPEC-034WQLTUarQM5Vm4biCEVQ"
type: "test-spec"
status: "approved"
title: "OS-L1 GPUI Desktop CI 構造契約(gpui-verify lane)"
created: "2026-11-13"
updated: "2026-11-13"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/206"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "tools/check-ci-workflow.mjs"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../plans/implement-ci-os-l1-gpui-desktop.md"
    - "../tasks/ci-os-l1-gpui-desktop.md"
    - "../test-specs/full-ci-ci-summary-issue-158.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-ci-os-l1-gpui-desktop.md"
  derived-by: []
  verifies:
    - "docs/specs/ci-os-l1-gpui-desktop.md"
    - "docs/designs/ci-os-l1-gpui-desktop.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# OS-L1 GPUI Desktop CI 構造契約(gpui-verify lane)

## Purpose

Issue #206 が規定する GPUI OS-L1 lane の構造契約を、lane の
silent skip・fail-open 集約・構造契約検査の形骸化に対して
fail-closed で保つ。実装は `tools/check-ci-workflow.mjs`
(prek hook → hygiene job で全 run 実施、
`mise run check-ci-workflow` が検証入口)と、lane 自体の
step 構成(`.github/actions/gpui-verify/action.yml`)が担う。

`ci-summary` が Ruleset Required 化され `summary` が撤去
される最終段階(#158 design の延期項目)で本 spec の
job 集合・needs 契約を再評価する。

## Feature

ci.yml が宣言する GPUI OS-L1 検証 lane の構造契約 —
lane job 集合・needs/if・composite action の検証 step・
集約 job への組み込み・headless 誠実性。

## Rules

- `ci.yml` の job 集合は 14 job: 既存 12 job に
  `gpui-verify`・`gpui-verify-macos` を加えたもの
  (追加・削除・rename は契約違反)。
- `gpui-verify`/`gpui-verify-macos` は他 lane と同じ
  `needs: [ruviform-access, expected-sha]` と両 preflight
  success を課す `if:` を持ち、event による選択実行を
  持たない(常時実行)。
- lane 本体は `.github/actions/gpui-verify/action.yml` の
  単一定義を ubuntu/windows legs と macOS leg が共有し、
  `check-ui-gpui`・`lint-ui-gpui`・`test-ui-gpui`・
  `build-ui-gpui` の 4 検証を全 OS で実行する
  (feature flag による silent skip を許さない)。
- `gpui-test` は `gpui` を内包し `al-desktop` bin は
  `required-features = ["gpui"]` を持つため、4 検証の
  成功は `al-desktop` の型・lint・link・headless test を
  実質カバーする。`build-ui-gpui` は production feature
  `gpui` のみで `al-desktop` bin を build し、Native
  Build/Link を明示的に証明する。
- `ci-summary` の needs は全 14 job を含み `if: always()`、
  `summary` の needs は macOS leg を含まない scope
  (gpui-verify は含み gpui-verify-macos は含まない)で
  `if: always()` — #157/#158 の分離契約を維持する。
- OS 別 NotRun/Skipped/Fail は `ci-summary`/`summary` の
  fail-closed 判定で Pass にならない。`gpui-test` の
  headless 成功を実 OS Window 操作(GUI-L2)の Pass と
  記録しない — `gui-real` の not_run 証跡は維持する。
- 既存 lane(verify/ui-verify/webview-verify)の job・
  action・証跡行は本変更で削除・改変されない
  (verify lane の `check-full` step 撤去のみ例外で、
  `lint-full` の clippy が型診断を包含する設計判断済み)。

## Examples

- Example: `gpui-verify (ubuntu-latest)` job が
  `check-ui-gpui` で失敗した場合、`ci-summary` と `summary`
  の両方が fail-closed で run を失敗にし、証跡表に
  `gpui-verify | ubuntu-latest / windows-latest | failure`
  が記録される。
- Example: `gpui-verify-macos` のみ失敗した場合、
  `ci-summary` は失敗、`summary`(macOS 非含有 scope)は
  影響を受けない — #157 の性質維持。
- Example: `tools/check-ci-workflow.mjs` が ci.yml から
  `gpui-verify` job を削除した fixture を検査すると、
  job 集合契約違反として exit 1 になる。

## Guarantees

- GPUI OS-L1 の 3 OS 証拠が Daily/手動 Full CI の各 run で
  必ず生成され、未実行・skip・失敗が Pass に化けない。
- 構造契約の回帰(lane 削除・needs 欠落・集約分離の破壊)
  が prek hook / hygiene job の静的検査で検出される。
- 旧 WebView 撤去(#189)と独立して本 lane は成立し、
  既存 Oracle 証跡を失わない。

## Non-goals

- `gpui-test` の test 内容・feature 定義・`crates/` 実装の
  正しさ(consumer 側 test-spec/impl の責務)。
- 実 OS Window の GUI-L2 検証と macOS/Linux の GUI-L2/
  EXT-L3(Deferred)。
- `daily-ci.yml` の dispatch 判定ロジック、tracker 系
  workflow、`mise.ci.toml` の toolchain pin。
- Ruviform 側 `ruviform-ui-gpui` の検証(consumer repo の
  責務分界)。

## Risk

- lane 契約が形骸化すると、GPUI 機能の OS-L1 回帰が
  Daily/手動 CI をすり抜け、POLICIES の Required
  OS-L1 が事実上 NotRun のまま放置される。
- `summary`/`ci-summary` の needs 更新漏れは fail-open
  (新 lane 失敗を集約が拾わない)または恒常 fail
  (存在しない job を needs)を生む — 構造契約検査が
  防衛線。
