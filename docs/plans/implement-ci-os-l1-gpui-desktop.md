---
id: "PLAN-034WQMHCumqJGCDGugGOYF"
type: "plan"
status: "approved"
title: "Issue #206 OS-L1 GPUI Desktop CI 統合 実装計画"
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
  implements:
    - "docs/specs/ci-os-l1-gpui-desktop.md"
    - "docs/designs/ci-os-l1-gpui-desktop.md"
  implemented-by:
    - "../tasks/ci-os-l1-gpui-desktop.md"
  depends-on:
    - "../designs/full-ci-ci-summary-issue-158.md"
    - "../designs/sccache-compile-cache-issue-148.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/audio-lens/issues/189"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/ci-os-l1-gpui-desktop.md"
    - "docs/designs/ci-os-l1-gpui-desktop.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/ci-os-l1-gpui-desktop.md"
  references:
    - "POLICIES.md"
  defers: []
  deferred-by: []
---

# Issue #206 OS-L1 GPUI Desktop CI 統合 実装計画 Implementation Plan

## Goal

`ci.yml` Full CI に `gpui-verify`/`gpui-verify-macos` lane を
追加し、`al-gui --features gpui-test` の型/Clippy/Headless
テストと `al-desktop` bin の Native Build/Link を 3 OS で
fail-closed に検証する。併せて `verify` lane の
`check-full`/`lint-full` 重複 compile を整理する。

## Scope

単一実装ストリーム。workflow・composite action・構造契約
検査・mise task は同一の構造契約で atomic に成立させる
(`check-ci-workflow.mjs` が job 集合を固定検査するため、
ci.yml と checker を分割すると中間状態で必ず fail する)。

## Upstream Documents

- `docs/specs/ci-os-l1-gpui-desktop.md`(approved)

## Design Inputs

- `docs/designs/ci-os-l1-gpui-desktop.md`(approved、lane 構成
  ・OS 別 deps・check-full 整理の設計判断の正本)

## File Map

| Path | Responsibility | Notes |
|------|----------------|-------|
| `.github/actions/gpui-verify/action.yml` | GPUI lane 本体の単一定義 | 新規。webview-verify 同型 |
| `.github/workflows/ci.yml` | gpui-verify 2 job 追加・集約更新 | top comment も同期 |
| `.github/actions/verify/action.yml` | check-full step 撤去・証跡表更新 | lint-full が型診断を包含 |
| `tools/check-ci-workflow.mjs` | 構造契約の job 集合/needs 更新 | 14 job 契約 |
| `mise.toml` | `build-ui-gpui` task 追加 | CI 関連入口のみ |
| `docs/specs|designs|plans|tasks|test-specs/ci-os-l1-gpui-desktop.md` | doc-chain | index は doc-maintenance で再生成 |

## Implementation Sequence

- [ ] `.github/actions/gpui-verify/action.yml` を作成
  (access gate → mise → Linux GPUI deps → cargo cache →
  sccache → ruviform access/probe → fetch → rev guard →
  check/lint/test/build-ui-gpui → 証跡)
- [ ] `mise.toml` に `[tasks.build-ui-gpui]` を追加
- [ ] `ci.yml` に `gpui-verify`/`gpui-verify-macos` job を
  追加し、`ci-summary`/`summary` の needs・証跡・fail-closed
  pair・resolved SHA 連鎖・Owner 切り分け・top comment を更新
- [ ] `tools/check-ci-workflow.mjs` の EXPECTED_JOBS/LANES/
  CISUMMARY_NEEDS/SUMMARY_NEEDS と契約コメントを更新
- [ ] `verify` action から `check-full` step を撤去し証跡表を
  更新
- [ ] doc index を `doc_maintenance.js index apply` で再生成
- [ ] `mise exec -- actionlint`・`mise run check-ci-workflow`
  で静的検証
- [ ] branch push → PR 作成 → 手動 Full CI 実走で 3 OS
  OS-L1 結果を記録

## Dependencies and Risks

- **Dependency**: lane は `ruviform-access`/`expected-sha`
  の success を needs する既存 preflight に依存。Ruviform
  resolved SHA は `fd1d4d5`(Cargo.lock 正本)。
- **Risk(macOS 未実績)**: Ruviform に macOS gpui leg がなく
  初回実走で失敗し得る。手動 Full CI で実測し、失敗時は
  原因切り分けて報告 — fail-closed で顕在化するのが契約。
- **Risk(Linux deps)**: Ruviform `ui-gpui-verify` と同一
  gpui 0.2.2・同一 apt 群を採用し差分リスクを最小化。
- **Risk(check-full 撤去)**: clippy が型診断を包含するため
  診断欠落なし。`test-full` の compile が下限保証。
  計測証拠は design doc に記録済み。

## Task Breakdown

- `docs/tasks/ci-os-l1-gpui-desktop.md` — 本計画全体を
  1 タスクとして tracking(単一 commit 境界の論理変更)。

## Verification

- [ ] `mise exec -- actionlint` exits 0
- [ ] `mise run check-ci-workflow` exits 0(更新後契約)
- [ ] `mise run check-ci-tracker-workflows` exits 0(非対象
  workflow の非回帰確認)
- [ ] `gh workflow run ci.yml --ref <branch> -f
  expected_sha=<sha>` の run で `gpui-verify`/
  `gpui-verify-macos`/`ci-summary`/`summary` が success
  (3 OS OS-L1 Pass の証拠)

## Open Questions / Gaps

- macOS runner での gpui build 時間・成否は初回実走まで
  未実測(手動 Full CI で確認)。
- `summary` job への `gpui-verify` 追加は「非 macOS lane は
  全て Required scope に含める」既存不変条件に従う判断。
  `ci-summary` Required 化時の再評価は #158 延期項目。

## Self-Review

- [x] Every spec requirement maps to a task(本 task に集約)。
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology
  are consistent.

## Implementation Handoff

- Recommended mode: inline execution(単一ストリーム・共有
  ファイル競合なし)。
- Independent work streams: none
- User approval required before dispatch: no(本タスクは
  単一 worker が逐次実行)

## Review Handoff

- Reviewer focus: `summary` needs への `gpui-verify` 追加判断、
  `check-full` 撤去の診断等価性、Linux deps の過不足、
  macOS leg の初回実走リスク。
