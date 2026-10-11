---
id: "TASK-034DYiGC8yp76ElLB8R0J5"
type: "task"
status: "done"
title: "al-locate: dsp::fft と bench_dsp の split API 追従"
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

# al-locate: dsp::fft と bench_dsp の split API 追従

## Work

plan S3: `al-locate` 側の FFT 境界利用を split API に追従させ、
`dsp::fft` が呼出しごとに行う `Vec<c64>` 確保と往復コピーを除去
する。S4 とは担当ファイルが独立のため並行可能。

- `crates/al-locate/src/dsp.rs`: `fft(re, im, inverse)` 内の
  `Vec<c64>` 詰め替えを削除し、`al_math::fft::Fft::new(re.len())`
  の `transform(re, im, inverse)` を直接呼ぶ。`al_math::c64` 参照を
  除去する。`dsp::fft` のシグネチャ `fft(re, im, inverse)` は維持
  する(ラッパ自体の再設計は scope 外)
- `crates/al-locate/examples/bench_dsp.rs`: `complex fft 4M` 腕を
  re/im バッファへの直接変換に、`rfft 4M` 腕の `spectrum` を
  re/im ペアに書き換え、`al_math::c64` 参照を除去する

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/al-math-api-boundary.md`(S3)
- `docs/designs/al-math-api-boundary.md`

## Dependencies

- `docs/tasks/al-math-split-re-im-internals.md`(S1: split API が
  存在することが前提)

## Affected Files

- `crates/al-locate/src/dsp.rs`
- `crates/al-locate/examples/bench_dsp.rs`

## Done When

- [ ] `dsp::fft` 内の `Vec<c64>` pack/unpack が除去され、
  `Fft::transform(re, im, inverse)` を直接呼んでいる
- [ ] `dsp.rs`/`bench_dsp.rs` に `al_math::c64` 参照が残っていない
- [ ] `dsp::fft` の公開シグネチャが変更されていない

## Verification

- [ ] `mise exec -- cargo test -p al-locate` が合格する
- [ ] `mise exec -- cargo build -p al-locate --examples` が成功する
