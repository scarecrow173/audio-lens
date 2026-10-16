---
id: "TASK-034GMMJwLQfp3HEnhqRERT"
type: "task"
status: "done"
title: "CorrelationPool 新設(形状キー plan/workspace 再利用)"
created: "2026-10-16"
updated: "2026-10-16"
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
    - "docs/plans/locate-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/locate-plan.md"
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
    - "docs/test-specs/locate-plan-pool-inputcache-coarse-to-fine.md"
  references: []
  defers: []
  deferred-by: []
---

# CorrelationPool 新設(形状キー plan/workspace 再利用)

## Work

ワーカー単位で所有する形状キー相関プールを `crates/al-locate/src/pool.rs` に新設する。

- `HashMap<(template_len, signal_len), Entry>`、Entry は `CorrelationPlan` + `CorrelationWorkspace` を1組保持
- `ncc` / `gcc_phat` は借用曲線 `&[f64]` を返し、`.to_vec()` 複製を消す
- `MAX_SHAPES = 8`、超過時は全エントリ clear の単純戦略
- `builds` / `hits` の `PoolStats` を公開
- `lib.rs` で `pub mod pool` を登録

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/locate-plan.md`
- `docs/specs/locate-plan-cache.md`

## Dependencies

- `docs/plans/locate-plan.md`

## Affected Files

- `crates/al-locate/src/pool.rs`
- `crates/al-locate/src/lib.rs`

## Done When

- [x] 同一形状が build 1 回で再利用され、出力が `dsp::ncc`/`dsp::gcc_phat` と完全一致する
- [x] 形状数が `MAX_SHAPES` を超えない
- [x] builds/hits が計測できる

## Verification

- [x] `cargo test -p al-locate pool::tests`
- [x] `cargo clippy -p al-locate --all-targets -- -D warnings`
