---
id: "TASK-034DYiz9Z4rXXkKoNvy4vI"
type: "task"
status: "done"
title: "al-math: fft/correlation/filter の split re/im 内部化"
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
  blocks:
    - "docs/tasks/al-math-split-re-im-tests.md"
    - "docs/tasks/al-locate-split-re-im-fft.md"
    - "docs/tasks/bench-correlation-split-re-im.md"
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

# al-math: fft/correlation/filter の split re/im 内部化

## Work

plan S1(分割不能な統合スライス): `al-math` 内部を split re/im 化する。
同一 lib ターゲット内で `fft.rs` の `pub(crate)`
`forward_with_simd`/`inverse_with_simd` を `correlation.rs`/
`filter.rs` が `Vec<c64>` バッファで呼んでいるため、3 ファイルを
分割せず 1 スライスで書き換える。

- `fft.rs`:
  - `roots()` を `(Vec<f64>, Vec<f64>)` ペアにし、`Fft` の twiddle
    を `twiddles_re`/`twiddles_im` に split 格納する
  - `Fft::transform` を `(&self, re: &mut [f64], im: &mut [f64],
    inverse: bool)` に変更し、`re.len() == im.len() == self.n` を
    assert する。`Transform` カーネル構造体と内部 `transform()` を
    追従させる
  - bit-reversal の `values.swap` 2 箇所(単純ループ・n>=1024
    タイル化)を `re.swap`+`im.swap` に、inverse 1/N スケールを
    re/im 各配列の `mul_f64s` に書き換える
  - `butterfly`/`stage` を re/im 引数 + split twiddles に書き換える。
    `mul_c64s` は `(xr·wr − xi·wi, xr·wi + xi·wr)`、`conj_c64s` は
    虚部 negate として実部演算に分解し、FMA 融合は
    `mul_add_f64s`/`negate_mul_add_f64s` で現行の丸め形を可能な範囲で
    再現する。`len == 2` 経路と scalar tail も同様
  - `RealFft` の `roots`/`scratch` を re/im ペアにし、
    `forward(input, out_re, out_im)` / `inverse(in_re, in_im,
    output)` に変更する。長さ契約(input=n、spectrum=n/2+1)と
    assert を維持し、`in_re`/`in_im` の同一スライス共有禁止を
    debug assert で明示する
  - `forward_with_simd`/`inverse_with_simd` を同形の split
    シグネチャに変更(`pub(crate)` と選択済み ISA の複合カーネル
    共有は維持)。`RealBuffer`/`SpectrumBuffer`/`RealTransform` を
    re/im ペア構造に書き換える
  - `RealTransform::with_simd` 本体を実部演算に書き換える:
    2点詰め込み/展開はストライドコピー、DC/Nyquist 組立てと conj
    配置は re/im 演算、`recombine_pair`/`recombine` は実部演算化。
    `low`/`high` は re/im 両配列で split・prefill・reverse を対で
    行う
  - `use crate::{...}` から `c64` を除去し、モジュール内テスト
    `scalar_and_dispatch_share_the_kernel` を split API 形にする
- `correlation.rs`: `a`/`b` を `Vec<f64>` re/im ペアにし、
  `cross_spectrum` を `mul(conj(a), b)` = `(ar·br + ai·bi,
  ar·bi − ai·br)` の実部演算に、PHAT の `magnitude` を energy
  配列(`re² + im²`)にして `bytemuck::cast_slice_mut` 2 箇所を
  除去、同一 energy で re/im に `normalize` する。`use` から
  `c64` を除去
- `filter.rs`: `spectrum`/`masked` を re/im ペアにし、band ループの
  `fill`/`copy_from_slice` を両配列に、`spectral_envelope` の
  `magnitudes` を `Vec<f64>` 化して `abs2_c64s` を `re² + im²` に、
  `cast_slice_mut` 1 箇所を除去。`use` から `c64` を除去

この時点では integration test/example/下流 crate のビルドは壊れて
いてよい。`--lib` で lib ターゲットのみ確認する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/al-math-api-boundary.md`(S1)
- `docs/designs/al-math-api-boundary.md`(正本: 公開シグネチャ・
  内部表現・FMA 方針・誤差閾値)
- `docs/adr/al-math-api-fft-split-re-im.md`

## Dependencies

- なし(先行タスクなし。S2・S3・S4 を block する起点スライス)

## Affected Files

- `crates/al-math/src/fft.rs`
- `crates/al-math/src/correlation.rs`
- `crates/al-math/src/filter.rs`

## Done When

- [ ] `Fft`/`RealFft`/`forward_with_simd`/`inverse_with_simd` が
  split re/im シグネチャになり、twiddles/scratch/roots が split
  格納されている
- [ ] `correlation.rs`/`filter.rs` のスペクトラムバッファが re/im
  ペアになり、`bytemuck::cast_slice_mut` 3 箇所が除去されている
- [ ] 両ファイルの `use` から `c64` が除去されている
- [ ] 関数単位の置き換えごとに `--lib` テストで scalar/dispatch
  一致(1e-10 相対)を確認した

## Verification

- [ ] `mise exec -- cargo test -p al-math --lib` が合格する
  (FFT 系・correlation/filter の scalar/dispatch 一致テストを含む)
