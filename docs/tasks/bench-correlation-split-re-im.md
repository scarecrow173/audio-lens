---
id: "TASK-034DYiHj7TcoYoEFn29rMP"
type: "task"
status: "done"
title: "bench_correlation: split API 追従と previous_fft 腕の削除"
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

# bench_correlation: split API 追従と previous_fft 腕の削除

## Work

plan S4: `bench_correlation.rs` を split API に追従させる。S3 とは
担当ファイルが独立のため並行可能。

- `crates/al-math/examples/bench_correlation.rs`:
  - `previous_fft` 比較腕を削除する(c64 dispatch 版 FFT は公開
    API で表現不能になるため。設計は削除または書き換えを許容して
    おり、plan は削除を採用)
  - `legacy_bandpass` の `previous_fft` 使用分岐と `previous`/
    `previous_many` ベンチ、speedup 表示を整理する
  - 残る腕(`original_fft` 比較・現行 `filter`/`correlation`)を
    動作させる

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/al-math-api-boundary.md`(S4)
- `docs/designs/al-math-api-boundary.md`(`previous_fft` 腕の削除
  または書き換えの許容)

## Dependencies

- `docs/tasks/al-math-split-re-im-internals.md`(S1: split API が
  存在することが前提)

## Affected Files

- `crates/al-math/examples/bench_correlation.rs`

## Done When

- [ ] `previous_fft` 腕・`previous`/`previous_many` ベンチと
  speedup 表示が削除・整理されている
- [ ] `bench_correlation.rs` が `al_math::c64` を参照していない
- [ ] 残る腕(`original_fft` 比較・`filter`/`correlation`)が
  旧版比較に使える状態で動作する

## Verification

- [ ] `mise exec -- cargo test -p al-math` が合格する
- [ ] `mise exec -- cargo build -p al-math --examples` が成功する
