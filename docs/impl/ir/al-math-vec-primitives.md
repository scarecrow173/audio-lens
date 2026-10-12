---
id: "IMPL-034Eq5vi0HaWA9IMWHvhiG"
type: "impl"
status: "completed"
title: "al-math vec primitives"
created: "2026-10-12"
updated: "2026-10-12"
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
    - "docs/tasks/al-math-vec-primitives.md"
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

# al-math vec primitives

## Summary

`al_math::vec` モジュールを新設し、f64 スライスの 8 プリミティブ
(`dot`/`sum_sq`/`corr0`/`rms`/`scale`/`axpy`/`sub_scaled`/`mag_sq`)を
カーネル・`WithSimd` アダプタ・公開 wrapper の 3 層パターンで実装した。
`linalg.rs` の内部 `dot` を `vec::dot_kernel` に共有化し、`reflect` を
`dot_kernel` + `sub_scaled_kernel` のカーネルレベル消費に切り替えた。

## Context

task `docs/tasks/al-math-vec-primitives.md`、design
`docs/designs/al-math-vec-primitives.md`、ADR
`docs/adr/al-math-vec-primitives-slice-boundary.md`(公開境界・panic・値契約)
と `docs/adr/simd-reduction-lane-fma.md`(`mul_add_e_f64s` 規則・scalar
ビット一致)に従う。

## Implementation

### Changes

- `crates/al-math/src/vec.rs`(新設): 共有 `check_same_len`、8 つの
  `pub(crate) xxx_kernel<S: Simd>`、対応する `WithSimd` アダプタ、
  入口で `dispatch` を 1 回呼ぶ公開 wrapper。積和は `mul_add_e_f64s`
  (`sub_scaled` は `negate_mul_add_e_f64s`、`mag_sq` は
  `mul_add_e(r, r, mul_f64s(i, i))`、`scale` は純粋乗算 `mul_f64s`)。
  `rms_kernel` は `sum_sq_kernel` を再利用し追加 dispatch しない。
  `#[cfg(test)]` に契約・`should_panic`・scalar ビット一致テスト
- `crates/al-math/src/lib.rs`: `pub mod vec;` 追加
- `crates/al-math/src/linalg.rs`: 内部 `fn dot` と `reflect` 本体を
  削除し、`crate::vec::{dot_kernel, sub_scaled_kernel}` を既存の
  `WithSimd` コンテキスト内で直接呼ぶ形に統一(二重 dispatch なし)

### Decisions During Implementation

- `axpy` の引数順は design 確定シグネチャ `axpy(y, a, x)`
  (`sub_scaled(y, a, x)` と対称)を採用した
- `check_same_len` は長さ(`usize`)ペアを受け取る形にし、
  `mag_sq` の 3 スライス確認を 2 呼出しで共有した
- corr0 の INFINITY 入力は `inf/inf = NaN` を返す(両エネルギー > 0
  のため契約上は ratio 経路)ため、契約テストは NaN・ゼロエネルギー
  の 0.0 写像のみを assert する範囲に留めた

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

- `mise exec -- cargo test -p al-math`: 29 tests pass(vec.rs 12 件 —
  corr0 契約 2、rms 契約 1、等長 panic 7、scalar ビット一致 2)
- `mise exec -- cargo build --workspace`: green
- `mise exec -- cargo fmt --all -- --check`: green
- `mise exec -- cargo clippy -p al-math --all-targets -- -D warnings`: green

## Risks

- `mul_add_e_f64s` 化により linalg の積和が x86-v3 で真の FMA に変わり、
  従来の `mul`+`add` 逐次形と ~ulp 級の差が生じ得る(design/ADR で
  許容済み、既存テストは全て pass)
- dispatch↔Scalar 等価テスト(~1M 含む)は後続タスク
  `docs/tasks/vec-primitives-verification-bench.md` の範囲で未追加

## Follow-ups

- T2 以降: `al_locate::dsp` re-export・呼出し側移行・criterion ベンチ
  (`docs/tasks/al-locate-vec-migration.md`・
  `docs/tasks/al-analysis-vec-migration.md`・
  `docs/tasks/vec-primitives-verification-bench.md`)
