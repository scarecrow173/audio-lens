---
id: "SPEC-nakcIRYkow5Nc6ljaz99ft"
type: "spec"
status: "approved"
title: "Native GUI の Jobs 対話選択と Run 再開・取消の仕様 (Issue #166 第7 slice)"
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
    - "native-gui-job-run-mount.md"
    - "https://github.com/scarecrow173/ruviform/issues/448"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/native-gui-interactive-job-run.md"
    - "../test-specs/native-gui-interactive-job-run.md"
    - "https://github.com/scarecrow173/ruviform/issues/456"
  refines: []
  refined-by: []
  derives-from:
    - "native-gui-job-run-mount.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/native-gui-interactive-job-run.md"
  references: []
  defers: []
  deferred-by: []
---

# Native GUI の Jobs 対話選択と Run 再開・取消の仕様 (Issue #166 第7 slice)

## 背景

第6 slice（`native-gui-job-run-mount.md`）で al-desktop は Jobs・Run・Dataset の Tabs を mount したが、
job の選択は起動引数だけで、Run の操作（再開・取消）は未接続だった。ruviform#448（`a998101c`）で
読み取り専用 Table の行選択と `Button` による command 起動の ViewSpec 契約が入ったため、本 slice で
これらを AudioLens の Job 操作に接続する。

## 目的

- ユーザーが Jobs の行を選ぶと、その job が Dataset tab（先頭 input の Source・Spectrogram・Analysis）と Run tab に反映される。
- 選択中の job に対して Run の再開・取消を Button から行える。既存の service/domain 操作だけを使う。
- 起動引数 `<job-id>` は初期選択として機能し、対話選択が主経路になる。
- 同じ ViewSpec に対し Fake（headless）と GPUI の intent・snapshot が一致することを検査する。
- driver-neutral な ViewSpec のみを使い、GPUI 型を通常の view・domain・CLI 層に持ち込まない。

## 受入条件

| # | 受入 | 判定基準 |
| --- | --- | --- |
| A1 | Jobs の行選択 | Jobs の Table が行選択可能で、選択 intent（`action: select`, `params: {row}`）を発行する |
| A2 | 選択の反映 | 選択が receipt `applied` で確定すると、Run・Dataset tab が選択 job で再射影される |
| A3 | 拒否の扱い | 範囲外の行など receipt `rejected` の選択は表示上も直前の選択に戻り、画面の job は変わらない |
| A4 | 再開 | Run tab の Resume Button が選択 job の `al.job.resume` を起動する。receipt は runtime の `accepted` |
| A5 | 取消 | Run tab の Cancel Button が window の実行中 job を取り消す。取消対象が無ければ `rejected`、受理なら `accepted` |
| A6 | 終端の反映 | job が終端（finished / failed / cancelled）に達したら Run・Dataset tab を再射影する |
| A7 | 初期選択 | 起動引数 `<job-id>` は初期の job 選択として Run・Dataset tab を射影する |
| A8 | parity | 同一 ViewSpec で Fake と GPUI の select・invoke intent と selected row が一致する |
| A9 | 境界 | GPUI 型は `bin/al-desktop.rs` 以外に出ない。`check-cli-headless` が pass する |

## スコープ

- `al-gui`: command intent 契約（`command` モジュール）、service の `rvf.ui.command.v1` op、Jobs 選択可能 Table、Run tab の Resume/Cancel Button、`DesktopSession` の選択追跡と cancel 受理、al-desktop の再射影。
- `ruviform` の依存 rev を `a998101c` へ更新（`Cargo.lock` と `ui/vendor`）。
- テスト: service op・desktop session（FakeDriver）・desktop screen（Fake）・GPUI parity。
- 文書: 本 spec、設計・計画・タスク・検証仕様、`native-gui-job-run-mount.md` §3・§6 の更新。

## 非スコープ（未達として明示）

- **新規 Run の起動（`al.job.run`）の UI 接続**: 新規 job には入力 file の指定が要るが、この slice には入力選択 UI が無い。
  `al.job.run` は service op として既存のまま。UI からの新規起動は入力源を決める別 slice で扱う。
  なお既存 job の定義から再実行する案は採らない（`resolve_config` が現在の辞書で snapshot を置き換えるため、
  保存済み定義の再現性を損なう）。
- **job 終端の receipt**: 終端の `applied`/`rejected` は runtime が command id へ返さない（ruviform#456）。
  本 slice の Resume は `accepted` で止まり、進捗・終端は Run tab の再射影で示す。
- **進捗表示**: stage の進捗は表示しない（永続 manifest の最終状態のみ、`native-gui-job-run-mount.md` §5 を継承）。
- **Tab の状態保持**: 再射影で Tabs の `active` は spec の値（Jobs）に戻る。保持するには ruviform 側の契約が要るため本 slice では扱わない。
- **初期選択行の強調**: 起動引数の job は Run・Dataset に反映するが、Jobs 表の行の強調（selected row）は初期化しない。
- **webview driver**: `ruviform-ui-webview` は `ViewSpec` を描画しない（RF#448 の非スコープ）。
- 実 OS window での目視確認は本 slice の必須条件にしない（NotRun）。

## 不変条件

- 選択の正本は ViewHost の選択行（表示）と、session の `selected_job`（射影の入力）の二つだけとし、job の一覧・manifest の第二の状態は持たない。
- 行 index から job への対応は service 層の一覧（`JobService::list`）だけが決める。
- receipt の status 語彙は runtime の `accepted`/`applied`/`rejected` に従い、新しい語彙を作らない。
- ruviform の契約を consumer 側で再実装・恒久 workaround しない。receipt 欠落は ruviform#456 で扱う。
