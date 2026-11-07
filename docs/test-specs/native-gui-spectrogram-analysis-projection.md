---
id: "TSPEC-Kv7Rn3Ac9WxS5pZdM2hEgH"
type: "test-spec"
status: "draft"
title: "Native GUI Spectrogram・Job/Run/Analysis 射影と drawn parity の検証仕様"
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
    - "../specs/native-gui-spectrogram-analysis-projection.md"
    - "../designs/native-gui-spectrogram-analysis-projection.md"
    - "https://github.com/scarecrow173/ruviform/issues/442"
    - "https://github.com/scarecrow173/ruviform/issues/443"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "../specs/native-gui-spectrogram-analysis-projection.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native GUI Spectrogram・Job/Run/Analysis 射影と drawn parity の検証仕様

## Feature: Spectrogram の 1 対 1 写像

### Rule: 列の値と gap は Tile の値をそのまま保つ

- Example: `tests/spectrogram.rs::columns_map_one_to_one_to_tile_values_and_frame_spans`
- Example: `tests/spectrogram.rs::all_nan_columns_keep_count_zero_and_nan_cells`

### Rule: 列の frame 区間は上流 `Spectrogram::tile` の等分割と一致する

- Example: `tests/spectrogram.rs::columns_split_span_like_ruviform_tile_when_not_divisible`
- Example: `tests/spectrogram.rs::offset_request_shifts_column_frames_to_absolute_frames`

### Rule: 形状が合わない Tile は拒否する

- Example: `tests/spectrogram.rs::rejects_tile_whose_values_do_not_match_its_shape`

### Rule: Fake 層で mount でき、spectrogram key が観測できる

- Example: `tests/spectrogram.rs::mounts_in_fake_host_and_observes_spectrogram_key`

## Feature: Job/Run/Analysis の Table・Label 射影

### Rule: stage 表は失敗理由と実行時間を欠落させない

- Example: `tests/analysis_screen.rs::run_status_label_and_stage_table_keep_failure_reasons`

### Rule: job 一覧は一列の Table(List)

- Example: `tests/analysis_screen.rs::job_list_is_a_one_column_table_of_job_ids`

### Rule: events・alignment は typed 値を表示し、非有限の数値は欠損セルにする

- Example: `tests/analysis_screen.rs::analysis_screen_projects_events_and_alignments_as_tables`

### Rule: 行数が Table の上限を超える場合は拒否する

- Example: `tests/analysis_screen.rs::analysis_rejects_more_rows_than_a_table_can_carry`

### Rule: Run と Analysis は Fake 層で mount できる

- Example: `tests/analysis_screen.rs::run_and_analysis_screens_mount_in_the_fake_host`

## Feature: dataset からの Tabs 画面(al-desktop の mount 対象)

### Rule: 実 dataset から Source・Spectrogram・Analysis の 3 tab を得て、各 tab が mount できる

- Example: `tests/view.rs::real_dataset_produces_view_models`(tab 切替ごとに mount し、
  `waveform`・`spectrogram`・`alignments` が観測できる)

## Feature: GPUI 描画 plan の parity

### Rule: Meter bar・TimeAxis mark・Waveform bin は mount した spec の期待値と一致する

- Example: `tests/gpui_parity.rs::drawn_plans_match_fake_expectations_for_meter_ruler_and_waveform`

### Rule: gap の Waveform bin は描かれない

- Example: `tests/gpui_parity.rs::drawn_waveform_omits_the_nan_gap_range`

### Rule: Spectrogram・Label・Table の描画 plan は NotRun(観測手段が無い)

- 検証しない。理由は ruviform#442 の解消待ちで、PR と IR に NotRun として記録する。

## Feature: 既存の数値 oracle の非回帰

### Rule: 既存の Source・Waveform・Meter の射影は変更前と同じ値を保つ

- Example: `tests/screen.rs` の既存テスト(bit 同一・gap・Meter の peak)が pass する
- Example: `tests/gpui_parity.rs` の既存テスト(mount key・projection・Seek intent)が pass する
