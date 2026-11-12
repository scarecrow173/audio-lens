---
id: "IMPL-m8DwS64am3rDitdQyEvkiV"
type: "impl"
status: "completed"
title: "Native GUI の job 終端 receipt の消費と再射影検証の実装記録(IR, 第8 slice)"
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
    - "../../tasks/native-gui-job-receipt-consumption.md"
  blocks: []
  supersedes: []
  superseded-by: []
  refines: []
  refined-by: []
  derives-from:
    - "../../plans/native-gui-job-receipt-consumption.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/456"
    - "https://github.com/scarecrow173/ruviform/pull/458"
    - "https://github.com/scarecrow173/ruviform/issues/451"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Native GUI の job 終端 receipt の消費と再射影検証の実装記録(IR, 第8 slice)

## Summary

Native GUI の job 終端 receipt の消費と再射影検証(第8 slice)。
ruviform rev を `a5b1db13`(RF#458)へ更新し、resume の ack・
job 終端 receipt・Run tab の phase 遷移・同一 spec 再射影での
表示保持を FakeDriver/ViewHost の e2e で検証した。receipt の
decode・phase 遷移の実装は ruviform 側を利用し、AL 側の再実装は
追加していない。

## Context

Issue #166(`relations.source`)の Native GUI slice(第8 slice)。
task `docs/tasks/native-gui-job-receipt-consumption.md`
(depends-on)と plan
`docs/plans/native-gui-job-receipt-consumption.md`
(derives-from)に従う。references の ruviform Issue/PR
(#456・#458・#451)に対応する rev `a5b1db13` へ更新し、receipt の
配送・decode・phase 遷移は ruviform 側契約に委譲する。

## Implementation

- ruviform の依存 rev を `fd76b18` 時点の `a998101c` から `a5b1db13`（RF#458、Issue #456 解消）へ更新した。
  `node tools/update-ruviform-rev.mjs a5b1db13…` で `Cargo.lock` の 20 package を更新し、`sync-ui-deps` で
  `ui/vendor/ruviform` を再生成した（gitignore 済み）。
- RF#451 で `TabsSpec` に加わった `selection` を `al-gui::view::tabs` で `None` と明示した（挙動不変）。
- `crates/al-gui/tests/desktop.rs`: resume の ack（`accepted`、`status` なし）と、job 終端の 2 回目の `result`
  （完了は `applied`・job event の後、失敗は `ok:false`）を session（FakeDriver）で検証する 2 件を追加した。
- `crates/al-gui/tests/receipt.rs`（新規）: ViewHost に Run tab を mount し、Resume Button の phase が
  `Pending` → `Accepted` → `Applied` と遷移すること、同一 spec の再射影で表示中の Run tab と phase が保たれること、
  spec の `active` が Jobs に戻る再射影の既知の挙動を検証する 2 件を追加した。
- 文書: spec・design・plan・task・test-spec を追加し、各 type の managed index を `doc-maintenance` で再生成した（追加行のみ）。
- AL 側の receipt の decode・phase 遷移の実装は追加していない。receipt は ruviform が配送し decode する（AL 側の再実装なし）。

## Related Experiments

### Adopted

- なし — `docs/impl/exp/` に本 slice に対応する実験ログは
  存在しない(既存ログは CI 系 issue-139〜153 のみ)。
  `metadata.experiments` も空。

### Rejected

- なし(同上)。

## Validation

### 受入対応表

| 受入 | 結果 | 根拠 |
| --- | --- | --- |
| R1 rev 整合 | 通過 | `check-ruviform-lock`・`check-ruviform-rev`・`sync-ui-deps` |
| R2 ack | 通過 | `resume_is_acked_then_receives_an_applied_receipt_after_the_job_finishes`（`tests/desktop.rs`） |
| R3 終端（完了） | 通過 | 同上（2 回目 `applied`、job event より後、`take_projection_dirty`） |
| R4 終端（失敗） | 通過 | `resume_of_a_missing_job_ends_with_a_rejected_receipt` |
| R5 phase 遷移 | 通過 | `resume_receipt_moves_the_run_button_from_pending_to_applied`（`tests/receipt.rs`） |
| R6 再射影の保持 | 通過（同一 spec の場合） | 同上。spec の `active` が変わる再射影は Jobs に戻る（既知の挙動、`spec_driven_reprojection_returns_the_displayed_tab_to_jobs`） |
| R7 parity | 通過 | `mise run test-ui-gpui`（`gpui_parity.rs` を含む） |

### 検証結果（コード HEAD `e5a4ca2`、検証時点の `origin/main` は `fd76b18` のまま）

| task | 結果 |
| --- | --- |
| `mise run fmt` | 通過（exit 0。`fmt` は `cargo fmt --all -- --check`） |
| `mise run check` | 通過（exit 0） |
| `mise run lint` | 通過（exit 0。biome は 5 warnings・2 infos を出すが、いずれも `tools/` 配下の既存ファイルで本 slice の変更外） |
| `mise run test` | 通過（exit 0。全 test binary で `0 failed`） |
| `mise run test-ui-gpui` | 通過（exit 0。`desktop` 20 件、`receipt` 2 件、`gpui_parity` を含む） |
| `mise run check-ui-gpui` / `lint-ui-gpui` | 通過（exit 0） |
| `mise run check-cli-headless` | 通過（exit 0） |
| `mise run check-ruviform-lock` | 通過（`a5b1db130d`、20 lockfile source、単一 SHA） |
| `mise run check-ruviform-rev` | 通過（lockfile-primary、REV match） |
| `mise run sync-ui-deps` | 通過（`synced ruviform ui/* @ a5b1db130d`） |
| `mise run check-ui` | 通過（exit 0、`tsc --noEmit`） |
| `mise run test-ui` | 通過（112 pass、0 fail） |

- 環境の補足: worktree に `ui/node_modules` が無く、初回の `check-ui`・`test-ui` は失敗した（`tsc`・`happy-dom` が無い）。
  `mise run setup-ui`（`npm ci --prefix ui`、lockfile からの復元、依存の変更なし）の後に再実行して通過した。コードの失敗ではない。
- 失敗から直した点: 初回の `cargo check` で `TabsSpec.selection` が不足していた（rev 追従）。`selection: None` で解消した。

### NotRun / Unsupported

- `mise run check-ui-driver-boundary`・`test-ui-parity`・`fmt-check`: `mise.toml` に存在しない task 名のため未実行（spec の記載は実在しない）。
- `mise run check-ui-webview` / `lint-ui-webview` / `test-ui-webview`: webview feature に触れないため未実行。
- CI profile（`check-ci`・`lint-ci`・`test-ci`・`build-ci`・`verify-ci`）: 未実行（dev profile の `check`・`lint`・`test` で代替）。
- 実 OS window での Run tab の操作と目視: NotRun（Run tab は pointer で到達できない。上記の判断を参照）。
- `doc-maintenance plan` の全体の診断: 既存文書の `broken-relation`・`orphan-relation` 等が多数あり、本 slice と無関係。
  apply は type ごとに managed index の再生成のみを行い、追加行は本 slice の 5 文書だけであることを diff で確認した。

### 独立レビュー（review-protocol）の結果

- 独立の subagent は、この worker の指示（subagent 不使用）に従い起動しなかった。protocol の役割（Reviewer・Validator・Judge）は
  同一 worker の別パスとして実施した。レビュー回数は 1 回。
- 協調パス: 受入条件と検証結果の対応を確認した。
- 敵対パス: 「テストが誤って通る」可能性を確認した。`results()` の ack と終端の件数・順序、`job_end < receipt`、
  `rejected` の `ok:false` を assert しており、件数の不一致や順序の逆転で失敗する。
- 検証パス: 指摘を 3 件に絞り、実物で確認した。
  1. 設計の「既存設計を更新した」という記述が実際の編集と違っていた → 「変更しない・refines で補足」に修正（CONFIRMED・P2、修正済み）。
  2. test-spec の「ui_dvr_03 が取消の rejected を検証」という記述が誤り（panic 経路）→ 訂正（CONFIRMED・P2、修正済み）。
  3. 新規 5 文書の ID が slice7 の design・test-spec と重複していた → 新 ID を採番し直した（CONFIRMED・P1、修正済み）。
- blocker は残っていない。

## Risks

- Tabs の `selection` を採用しない判断により、実 GUI で Run tab
  へ pointer 到達できない — Resume の実機確認は Fake 層
  (spec の `active` を 1 にした mount)での検証に留まる。
- 取消(`Cancelled`)された resume job の終端 receipt は
  ruviform の `Cancelled → rejected` 分類に依存しており、
  AudioLens 側の直接検証は未実施。
- spec の `active` が変わる再射影では表示中の Run tab が Jobs
  に戻る既知の挙動がある(同一 spec の再射影では phase は
  保持される)。

## Follow-ups

### 判断と未達

- **Tabs の pointer 切替は実装しない（判断）**: ruviform の `ViewHost::select_tab` は `TabsSpec.selection` が `None` のとき
  "tabs are not selectable" で拒否する。AL の Tabs は `selection` を持たないため、実 GUI で Run tab へ移動できない。
  `selection` を採用するには AL の service が `tab-select` action に応答（receipt `applied`）する必要があり、
  受入の範囲を超えるため本 slice では採らなかった。後続 slice で判断を求める（Coordinator へ報告）。
- **Run tab の検証の方法（判断）**: 上記のため、テストは spec の `active` を 1 にして Run tab を mount する Fake 層で行った。
  実 GUI の操作ではない。
- **ruviform への Issue 起票は行わない**: 本 slice で ruviform の契約の欠陥は見つからなかった（receipt の配送・decode・
  Tabs の `selection` の既定の意味は契約どおり）。Tabs の pointer 切替は AL 側の採用判断であり、ruviform の不具合ではない。

### 逸脱・残課題

- 実 GUI で Run tab に到達できない（Tabs の `selection` 未採用）。Resume の実機確認は次 slice の前提である。
- `al.job.run` の UI 接続は第7 slice の判断を維持し、本 slice の対象外。
- 取消（`Cancelled`）された resume job の終端 receipt の AudioLens 側の直接検証は未実施（ruviform の `Cancelled → rejected` の分類に依る）。
