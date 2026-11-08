---
id: "TASK-bcFP7IwsUr4usGeNSqbpVn"
type: "task"
status: "done"
title: "Issue #166 Tabs 選択契約の採用と pointer 切替 実装タスク (第9 slice)"
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
    - "../plans/native-gui-tabs-selection.md"
  implemented-by: []
  depends-on:
    - "../plans/native-gui-tabs-selection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/native-gui-tabs-selection.md"
    - "../designs/native-gui-tabs-selection.md"
    - "../test-specs/native-gui-tabs-selection.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/native-gui-tabs-selection.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #166 Tabs 選択契約の採用と pointer 切替 実装タスク (第9 slice)

## T1: 契約の値と view の selection を定義する

- `command.rs` に `DESKTOP_TARGET`（`al.desktop`）と `ACTION_TAB_SELECT`（ruviform の `TAB_SELECT_ACTION`）を追加する。
- `view/mod.rs` の `tabs()` に `selection: Option<&str>` を追加する。`workspace.rs` は `None` を渡す。
- `view/desktop.rs` に `TAB_COUNT = 3` を定義し、root Tabs に `Some(DESKTOP_TARGET)` を渡す。

## T2: 受け口の失敗テストを先に書く

- `service_ops.rs`: `tab-select` の範囲内 `applied`、範囲外・params 欠落・別 target の `INVALID_PARAMS`。
- `desktop.rs`: session 経由の `tab-select` の receipt、`selected_job` と dirty が変わらないこと。
- `desktop_screen.rs`: root Tabs の `selection` の target、`select_tab` の intent の形、tab 数と `TAB_COUNT` の一致。
- 未実装の状態で失敗することを確認する（`TAB_COUNT` 未定義によるコンパイル失敗を含む）。

## T3: service の tab-select を実装する

- `service.rs` の `command()` を `(target, action)` の match にし、`(DESKTOP_TARGET, ACTION_TAB_SELECT)` で範囲を検証する。
- 範囲内は `{"status": "applied", "index": n}` を返す。範囲外・不正は `invalid` として `INVALID_PARAMS` に写す。
- capability の summary に tab 選択を加える。

## T4: receipt と parity の検証を書く

- `receipt.rs`: Run tab を `ViewHost::select_tab` で選び、`applied` の receipt を検証する。範囲外は ViewHost の事前検証に当たるため、
  receipt の revert は al-gui service を登録しない runtime で `rejected` を発生させて検証する。
- 再射影（同一 spec・`active` 0）で表示中の tab と Resume の phase が保たれることを検証する。
- `gpui_parity.rs`: 同じ `desktop_screen` を Fake と GPUI で評価し、`tab-select` の intent 列と観測された選択を比較する。

## T5: 検証と文書

- `mise run fmt`・`check`・`lint`・`test`・`test-ui-gpui`・`lint-ui-gpui`・`check-cli-headless`・`check-ruviform-lock`・`check-ruviform-rev` を実行する。
- 文書（spec・design・plan・task・test-spec・impl）を作成し、索引を `doc-maintenance` の managed index として再生成する。

## 非タスク（本 slice で扱わない）

- workspace の Tabs（Source・Spectrogram・Analysis）の pointer 切替。
- キーボードによるタブ切替（upstream FU-1）。
- 実 OS window での pointer 操作の確認（NotRun）。
- ruviform の rev 変更（`a5b1db13` のまま）。
