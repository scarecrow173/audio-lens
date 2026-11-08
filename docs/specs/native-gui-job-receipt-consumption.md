---
id: "SPEC-sRKVMEZ5VLB52lPAbUYMhA"
type: "spec"
status: "approved"
title: "Native GUI の job 終端 receipt の消費と再射影検証の仕様 (Issue #166 第8 slice)"
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
    - "native-gui-interactive-job-run.md"
    - "https://github.com/scarecrow173/ruviform/issues/456"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/native-gui-job-receipt-consumption.md"
    - "../test-specs/native-gui-job-receipt-consumption.md"
    - "https://github.com/scarecrow173/ruviform/pull/458"
  refines:
    - "native-gui-interactive-job-run.md"
  refined-by: []
  derives-from:
    - "native-gui-interactive-job-run.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/native-gui-job-receipt-consumption.md"
  references: []
  defers: []
  deferred-by: []
---

# Native GUI の job 終端 receipt の消費と再射影検証の仕様 (Issue #166 第8 slice)

## 背景

第7 slice（`native-gui-interactive-job-run.md`）では、Run の Resume は runtime の `accepted` で止まり、
job の終端は Tick の `job` event による再射影で示していた。その時点の ruviform（`a998101c`）は、
job を spawn した command id へ終端 receipt を返さなかったためである（ruviform#456）。

ruviform#458（`a5b1db13`、Issue #456 解消）で、job を spawn する command は次の二段の契約になった。

- ack: 同じ command id へ `accepted`（bare `ok`）を返す。
- 終端: job の終端で同じ command id へ、`Finished` は `applied`、`Failed`・panic・`Cancelled` は `rejected`
  （`ok: false`）の receipt を返す。終端 receipt は終端 job event の後に出る。
- `job.cancel` は独立 command であり、その result は取消要求の ack のみを表す。

本 slice は、この契約を AudioLens が消費して Run tab の Resume の phase を正しく遷移させることと、
slice7 で未検証だった再射影経路と Tabs の状態を検証することを目的にする。

## 目的

- ruviform の依存 rev を `a5b1db13` へ更新し、lock・rev・UI vendor の整合を保つ。
- Resume の command id に対し、ack `accepted` の後、job 終端で `applied`（完了）または `rejected`（失敗・取消）が届く。
- その receipt が Run tab の Resume Button の phase（`Pending` → `Accepted` → `Applied`／`Rejected`）として反映される。
- job 終端の再射影（`update_view` 相当）後も、同一 spec の再射影で表示中の Run tab と Button の phase が保たれることを確かめる。
- Fake（headless）と GPUI の parity を維持する。

## 受入条件

| # | 受入 | 判定基準 |
| --- | --- | --- |
| R1 | rev 更新 | `Cargo.lock` の ruviform 全 crate が `a5b1db13` に解決し、`check-ruviform-lock`・`check-ruviform-rev` が pass する。`ui/vendor/ruviform/REV` が同じ SHA を指す |
| R2 | ack | Resume の ack は `ok` かつ `data.status` を持たない（`accepted`） |
| R3 | 終端 receipt（完了） | job が `Finished` に達すると、同じ command id に `status: applied` の 2 回目の `result` が届き、その `job` event より後に配送される |
| R4 | 終端 receipt（失敗） | job が `Failed` に達すると、同じ command id に `ok: false` の 2 回目の `result` が届く（`rejected`） |
| R5 | phase 遷移 | ViewHost 上の Run tab の Resume Button の phase が `Pending` → `Accepted`（ack）→ `Applied`（終端）と遷移する |
| R6 | 再射影 | job 終端で再射影しても、同一 spec の再射影で表示中の Run tab（`selected`）と Resume の phase は保たれる |
| R7 | parity | `test-ui-gpui`（Fake と GPUI の既存 parity を含む）が pass する |

## スコープ

- ruviform rev の更新（`Cargo.lock`、`ui/vendor/ruviform/REV`）と、その追従に必要な最小の変更
  （RF#451 で `TabsSpec` に加わった `selection` は既存挙動を保つ `None` を明示する）。
- テスト: session（FakeDriver）での ack と終端 receipt の順序・値（`tests/desktop.rs`）と、
  ViewHost への receipt 適用と phase 遷移・再射影（`tests/receipt.rs`）。
- 文書: 本 spec、設計・計画・タスク・検証仕様・実装記録。

## 非スコープ（未達として明示）

- **Tabs の表示切替（pointer による Run tab の選択）**: ruviform の `TabsSpec.selection` が `None` のとき、
  `ViewHost::select_tab` は "tabs are not selectable" で拒否する。AL の Tabs は `selection` を持たないため、
  実 GUI では Run tab へ pointer で移動できない。`selection` の採用には AL 側の `tab-select` action を受ける
  service 応答（receipt `applied`）が要るため、本 slice では扱わず、後続 slice で判断する（コード根拠: ruviform `a5b1db13`
  の `crates/ruviform-ui/src/view.rs`）。
  本 slice の Run tab の検証は、テスト内で spec の `active` を 1 にして mount する Fake 層の検証に限る。
- **Tabs の active 保持（pointer 切替を伴う保持）**: 上記の理由で pointer 切替の保持は検証しない。
  spec 駆動の再射影は `active` の値（Jobs）へ戻る。この挙動は第7 slice の既知の挙動として記録する。
- **新規 Run の起動（`al.job.run`）の UI 接続**: 第7 slice の判断を維持する。入力源を決める別 slice で扱う。
- **job 終端 receipt を用いた画面の確定表示の拡張**（進捗表示、終端の文言など）。
- **Cancel の終端**: 取消は `job.cancel` の ack（`accepted`）のみで、取消された job の終端は resume 側の
  `rejected` receipt として届く。取消経路の headless 検証は既存の `cancel_intent_cancels_the_window_owned_job_and_replies_accepted` に委ね、本 slice では新規に追加しない。
- 実 OS window での目視確認（NotRun）。

## 不変条件

- receipt の status 語彙は runtime の `accepted`/`applied`/`rejected` に従い、新しい語彙を作らない。
- receipt の decode は ruviform の `CommandReceipt::from_result` に集約し、AL 側で再実装しない。
- 同一 command id の複数 `result` の解釈（ack と終端）は ruviform の契約に従い、AL 側で状態を持たない。
- ruviform の契約を consumer 側で再実装・恒久 workaround しない。
