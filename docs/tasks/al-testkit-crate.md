---
id: "TASK-034AaFYEyp2hJkFmKrHAYS"
type: "task"
status: "done"
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

dev-dependency 専用の補助クレート `al-testkit` を作り、信号生成/変換(gain/EQ/delay/jitter/polarity/mix/SRC/clip/drop/repeat/stuck/noise)とメディア fixture を提供する。`AviSpec`/`write_avi` を `al-media` から移管し、パラメタ化 AVI writer(movi パケット列・per-stream dwStart・0 長チャンク指定可能)へ拡張する。`al-cli` の gen_samples 例は dev-dependency で参照する。MP4 writer は WS-C の成果物を集約する。

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
- [ ] VFR/dwStart≠0/dup/drop を含む AVI fixture を生成できる
- [ ] gen_samples 例が al-testkit 経由でビルドされる

## Verification

- [ ] `cargo test -p al-testkit`
- [ ] `cargo build --workspace`
