---
id: "TASK-034Z9kQmDatasetScen03"
type: "task"
status: "done"
title: "Dataset tab→workspace scenario 実装タスク"
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
    - "docs/designs/issue-127-dataset-scenario.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-127-dataset-scenario.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
---

# Dataset tab→workspace scenario タスク

- [x] pin bump 判定を確定（Coordinator 承認: `cbcc8343`→`b6d0cc6a`、
      window-close/DPI 含有）
- [x] pin bump 実施: `update-ruviform-rev.mjs` → `sync-ui-deps` →
      `cargo fetch --locked` → check lanes
- [x] rev-watch 文書チェーン 5 点セット +
      `evidence/al127/b6d0cc6a-verification.md`
- [x] `tests/common/scenario.rs` に fixture・scenario・oracle を実装
- [x] `tests/scenario.rs` / `tests/scenario_gpui.rs` に adapter test 追加
- [x] `al-gui-real-scenario.rs` CASES + live path 拡張
      （dataset workspace case・window-close case）
- [x] `tests/scenario_real.rs` EXPECTED_CASES/pinned id 同期
- [x] slice 文書チェーン（spec/design/task/test-spec/ir/exp）
- [x] 検証 lane 全実行
- [x] review-protocol レビュー → commit/push/PR(`Refs #127`)→
      Issue #127 コメント
