---
id: "TSPEC-eMztySmGDwi7ed2B73x33s"
type: "test-spec"
status: "approved"
title: "ruviform#464 Step C / AL#127 SemanticGuiTest scenario 採用のテスト仕様"
created: "2026-11-09"
updated: "2026-11-09"
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
    - "docs/plans/ruviform-464-scenario-adoption.md"
  derived-by: []
  verifies:
    - "docs/specs/ruviform-464-scenario-adoption.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform#464 Step C / AL#127 SemanticGuiTest scenario 採用のテスト仕様

## Purpose

`SemanticGuiTest` scenario が AL の実画面（desktop jobs・source
waveform/seek）で driver-neutral に動作し、Fake/GPUI 両 adapter で
同一の intent・snapshot を観測し、能力境界が正直に報告されることを
保証する。pin 退行・scenario の driver 依存化・能力の過剰申告・
stale target の黙殺を検出するための仕様。

## Feature: desktop jobs scenario（Fake）

### Rule: semantic gesture が CommandIntent 列を発行する

`tests/scenario.rs::fake_desktop_jobs_scenario_passes_with_honest_report`
は、desktop jobs spec を `FakeGuiTest` に mount し、
`desktop_jobs_scenario`（jobs row select → Run tab select → resume）
を実行する。`drain_intents` が
`JOB_TARGET/select {"row":0}` → `DESKTOP_TARGET/tab-select {"index":1}`
→ `JOB_TARGET/resume {"job_id":...}` の順の `CommandIntent` を返し、
`jobs_selected == Some(0)`、`observe` の snapshot が期待値を持つこと。
`ScenarioReport` は `driver=Fake`・`verdict=Pass`・`lane_status="pass"`。

### Rule: target resolution は WindowId/ViewInstanceId/ComponentKey で行う

`fake_target_resolution_covers_window_view_component_and_pins` は
`TargetQuery::new(window, al_view()).component(JOBS_KEY)` で
`ScenarioTarget` を解決し、`target.window`/`view`/`component` が
pin されることを確認する。未知 component は
`ScenarioError::UnknownComponent`、未知 view は `NoSuchView`、
未知 window は `NoSuchWindow`、`generation` 不一致の query は
`StaleGeneration` を返す。`detach_view` 後の旧 target による
`observe` は `StaleGeneration`、再 mount（新 `AttachEpoch`）後の
旧 target は `StaleEpoch` で失敗し、silent に動かない。

### Rule: 実行不能項目は Unsupported/NotRun を返す

`fake_unrunnable_items_report_unsupported_not_run_not_pass` は
`VisualCapture`/`RealOsPointerOrKeyboard`/`Ime`/`NativePluginDock`
の `require_capabilities` が `ScenarioError::Unsupported` を返し、
`capture_frame` が `CaptureOutcome::Unsupported`、`golden_check` が
`GoldenVerdict::Unsupported` を返すことを確認する。`ScenarioReport`
は `driver=Fake`・`verdict=Unsupported`・`lane_status="unsupported"`
で、`capabilities` に `VisualCapture`/`RealOsPointerOrKeyboard` を
含まない。fake の pass を実機 pass と偽らない。

### Rule: seek gesture は committed value を観測する

`fake_source_seek_scenario_commits_transport_seek` は source screen の
seek control に Begin → Preview → Commit を dispatch し、
`CommandIntent::Commit` が `SEEK_TARGET` 宛で committed plain value
`0.05` を持つことを確認する。`ScenarioReport` は `driver=Fake`・
`verdict=NotRun`（semantic 実行済みだが capability 完全性の
pre-flight は未通過）として記録される。

## Feature: GPUI adapter scenario

### Rule: 同一 scenario が GPUI adapter で Pass する

`tests/scenario_gpui.rs::gpui_desktop_jobs_scenario_passes_with_declared_capabilities`
は `GpuiGuiTest`（`gpui-test` feature）に同一 `desktop_jobs_scenario`
を実行し、Fake と同じ `CommandIntent` 列と `jobs_selected` を観測
する。`gui.driver() == ScenarioDriver::GpuiSimulated`、report は
`verdict=Pass`・`lane_status="pass"`。

### Rule: GPUI adapter の能力境界は semantic + multi-window のみ

同 test は `report.capabilities` が
`{SemanticAction, MultiWindow}` と一致し、
`SimulatedPointerOrKeyboard`・`RealOsPointerOrKeyboard`・`Ime`・
`VisualCapture`・`NativePluginDock` を含まないことを確認する。
`dispatch_input` は hit-test を通らない semantic 注入であり、
simulated pointer / real OS / pixel の証拠へ昇格しない。

### Rule: Fake と GPUI で同一 scenario が同じ証拠を出す

`gpui_and_fake_run_the_same_scenarios_with_equal_evidence` は
同一 spec を 2 window（desktop + source）に mount し、両 adapter で
`desktop_jobs_scenario` と `source_seek_scenario` を実行する。
`CommandIntent` 列・`jobs_selected`・committed plain value が
一致することを確認する。

### Rule: GPUI でも target resolution と stale pin が機能する

`gpui_target_resolution_and_stale_pins_cover_al_view` は
`GpuiGuiTest` で `WindowId`/`ViewInstanceId`/`ComponentKey` 解決・
未知 target の `UnknownComponent`/`NoSuchView`/`NoSuchWindow`・
detach 後の `StaleGeneration` を確認する。

### Rule: GPUI の capture/golden は Unsupported を返す

`gpui_unrunnable_items_report_unsupported_not_pass` は
`GpuiGuiTest` で未申告能力の `require_capabilities` が
`ScenarioError::Unsupported`、`capture_frame` が
`CaptureOutcome::Unsupported`、`golden_check` が
`GoldenVerdict::Unsupported` を返すことを確認する。
`ScenarioReport` は `driver=GpuiSimulated`・`verdict=Unsupported`・
`lane_status="unsupported"` で記録される。

## Examples

- Fake desktop jobs: select row 0 → Run tab → resume → intents
  `[select{row:0}, tab-select{index:1}, resume{job_id}]`、
  verdict Pass。
- Fake source seek: Begin/Preview/Commit → Commit intent
  `SEEK_TARGET`、committed `0.05`、verdict NotRun。
- Fake/GPUI parity: 同一 spec・同一 scenario → 同一 intent 列・
  同一 committed value。
- GPUI unsupported: `VisualCapture` 要求 → `Unsupported` error、
  `capture_frame` → `CaptureOutcome::Unsupported`。
