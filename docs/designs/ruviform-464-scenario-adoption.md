---
id: "DESIGN-6qppTQVCUfffmR60C18lKG"
type: "design"
status: "approved"
title: "ruviform#464 Step C / AL#127 SemanticGuiTest scenario 採用の設計"
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
  related:
    - "docs/plans/ruviform-464-scenario-adoption.md"
    - "docs/test-specs/ruviform-464-scenario-adoption.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/ruviform-464-scenario-adoption.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform#464 Step C / AL#127 SemanticGuiTest scenario 採用の設計

## Context

ruviform#464（merge `57e4e8ce`）で `ruviform-ui-test::scenario` が
consumer 中立の `SemanticGuiTest` port として提供された。`FakeGuiTest`
（`ViewHost` backed）と `GpuiGuiTest`（`GpuiObserver` backed、
`gpui-test` feature）が同一 contract を実装し、
`parity_gesture_scenario` が Begin → Preview → Commit の共通
semantic gesture を提供する。#314（merge `2230018e`/`7ae916b3`）で
`scenario::visual`（`capture_frame`/`golden_check`）が接続され、
capture backend 不在時は `CaptureOutcome::Unsupported` /
`GoldenVerdict::Unsupported` を返す。

AudioLens 側は既に driver-neutral な ViewSpec projection
（`al_gui::view::screen::source_screen`、desktop workspace tabs、
jobs/run/dataset screen）を持ち、`gpui_parity.rs` で Fake/GPUI の
個別 parity を検証済み。本設計はそれを scenario contract へ移す。

## Scope

- `crates/al-gui/tests/common/scenario.rs`: AL 固有の fixture
  （desktop jobs spec、source seek spec）と driver-neutral scenario
  関数（`desktop_jobs_scenario`、`source_seek_scenario`）、および
  `ScenarioReport` 組み立て helper（`al_scenario_report`）を定義。
  Ruviform 側の共有実装は複製しない。
- `crates/al-gui/tests/scenario.rs`（headless、gpui feature 不要）:
  `FakeGuiTest` での semantic gesture pass、target resolution
  （WindowId/ViewInstanceId/ComponentKey/AttachEpoch/generation）、
  stale target error、capability preflight、Unsupported/NotRun の
  正直な報告を検証。
- `crates/al-gui/tests/scenario_gpui.rs`（`#![cfg(feature = "gpui-test")]`）:
  `GpuiGuiTest` で同一 scenario を実行し、Fake/GPUI の intent・
  snapshot 一致、`GpuiSimulated` 能力境界、capture/golden の
  Unsupported を検証。GPUI 型はこのファイルと mount helper に閉じる。
- `Cargo.lock`: ruviform pin を `84d21641` へ更新（
  `tools/update-ruviform-rev.mjs` 経由）。

## Non-Goals

- 製品コード（`src/`）の公開 API 変更は行わない。scenario 採用に
  必要な接続は既存の ViewSpec/`ViewHost`/`GpuiObserver` 経路で足りる。
- JSON DSL、第 2 test executor、新しい gesture/state authority、
  semantic contract の重複定義は追加しない。
- 実 OS window での実入力・IME・DPI・pixel golden の検証は本タスクの
  範囲外（別 Issue の独立 gate）。
- draw plan 観測を pixel 証拠として扱わない。

## Design Decisions

### D1: shared scenario は `tests/common/scenario.rs` に置く

`tests/scenario.rs` と `tests/scenario_gpui.rs` の両方から
`mod common` 経由で同一 scenario 関数を呼ぶ。これにより「同一
scenario が両 driver で同じ CommandIntent を発行する」ことを
構造的に保証する。Ruviform の `parity_gesture_scenario` は
Begin/Preview/Commit の定型を提供するが、AL の jobs 選択→tab
切替→resume のような複合 intent 列は AL 側 scenario が組み立てる。

### D2: target は ComponentKey で動的解決する

`SEEK_TARGET`/`JOBS_KEY` 等の安定 `ComponentKey` を使い、
`TargetQuery::new(window, view).component(key)` で
`ScenarioTarget` を解決する。表示文字列や pixel 座標には依存しない。
解決後の `WindowId`/`ViewInstanceId`/`ComponentKey`/
`AttachEpoch`/`generation` pin が detach・再 mount で stale になり、
`ScenarioError::StaleGeneration`/`StaleEpoch` として明示 error に
なることを検証する。

### D3: 能力申告は adapter の実装をそのまま信頼し、test 側で境界を固定する

`FakeGuiTest`/`GpuiGuiTest` は `capabilities()` を自前で申告する。
test は「申告された能力集合が期待集合と一致すること」「未申告能力の
`require_capabilities` が `Unsupported` error になること」を assert
し、semantic pass が実機 pass に昇格しないことを `ScenarioReport`
（driver・capabilities・verdict・detail）で記録する。

### D4: GPUI 型の閉じ込め

`tests/scenario_gpui.rs` の `#![cfg(feature = "gpui-test")]` と
mount helper（`ViewHost::new` + `create_view_window`）内に
`gpui::TestAppContext`/`Entity`/`Window`/`Context` を閉じ込める。
`tests/common/scenario.rs` と `tests/scenario.rs` は contract 型
（`SemanticGuiTest`/`ScenarioTarget`/`ViewSpec` 等）のみを使い、
`ruviform-ui-gpui` への依存を持たない。

### D5: 観測は read-only、receipt/projection は既存経路

`observe`/`settle`/`drain_intents` は read-only 観測として使い、
CommandReceipt・state projection は既存の
`ViewHost::deliver_event`/`drain_events` 経路を通る。
`!Send` の GPUI state を thread 間で移動しない（全て同一
`TestAppContext` 内で完結）。

## Risks

- pin 更新で `ruviform-ui-test`/`ruviform-ui-gpui` の API が変わる
  可能性 → `check-ui-gpui`/`test-ui-gpui` lane で早期検出。
  現行 pin では `mise run check`・`check-ui-gpui` が pass 済み。
- `GpuiGuiTest` の settle/observe が非同期 window lifecycle と
  競合する可能性 → `gui.settle()` を mount 後・各 scenario 後に挟み、
  `gpui_parity.rs` の既存パターンに倣う。
