---
id: "IMPL-3nlQqq9VVjLe6rgtwK4YjN"
type: "impl"
status: "completed"
title: "Native GUI の Tabs 選択契約の採用と pointer 切替の実装記録(IR, 第9 slice)"
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
    - "../../tasks/native-gui-tabs-selection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  refines: []
  refined-by: []
  derives-from:
    - "../../plans/native-gui-tabs-selection.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/pull/458"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Native GUI の Tabs 選択契約の採用と pointer 切替の実装記録(IR, 第9 slice)

## 実装内容

- `al-gui::command`: `DESKTOP_TARGET`（`al.desktop`）と `ACTION_TAB_SELECT`（ruviform の `TAB_SELECT_ACTION` を再利用）を追加。
- `al-gui::view::tabs()`: `selection: Option<&str>` を受け、`TabsSpec.selection` を組み立てる（`table()` と同じ形）。
  `workspace_screen` は `None`（非採用のまま）。
- `al-gui::view::desktop`: `TAB_COUNT = 3` を定義し、root Tabs を `Some(DESKTOP_TARGET)` で組む。
- `al-gui::service::command()`: `(target, action)` の match にし、`(DESKTOP_TARGET, tab-select)` を範囲検証のうえ
  `{"status": "applied", "index": n}` で応答。範囲外・params 不正は `invalid`（`INVALID_PARAMS`）。
- `al-gui::desktop`（session）と `bin/al-desktop.rs`: 変更なし。`tab-select` は runtime → service → receipt の経路で処理され、
  session の job 選択・dirty を動かさない。再射影は既存の `update_view`（dirty 時のみ）をそのまま使う。
- ruviform の依存 rev は `a5b1db13` のまま（`Cargo.lock`・`ui/vendor` の変更なし）。

## テスト（先に書いた）

テストは実装より先に書いた。実装前の時点では `TAB_COUNT` が未定義のため `service_ops`・`desktop_screen` のテストが
コンパイルに失敗した（確認したのはこのコンパイル失敗のみで、振る舞いの red は観測していない）。`tests/receipt.rs` の harness は Run tab を `active: 1` の mount ではなく
`ViewHost::select_tab` で選ぶよう置き換えた（第8 slice の workaround の除去）。

## 受入対応表

| 受入（spec） | 結果 | 根拠 |
| --- | --- | --- |
| R1 root Tabs の selection target | Passed | `desktop_tabs_are_selectable_by_pointer_on_the_desktop_target` |
| R2 select_tab の intent（action・params・target） | Passed | 同上、`tab_selection_intent_and_selected_tab_match_between_fake_and_gpui` |
| R3 範囲内 applied・表示保持 | Passed | `command_tab_select_applies_an_in_range_index_and_rejects_the_rest`、`tab_select_replies_applied_for_a_tab_and_rejected_outside_the_tabs`、`pointer_tab_selection_is_applied_by_the_service_receipt` |
| R4 範囲外・不正の rejected と巻き戻し | Passed | 上記 service・session テスト、`rejected_tab_selection_restores_the_previous_tab`（`rejected` の到着を先に確認） |
| R5 job 選択・dirty の不変 | Passed | `tab_select_replies_applied_for_a_tab_and_rejected_outside_the_tabs`（`selected_job` None・dirty false）、`pointer_tab_selection_is_applied_by_the_service_receipt` |
| R6 同一 spec 再射影での保持 | Passed | `resume_receipt_moves_the_run_button_from_pending_to_applied`（Run 選択 → 終端再射影後も `selected` = 1、Resume phase = Applied） |
| R7 Fake/GPUI parity | Passed | `tab_selection_intent_and_selected_tab_match_between_fake_and_gpui`（`test-ui-gpui` の gpui_parity 11 件中） |
| R8 検証 | Passed | 下記「検証結果」 |

## 判断と未達

- **target を `al.desktop` とした**: 既存の `al.job` は Jobs・Run の操作であり、tab 選択と意味が異なるため分けた。
- **action 名は ruviform の定数を再利用**: 同じ文字列を AudioLens が別定義すると表現が二重化するため。
- **範囲検証を service に置いた**: IPC の intent は service の境界に届く。ruviform の事前検証（`select_tab` の範囲外拒否）と
  重複するが、判定の再実装ではなく境界の防御として残した（design §3）。
