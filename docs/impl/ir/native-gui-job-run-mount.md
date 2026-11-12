---
id: "IMPL-OXlJYYPSVLg1KyiRb5CoNx"
type: "impl"
status: "completed"
title: "Native GUI の Job/Run tab mount と Tile::columns 消費の実装記録(IR, 第6 slice)"
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
    - "../../tasks/native-gui-job-run-mount.md"
  blocks: []
  supersedes: []
  superseded-by: []
  refines: []
  refined-by: []
  derives-from:
    - "../../plans/native-gui-job-run-mount.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/443"
    - "https://github.com/scarecrow173/ruviform/issues/446"
    - "https://github.com/scarecrow173/ruviform/issues/448"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Native GUI の Job/Run tab mount と Tile::columns 消費の実装記録(IR, 第6 slice)

## Summary

Native GUI の Job/Run tab mount と `Tile::columns` 消費
(第6 slice)。ruviform rev を `d3be1ad9` へ更新し、
`spectrogram_columns` 複製を削除して `Tile::columns()` を消費、
`view::desktop` の root Tabs(Jobs・Run・Dataset)と al-desktop の
`[<base-dir> [<job-id>]]` 引数を実装した。

## Context

Issue #166(`relations.source`)の Native GUI slice(第6 slice)。
task `docs/tasks/native-gui-job-run-mount.md`(depends-on)と
plan `docs/plans/native-gui-job-run-mount.md`(derives-from)に
従う。references の ruviform#443/#446/#448 に対応し、rev
`d3be1ad9` の `Tile::columns` 契約と ViewSpec の Tabs/Panel/
Table/Spectrogram を利用する。

## Implementation

コード HEAD は `adbd5ed`。`origin/main`(`6516091`)からの 4 commit と本 IR の commit から成る。

| commit | 内容 |
| --- | --- |
| `f7232c2` | `chore(ruviform)`: ruviform rev を `3121f8e6` から `d3be1ad9` へ。`Cargo.lock` の 20 source のみ。`sync-ui-deps` で `ui/vendor` を再生成 |
| `1fd8855` | `feat(al-gui)`: `view::screen::spectrogram_columns` の複製を削除し、`Tile::columns()` の frame 区間・`count`・`magnitudes` を `SpectrogramColumn` へ写す |
| `07f421c` | `feat(al-gui)`: `view::desktop`(root Tabs: Jobs・Run・Dataset)、`RunOutcome::from_manifest`、`tabs`・`unavailable` の共有 helper、al-desktop の引数 `[<base-dir> [<job-id>]]` と mount、テスト |
| `adbd5ed` | `docs(gui)`: spec・design・plan・test-spec・task と各 README の索引 |

- RF443: `spectrogram_screen` は `tile.columns()` を消費する。非空 tile の span は複製と一致する
  (`cols > span` でも `cf1 ≤ span` が成り立つため、クランプは無変更)。差分は空 frame 範囲の tile だけで、
  上流は全列を `7..7`・`count == 0` にする。複製は `7..8` を返していた。
- Job/Run mount: `desktop_screen(base_dir, job)` が root Tabs(`desktop`)を作る。
  - `tab-jobs`: `JobService::list` を 1 行 1 job の Table(`job_list`)に写す。
  - `tab-run`: `JobService::inspect` の manifest を `RunOutcome::from_manifest` で写し、`run_screen` を使う。
  - `tab-dataset`: 要求された job の `open_dataset` と manifest の先頭 input で `workspace_screen` を使う。
- 各 tab は独立に読む。失敗した tab は `unavailable: <理由>` の Label を持つ Panel になり、画面全体は開く。
- al-desktop: 引数 `[<base-dir> [<job-id>]]`。base_dir は引数、無ければ preferences の `base_dir`。
  job-id は `JobId::parse` で検証し、不正なら終了コード 2。旧 `<dataset-root> <input-id>` は削除した。
- レビュー指摘への対応(後述): dataset tab が manifest の job id ではなく要求された job id を開くように修正
  (B1)。dataset 未作成の AC5 を検証するテストを追加(B5)。`cols > span` の例を検証するテストを追加(A2)。
  設計 §4 の「job-id のみ不可」の記述を、実際の挙動(1 個の引数は base-dir)に合わせて修正(B3)。

## Related Experiments

### Adopted

- なし — `docs/impl/exp/` に本 slice に対応する実験ログは
  存在しない(既存ログは CI 系 issue-139〜153 のみ)。
  `metadata.experiments` も空。

### Rejected

- なし(同上)。

## Validation

### 受入対応表

