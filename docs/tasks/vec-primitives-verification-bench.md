---
id: "TASK-034EpPfi3haBLwJwM4xl66"
type: "task"
status: "done"
title: "vec プリミティブ等価性検証・criterion ベンチ・residual 再構成判定"
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
    - "docs/impl/ir/vec-primitives-verification-and-bench.md"
  depends-on:
    - "docs/plans/al-math-vec-primitives.md"
    - "docs/tasks/al-analysis-vec-migration.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/al-math-vec-primitives.md"
    - "docs/specs/al-math-vec-primitives.md"
    - "docs/adr/simd-reduction-lane-fma.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/al-math-vec-primitives.md"
  references: []
  defers: []
  deferred-by: []
---

# vec プリミティブ等価性検証・criterion ベンチ・residual 再構成判定

## Work

dispatch↔Scalar 等価性テスト、criterion ベンチ新設、置換前後比
計測、および `residual.rs` 減算ループ再構成の計測・採否判定を
行う(plan T4)。

- `crates/al-math/src/vec.rs` の `#[cfg(test)] mod tests` に
  dispatch↔Scalar 等価性テストを追加する
  (`docs/adr/simd-reduction-lane-fma.md` が権威):
  - スカラー出力(`dot`/`sum_sq`/`corr0`/`rms`):
    `|a - b| <= 1e-12 * max(1.0, |a|, |b|)`。ベクトル出力
    (`scale`/`axpy`/`sub_scaled`/`mag_sq`): 同 predicate を
    要素毎に適用。経路間比較に `assert_eq!` は使わない
  - サイズ行列: 0..=33(空・1要素・lane 端数の全経過)・
    1k・64k・~1M(1_048_576)。負値・混合桁を含む決定的入力
    で lane 順序差を露出させる
  - 経路の取り方は lib.rs 既存テストのパターン:
    Scalar 経路は `pulp::Scalar.vectorize(Kernel{...})`、
    dispatch 経路は `dispatch(Kernel{...})`。本ビルドの pulp
    features は `["std","x86-v3"]` で、検証経路は x86-v3 と
    Scalar のみ
- criterion ベンチ新設 `crates/al-math/benches/vec_primitives.rs`
  + `crates/al-math/Cargo.toml` に `[[bench]] name =
  "vec_primitives" harness = false`(criterion dev-dep は
  observability foundation で導入済み):
  - criterion group `vec`、`BenchmarkId` で全 8 プリミティブを
    計測。reduction は `Throughput::Bytes`(GB/s)を報告
  - サイズ: 32(小ベクトル・per-call dispatch コスト)、
    1_024、65_536、1_048_576(methods.rs preflight 全長
    energy 相当)。決定的入力(sin/LCG)+ `black_box`。
    `dsp_smoke` は smoke のまま維持
- 置換前後比: 既存 `crates/al-locate/examples/bench_*`
  (`bench_dsp`・`bench_ncc`・`bench_locate_prepared`)の
  `Instant` 計測で代表呼出し経路(methods.rs preflight
  energy、band_gain.rs window dot、cheap.rs corr0 lag 走査、
  residual.rs 減算ループ)の置換前後を同一条件で計測し、
  criterion ノイズ幅(目安 ±5%)を超える回帰がないことを確認
- `residual.rs` 減算ループ(:163-169)条件付き再構成の計測
  プロトコル(design L293-327):
  1. 正しさゲート: 既存 subtract_fit/prepare_residual テスト
     green + 同一 fixture で要素毎 `|a-b| <= 1e-12 *
     max(1.0,|a|,|b|)` を満たすこと(一時比較コード/テストで
     確認)
  2. 性能: `bench_locate_prepared` で代表入力(実際の
     capture チャンネル × source チャンネル)の `Instant` 計測
  3. 採用条件: ゲート合格かつ ±5% を超える回帰なし →
     `for (cap, row) in residual.iter_mut().zip(&est.matrix)
     { for (s, &m) in src.iter().zip(row) { dsp::sub_scaled(cap,
     m, s); } }` 形へ再構成し、列 gather dot(:166)も
     プリミティブへ解消。回帰・正しさ不合格なら現行ループを
     保持し列 gather dot は残存許容のまま
  4. 採否いずれの場合も計測値と根拠を Implementation Record
     に記録(spec AC)

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/al-math-vec-primitives.md`
- `docs/specs/al-math-vec-primitives.md`(等価性検証・bench・
  residual 判定の Acceptance Criteria)
- `docs/designs/al-math-vec-primitives.md`(テスト計画・ベンチ
  計画・計測プロトコル L293-381)
- `docs/adr/simd-reduction-lane-fma.md`(等価性 predicate・
  サイズ行列の権威)

## Dependencies

- `docs/tasks/al-analysis-vec-migration.md`(T3): 置換前後比・
  residual 判定は移行完了後のコードで実施する
  (推移的に T1・T2 に依存)

## Affected Files

- `crates/al-math/src/vec.rs`: 等価性テスト追加
- `crates/al-math/benches/vec_primitives.rs`(新設)・
  `crates/al-math/Cargo.toml`(`[[bench]]` 追加)
- `crates/al-locate/src/residual.rs`: 採用時のみ per-source
  `sub_scaled` 再構成
- Implementation Record(`docs/impl/ir/`): 等価性・bench
  実測値・置換前後比・residual 採否根拠・許容比較修正一覧の
  記録先

## Done When

- [ ] dispatch↔Scalar 等価性テストがサイズ行列(0..=33・1k・
  64k・1_048_576)で pass する
- [ ] `mise exec -- cargo bench -p al-math` で `vec` group の
  vec_primitives ベンチが 4 サイズで実行できる
- [ ] 置換前後比・ベンチ実測値・residual 再構成の採否と根拠が
  Implementation Record に記録される
- [ ] `mise exec -- cargo test --workspace` が golden・
  Validity ゲートを含めて green

## Verification

- [ ] `mise exec -- cargo test -p al-math` で等価性テストを含む
  全テストが pass
- [ ] `mise exec -- cargo bench -p al-math` で vec ベンチが実行
  できる
- [ ] `mise exec -- cargo test --workspace`・`mise exec --
  cargo clippy`・`cargo fmt --check` が green
