---
id: "DESIGN-034TTalkyZgGfHde64ztJY"
type: "design"
status: "approved"
title: "Native GUI の Dataset workspace Tabs 選択契約の採用と pointer による tab 切替の設計 (第10 slice)"
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
    - "../specs/native-gui-workspace-tabs-selection.md"
  implemented-by: []
  depends-on:
    - "../designs/native-gui-tabs-selection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../test-specs/native-gui-workspace-tabs-selection.md"
  refines:
    - "../designs/native-gui-tabs-selection.md"
  refined-by: []
  derives-from:
    - "../specs/native-gui-workspace-tabs-selection.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native GUI の Dataset workspace Tabs 選択契約の採用と pointer による tab 切替の設計 (第10 slice)

## 概要

Dataset tab の内側にある workspace の Tabs（Source・Spectrogram・Analysis）に
ruviform の `TabsSpec.selection` を採用する。タブの選択は ruviform が intent と
receipt の契約として扱い、AudioLens は intent の受け口（service）と受理の応答
（receipt `applied`）だけを持つ。表示 tab の状態は ruviform の `ViewHost` が正本であり、
AudioLens の session は tab の状態を持たない。

## 1. 契約の値

| 項目 | 値 | 定義の場所 |
| --- | --- | --- |
| target | `al.workspace`（`command::WORKSPACE_TARGET`） | `crates/al-gui/src/command.rs` |
| action | `tab-select`（`command::ACTION_TAB_SELECT` = `ruviform_ui::view::TAB_SELECT_ACTION`） | ruviform の定数を再利用する（第9 slice と同じ） |
| params | `{"index": n}`（`0 <= n < WORKSPACE_TAB_COUNT`） | 生成は ruviform の `select_tab`、検証は service |
| 上限 | `WORKSPACE_TAB_COUNT = 3` | `crates/al-gui/src/view/workspace.rs` |
| component key | `workspace`（`workspace::WORKSPACE_KEY`） | `crates/al-gui/src/view/workspace.rs` |

- target は tab の属する画面を表す論理名で、component key `workspace` に対応させる
  （`al.desktop` ↔ `desktop` と同じ対応）。Dataset tab の内容ではあるが、
  `al.desktop`（root Tabs）とは別の index 空間を持つため target を分ける。
- `al.desktop` と `al.workspace` は同じ action 名 `tab-select` を使うが、
  service は `(target, action)` の組で解釈するため衝突しない。
- index 上限は tabs 数と一致させる。`WORKSPACE_TAB_COUNT` の正本は `view::workspace` で、
  `workspace_screen` の tab 数との一致は `tests/desktop_screen.rs` で固定する。

## 2. 流れ

```
workspace Tabs header 押下 (GPUI の mouse listener)
  └ ViewHost::select_tab(generation, "workspace", n)   ← 表示を即座に n へ移す（optimistic）
      └ ViewEvent::Intent { action: tab-select, params: {index: n}, target: al.workspace }
          └ window へ Action（rvf.ui.command.v1, expect_receipt: true）
              └ DesktopSession → UiRuntime::dispatch_window_intent
                  └ AlGuiService::command
                      ├ 範囲内: result(command_id, ok, {status: applied, index: n})   → Applied（表示を保つ）
                      └ 範囲外・不正: result_err(command_id, INVALID_PARAMS)         → Rejected（直前の tab へ戻す）
```

- `DesktopSession` は `tab-select` を特別扱いしない。runtime への配送と receipt の配送だけを行う。
  `is_action(intent, ACTION_SELECT)` は action 名の完全一致であり、`tab-select` は
  job 選択の `select` と混同されない。
- service は `(target, action)` の組で解釈する。`al.job` の `select`・`resume`、
  `al.desktop` の `tab-select`、`al.workspace` の `tab-select` は独立に対応する。
- workspace の Tabs は Dataset tab の child として spec に常に含まれる
  （`validate_spec` が Panel・Tabs の子を再帰的に flatten し、表示中でない tab の
  子も mount される）。したがって `select_tab("workspace", n)` は Dataset tab が
  表示されていなくても intent を発行できるが、実際の pointer 操作は
  Dataset tab 表示中の workspace header でのみ起こる。

## 3. service の検証境界

- 範囲の検証は service の境界で行う。params は外部入力として扱い、`index` が u64 で
  対象 Tabs の tab 数未満であることを確かめる。
- ruviform の `select_tab` は範囲外・非 selectable・未 mount の key を事前に拒否して
  intent を発行しない。AudioLens の検証はそれと重複するが、IPC 経由の任意の intent に
  対する境界の防御であり、ruviform の判定の再実装ではない。