- **workspace の Tabs は非採用**: Dataset 内の Source・Spectrogram・Analysis は pointer で到達できない。
  採用には別 target と index 上限が要るため、受入の範囲（root Tabs）を超えると判断して後続 slice に送った。
- **session に tab 状態を持たせなかった**: 表示 tab の正本は ruviform の `ViewHost`。consumer 側に第二の状態を持たない。
- **ruviform の issue は起票していない**: 本 slice の検証で ruviform 側の欠陥は見つからなかった。
  キーボード（FU-1）は upstream の既存作業のため重複起票しない。

## 検証結果

コード HEAD は `26f42d7`（main）の上の作業ツリー。すべて `mise run` 経由（canonical）で実行した。

| コマンド（mise task） | 環境 | exit | 結果 |
| --- | --- | --- | --- |
| `mise run fmt` | Windows x64, rustc 1.99.0, cargo 1.99.0, mise 2026.10.1 | 0 | Passed |
| `mise run check` | 同上 | 0 | Passed |
| `mise run lint`（biome check + clippy `-D warnings`） | 同上 | 0 | Passed |
| `mise run test`（cargo test --workspace） | 同上 | 0 | Passed（646 件、failed 0） |
| `mise run test-ui-gpui` | 同上 | 0 | Passed（al-gui 102 件、failed 0。`gpui_parity` 11 件を含む） |
| `mise run lint-ui-gpui` | 同上 | 0 | Passed |
| `mise run check-cli-headless` | 同上 | 0 | Passed（10 headless crate に GUI framework 依存なし） |
| `mise run check-ruviform-lock` | 同上 | 0 | Passed（20 packages が `a5b1db13`） |
| `mise run check-ruviform-rev` | 同上 | 0 | Passed（lockfile primary、vendor 木なし） |

注: `check` の出力に依存クレート（proc-macro-error2）の future-incompat 警告があるが、本 slice の変更とは無関係で、既存の警告である。

### NotRun / Unsupported

- **実 OS window での pointer タブ切替**: NotRun。GPUI の mouse 押下から `select_tab` までの配線は ruviform の gpui-test
  （`tab_header_press_selects_the_tab_through_the_pointer`）が検証する。AudioLens 側の検証は `select_tab` 以降に限る。
- **workspace の Tabs の pointer 切替**: Unsupported（非採用）。
- **キーボードによるタブ切替**: NotRun（upstream FU-1）。
- **`check-ui` / `test-ui` / `check-ui-driver-boundary`**: 本 slice は `ui/` を変更せず、driver 境界の task は mise.toml に存在しない。
  `check-ui-driver-boundary` は NotRun として PR に明記する。

## 独立レビュー（review-protocol）の結果

- 実施: 1 回。subagent の起動はランタイム方針で行わず、Reviewer・Validator・Judge を分離した 3 pass として同一エージェントが実施した。
  このため独立性は subagent 実行より低い点に注意する。
- Reviewer 観点: 契約の一致（target・action・params）、service の境界検証、session の不変、再射影の保持、テストの偽陽性。
- Issue Candidate:
  1. （P3）`tests/desktop.rs` の `command_intent_on` は tab 用 intent にも `control_id: "job-table"` を入れる。service は読まず、テストの意味も変えないため非ブロッキング。
  2. （P2・既知の残課題）workspace の Tabs が pointer で到達できない。spec の非スコープとして明示済みで、受入の欠落ではない。
  3. （P3）tab の子画面の内容が切替後に正しく射影されることは、切替と selection の保持のみを新規に検証した。内容は既存の `desktop_screen`・parity テストに委ねた。
- Validator: 上記 1〜3 はいずれも `CONFIRMED` だが、P0/P1 は無し。
- Judge: Blocking なし。判定は **ACCEPT WITH NON-BLOCKING NOTES**。P2/P3 は報告のみで修正ループには入れていない。

## 逸脱・残課題

- workspace の Tabs の pointer 切替（上記 2）は次 slice の判断事項。
- 実 OS window での確認（NotRun）は Coordinator 側の手元確認に委ねる。
