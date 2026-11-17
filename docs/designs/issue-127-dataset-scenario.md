---
id: "DESIGN-034Z9kQmDatasetScen02"
type: "design"
status: "approved"
title: "Dataset tab→workspace scenario の設計"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-127-dataset-scenario.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-127-dataset-scenario.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-127-dataset-scenario.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-219-gui-real-e2e.md"
---

# Dataset tab→workspace scenario の設計

## 構成

`tests/common/scenario.rs` に 3 要素を追加（driver-neutral、
`SemanticGuiTest` contract 型のみ）:

1. **`dataset_workspace()` fixture** — `tests/view.rs` の
   `real_dataset_produces_view_models` と同規約: source(4096 sample)
   を capture(24576 sample)の offset 9600 へ gain 1.0 で embed し、
   `Ingest → Locate → CheapAnalysis → ExpensiveAnalysis → Dataset`
   を完走させた job の dataset を `workspace_screen(&session,
   &capture_input_id)` で投影。`DatasetWorkspace{dir, spec, job_id,
   capture_input_id, source_input_id}` が fixture と oracle 期待値を
   保持する。
2. **`dataset_workspace_scenario()`** — workspace Tabs
   (`WORKSPACE_KEY`)へ narrow した target を resolve し、
   `[1,2,0]`（Spectrogram→Analysis→Source）の順で `SelectTab` を
   発行。各 step で `settle` → `drain_intents` → `observe().selected`
   を記録し、最後に 6 代表 key（seek/waveform/spectrogram/
   spectrogram-view/events/alignments）の mounted を観測する。
   `UnknownComponent` は mounted=false に写し観測不能を Pass にしない。
3. **`dataset_workspace_cells()` oracle** — `SemanticGuiTest` の
   snapshot は table 行を持たないため、mount される `ViewSpec`
   自体の Analysis tab typed cell を検査する。spec の cell は
   ViewHost がそのまま mount する投影であり、spec 側の不一致は
   mount 後表示にも同じく現れる。期待値は fixture 規約由来の
   AL 独自値: tab 構成（3 tab・Source/Spectrogram/Analysis・
   selection target `al.workspace`）、alignments 1 行・status
   `accepted`・capture 範囲が embed offset 近傍、events の
   start_s 有限 f64・strength 非有限→欠損セル、typed enum 語彙。

## adapter・lane 接続

- `tests/scenario.rs`: `FakeGuiTest` で mount→scenario→intent/
  selected/mounted/oracle を検証し `ScenarioReport` を構成。
- `tests/scenario_gpui.rs`: `GpuiGuiTest`（`mount_gpui`）で同一
  契約を実行。`ScenarioDriver::GpuiSimulated` のまま Pass を記録し
  実機証拠へ昇格しない。
- `al-gui-real-scenario.rs`: 3 番目の window に dataset spec を
  mount（desktop→source→dataset の mount 順）。`dataset_workspace_
  case` が共有 scenario + oracle を `run_scenario` 経由で実行。
- window-close/DPI: `al.real-os.window`/`close-veto-approve` を
  catalog 末尾に追加（window 破壊のため最後）。`window_geometry`
  の実測値を observed に記録し、armed Veto→生存・armed Approve→
  `Destroyed` を `window_events` で oracle 判定。観測面の前提不成立
  ・イベント未観測は `NotRun`。

## catalog 同期

`CASES` を 7→9 件に拡張（`CASE_DATASET_WORKSPACE` を source.seek の
次、`CASE_WINDOW_CLOSE` を末尾）。`scenario_real.rs` の
`EXPECTED_CASES=9` と pinned id 一覧を同期 — live path と
`blocked_reports` が同じ表を読む構造のため drift しない。

## 判定の正直さ

- semantic/Fake/GpuiSimulated の Pass は `ScenarioDriver` を
  そのまま記録し、real-window 証拠に昇格しない。
- `window_close_case` は `window_geometry`/`request_close` の
  `ScenarioError` を `NotRun` に写し、`CloseRequested` 未観測も
  `NotRun`。観測された契約違反（veto で死ぬ/approve で残る）のみ
  `Fail`。
