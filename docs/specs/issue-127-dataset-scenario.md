---
id: "SPEC-034Z9kQmDatasetScen01"
type: "spec"
status: "approved"
title: "Dataset tab→workspace driver-neutral scenario + AL 独立 oracle（AL#127）"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/631"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-127-dataset-scenario.md"
      - type: "file"
        path: "docs/designs/issue-127-dataset-scenario.md"
      - type: "file"
        path: "docs/tasks/issue-127-dataset-scenario.md"
      - type: "file"
        path: "docs/test-specs/issue-127-dataset-scenario.md"
      - type: "file"
        path: "docs/impl/ir/issue-127-dataset-scenario.md"
      - type: "file"
        path: "docs/impl/exp/issue-127-dataset-scenario.md"
      - type: "file"
        path: "docs/impl/exp/issue-127-dataset-scenario-exp.jsonl"
    modified:
      - type: "file"
        path: "crates/al-gui/tests/common/scenario.rs"
      - type: "file"
        path: "crates/al-gui/tests/scenario.rs"
      - type: "file"
        path: "crates/al-gui/tests/scenario_gpui.rs"
      - type: "file"
        path: "crates/al-gui/tests/scenario_real.rs"
      - type: "file"
        path: "crates/al-gui/src/bin/al-gui-real-scenario.rs"
      - type: "file"
        path: "Cargo.lock"
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
    - "docs/specs/issue-219-gui-real-e2e.md"
    - "docs/specs/issue-127-rf628-consumer-oracle.md"
---

# Dataset tab→workspace driver-neutral scenario + AL 独立 oracle

## 目的

`scarecrow173/ruviform#631` 投影の残件として、Dataset tab→workspace
（Source/Spectrogram/Analysis）の driver-neutral `SemanticGuiTest`
scenario を整備し、`al-gui-real-scenario` lane catalog へ接続する。
併せて AL 独立 oracle（typed cell / receipt 系の製品 semantics を
AL 側の期待値で判定）を持たせ、共通 Test Harness の自己検証を
AL の Pass に転用しない。

## 要件

- R1: `tests/common/scenario.rs` に `al.dataset.workspace`/
  `workspace-tabs` scenario を追加。`SemanticGuiTest` port のみで
  記述し、GPUI 型を common module に漏らさない。
- R2: Dataset tab→workspace（`workspace_screen`、`DatasetSession`
  fixture — `make_job` + `JobService::dataset_dir` +
  `DatasetSession::open` の `tests/view.rs` 先例）を mount し、
  Source/Spectrogram/Analysis の 3 tab を巡回する
  `al.workspace`/`tab-select` intent（index 0,1,2）と各 tab の
  mounted component・`selected` を観測する。
- R3: AL 独立 oracle — Analysis tab の typed cell（events/
  alignments table の行・typed enum 語彙・f64/Validity 欠損の
  正直な写像）を fixture の既知期待値（embed offset・受理済み
  alignment）で判定する。spec 投影は ViewHost がそのまま mount
  するため spec 側検査が mount 後表示の oracle になる。
- R4: Fake adapter（`tests/scenario.rs`）と GpuiSimulated adapter
  （`tests/scenario_gpui.rs`）で同一契約を実行。
- R5: `al-gui-real-scenario` の `CASES` catalog に新 case を追加し、
  `tests/scenario_real.rs` の `EXPECTED_CASES`/pinned id を同期。
  前提不成立は `NotRun`/`Unsupported`/`environment_blocked` に写す。
- R6: pin bump `cbcc8343` → `b6d0cc6a` を含め、window-close/DPI
  case（`al.real-os.window`/`close-veto-approve`、
  `ruviform#709` 観測面）を catalog 末尾に追加して
  `audio-lens#219` Deferred 行を解放する。

## 受入条件（slice）

- AC1: 上記 R1–R6 が実装され、Fake/GpuiSimulated adapter test が
  Pass。
- AC2: blocked-probe が catalog 9 case 全件 `NotRun`+
  `environment_blocked` の fail-closed 形状を維持。
- AC3: pin bump の rev-watch 文書チェーンと
  `evidence/al127/b6d0cc6a-verification.md` が揃う。
- AC4: 実機 GUI-L2・実 pixel・macOS/Linux・cross-consumer は
  `NotRun`/`Deferred`/対象外と正直に記録（headless 完結）。

## 範囲外

- `test-ui-gpui-real` interactive 実走（常設 gate）。
- `ruviform#662` ScenarioMatrix catalog API への移行（任意 refactor）。
- 製品 view/session 本体の変更。
- Issue #127 の Close（`Refs #127` のみ）。
