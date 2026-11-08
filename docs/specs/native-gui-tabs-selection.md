---
id: "SPEC-y3fZkq9ExqlJKunYgf6tQs"
type: "spec"
status: "approved"
title: "Native GUI の Tabs 選択契約の採用と pointer による画面 tab 切替の仕様 (Issue #166 第9 slice)"
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
    - "native-gui-job-receipt-consumption.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/native-gui-tabs-selection.md"
    - "../test-specs/native-gui-tabs-selection.md"
    - "https://github.com/scarecrow173/ruviform/pull/458"
  refines:
    - "native-gui-job-receipt-consumption.md"
  refined-by: []
  derives-from:
    - "native-gui-job-receipt-consumption.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/native-gui-tabs-selection.md"
  references: []
  defers: []
  deferred-by: []
---

# Native GUI の Tabs 選択契約の採用と pointer による画面 tab 切替の仕様 (Issue #166 第9 slice)

## 背景

第8 slice（`native-gui-job-receipt-consumption.md`）では、AudioLens の Tabs は `TabsSpec.selection` を持たないため、
ruviform の `ViewHost::select_tab` が "tabs are not selectable" で拒否し、Run tab へ pointer で移動できなかった。
Run tab の検証は、spec の `active` を 1 にして mount する Fake 層に限っていた。

ruviform（`a5b1db13`、RF#451）は `TabsSpec.selection: Option<TabsSelectionSpec>` を opt-in で持ち、
タブ header の押下は `tab-select` の `Action` intent（params `{"index": n}`）を発行する。receipt が
`rejected` / `unsupported` なら直前のタブへ戻し、`applied` なら表示を保つ。表示中の tab は
`ComponentSnapshot.selected` で観測できる。本 slice は、この契約を AudioLens が採用し、
画面の root Tabs を pointer で切り替えられるようにする。

## 目的

- 画面の root Tabs（Jobs・Run・Dataset）に `TabsSpec.selection` を採用し、pointer で Run・Dataset tab へ移動できるようにする。
- `tab-select` の intent を AudioLens の service 層で受け、receipt `applied` で応答する。範囲外の index は `rejected` とし、
  ruviform の契約どおり直前のタブへ戻す。
- Fake（headless）と GPUI の parity（intent・observed selection）を検証する。

## 受入条件

| # | 受入 | 判定基準 |
| --- | --- | --- |
| R1 | 採用 | 画面の root Tabs の `selection.target` が `command::DESKTOP_TARGET`（`al.desktop`）である |
| R2 | 発行 | tab header 押下相当（`ViewHost::select_tab`）は、action `tab-select`・params `{"index": n}`・target `al.desktop` の intent を 1 件発行する |
| R3 | applied | 範囲内（`n < TAB_COUNT`）の `tab-select` は window へ `status: applied` の receipt で返り、表示 tab は `n` のまま保たれる |
| R4 | rejected | 範囲外の index、params の欠落、`index` が u64 でない、target が `al.desktop` でない `tab-select` は `rejected`（`INVALID_PARAMS`）で返り、表示 tab は直前のタブへ戻る |
| R5 | 切替と job 選択の独立 | tab 切替は job 選択（`selected_job`）と射影の dirty を変えない |
| R6 | 再射影 | 表示中の tab を持つ同一 spec（`active` 0）の再射影で、表示中の tab と Run の Resume の phase は保たれる |
| R7 | parity | 同一の `desktop_screen` を Fake と GPUI で評価し、`tab-select` の intent 列が一致し、観測された選択が一致する |
| R8 | 検証 | `mise run fmt` / `check` / `lint` / `test` / `test-ui-gpui` / `lint-ui-gpui` / `check-cli-headless` / `check-ruviform-lock` / `check-ruviform-rev` が pass する |

## スコープ

- 画面の root Tabs（`desktop_screen`）の `selection` の採用と、その target・action 名の定義（`al-gui::command`）。
- service の `tab-select` 受け（target `al.desktop` の範囲内 index を `applied`、それ以外を `rejected`）。
- 上記の検証（`tests/service_ops.rs`・`tests/desktop.rs`・`tests/desktop_screen.rs`・`tests/receipt.rs`・`tests/gpui_parity.rs`）。
- 文書チェーン（spec・design・plan・task・test-spec・impl）と索引。
- ruviform の依存 rev は `a5b1db13` のまま変えない（RF#451 は同 rev に含まれる）。

## 非スコープ（未達として明示）

- **workspace の Tabs（Dataset 内の Source・Spectrogram・Analysis）の pointer 切替**: 同じ `tabs()` の呼び出しだが、
  本 slice は `selection: None` のままにする。Dataset tab 自体は pointer で到達できるが、その内側の tab は到達できない。
  採用には別の target（例 `al.workspace`）と index 上限の定義が要るため、後続 slice で扱う。
- **キーボードによるタブ切替**: upstream の FU-1 で別途対応中。本 slice は pointer のみ。
- **実 OS window での pointer 操作の確認**: 本 slice では実行しない（NotRun）。GPUI の mouse 押下 → `select_tab` の配線は
  ruviform 側の gpui-test（`tab_header_press_selects_the_tab_through_the_pointer`）が検証しており、AudioLens の検証は
  `select_tab` 以降（intent・receipt・観測）に限る。
- **新規 Run の起動（`al.job.run`）の UI 接続**、**Cancel の終端**: 第7・第8 slice の判断を維持する。

## 不変条件

- tab の選択状態の正本は ruviform の `ViewHost`（displayed selection）であり、AudioLens は表示 tab を session に保持しない。
  AudioLens が持つ選択状態は job の選択（`selected_job`）だけである。
- receipt の status 語彙は runtime の `accepted`/`applied`/`rejected` に従い、新しい語彙を作らない。
- `tab-select` の成否は service の応答のみで決まる。ruviform の `select_tab` の事前検証（index 範囲）を AudioLens 側で再実装しない。
  ただし untrusted な params の検証は service の境界で行う。
- ruviform の契約（rejected/unsupported で直前のタブへ戻す、同一 spec の再射影で表示を保つ）を consumer 側で再実装・恒久 workaround しない。
- 既存の job 選択（Jobs 表）・Run・Dataset の射影と receipt の挙動（第7・第8 slice）は変えない。
