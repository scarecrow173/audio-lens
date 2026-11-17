---
id: "TSPEC-034Z0TNTa3fbvzgGIfWd6y"
type: "test-spec"
status: "approved"
title: "GPUI 実 OS E2E lane の検証契約"
created: "2026-11-17"
updated: "2026-11-17"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-219-gui-real-e2e.md"
      - type: "doc"
        path: "docs/designs/issue-219-gui-real-e2e.md"
      - type: "doc"
        path: "docs/plans/issue-219-gui-real-e2e.md"
      - type: "doc"
        path: "docs/tasks/issue-219-gui-real-e2e.md"
      - type: "doc"
        path: "docs/test-specs/issue-219-gui-real-e2e.md"
      - type: "doc"
        path: "docs/impl/ir/gpui-gui-os-e2e-lane-issue-219.md"
      - type: "doc"
        path: "docs/impl/exp/issue-219-gui-real-e2e-exp.jsonl"
      - type: "file"
        path: "crates/al-gui/src/bin/al-gui-real-scenario.rs"
      - type: "file"
        path: "crates/al-gui/tests/scenario_real.rs"
      - type: "file"
        path: "evidence/al219/gui-real-scenario.jsonl"
      - type: "file"
        path: "evidence/al219/gui-real-headless.jsonl"
      - type: "file"
        path: "evidence/al219/README.md"
    modified:
      - type: "file"
        path: "crates/al-gui/Cargo.toml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
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
    - "docs/designs/issue-219-gui-real-e2e.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-219-gui-real-e2e.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---
# GPUI 実 OS E2E lane の検証契約

## Purpose

`al-gui-real-scenario` lane が「実行したものだけを正直に記録する」
契約を固定する。headless/semantic の成功を実機 Pass へ昇格しない
こと、blocked を fail にしないこと、catalog が drift しないことを
機械検査可能にする。

## Feature

実 OS E2E lane は、catalog 全 case について `ScenarioReport` を
`ScenarioDriver::RealOs` で生成し、判定語彙
(Pass/Fail/NotRun/Unsupported/Blocked)と environment 記録
(`environment_kind`)を証拠として出力しなければならない。

## Rules

- Rule: blocked/headless 記録は `NotRun` +
  `observed.environment_blocked` であり、`environment_kind` に
  `real-window` を含めてはならない。
- Rule: `Fail` verdict のみが exit 1 を引き起こす。
  `NotRun`/`Unsupported` は lane 失敗ではない。
- Rule: catalog の scenario/case id・required capability は
  `CASES` 表を正本とし、blocked/live 両経路で一致する。
- Rule: `--blocked-probe` は window を開かず決定的に動作し、
  形状違反を exit 1 で報告する。
- Rule: 未知の引数・`--out` の path 欠落は exit 2。

## Examples

- Example: headless host で `--blocked-probe` → run header 1 行 +
  7 case 行、全て `verdict=not_run`、`driver=real-os`、
  `observed.observed=environment_blocked`、
  `environment_kind != real-window`、exit 0。
- Example: interactive session で live lane → session が開始
  すれば各 case が実測 verdict を持ち、
  `environment_kind=real-window`。session 開始失敗
  (`UiError::Host`)なら全件 `not_run`+`environment_blocked`。
- Example: `al-gui-real-scenario --bogus` → exit 2、出力なし。

## Guarantees

- 実行していない検証を Pass と記録しない。
- pixel/実入力の無い経路で `real-window` 環境を主張しない。
- 既定 feature・gpui-test lane の挙動を変えない。

## Non-goals

- live lane の oracle 値そのものの正しさ(それは scenario helper
  と view 契約の test が担保)。
- 実走 Pass の Issue 受入判定(環境 gate・人間判断)。

## Risk

- catalog drift・environment_kind の誇張・exit code の
  誤用があると、headless 証拠を実機 Pass と誤読する危険がある。
