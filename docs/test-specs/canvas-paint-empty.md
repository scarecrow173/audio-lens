---
id: "TSPEC-TtiAdwHDGfAC5s9X2nNkrz"
type: "test-spec"
status: "approved"
title: "canvas paint 是正のテスト仕様"
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/canvas-paint-empty.md"
  derived-by: []
  verified-by: []
  verifies:
    - "docs/specs/canvas-paint-empty.md"
  references: []
  defers: []
  deferred-by: []
---

# canvas paint 是正のテスト仕様

## Feature: DatasetWaveform の単位振幅供給

`DatasetWaveform`(`al-service`)が `inputs/<id>.wav` を range read
する際、返す `PcmBlock.samples` は `audio.sample_format` に
対応するスケールで [-1,1] に正規化されていなければならない。

### Rule: pcm_s16 入力は 32768 で除算される

- **Example**: `(i % 400) - 200` の s16 サンプルを持つ wav を
  ingest した dataset で `session.waveform(input_id).read(0, 256)`
  を呼ぶと、返るサンプルの最大絶対値は `200.0 / 32768.0`
  (≈0.0061035)に一致する。生スケールの `200.0` が返っては
  ならない。

### Rule: 正規化は NaN を壊さない

- **Example**: 非有限値(欠損マーカー)を含むブロックでも、
  除算後も `NaN` のまま保持され、gap 判定
  (`count == 0` / 非有限値の除外)を壊さない。

### Rule: float 入力は無変換

- **Example**: `sample_format` が `Float32`/`Float64` の dataset
  では `format_scale = 1.0` となり、サンプル値は bit 同一で
  返る。

## 検証証拠(実窓)

headless テストは port 契約のみを保証する。Issue #179 の受入は
Windows 実窓の pixel 解析で行い、Waveform bins・Meter bar・
TimeAxis tick mark・Spectrogram cell が実値で描画されることを
`.devin/evidence/al179-canvas-paint/` に保存する。
