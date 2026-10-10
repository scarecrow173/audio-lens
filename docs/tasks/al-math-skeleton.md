---
id: "TASK-035cMnBvCxZaSdFgHjKlQp"
type: "task"
status: "in-progress"
title: "al-math: クレート骨格 + pulp dispatch 基盤"
created: "2026-10-09"
updated: "2026-10-10"
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
    - "docs/plans/simd-dsp-foundation.md"
  implemented-by: []
  depends-on:
    - "docs/plans/simd-dsp-foundation.md"
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
    - "docs/test-specs/simd-dsp-foundation.md"
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/42"
  defers: []
  deferred-by: []
---

# al-math: クレート骨格 + pulp dispatch 基盤

## Work

`al-math` クレートを workspace に新設する。pulp を唯一の外部 SIMD
依存とし、`Arch::new()` を共有した dispatch 基盤を集約する。

- workspace `Cargo.toml` に `al-math` を登録
- `pulp` 依存追加(ユーザーの採用決定: ADR simd-arithmetic-pulp)
- `Arch` をプロセス共有で保持し `dispatch(WithSimd)` を呼び出す
  公開エントリ
- f64xN / c64xN など AudioLens で使う型の再エクスポート
- scalar フォールバック経路(pulp が自動生成)が同一ソースから
  動くことを確認する smoke test

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/simd-dsp-foundation.md`
- `docs/adr/simd-arithmetic-pulp.md`

## Dependencies

なし(プラン直下の最初のタスク)

## Affected Files

- `Cargo.toml`(workspace members)
- `crates/al-math/Cargo.toml`(新規)
- `crates/al-math/src/lib.rs`(新規)

## Done When

- [ ] `cargo build -p al-math` が通る
- [ ] `Arch` dispatch 経由で f64 カーネルが動作する
- [ ] scalar フォールバック結果と一致する smoke test

## Verification

- [ ] `cargo test -p al-math`
