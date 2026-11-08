---
id: "SPEC-9RvxFJGfN5O4Xk7IQ7Kzc3"
type: "spec"
status: "approved"
title: "canvas 系 component の実窓 paint 是正（DatasetWaveform の単位振幅正規化）(Issue #179)"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/179"
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
    - "../designs/canvas-paint-empty.md"
    - "../test-specs/canvas-paint-empty.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/canvas-paint-empty.md"
  references:
    - "native-gui-workspace-tabs-selection.md"
    - "spectrogram-tab-hang.md"
  defers: []
  deferred-by: []
---

# canvas 系 component の実窓 paint 是正（DatasetWaveform の単位振幅正規化）(Issue #179)

## What Should Be Built

Dataset → Source 画面の Waveform・Meter・TimeAxis tick mark・
Spectrogram cell が、Windows 実窓で実値どおりに描画される状態を
実現する。AL#166 受入時の「canvas bounds は非ゼロだが paint が空」
症状の root cause は、AudioLens の `DatasetWaveform`
(`al-service`) が dataset の `inputs/<id>.wav` を**生の整数 PCM
スケール**(pcm_s16 なら ±32768)のまま `ruviform-viz` の
`WaveformSource` port に供給していたことである。ruviform-viz の
port は振幅スケールを持たず、GPUI renderer は Waveform を [-1,1]・
Meter を [0,1] の単位振幅として paint するため、生スケール値は
桁違いに巨大な quad となり、canvas 全面を `viz_waveform_rms` 色で
塗り潰していた(旧 pin `a5b1db1` では RF#453 の canvas 0 サイズが
これを「何も描かれない」空 paint として隠していた)。

本 spec は `DatasetWaveform` が `audio.sample_format` に対応する
`format_scale` でサンプルを [-1,1] に正規化して返すこと、および
実窓 pixel 解析による受入証拠の保存を定める。

## Why Now

AL#166 実機受入で Waveform/Meter/TimeAxis tick が描画されない
ことが報告され(AL#179)、AL#180 で Spectrogram の hang は
ruviform pin `863a7a9d` 追従により解消済みだが、Source 画面の
canvas paint は未検証のまま残っていた。headless/Fake parity は
projection が非空であることしか保証せず、renderer が仮定する
振幅スケールの契約は検証対象外だった。

## Acceptance Criteria

- `DatasetWaveform::read` が返す `PcmBlock.samples` が
  `audio.sample_format` に対応するスケールで [-1,1] に正規化される
  (pcm_s16 → /32768、float → 無変換)。
- Source 画面で Waveform bins・Meter bar・TimeAxis tick mark・
  Spectrogram cell が実値で描画される pixel 解析証拠を
  `.devin/evidence/` に保存する。
- AL projection(`source_screen`)が供給する bins/bars/ticks/cells
  が非空であることを確認済みであること(切り分け証拠)。

## Non-Goals

- ruviform 側の変更(RF#453/RF#461 は `863a7a9d` で解消済み)。
- Spectrogram の大きさ正規化(`spectrogram_screen` の
  `display_range` がデータ駆動で既に自己正規化している)。
- `al.view.waveform` op の RVBW フォーマット変更(正規化後の値を
  そのまま載せるのみ)。
