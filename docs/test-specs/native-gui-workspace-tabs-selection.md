---
id: "TSPEC-034TTalkyWEhHcBPxXaOHj"
type: "test-spec"
status: "approved"
title: "Native GUI の Dataset workspace Tabs 選択契約の採用と pointer 切替の検証仕様 (第10 slice)"
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
    - "../plans/native-gui-workspace-tabs-selection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/native-gui-workspace-tabs-selection.md"
    - "../tasks/native-gui-workspace-tabs-selection.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/native-gui-workspace-tabs-selection.md"
  derived-by: []
  verifies:
    - "../specs/native-gui-workspace-tabs-selection.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native GUI の Dataset workspace Tabs 選択契約の採用と pointer 切替の検証仕様 (第10 slice)

## 検証対象と層

| 受入（spec） | 層 | テスト（ファイル・関数） |
| --- | --- | --- |
| R1・R2 workspace selection の target と intent の形 | view（ViewHost） | `tests/desktop_screen.rs` `workspace_tabs_are_selectable_by_pointer_on_the_workspace_target` |
| R3 workspace 範囲内の applied | service（UiRuntime） | `tests/service_ops.rs` `command_workspace_tab_select_applies_an_in_range_index_and_rejects_the_rest` |
| R3・R5 applied の receipt と job 選択・dirty の不変 | session（FakeDriver） | `tests/desktop.rs` `workspace_tab_select_replies_applied_and_rejected_without_touching_the_projection` |
| R3 applied の receipt と表示の保持 | ViewHost（receipt 適用） | `tests/receipt.rs` `workspace_tab_selection_is_applied_and_survives_reprojection` |
| R4 rejected の巻き戻し | ViewHost（receipt 適用） | `tests/receipt.rs` `rejected_tab_selection_restores_the_previous_tab`（workspace も対象に拡張） |
| R4 範囲外・不正の拒否 | service・session | `tests/service_ops.rs`、`tests/desktop.rs`（上と同じ関数） |
| R5 target 独立性 | service | `tests/service_ops.rs`（`al.desktop` と `al.workspace` を別々に検証） |
| R6 再射影での保持 | ViewHost（receipt 適用と再射影） | `tests/receipt.rs` `workspace_tab_selection_is_applied_and_survives_reprojection` |
| R7 parity | Fake と GPUI（GpuiObserver） | `tests/gpui_parity.rs` `workspace_tab_selection_intent_and_selected_tab_match_between_fake_and_gpui` |

## 観測の規則

- intent は `ViewHost::drain_events` の `ViewEvent::Intent` を `serde_json::to_value` した形で比べる（`target`・`action`・`params`）。
- 表示 tab は `ComponentSnapshot.selected` で観測する。receipt の到着は `CommandReceipt::from_result` の status で確かめる。
- workspace の Tabs は Dataset tab の child として spec に含まれるため、`select_tab` の key は `WORKSPACE_KEY`（`workspace`）。
  root Tabs とは別 key・別 target で観測する。
- revert の検証は、`rejected` の receipt が届いたことを先に確かめてから、表示が直前の tab に戻ったことを確かめる（偽陽性の防止）。
- GPUI の観測は `GpuiObserver` の seam（`select_tab`・`settle`・`observe`・`intents`）だけを使う。

## 未検証（NotRun / Unsupported）

- **実 OS window での pointer クリック**: 実行しない（NotRun）。GPUI の mouse 押下から
  `select_tab` までの配線は、ruviform の gpui-test
  （`tab_header_press_selects_the_tab_through_the_pointer`）が検証する。
  AudioLens の検証は `select_tab` 以降に限る。実施が可能な場合のみ補助的に確認する。
- **キーボードによるタブ切替**: upstream FU-1 の範囲で、本 slice では検証しない（NotRun）。
- **Dataset が開けない場合の workspace Tabs**: workspace の key が spec に存在しないため
  選択対象が無い（Unsupported — spec の非スコープ）。
- **Tab 内容の射影の網羅**: tab の子画面（Source 波形・Spectrogram・Analysis）の内容は
  既存の `tests/desktop_screen.rs`・`tests/view.rs`・`tests/gpui_parity.rs` の検証に委ね、
  本 slice では tab 切替と selection の保持のみを新規に検証する。
