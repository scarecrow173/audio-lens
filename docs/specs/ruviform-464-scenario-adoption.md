---
id: "SPEC-VTjFD9We5lvQrqihJN9FVo"
type: "spec"
status: "approved"
title: "ruviform pin 84d21641 追従と al-gui SemanticGuiTest scenario 採用（ruviform#464 Step C / AL#127）"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/464"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "file"
        path: "crates/al-gui/tests/common/scenario.rs"
      - type: "file"
        path: "crates/al-gui/tests/scenario.rs"
      - type: "file"
        path: "crates/al-gui/tests/scenario_gpui.rs"
      - type: "file"
        path: "docs/specs/ruviform-464-scenario-adoption.md"
      - type: "file"
        path: "docs/designs/ruviform-464-scenario-adoption.md"
      - type: "file"
        path: "docs/plans/ruviform-464-scenario-adoption.md"
      - type: "file"
        path: "docs/tasks/ruviform-464-scenario-adoption.md"
      - type: "file"
        path: "docs/test-specs/ruviform-464-scenario-adoption.md"
    modified:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/al-gui/tests/common/mod.rs"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
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
    - "../designs/ruviform-464-scenario-adoption.md"
    - "../test-specs/ruviform-464-scenario-adoption.md"
    - "ruviform-463-spectrogram-intensity.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ruviform-464-scenario-adoption.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform pin 84d21641 追従と al-gui SemanticGuiTest scenario 採用（ruviform#464 Step C / AL#127）

## What Should Be Built

AudioLens の ruviform pin を `54439fdc` から `84d21641`（ruviform main
HEAD、RF#464「Consumer-neutral GUI Scenario Contract・Test Adapter・
Fake/GPUI 共通実行」（merge `57e4e8ce`）と RF#314「Golden Utility と
Scenario Contract 接続」（merge `2230018e`/`7ae916b3`）を含む）へ
`tools/update-ruviform-rev.mjs` 経由で追従する。

al-gui の実画面（desktop jobs 画面と source waveform/seek 画面）に対し、
Ruviform が提供する driver-neutral `SemanticGuiTest` contract
（`ruviform-ui-test` の `scenario` module）を採用したテストを追加する。
同一の scenario 関数を `FakeGuiTest`（`ViewHost` backed、同期）と
`GpuiGuiTest`（`GpuiObserver` backed、`gpui-test` feature）の両 adapter
で実行し、Fake 側は semantic gesture が Pass すること、GPUI 側は
`GpuiSimulated` driver として能力を正直に申告し、実行不能項目が
`ScenarioReport` の `Unsupported`/`NotRun` として記録されることを保証する。

## Why

RF#464 は GUI テストを driver 非依存の semantic contract に統一する
consumer 側 Step C を要求する。AudioLens は Ruviform の ViewSpec
projection（source screen / waveform / seek / meter / jobs / tabs /
resume）を既に持ち、`gpui_parity.rs` で Fake と GPUI の parity を
個別に検証していたが、scenario contract としての統一実行・能力申告・
stale target 検証は未採用だった。

fake の semantic pass を実 OS 入力・hit-test・pixel 証拠と偽ってはいけない。
本 spec は「採用」と「正直な能力境界の固定」を同時に要求する。

## Acceptance Criteria

- ruviform pin が `84d21641a6bd50fa570ed074ae263284d05262ca` に解決され、
  `mise run check-ruviform-lock` が pass する。
- al-gui の実画面で driver-neutral `SemanticGuiTest` scenario が動作する。
- target resolution が `WindowId` + `ViewInstanceId` + `ComponentKey` +
  `AttachEpoch`/`generation` pin で機能し、stale target が明示 error に
  なる。
- Fake adapter は semantic action と multi-window のみを申告し、
  GPUI adapter は `GpuiSimulated` として同能力を申告する。
  `SimulatedPointerOrKeyboard`・`RealOsPointerOrKeyboard`・`Ime`・
  `VisualCapture`・`NativePluginDock` は申告しない。
- `capture_frame`/`golden_check` は capture backend 不在のため
  `Unsupported`/`NotRun` を返し、pass に昇格しない。
- GPUI 型（`gpui::TestAppContext` 等）は GPUI adapter/構成 root に閉じ、
  shared scenario code は contract 型のみを使う。
- 実行不能項目（実 OS capture・IME・実 pointer 等）は
  `ScenarioReport` 上で `NotRun`/`Unsupported` として記録される。
