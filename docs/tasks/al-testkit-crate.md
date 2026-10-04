---
id: "TASK-034AaFYEyp2hJkFmKrHAYS"
type: "task"
status: "todo"
title: "al-testkit: 補助クレート + 信号/メディアビルダ"
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
    - "docs/plans/golden-validation.md"
  implemented-by: []
  depends-on:
    - "docs/plans/golden-validation.md"
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
    - "docs/test-specs/golden-validation.md"
  references: []
  defers: []
  deferred-by: []
---

# al-testkit: 補助クレート + 信号/メディアビルダ

## Work

dev-dependency 専用の補助クレート `al-testkit` を作り、信号生成/変換(gain/EQ/delay/jitter/polarity/mix/SRC/clip/drop/repeat/stuck/noise)とメディア fixture(WAV/AVI)を提供する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/golden-validation.md`
- `docs/test-specs/golden-validation.md`

## Dependencies

- none

## Affected Files

- `crates/al-testkit/Cargo.toml`
- `crates/al-testkit/src/lib.rs`
- `crates/al-testkit/src/signal.rs`
- `crates/al-testkit/src/media.rs`
- `Cargo.toml`

## Done When

- [ ] クレートが workspace に追加され各 crate の dev-dependency で使える
- [ ] 変換が既知パラメータで生成できる

## Verification

- [ ] `cargo test -p al-testkit`
- [ ] `cargo build --workspace`
