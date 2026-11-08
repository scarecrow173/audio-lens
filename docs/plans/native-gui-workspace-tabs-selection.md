---
id: "PLAN-034TTalkyYVJto39C4jwnT"
type: "plan"
status: "approved"
title: "Native GUI の Dataset workspace Tabs 選択契約の採用と pointer による tab 切替の実装計画 (第10 slice)"
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
  implemented-by:
    - "../tasks/native-gui-workspace-tabs-selection.md"
  depends-on:
    - "../designs/native-gui-workspace-tabs-selection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../test-specs/native-gui-workspace-tabs-selection.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/native-gui-workspace-tabs-selection.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/native-gui-workspace-tabs-selection.md"
  references: []
  defers: []
  deferred-by: []
---

# Native GUI の Dataset workspace Tabs 選択契約の採用と pointer による tab 切替の実装計画 (第10 slice)

## スコープ

- workspace の Tabs の `selection` 採用（target `al.workspace`）と、`tab-select` の
  service 受理（範囲内 `applied`、範囲外 `rejected`）。
- 検証: service・session・view・receipt（ViewHost への適用と巻き戻し）・Fake/GPUI parity。
- 文書チェーン（spec・design・plan・task・test-spec・impl）と索引。
- ruviform の rev は変更しない（`a5b1db13`）。

非スコープは spec の「非スコープ」に従う（キーボード、実 OS window の pointer 操作、
ruviform 側の契約変更）。

## ファイルマップ

| パス | 変更 | 目的 |
| --- | --- | --- |
| `crates/al-gui/src/command.rs` | 追加 | `WORKSPACE_TARGET`（`al.workspace`） |
| `crates/al-gui/src/view/workspace.rs` | 変更 | `WORKSPACE_TAB_COUNT = 3`、`tabs()` に `Some(WORKSPACE_TARGET)`、モジュール説明 |
| `crates/al-gui/src/view/desktop.rs` | 変更 | モジュール説明に workspace Tabs の選択を追記 |
| `crates/al-gui/src/service.rs` | 変更 | `(WORKSPACE_TARGET, tab-select)` を受理。index 検証を helper に集約 |
| `crates/al-gui/tests/service_ops.rs` | 追加 | `al.workspace` の `tab-select` の範囲内 `applied`・範囲外 `INVALID_PARAMS`・target 独立性 |
| `crates/al-gui/tests/desktop.rs` | 追加 | session 経由の workspace `tab-select` の receipt と job 選択・dirty の不変 |
| `crates/al-gui/tests/desktop_screen.rs` | 追加 | workspace Tabs の `selection` の target、`select_tab` の intent の形、tab 数と `WORKSPACE_TAB_COUNT` の一致 |
| `crates/al-gui/tests/receipt.rs` | 変更 | workspace tab を pointer 相当で選ぶ経路、applied の保持、rejected の巻き戻し、再射影での保持 |
| `crates/al-gui/tests/gpui_parity.rs` | 追加 | Fake と GPUI の workspace `tab-select` intent・観測された選択の一致 |
| `docs/**` | 新規・再生成 | 文書チェーンと索引 |

`src/desktop.rs`（session）と `bin/al-desktop.rs` は変更しない。`tab-select` は
runtime → service → receipt の経路で処理され、session の状態を動かさないためである。

## 順序と依存

1. 契約の値（`command.rs`・`view/workspace.rs` の `WORKSPACE_TAB_COUNT`）を定義する。
2. テストを先に書き、未実装の状態で失敗することを確認する。
3. service の workspace `tab-select` 受理と view の `selection` 採用を実装し、テストを通す（green）。
4. parity と receipt の revert を含む全 al-gui テストを通す。
5. 文書チェーンを完成させ、索引を再生成する。
6. 最新 `origin/main` を取り込み、検証マトリクスを実行する。

## リスク登録

| リスク | 影響 | 対応 |
| --- | --- | --- |
| `al.desktop` と `al.workspace` の target 取り違え | 別 Tabs の上限で検証、または unknown command | `(target, action)` の match で分岐し、service_ops で両 target を検証 |
| `WORKSPACE_TAB_COUNT` と tab 数の不一致 | 範囲外を受理、または有効 tab を拒否 | `tests/desktop_screen.rs` で一致を固定 |
| revert 経路の検証が「移動していない」と区別できない | 偽陽性 | `receipt_statuses` で `rejected` の到着を先に確かめる |
| Dataset が開けない場合に workspace key が無い | 選択対象の不在を誤って扱う | spec の非スコープどおり「tab が無い」状態として扱い、テストでも unavailable Panel の既存検証に委ねる |
| 実 OS window の pointer 確認がない | 実機の不具合を見逃す | NotRun と明記。配線の根拠は ruviform の gpui-test |

## ロールバック

- workspace の `selection` を `None` に戻し、`al.workspace` の `tab-select` 処理を削除すると、
  第9 slice の挙動（workspace Tabs は pointer 非対応）に戻る。`TabsSpec.selection` の値は
  rev に依存しない（`a5b1db13` で既に存在する）ため、rev の変更は不要。
- 文書とテストは独立しており、個別に revert できる。

## 検証マトリクス

| 検証 | コマンド（mise task） | 結果の扱い |
| --- | --- | --- |
| 整形 | `fmt` | Passed/Failed |
| 型・全 target | `check` | Passed/Failed |
| lint（workspace） | `lint` | Passed/Failed |
| テスト（workspace） | `test` | Passed/Failed |
| GPUI feature 型チェック | `check-ui-gpui` | Passed/Failed |
| GPUI feature テスト（parity 含む） | `test-ui-gpui` | Passed/Failed |
| GPUI feature lint | `lint-ui-gpui` | Passed/Failed |
| CLI 境界 | `check-cli-headless` | Passed/Failed |
| rev の整合 | `check-ruviform-lock`, `check-ruviform-rev` | Passed/Failed |
| UI（TS, headless） | `check-ui`, `test-ui` | 本 slice は ui/ を変更しないため、実行した結果のみ記録 |
| driver 境界 | `check-ui-driver-boundary` | mise.toml に存在しないため NotRun として明記 |
