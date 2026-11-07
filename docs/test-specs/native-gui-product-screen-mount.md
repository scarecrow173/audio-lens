---
id: "TSPEC-O65pjV4Ci4jMZTRqGNRUm2"
type: "test-spec"
status: "draft"
title: "Native GUI Source 画面の ViewSpec 射影・GPUI parity 検証仕様"
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
    - "../specs/native-gui-product-screen-mount.md"
    - "../designs/native-gui-product-screen-mount.md"
    - "https://github.com/scarecrow173/ruviform/issues/429"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "../specs/native-gui-product-screen-mount.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native GUI Source 画面の ViewSpec 射影・GPUI parity 検証仕様

## Feature: 数値の 1 対 1 写像

### Rule: 射影 bins は解決済み `ViewBin` と bit 同一

- Example: `tests/screen.rs::projected_bins_are_bit_exact_to_resolved_view_bins`

### Rule: Meter の peak は bucket の極値の絶対値の最大で、他は写すだけ

- Example: `tests/screen.rs::meter_bins_mirror_window_bins_with_peak_from_bucket_extremes`

### Rule: gap は NaN・count 0 のまま(Waveform と Meter の両方)

- Example: `tests/screen.rs::gap_bins_stay_nan_with_zero_count`
- Example: `tests/screen.rs::meter_gap_bins_stay_nan_with_zero_count`

## Feature: 範囲と操作

### Rule: viewport と Seek 範囲は source frames に由来する

- Example: `tests/screen.rs::viewport_and_seek_range_come_from_source_frames`

### Rule: 空区間でも bins は viewport の内側にある

- Example: `tests/screen.rs::zero_width_window_keeps_bins_inside_viewport`

### Rule: TimeAxis は viewport に対する区間数を上限内に保ち、tick は viewport 内にある

- Example: `tests/screen.rs::ruler_spans_the_source_viewport_with_bounded_ticks`
- Example: `tests/screen.rs::ruler_ticks_stay_inside_the_viewport`
- Example: `src/view/screen.rs::tests::long_span_gets_coarse_ticks_within_the_bound`
- Example: `src/view/screen.rs::tests::span_beyond_the_largest_step_is_rejected`

### Rule: bins 上限を超える要求は読み出し前に拒否する

- Example: `tests/screen.rs::bins_beyond_limit_are_rejected_before_reading`

### Rule: Seek の Commit は範囲内の transport intent を発行する

- Example: `tests/screen.rs::seek_gesture_commits_transport_intent_within_source_range`

## Feature: GPUI parity(同一 fixture を Fake と GPUI で評価)

### Rule: mount 済み component の集合は Fake と GPUI で一致する

- Example: `tests/gpui_parity.rs::mounted_components_match_between_fake_and_gpui`

### Rule: 操作前の各 component の projection は Fake と GPUI で一致する

- Example: `tests/gpui_parity.rs::projections_match_before_any_gesture`

### Rule: Seek gesture の intent と操作後の projection は Fake と GPUI で一致する

- Example: `tests/gpui_parity.rs::seek_gesture_intents_and_projections_match`

## Feature: Theme 中立

### Rule: light/dark の解決で semantic event stream が変わらない

- Example: `tests/screen.rs::product_theme_modes_do_not_change_semantic_event_stream`

## Feature: 境界

### Rule: 範囲外 channel は拒否する

- Example: `tests/screen.rs::channel_out_of_range_is_rejected`

## 未実施(NotRun)

- **Meter/TimeAxis の描画 plan の parity**: `GpuiObserver` が描画 plan を公開していない
  (Ruviform 側の契約ギャップ。scarecrow173/ruviform#436)。
- **al-desktop の実 OS window 起動・操作**: CI と本作業では実行していない。
- **Job/Run/Analysis/Spectrogram の射影**: 現行 ViewSpec に kind が無く対象外。
- **AL-G4 / AL-G5**(Save/Reload・visual regression・性能・Consumer Gate): 対象外。
