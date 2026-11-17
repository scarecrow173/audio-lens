---
id: "TASK-034Z0TAlmWUlPYLbNXWQdH"
type: "task"
status: "done"
title: "GPUI 製品 GUI 実 OS E2E 接続 lane(Issue #219)"
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
    - "docs/plans/issue-219-gui-real-e2e.md"
    - "docs/specs/issue-219-gui-real-e2e.md"
    - "docs/designs/issue-219-gui-real-e2e.md"

  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---
# GPUI 製品 GUI 実 OS E2E 接続 lane(Issue #219)

## Work

1. `gpui-real` feature(`gpui` + `ruviform-ui-gpui/real-scenario`)
   と `[[bin]] al-gui-real-scenario`(`required-features`)を追加。
   `gpui-test` との同居を compile error で排他。
2. bin: `desktop_spec`/`source_spec` の `ViewSpec` を
   `ViewHost`/`ViewMount` で `RealWindowMount` 化し、
   `RealOsGuiTest::run` で 1 session・2 window に mount。
   catalog 7 case(semantic 2・OS input 3・visual 2)を
   `run_scenario`/`ScenarioSpec` で実行し verdict を写像。
3. JSONL(`run`+`case` 行、targets 非出力)を `--out`/stdout へ。
   `Fail` のみ exit 1。session 失敗は全件 `NotRun`+
   `environment_blocked` で fail-closed。
4. `--blocked-probe`: window を開かず catalog 全件の形状を
   自己検査、違反時 exit 1。
5. `tests/scenario_real.rs`(`#![cfg(feature = "gpui-real")]`)で
   blocked-probe/引数/catalog を検証。
6. mise `test-ui-gpui-real` / `test-ui-gpui-real-headless` 追加、
   ci.yml `gui-real` の stale note 更新(`status=not_run` 維持)。
7. docs chain + `evidence/al219/` + IR/exp 記録。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-219-gui-real-e2e.md`
- `docs/specs/issue-219-gui-real-e2e.md`
- `docs/designs/issue-219-gui-real-e2e.md`

## Dependencies

- ruviform pin `cbcc8343`(`RealOsGuiTest`/`run_scenario` API)。

## Affected Files

- `crates/al-gui/Cargo.toml`
- `crates/al-gui/src/bin/al-gui-real-scenario.rs`(新規)
- `crates/al-gui/tests/scenario_real.rs`(新規)
- `mise.toml`(追加のみ)
- `.github/workflows/ci.yml`(gui-real note のみ)
- `docs/**/issue-219-gui-real-e2e.md`、`docs/impl/ir|exp/*`
- `evidence/al219/`

## Done When

- [x] `gpui-real` feature/bin が既定非回帰で build される
- [x] bin が実 window mount + RealOs scenario + os_* input +
      capture_frame + JSONL を実装する
- [x] blocked-probe が決定的 fail-closed(exit 0・違反時 1)
- [x] mise lane 2 本が動作し headless probe が exit 0
- [x] ci.yml 構造契約が無変更で通る
- [x] interactive session で live lane が実行可能
      (本 worktree で全 7 case Pass を実測)

## Verification

- [x] `mise run fmt`
- [x] `mise run check|lint|test al-gui`(既定)
- [x] `mise run check|lint|test al-gui --features gpui-real`
- [x] `mise run check|lint|test|build-ui-gpui`(gpui-test 非回帰)
- [x] `mise run check-cli-headless` / `check-ci-workflow` /
      actionlint
- [x] `mise run test-ui-gpui-real-headless`(exit 0)
- [x] `mise run test-ui-gpui-real`(interactive、全 case pass)