- `al.desktop` と `al.workspace` の検証は同じ手順で、上限値だけが異なる。
  重複を避けるため index の取り出しと範囲検証は 1 つの helper に集約する。

## 4. 再射影と表示 tab の状態

- `desktop_screen` は root・workspace ともに常に `active: 0` で作る。ruviform の
  `carried_selection` 規則により、同一 key の Tabs は「spec の `active` が前回と同じなら、
  表示中の選択を保つ」。したがって、ユーザーが選んだ workspace の tab は job 選択や
  job 終端による再射影で失われない。
- workspace の tab の子画面（Source 波形・Spectrogram・Analysis）は、tab の表示に
  関係なく毎回の射影で作り直される。tab 切替は射影を変更しないため、切替と
  dirty/`update_view` の再射影は独立に扱える。
- Dataset が開けない場合（job 未選択・dataset 不在）は `dataset_panel` が
  `unavailable` の Panel を返し、workspace の key は spec に存在しない。
  この場合 workspace の選択状態も存在しない — これは「tab が無い」ことの正しい表現であり、
  選択状態を別途保持する設計にはしない（表示 tab の正本は ViewHost の mounted tree）。

## 5. 責務と境界

| 層 | 責務 | 本 slice での変更 |
| --- | --- | --- |
| ruviform（`ruviform-ui`） | 表示 tab の正本、intent の発行、receipt による巻き戻し | なし（rev は `a5b1db13` のまま） |
| `al-gui::command` | intent 語彙（target・action 名） | `WORKSPACE_TARGET` を追加 |
| `al-gui::view` | Tabs の組み立て（selection の有無） | `workspace_screen` が `Some(WORKSPACE_TARGET)`、`WORKSPACE_TAB_COUNT` を定義 |
| `al-gui::service` | `tab-select` の受理と receipt | `(target, action)` の match に workspace を追加し、index 検証を helper に集約 |
| `al-gui::desktop`（session） | 配送と job 選択 | なし |
| tests | 契約の検証（service・session・view・receipt・Fake/GPUI parity） | 追加・更新 |

## 6. 代替案

- **`al.desktop` target を workspace にも使う案**: 却下。root Tabs と workspace Tabs は
  別の index 空間（Jobs/Run/Dataset と Source/Spectrogram/Analysis）を持ち、
  同じ target では service がどちらの上限で検証すべきか区別できない。
  `(target, action)` の組で一意に決まる必要がある。
- **target を `al.dataset.workspace` のような階層名にする案**: 却下。target は
  「tab の属する画面」の論理名であり、`workspace` という component key・画面名と
  `al.workspace` の対応は `al.desktop` ↔ `desktop` と同じ規則で一貫する。
  階層を表す必要は現状ない（KISS）。
- **session に表示 tab を保持して、service が state を更新する案**: 却下（第9 slice と同じ）。
  表示 tab の正本は ruviform の `ViewHost` であり、consumer 側に第二の状態を持つと
  optimistic な表示と食い違う。
- **service が範囲検証を省き、ruviform の事前検証に任せる案**: 却下（第9 slice と同じ）。
  IPC の intent は service の境界に届くため、範囲の検証は AudioLens 側でも必要である（§3）。

## 7. リスク

- ruviform の receipt 契約（rejected/unsupported で巻き戻す）が変わると、
  `tests/receipt.rs` の revert テストが失敗する。これは意図された検出である。
- `WORKSPACE_TAB_COUNT` と `workspace_screen` の tab 数の不一致は、範囲外の受理または
  有効 tab の拒否を招く。`tests/desktop_screen.rs` で一致を固定する。
- nested Tabs（Dataset tab の子としての workspace）は GPUI renderer が再帰的に
  `element_for` するため header の mouse 配線も同じ経路を通るが、実 OS window での
  確認は本 slice では行わない（NotRun）。
- 実 OS window の pointer 操作は本 slice では確認しない（NotRun）。GPUI の mouse 押下から
  `select_tab` までの配線は ruviform 側の検証に依存する。

## 8. 既存設計との関係

- 第9 slice の設計（`native-gui-tabs-selection.md`）の §6「workspace の Tabs も同時に採用する案」
  の却下理由（target と index 上限の定義が必要）は本 slice で解消される。
  同設計の「非スコープ: workspace の Tabs」は本 slice が引き受ける。
- 第9 slice の契約値・検証境界・receipt の流れはそのまま踏襲し、target と上限だけを
  workspace 用に追加する。root Tabs 側の実装・テストは変更しない。
