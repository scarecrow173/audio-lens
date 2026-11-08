---
id: "PLAN-VTpkboiddjpNtaMhsfyNwK"
type: "plan"
status: "approved"
title: "Native GUI の Jobs 対話選択と Run 再開・取消の実装計画 (第7 slice)"
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
    - "../specs/native-gui-interactive-job-run.md"
  implemented-by:
    - "../tasks/native-gui-interactive-job-run.md"
  depends-on:
    - "../designs/native-gui-interactive-job-run.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../test-specs/native-gui-interactive-job-run.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/native-gui-interactive-job-run.md"
  derived-by:
    - "../tasks/native-gui-interactive-job-run.md"
  verifies: []
  verified-by:
    - "../test-specs/native-gui-interactive-job-run.md"
  references: []
  defers: []
  deferred-by: []
---

# Native GUI の Jobs 対話選択と Run 再開・取消の実装計画 (第7 slice)

## スコープ

設計 `native-gui-interactive-job-run.md` の §1〜§5 を実装する。ruviform 依存の rev 更新、command 語彙、
service op、view の射影、session の選択・取消、composition root の再射影、テストと文書。

## ファイルマップ

| ファイル | 変更 |
| --- | --- |
| `Cargo.lock` | ruviform 20 source を `a998101c` へ（`tools/update-ruviform-rev.mjs`） |
| `ui/vendor/ruviform/` | `sync-ui-deps` で再生成（gitignore 対象・コミットしない） |
| `crates/al-gui/src/command.rs` | 新規。op 名・target・action・`decode_action` |
| `crates/al-gui/src/lib.rs` | `pub mod command` |
| `crates/al-gui/src/service.rs` | `rvf.ui.command.v1` の op 登録と `command`（select・resume） |
| `crates/al-gui/src/view/mod.rs` | `button` 追加、`table` に `selection` 引数 |
| `crates/al-gui/src/view/run_screen.rs` | `run_screen` が actions を受ける、`job_list` を選択可能に |
| `crates/al-gui/src/view/desktop.rs` | Run tab の Resume/Cancel Button |
| `crates/al-gui/src/view/analysis_screen.rs` | `table` 呼び出しに `None` |
| `crates/al-gui/src/desktop.rs` | 選択 job・dirty・cancel 受理・選択 receipt の採用 |
| `crates/al-gui/src/bin/al-desktop.rs` | 初期選択・dirty 時の `update_view` |
| `crates/al-gui/tests/*.rs` | 新規テストと既存呼び出しの更新 |
| `docs/` | 本 slice の文書チェーン、`native-gui-job-run-mount.md` §3・§6・§8、各 README 索引 |

## 順序と依存

1. rev 更新と lock/rev 検証（完了・`3f30b2b`）
2. `command` 語彙と service op（session・view が依存するため先）
3. view の射影（selection・Button）と既存呼び出しの更新
4. session の選択・cancel（2 に依存）
5. bin の再射影（4 に依存）
6. テスト: service op → session → screen → GPUI parity
7. 文書と索引の更新
8. 検証（fmt・clippy・test・GPUI・ruviform 検証・ui）

## リスク登録

| リスク | 影響 | 対応 |
| --- | --- | --- |
| job 終端の receipt が来ない | Resume の phase が accepted で止まる | ruviform#456 を起票。終端は job event で再射影 |
| 再射影で Tabs の active が戻る | 操作後に Jobs へ戻る | 未検証のリスクとして spec・設計に明記。ruviform 側の契約で扱う |
| 行 index と一覧の順序のずれ | 別 job の選択 | service の一覧（job id 昇順）を正本とし、設計 §7 に明記 |
| GPUI の rebuild コスト | 検証時間 | `test-ui-gpui`・`lint-ui-gpui` を分けて実行 |

## ロールバック

コミットは論理単位で分割している（rev 更新・command 語彙・service・view・session・bin・テスト・文書）。
問題があれば直前のコミットへ戻す。`Cargo.lock` は `git checkout -- Cargo.lock` で `d3be1ad9` に戻せる。

## 検証マトリクス

| 項目 | コマンド | 状態 |
| --- | --- | --- |
| 依存 | `check-ruviform-lock`・`check-ruviform-rev` | 実施（task 参照） |
| 形式 | `fmt` | 実施（task 参照） |
| 型・lint（headless） | `check`・`lint`・`check-cli-headless` | 実施（task 参照） |
| テスト（headless） | `test` | 実施（task 参照） |
| GPUI | `check-ui-gpui`・`lint-ui-gpui`・`test-ui-gpui` | 実施（task 参照） |
| UI（TypeScript） | `check-ui`・`test-ui` | 実施（task 参照） |
| 実 OS window | 手動の目視 | NotRun（非スコープ） |
