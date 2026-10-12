---
id: "TASK-034EO4gPvPkx3voaC6KOlY"
type: "task"
status: "todo"
title: "機械的 caller クリーンアップ A6: band_correlation の bandpass_many バッチ化"
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
  implemented-by: []
  depends-on:
    - "docs/plans/mechanical-caller-cleanup.md"
    - "docs/tasks/mech-cleanup-bench-baseline-characterization.md"
  blocks:
    - "docs/tasks/mech-cleanup-a7-a8-csd-inlining.md"
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

# 機械的 caller クリーンアップ A6: band_correlation の bandpass_many バッチ化

## Work

plan T2 = design の A6-opt コミット(step 2)。
`reference.rs:742-747` の `bands.iter().take(8)` ループ +
16 回の `bandpass` 呼出し(src/cap × 8 band)を、
bound-safe な cap slice + `bandpass_many` 2 呼出しへ
置き換える。

- 置換形状(design sketch 通り):
  `let capped = &bands[..bands.len().min(8)];` →
  `bandpass_many(src_mono, rate, capped)`・
  `bandpass_many(cap_mono, rate, capped)` →
  `xs.iter().zip(&ys).map(|(x, y)| corr0(x, y))` で
  band 順に `band_corrs` を組み立てる
- **禁止**: `&bands[..8]` リテラル(`bands < 8` で
  panic — 現行 `take(8)` は安全に退化する)。必ず
  `bands.len().min(8)` で bound する
- **等価の根拠**: `bandpass` は `bandpass_many(..,
  &[(lo, hi)])` `.pop()` の thin wrapper
  (`crates/al-math/src/filter.rs:7-11`)で、mask 規則・
  切り戻し・出力順が同一 → `band_corrs` は
  bit-identical
- **依存**: `bandpass_many` は `al_locate::dsp` から
  re-export 済み(`dsp.rs:34`)。`use` リストへの追加のみ
- **import prune(同一コミット必須)**: 置換で
  `reference.rs:13` の `use ... bandpass` が未使用化し
  clippy `-D warnings` で fail するため、`bandpass` を
  `bandpass_many` へ入れ替える。`al-math`/`dsp.rs` 側の
  `bandpass` 公開面自体は残す(consumer file の `use`
  記述のみ削る)
- 等価性ゲート: T1 の characterization(band_correlation
  tensor 要素 `to_bits` + `bands < 8` edge)が opt 前後で
  green
- 再計測: `band_correlation` criterion 腕を
  `--baseline <T1 名>` で比較、`bench_reference` 出力を
  `target/bench-baseline/` と比較し、plan/forward 16→2
  を含む before/after を `docs/impl/ir/` へ転記
  (opt コミットまたは直後の docs コミット)

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/mechanical-caller-cleanup.md`(T2)
- `docs/designs/mechanical-caller-cleanup.md`(A6 sketch・
  import prune・計測方法)
- `docs/specs/mechanical-caller-cleanup.md`(A6 要件・禁止
  パターン)

## Dependencies

- `docs/tasks/mech-cleanup-bench-baseline-characterization.md`
  (T1: A6 characterization + `band_correlation` 腕 +
  baseline)

## Affected Files

- `crates/al-analysis/src/reference.rs`(`:742-747` 置換 +
  `:13` use リスト prune)
- `docs/impl/ir/<record>.md`(A6 before/after 転記)

## Done When

- [ ] `reference.rs` の band_correlation 構築が
  `bandpass_many` ちょうど 2 呼出し(src/cap)+
  `bands.len().min(8)` bound + band 順 `corr0` zip の
  形状になっている
- [ ] `reference.rs` に `bandpass(` 呼出しが残らず、
  use リストの `bandpass` が `bandpass_many` へ入替済み
- [ ] T1 の A6 characterization(tensor `to_bits` +
  `bands < 8` edge)が opt 前後両時点で green
- [ ] `band_correlation` 腕と `bench_reference` の
  before/after が IR に記録されている
- [ ] 本コミットが T1 の char コミットより後の git
  履歴にある

## Verification

- [ ] `mise exec -- cargo test -p al-analysis --test
  reference` が green
- [ ] `rg 'bandpass\(' crates/al-analysis/src/reference.rs`
  が 0 件、`rg 'bandpass_many' crates/al-analysis/src/
  reference.rs` が use + 2 呼出しを示す
- [ ] `rg 'bands\[\.\.8\]' crates/` が 0 件(`.min(8)`
  bound のみ)
- [ ] `mise exec -- cargo bench -p al-analysis --bench
  band_correlation -- --baseline <name>` で比較が実行
  できる
- [ ] `mise exec -- cargo clippy --workspace --all-targets
  -- -D warnings`・`mise exec -- cargo fmt --all --
  --check` が green(未使用 import が残らない)
- [ ] `mise exec -- cargo test --workspace` が green
  (golden 含む)

## Out of Scope

- A7/A8(`csd` インライン化、T3)・A9 以降の全項目
- `al-math`/`dsp.rs` の `bandpass` 公開面の変更
- `reference.rs` の他サイトへの手出し
