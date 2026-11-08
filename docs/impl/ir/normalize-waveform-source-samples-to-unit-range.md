---
id: "IMPL-034TZKE6XdzDisCy6N9wGD"
type: "impl"
status: "completed"
title: "normalize waveform source samples to unit range"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "code"
        path: "crates/al-service/src/source.rs"
      - type: "code"
        path: "crates/al-service/tests/service.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/canvas-paint-empty.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/canvas-paint-empty.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/canvas-paint-empty.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# normalize waveform source samples to unit range

## Summary

`DatasetWaveform` が dataset WAV を生 PCM スケール
(例: pcm_s16 → ±32768)のまま `ruviform-viz` の
`WaveformSource` port に供給していた問題を修正し、
`audio.sample_format` に対応する `format_scale` で
[-1, 1] に正規化して返すようにした。

## Context

`docs/tasks/canvas-paint-empty.md`(Issue #179)に従う。
ruviform pin `863a7a9d`(RF#453 canvas サイズ修正 +
RF#461 `paint_layer` batch 化)で canvas が実サイズを得た結果、
過大な振幅値による `viz.waveform_rms` の全面塗り潰しが
実窓で顕在化した。旧 pin `a5b1db1` では canvas 0 サイズが
この描画を「空 paint」として隠していた。

## Implementation

### Changes

- `crates/al-service/src/source.rs`
  - `DatasetWaveform` に `scale: f64` フィールドを追加。
  - `DatasetWaveform::open` で `<input_id>.metadata.json` の
    `audio.sample_format` を読み、`format_scale` で scale を
    決定(不正値は open 時にエラー)。
  - `read` でデコードした各サンプルを `scale` で除算し、
    `PcmBlock` として [-1, 1] の単位振幅で返す。
- `crates/al-service/tests/service.rs`
  - 回帰テスト
    `waveform_source_yields_samples_normalized_to_unit_range`
    を追加。pcm_s16 入力で最大絶対サンプルが
    `200 / 32768` になることを検証(修正前は 200)。

### Decisions During Implementation

- 正規化は `DatasetWaveform`(source 境界)で一箇所だけ行い、
  renderer 側・projection 側に補償を分散させない。
  `waveform_view` op 等の他 consumer も同一 port 経由で
  正規化値を受け取る。
- Float32/Float64/Other の scale は `format_scale` が 1.0 を
  返すため追加分岐は不要。

## Related Experiments

### Adopted

<!-- 実窓 pixel 解析による切り分け(投影非空 → renderer 契約違反)を
     exp にはせず本記録の Context に集約した。 -->

### Rejected

<!-- なし -->

## Validation

- `cargo test -p al-service waveform_source_yields_samples_normalized_to_unit_range`(補助証拠): Passed
- 実窓検証(Windows 11, al-desktop debug build, pin `863a7a9d`):
  `.devin/evidence/al179-canvas-paint/` の `verdict.md` と
  `41-dataset-click-after.png` / `42-spectro-click-after.png`。
  Waveform envelope・Meter bars・TimeAxis tick 9 本・
  Spectrogram 65,536 cell の非空 paint を pixel 解析で確認。
- Spectrogram cell の intensity(alpha)が実窓 pixel に反映されない
  別件の異常を観測し、ruviform Issue #463 を起票
  (AL projection は正常と確認済み)。

## Risks

- dataset WAV 以外の `WaveformSource` 実装が同じ契約違反を
  持つ場合は個別に確認が必要(現状 `DatasetWaveform` のみ)。

## Follow-ups

- ruviform#463: Spectrogram cell intensity(alpha)が実窓で
  反映されない件の ruviform 側調査。
