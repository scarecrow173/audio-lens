---
id: "TASK-034DYiMM43O9mMDdme6ijq"
type: "task"
status: "done"
title: "al-math: 公開面の pub(crate) 化と intra-doc link 修正"
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
    - "docs/plans/al-math-api-boundary.md"
  implemented-by: []
  depends-on:
    - "docs/plans/al-math-api-boundary.md"
    - "docs/tasks/al-math-split-re-im-tests.md"
    - "docs/tasks/al-locate-split-re-im-fft.md"
    - "docs/tasks/bench-correlation-split-re-im.md"
  blocks:
    - "docs/tasks/al-math-api-boundary-verification.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/al-math-api-boundary.md"
  references: []
  defers: []
  deferred-by: []
---

# al-math: 公開面の pub(crate) 化と intra-doc link 修正

## Work

plan S5: `lib.rs` 公開面の `pub(crate)` 化。外部の `al_math::c64`
等の参照が全て消えた後(S1〜S4 完了後)に実施する。

- `crates/al-math/src/lib.rs`:
  - `pub use pulp::{Simd, WithSimd, c64}` を `pub(crate) use` に
  - `pub type F64s`/`C64s` を `pub(crate) type` に
  - `pub fn dispatch` を `pub(crate) fn` に降格する
  - クレート doc の `[`Simd`]`/`[`WithSimd`]`/`[`dispatch`]`
    intra-doc link をコード表記に書き換える

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/al-math-api-boundary.md`(S5)
- `docs/specs/al-math-api-boundary.md`(公開面の中立性の受入基準)
- `docs/test-specs/al-math-api-boundary.md`(R1)

## Dependencies

- `docs/tasks/al-math-split-re-im-tests.md`(S2)
- `docs/tasks/al-locate-split-re-im-fft.md`(S3)
- `docs/tasks/bench-correlation-split-re-im.md`(S4)

外部参照が残ったまま `pub(crate)` 化すると workspace がコンパイル
不能になるため、全追従スライスの完了が前提。

## Affected Files

- `crates/al-math/src/lib.rs`

## Done When

- [ ] `Simd`/`WithSimd`/`c64`/`F64s`/`C64s`/`dispatch` が全て
  `pub(crate)` または private である
- [ ] クレート doc の intra-doc link がコード表記に修正されている
- [ ] `al-math` の公開関数シグネチャにバックエンド固有型がない

## Verification

- [ ] `mise exec -- cargo build --workspace` が成功する(第一ゲート。
  brace import や複数行 `use` など grep が取りこぼす参照も
  コンパイルで検出する)
- [ ] `mise exec -- cargo doc -p al-math` が warning なく成功する
- [ ] `rg 'al_math::[\w:{}, ]*\b(c64|Simd|WithSimd|F64s|C64s|dispatch)\b' crates/`
  のヒットが 0 件
- [ ] `rg 'pulp' crates/ --glob '!al-math/**'` のヒットが 0 件
  (`al-math` 内部の `pulp`/`crate::Simd` 等の参照は許容)
