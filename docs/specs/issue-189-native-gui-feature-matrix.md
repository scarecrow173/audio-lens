---
id: "SPEC-034bAl189GuiMtx001"
type: "spec"
status: "proposed"
title: "Native `al-desktop` 機能 × 検証 lane 対応表（Issue #189 AC2）"
created: "2026-11-24"
updated: "2026-11-24"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/189"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-189-native-gui-feature-matrix.md"
    modified:
      - type: "doc"
        path: "docs/evidence/gui-verification.md"
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
    - "docs/evidence/gui-verification.md"
    - "docs/discovery/issue-127-legacy-webview-inventory.md"
    - "docs/specs/issue-219-gui-real-e2e.md"
    - "docs/specs/issue-127-dataset-scenario.md"
    - "docs/specs/issue-127-rf628-consumer-oracle.md"
    - "docs/specs/ci-os-l1-gpui-desktop.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native `al-desktop` 機能 × 検証 lane 対応表（Issue #189 AC2）

## 目的

Issue #189 AC2 の受入判定に使う、Native `al-desktop`（GPUI 製品 GUI）の
機能 × 検証 lane の単一対応表を固定する。行は画面・操作の機能単位、列は
検証 lane である。各セルは lane 別の個別判定（verdict）と、その判定の
根拠となる test・lane・evidence への参照を持つ。

本書は新規の検証実走を行わない。既存の lane 検証記録
（`docs/evidence/gui-verification.md`）と slice 単位の検証仕様
（`docs/test-specs/native-gui-*.md`・`docs/test-specs/issue-219-gui-real-e2e.md`・
`docs/test-specs/issue-127-dataset-scenario.md`）を、一つの行列へ
集約し直したものである。

## 判定の語彙

`POLICIES.md` §3 に従い、要求状態と実行結果を別軸で記録する。

- 要求状態: `Required`（現行必須）、`Deferred`（利用条件成立まで保留）、
  `NotApplicable`（対象機能では不要）。
- 実行結果: `Pass`、`Fail`、`NotRun`、`Unsupported`（根拠を伴う技術的
  非対応）。

`Required` なのに実行環境や実走記録がないものは `NotRun` であり、
`Pass` ではない。Headless・Fake の成功を実 OS window の `Pass` へ昇格
しない（`POLICIES.md` §2.1）。

## 検証 lane の定義

| lane | 実行形態 | 実行入口 | 要求状態 |
| --- | --- | --- | --- |
| Headless Fake | `ViewHost` / ViewSpec の model・screen・service・receipt test（`crates/al-gui/tests/`） | `mise run test al-gui`（既定 feature） | Required（OS-L1、3OS） |
| GPUI simulated | `gpui-test` による実 GPUI mount と `GpuiObserver` parity、`scenario_gpui` の同一 scenario 契約 | `mise run check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui` / `build-ui-gpui`（AL#206 で 3OS CI 接続済み・CLOSED） | Required（OS-L1、3OS） |
| 実 OS input | `al-gui-real-scenario`（`gpui-real` feature、test-only bin）の semantic・pointer・keyboard case | `mise run test-ui-gpui-real`（interactive Windows session の opt-in）/ `mise run test-ui-gpui-real-headless`（blocked-probe の fail-closed 検査） | Required（Windows GUI-L2。提供する機能に限る） |
| 実 pixel | 同 bin の `capture-frame-*` case（`VisualGuiTest::capture_frame` の実 window readback）と、実窓の component 別 paint 確認 | 同上の lane と、手動の実窓検証 | Required（同上） |

macOS・Linux の GUI-L2 は `POLICIES.md` §2 の Deferred を維持する。
実 OS input・実 pixel lane の `ScenarioReport` は `environment_kind` が
`real-window` の場合だけ実 window の証拠と名乗れる。blocked/headless
記録は `deterministic-backend` と `environment_blocked` を名乗り、
実機を主張しない（`docs/test-specs/issue-219-gui-real-e2e.md` の
fail-closed 契約）。

