---
id: "TASK-034AaFTafwUqSb6Zp9Pr8J"
type: "task"
status: "done"
title: "exp-channel: 周波数依存混合行列 + intermod 証拠"
created: "2026-10-04"
updated: "2026-10-04"
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
    - "docs/plans/expensive-families.md"
  implemented-by: []
  depends-on:
    - "docs/plans/expensive-families.md"
    - "docs/tasks/expensive-module-split.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/expensive-families.md"
  references: []
  defers: []
  deferred-by: []
---

# exp-channel: 周波数依存混合行列 + intermod 証拠

## Work

帯域ごとの最小二乗混合係数(周波数依存混合行列)と intermodulation 証拠(強トーン周辺の実用的範囲)を `expensive/channel.rs` に実装する。混合行列は source→capture のペア証拠が必要で、リファレンスペア不在時は invalid(`pair evidence unavailable`)とする。係数の符号を極性ルーティング証拠としても記録する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/expensive-families.md`
- `docs/test-specs/expensive-families.md`

## Dependencies

- `docs/tasks/expensive-module-split.md`

## Affected Files

- `crates/al-analysis/src/expensive/channel.rs`

## Done When

- [ ] 既知混合で帯域係数(符号含む)が回復される
- [ ] 条件数の悪い推定は valid にならない

## Verification

- [ ] `cargo test -p al-analysis`
