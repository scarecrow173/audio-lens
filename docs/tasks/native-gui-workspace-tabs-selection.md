---
id: "TASK-034TTalkyKgrZqiG6JnpEc"
type: "task"
status: "done"
title: "Issue #166 Dataset workspace Tabs 選択契約の採用と pointer 切替 実装タスク (第10 slice)"
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
  implements:
    - "../plans/native-gui-workspace-tabs-selection.md"
  implemented-by: []
  depends-on:
    - "../plans/native-gui-workspace-tabs-selection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/native-gui-workspace-tabs-selection.md"
    - "../designs/native-gui-workspace-tabs-selection.md"
    - "../test-specs/native-gui-workspace-tabs-selection.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/native-gui-workspace-tabs-selection.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #166 Dataset workspace Tabs 選択契約の採用と pointer 切替 実装タスク (第10 slice)

## T1: 契約の値と workspace の selection を定義する

- `command.rs` に `WORKSPACE_TARGET`（`al.workspace`）を追加する。
- `view/workspace.rs` に `WORKSPACE_TAB_COUNT = 3` を定義し、`workspace_screen` の
  `tabs()` に `Some(WORKSPACE_TARGET)` を渡す。
- `view/desktop.rs` のモジュール説明に workspace Tabs の選択を追記する。

## T2: 受け口の失敗テストを先に書く

- `service_ops.rs`: `al.workspace` の `tab-select` の範囲内 `applied`、範囲外・params 欠落・
  別 target の `INVALID_PARAMS`、`al.desktop` との index 上限の独立性。
- `desktop.rs`: session 経由の workspace `tab-select` の receipt、`selected_job` と dirty が
  変わらないこと。
- `desktop_screen.rs`: workspace Tabs の `selection` の target、`select_tab` の intent の形、
  tab 数と `WORKSPACE_TAB_COUNT` の一致。
- 未実装の状態で失敗することを確認する。

## T3: service の workspace tab-select を実装する

- `service.rs` の `command()` の `(target, action)` match に
  `(WORKSPACE_TARGET, ACTION_TAB_SELECT)` を追加する。
- index の取り出しと範囲検証を `al.desktop` と共通の helper に集約し、
  上限値だけを target ごとに渡す。
- 範囲内は `{"status": "applied", "index": n}` を返す。範囲外・不正は `invalid` として
  `INVALID_PARAMS` に写す。

## T4: receipt と parity の検証を書く

- `receipt.rs`: workspace tab を `ViewHost::select_tab`（`WORKSPACE_KEY`）で選び、
  `applied` の receipt を検証する。rejected の巻き戻しと再射影での保持も検証する。
- `gpui_parity.rs`: 同じ `desktop_screen` を Fake と GPUI で評価し、workspace の
  `tab-select` の intent 列と観測された選択を比較する。

## T5: 検証と文書

- `mise run fmt`・`check`・`lint`・`test`・`check-ui-gpui`・`lint-ui-gpui`・`test-ui-gpui`・
  `check-cli-headless`・`check-ruviform-lock`・`check-ruviform-rev`・`check-ui`・`test-ui` を実行する。
- 文書（spec・design・plan・task・test-spec・impl）を完成させ、索引を
  `doc-maintenance` の managed index として再生成する。

## 非タスク（本 slice で扱わない）

- キーボードによるタブ切替（upstream FU-1）。
- 実 OS window での pointer 操作の確認（NotRun。可能なら補助的に実施）。
- ruviform の rev 変更（`a5b1db13` のまま）。
- Dataset が開けない場合の workspace Tabs（spec の非スコープどおり対象外）。
