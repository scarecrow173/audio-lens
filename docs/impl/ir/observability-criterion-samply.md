---
id: "IMPL-01M4PAYJMMWWXX3FJG5J45TC2M"
type: "impl"
status: "completed"
title: "criterion smoke bench と samply 手順書"
created: "2026-10-11"
updated: "2026-10-11"
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
    - "docs/tasks/observability-criterion-samply.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/observability-foundation.md"
    - "docs/specs/observability-foundation.md"
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

# criterion smoke bench と samply 手順書

## Summary

L1 計測として al-math に criterion smoke bench を追加し、L3 計測
として `docs/guides/profiling-samply.md` に samply 手順書を作成
した。

## Context

対応 task は `docs/tasks/observability-criterion-samply.md`、
plan は `docs/plans/observability-foundation.md`、上位設計は
`docs/designs/observability-foundation.md`、spec は
`docs/specs/observability-foundation.md`、検証仕様は
`docs/test-specs/observability-foundation.md`。

## Implementation

### Changes

- `crates/al-math/Cargo.toml`: dev-dependency に
  `criterion = "=0.8.2"` を pin、`[[bench]] name = "dsp_smoke"
  harness = false` を追加
- `crates/al-math/benches/dsp_smoke.rs`(新設): 公開 dispatch 経路
  の `al_math::correlation::ncc` と `al_math::filter::
  spectral_envelope` を固定サイズ(signal 8192 / template 1024
  サンプル)で測る最小ベンチ
- `docs/guides/profiling-samply.md`(新設): install・
  `[profile.release] debug = true`・実行例・`.samply` 出力の
  見方を記述
- `docs/guides/README.md`(新設): guides 索引
- root `Cargo.toml`: `[profile.release] debug = true` を追加

### Decisions During Implementation

- **criterion 版**: `=0.8.2`(crates.io 公開日 2026-02-04、追加日
  2026-10-11 で ≥7 日経過ルールを充足。最新安定版かつ
  rust-version 1.86 ≤ toolchain 1.98.1)。pulp と同じ `=` pin
  規約に合わせた
- **`[profile.release] debug = true`**: samply のシンボル解決に
  debug info が必須のため追加。codegen/最適化は不変で cost は
  成果物サイズのみ — 計測対象そのものが profiling 用途のため
  design/spec の意図と整合すると判断
- bench 対象は既存 dispatch 経路の public kernel 2 本に限定。
  vec プリミティブ本格ベンチ(計測対象・項目の選定)は次 chain
  の `al-math-simd-reduction` に deferred

## Related Experiments

### Adopted

### Rejected

## Validation

- `cargo bench -p al-math` 実行成功:
  - `correlation::ncc`: 約 413 µs([408.56–419.41 µs])
  - `filter::spectral_envelope`: 約 79 µs([77.98–80.79 µs])
- `cargo test -p al-math`・`cargo fmt --check` green

## Risks

- criterion は dev-dep のみで production 依存グラフに影響なし

## Follow-ups

- vec プリミティブ本番ベンチ: `al-math-simd-reduction` chain
- logging OFF 時 overhead の記録: dsp_smoke を足場に
  criterion で空転 cost を測るか、examples ベンチ前後比較かは
  未決(plan の Open Question)
