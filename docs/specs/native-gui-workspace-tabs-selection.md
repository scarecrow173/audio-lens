---
id: "SPEC-034TTalkmZRrkmu9SMhAHa"
type: "spec"
status: "approved"
title: "Native GUI の Dataset workspace Tabs 選択契約の採用と pointer による tab 切替の仕様 (Issue #166 第10 slice)"
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
    - "native-gui-tabs-selection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/native-gui-workspace-tabs-selection.md"
    - "../test-specs/native-gui-workspace-tabs-selection.md"
  refines:
    - "native-gui-tabs-selection.md"
  refined-by: []
  derives-from:
    - "native-gui-tabs-selection.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/native-gui-workspace-tabs-selection.md"
  references: []
  defers: []
  deferred-by: []
---

# Native GUI の Dataset workspace Tabs 選択契約の採用と pointer による tab 切替の仕様 (Issue #166 第10 slice)

## 背景

第9 slice（`native-gui-tabs-selection.md`）で、画面の root Tabs（Jobs・Run・Dataset）に
ruviform の `TabsSpec.selection` を採用し、`tab-select` intent を `al.desktop` target で受理した。
同 slice は Dataset tab の内側にある workspace の Tabs（Source・Spectrogram・Analysis）を
`selection: None` のまま非スコープとし、「採用には別の target と index 上限の定義が要る」
として後続 slice に送った。

その結果、Dataset tab へは pointer で到達できるが、その内側の Source・Spectrogram・Analysis
の tab header は押せず、Spectrogram・Analysis の画面は実 GUI では到達不能のままである。
ruviform（`a5b1db13`、RF#451）の契約は `TabsSpec.selection` を持つ任意の Tabs strip に
適用され、表示 tab の正本は `ViewHost`、receipt が `rejected` / `unsupported` なら
直前のタブへ戻る。本 slice は同じ契約を workspace の Tabs に採用する。

## 目的

- Dataset 内の workspace Tabs（Source・Spectrogram・Analysis）に `TabsSpec.selection` を採用し、
  pointer で 3 画面を切り替えられるようにする。
- `tab-select` の intent を `al.workspace` target で AudioLens の service 層が受け、
  receipt `applied` で応答する。範囲外の index は `rejected` とし、
  ruviform の契約どおり直前のタブへ戻す。
- root Tabs（`al.desktop`）と workspace Tabs（`al.workspace`）の `tab-select` を
  `(target, action)` の組で区別し、index 上限をそれぞれ独立に検証する。
- Fake（headless）と GPUI の parity（intent・observed selection）を検証する。

## 受入条件

| # | 受入 | 判定基準 |
| --- | --- | --- |
| R1 | 採用 | Dataset tab 内の workspace Tabs の `selection.target` が `command::WORKSPACE_TARGET`（`al.workspace`）である |
| R2 | 発行 | workspace の tab header 押下相当（`ViewHost::select_tab` に `WORKSPACE_KEY`）は、action `tab-select`・params `{"index": n}`・target `al.workspace` の intent を 1 件発行する |
| R3 | applied | 範囲内（`n < WORKSPACE_TAB_COUNT`）の workspace `tab-select` は window へ `status: applied` の receipt で返り、表示 tab は `n` のまま保たれる |
| R4 | rejected | workspace の `tab-select` で範囲外の index・params の欠落・`index` が u64 でないものは `rejected`（`INVALID_PARAMS`）で返り、表示 tab は直前のタブへ戻る。未知の target の `tab-select` も `rejected` であり、`al.desktop` の `tab-select` は別 index 空間として独立に受理される |
| R5 | 独立性 | workspace の tab 切替は job 選択（`selected_job`）・射影の dirty・root Tabs の表示選択を変えない。`al.desktop` と `al.workspace` の `tab-select` は互いの index 上限で独立に判定される |
| R6 | 再射影 | Dataset が開いている状態での再射影（同一 spec・`active` 0）で、workspace の表示中の tab は保たれる |
| R7 | parity | 同一の `desktop_screen` を Fake と GPUI で評価し、workspace の `tab-select` の intent 列が一致し、観測された選択が一致する |
| R8 | 検証 | `mise run fmt` / `check` / `lint` / `test` / `test-ui-gpui` / `lint-ui-gpui` / `check-ui-gpui` / `check-cli-headless` / `check-ruviform-lock` / `check-ruviform-rev` が pass する |

## スコープ

- workspace の Tabs（`view::workspace::workspace_screen`）の `selection` の採用と、
  その target・index 上限の定義（`al-gui::command`・`al-gui::view::workspace`）。
- service の `tab-select` 受けへの `(al.workspace, tab-select)` の追加
  （範囲内 `applied`、それ以外 `rejected`）。
- 上記の検証（`tests/service_ops.rs`・`tests/desktop.rs`・`tests/desktop_screen.rs`・
  `tests/receipt.rs`・`tests/gpui_parity.rs`）。
- 文書チェーン（spec・design・plan・task・test-spec・impl）と索引。
- ruviform の依存 rev は `a5b1db13` のまま変えない（`TabsSpec.selection` は同 rev に含まれる）。

## 非スコープ（未達として明示）

- **キーボードによるタブ切替**: upstream の FU-1 で別途対応中。本 slice は pointer のみ。
- **実 OS window での pointer 操作の確認**: 本 slice では実行しない（NotRun）。
  GPUI の mouse 押下 → `select_tab` の配線は ruviform 側の gpui-test
  （`tab_header_press_selects_the_tab_through_the_pointer`）が検証しており、
  AudioLens の検証は `select_tab` 以降（intent・receipt・観測）に限る。
  実施が可能な場合のみ補助的に確認する。
- **Dataset が開けない場合の workspace Tabs**: job 未選択・dataset 不在などで
  `dataset_panel` が `unavailable` の Panel に置き換わる場合、workspace の Tabs 自体が
  spec に存在しないため選択対象も存在しない（既存の射影規則どおり）。
- **新規 Run の起動（`al.job.run`）の UI 接続**、**Cancel の終端**: 第7・第8 slice の判断を維持する。
- **ruviform 側の契約変更**: `TabsSpec.selection`・receipt 契約は rev `a5b1db13` のものをそのまま使う。

## 不変条件

- tab の選択状態の正本は ruviform の `ViewHost`（displayed selection）であり、
  AudioLens は表示 tab を session に保持しない。AudioLens が持つ選択状態は
  job の選択（`selected_job`）だけである。
- receipt の status 語彙は runtime の `accepted`/`applied`/`rejected` に従い、新しい語彙を作らない。
- `tab-select` の成否は service の応答のみで決まる。ruviform の `select_tab` の
  事前検証（key の存在・index 範囲・selection の有無）を AudioLens 側で再実装しない。
  ただし untrusted な params の検証は service の境界で行う。
- ruviform の契約（rejected/unsupported で直前のタブへ戻す、同一 spec の再射影で
  表示を保つ）を consumer 側で再実装・恒久 workaround しない。
- 既存の job 選択（Jobs 表）・Run・Dataset の射影と receipt の挙動
  （第7・第8・第9 slice）は変えない。
