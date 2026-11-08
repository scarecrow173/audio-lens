---
id: "PLAN-ibK26JpqMzrShnkIXoAamb"
type: "plan"
status: "approved"
title: "Native GUI の Tabs 選択契約の採用と pointer による画面 tab 切替の実装計画 (第9 slice)"
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
  implemented-by:
    - "../tasks/native-gui-tabs-selection.md"
  depends-on:
    - "../designs/native-gui-tabs-selection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../test-specs/native-gui-tabs-selection.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/native-gui-tabs-selection.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/native-gui-tabs-selection.md"
  references: []
  defers: []
  deferred-by: []
---

# Native GUI の Tabs 選択契約の採用と pointer による画面 tab 切替の実装計画 (第9 slice)

## スコープ

- 画面の root Tabs の `selection` 採用（target `al.desktop`）と、`tab-select` の service 受理（範囲内 `applied`、範囲外 `rejected`）。
- 検証: service・session・view・receipt（ViewHost への適用と巻き戻し）・Fake/GPUI parity。
- 文書チェーン（spec・design・plan・task・test-spec・impl）と索引。
- ruviform の rev は変更しない（`a5b1db13`）。

非スコープは spec の「非スコープ」に従う（workspace の Tabs、キーボード、実 OS window の pointer 操作）。

## ファイルマップ

| パス | 変更 | 目的 |
| --- | --- | --- |
| `crates/al-gui/src/command.rs` | 追加 | `DESKTOP_TARGET`、`ACTION_TAB_SELECT`（ruviform の `TAB_SELECT_ACTION` を再利用） |
| `crates/al-gui/src/view/mod.rs` | 変更 | `tabs()` が `selection: Option<&str>` を受ける（`table()` と同じ形） |
| `crates/al-gui/src/view/workspace.rs` | 変更 | `tabs()` の呼び出しに `None`（workspace は非採用のまま） |
| `crates/al-gui/src/view/desktop.rs` | 変更 | `TAB_COUNT`、root Tabs に `Some(DESKTOP_TARGET)`、モジュール説明 |
| `crates/al-gui/src/service.rs` | 変更 | `command()` を `(target, action)` の match にし、`tab-select` を受理。capability の summary |
| `crates/al-gui/tests/service_ops.rs` | 追加 | `tab-select` の範囲内 `applied`・範囲外 `INVALID_PARAMS` |
| `crates/al-gui/tests/desktop.rs` | 追加 | session での `tab-select` の receipt と job 選択・dirty の不変 |
| `crates/al-gui/tests/desktop_screen.rs` | 追加 | root Tabs の `selection` と intent の形、`TAB_COUNT` と tab 数の一致 |
| `crates/al-gui/tests/receipt.rs` | 変更 | Run tab を pointer 相当で選ぶ harness、applied の保持、rejected の巻き戻し、再射影での保持 |
| `crates/al-gui/tests/gpui_parity.rs` | 追加 | Fake と GPUI の `tab-select` intent・観測された選択の一致 |
| `docs/**` | 新規・再生成 | 文書チェーンと索引 |

`src/desktop.rs`（session）は変更しない。`tab-select` は runtime → service → receipt の経路で処理され、session の状態を動かさないためである。

## 順序と依存

1. 契約の値（`command.rs`）と view の `tabs()` 変更（コンパイルを通す）。
2. テストを先に書き、`TAB_COUNT` の未定義によりコンパイルが失敗することを確認する。
3. service の `tab-select` 受理と view の `selection` 採用を実装し、テストを通す（green）。
4. parity と receipt の revert を含む全 al-gui テストを通す。
5. 文書チェーンを作成し、索引を再生成する。
6. 最新 `origin/main` を取り込み、検証マトリクスを実行する。

## リスク登録

| リスク | 影響 | 対応 |
| --- | --- | --- |
| 範囲検証の重複（ruviform の事前検証と service） | 冗長 | IPC 境界の防御として残す（design §3）。判定の再実装ではない |
| `TAB_COUNT` と `desktop_screen` の tab 数の不一致 | 範囲外を受理、または有効 tab を拒否 | `tests/desktop_screen.rs` で一致を固定 |
| revert 経路の検証が「移動していない」と区別できない | 偽陽性 | `receipt_statuses` で `rejected` の到着を先に確かめる |
| workspace tab が pointer で到達できない | 受入の範囲外の残課題 | 非スコープとして明示。後続 slice で `al.workspace` を定義 |
| 実 OS window の pointer 確認がない | 実機の不具合を見逃す | NotRun と明記。配線の根拠は ruviform の gpui-test |

## ロールバック

- `selection` を `None` に戻し、`tab-select` の処理を削除すると、第8 slice の挙動に戻る。`TabsSpec.selection` の値は
  rev に依存しない（`a5b1db13` で既に存在する）ため、rev の変更は不要。
- 文書とテストは独立しており、個別に revert できる。

## 検証マトリクス

| 検証 | コマンド（mise task） | 結果の扱い |
| --- | --- | --- |
| 整形 | `fmt` | Passed/Failed |
| 型・全 target | `check` | Passed/Failed |
| lint（workspace） | `lint` | Passed/Failed |
| テスト（workspace） | `test` | Passed/Failed |
| GPUI feature テスト（parity 含む） | `test-ui-gpui` | Passed/Failed |
| GPUI feature lint | `lint-ui-gpui` | Passed/Failed |
| CLI 境界 | `check-cli-headless` | Passed/Failed |
| rev の整合 | `check-ruviform-lock`, `check-ruviform-rev` | Passed/Failed |
| UI（TS, headless） | `check-ui`, `test-ui` | 本 slice は ui/ を変更しないため、実行した結果のみ記録 |
| driver 境界 | `check-ui-driver-boundary` | mise.toml に存在しないため NotRun として明記 |
