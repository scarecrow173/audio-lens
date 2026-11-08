---
id: "DESIGN-byFehhoxeAg9rB1zUWZc59"
type: "design"
status: "approved"
title: "canvas paint 空の設計（DatasetWaveform での振幅正規化）"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source: []
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
    - "docs/specs/canvas-paint-empty.md"
  refines:
    - "docs/specs/canvas-paint-empty.md"
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

## 概要

AL#179 の「canvas paint が空/単色」の root cause を 2 段階で切り分けた。

1. **pin `a5b1db1` 時点(AL#166 観測)**: ruviform-ui-gpui が viz canvas に
   0 px の高さを与える RF#453 不具合により、paint 自体が空だった。
   RF#453 は ruviform `769d595` で、RF#461 の `paint_layer` batch 化は
   `863a7a9` で修正済み。
2. **pin `863a7a9d` 時点(本件)**: canvas は正しい bounds を得るが、
   `DatasetWaveform` が `inputs/<id>.wav` の生 PCM 値(pcm_s16 で
   ±32768 スケール)をそのまま返していた。`WaveformView` →
   `source_screen` は bucket の min/max/rms をそのまま
   `WaveformBin`/`MeterBin` に写し、ruviform-ui-gpui は
   `y = top + (1 - (v*0.5+0.5)) * ch_h` で quad を組むため、
   ±25600 級の値は ±12800 ch_h の巨大 quad となり canvas 全面を
   `viz_waveform_rms`(#2B5F9E 系)で塗り潰す。Meter は peak を
   [0,1] に clamp するため全 bar が全高になり同色のベタ塗りに
   なる。実機 capture で観測された単色 `#2B5F9E` フレームは
   DComp artifact ではなくこの paint 結果そのものであった。

ruviform-viz の `WaveformSource`/`PcmBlock` は振幅スケールを
契約に持たず、renderer 側は単位振幅を暗黙に仮定する。dataset の
artifact(`pcm: unscaled_interleaved_f64le` + `audio.sample_format`)
は AL 固有の形式であり、スケールの正本は AL 側にある。したがって
修正境界は `DatasetWaveform`(`al-service`)とし、`read` が返す
サンプルを `format_scale(sample_format)` で [-1,1] に正規化する。

## 変更内容

- `crates/al-service/src/source.rs`
  - `DatasetWaveform` に `scale: f64` フィールドを追加。
  - `open` で `audio.sample_format` を `SampleFormat` に
    deserialize し `al_locate::decode::format_scale` でスケールを
    求める。欠落・不正値は他の audio metadata 欠落と同様に
    `InvalidData` で open を失敗させる。
  - `read` で `scale != 1.0` のとき各サンプルを `scale` で除算
    する。`NaN` は `NaN/scale = NaN` で温存され、gap 意味論は
    維持される。
- `crates/al-service/tests/service.rs`
  - `waveform_source_yields_samples_normalized_to_unit_range`:
    pcm_s16 入力の dataset で `session.waveform().read()` の
    最大絶対値が `200/32768` になることを検証する回帰テスト。

## 代替案と却下理由

- **al-gui の `waveform_bin`/`meter_bin` で正規化**: scale を
  `WaveformInfo` に載せる必要があり ruviform-viz の API 変更を
  要求する。`al.view.waveform` op(RVBW)経路も生値のまま残り
  不整合になるため却下。
- **ruviform 側で正規化**: port 契約はスケール非依存であり、
  dataset artifact 形式は AL 固有。責務境界上 AL 側が正しい。
- **`DatasetSession::waveform` の戻り値をラップ**: `DatasetWaveform`
  単体で完結するため、構造体内部で吸収する方が単純。

## リスクと影響

- `session.waveform()` の全 consumer(GUI view・`waveform_view` op)
  が正規化値を受け取る。RVBW を消費する webview 側 renderer も
  単位振幅前提であり整合する。
- float 系 `SampleFormat` は `format_scale = 1.0` で除算を skip
  し、従来と bit 同一の値を返す。
- 生スケールが必要な解析経路は `session.waveform` を使っていない
  ことを確認済み(`al-locate::decode` は別経路で `format_scale`
  済み)。
