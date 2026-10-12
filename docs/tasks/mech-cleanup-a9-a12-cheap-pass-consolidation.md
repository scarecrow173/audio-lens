---
id: "TASK-034EO4qUmlyu3qleVAOHoo"
type: "task"
status: "done"
title: "機械的 caller クリーンアップ A9/A12: cheap 側 power 一括化・mean_row 融合・HashSet 事前計数"
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
    - "docs/plans/mechanical-caller-cleanup.md"
  implemented-by:
    - "docs/impl/ir/mech-cleanup-a9-a12-cheap-pass-consolidation.md"
  depends-on:
    - "docs/plans/mechanical-caller-cleanup.md"
    - "docs/tasks/mech-cleanup-a7-a8-csd-inlining.md"
  blocks:
    - "docs/tasks/mech-cleanup-a15-interleave-block-writes.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/mechanical-caller-cleanup.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/mechanical-caller-cleanup.md"
  references: []
  defers: []
  deferred-by: []
---

# 機械的 caller クリーンアップ A9/A12: cheap 側 power 一括化・mean_row 融合・HashSet 事前計数

## Work

plan T4 = design の A9A12-opt コミット(step 4)。
`cheap.rs` 側の pass 集約(A9)と `HashSet` 事前計数
(A12)を同一関数群内の機械的変更として束ねる。
A9 の `reference.rs:653-654` 側は T6(A16a と同一変更
単位)に含め、本タスクには入れない。

**A9 — `power()` 一括化(design sketch 通り)**:

- `cheap.rs:632` 付近で、`m` の move
  (`channel_stft.push(Some(m))`、`:665`)**より前に**
  全てを行う:
  - `let pow = m.power();` — 唯一の `mag_sq` pass
  - `let norm = (0.375 * m.window as f64) * m.sample_rate
    as f64;`(private `window_power()`、`stft.rs:41-44`
    と同じ形。`window`/`sample_rate` は pub field)
  - `let inv = 1.0 / norm;`・`let half = m.fft_size / 2;`
  - `mag`: `pow` の各行を `sqrt` して `Vec<Vec<f64>>`
    へ集めつつ、**`mean_row` 累積を融合**する(row
    昇順・`k < half`・0.0 始まり、最後に
    `*v /= mag.len() as f64` で除算 —
    `cheap.rs:794-796` の現行 `mean_row` と加算対象・
    加算順・除算が同一で bit-identical)
  - `psd`: `pow` の各行を `v * inv` で `Vec<Vec<f64>>`
    へ。**`v * inv`(reciprocal 乗算)のみ**。
    `/ norm` リテラルは禁止(ulp 差) — 現行 `psd()`
    の `scale(&power, 1.0/norm, …)` と同じ乗算なので
    bit-identical
- **`half` の重複定義を避ける**: 導入する
  `let half = m.fft_size / 2` と既存 `cheap.rs:668` の
  `let half = mag[0].len() / 2` は同一値(`mag` 行長 =
  `fft_size`)。`:668` 側を削除し push 前の `half` を
  再利用する(shadow・重複定義にしない)
- **ordering**: `mag`/`psd`/`mean_row` の導出と
  `m.fft_size`/`m.window`/`m.sample_rate` の読みは全て
  `:665` の `push(Some(m))` より前に完了させる
- `spec.slope` 以降の `mean_row` 消費点は変えない
- **単一消費者例外(変えない)**: `cheap.rs:942`
  `mm.magnitude()`、`reference.rs:1189` `rm.psd()`。
  `reference.rs:1071` の `m.magnitude()` は A16b(T6)
  で除去
- `StftMatrix` に新規メソッドを追加しない

**A12 — `HashSet<u64>` 事前計数(design sketch 通り)**:

- `cheap.rs:577-603` を、全サンプルの `v.to_bits()` を
  `std::collections::HashSet<u64>` へ集めた
  `distinct` へ置き換える
- `2 <= distinct.len() <= 1_000_000` の in-gate のみ
  `f64::from_bits` 実体化 → `sort_by(total_cmp)` →
  `windows(2)` 最小正差(`f64::INFINITY` 始まり
  `f64::min` fold)→ `quant`。gate 外では
  `f64::NAN` へ(両 sort・f64 実体化を完全 skip)
- support は `distinct.len() as u64`(現行 `uniq.len()`
  と同値)を `:603` へ使う
- **禁止**: 近似 distinct count(support が変わる)。
  `HashSet` は std のみ(新規依存なし)

