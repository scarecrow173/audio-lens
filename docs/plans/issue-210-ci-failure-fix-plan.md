---
id: "PLAN-034Xq09g8IwNjIHhpHYbgT"
type: "plan"
status: "approved"
title: "main CI hygiene/verify 失敗修正 実装計画"
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
    - "docs/specs/issue-210-ci-failure-fix.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-210-ci-failure-fix.md"
    - "docs/designs/issue-210-ci-failure-fix-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-210-ci-failure-fix.md"
  references: []
  defers: []
  deferred-by: []
---

# main CI hygiene/verify 失敗修正 実装計画 Implementation Plan

## Goal

main CI の `hygiene`(`.pre-commit-config.yaml` parse error)と
`verify (ubuntu-latest)`(`biome check` exit 1)の root cause を
修正し、verify ubuntu lane 相当のローカル検証を全て Pass にする。

## Scope

単一の実装ストリーム(CI 修復 + 証跡 + doc チェーン)。変更は
lease 内の 4 ファイル + `evidence/al210/` + docs に限定され、
相互依存はあるが独立して ship できる部分はない。

## Upstream Documents

- [docs/specs/issue-210-ci-failure-fix.md](../specs/issue-210-ci-failure-fix.md)
- Issue [#210](https://github.com/scarecrow173/audio-lens/issues/210)
- 失敗 run: <https://github.com/scarecrow173/audio-lens/actions/runs/43540885602>

## Design Inputs

- [docs/designs/issue-210-ci-failure-fix-design.md](../designs/issue-210-ci-failure-fix-design.md)
  (approved)— root cause A(YAML 構文破損)・root cause B(biome
  診断)の分析と修復方針。

## File Map

- `.pre-commit-config.yaml` — `check-ci-workflow` hook の
  `files:` scalar 閉鎖(`$'` 復元)と重複 `check-ci-tracker-workflows`
  ブロック除去。hook 契約は不変。
- `tools/check-ci-workflow.mjs` — `useIndexOf`/`useTemplate`/
  `useOptionalChain`/formatter drift を biome 推奨形へ。
  照合文字列・判定ロジックは不変。
- `tools/check-ci-tracker-workflows.mjs` — `useIndexOf` 修正 +
  `${{ }}` 照合リテラル 3 箇所へ理由付き `biome-ignore`。
- `tools/cinematic-sword-cutscene/src/three/vfx/ParticleSystem.tsx`
  — `getContext("2d")!` を null ガード + throw へ。
- `evidence/al210/` — root cause 分析・検証台帳(新規)。
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/
  issue-210-ci-failure-fix*.md` — doc チェーン(新規)。
- `docs/{specs,designs,plans,tasks,test-specs}/README.md`・
  `docs/impl/ir/README.md` — 管理 index 再生成(生成物)。

## Implementation Sequence

- [x] 失敗 run の一次ログを取得し、hygiene parse error と
      lint-full の biome 診断一覧を確定。
- [x] `.pre-commit-config.yaml` を修復(quote 閉鎖 + 重複除去)。
- [x] `biome check --write --unsafe` で機械的 fix を適用し、
      diff を全件レビュー。
- [x] `ParticleSystem.tsx` の non-null assertion を実行時
      チェックへ書き換え。
- [x] `noTemplateCurlyInString` 3 箇所へ理由付き biome-ignore。
- [x] `mise exec -- prek run --all-files`・`biome check`・
      両契約 script で緑化を確認。
- [x] doc チェーン(spec/design/plan/task/test-spec/ir)作成、
      review-protocol で blocker なしを確認。
- [x] `mise run lint-full`/`test-full`/`fmt`/`check-cli-headless`/
      `verify-toolchain`/`check-ruviform-rev`/
      `check-ruviform-lock` の完走確認。
- [x] `evidence/al210/` の台帳を最終結果で更新。
- [ ] origin/main 取込 → Conventional Commits で論理分割コミット
      → push → 日本語 PR(`Refs #210`)。

## Dependency Graph

| Step | Depends on | Blocks |
|------|------------|--------|
| config 修復 | — | prek 全実行 |
| biome 修正 | — | lint-full Pass |
| doc チェーン | spec/design approved | index 再生成 |
| index 再生成 | doc チェーン | commit |
| 最終検証 | 全修正 | commit/PR |

修正自体は並列可能だが、検証は全修正完了後に一括で行う。

## Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | unsafe fix が semantics を変える | low | med | diff 全件レビュー + 契約 script 再実行で確認済み |
| 2 | biome-ignore が検査を広く握りつぶす | low | med | rule 名・理由明記・対象行限定。`biome check` が 0 診断を確認 |
| 3 | parse 修復で hook 本体の失敗が露出 | low | med | `prek run --all-files` で全 hook 実実行済み(Pass) |
| 4 | lint-full 後続 step に更なる失敗 | med | med | test-full/check-cli-headless 等を lane 相当で実走して確認 |
| 5 | Windows ローカルと ubuntu CI の差異 | low | low | biome/clippy は OS 非依存の静的検査。OS 依存差異は PR 本文に NotRun/差異として記録 |

## Task Breakdown

- `docs/tasks/issue-210-ci-failure-fix.md` — 単一タスク
  (修正規模が 1 コミット群に収まるため分割しない)。

## Verification

実行環境: Windows 11 / mise 管理 tool(prek 0.5.7, biome 2.6.2,
actionlint 1.7.12, node 26.12.1)/ rustc 1.99.1(環境導入、
repo は rust を pin しない)。`MISE_ENV` 未設定 = ローカル標準
profile(CI の `mise.ci.toml` pin とは別)。

- [x] `mise exec -- prek run --all-files` → exit 0、全 hook Pass。
- [x] `mise exec -- biome check` → exit 0、診断 0 件。
- [x] `node tools/check-ci-workflow.mjs` → OK。
- [x] `node tools/check-ci-tracker-workflows.mjs` → OK。
- [x] `mise run fmt` → exit 0。
- [x] `mise run check-cli-headless` → exit 0。
- [x] `mise run verify-toolchain` → exit 0。
- [x] `mise run check-ruviform-lock` → exit 0。
- [x] `mise run check-ruviform-rev` → exit 0。
- [x] `mise run check-ci-workflow` → exit 0。
- [x] `mise run check-ci-tracker-workflows` → exit 0。
- [x] `mise run lint-full` → exit 0。
- [x] `mise run test-full` → exit 0。
- [ ] NotRun: verify-macos/ui-verify-macos/webview-verify-macos/
      gpui-verify-macos の macOS leg、`gui-real`、3OS matrix の
      Windows/Linux 以外 leg、`*-msrv` 系(ローカル実行不可・
      daily-CI scope)。

## Open Questions / Gaps

- `mise install --locked` が user-global mise 設定の未 pin tool
  (pnpm/ffmpeg/python/rust/codex/ccusage/mr-boxington)を理由に
  失敗する。repo 必須 tool は全て install 済みで `mise exec` が
  解決するため実害なし。環境起因として証跡に記録。
- ubuntu CI でのみ顕在化した診断はローカル Windows でも同一
  biome version で再現・解消済み。clippy の OS 差異は残るが
  今回の変更は Rust コードに触れない。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution(単一 worker・逐次)。
- Independent work streams: none。
- User approval required before dispatch: 対象外(単独実行)。

## Review Handoff

- reviewer focus: `biome check --write --unsafe` の diff が
  semantics を保存していること、`biome-ignore` の限定性、
  `.pre-commit-config.yaml` が PR #209 の意図(files に composite
  action 追加)を保持したまま修復されていること。
