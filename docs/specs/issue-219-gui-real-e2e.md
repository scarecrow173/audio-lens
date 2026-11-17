---
id: "SPEC-034Z0TmxoCht7pvLdnKbkE"
type: "spec"
status: "approved"
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
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-219-gui-real-e2e.md"
  references: []
  defers: []
  deferred-by: []
---
# GPUI 製品 GUI 実 OS E2E 接続 lane(Issue #219)

## What Should Be Built

AudioLens 製品 GUI(`desktop_screen`/`source_screen` の `ViewSpec`)を
Ruviform pin `cbcc8343` の `RealOsGuiTest` adapter 経由で実 OS
(Windows)window に mount し、driver-neutral scenario を
`ScenarioDriver::RealOs` として実行する opt-in E2E lane。
`al-gui-real-scenario` test-only bin(`gpui-real` feature)が
semantic scenario・実 OS 入力(pointer/keyboard)・実 pixel capture
の各 case を `ScenarioReport` として JSONL 証拠に記録する。

## Why Now

Issue #189 の Windows GUI-L2 受入には、製品 ViewSpec が実 OS
window 上で起動・遷移・描画・実入力される E2E 証拠が前提となる。
Ruviform 側 runner(`scarecrow173/ruviform#631`)は pin 済みだが、
consumer 側の接続 lane が存在しない。本 slice はその接続実装で
あり、最終受入の実走は interactive session の環境 gate として残る。

## Users and Value

- 開発者/レビュア: `evidence/al219/*.jsonl` の実測 verdict
  (Pass/Fail/NotRun/Unsupported)で GUI-L2 受入の証拠を確認できる。
- CI: headless でも blocked-probe が決定的に fail-closed 記録を
  出し、実 GUI 検証の代用にならないことを機械的に示せる。

## Scope

### In Scope

- `gpui-real` feature と `al-gui-real-scenario` test-only bin
  (`required-features = ["gpui-real"]`)。
- 製品 `ViewSpec` の `RealWindowMount` mount、共有 scenario helper
  の `run_scenario` 実行、`os_*` 実入力 case、`capture_frame`
  pixel case、JSONL 出力(`--out`/stdout)、`--blocked-probe`。
- opt-in mise lane 2 本(`test-ui-gpui-real` /
  `test-ui-gpui-real-headless`)と ci.yml `gui-real` job の
  stale note 更新(`status=not_run` 維持)。
- headless/非対話 host での全 case `NotRun` +
  `observed.environment_blocked` fail-closed 記録。

### Out of Scope

- `al-desktop` 別プロセス起動を要求する解釈(AC1 解釈は
  in-process mount を採用 — 下記 Requirements 参照)。
- window close/DPI 観測 case(pin 後着 API、pin bump 要)。
- macOS/Linux の GUI-L2(Deferred)。
- CI での自動 GUI 受入 lane 化(実 OS window は CI runner で
  開けない — opt-in のまま)。
- 製品 GUI 本体の変更。

## Requirements

- R1: `gpui-test` と `gpui-real` は同一 session で同居不可
  (ruviform-ui-gpui 注記) — feature 組合せは compile error で
  排他する。
- R2: 実走証拠は `environment_kind=real-window` を名乗れるのは
  `RealOsGuiTest::run` が実際に session を開始した場合のみ。
  blocked/headless 記録は `deterministic-backend` を名乗り、
  `real-window` を主張しない。
- R3: `UiError::Host` 等 session 開始失敗時は catalog 全 case を
  `NotRun` + `observed.environment_blocked` で記録し exit 0
  (fail-closed)。
- R4: `Fail` verdict のみ exit 1。`NotRun`/`Unsupported` は
  正直な記録であり lane 失敗ではない。
- R5: pixel 証拠は `VisualGuiTest::capture_frame` の実 OS
  window readback のみ。semantic snapshot/draw plan からの
  昇格はしない。
- R6: AC1 解釈 — 「製品 GUI を実 OS で起動・駆動」は、製品
  `ViewSpec` を `RealOsGuiTest::run` で in-process の実 OS
  window へ mount する方式と解釈する。根拠:
  `scarecrow173/ruviform#631` AC2「同 Runner から AudioLens
  実 GUI へ Driver-neutral にシナリオを投影」は ViewSpec mount
  モデルと整合し、`tests/common/scenario.rs` の driver-neutral
  helper がそのまま適用できる。`al-desktop` 別プロセス起動は
  driver-neutral 投影を迂回するため採用しない。

## Acceptance Criteria

- [x] `gpui-real` feature + test-only bin が追加され、既定
  feature の check/lint/test が非回帰である。
- [x] bin が製品 ViewSpec を実 OS window に mount し、共有
  scenario を `ScenarioDriver::RealOs` で実行する。
- [x] `os_input_preflight`/`os_press_key`/`os_pointer_move_to`/
  `os_mouse_down`/`os_mouse_up`/`capture_frame` を使う case が
  bin 側に記述されている。
- [x] 全 case の `ScenarioReport` が lane/run_id/environment
  付き JSONL で出力される。
- [x] 非対話・headless host で全 case `NotRun` +
  `environment_blocked` の fail-closed・exit 0。
- [x] `--blocked-probe` が window を開かず全 report の形状を
  自己検査し、違反時 exit 1。
- [x] opt-in mise lane 2 本が追加され、headless blocked-probe
  が exit 0 で完走する。
- [x] ci.yml `gui-real` job が `status=not_run` を維持しつつ
  stale note が更新され、構造契約検査が無変更で通る。
- [ ] 実 OS での起動・駆動の最終 Pass 実走証拠
  (interactive Windows desktop session — 本 slice の環境 gate。
  本 worktree では `mise run test-ui-gpui-real` が全 7 case
  Pass を記録したが、これは本 slice 実行環境での 1 回の実測
  であり Issue #219 の受入確定は別途判断)。

## Deferred Design Concerns

- window close/DPI 観測 case | pin 後着 API(`5f0cd43f`)が必要
  で pin bump + `Cargo.lock` 再生成を伴う | ruviform pin 更新
  時 | 現 pin では観測面が無く case を正直に書けない
- macOS/Linux GUI-L2 | Deferred(POLICIES §2) | 各 OS の
  interactive session 整備時 | Windows のみの証拠では
  cross-OS 受入を満たせない

## Sources and Relations

- Issue: audio-lens#219(本 spec の対象 Issue)
- Upstream runner: `scarecrow173/ruviform#631`
- Policy: POLICIES.md §2.1(実 GUI 検証の代用禁止)・§4・§5
