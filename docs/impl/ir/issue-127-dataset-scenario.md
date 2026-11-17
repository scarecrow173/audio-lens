---
id: "IMPL-034Z9kQmDatasetScen05"
type: "impl"
status: "completed"
title: "Dataset tab→workspace scenario の実施記録"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "code"
        path: "docs/specs/issue-127-dataset-scenario.md"
      - type: "code"
        path: "docs/designs/issue-127-dataset-scenario.md"
      - type: "code"
        path: "docs/tasks/issue-127-dataset-scenario.md"
      - type: "code"
        path: "docs/test-specs/issue-127-dataset-scenario.md"
      - type: "code"
        path: "docs/impl/ir/issue-127-dataset-scenario.md"
      - type: "code"
        path: "docs/impl/exp/issue-127-dataset-scenario.md"
      - type: "code"
        path: "docs/impl/exp/issue-127-dataset-scenario-exp.jsonl"
      - type: "code"
        path: "crates/al-gui/tests/common/scenario.rs"
      - type: "code"
        path: "crates/al-gui/src/bin/al-gui-real-scenario.rs"
    modified:
      - type: "code"
        path: "crates/al-gui/tests/scenario.rs"
      - type: "code"
        path: "crates/al-gui/tests/scenario_gpui.rs"
      - type: "code"
        path: "crates/al-gui/tests/scenario_real.rs"
      - type: "code"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-127-dataset-scenario.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-127-dataset-scenario.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
---

# Dataset tab→workspace scenario の実施記録

## 実装内容

- `tests/common/scenario.rs`: `DatasetWorkspace` fixture
  （5-stage job → `DatasetSession::open` → `workspace_screen`）、
  `dataset_workspace_scenario`（tab 巡回 + mounted 観測）、
  `dataset_workspace_cells`（Analysis tab typed-cell oracle）、
  `DATASET_EMBED_OFFSET`/`WORKSPACE_TAB_SEQUENCE` 定数。
- `tests/scenario.rs`: `fake_dataset_workspace_scenario_switches_
  tabs_with_al_oracle` — intent 列・selected・mounted・oracle・
  report 構成を検証。
- `tests/scenario_gpui.rs`: `gpui_dataset_workspace_scenario_
  switches_tabs_with_al_oracle` — 同一契約を GpuiSimulated で検証。
- `al-gui-real-scenario.rs`: dataset spec を 3 番目の window に
  mount、`dataset_workspace_case`・`window_close_case` を追加、
  `CASES` 7→9、`lifecycle_events_json` helper。
- `tests/scenario_real.rs`: `EXPECTED_CASES=9`・pinned id 同期。

## 検証結果（実測）

- `cargo test -p al-gui --test scenario fake_dataset_workspace`:
  PASS（fixture 完走・oracle 合格）。
- `cargo test -p al-gui --features gpui-test --test scenario_gpui
  gpui_dataset_workspace`: PASS。
- `cargo test -p al-gui --features gpui-real --test scenario_real`:
  5 PASS（blocked-probe fail-closed・catalog pin 一致）。
- `cargo check -p al-gui --all-targets`（既定/gpui-test/gpui-real
  各 feature）: exit 0。

詳細な lane 結果は `evidence/al127/b6d0cc6a-verification.md` と
PR 本文の検証節を参照。