**A12 実施結果 — bench 棄却(deviation)**: T1 baseline
(`target/bench-baseline/criterion-mech-cleanup-pre.txt`)で
HashSet 候補は両経路で現行 sort+dedup を下回った
(in-gate 7.29→14.17 ms、>1M 27.63→55.18 ms)。T4 の再計測
(`target/bench-baseline/quant_step-mech-cleanup-a9a12.txt`)
でも同傾向(6.44 vs 14.07 ms・26.74 vs 50.11 ms)を確認。
early-exit 変種は support(正確な distinct count)契約を
破り、近似判別子は spec の禁止事項のため、**コード不変更で
棄却**とした。判断の詳細・数値は IR
`docs/impl/ir/mech-cleanup-a9-a12-cheap-pass-consolidation.md`
を参照。

等価性ゲート: T1 の characterization(mag/power/psd
tensor・`spec.slope`・`dig.quant_step_est` の両経路
`to_bits`/record 一致)が opt 前後で green。

再計測: `quant_step`・`spectral_pass` criterion 腕を
`--baseline` で比較し、`bench_reference` 出力と併せて
before/after を `docs/impl/ir/` へ転記。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/mechanical-caller-cleanup.md`(T4)
- `docs/designs/mechanical-caller-cleanup.md`(A9/A12
  sketch・`half` 重複除去・mean_row 融合・ordering)
- `docs/specs/mechanical-caller-cleanup.md`(A9/A12 要件・
  禁止パターン)

## Dependencies

- `docs/tasks/mech-cleanup-bench-baseline-characterization.md`
  (T1: characterization + `quant_step`/`spectral_pass` 腕
  + baseline)
- `docs/tasks/mech-cleanup-a7-a8-csd-inlining.md`
  (T3: `cheap.rs` stereo `csd` の先行 opt — 直列適用)

## Affected Files

- `crates/al-analysis/src/cheap.rs`(`:632` 付近 pass
  集約・`:665` move 前 ordering・`:668` `half` 重複
  除去・`:794-796` `mean_row` 融合・`:577-603` HashSet)
- `docs/impl/ir/<record>.md`(A9/A12 before/after 転記)

## Done When

- [ ] `cheap.rs` で `m.power()` が唯一の `mag_sq` pass
  であり、`mag`/`psd`/`mean_row` がその 1 pass から
  導出されている
- [ ] psd 導出が `* inv`(`1.0/norm`)形のみで
  `/ norm` リテラルがない
- [ ] 全導出と `m` field 読みが `channel_stft.push(
  Some(m))`(`:665`)より前に完了し、`:668` の
  `half` 重複定義が除去されている
- [ ] `mean_row` が row-major pass へ融合され
  `*v /= mag.len() as f64` 除算を維持している
- [ ] `cheap.rs:577-603` が `HashSet<u64>` 事前計数 +
  in-gate のみ `total_cmp` sort で、gate 外は両 sort・
  f64 実体化を skip し、support が `distinct.len()`
- [ ] T1 の A9/A12 characterization が opt 前後両時点で
  green(in-gate・>1M skip 両経路)
- [ ] `quant_step`・`spectral_pass` 腕と
  `bench_reference` の before/after が IR に記録済み
- [ ] 本コミットが char・A6-opt・A7A8-opt 各コミット
  より後の git 履歴にある

## Verification

- [ ] `mise exec -- cargo test -p al-analysis cheap` が
  green(mag/pow/psd/`spec.slope`/`quant_step_est` 両経路)
- [ ] `rg '/ norm|/norm' crates/al-analysis/src/cheap.rs`
  が psd 派生経路にヒットしない(目視で `* inv` 形を
  確認)
- [ ] `rg 'm\.magnitude\(\)|m\.psd\(\)|m\.power\(\)'
  crates/al-analysis/src/cheap.rs` で `power()` が 1 回・
  `magnitude()`/`psd()` が対象ブロックに残らない
- [ ] `mise exec -- cargo bench -p al-analysis --bench
  quant_step --baseline <name>`・`--bench spectral_pass`
  で比較が実行できる
- [ ] `mise exec -- cargo clippy --workspace --all-targets
  -- -D warnings`・`mise exec -- cargo fmt --all --
  --check` が green
- [ ] `mise exec -- cargo test --workspace` が green
  (golden 含む)

## Out of Scope

- A9 の `reference.rs:653-654` psd→power 側(T6、A16a
  と同一変更単位)
- `dom()` の `m.magnitude()` 除去(T6 = A16b)
- `StftMatrix` への新規メソッド追加(禁止)
- 単一消費者例外 `cheap.rs:942`・`reference.rs:1189`
- A15(T5)・A16(T6)の他サイト
