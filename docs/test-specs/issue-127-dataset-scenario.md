---
id: "TSPEC-034Z9kQmDatasetScen04"
type: "test-spec"
status: "approved"
title: "Dataset tab→workspace scenario の検証仕様"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-127-dataset-scenario.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-219-gui-real-e2e.md"
---

# Dataset tab→workspace scenario の検証仕様

## Feature: driver-neutral scenario

- Rule: `dataset_workspace_scenario` は `SemanticGuiTest` port のみで
  動作し、Fake/GpuiSimulated 両 adapter で同一結果を返す。
- Rule: workspace Tabs への `SelectTab` index 1,2,0 が
  `al.workspace`/`tab-select`/`{"index":n}` intent として発行され、
  各 step 後の `selected` が `[Some(1),Some(2),Some(0)]`。
- Rule: seek/waveform/spectrogram/spectrogram-view/events/
  alignments の 6 代表 component が mounted。
- Rule: `resolve_target` が `WORKSPACE_KEY` component へ narrow
  した target を返し、capability pre-flight が
  `SemanticAction` を要求する。

## Feature: AL 独立 oracle

- Rule: workspace spec root が `Tabs`・3 tab・title
  Source/Spectrogram/Analysis・selection target `al.workspace`。
- Rule: Analysis tab に events/alignments table が存在。
- Rule: alignments が 1 行・status `accepted`・source/capture
  input id が fixture と一致・capture_start/end が embed offset
  9600 付近の範囲。
- Rule: events 各行で start_s が有限 f64、strength が非有限なら
  欠損セル（`None`）として正直に写像される。
- Rule: oracle の期待値は fixture 規約由来の AL 独自値であり、
  ruviform Test Harness の自己検証を転用しない。

## Feature: real lane catalog

- Rule: `CASES` が 9 件で pinned 順序（desktop.jobs→source.seek→
  dataset.workspace→input preflight→pointer→keyboard→visual×2→
  window close）を維持。
- Rule: `--blocked-probe` が全件 `NotRun`+`environment_blocked`・
  `environment_kind != real-window` の fail-closed 形状（exit 0）。
- Rule: `case_catalog_ids_are_stable` が pinned id 一覧と一致。

## Feature: window-close/DPI

- Rule: `al.real-os.window`/`close-veto-approve` が catalog 末尾。
- Rule: `window_geometry` の physical 寸法・scale factor を observed
  に記録。
- Rule: armed `Veto` で window 生存・armed `Approve` で `Destroyed`
  を `window_events` で確認。前提不成立・未観測は `NotRun`、
  観測された契約違反のみ `Fail`。
