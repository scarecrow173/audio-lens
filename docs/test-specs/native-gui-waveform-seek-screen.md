---
id: "TSPEC-7JyfOx5Dj1B55uTKms7L2h"
type: "test-spec"
status: "draft"
title: "Native GUI Waveform/Seek 画面の ViewSpec 射影 検証仕様"
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
    - "../specs/native-gui-waveform-seek-screen.md"
    - "../designs/native-gui-waveform-seek-screen.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../plans/native-gui-waveform-seek-screen.md"
    - "../tasks/native-gui-waveform-seek-screen.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "../specs/native-gui-waveform-seek-screen.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native GUI Waveform/Seek 画面の ViewSpec 射影 検証仕様

対象: `crates/al-gui/tests/screen.rs`(Fake 層 = `ViewHost` headless
projection)。実行: `mise run test` または
`cargo test -p al-gui --test screen`。

## Feature: 数値の 1 対 1 写像

### Rule: 射影 bins は解決済み `ViewBin` と bit 同一

- Example: `projected_bins_are_bit_exact_to_resolved_view_bins`
  - 0.0〜0.1 秒・16 bins の射影と `WaveformView::window` を比較し、
    `start`/`end`/`count`/`min`/`max`/`rms`(bit 同一)が一致する。

### Rule: gap は NaN・count 0 のまま

- Example: `gap_bins_stay_nan_with_zero_count`
  - `[1200, 1440)` frame の NaN gap が `count == 0` かつ `min/max/rms`
    NaN の bin として現れ、射影でも変換されない。

## Feature: 範囲と操作

### Rule: viewport と Seek 範囲は source frames に由来する

- Example: `viewport_and_seek_range_come_from_source_frames`
  - `viewport == frame_range(...)`、`frames`/`sample_rate` が metadata と
    一致、Seek の `max == frames / sample_rate`、target が `transport.seek`。

### Rule: Seek の Commit は範囲内の transport intent を発行する

- Example: `seek_gesture_commits_transport_intent_within_source_range`
  - `Begin → Preview(0.05) → Commit` で `CommandIntent::Commit`
    (target `transport.seek`)が出て、committed 値が 0.05 になる。

## Feature: Theme 中立

### Rule: light/dark の解決で semantic event stream が変わらない

- Example: `product_theme_modes_do_not_change_semantic_event_stream`
  - `theme::MODES` の各 mode で ResolvedTheme が解決でき
    (`color.waveform` あり)、同一 gesture の event stream が一致する。

## Feature: 境界

### Rule: 範囲外 channel は拒否する

- Example: `channel_out_of_range_is_rejected`

## 未実施(NotRun)

- GPUI driver 上の projection・intent 観測(上流 #421/#426 待ち。
  観測 seam が crate 外に無い)。
- 実 OS window の描画・Seek 実入力・IME・a11y(AL-G4/G5)。
- 性能非回帰の実機計測、Golden 画像による視覚判定。
