---
id: "TSPEC-SCd44Z1STaGO2PilFgvrtP"
type: "test-spec"
status: "approved"
title: "Native GUI の Tabs 選択契約の採用と pointer 切替の検証仕様 (第9 slice)"
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
    - "../plans/native-gui-tabs-selection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/native-gui-tabs-selection.md"
    - "../tasks/native-gui-tabs-selection.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/native-gui-tabs-selection.md"
  derived-by: []
  verifies:
    - "../specs/native-gui-tabs-selection.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native GUI の Tabs 選択契約の採用と pointer 切替の検証仕様 (第9 slice)

## 検証対象と層

| 受入（spec） | 層 | テスト（ファイル・関数） |
| --- | --- | --- |
| R1・R2 selection の target と intent の形 | view（ViewHost） | `tests/desktop_screen.rs` `desktop_tabs_are_selectable_by_pointer_on_the_desktop_target` |
| R3 範囲内の applied | service（UiRuntime） | `tests/service_ops.rs` `command_tab_select_applies_an_in_range_index_and_rejects_the_rest` |
| R3・R5 applied の receipt と job 選択・dirty の不変 | session（FakeDriver） | `tests/desktop.rs` `tab_select_replies_applied_for_a_tab_and_rejected_outside_the_tabs` |
| R3 applied の receipt と表示の保持 | ViewHost（receipt 適用） | `tests/receipt.rs` `pointer_tab_selection_is_applied_by_the_service_receipt` |
| R4 rejected の巻き戻し | ViewHost（receipt 適用） | `tests/receipt.rs` `rejected_tab_selection_restores_the_previous_tab` |
| R4 範囲外・不正の拒否 | service・session | `tests/service_ops.rs`、`tests/desktop.rs`（上と同じ関数） |
| R6 再射影での保持 | ViewHost（receipt 適用と再射影） | `tests/receipt.rs` `resume_receipt_moves_the_run_button_from_pending_to_applied` |
| R7 parity | Fake と GPUI（GpuiObserver） | `tests/gpui_parity.rs` `tab_selection_intent_and_selected_tab_match_between_fake_and_gpui` |

## 観測の規則

- intent は `ViewHost::drain_events` の `ViewEvent::Intent` を `serde_json::to_value` した形で比べる（`target`・`action`・`params`）。
- 表示 tab は `ComponentSnapshot.selected` で観測する。receipt の到着は `CommandReceipt::from_result` の status で確かめる。
- revert の検証は、`rejected` の receipt が届いたことを先に確かめてから、表示が直前の tab に戻ったことを確かめる（偽陽性の防止）。
- GPUI の観測は `GpuiObserver` の seam（`select_tab`・`settle`・`observe`・`intents`）だけを使う。

## 未検証（NotRun / Unsupported）

- **実 OS window での pointer クリック**: 実行しない（NotRun）。GPUI の mouse 押下から `select_tab` までの配線は、
  ruviform の gpui-test（`tab_header_press_selects_the_tab_through_the_pointer`）が検証する。AudioLens の検証は `select_tab` 以降に限る。
- **workspace の Tabs（Source・Spectrogram・Analysis）の pointer 切替**: 採用していないため検証しない（Unsupported）。
- **キーボードによるタブ切替**: upstream FU-1 の範囲で、本 slice では検証しない（NotRun）。
- **Tab 内容の射影の網羅**: tab の子画面（Jobs 表・Run・Dataset）の内容は既存の `tests/desktop_screen.rs` と `tests/gpui_parity.rs` の検証に委ね、
  本 slice では tab 切替と selection の保持のみを新規に検証する。