| AC | 内容 | 証拠 | 結果 |
| --- | --- | --- | --- |
| AC1 | root が Tabs `desktop`、tab 順 `tab-jobs`・`tab-run`・`tab-dataset`、mount 可能 | `tests/desktop_screen.rs::desktop_mounts_jobs_run_and_dataset_tabs_for_the_selected_job` | 達成 |
| AC2 | Jobs tab は base_dir の job を 1 行 1 job の Table で示す | 同上(`jobs` の行 = job id)、`jobs_tab_lists_jobs_while_run_and_dataset_report_no_selection` | 達成 |
| AC3 | Run tab は stage 状態・失敗理由・所要時間を manifest から示す | 同上(`completed 3/3, failed 0`、stage 表 3 行 completed) | 達成 |
| AC4 | Dataset tab は Workspace(Source・Spectrogram・Analysis)を示し、Spectrogram が列を持つ | 同上(`tab-dataset` → `workspace` → `tab-spectrogram` の Spectrogram 列 > 0) | 達成 |
| AC5 | base_dir 未設定・job 未選択・job 不在・dataset 未作成で該当 tab だけ理由 Label | `without_base_dir_every_tab_reports_the_missing_base`、`jobs_tab_lists_…no_selection`、`a_job_outside_the_base_dir_fails_only_its_tabs`、`job_without_dataset_stage_fails_only_the_dataset_tab` | 達成 |
| AC6 | 列が `Tile::columns` と一致し、空 tile は空区間・count 0 | `spectrogram.rs` の `columns_are_the_tile_columns_frames_count_and_magnitudes`、`empty_tile_keeps_empty_column_frames_and_zero_count`、`more_columns_than_frames_share_frames_like_tile_columns` ほか既存 gap テスト | 達成 |
| AC7 | GpuiObserver の drawn が desktop の active tab(Jobs)の Table を Fake 期待値と一致して描く | `gpui_parity.rs::drawn_desktop_jobs_tab_matches_fake_expectations`(描画 key は `[jobs]`) | 達成 |
| AC8 | lock・rev・vendor が全 20 package で `d3be1ad9` に一致 | `check-ruviform-lock`・`check-ruviform-rev`・`sync-ui-deps`・`check-ui` が通過 | 達成 |
| AC9 | GPUI 型は `bin/al-desktop.rs` と gpui-test の parity test の外に出ない | `view/desktop.rs` は `al_service`・`ruviform_ui::view` のみ import。`check-cli-headless` 通過 | 達成 |
| AC10 | 既存数値 oracle・既存テストが非回帰 | `mise run test`(714 件 pass、失敗 0)・`test-ui-gpui` 通過 | 達成 |

Job 選択の対話操作と Run の起動・再開・取消は spec の Non-goals(未達)として扱う(後述)。

### 検証結果(コード HEAD `adbd5ed`、working tree はクリーン)

| task | 結果 |
| --- | --- |
| `mise run fmt` | 通過(差分なし) |
| `mise run check` | 通過 |
| `mise run lint` | 通過(biome check・`cargo clippy --workspace --all-targets -D warnings`) |
| `mise run test` | 通過(全 test binary の合計 714 件 pass、失敗 0) |
| `mise run build` | 通過 |
| `mise run check-ui-gpui` / `lint-ui-gpui` | 通過 |
| `mise run test-ui-gpui` | 通過(`desktop_screen` 6 件、`spectrogram` 9 件、`gpui_parity` 9 件を含む) |
| `mise run check-cli-headless` | 通過 |
| `mise run check-ruviform-lock` | 通過(`d3be1ad9`、20 lockfile source、単一 SHA) |
| `mise run check-ruviform-rev` | 通過(lockfile-primary、REV match) |
| `mise run sync-ui-deps` | 通過 |
| `mise run check-ui` | 通過(`tsc --noEmit`) |
| `mise run test-ui` | 通過(112 件 pass、失敗 0) |

- 環境の補足: worktree に `ui/node_modules` が無く、初回の `check-ui`/`test-ui` は失敗した
  (`tsc`・`happy-dom` が無い)。`mise run setup-ui`(`npm ci --prefix ui`、lockfile からの復元。依存の変更なし)
  の後に再実行して通過した。コードの失敗ではない。

### NotRun / Unsupported

- `mise run check-ui-webview` / `lint-ui-webview` / `test-ui-webview`: 本 slice の変更は webview feature に
  触れないため未実行。
- CI profile(`check-ci`・`lint-ci`・`test-ci`・`build-ci`・`verify-ci`): 未実行(ローカルの dev profile で代替)。
- 実 OS window での Jobs・Run・Dataset の表示確認: 未実行(AL-G5 の範囲。CI は compile・lint・headless のみ)。
- `route_graph.js` による文書 chain の検証: 既存の文書の `broken-relation` 診断で blocked になるため
  実行できなかった(本 slice と無関係な既存文書の診断)。文書は前 slice の chain の形式に合わせた。

### 独立レビュー(review-protocol)の結果

- 独立した read-only サブエージェント 2 件を並列に起動した。Reviewer A(協調)は受入基準と文書の整合を、
  Reviewer B(敵対的)は panic・mount 失敗・意味論の回帰・境界を検査した。両者とも Issue Candidate のみを出し、
  編集はしていない。
