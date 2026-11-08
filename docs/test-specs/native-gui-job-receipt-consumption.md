---
id: "TSPEC-sHEYPIKg4p6mrGf3MiYcjR"
type: "test-spec"
status: "approved"
title: "Native GUI の job 終端 receipt の消費と再射影の検証仕様 (第8 slice)"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../specs/native-gui-job-receipt-consumption.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/native-gui-job-receipt-consumption.md"
    - "../plans/native-gui-job-receipt-consumption.md"
    - "../tasks/native-gui-job-receipt-consumption.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/native-gui-job-receipt-consumption.md"
  derived-by: []
  verifies:
    - "../specs/native-gui-job-receipt-consumption.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native GUI の job 終端 receipt の消費と再射影の検証仕様 (第8 slice)

## 検証対象と層

| 受入 | 検証 | 場所 | 層 |
| --- | --- | --- | --- |
| R1 rev 整合 | `check-ruviform-lock`・`check-ruviform-rev`・`sync-ui-deps` | mise | 依存 |
| R2 ack | `resume_is_acked_then_receives_an_applied_receipt_after_the_job_finishes`（ack は `status` を持たない） | `tests/desktop.rs` | session（FakeDriver） |
| R3 終端（完了） | 同上（2 回目の `result` は `status: applied`、`job` event の後に配送、`take_projection_dirty`） | `tests/desktop.rs` | session（FakeDriver） |
| R4 終端（失敗） | `resume_of_a_missing_job_ends_with_a_rejected_receipt`（2 回目の `result` は `ok: false`） | `tests/desktop.rs` | session（FakeDriver） |
| R5 phase 遷移 | `resume_receipt_moves_the_run_button_from_pending_to_applied`（`Pending` → `Accepted` → `Applied`） | `tests/receipt.rs` | Fake（ViewHost） |
| R6 再射影の保持 | 同上（同一 spec の再射影後も `selected` = Run と phase `Applied` が保たれる） | `tests/receipt.rs` | Fake（ViewHost） |
| R6 既知の挙動 | `spec_driven_reprojection_returns_the_displayed_tab_to_jobs` | `tests/receipt.rs` | Fake（ViewHost） |
| R7 parity | `test-ui-gpui`（`gpui_parity.rs` の既存 parity を含む） | mise | Fake と GPUI |

## 観測の規則

- receipt は `ruviform_ui::ipc::Event::Result` の `id`・`ok`・`data.status` で判定し、ViewHost への適用は
  `CommandReceipt::from_result` の decode を経由する（GPUI driver の `drain_inbox` と同じ経路）。
- 順序は `FakeDriver::events_for` の配送順で判定する。
- phase は `ViewHost::observe(key).phase`、Tabs の表示は `ComponentSnapshot.selected` で判定する。
- job の終端は `DesktopSession::runtime().has_running_jobs()` が落ちるまで Tick を流して待つ。

## 未検証（NotRun / Unsupported）

- 実 OS window での Run tab の操作と目視（NotRun）。Run tab は pointer で選べないため（spec の非スコープ）、
  実機確認の手順自体が未定義。
- Tabs の pointer 切替と、その状態の保持（Unsupported: `TabsSpec.selection` を採用していない）。
- 取消（`Cancelled`）された resume job の終端 receipt（`rejected`）の AudioLens 側の直接検証は未実施。
  ruviform の `ui_dvr_03_parity_job_command_receipt_phases` は panic 経路の `rejected` を検証しており、
  Cancelled の終端 receipt は ruviform の `runtime.rs` の分類（`Cancelled` → `rejected`）に依る。
- `check-ui-driver-boundary`・`test-ui-parity` 等の task 名（mise.toml に存在しない。Unsupported）。
- `fmt-check` task（mise.toml に存在しない。`fmt` が check を行う）。
