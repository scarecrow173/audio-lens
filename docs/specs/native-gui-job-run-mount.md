---
id: "SPEC-rgCOOoX0ixQnYPjuCm3KLQ"
type: "spec"
status: "draft"
title: "Native GUI の Job/Run tab mount と Tile::columns 消費の仕様 (Issue #166 第6 slice)"
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
    - "native-gui-drawn-parity-run-mount.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/native-gui-job-run-mount.md"
    - "../test-specs/native-gui-job-run-mount.md"
    - "https://github.com/scarecrow173/ruviform/issues/443"
    - "https://github.com/scarecrow173/ruviform/issues/446"
  refines: []
  refined-by: []
  derives-from:
    - "native-gui-drawn-parity-run-mount.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/native-gui-job-run-mount.md"
  references: []
  defers: []
  deferred-by: []
---

# Native GUI の Job/Run tab mount と Tile::columns 消費の仕様

## What Should Be Built

Issue [#166](https://github.com/scarecrow173/audio-lens/issues/166) の AL-G4 残課題を第 6 slice として満たす。
第 5 slice(`native-gui-drawn-parity-run-mount.md`)は al-desktop への Job/Run tab 追加を「UI 契約の判断待ち」として
未達にした。本 slice は、その判断をユーザーが決めた(Job/Run mount を進める)ことを受けて実装する。

- **Ruviform rev の追従**: `3121f8e6` から最新 main `d3be1ad9` へ `Cargo.lock` の 20 source を更新する。
  このリビジョンは `ruviform-viz` の `Tile::columns`・`TileColumn`・`column_offsets`(`ccee0cb1`、RF#443 の
  上流規則の公開)と、空 frame 範囲の `Spectrogram::tile` panic 修正(`d3be1ad9`、RF#446)を含む。
  `Cargo.toml` は変更しない。
- **RF443 の消費**: `view::screen` の列区間の複製 `spectrogram_columns` を除去し、上流 `Tile::columns` の
  frame 区間・`count`(有限セル数、`0` = gap 列)・`magnitudes` をそのまま `SpectrogramColumn` へ写す。
- **al-desktop の Job/Run tab mount**: root 画面を Tabs(Jobs・Run・Dataset)にし、job の置き場 `base_dir` と
  起動時に選んだ job から各 tab を作る。UI 契約は設計書に定める。
- **テスト**: 実 job(`JobService::run`)を使う headless e2e(tab 構造・Job 一覧・Run 画面・Dataset 画面・
  fallback)と、GPUI drawn parity への desktop 画面の追加。

## UI 契約(要点)

詳細と判断理由は設計書(`../designs/native-gui-job-run-mount.md`)に従う。

- 起動: `al-desktop [<base-dir> [<job-id>]]`。`base-dir` 省略時は preferences の `base_dir`、`job-id` 省略時は job を選ばない。
- Jobs tab: `JobService::list` を 1 行 1 job の Table に写す。
- Run tab: 選択 job の永続 manifest の stage 状態・失敗理由・所要時間を Label と Table に写す。
- Dataset tab: 選択 job の `analysis/` を開き、先頭 input を Source・Spectrogram・Analysis の画面へ写す。
- 読めない tab は理由を示す Label に置き換え、画面全体は開く。

## Acceptance

- **AC1** root 画面は Tabs(key `desktop`)で、tab key が `tab-jobs`・`tab-run`・`tab-dataset` の順に並び、
  `ViewHost` へ mount できる。
- **AC2** Jobs tab は `base_dir` 配下の job を、job id を 1 列に並べた Table として表示する。
- **AC3** Run tab は選択 job の stage 状態・`failed` の理由・`duration_ms` を `run_screen` の Label・Table と
  同じ形で表示する。stage 数は manifest と一致する。
- **AC4** Dataset tab は選択 job の dataset を Workspace(Source・Spectrogram・Analysis)の Tabs として表示し、
  Spectrogram は実 tile の列を持つ。
- **AC5** `base_dir` 未設定・job 未選択・job 不在・dataset 未作成のとき、該当 tab は理由を示す Label を
  持ち、他の tab と画面全体は開く。
- **AC6** `spectrogram_screen` の列は `Tile::columns` の frame 区間・`count`・`magnitudes` と一致する。
  空 frame 範囲の tile は全列が空区間で `count == 0` になる。
- **AC7** `GpuiObserver::drawn` が desktop 画面の active tab(Jobs)の Table を、mount した spec の payload
  から導いた期待値と一致する形で描く。
- **AC8** `check-ruviform-lock`・`check-ruviform-rev`・`sync-ui-deps`・`check-ui` が通り、
  Cargo.lock の ruviform source は全 20 package で `d3be1ad9` に一致する。
- **AC9** GPUI 型は `bin/al-desktop.rs` と gpui-test parity test の外に出ない(`check-cli-headless` が通る)。
- **AC10** 既存の数値 oracle(f64・NaN・gap・count・Validity・Provenance・PTS)と既存テストは非回帰
  (`mise run test` と `mise run test-ui-gpui` が通る)。

## Non-goals(本 slice では扱わない)

- **Job 選択の対話操作**: `ViewSpec` の `Table` は読み取り専用で、行選択の intent が無い。本 slice は job を
  起動引数で与える。対話選択は ruviform 側の契約(ruviform#448、設計書 §3・§8)が必要で、未達として扱う。
- **Job の起動・再開・取消**: `ViewSpec` に command を発行する component が無く、desktop からは実行しない。
  Run tab は永続 manifest を読むだけで、実行中の進捗は表示しない。未達として扱う。
- **dataset root の直接指定**: 旧 `al-desktop <dataset-root> <input-id>` は廃止する。dataset は常に job の
  `analysis/` から開く(pre-release のため互換層は設けない)。
- **Dataset tab の input 選択**: 先頭 input のみを表示する。input 切替は UI 操作を要するため対象外。
- **RF443 の残余**: `Tile` の列 count の意味論の変更、Spectrogram の theme token(ruviform#434 の F3)は対象外。
- **実 OS window の表示確認・AL-G5**: 対象外。
- 数値 oracle・Theme・Source の既存契約は変更しない。GPUI 型は通常画面・ViewSpec・Domain・CLI へ出さない。
