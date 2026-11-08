---
id: "TASK-o02rWo6RN2mne2tPgyG9Cx"
type: "task"
status: "approved"
title: "Issue #166 job 終端 receipt の消費と再射影検証 実装タスク (第8 slice)"
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
    - "../plans/native-gui-job-receipt-consumption.md"
  implemented-by: []
  depends-on:
    - "../plans/native-gui-job-receipt-consumption.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/native-gui-job-receipt-consumption.md"
    - "../designs/native-gui-job-receipt-consumption.md"
    - "../test-specs/native-gui-job-receipt-consumption.md"
    - "https://github.com/scarecrow173/ruviform/issues/456"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/native-gui-job-receipt-consumption.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #166 job 終端 receipt の消費と再射影検証 実装タスク (第8 slice)

## T1: ruviform rev を a5b1db13 へ更新する

- [x] `node tools/update-ruviform-rev.mjs a5b1db130d3eeb92dab292b6d431a020e0a8a490`
- [x] `mise run sync-ui-deps`
- [x] `mise run check-ruviform-lock`・`mise run check-ruviform-rev` が pass する
- [x] `TabsSpec.selection` の追加に追従（`selection: None`、挙動不変）し、`mise run check` が pass する

## T2: resume の ack と終端 receipt を session で検証する

- [x] `tests/desktop.rs`: `resume_is_acked_then_receives_an_applied_receipt_after_the_job_finishes`（R2・R3）
- [x] `tests/desktop.rs`: `resume_of_a_missing_job_ends_with_a_rejected_receipt`（R4）

## T3: Run tab の Resume phase と再射影を ViewHost で検証する

- [x] `tests/receipt.rs`: `resume_receipt_moves_the_run_button_from_pending_to_applied`（R5・R6）
- [x] `tests/receipt.rs`: `spec_driven_reprojection_returns_the_displayed_tab_to_jobs`（既知の挙動の記録）

## T4: 検証と文書

- [x] `mise run fmt`（`cargo fmt --all` で整形後に pass）
- [x] `mise run test`・`mise run test-ui-gpui` が pass する（R7）
- [ ] `mise run lint`・`check-cli-headless`・`check-ui`・`test-ui`・`check-ui-gpui`・`lint-ui-gpui` の結果を IR に記録する
- [ ] `docs/` の索引を `doc-maintenance` で再生成する（対象 type を絞る）
- [ ] 最新 `origin/main` を取り込み、再度検証する

## 非タスク（本 slice で扱わない）

- Tabs の pointer 切替（`selection` の採用）と `tab-select` の service 応答
- `al.job.run` の UI 接続