## 機能 × lane 対応表

| 機能 | Headless Fake | GPUI simulated | 実 OS input | 実 pixel |
| --- | --- | --- | --- | --- |
| 型・モデル（driver-neutral ViewSpec 射影層） | Pass | Pass | Pass | Pass |
| Job 選択 | Pass | Pass | Pass | Pass |
| Run/Resume（Cancel 含む） | Pass | Pass（Resume）/ NotRun（Cancel） | Pass（Resume）/ NotRun（Cancel） | NotRun |
| Dataset（workspace tabs 含む） | Pass | Pass | NotRun | NotRun |
| Source | Pass | Pass | Pass | Pass |
| Analysis | Pass | Pass | NotRun | NotRun |
| Waveform | Pass | Pass | NotRun | Pass |
| Spectrogram | Pass | Pass | NotRun | Pass |
| Meter | Pass | Pass | NotRun | Pass |
| TimeAxis | Pass | Pass | NotRun | Pass |
| Seek | Pass | Pass | Pass | Pass |

viz 系（Waveform・Spectrogram・Meter・TimeAxis）は入力を受けない
表示専用 component のため、実 OS input lane では対象とする入力 case が
存在せず `NotRun` と記録する。これらの実表示の判定は実 pixel lane が
負う。実 pixel の `capture-frame-*` case は frame 全体の
`pixel_count > 0` を oracle とし、component 別の画素照合は行わない。
component 単位の非空 paint は実窓検証（pin `863a7a9d`）が別途記録した
ものだけを `Pass` の根拠にする。

## セル別の証拠

### Headless Fake（`crates/al-gui/tests/`、既定 feature）

- 型・モデル: `tests/view.rs`（`real_dataset_produces_view_models`、
  `stream_time_view_bounded_ticks`、`stream_time_view_from_metadata_array`、
  `time_map_offset_rate_mapping`、`real_avi_metadata_produces_stream_time_views`、
  typed 行写像の `event_row_maps_typed_enums`・
  `alignment_row_preserves_typed_contracts`・`feature_table_typed_records`）、
  `tests/scenario.rs::fake_target_resolution_covers_window_view_component_and_pins`。
- Job 選択: `tests/desktop_screen.rs::jobs_table_selects_jobs_and_run_tab_offers_resume_and_cancel`、
  `tests/desktop.rs::select_intent_adopts_the_job_and_marks_the_projection_dirty`・
  `rejected_selection_keeps_the_previous_job`、
  `tests/service_ops.rs::command_select_maps_row_to_job_and_replies_applied`、
  `tests/scenario.rs::fake_desktop_jobs_scenario_passes_with_honest_report`。
- Run/Resume・Cancel: `tests/service_ops.rs::command_resume_starts_the_job_and_replies_accepted`・
  `job_cancel_op_is_routed`、
  `tests/desktop.rs::resume_is_acked_then_receives_an_applied_receipt_after_the_job_finishes`・
  `resume_of_a_missing_job_ends_with_a_rejected_receipt`・
  `cancel_intent_cancels_the_window_owned_job_and_replies_accepted`・
  `cancel_without_a_window_job_replies_rejected`、
  `tests/receipt.rs::resume_receipt_moves_the_run_button_from_pending_to_applied`。
- Dataset: `tests/desktop_screen.rs`（Dataset tab mount・
  `dataset_tab_opens_the_requested_job_not_the_id_in_its_manifest`・
  fallback 系）、
  `tests/scenario.rs::fake_dataset_workspace_scenario_switches_tabs_with_al_oracle`、
  workspace tab-select は `tests/service_ops.rs`・`tests/desktop.rs`・
  `tests/receipt.rs` の `workspace` 系 test。
- Source: `tests/screen.rs`（`projected_bins_are_bit_exact_to_resolved_view_bins`・
  `gap_bins_stay_nan_with_zero_count`・`viewport_and_seek_range_come_from_source_frames`・
  `zero_width_window_keeps_bins_inside_viewport`・`bins_beyond_limit_are_rejected_before_reading`・
  `channel_out_of_range_is_rejected`）、
  `tests/scenario.rs::fake_source_seek_scenario_commits_transport_seek`。
