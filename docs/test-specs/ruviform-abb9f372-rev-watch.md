---
id: "TSPEC-034Y1A2b3C4d5E6f7G8h9"
type: "test-spec"
status: "approved"
title: "ruviform pin abb9f372 追従 + consumer oracle テスト仕様"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/ruviform-abb9f372-rev-watch.md"
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
    - "docs/test-specs/issue-127-rf628-consumer-oracle.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/ruviform-abb9f372-rev-watch.md"
  derived-by: []
  verifies:
    - "docs/specs/ruviform-abb9f372-rev-watch.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform pin abb9f372 追従 + consumer oracle テスト仕様

## Purpose

pin 追従と consumer oracle の検証対応表。oracle テスト詳細の
正本は `docs/test-specs/issue-127-rf628-consumer-oracle.md`。

## Feature

pin `abb9f372` 解決の健全性 + `rf628_oracle.rs` の 3 脚が
file-based 解析の f64/長尺/gap を独立 Oracle で判定する。

## Rules

- **R1 単一 SHA**: `check-ruviform-lock`/`check-ruviform-rev` が
  26 package・単一 SHA `abb9f372`・vendor REV 一致で Pass。
- **R2 fixture/oracle 契約**: `pcm`/`oracle` module は前 pin
  無変更であり、`pcm_assert_eq` による fixture 検証が全 pattern
  （Exact/AbsEpsilon(1e-5)）で Pass。
- **R3 AL-1 f64 fidelity**: level record が oracle f64 統計量・
  解析的参照と 1e-9 級許容差で一致（dc/peak/tp/rms_dbfs/crest）。
- **R4 AL-2 長尺**: 30s contiguous scenario（Noise→Sine→Square）
  で trajectory window 数一致・`ref.gain_ls`/`rms_ratio`≈0.5・
  `lag_median`≈0・`similarity_fullband`>0.9・capture_range.start
  ≈embed_at。
- **R5 AL-3 gap**: 非連続 scenario の NaN 窓位置・robust
  support_count・dc_offset null・peak 有限・rms_dbfs -240
  floor・`params.ruviform` stamp が segment 幾何由来の期待
  validity map と一致。
- **R6 正直な verdict**: 実機/3OS/cross-consumer/clock/device
  legs は NotRun/NotApplicable/Deferred と記録。

## Examples

- `rf628_fixture_matches_independent_oracle`: 8 pattern ×
  1ch/2ch で `pcm_assert_eq`。
- `rf628_f64_fidelity_{constant,periodic,noise,stereo_channels}`:
  1-4s 入力で level record 照合 + tonal 周波数 + provenance。
- `rf628_long_scenario_non_regression`: 30s/1.44M frames。
- `rf628_gap_nan_validity`: 4.5s・gap 0.5s・51 NaN windows。

## Guarantees

- 上記 Rules の全 assert が対象 revision の Windows debug
  プロファイルで Pass すること。

## Non-goals

- clock/device-pipeline legs（Deferred/NotApplicable）、
  実機 GUI/3OS/cross-consumer、spectral gap 縮退の修正。

## Risk

- 長尺テストの debug 実行時間（実測 ~85s）→ test-spec に規模の
  根拠を記録済み。
