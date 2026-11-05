---
id: "TASK-034Smu7rL1jyWzIfwIzmyV"
type: "task"
status: "done"
title: "Full CI 統一 安全な第一段階(AL#158): 全実行化・ci-summary・expected_sha"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/158"
  changes:
    added:
      - type: "file"
        path: "tools/check-ci-workflow.mjs"
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: ".github/actions/verify/action.yml"
      - type: "file"
        path: ".github/actions/ui-verify/action.yml"
      - type: "file"
        path: ".github/actions/webview-verify/action.yml"
      - type: "file"
        path: ".pre-commit-config.yaml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/designs/audiolens-independent-ci.md"
      - type: "file"
        path: "docs/guides/release-macos-verification.md"
      - type: "file"
        path: "docs/designs/impact-based-ci-issue-149.md"
      - type: "file"
        path: "docs/tasks/impact-based-ci-issue-149.md"
    deleted:
      - type: "file"
        path: "tools/classify-impact.mjs"
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-full-ci-ci-summary-issue-158.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-full-ci-ci-summary-issue-158.md"
    - "../designs/full-ci-ci-summary-issue-158.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "https://github.com/scarecrow173/audio-lens/issues/147"
    - "https://github.com/scarecrow173/audio-lens/issues/159"
    - "../tasks/impact-based-ci-issue-149.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-full-ci-ci-summary-issue-158.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Full CI 統一 安全な第一段階(AL#158): 全実行化・ci-summary・expected_sha

## Work

`ci.yml` を Issue #158 の安全な第一段階へ改修する。選択実行機構
(`impact` job・`tools/classify-impact.mjs`・`CI_IMPACT_SELECTIVE`・
Shadow Mode・期待集合照合)を撤去し、3 lane × 3 OS を常時実行、
`expected-sha` gate・`ci-summary` fail-closed 集約を追加する。
Required context `summary`(macOS 非含有の現行 scope)と
pull_request/push trigger は維持する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `../plans/implement-full-ci-ci-summary-issue-158.md`
- `../designs/full-ci-ci-summary-issue-158.md`
- `../specs/full-ci-ci-summary-issue-158.md`

## Dependencies

- なし(単一 worktree 内の一連編集)。

## Affected Files

- `.github/workflows/ci.yml`、`tools/classify-impact.mjs`(削除)、
  `tools/check-ci-workflow.mjs`(新規)、`mise.toml`、
  `.pre-commit-config.yaml`、`.github/actions/*/action.yml`
  (コメントのみ)、docs 同期対象(plan File Map 参照)。

## Done When

- [x] ci.yml が全イベントで全 lane 常時実行となり、impact 機構への
      参照が残らない。
- [x] `expected-sha` job が dispatch 時の `expected_sha` 一致を
      fail-closed で照合する。
- [x] `ci-summary` が全 job を `always()` で fail-closed 集約し、
      Job Summary に SHA/resolved SHA/lane×OS/検証範囲/Run URL を
      記録する。
- [x] `summary` job が macOS 非含有 scope で維持される
      (required context 不死)。
- [x] `check-ci-workflow.mjs` が契約検査として prek hook 経由で
      CI に組み込まれる。
- [x] impact 設計/task が superseded/done に更新され、関連 docs が
      同期される。
- [x] 検証記録(実コマンド + exit code)を本書に記入済み。

## Verification

実行環境: Windows 11 (MINGW64, DESKTOP-NH00KUI) / mise 管理
toolchain (rust 1.99.0, node 26.12.1, uv 0.13.5, actionlint 1.7.12,
biome 2.6.2, prek 0.5.7, git-lfs 3.8.0)

- [x] `mise run check-ci-workflow`
      (`node tools/check-ci-workflow.mjs`) → exit 0
- [x] 負例 fixture(ci.yml を複製して needs 欠落・expected_sha 非
      required・lane→impact 差し替え・summary 改名を個別適用)で
      非 0 exit を確認 → 全負例で exit 1(契約違反検出)
- [x] `mise exec -- actionlint .github/workflows/ci.yml` → exit 0
- [x] `mise exec -- prek run --all-files` → exit 0
      (biome/actionlint/check-ci-workflow 含む全 hook 通過)
- [x] `mise run lint-ci` → exit 0(biome check + cargo check)
- [x] `git grep` で選択実行機構の残滓確認 → コード部に残滓なし
      (ci.yml 内の言及は設計経緯を記す YAML コメントのみ、
      checker はコメント部を対象外とする契約上の意図どおり)
- [x] doc-maintenance apply による管理 index 再生成 → exit 0
      (新規 findings なし、残存 70 件は既存の manual/migration
      findings)

未検証項目: 実 GitHub Actions 上の `needs.<id>.result` 評価・
runner 実行・Job Summary 出力はローカルでは実行不能(Actions
dispatch 禁止)。構造契約は check-ci-workflow.mjs で静的に担保。
macOS runner 実動作も未検証(ローカルは Windows)。