- Analysis: `tests/analysis_screen.rs`（`run_status_label_and_stage_table_keep_failure_reasons`・
  `job_list_is_a_one_column_table_of_job_ids`・
  `analysis_screen_projects_events_and_alignments_as_tables`・
  `analysis_rejects_more_rows_than_a_table_can_carry`）、
  `tests/view.rs::real_expensive_dataset_produces_event_rows`。
- Waveform: `tests/screen.rs` の bin 射影系（上記 Source と同一 file）。
- Spectrogram: `tests/spectrogram.rs`（`columns_map_one_to_one_to_tile_values_and_frame_spans`・
  `all_nan_columns_keep_count_zero_and_nan_cells`・
  `columns_split_span_like_ruviform_tile_when_not_divisible`・
  `offset_request_shifts_column_frames_to_absolute_frames`・
  `rejects_tile_whose_values_do_not_match_its_shape`・
  `mounts_in_fake_host_and_observes_spectrogram_key`）。
- Meter: `tests/screen.rs::meter_bins_mirror_window_bins_with_peak_from_bucket_extremes`・
  `meter_gap_bins_stay_nan_with_zero_count`。
- TimeAxis: `tests/screen.rs::ruler_spans_the_source_viewport_with_bounded_ticks`・
  `ruler_ticks_stay_inside_the_viewport`、`src/view/screen.rs` の
  `long_span_gets_coarse_ticks_within_the_bound`・`span_beyond_the_largest_step_is_rejected`。
- Seek: `tests/screen.rs::seek_gesture_commits_transport_intent_within_source_range`、
  `tests/scenario.rs::fake_source_seek_scenario_commits_transport_seek`。

### GPUI simulated（`gpui-test` lane）

- 型・モデル: `tests/gpui_parity.rs::mounted_components_match_between_fake_and_gpui`・
  `projections_match_before_any_gesture`（mount key と projection の
  Fake/GPUI bit 一致）、
  `tests/scenario_gpui.rs::gpui_target_resolution_and_stale_pins_cover_al_view`。
- Job 選択・Run/Resume: `tests/gpui_parity.rs::jobs_selection_and_resume_button_match_between_fake_and_gpui`
  （select・resume invoke の intent JSON と `selected` の Fake/GPUI 一致）、
  `tests/scenario_gpui.rs::gpui_desktop_jobs_scenario_passes_with_declared_capabilities`・
  `gpui_and_fake_run_the_same_scenarios_with_equal_evidence`。
  Cancel の intent は GPUI simulated では未検証（`NotRun`）。
- Dataset: `tests/scenario_gpui.rs::gpui_dataset_workspace_scenario_switches_tabs_with_al_oracle`、
  `tests/gpui_parity.rs::workspace_tab_selection_intent_and_selected_tab_match_between_fake_and_gpui`
  （選択後に spectrogram が paint 対象になることまで確認）、
  `tab_selection_intent_and_selected_tab_match_between_fake_and_gpui`。
- Source・Seek: `tests/gpui_parity.rs::seek_gesture_intents_and_projections_match`。
- Waveform・Meter・TimeAxis: `tests/gpui_parity.rs::drawn_plans_match_fake_expectations_for_meter_ruler_and_waveform`・
  `drawn_waveform_omits_the_nan_gap_range`。
- Spectrogram: `tests/gpui_parity.rs::drawn_spectrogram_and_range_label_match_fake_expectations`
  （gap 列・非有限 cell の除外含む）。
- Analysis: `tests/gpui_parity.rs::drawn_run_status_and_stage_table_match_fake_expectations`・
  `drawn_job_list_matches_fake_expectations`・
  `drawn_desktop_jobs_tab_matches_fake_expectations`（Table・Label の
  drawn plan 照合。欠損 cell が `None` で残ることも検証）。Analysis tab
  本体は `gpui_dataset_workspace_scenario` が events/alignments の mount
  と AL 独立 oracle の spec cell を照合する。

