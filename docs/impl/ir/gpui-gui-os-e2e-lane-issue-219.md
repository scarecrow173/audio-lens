---
id: "IMPL-034Z0TOzRf8EG8PhLYl8aD"
type: "impl"
status: "completed"
title: "GPUI 製品 GUI 実 OS E2E 接続 lane(Issue #219)"
created: "2026-11-17"
updated: "2026-11-18"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-219-gui-real-e2e.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-219-gui-real-e2e-exp.jsonl"
    rejected: []
---
# GPUI 製品 GUI 実 OS E2E 接続 lane(Issue #219)

## Summary

`gpui-real` feature と test-only bin `al-gui-real-scenario` を
追加し、製品 `ViewSpec` を `RealOsGuiTest::run` の実 OS window
へ in-process mount して driver-neutral scenario・実 OS 入力・
実 pixel capture を `ScenarioDriver::RealOs` の `ScenarioReport`
として JSONL 証拠に記録する opt-in lane を実装した。
`--blocked-probe` による決定的 fail-closed 自己検証と、
interactive session 前提の `mise run test-ui-gpui-real` lane
を整備した。

## Context

`docs/tasks/issue-219-gui-real-e2e.md` ← plan ← spec/design。
AC1 解釈は spec R6 で確定: 「製品 GUI を実 OS で起動・駆動」を
製品 `ViewSpec` の in-process `RealWindowMount` mount と解釈
(`scarecrow173/ruviform#631` AC2 の driver-neutral 投影と整合)。

## Implementation

### Changes

- `crates/al-gui/Cargo.toml`: `gpui-real = ["gpui",
  "ruviform-ui-gpui/real-scenario"]` feature と
  `[[bin]] al-gui-real-scenario`(`required-features`)を追加。
  `gpui-test`/`gpui-real` 同居を `compile_error!` で排他
  (bin 内 cfg)。
- `crates/al-gui/src/bin/al-gui-real-scenario.rs`(新規):
  `tests/common/scenario.rs` を `#[path]` で取込み、
  `desktop_spec`/`source_spec` の `ViewSpec` を `ViewHost`/
  `ViewMount` で `RealWindowMount` 化。`RealOsGuiTest::run` で
  1 session・2 window(desktop→source)に mount し、catalog 7
  case を `run_scenario`/`ScenarioSpec` で実行。JSONL
  (`run`+`case`、targets 非出力)を `--out`/stdout へ出力。
  `Fail` のみ exit 1、引数不正は exit 2。session 失敗は
  `blocked_reports` で全件 `NotRun`+`environment_blocked`。
  `--blocked-probe` は window を開かず catalog 形状を検査。
- `crates/al-gui/tests/scenario_real.rs`(新規):
  `#![cfg(feature = "gpui-real")]`。`CARGO_BIN_EXE_` 経由で
  blocked-probe JSONL・stdout 形状・引数 exit 2・catalog id
  固定の 5 test。
- `mise.toml`: `test-ui-gpui-real`(interactive 実走)と
  `test-ui-gpui-real-headless`(blocked-probe)を追加のみ。
- `.github/workflows/ci.yml`: `gui-real` job の stale note
  (「Ruviform #312/#313 未接続」)を pin `cbcc8343` 接続済み・
  opt-in/interactive 実走待ちの表記へ更新。`status=not_run`
  出力と job 構造は不変(`check-ci-workflow` 無変更で通過)。
- `evidence/al219/gui-real-scenario.jsonl` /
  `gui-real-headless.jsonl`: lane 実走証拠。

### Decisions During Implementation

- scenario helper は lib 昇格せず `#[path]` で bin に取込む
  (driver-neutral で bin が唯一の real consumer。lib 昇格は
  Fake/gpui-test lane の import を書き換える)。
- semantic 2 case は共有 helper の outcome を oracle 化
  (intent 列一致・committed 値)して `Pass`/`Fail` を判定。
- OS input case は `OsInputOutcome` を verdict に写像し、
  `Injected` のみ oracle 判定へ進める。keyboard `End` は
  component focus 前提のため focus 確立 click も証拠に含め、
  focus 不成立時は `NotRun`/`Unsupported` に落とす。
- `environment_kind` は `real-window`(live session)と
  `deterministic-backend`(blocked)を分離し、blocked 記録が
  実環境を主張しないことを probe が検査する。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-219-gui-real-e2e-exp.jsonl` — blocked
  probe の fail-closed 設計・verdict 写像・JSONL 射影を採用。

### Rejected

- `al-desktop` 別プロセス駆動 — driver-neutral 投影と不整合の
  ため不採用(spec R6)。
- scenario helper の lib 昇格 — 既存 lane への波及が大きく
  本 slice の範囲を超えるため不採用。

## Validation

- `mise run fmt` — pass
- `mise run check al-gui` / `lint al-gui` / `test al-gui`
  (既定 feature) — pass(非回帰)
- `mise run check al-gui --features gpui-real` /
  `lint al-gui --features gpui-real` /
  `test al-gui --features gpui-real` — pass
  (scenario_real 5 test 含む全 pass)
- `mise run check-ui-gpui` / `lint-ui-gpui` /
  `test-ui-gpui` / `build-ui-gpui` — pass(非回帰)
- `mise run check-cli-headless` — pass
- `mise run check-ci-workflow` + actionlint — pass
- `mise run test-ui-gpui-real-headless` — exit 0、
  全 7 case `not_run`+`environment_blocked`
- `mise run test-ui-gpui-real` — exit 0、interactive Windows
  session で全 7 case `pass`・`environment_kind=real-window`
  (`evidence/al219/gui-real-scenario.jsonl`)

## Risks

- live lane の Pass は実行環境依存 — headless/非対話 host では
  `UiError::Host`→全件 NotRun が正しい結果。実走証拠は
  `evidence/al219/gui-real-scenario.jsonl` の 1 回分であり、
  Issue #219 の受入確定は別途。
- `#[path]` 取込みは helper の bin への結合を生む — helper が
  lib 化された場合は import 先の見直しが要る。

## Follow-ups

- Issue #219 の最終受入判断(実走証拠の十分性・interactive
  session での再現確認)。
- window close/DPI case(pin bump `5f0cd43f` 以降が必要)。
- macOS/Linux GUI-L2(Deferred)。
