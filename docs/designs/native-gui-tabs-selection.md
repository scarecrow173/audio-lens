---
id: "DESIGN-g3Nc8glJv8VXvtDiOoZzpP"
type: "design"
status: "approved"
title: "Native GUI の Tabs 選択契約の採用と pointer による画面 tab 切替の設計 (第9 slice)"
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
    - "../specs/native-gui-tabs-selection.md"
  implemented-by: []
  depends-on:
    - "../designs/native-gui-job-receipt-consumption.md"
    - "https://github.com/scarecrow173/ruviform/pull/458"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../test-specs/native-gui-tabs-selection.md"
  refines:
    - "../designs/native-gui-job-receipt-consumption.md"
  refined-by: []
  derives-from:
    - "../specs/native-gui-tabs-selection.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native GUI の Tabs 選択契約の採用と pointer による画面 tab 切替の設計 (第9 slice)

## 概要

画面の root Tabs（Jobs・Run・Dataset）に ruviform の `TabsSpec.selection` を採用する。
タブの選択は ruviform が intent と receipt の契約として扱い、AudioLens は intent の受け口（service）と
受理の応答（receipt `applied`）だけを持つ。表示 tab の状態は ruviform の `ViewHost` が正本であり、
AudioLens の session は tab の状態を持たない。

## 1. 契約の値

| 項目 | 値 | 定義の場所 |
| --- | --- | --- |
| target | `al.desktop`（`command::DESKTOP_TARGET`） | `crates/al-gui/src/command.rs` |
| action | `tab-select`（`command::ACTION_TAB_SELECT` = `ruviform_ui::view::TAB_SELECT_ACTION`） | ruviform の定数を再利用する |
| params | `{"index": n}`（`0 <= n < TAB_COUNT`） | 生成は ruviform の `select_tab`、検証は service |
| 上限 | `TAB_COUNT = 3` | `crates/al-gui/src/view/desktop.rs` |

action 名は ruviform の定数をそのまま使う。AudioLens が同じ文字列を別に定義すると、表現の二重化になるためである。
target は tab の属する画面を表す論理名で、既存の `al.job`（Jobs・Run の操作）と分ける。
workspace の Tabs は同じ action 名を使うが target が異なるため、将来 target を分けて扱える（本 slice では採らない）。

## 2. 流れ

```
Desktop Tabs header 押下 (GPUI の mouse listener)
  └ ViewHost::select_tab(generation, "desktop", n)       ← 表示を即座に n へ移す（optimistic）
      └ ViewEvent::Intent { action: tab-select, params: {index: n}, target: al.desktop }
          └ window へ Action（rvf.ui.command.v1, expect_receipt: true）
              └ DesktopSession → UiRuntime::dispatch_window_intent
                  └ AlGuiService::command
                      ├ 範囲内: result(command_id, ok, {status: applied, index: n})   → Applied（表示を保つ）
                      └ 範囲外・不正: result_err(command_id, INVALID_PARAMS)         → Rejected（直前の tab へ戻す）
```

- `DesktopSession` は `tab-select` を特別扱いしない。runtime への配送と receipt の配送だけを行う。
  `tab-select` は job の選択（`selected_job`）や射影の dirty を変えないため、session の状態は動かない。
- service は `(target, action)` の組で解釈する。`al.job` の `select`・`resume` と `al.desktop` の `tab-select` は独立に対応する。

## 3. service の検証境界

- 範囲の検証は service の境界で行う。params は外部入力として扱い、`index` が u64 で `TAB_COUNT` 未満であることを確かめる。
- ruviform の `select_tab` は範囲外を事前に拒否して intent を発行しない。AudioLens の検証はそれと重複するが、
  IPC 経由の任意の intent に対する境界の防御であり、ruviform の判定の再実装ではない。
- `TAB_COUNT` の正本は `view::desktop`。`desktop_screen` の tab 数との一致は、`tests/desktop_screen.rs` で固定する。

## 4. 再射影と表示 tab の状態

- `desktop_screen` は常に `active: 0`（Jobs）で作る。ruviform の `carried_selection` 規則により、同一 key の Tabs は
  「spec の `active` が前回と同じなら、表示中の選択を保つ」。したがって、ユーザーが選んだ tab は job 終端や
  job 選択による再射影で失われない（`tests/receipt.rs` の再射影テストで固定）。
- 第8 slice の「spec 駆動の再射影で Jobs に戻る」は、`active` が変わった場合の ruviform の挙動であり、
  AudioLens の projection では起こらない。この記述は第8 slice の spec・design の時点の状態として残す。
- tab の子画面（Jobs 表・Run・Dataset）は、tab の表示に関係なく毎回の射影で作り直される。tab 切替は射影を
  変更しないため、切替と dirty/`update_view` の再射影は独立に扱える。

## 5. 責務と境界

| 層 | 責務 | 本 slice での変更 |
| --- | --- | --- |
| ruviform（`ruviform-ui`） | 表示 tab の正本、intent の発行、receipt による巻き戻し | なし（rev は `a5b1db13` のまま） |
| `al-gui::command` | intent 語彙（target・action 名） | `DESKTOP_TARGET`・`ACTION_TAB_SELECT` を追加 |
| `al-gui::view` | Tabs の組み立て（selection の有無） | `tabs()` が selection を受け、`desktop_screen` は `Some`、workspace は `None` |
| `al-gui::service` | `tab-select` の受理と receipt | `(target, action)` の match に `tab-select` を追加 |
| `al-gui::desktop`（session） | 配送と job 選択 | なし |
| tests | 契約の検証（service・session・view・receipt・Fake/GPUI parity） | 追加・更新 |

## 6. 代替案

- **workspace の Tabs も同時に採用する案**: 本 slice では却下。`tabs()` の呼び出しは同一だが、target と index 上限を
  workspace 用に定義する必要があり、受入の範囲（Run・Dataset の到達）を超える。後続 slice で `al.workspace` として扱う。
- **session に表示 tab を保持して、service が state を更新する案**: 却下。表示 tab の正本は ruviform の `ViewHost` であり、
  consumer 側に第二の状態を持つと、optimistic な表示と食い違う（AGENTS.md の単一の正本の原則）。
- **service が範囲検証を省き、ruviform の事前検証に任せる案**: 却下。IPC の intent は service の境界に届くため、
  範囲の検証は AudioLens 側でも必要である（§3）。
- **`tab-select` の receipt を `accepted` で返す案**: 却下。`applied` は表示の受理を表し、表示 tab の確定に使う。

## 7. リスク

- ruviform の receipt 契約（rejected/unsupported で巻き戻す）が変わると、`tests/receipt.rs` の revert テストが失敗する。これは意図された検出である。
- workspace の tab は pointer で到達できない。Dataset の中の Spectrogram・Analysis は、実 GUI では本 slice の後も到達できない（§非スコープ）。
- 実 OS window の pointer 操作は本 slice では確認しない（NotRun）。GPUI の mouse 押下から `select_tab` までの配線は
  ruviform 側の検証に依存する。

## 8. 既存設計との関係

- 第8 slice の設計（`native-gui-job-receipt-consumption.md`）の §3 と §5 の「Tabs の pointer 切替は非スコープ」は、
  本 slice で refines される。その時点の判断は当時の根拠（`selection` 未採用）として残す。
- 第7 slice の設計（`native-gui-interactive-job-run.md`）の job 選択・Resume の契約は変更しない。