- Validator と Judge の役割は coordinator(親)が担った。候補は該当コードと該当テストを読んで確認した。
  独立した validator のサブエージェントは起動していない。
- 判定: **ACCEPT WITH NON-BLOCKING NOTES**(blocker なし)。

| ID | 内容 | 重大度 | 処置 |
| --- | --- | --- | --- |
| B1 | dataset tab が manifest の job id を開いていた(要求された id ではなく)。job ディレクトリの manifest が書き換えられていると別 job の dataset を表示する | P2 | **修正**(`dataset_panel` が要求された `JobId` で `open_dataset`)。`dataset_tab_opens_the_requested_job_not_the_id_in_its_manifest` で検証 |
| B5 / AC5 | dataset 未作成の AC5 の検証なし | P3 | **テスト追加**(`job_without_dataset_stage_fails_only_the_dataset_tab`) |
| A2 | test-spec の `cols > span` の例にテストが無い | P3 | **テスト追加**(`more_columns_than_frames_share_frames_like_tile_columns`) |
| A3 | 上流 Issue の番号が設計・タスクに無い | P3 | **対応**: ruviform#448 を起票し、#443 にコメント追記。設計 §8 とタスク T6 を更新 |
| B3 | 1 個の引数は base-dir として解釈される。設計の記述が不正確だった | P3 | 設計 §4 を実際の挙動に合わせて修正。誤指定の検出は残課題 |
| A1 | Jobs tab は job が 4096 件を超えると `MAX_TABLE_ROWS` の拒否で tab 全体が理由 Label になる | P3 | **残課題**(`JobService::list` に bounded query が無い)。本 slice では対応しない |
| B2 | 上流 `Tile::columns` は `frames.end < frames.start` で減算 underflow する。旧複製は `saturating_sub` だった。現在の上流 producer(`Spectrogram::tile`)は `end ≥ start` を保証するため到達しない | P3 | **残課題**。上流契約の注記、または `spectrogram_screen` での検査を別途検討 |
| B4 | Dataset tab は先頭 input を読めないと tab 全体が失敗する。他の input が dataset にあっても表示しない | P3 | **残課題**(設計 §5 の決定。input 選択の UI は対象外) |

## Risks

- Jobs tab は `JobService::list` に bounded query が無く、job が
  4096 件を超えると `MAX_TABLE_ROWS` の拒否で tab 全体が理由
  Label になる(A1 残課題)。
- 上流 `Tile::columns` は `frames.end < frames.start` で減算
  underflow する。旧複製は `saturating_sub` だったが、現行の
  上流 producer(`Spectrogram::tile`)は `end ≥ start` を保証
  するため到達しない — 上流契約の注記が無い点は残課題(B2)。
- Dataset tab は manifest の先頭 input を読むだけで、先頭
  input を読めないと tab 全体が失敗する(B4、設計 §5 の決定)。
- al-desktop の引数仕様を `<dataset-root> <input-id>` から
  `[<base-dir> [<job-id>]]` へ破壊的に変更した(pre-release の
  ため互換層なし。1 個の引数は base-dir として解釈され、
  誤指定の検出は残課題)。

## Follow-ups

### 判断と未達

- **Job 選択(部分達成)**: 選択は起動引数で行う。`ViewSpec` の `Table` は読み取り専用で、行選択の intent が
  無い。対話選択は ruviform#448(本 slice で起票)の完了後に行う。
- **Run の起動・再開・取消(未達)**: `ViewSpec` に command 起動 component が無い(ruviform#448)。
  Run tab は永続 manifest を読むだけで、実行中の進捗は表示しない。
- **CLI の破壊的変更**: `al-desktop <dataset-root> <input-id>` を廃止し、`[<base-dir> [<job-id>]]` にした。
  pre-release のため互換層は作らない。PR 本文に明記する。
- **Dataset の入力**: manifest の先頭 input を表示する。role による選択は採用しない(設計 §5)。
- **merge 後 HEAD**: 作業ブランチの base は `origin/main`(`6516091`)と同一で、merge commit は生じない。
  最終検証は本 IR の直前のコード HEAD `adbd5ed` で実施した。PR 作成後に head SHA を報告する。

### 逸脱・残課題

- **Job 選択の対話操作・Run の起動・再開・取消**: 未達。ruviform#448 に依存(本文の「判断と未達」)。
- **Jobs の件数上限(A1)、先頭 input 固定(B4)、誤った単一引数の検出(B3)**: 上記の P3 残課題。
- **RF443**: 複製は除去済み。ruviform#443 は部分達成のまま open。消費状況をコメントで追記した。
- **Spectrogram の読み出し上限**: 先頭 4096 STFT frame・128×512 セル(従来どおり)。
- **ruviform 側の参照**: ruviform#446 は `d3be1ad9`(#447)で修正済み。
- **文書 chain の router 検証**: 既存文書の `broken-relation` により blocked(本 slice と無関係)。