### 実 OS input（`al-gui-real-scenario` catalog、`test-ui-gpui-real`）

証拠は `evidence/al219/gui-real-scenario.jsonl`（interactive Windows
session、sha `826dbf1` の時点の 7-case 実走、全 case `pass`、
`environment_kind=real-window`）と `evidence/al219/README.md`。
pointer は win32-SendInput、capture は win32-printwindow backend。
現行 catalog（`crates/al-gui/src/bin/al-gui-real-scenario.rs` の `CASES`、
`tests/scenario_real.rs::case_catalog_ids_are_stable` で pin）は 9 case。

- Job 選択・Run/Resume: `al.desktop.jobs`/`job-select-resume`（semantic。
  job 行の SelectRow→Run tab 選択→Resume Button の Invoke）。
  Cancel を起動する case は catalog に無く `NotRun`。
- Source・Seek: `al.source.seek`/`seek-commit`（semantic の
  Begin→Preview→Commit）、`al.real-os.pointer`/`seek-pointer-click`、
  `al.real-os.keyboard`/`seek-key-end`、
  `al.real-os.input`/`os-input-preflight`。
- Dataset・Analysis: `al.dataset.workspace`/`workspace-tabs` は catalog
  拡張（pin `b6d0cc6a`）後の追加 case で、interactive 実走の記録がない
  ため `NotRun`。blocked-probe の fail-closed 形状（9 件全件
  `not_run` + `environment_blocked`）は
  `mise run test-ui-gpui-real-headless` で検証済み。
- `al.real-os.window`/`close-veto-approve` も同じく interactive 実走が
  未記録で `NotRun`（機能行とは独立の window 寿命 case）。

### 実 pixel

- 画面レベル: `al.real-os.visual`/`capture-frame-desktop` と
  `capture-frame-source` が実 OS window の frame readback を取得し、
  `pixel_count > 0` を確認（同じく `evidence/al219` の 7-case 実走）。
- component 別 paint: `docs/evidence/gui-verification.md`「実窓検証
  （AL#179/#180/RF#463）」の記録（pin `863a7a9d`）で、Waveform
  envelope・Meter bars・TimeAxis tick 9 本・Spectrogram 65,536 cell の
  非空 paint を確認。`tests/spectrogram_tab_paint.rs` は本番解像度の
  paint を 0.13s で pass。
- Run・Dataset・Analysis の各 tab は実 window の画素記録がなく
  `NotRun`。

## 既知の NotRun・制約

- 新規 run の起動（`al.job.run` の UI 接続）: `al-desktop` に入力 file
  を選ぶ UI がなく未接続。`docs/specs/native-gui-interactive-job-run.md`
  の非スコープ節で明示済み。`bin/al-desktop.rs` の `update_view` による
  再射影の実行系にも自動 test がない。
- `Cancelled` された resume job の終端 receipt: ruviform の receipt
  分類（`Cancelled` → `rejected`）に依存しており、AL 側の直接検証は
  未実施（`docs/test-specs/native-gui-job-receipt-consumption.md`）。
- Dataset 内 Source・Spectrogram・Analysis tab への実 pointer 到達:
  workspace Tabs の非採用経路は P2 の既知残課題で、実 OS window での
  pointer 巡回は未検証（受入欠落ではなく非スコープ）。`select_tab`
  seam・command intent 経路は Fake・GPUI simulated で検証済み。
- `al-gui-real-scenario` の 9-case catalog うち新規 2 case
  （`al.dataset.workspace`/`workspace-tabs`・`al.real-os.window`/
  `close-veto-approve`）の interactive real-window 実走: 現行 SHA で
  `NotRun`。`evidence/al219/gui-real-scenario.jsonl` は 7-case 時点
  （sha `826dbf1`）の記録であり、2 case の追加後の実走証拠はない。
