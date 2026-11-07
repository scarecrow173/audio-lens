---
id: "TSPEC-zOhqLzVO640w2ox49XQgzs"
type: "test-spec"
status: "draft"
title: "Native GUI drawn parity の Spectrogram・Label・Table と Job/Run 射影の検証仕様 (第5 slice)"
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
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/native-gui-drawn-parity-run-mount.md"
    - "../designs/native-gui-drawn-parity-run-mount.md"
    - "https://github.com/scarecrow173/ruviform/issues/442"
    - "https://github.com/scarecrow173/ruviform/issues/443"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "../specs/native-gui-drawn-parity-run-mount.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native GUI drawn parity の Spectrogram・Label・Table と Job/Run 射影の検証仕様

## Feature: drawn parity の Spectrogram・Label・Table 照合

### Rule: Spectrogram cell は mount した payload の規則から導いた期待値と一致する

- Example: `tests/gpui_parity.rs::drawn_spectrogram_and_range_label_match_fake_expectations`
  - gap 列(`count == 0`)と非有限セルを除いた cell 数・位置・intensity が一致する
  - fixture は期待 cell 数が `rows × cols` より小さい(gap と非有限セルを含む)ことを assert する

### Rule: Label は payload の text を、描画順は spec の順を保つ

- Example: `tests/gpui_parity.rs::drawn_spectrogram_and_range_label_match_fake_expectations`(range Label)
- Example: `tests/gpui_parity.rs::drawn_run_status_and_stage_table_match_fake_expectations`(status Label)
  - 描画 key の順は `[spectrogram-range, spectrogram]` と `[run-status, stages]`

### Rule: Table は header と格子を payload と一致させ、欠損セルの位置を保つ

- Example: `tests/gpui_parity.rs::drawn_run_status_and_stage_table_match_fake_expectations`
  - 欠損セル(`duration_ms` の null・`error` の無い行)が `None` として残る
- Example: `tests/gpui_parity.rs::drawn_job_list_matches_fake_expectations`
  - 一列の job 一覧 Table の header と行が一致する

## Feature: 実 job の Job/Run 射影 e2e

### Rule: 実 job の一覧は 1 行 1 job の Table になる

- Example: `tests/view.rs::real_dataset_produces_view_models`
  - `job_list` の行数が 1、先頭セルが実 job id と一致する

### Rule: 実 manifest の stage は Run 画面の状態テキストと stage 表へ写り、Fake 層に mount できる

- Example: `tests/view.rs::real_dataset_produces_view_models`
  - 状態テキストが `job <id>: completed N/N, failed 0`(N は manifest の stage 数)
  - stage 表の行数が N、全行の status が `completed`
  - `ViewHost` へ mount でき、`stages` key が観測できる

## Feature: 既存契約の非回帰

### Rule: 既存の Meter・TimeAxis・Waveform の drawn parity と数値 oracle は変わらない

- Example: `tests/gpui_parity.rs::drawn_plans_match_fake_expectations_for_meter_ruler_and_waveform`
- Example: `tests/gpui_parity.rs::drawn_waveform_omits_the_nan_gap_range`
- Example: `mise run test` 全件(数値 oracle を含む)
