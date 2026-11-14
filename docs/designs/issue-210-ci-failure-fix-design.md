---
id: "DESIGN-034XpYvyDBAl0hCspnQyXV"
type: "design"
status: "approved"
title: "main CI hygiene/verify 失敗の修正設計"
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
  derives-from:
    - "docs/specs/issue-210-ci-failure-fix.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# main CI hygiene/verify 失敗の修正設計

## Context

main の `ci` run 43540885602(sha `0e2bfee` = PR #209 merge)で
`hygiene` と `verify (ubuntu-latest)` のみが失敗した。一次ログと
repo 現行ファイルの照合により、2 系統の独立した root cause を
特定した。

### root cause A: `.pre-commit-config.yaml` の YAML 構文破損

PR #209 内の commit `0ea1ad6` が `check-ci-workflow` hook の
`files:` 行を更新した際、single-quoted scalar の閉じ `$'` を
脱落させ、さらに `check-ci-tracker-workflows` hook ブロック全体を
誤って重複挿入した。結果として L52 の `'` が閉じられず、後続行が
multiline quoted scalar として解釈されようとして
`line 53 column 9: invalid indentation in multiline quoted scalar`
で parse error となる。prek は config 全体を先に parse するため
hook が 1 件も実行されず exit 2。CI `hygiene` job
(`prek run --all-files`)とローカル pre-commit の双方が死ぬ。

修復は「壊れた編集を取り消して意図された形に戻す」ことであり、
PR #209 が意図した `files:` への composite action path 追加
(`.github/actions/(verify|webview-verify|gpui-verify)/action.yml`)
は保持する。重複した tracker hook ブロックは除去する
(hook id の重複定義は契約違反であり、残す方が誤り)。

### root cause B: `biome check` の診断残存

PR #209 で `lint-full` が Linux 専用 lane に集約され
(`biome check` + `cargo clippy -D warnings`)、あわせて Biome
導入 commit `28ada9c` 以降に蓄積していた JS/TS 診断が
ubuntu leg で初めて顕在化した。診断は 3 ファイルに限定される:

- `tools/check-ci-workflow.mjs`: `useIndexOf`×1、
  `useTemplate`×11、`useOptionalChain`×1、formatter drift×1。
  いずれも PR #209 で追加された composite action 契約検査節
  (L370 以降)と既存 `topBlock`/`summary` 判定部に由来し、
  すべて semantics を変えない機械的変換で解消できる。
- `tools/check-ci-tracker-workflows.mjs`: `useIndexOf`×1 と
  `noTemplateCurlyInString`×3。後者は workflow YAML の
  `${{ github.repository }}`・`${{ secrets.RUVIFORM_DEPLOY_KEY }}`
  等を**照合対象リテラルとして保持する必要がある**文字列で、
  template literal 化は JS の式展開対象となり検査意味を壊す。
- `tools/cinematic-sword-cutscene/src/three/vfx/ParticleSystem.tsx`:
  `noNonNullAssertion`×1(`c.getContext("2d")!`)。canvas 2d
  context は null を返し得るため、実行時チェックへの
  書き換えが正当。

## Scope

### In Scope

- `.pre-commit-config.yaml` の構文修復(quote 閉鎖 + 重複ブロック
  除去)。hook 契約(id/entry/files/pass_filenames)は不変。
- 上記 3 ファイルの biome 診断解消。`biome.jsonc`・rule 設定・
  hook 定義・files フィルタは変更しない。
- `evidence/al210/` の root cause 分析・検証台帳。

### Out of Scope

- gate/検査の弱化全般、Rust コード、workflow、mise 設定、
  ruviform、OS 固有 lane、`*-msrv`。

## Components and Boundaries

- `.pre-commit-config.yaml` — prek hook 定義の正本。構文が
  valid であることが全 hook 実行の前提。
- `tools/check-ci-workflow.mjs` / `tools/check-ci-tracker-workflows.mjs`
  — ci.yml / tracker workflow の構造契約を fail-closed で照合する
  依存ゼロ script。照合文字列は契約そのものであり変更禁止。
- `tools/cinematic-sword-cutscene/src/three/vfx/ParticleSystem.tsx`
  — Remotion scene の sprite texture 生成。`getContext("2d")` の
  null を honest に扱う。
- `biome.jsonc` — lint/format 正本(変更しない)。

## Data and Control Flow

- hygiene: `prek run --all-files` → config parse → 各 hook
  (builtin + local)を全ファイルへ適用。
- verify lint: `mise run lint-full` → `biome check`(JS/TS/JSON)
  → `cargo clippy --workspace --all-targets -D warnings`。
- 契約 script は両 hook(prek)と mise task の双方から呼ばれ、
  修正後も `check-ci-workflow: ... OK` / `check-ci-tracker-workflows:
  ... OK` を出力し続けることが semantics 保存の証拠。

## Non-Functional Constraints

- 検査の fail-closed 性を維持する: 文字列照合・判定ロジック・
  hook の発火条件を緩めない。
- `noTemplateCurlyInString` の 3 箇所は template literal 化が
  構文的に不可能/意味破壊的であるため、理由付き `biome-ignore`
  コメントで抑制する(照合文字列は 1 文字も変えない)。
  これは rule の off 化ではなく、当該リテラルへの限定抑制である。
- `useTemplate`/`useOptionalChain`/`useIndexOf` は biome の
  unsafe fix が semantics を保存することを各サイトで確認した
  上で適用する(`biome check --write --unsafe` の出力を diff で
  全件レビュー)。

## Risks and Trade-offs

- `biome check --write --unsafe` が意図しない fix を適用する
  リスク → 適用後に `git diff` で全変更をレビューし、診断一覧
  (CI ログで 19 件確定)と 1 対 1 で照合する。
- `biome-ignore` の抑制が広すぎるリスク → rule 名と理由を
  明記し、当該行のみに限定する。
- `ParticleSystem.tsx` で throw に変えた場合の挙動差 →
  `!` の場合も `ctx.createRadialGradient` で TypeError となる
  ため、明示 Error への置き換えは悪化ではなく診断可能性の
  向上。silent fallback(テクスチャ無しで継続)は描画を壊すため
  採用しない。
- parse 修復で hook 本体の失敗が新たに露出するリスク →
  `prek run --all-files` で全 hook の実実行を確認済み。

## Deferred Design Concerns

- なし。

## Verification Notes

- [x] `mise exec -- prek run --all-files` exit 0(全 hook Pass)。
- [x] `mise exec -- biome check` exit 0(診断 0 件)。
- [x] `node tools/check-ci-workflow.mjs` /
      `node tools/check-ci-tracker-workflows.mjs` が OK を維持。
- [x] `mise run lint-full`/`test-full`/`fmt`/`check-cli-headless`/
      `verify-toolchain`/`check-ruviform-rev`/`check-ruviform-lock`
      の exit 0(plan の Verification Matrix で確定)。
