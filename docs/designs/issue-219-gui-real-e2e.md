---
id: "DESIGN-034Z0TyUt7pmgy9bnozhRA"
type: "design"
status: "approved"
title: "GPUI 製品 GUI 実 OS E2E 接続 lane 設計"
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
    - "docs/specs/issue-219-gui-real-e2e.md"

  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---
# GPUI 製品 GUI 実 OS E2E 接続 lane 設計

## Context

Ruviform pin `cbcc8343` が `RealOsGuiTest`(real OS window adapter)
・`RealWindowMount`/`RealWindowOptions`・`run_scenario`/
`ScenarioSpec`/`ScenarioReport`・`SemanticGuiTest`/
`VisualGuiTest` を提供する。AudioLens 側には driver-neutral
scenario helper(`tests/common/scenario.rs`)と `gpui-test`
simulated lane(`scenario_gpui.rs`)が既存。`gpui-test` と
`real-scenario` は同一 session で同居不可(ruviform-ui-gpui
注記: test platform が OS surface を stub する)ため、別 feature
`gpui-real` で分離する。

## Scope

### In Scope

- `gpui-real` feature、`al-gui-real-scenario` bin、case catalog、
  JSONL renderer、blocked-probe、mise lane、ci.yml note 更新。

### Out of Scope

- ruviform runner 本体、window close/DPI case、macOS/Linux、
  CI 自動 GUI 受入、製品 GUI 本体の変更。

## Components and Boundaries

- `crates/al-gui/src/bin/al-gui-real-scenario.rs` — lane の全責務
  (mount 組立・case 実行・verdict 写像・JSONL・引数処理)を持つ
  test-only bin。bin は `tests/common/scenario.rs` を
  `#[path]` で取り込み、driver-neutral helper を再利用する
  (helper の lib 昇格は既存 test 構造を変えるため採用しない)。
- `tests/common/scenario.rs` — 変更なし。`desktop_spec`/
  `source_spec`/`desktop_jobs_scenario`/`source_seek_scenario`/
  `command_actions`/`al_view` をそのまま利用。
- `crates/al-gui/tests/scenario_real.rs` — `gpui-real` gate の
  integration test。`CARGO_BIN_EXE_al-gui-real-scenario` 経由で
  blocked-probe・引数検証・JSONL 形状を検査。live window は
  開かない。

## Data and Control Flow

1. `desktop_spec()`/`source_spec()` で製品 `ViewSpec` を 2 系統
   生成し、`ViewHost::new(al_view(), epoch)` → `host.mount(spec)`
   → `ViewMount{host, theme: None}` で `RealWindowMount` を 2 件
   構成(`al-desktop`/`scenario_gpui` と同型)。
2. `RealOsGuiTest::run(mounts, RealWindowOptions::default(), body)`
   で 1 session・2 window(desktop → source、mount 順=WindowId
   昇順)を開く。失敗時は `blocked_reports` へ写し fail-closed。
3. case catalog(7 件)を `run_scenario`/`ScenarioSpec` で順に実行:
   - semantic 2 件: `desktop_jobs_scenario`/`source_seek_scenario`
     を `SemanticGuiTest` port で実行し、intent 列/committed 値を
     oracle とする。
   - OS input 3 件: `os_input_preflight`、Seek への実 pointer
     click、focus 確立後の実 `End` key。`OsInputOutcome` の
     `EnvironmentBlocked`→`NotRun`+`environment_blocked`、
     `Unsupported`→`Unsupported`、`NotRun`→`NotRun` に写像し、
     `Injected` のみ oracle 判定へ進む。
   - visual 2 件: `capture_frame` の `Captured`/`Unsupported`/
     `NotRun` を verdict に写像。`Captured` は
     `last_capture_meta`(backend/寸法/scale)を証拠記録。
4. `ScenarioReport` を `run` header + `case` 行の JSONL に射影
   (`targets` は session-local id のため出さない — plugin-dock
   先例)。`--out` または stdout。
5. exit code: `Fail` verdict があれば 1、なければ 0。
   `--blocked-probe` は catalog 全件の NotRun/environment_blocked
   形状を検査し、違反時 1。

## Non-Functional Constraints

- 決定性: `--blocked-probe` は window を開かず常に同一形状の
  JSONL を出す(repo_sha のみ環境依存、`null` 許容)。
- 正直性: semantic dispatch を実入力/pixel 証拠へ昇格しない。
  `environment_kind` は session 実走時のみ `real-window`。
- 追加性: 既定 feature・`gpui-test` lane・CI Required gate を
  変更しない。

## Risks and Trade-offs

- `#[path]` による `tests/common` 取込みは test helper の bin
  への逆流に見えるが、helper は driver-neutral であり bin が
  唯一の real consumer。lib 昇格は Fake/gpui-test 両 lane の
  import を書き換えるため本 slice では採用しない。
- window の foreground/focus は OS 依存 — `request_focus` 失敗
  時も `os_*` op が `EnvironmentBlocked`/`NotRun` を返すため
  verdict は正直に落ちる(focus は case の被検査対象ではない)。
- keyboard `End` の意味は component focus 依存 — focus 確立の
  pointer click が `Injected` でない場合は前提不成立として
  `NotRun`/`Unsupported` にし、効果不明のまま Fail にしない。

## Deferred Design Concerns

- 別プロセス `al-desktop` 駆動 | in-process mount が driver-
  neutral 投影と整合するため不要 | `scarecrow173/ruviform#631`
  の driver-neutral 要件が変わった場合 | 実アプリ起動経路の
  証拠が取れない
- window close/DPI case | pin 後着 API | ruviform pin bump |
  観測面が無い case は書けない

## Verification Notes

- `mise run test al-gui --features gpui-real` で
  `scenario_real.rs` の 5 test(blocked-probe JSONL/形状・引数
  exit 2・catalog id 固定)が pass。
- `mise run test-ui-gpui-real-headless` が exit 0・全 case
  `not_run`+`environment_blocked`。
- `mise run test-ui-gpui-real`(interactive session)が全 7 case
  `pass`・`environment_kind=real-window` の JSONL を出力
  (`evidence/al219/gui-real-scenario.jsonl`)。
