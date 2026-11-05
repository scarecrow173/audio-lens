---
id: "PLAN-034SmwSu88qZaqPULzbz13"
type: "plan"
status: "approved"
title: "Issue #158 Full CI 統一(安全な第一段階)実装計画"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/158"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/full-ci-ci-summary-issue-158.md"
    - "../designs/full-ci-ci-summary-issue-158.md"
  implemented-by:
    - "../tasks/full-ci-ci-summary-issue-158.md"
  depends-on:
    - "../designs/audiolens-independent-ci.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "https://github.com/scarecrow173/audio-lens/issues/147"
    - "https://github.com/scarecrow173/audio-lens/issues/159"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/full-ci-ci-summary-issue-158.md"
    - "../designs/full-ci-ci-summary-issue-158.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/full-ci-ci-summary-issue-158.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #158 Full CI 統一(安全な第一段階)実装計画 Implementation Plan

## Goal

`ci.yml` を選択実行から常時全 lane の Full CI へ移し、`ci-summary`
fail-closed 集約・`expected_sha` 契約・構造回帰テストを、Required
context `summary` と既存 trigger を壊さず導入する。

## Scope

単一実装ストリーム。`ci.yml`・`tools/`・`mise.toml`・
`.pre-commit-config.yaml`・関連 docs の一連の変更で、workflow と
構造テストは同一コミットで atomic に成立させる(契約テストが旧
構造を fail させるため分割不可)。

## Upstream Documents

- `docs/specs/full-ci-ci-summary-issue-158.md`(approved)

## Design Inputs

- `docs/designs/full-ci-ci-summary-issue-158.md`(approved、job DAG・
  gate 意味論・証跡形式の正本)

## File Map

- `.github/workflows/ci.yml` — Full CI 化(impact 撤去・全 lane
  常時・expected-sha/ci-summary 追加・summary 互換維持・
  macos-summary 撤去・top comment 同期)
- `tools/classify-impact.mjs` — 削除
- `tools/check-ci-workflow.mjs` — 新規: workflow 構造契約の
  fail-closed 検査(trigger・expected_sha・job 集合・needs・
  `always()`・impact 機構不在)
- `mise.toml` — `[tasks.check-ci-workflow]` 追加
- `.pre-commit-config.yaml` — local hook `check-ci-workflow` 追加
  (hygiene job の `prek run --all-files` 経由で CI 実行)
- `.github/actions/{verify,ui-verify,webview-verify}/action.yml` —
  macOS 集約先・共有構造のコメント同期(動作不変)
- `docs/designs/audiolens-independent-ci.md` — §4 lane 構成・集約
  記述を Full CI DAG に同期
- `docs/guides/release-macos-verification.md` — `macos-summary`
  参照を `ci-summary` + macOS legs に更新
- `docs/designs/impact-based-ci-issue-149.md` — status superseded +
  superseded-by
- `docs/tasks/impact-based-ci-issue-149.md` — status done + 退役注記 +
  superseded-by
- `docs/tasks/full-ci-ci-summary-issue-158.md` — 実装 task(front
  matter・受入・検証記録)

## Implementation Sequence

- [ ] 1. `tools/check-ci-workflow.mjs` を実装(契約定義の正本として先に書く)
- [ ] 2. `ci.yml` を Full CI DAG へ改修(top comment・on/env・
       expected-sha・impact 撤去・lane if 簡素化・macos-summary 撤去・
       ci-summary 追加・summary 簡素化)
- [ ] 3. `tools/classify-impact.mjs` 削除・composite action コメント同期
- [ ] 4. `mise.toml` task + `.pre-commit-config.yaml` hook 追加
- [ ] 5. `check-ci-workflow`/`actionlint`/`prek run --all-files` 実行で
       契約成立を検証(negative case も stub で確認)
- [ ] 6. docs 同期(independent-ci 設計・macOS 手順・impact 文書の
       superseded 化・task doc の受入/検証記入)
- [ ] 7. Conventional Commits で commit

## Dependencies and Risks

- check-ci-workflow.mjs は indent ベースの構造検査 — ci.yml の表記
  揺れ(inline needs 等)に対し、検査対象の書法を本変更で固定する
  (needs は flow list、job id は `^  <id>:` の 2 空白)。
- `needs` の job id にハイフンを含む `verify-macos` 等は
  `needs.verify-macos.result` 参照が必要 — YAML/式の記法は現行
  `macos-summary` と同型を踏襲。
- PR での Full CI 重量化は既知 trade-off(spec Deferred)。
- GitHub Actions 実環境での `needs.<id>.result` 評価は worker 環境で
  実証不能 — actionlint + 構造テスト + 既存同型コードの踏襲で担保し、
  未検証項目として報告する。

## Task Breakdown

- `docs/tasks/full-ci-ci-summary-issue-158.md` 1 task(上記
  sequence を受入項目に対応付け)。

## Verification

- [ ] `mise exec -- node tools/check-ci-workflow.mjs` exit 0(契約成立)
- [ ] 契約を意図的に壊した fixture で非 0 exit(negative case)
- [ ] `mise exec -- actionlint .github/workflows/ci.yml` exit 0
- [ ] `mise exec -- prek run --all-files` exit 0(hook 経由の実施含む)
- [ ] `git grep` で `classify-impact|CI_IMPACT_SELECTIVE|macos-summary|
      not_applicable` が ci.yml/tools から消えている
- [ ] commit が Conventional Commits 形式

## Open Questions / Gaps

- `workflow_dispatch` UI での `expected_sha` 手入力の運用手順は
  dispatch 実行者のドキュメント(#159 Scheduler 側)で扱う。
- GitHub 上での実 run 挙動(check 名・Pending 維持・expected_sha
  不一致時の skip 伝播)は本 push/merge 後の run でのみ確認可能。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution(単一 worktree の一連編集)
- Independent work streams: none
- User approval required before dispatch: yes
- If approved: sequence 1-7 を順に実施し、5 で契約検証、6-7 で docs
  同期と commit。
- If declined or unavailable: 同じ手順を inline で実施。

## Review Handoff

- 重点: `summary` needs に macOS legs を含めない判断(#157 性質
  維持)と `ci-summary` needs の完全性、expected-sha の no-op
  success 設計(PR/push で DAG を壊さない)。
