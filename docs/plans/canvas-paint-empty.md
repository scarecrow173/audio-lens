---
id: "PLAN-xXrPVbEkKKBVIdkHNYU0Jw"
type: "plan"
status: "approved"
title: "canvas paint 是正の実装計画"
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
  implements:
    - "docs/specs/canvas-paint-empty.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines:
    - "docs/specs/canvas-paint-empty.md"
    - "docs/designs/canvas-paint-empty.md"
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/canvas-paint-empty.md"
  references: []
  defers: []
  deferred-by: []
---

## 概要

`DatasetWaveform` が dataset の生 PCM スケールをそのまま
`WaveformSource` port に供給していたため、ruviform-ui-gpui が
仮定する単位振幅([-1,1]/[0,1])を満たさず、Waveform/Meter の
quad が canvas 全面を塗り潰していた。修正は `DatasetWaveform`
での正規化 1 点に限定し、AL projection・ruviform・ViewSpec の
変更は行わない。

## ファイルマップ

- `crates/al-service/src/source.rs`(変更)
  - `DatasetWaveform` に `scale: f64` を追加し、`open` で
    `audio.sample_format` → `format_scale` を解決、`read` で
    除算する。
- `crates/al-service/tests/service.rs`(変更)
  - 正規化を検証する回帰テストを追加。

## 手順

1. `service.rs` に失敗する回帰テストを追加(red: 生スケール 200.0
   が返ることを確認)。
2. `source.rs` を修正して green にする。
3. `cargo build -p al-desktop` で実窓バイナリを再生成し、
   Windows 実窓で Dataset → Source / Spectrogram を pixel 解析。
4. 証拠を `.devin/evidence/al179-canvas-paint/` に保存。
5. `docs/impl/ir/` に実装記録を残し、index を更新。

## 検証マトリクス

| 層 | 検証 | 方法 |
| --- | --- | --- |
| port 契約 | samples が [-1,1] | `cargo test -p al-service`(補助) |
| 正規検証 | fmt/check/lint/test + ui 系 | `mise run *` |
| 実窓受入 | Waveform/Meter/Tick/Spectrogram の pixel | 実機 capture + 色解析 |

## リスク

- `waveform_view` op(RVBW)も正規化値になる。webview 側が
  生スケールを期待していないかは既存 renderer が単位振幅前提
  であることから問題なし。
- `sample_format` 欠落 dataset は open 時に `InvalidData` で
  失敗する。pre-release のため artifact 再生成で対応可能。
