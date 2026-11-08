---
id: "DESIGN-Gtwe8oh10Al6yAAXSmfGkL"
type: "design"
status: "approved"
title: "Native GUI の job 終端 receipt の消費と再射影検証の設計 (第8 slice)"
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
  implemented-by: []
  depends-on:
    - "../designs/native-gui-interactive-job-run.md"
    - "https://github.com/scarecrow173/ruviform/issues/456"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../test-specs/native-gui-job-receipt-consumption.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/native-gui-job-receipt-consumption.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native GUI の job 終端 receipt の消費と再射影検証の設計 (第8 slice)

## 概要

本 slice の AudioLens 側の変更は、ruviform の依存 rev 更新と、その追従に必要な最小の変更だけに絞る。
job 終端 receipt の配送・decode・phase 遷移はすべて ruviform が担う（ruviform#458）。
AudioLens は、その契約を受け取った結果（Run tab の Resume の phase、再射影後の表示）が
期待どおりになることを headless（Fake）で検証する。

## 1. 依存の更新

- `node tools/update-ruviform-rev.mjs a5b1db130d3eeb92dab292b6d431a020e0a8a490` で `Cargo.lock` を更新する。
  cargo の precise 更新は全 ruviform crate を同時に動かすため、単一 SHA の不変条件（`check-ruviform-lock`）が保たれる。
- `mise run sync-ui-deps` で `ui/vendor/ruviform`（gitignore 済み生成物）を同じ SHA で再生成する。
- RF#451 の `TabsSpec.selection` 追加（`Option`、serde default）により、`al-gui` の Tabs 組み立てで
  `selection: None` を明示する。`None` は spec 駆動の active 表示で、従来の挙動と同じである。

## 2. 契約の流れ

```
Run tab Resume Button (ViewHost)
  └ invoke → ViewEvent::Intent { command_id, intent }
      └ window へ Action（rvf.ui.command.v1, expect_receipt: true）
          └ DesktopSession → UiRuntime::dispatch_window_intent
              └ AlGuiService::command → al.job.resume → CallOutcome::Job
                  ├ ack:  result(command_id, ok, data = {job})        → CommandReceipt::Accepted
                  └ 終端: job event(Finished/Failed/Cancelled) の後に
                          result(command_id, applied | ok:false)    → CommandReceipt::Applied | Rejected
```

- ack と終端は同じ command id の 2 つの `result` である。decode は `CommandReceipt::from_result` に集約されており、
  AudioLens は `status` の語彙を解釈しない。
- 終端 receipt は job event の後に配送されるため、`DesktopSession` は Tick の drain で `job` event を
  受けて `projection_dirty` を立て、続けて receipt を window へ配送する。
- `job.cancel` は独立 command で、その result は取消要求の ack のみを表す（`cancel_window_job` の既存の応答）。

## 3. 再射影と Tabs の状態

- AudioLens の `al-desktop` は `projection_dirty` を見て `desktop_screen(base_dir, selected_job)` で画面を作り直し、
  `update_view` で同一 key の tree へ reconcile する。
- RF#451 の `carried_selection` 規則により、同一 key の Tabs は「spec の `active` が前回と同じなら、
  表示中の選択を保つ」。AudioLens は `active` を常に 0（Jobs）で作るため、表示中の Run tab は再射影で失われない。
- 一方、Run tab を表示していない spec（`active` 0）への再射影は、表示を Jobs に戻す。これは spec が
  表示の正本であるための挙動であり、第7 slice の既知の挙動を維持する。
- Tabs に `selection` を持たせると Run tab を pointer で選べるが、その場合は `tab-select` action の
  service 応答（receipt）が必要になる。本 slice では採らない（spec の非スコープ）。

## 4. 責務と境界

| 層 | 責務 | 本 slice での変更 |
| --- | --- | --- |
| ruviform（`ruviform-ui`） | receipt の配送・decode・binding の phase | なし（依存 rev の更新のみ） |
| `al-gui::service` | `al.job.resume` の spawn（`CallOutcome::Job`）、job の結果 | なし |
| `al-gui::desktop` | window への配送、射影の dirty 通知 | なし |
| `al-gui::view` | 画面の `ViewSpec` 射影（Tabs の `selection: None` の明示のみ） | 1 行 |
| `al-gui::tests` | 契約の検証（session・ViewHost） | 追加 |

## 5. 代替案

- **AudioLens 側で終端を検出して receipt を合成する案**（`job` event から `applied` を作る）: 却下。
  receipt の正本は ruviform であり、consumer で再実装すると契約の二重化になる（AGENTS.md の境界、spec の不変条件）。
- **Tabs に `selection` を追加して pointer 切替を有効にする案**: 本 slice では却下。Run tab の到達性は改善するが、
  `tab-select` の service 応答・receipt の扱い・parity の拡張を伴い、受入の範囲を超える。後続 slice で判断する。
- **再射影を止めて phase の遷移だけを見る案**: 却下。終端の再射影は「job 終端を画面へ反映する」受入（第7 slice A6）であり、維持する。

## 6. リスク

- ruviform の receipt 契約が変わると、AudioLens の phase 検証（`tests/receipt.rs`）が失敗する。これは意図された検出である。
- Run tab は実 GUI で pointer では到達できない（上記 3・5）。本 slice の成果は headless の検証に限られ、
  実 OS window での Run tab の操作は NotRun である。
- 終端 receipt の配送順（job event の後）は ruviform の契約であり、AudioLens は順序に依存する表示を持たない。
  順序は session テストで固定する。

## 7. 既存設計との関係

- 第7 slice の設計（`native-gui-interactive-job-run.md`）と仕様は変更しない。その「job 終端の receipt」
  「Tab の状態保持」の非スコープは、本 slice の spec（`refines`）が ruviform#456・#451 の解消を踏まえて補足する。
  Tabs の pointer 切替は引き続き非スコープである。
