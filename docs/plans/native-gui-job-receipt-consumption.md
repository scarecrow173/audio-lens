---
id: "PLAN-z8HSDcpYPWcoCvEcsik3gB"
type: "plan"
status: "approved"
title: "Native GUI の job 終端 receipt の消費と再射影検証の実装計画 (第8 slice)"
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
    - "../specs/native-gui-job-receipt-consumption.md"
  implemented-by:
    - "../tasks/native-gui-job-receipt-consumption.md"
  depends-on:
    - "../designs/native-gui-job-receipt-consumption.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../test-specs/native-gui-job-receipt-consumption.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/native-gui-job-receipt-consumption.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native GUI の job 終端 receipt の消費と再射影検証の実装計画 (第8 slice)

## スコープ

- ruviform rev を `a5b1db13` へ更新し、lock・vendor・rev の整合を保つ。
- `TabsSpec.selection` の追加に追従する（`None` を明示）。
- receipt の session 検証（ack → 終端の順序と値）と、ViewHost への適用による Run tab の Resume phase の検証。
- 再射影後の表示中 Run tab と phase の保持を検証する。
- 文書チェーン（spec・design・plan・task・test-spec・impl）と索引の再生成。

非スコープは spec の「非スコープ」に従う（Tabs の pointer 切替、`al.job.run` の UI 接続など）。

## ファイルマップ

| パス | 変更 | 目的 |
| --- | --- | --- |
| `Cargo.lock` | 更新 | ruviform 全 crate の resolved SHA を `a5b1db13` に（`update-ruviform-rev.mjs`） |
| `ui/vendor/ruviform/` | 再生成（gitignore） | `sync-ui-deps` で `a5b1db13` の ui/* を同期 |
| `crates/al-gui/src/view/mod.rs` | 1 行 | `TabsSpec.selection: None` を明示（RF#451 追従、挙動不変） |
| `crates/al-gui/tests/desktop.rs` | 追加 | resume の ack と終端 receipt（applied・rejected）の検証 |
| `crates/al-gui/tests/receipt.rs` | 新規 | ViewHost への receipt 適用と Run tab の phase・再射影の検証 |
| `docs/specs/native-gui-job-receipt-consumption.md` 他 | 新規 | 文書チェーン |
| `docs/specs/README.md` 他の索引 | 再生成 | `doc-maintenance` の managed index |

## 順序と依存

1. rev 更新と `TabsSpec` 追従（コンパイルを通す）— 完了していれば commit 1。
2. session の receipt テストを追加し、通ることを確認（commit 2）。
3. ViewHost の receipt テストを追加し、通ることを確認（commit 2 に含める）。
4. 文書チェーンを作成し、索引を再生成（commit 3）。
5. 最新 `main` を取り込み、検証マトリクスを実行（commit は必要な場合のみ）。

## リスク登録

| リスク | 影響 | 対応 |
| --- | --- | --- |
| ruviform の rev 更新で既存テストが壊れる | 既存挙動の退行 | 全 workspace `test` を実行して確認（実施済み、結果は IR） |
| `TabsSpec` の追加フィールドを誤って既定値にする | Tabs の表示選択の意味が変わる | `selection: None` は RF の既定の意味（spec 駆動）と一致することを確認 |
| 実 GUI で Run tab に到達できない | 受入の実機確認が不可 | 非スコープとして明示し、headless で検証。後続 slice の判断を求める |
| `doc-maintenance apply` が無関係な索引を書き換える | 無関係な差分 | `--type` を絞り、`plan` の結果を確認してから apply |

## ロールバック

- rev を戻すには `node tools/update-ruviform-rev.mjs <旧 SHA>` と `mise run sync-ui-deps` を実行する（旧 rev は `a998101c`）。
  `TabsSpec` の `selection: None` は旧 rev では存在しないため、rev を戻す場合は同時に削除する。
- テストと文書は独立しており、個別に revert できる。

## 検証マトリクス

| 検証 | コマンド（mise task） | 結果の扱い |
| --- | --- | --- |
| rev の整合 | `check-ruviform-lock`, `check-ruviform-rev`, `sync-ui-deps` | Passed/Failed |
| 整形 | `fmt`, `fmt-check` | `fmt` は check のみ。`fmt-check` が存在しない場合は明記 |
| 型・全 target | `check` | Passed/Failed |
| lint | `lint` | Passed/Failed |
| テスト | `test` | Passed/Failed |
| CLI 境界 | `check-cli-headless` | Passed/Failed |
| UI（TS, headless） | `check-ui`, `test-ui` | Passed/Failed |
| GPUI feature | `check-ui-gpui`, `lint-ui-gpui`, `test-ui-gpui` | Passed/Failed |
| driver 境界 | `check-ui-driver-boundary` | mise.toml に存在しないため NotRun として明記 |
| 存在しない parity task | `test-ui-parity` 等 | mise.toml に存在しないため実行せず、PR に明記 |
