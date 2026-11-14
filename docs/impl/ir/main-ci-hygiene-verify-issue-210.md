---
id: "IMPL-034Xq29FlHgba07R8zdOkr"
type: "impl"
status: "in-progress"
title: "main CI hygiene/verify 失敗修正(Issue #210)"
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
    modified:
      - type: "file"
        path: ".pre-commit-config.yaml"
      - type: "file"
        path: "tools/check-ci-workflow.mjs"
      - type: "file"
        path: "tools/check-ci-tracker-workflows.mjs"
      - type: "file"
        path: "tools/cinematic-sword-cutscene/src/three/vfx/ParticleSystem.tsx"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-210-ci-failure-fix.md"
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
  derives-from:
    - "docs/plans/issue-210-ci-failure-fix-plan.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# main CI hygiene/verify 失敗修正(Issue #210)

## Summary

main の `ci` run 43540885602(sha `0e2bfee` = PR #209 merge)で
失敗した `hygiene` と `verify (ubuntu-latest)` の root cause を
修正した。`.pre-commit-config.yaml` の YAML 構文破損を修復し、
`biome check` の全診断(error 1・warning 5・info 13)を
lint を弱めない形で解消した。証跡は `evidence/al210/` に記録。

## Context

`docs/tasks/issue-210-ci-failure-fix.md` /
`docs/plans/issue-210-ci-failure-fix-plan.md` /
`docs/specs/issue-210-ci-failure-fix.md` /
`docs/designs/issue-210-ci-failure-fix-design.md` に従う。
ci-issue-tracker 管理 Issue #210 の失敗 run を対象とする
是正タスク。

## Implementation

### Changes

- `.pre-commit-config.yaml`: L52 `check-ci-workflow` hook の
  `files:` single-quoted scalar を閉鎖(`\.mjs` → `\.mjs$'`)、
  誤挿入された `check-ci-tracker-workflows` 重複ブロック
  (stray `pass_filenames: false` + コメント + hook 定義の
  二重化)を除去。hook の id/entry/files/pass_filenames 契約は
  PR #209 の意図(composite action path 追加を保持)どおり不変。
- `tools/check-ci-workflow.mjs`: biome の unsafe fix を適用
  (`findIndex` → `indexOf`、文字列連結 → template literal、
  `!needs || !needs.some()` → `!needs?.some()`)し、
  formatter drift を `biome check --write` で解消。
  照合文字列・判定ロジックは不変。
- `tools/check-ci-tracker-workflows.mjs`: `findIndex` →
  `indexOf`。`${{ }}` GitHub 式を含む照合リテラル 3 箇所
  (`TRACKER_GROUP`・`ref:` pin・`ssh-key:`)に理由付き
  `biome-ignore lint/suspicious/noTemplateCurlyInString` を付与。
  template literal 化は JS 式展開で検査意味を壊すため不採用。
- `tools/cinematic-sword-cutscene/src/three/vfx/ParticleSystem.tsx`:
  `c.getContext("2d")!` を `const ctx = c.getContext("2d");
  if (!ctx) throw new Error(...)` へ。silent fallback は
  描画を壊すため不採用。
- `evidence/al210/`: root cause 分析・検証台帳を新規作成。
- doc チェーン(spec/design/plan/task/test-spec/ir)を作成し、
  影響 dir の管理 index を `doc_maintenance.js index apply` で
  再生成。

### Decisions During Implementation

- `biome check --write --unsafe` で機械的 fix を一括適用し、
  `git diff` で全件レビュー。unsafe fix が提案された
  `useIndexOf`/`useTemplate`/`useOptionalChain` はいずれも
  semantics 保存を確認(indexOf は等価比較、optional chain は
  null/undefined で同値、template literal は同一文字列生成)。
- `noTemplateCurlyInString` は rule off ではなく、当該リテラル
  への `biome-ignore` + 理由コメントで限定抑制。照合文字列は
  1 文字も変更していない(diff で確認可能)。
- `getContext("2d")` の失敗時は throw。`!` でも後続の
  `createRadialGradient` で TypeError となるため、明示 Error は
  診断可能性の向上であり挙動悪化ではない。
- `mise install --locked` は user-global mise 設定の未 pin tool
  を理由に失敗したが、repo 必須 tool は全て導入済みで
  `mise exec` が解決するため環境起因として記録のみ。

## Related Experiments

### Adopted

- なし(機械的修復のため実験ログなし)。

### Rejected

- なし。

## Validation

実行環境: Windows 11 / mise 管理 tool(prek 0.5.7, biome 2.6.2,
actionlint 1.7.12, node 26.12.1)/ rustc 1.99.1(環境導入、
`MISE_ENV` 未設定 = ローカル標準 profile)。

- `mise exec -- prek run --all-files` → exit 0、全 hook Pass
  (修正前は config parse error で hook 未実行のまま exit 2 を
  CI ログで確認)。
- `mise exec -- biome check` → exit 0、診断 0 件(修正前は
  exit 1、error 1/warning 5/info 13 をローカル再現)。
- `node tools/check-ci-workflow.mjs` → `... OK`。
- `node tools/check-ci-tracker-workflows.mjs` → `... OK`。
- `mise run fmt` → exit 0。
- `mise run check-cli-headless` → exit 0。
- `mise run verify-toolchain` → exit 0。
- `mise run check-ruviform-lock` → exit 0。
- `mise run check-ruviform-rev` → exit 0。
- `mise run check-ci-workflow` → exit 0。
- `mise run check-ci-tracker-workflows` → exit 0。
- `mise run lint-full` → exit 0(biome check + clippy
  `-D warnings`)。
- `mise run test-full` → exit 0(`cargo test --workspace`、
  全 test Pass・0 failed)。

## Risks

- `biome-ignore` による抑制が将来の同種リテラル追加時に
  参照されない可能性 → 新規 `${{ }}` リテラルは別途対応が必要。
  現状 3 箇所のみ。
- ubuntu CI と Windows ローカルの差異(clippy の OS 依存 lint、
  Rust toolchain pin)は残る。今回の変更は Rust/JS の静的検査
  範囲であり OS 依存箇所に触れないが、merge 後 CI で確認が必要。

## Follow-ups

- merge 後の main SHA に対する `ci` run で `hygiene`・`verify`
  を含む全必須 job の success を確認し、ci-issue-tracker が
  Issue #210 を auto-close すること(Coordinator が確認)。
- `mise install --locked` が user-global tool で失敗する件は
  環境起因。repo 側の対応は不要と判断。
