---
id: "IMPL-034z0V1ipXWWbtAQc4rt15"
type: "impl"
status: "completed"
title: "timing observation"
created: "2026-10-02"
updated: "2026-10-02"
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
    - "docs/tasks/timing-observation.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# timing observation

## Summary

タイムスタンプ系列から間隔・ドリフト・不連続・重複/ドロップ候補を
観測する `observe_timing` を実装した。CFR / 公称 FPS は仮定しない。

## Context

task `docs/tasks/timing-observation.md`、test-spec
`av-timing-observation` に従う。

## Implementation

### Changes

- `timing.rs`: `TimestampSource` trait + `SequenceTimestamps`、
  `ClockKind`(Audio/Video 個別保持)、`IntervalStats`、
  `Discontinuity`、`TimingObservation`、`observe_timing()` /
  `observe_source()`
- 不連続判定は局所中央値(2x 以上または負の間隔)で、VFR の変動が
  誤検出にならない
- drop 候補は「孤立した」大きな間隔のみに限定(両隣が中央値近傍)。
  持続的な VFR 低下は不連続として記録するが drop にはしない
- drift は nominal interval 指定時のみ ppm で報告(生系列は変更しない)

### Decisions During Implementation

- `TimestampSource` trait を導入し、コンテナ抽出・合成系列の双方が
  同一の観測経路に乗る形にした(plan/task の記載どおり)
- `observe_timing()` はスライス入力の純粋関数として残し、
  `observe_source()` が trait 経由の薄い入口になる

## Validation

- `cargo test -p al-media`: 16 tests pass(等間隔 / VFR / ギャップ /
  重複 / drift / 空系列)
- `clippy -D warnings` / `cargo fmt --check`: clean

## Follow-ups

- 実コンテナからの PTS 抽出(MKV/MP4 パーサ)は別プラン