- `docs/evidence/gui-verification.md` の「drawn plan に
  Spectrogram・Label・Table の variant が無く drawn parity の対象外
  （RF#442）」という記録は、現行コードに対して古い。
  `scarecrow173/ruviform#442` は CLOSED であり、`DrawnPlan` に
  `Spectrogram`・`Label`・`Table` variant が実装済みで、
  `tests/gpui_parity.rs` の drawn 照合が各 variant を検証している。
  本表は現行コード側の事実を採用し、この項目は `NotRun` ではなく解消
  済みの制約として扱う。

## f64・NaN・Inf・gap・PTS・長尺・Job/CLI の保護（#127 所有の独立 Oracle）

これらの数値・経路保証は GUI の lane ではなく、`scarecrow173/audio-lens#127`
が所有する独立 Oracle が負う。AL 側で再実装・再検証は行わず、実行可能な
test・evidence への参照だけを記す（`POLICIES.md` §6 の「AudioLens 独自
Oracle」の運用）。

- f64 fidelity・長尺非回帰・gap→NaN/Validity/Provenance:
  `crates/al-analysis/tests/rf628_oracle.rs` の 7 test
  （`rf628_fixture_matches_independent_oracle`、f64 fidelity の
  `constant`・`periodic`・`noise`・`stereo_channels`、
  `rf628_long_scenario_non_regression`（約 85s の長尺）、
  `rf628_gap_nan_validity`）。Windows debug で 7/7 `Pass` の記録は
  `docs/evidence/gui-verification.md`「consumer oracle（AL#127/RF#628）」
  節と `evidence/al127/` の各 `*-verification.md`。
- CLI Golden: `crates/al-cli/tests/cli.rs::full_pipeline_reaches_export`
  が `export/analysis/golden/snapshot.json` まで到達することを検証。
  併せて `crates/al-analysis/tests/golden.rs`・
  `crates/al-locate/tests/golden.rs`・`crates/al-media/tests/golden.rs`・
  `crates/al-dataset/src/export/golden.rs`。
- Job/Run の headless 経路: `crates/al-service/tests/service.rs`（run →
  inspect → list、cancel で Pending 保持 → resume 完走、corrupt
  manifest 系）、`crates/al-job/tests/hooks.rs`（stage 境界イベント・
  cancel・resume の Reused）、`crates/al-gui/tests/service_ops.rs` の
  `al.job.*` op 系。
- PTS/AV timing: `crates/al-media/tests/mp4.rs`・`aac.rs`・`ingest.rs`、
  `crates/al-gui/tests/view.rs::real_avi_metadata_produces_stream_time_views`、
  観測規範は `docs/test-specs/av-timing-observation.md`。
- headless 境界の guard: `mise run check-cli-headless`（GUI/GPU 依存の
  headless crate への混入を denylist 照合で検査。Issue #189 本文の
  `check-headless-deps` は stale 名で、現行 task 名は
  `check-cli-headless`）。

`scarecrow173/audio-lens#166`（CLOSED）の slice がこれらの lane 実装の
土台だが、#166 の Close だけをもって全能力を `Pass` としない。各セルの
判定は上表の lane 別証拠にだけ依る。

## 参照

- Issue: `scarecrow173/audio-lens#189`（AC2・詳細 B 節）。
- lane 検証記録: `docs/evidence/gui-verification.md`。
- 実 OS lane 契約: `docs/specs/issue-219-gui-real-e2e.md`・
  `docs/test-specs/issue-219-gui-real-e2e.md`・`evidence/al219/`。
- dataset workspace scenario: `docs/specs/issue-127-dataset-scenario.md`・
  `docs/test-specs/issue-127-dataset-scenario.md`。
- consumer oracle 仕様: `docs/specs/issue-127-rf628-consumer-oracle.md`。
- slice 別検証仕様: `docs/test-specs/native-gui-*.md`（第3〜第10 slice）。
- GPUI CI lane 接続: `docs/specs/ci-os-l1-gpui-desktop.md`（AL#206）。
- 旧 WebView 棚卸し（AC1）: `docs/discovery/issue-127-legacy-webview-inventory.md`。
