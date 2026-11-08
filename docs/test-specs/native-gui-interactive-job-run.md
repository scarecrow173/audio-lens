---
id: "TSPEC-Xaqw7FQfeezBD4YV3xV5Em"
type: "test-spec"
status: "approved"
title: "Native GUI の Jobs 対話選択と Run 再開・取消の検証仕様 (第7 slice)"
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
    - "../specs/native-gui-interactive-job-run.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/native-gui-interactive-job-run.md"
    - "../tasks/native-gui-interactive-job-run.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/native-gui-interactive-job-run.md"
  derived-by: []
  verifies:
    - "../specs/native-gui-interactive-job-run.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native GUI の Jobs 対話選択と Run 再開・取消の検証仕様 (第7 slice)

## 検証対象と層

| 受入 | 検証 | 場所 | 層 |
| --- | --- | --- | --- |
| A1 選択 intent | `jobs_table_selects_jobs_and_run_tab_offers_resume_and_cancel`（Table の selection target） | `tests/desktop_screen.rs` | Fake |
| A1・A4・A5 Button・intent の形 | `jobs_selection_and_resume_button_match_between_fake_and_gpui` | `tests/gpui_parity.rs` | Fake と GPUI |
| A2 選択の反映 | `select_intent_adopts_the_job_and_marks_the_projection_dirty` | `tests/desktop.rs` | session（FakeDriver） |
| A2 選択 receipt の応答 | `command_select_maps_row_to_job_and_replies_applied` | `tests/service_ops.rs` | service op |
| A3 拒否 | `rejected_selection_keeps_the_previous_job` | `tests/desktop.rs` | session |
| A4 再開 | `command_resume_starts_the_job_and_replies_accepted` | `tests/service_ops.rs` | service op |
| A5 取消（対象無し） | `cancel_without_a_window_job_replies_rejected` | `tests/desktop.rs` | session |
| A5 取消（受理）・A6 終端 | `cancel_intent_cancels_the_window_owned_job_and_replies_accepted` | `tests/desktop.rs` | session |
| 不正な command | `command_rejects_unknown_targets_and_actions` | `tests/service_ops.rs` | service op |
| A8 parity | `jobs_selection_and_resume_button_match_between_fake_and_gpui` | `tests/gpui_parity.rs` | Fake と GPUI |
| A9 境界 | `check-cli-headless`、`check-ui-gpui`・`lint-ui-gpui` | mise | 型・依存 |

## 観測の規則

- intent は Fake（`ViewHost`）と GPUI（`GpuiObserver::intents`）で `params.intent` の JSON を比べる。
- snapshot は `ComponentSnapshot.selected` で比べる。
- receipt は `Event::Result` の `id`・`ok`・`data.status` で判定する。

## 未検証（NotRun）

- 実 OS window での選択・ボタン操作（目視）
- `bin/al-desktop.rs` の `update_view` による再射影の実行（compile と lint のみ。自動テストは無い）
- 再射影後の Tabs の active 保持
- job 終端 receipt（ruviform#456 の解消後に検証する）
- 新規 run の起動（UI 接続なし）
