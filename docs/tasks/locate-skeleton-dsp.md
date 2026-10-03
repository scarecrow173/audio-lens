---
id: "TASK-034z8kzGSMXKkTFTGi2GJ6"
type: "task"
status: "done"
title: "al-locate 骨格 + DSP プリミティブ + workspace 配線"
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
    - "docs/plans/locate-and-tracking.md"
  implemented-by: []
  depends-on:
    - "docs/plans/locate-and-tracking.md"
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
---

# al-locate 骨格 + DSP プリミティブ + workspace 配線

## Work

workspace `members`/`workspace.dependencies`・`al-cli/Cargo.toml`・`al-locate` クレートを配線し、DSP プリミティブを実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/locate-and-tracking.md`
- `docs/designs/localization.md`

## Dependencies

- なし(先行 task は dataset 系、別ストリーム)

## Affected Files

- `Cargo.toml`
- `crates/al-cli/Cargo.toml`
- `crates/al-locate/Cargo.toml`(al-core / al-job / al-media / serde / serde_json)
- `crates/al-locate/src/lib.rs`
- `crates/al-locate/src/dsp.rs`(NCC カーネル、DFT/FFT 補助、バンドパス、スペクトル包絡、最小二乗、interleaved f64 の deinterleave)

## Done When

- [ ] `cargo metadata` が `al-locate` を workspace member として解決
- [ ] NCC・バンドパス・最小二乗の単体テストが通る
- [ ] deinterleave が ch 単位の連続配列を返す

## Verification

- [ ] `cargo test -p al-locate` パス
- [ ] `cargo clippy -p al-locate` clean
