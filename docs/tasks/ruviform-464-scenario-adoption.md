---
id: "TASK-W04xdHiaIegpHOFhvnlQ9d"
type: "task"
status: "done"
title: "ruviform#464 Step C / AL#127 SemanticGuiTest scenario 採用のタスク"
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
  implements:
    - "docs/plans/ruviform-464-scenario-adoption.md"
  implemented-by: []
  depends-on:
    - "docs/plans/ruviform-464-scenario-adoption.md"
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
    - "docs/test-specs/ruviform-464-scenario-adoption.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform#464 Step C / AL#127 SemanticGuiTest scenario 採用のタスク

## Work

ruviform pin を `54439fdc` → `84d21641` へ追従し、al-gui の desktop
jobs 画面と source waveform/seek 画面に driver-neutral
`SemanticGuiTest` scenario を採用するタスク。`tests/common/scenario.rs`
に shared scenario を置き、Fake（`tests/scenario.rs`）と GPUI
（`tests/scenario_gpui.rs`、`gpui-test` feature）で同一関数を実行する。

| # | 内容 | 状態 |
| --- | --- | --- |
| 1 | ruviform pin を `84d21641a6bd50fa570ed074ae263284d05262ca` へ更新（`tools/update-ruviform-rev.mjs`、Cargo.lock 20 package） | 完了 |
| 2 | `tests/common/scenario.rs` に AL fixture（desktop jobs spec・source seek spec）と scenario 関数・report helper を実装 | 完了 |
| 3 | `tests/scenario.rs` に Fake adapter の semantic gesture pass・target resolution・stale pin・capability・Unsupported/NotRun 報告を実装 | 完了 |
| 4 | `tests/scenario_gpui.rs` に GPUI adapter の同一 scenario 実行・能力境界・capture/golden Unsupported を実装 | 完了 |
| 5 | `mise run fmt`/`check`/`lint`/`test`/`check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`/`check-ruviform-lock`/`check-ruviform-rev` で検証 | 完了 |
| 6 | `git fetch origin main && git merge origin/main` で最新 main を取り込み、lockfile・docs index を再生成して再検証 | 完了 |
| 7 | Conventional Commits で commit、`feature/rf464-scenario-adoption` を push、PR 作成 | 完了 |
