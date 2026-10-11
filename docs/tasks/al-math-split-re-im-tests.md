---
id: "TASK-034DYiFVJy5OyohvPiyzkf"
type: "task"
status: "done"
title: "al-math: tests の split re/im 追従と独立参照の保持"
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
    - "docs/tasks/al-math-split-re-im-internals.md"
  blocks:
    - "docs/tasks/al-math-api-pub-crate.md"
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

# al-math: tests の split re/im 追従と独立参照の保持

## Work

plan S2(統合スライス直後の最優先): `al-math` の tests 2 件を split
API に追従させ、独立参照比較を split 形で保持する。S1 後は
scalar/dispatch 一致テストしか通らず、カーネル書き換えミスの系統
誤りを検出する独立参照比較が blind window になるため、S3・S4 に
先行させて閉じる。

- `crates/al-math/tests/fft.rs`:
  - `reference()`(radix-2 独立参照)を re/im ペア操作に書き換え、
    `close()` を re/im ペア比較にする。`al_math::c64` は使わず
    `(f64, f64)` タプルまたは re/im 演算で記述する
  - 全テスト(`complex_matches_legacy_forward_inverse_and_roundtrip`、
    `real_half_spectrum_matches_complex_and_roundtrips`、
    `rejects_*` 4 件、`matches_independent_dft_and_is_repeatable`、
    `real_inverse_ignores_endpoint_imaginary_parts`、
    `immutable_complex_plan_can_be_shared`)を split API に追従させる。
    naive DFT 比較は削除せず re/im 累積形に書き換えて保持する
- `crates/al-math/tests/correlation_filter.rs`:
  - `complex_spectrum` ヘルパを re/im ペア返却に書き換え、
    `real_kernels_preserve_previous_complex_spectrum_contract` 内の
    複素演算(mask・conj 積・norm_sqr)を実部演算に追従させる
- チェック: `power` ループ範囲(`tests/fft.rs` の `0..=16`、
  `fft.rs` モジュール内テストの `0..=10`)とエッジ条件テスト
  (n=1、DC/Nyquist、`real_inverse_ignores_endpoint_imaginary_parts`)
  を維持する

この時点では `bench_correlation.rs`(S4)のため example のビルドは
壊れていてよい。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/al-math-api-boundary.md`(S2)
- `docs/designs/al-math-api-boundary.md`
- `docs/test-specs/al-math-api-boundary.md`(R4 独立参照比較の保持)

## Dependencies

- `docs/tasks/al-math-split-re-im-internals.md`(S1: split API が
  存在することが前提)

## Affected Files

- `crates/al-math/tests/fft.rs`
- `crates/al-math/tests/correlation_filter.rs`

## Done When

- [ ] tests 2 件が `al_math::c64` を参照せず split API で記述されて
  いる
- [ ] radix-2 `reference()`・naive DFT との直接比較が split 形で
  保持されている(削除していない)
- [ ] `power` ループ範囲とエッジ条件テストが維持されている
- [ ] `rejects_*` 系の panic 契約テストが split API で維持されている

## Verification

- [ ] `mise exec -- cargo test -p al-math --tests` が合格する
