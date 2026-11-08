---
id: "TASK-LG5Rixm1dk6rFHJeCQjRqo"
type: "task"
status: "in-progress"
title: "al-gui webview host の driver-neutral API 移行のタスク"
created: "2026-11-08"
updated: "2026-11-08"
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
    - "docs/plans/webview-host-api-migration.md"
  implemented-by: []
  depends-on:
    - "docs/plans/webview-host-api-migration.md"
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
    - "docs/test-specs/webview-host-api-migration.md"
  references: []
  defers: []
  deferred-by: []
---

## タスク

| # | 内容 | 状態 |
| --- | --- | --- |
| 1 | 失敗ログ(run 42431214830)と pin 履歴の照合、root cause 特定 | 完了 |
| 2 | `al-gui.rs` を `run_multi` + `DesktopSession` へ移植 | 完了 |
| 3 | spec/design/plan/task/test-spec/IR 文書チェーン作成 | 完了 |
| 4 | mise 検証(fmt/check/lint/test/build + webview lane 3 本) | 完了 |
| 5 | origin/main merge → 再検証 → commit/push/PR | 進行中 |
| 6 | CI `webview-verify`(3 OS)の pass 確認 | 未着手 |

## 完了条件

- `check-ui-webview` / `lint-ui-webview` / `test-ui-webview` が
  pin `863a7a9d` で pass すること。
- CI `webview-verify` job が PR の CI で pass すること。
- root cause(AL 側の legacy API 未追従)が記録され、ruviform 側・
  CI 環境・flaky のいずれでもないことが示されること。
