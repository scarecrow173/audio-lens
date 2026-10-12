---
id: "TASK-034EpP87rKX85uUeXWXtA5"
type: "task"
status: "done"
title: "al-math: vec プリミティブモジュール(カーネル + dispatch + linalg 共有)"
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
    - "docs/plans/al-math-vec-primitives.md"
  implemented-by:
    - "docs/impl/ir/al-math-vec-primitives.md"
  depends-on:
    - "docs/plans/al-math-vec-primitives.md"
  blocks:
    - "docs/tasks/al-locate-vec-migration.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/al-math-vec-primitives.md"
    - "docs/adr/al-math-vec-primitives-slice-boundary.md"
    - "docs/adr/simd-reduction-lane-fma.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/al-math-vec-primitives.md"
  references: []
  defers: []
  deferred-by: []
---

# al-math: vec プリミティブモジュール(カーネル + dispatch + linalg 共有)

## Work

`crates/al-math/src/vec.rs` を新設し、8 プリミティブを design の
3 層パターンで実装する(plan T1)。`lib.rs` に `pub mod vec;` を
追加し、`linalg.rs` の内部 `dot` を共有カーネルに統一する。

- 構成(design L111-141): 共有 `check_same_len` ヘルパ、
  各プリミティブの `pub(crate) xxx_kernel<S: Simd>(simd, ...)`
  ジェネリックカーネル、カーネルを呼ぶ薄い `WithSimd` アダプタ
  struct、公開 wrapper(公開入口で `dispatch(...)` を1回だけ
  呼ぶ)。pulp 型は公開シグネチャに出さない
- 公開プリミティブ(design L142-166): `dot`/`sum_sq`/`corr0`/
  `rms`/`scale`/`axpy`/`sub_scaled`/`mag_sq`。`corr0` は
  Σxy・Σx²・Σy² の 3 アキュムレータを 1 パスで融合する
- 等長 panic: `dot`/`corr0`/`scale`/`axpy`/`sub_scaled` は
  2 スライス(入出力含む)、`mag_sq` は `re`/`im`/`out` の
  3 スライス全て。min-len(zip)黙認は継承しない
- 値の契約: `corr0` は `Σx² > 0 かつ Σy² > 0` でなければ
  0.0(NaN ・縮退エネルギーも 0.0)、中心化しない。
  `rms` は空入力で 0.0
- カーネルの pulp op(design L167-191、
  `docs/adr/simd-reduction-lane-fma.md` が権威): 積和は
  `mul_add_e_f64s` のみ。`mul_add_f64s`(Scalar でも融合となり
  ビット一致を破る)と `mul_f64s`+`add_f64s` の 2 命令形は禁止。
  `sub_scaled` は `negate_mul_add_e`、`mag_sq` は
  `mul_add_e(r, r, mul_f64s(i, i))`。`scale` は純粋乗算
  `mul_f64s`。lane アキュムレータ + `reduce_sum` + scalar tail
- `rms` は `n == 0 ? 0.0 : (sum_sq_kernel(x)/n).sqrt()` で
  カーネル共有し追加 dispatch しない
- `linalg.rs`: 内部 `fn dot<S: Simd>` を削除し呼出し側
  (Factor/Apply)を `crate::vec::dot_kernel` へ。`reflect`
  (:15-26、`*t = t - factor·v`)は既存 `WithSimd` コンテキスト内で
  `let factor = 2.0 * crate::vec::dot_kernel(simd, v, target);
  crate::vec::sub_scaled_kernel(simd, target, factor, v);` に
  書き換える(二重 dispatch を避けるカーネルレベル呼出し、
  等長は既に保証)
- `#[cfg(test)] mod tests`(vec.rs 内): 契約テスト(corr0 の
  ゼロ・片側ゼロ・NaN → 0.0、`corr0(x,x)`==1.0、直交 → ~0.0、
  `rms` 空 → 0.0・定数入力 → |c|)、`#[should_panic]` 等長
  テスト(dot/corr0/scale/axpy/sub_scaled、mag_sq の re≠im と
  re≠out)、scalar ビット一致テスト(`pulp::Scalar` 経路を
  テスト内 `.iter().sum()` 系参照と `assert_eq!`、サイズ
  0..=33・1k・64k)。dispatch↔Scalar 等価テスト(~1M 含む)
  は T4 で追加する

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/al-math-vec-primitives.md`
- `docs/designs/al-math-vec-primitives.md`(vec.rs 構成・カーネル
  表・linalg 共有)
- `docs/adr/al-math-vec-primitives-slice-boundary.md`(公開境界・
  panic 契約・値の契約の権威)
- `docs/adr/simd-reduction-lane-fma.md`(`mul_add_e_f64s` 規則・
  scalar ビット一致の権威)

## Dependencies

- なし(先行タスクなし。T2 が本タスクに依存する)

## Affected Files

- `crates/al-math/src/vec.rs`(新設): 全 8 プリミティブの
  3 層実装 + `#[cfg(test)]` テスト
- `crates/al-math/src/lib.rs`: `pub mod vec;` 追加
- `crates/al-math/src/linalg.rs`: 内部 `dot` 削除・呼出しを
  `dot_kernel` へ、`reflect` を `dot_kernel` +
  `sub_scaled_kernel` へ

## Done When

- [ ] 8 プリミティブが `&[f64]`/`&mut [f64]` + f64 スカラーの
  みの公開シグネチャで存在し、pulp 型が公開面に出ない
- [ ] 各公開 wrapper が `dispatch` を入口で 1 回だけ呼び、
  カーネルが `Simd` ジェネリック単一ソースである
- [ ] リダクション積和が `mul_add_e_f64s` のみで記述され、
  `mul_add_f64s`・`mul_f64s`+`add_f64s` 形が使われていない
- [ ] 契約テスト・`#[should_panic]` 等長テスト(2 入力全種 +
  `mag_sq` の 3 スライス)・scalar ビット一致テストが pass する
- [ ] `linalg.rs` の内部 `dot` が `vec::dot_kernel` と共有化
  され、`reflect` が `sub_scaled_kernel` を消費する

## Verification

- [ ] `mise exec -- cargo test -p al-math` で契約・panic・
  scalar ビット一致テストが pass
- [ ] `mise exec -- cargo build --workspace` が green
- [ ] `mise exec -- cargo clippy`・`cargo fmt --check` が green
