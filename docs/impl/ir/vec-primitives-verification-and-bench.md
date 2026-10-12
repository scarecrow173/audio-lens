---
id: "IMPL-034EqSNHDxZ4VHxY5oaaaX"
type: "impl"
status: "completed"
title: "vec primitives verification and bench"
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
    - "docs/tasks/vec-primitives-verification-bench.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# vec primitives verification and bench

## Summary

`al_math::vec` 8 プリミティブの dispatch↔Scalar 等価性テスト(サイズ
行列 0..=33・1k・64k・1_048_576、predicate `|a-b| <= 1e-12 *
max(1,|a|,|b|)`)を `vec.rs` テストに追加し、criterion ベンチ
`crates/al-math/benches/vec_primitives.rs`(group `vec`、4 サイズ、
`Throughput::Bytes` 付き)を新設した。既存 `bench_*` examples の
置換前後比では ±5% を超える回帰なし(±2% 内または改善)。
`residual.rs` 減算ループの per-source `sub_scaled` 再構成は正しさ
ゲート・性能基準ともに合格したため **採用** し、列 gather dot も
解消した。

## Context

task `docs/tasks/vec-primitives-verification-bench.md`
(TASK-034EpPfi3haBLwJwM4xl66)、design
`docs/designs/al-math-vec-primitives.md`(テスト/ベンチ計画・
residual 計測プロトコル)、test-spec
`docs/test-specs/al-math-vec-primitives.md`(Rule D/E/H)、ADR
`docs/adr/simd-reduction-lane-fma.md`(等価性 predicate・サイズ
行列の権威)に従う plan T4。T1-T3(IMPL-034Eq5vi0HaWA9IMWHvhiG・
IMPL-034EqrFgTWTPu16W3CuG6r・IMPL-034EqFnpaWTKLnca2cw3Z3)の
移行完了コードを対象とする。

## Implementation

### Changes

- `crates/al-math/src/vec.rs` `#[cfg(test)] mod tests`:
  - `mixed_inputs(n)`: 負値・1e-2〜1e2 の混合桁を含む決定的入力
    (lane 順序差露出用、既存 `inputs` と同じ固定パターン流儀)
  - `assert_equiv`/`assert_equiv_slice`: 等価性 predicate 実装。
    経路間比較に `assert_eq!` は使わない
  - `dispatch_matches_scalar_for_scalar_outputs`: `dot`/`sum_sq`/
    `corr0`/`rms` を公開 wrapper(dispatch 経路)と
    `pulp::Scalar.vectorize(Kernel)`(Scalar 経路)で比較。
    サイズ行列 `(0..=33).chain([1024, 65536, 1_048_576])`
  - `dispatch_matches_scalar_for_vector_outputs`: `scale`/`axpy`/
    `sub_scaled`/`mag_sq` を要素毎 predicate で同様に比較
  - `corr0` も 1_048_576 で実行(所要時間は問題なしのため
    最大サイズの除外は行わなかった)
- `crates/al-math/benches/vec_primitives.rs`(新設)+
  `crates/al-math/Cargo.toml` に `[[bench]] name = "vec_primitives"
  harness = false` 登録(`dsp_smoke` は smoke のまま維持)。
  group `vec`、`BenchmarkId` で全 8 プリミティブ × サイズ
  32/1_024/65_536/1_048_576 を計測。reduction 系は入力バイト数、
  要素毎系は read+write バイト数で `Throughput::Bytes` を報告
- `crates/al-locate/examples/bench_residual.rs`(新設): residual
  減算ループの計測プロトコル用 targeted example。現行 gather
  ループと per-source `sub_scaled` 形の両変形を同一プロセス内で
  交互計測(REPS=15、median/min/p95)し、同一 fixture 上の要素毎
  predicate をゲートとして assert する
- `crates/al-locate/src/residual.rs`: 計測プロトコル合格のため
  `subtract_fit` の減算ループ(:163-169)を design 指定形
  `for (cap, row) in residual.iter_mut().zip(&est.matrix) { for
  (s, &m) in src.iter().zip(row) { dsp::sub_scaled(cap, m, s); } }`
  へ再構成。列 gather dot(旧 :166)も同再構成でのみ許容された
  解消として除去

### Decisions During Implementation

- **residual 再構成: 採用**(詳細は下記「residual 再構成の採否」)。
  正しさゲート(既存テスト green + 要素毎 predicate 内)と性能
  (±5% 超回帰なし)の両条件を満たした
- 等価性テストの入力は `mixed_inputs`(混合桁・負値・決定的)と
  した。1M でも全 8 プリミティブが predicate を満たすことを実測
  で確認(相殺の強い人工入力は避け、桁混合は 5 桁に留めた)
- `bench_residual` は sequential 計測(legacy 全 rep → reorg 全
  rep)だと drift に引きずられて偽の回帰(2ch×2src で
  +15〜+37%)を示したため、A/B 交互計測に変更して実測値を
  安定させた(計測手順の記録として IR に残す)

## Related Experiments

### Adopted

- `residual.rs` 減算ループの per-source `sub_scaled` 再構成 —
  下記計測値のとおり正しさ・性能の両ゲートを満たしたため採用

### Rejected

<!-- No rejected experiments. -->

## 計測結果

### criterion ベンチ(vec group、`cargo bench -p al-math`)

中央値。reduction は入力バイト、要素毎は read+write バイトを
分母とした `Throughput::Bytes`。

| primitive | n=32 | n=1_024 | n=65_536 | n=1_048_576 |
| --- | --- | --- | --- | --- |
| dot | 7.68 ns / 62.1 GiB/s | 208.5 ns / 73.2 | 15.30 µs / 63.8 | 641.6 µs / 24.4 |
| sum_sq | 4.85 ns / 49.2 | 187.9 ns / 40.6 | 14.14 µs / 34.5 | 254.4 µs / 30.7 |
| corr0 | 12.83 ns / 37.2 | 223.0 ns / 68.4 | 16.14 µs / 60.5 | 666.1 µs / 23.5 |
| rms | 6.94 ns / 34.4 | 189.3 ns / 40.3 | 14.15 µs / 34.5 | 258.1 µs / 30.3 |
| scale | 7.54 ns / 63.2 | 97.6 ns / 156.4 | 18.37 µs / 53.2 | 679.8 µs / 23.0 |
| axpy | 8.48 ns / 84.4 | 129.1 ns / 177.3 | 17.78 µs / 82.4 | 708.0 µs / 33.1 |
| sub_scaled | 7.98 ns / 89.6 | 126.2 ns / 181.4 | 18.00 µs / 81.4 | 706.9 µs / 33.2 |
| mag_sq | 10.07 ns / 71.0 | 117.0 ns / 195.6 | 23.73 µs / 61.7 | 1.167 ms / 20.1 |

観測: n=32 では per-call dispatch + call オーバーヘッドが支配的
(5〜13 ns/call、spec 想定どおり観測のみ・Non-goal)。1M では
メモリ帯域限界(~20-34 GiB/s)に達し SIMD 化の効果が飽和する。
完全な生出力は `target/bench-vec-primitives.txt`。

### 置換前後比(Instant benches、baseline → post)

`target/bench-baseline-*.txt`(移行前)対
`target/bench-post-*.txt`。`bench_dsp`/`bench_ncc` は 3 rep・
単発でノイズが大きく、初回 post で +8〜+20% を示した行が
2 回目 post(post-2)で全て ±4% 内に戻ったため、両サンプルを
併記する。同じ `filter::bandpass` 2.4M が bench_dsp では
+17.2%、bench_correlation では -0.6% と食い違うことからも
初回の偏りは環境ノイズと判断。

| line | baseline | post run1 | post run2 | 移行経路? |
| --- | --- | --- | --- | --- |
| **bench_dsp** | | | | |
| legacy fft 4M (same process) | 297.1 ms | 297.0 | 308.5 (+3.8%) | なし(対照) |
| fft 4M (next_pow2) | 150.6 | 158.2 | 155.5 (+3.3%) | なし(FFT) |
| complex fft 4M (reused plan) | 106.5 | 121.5 | 108.2 (+1.6%) | なし |
| rfft 4M (reused plan/scratch) | 69.0 | 82.8 | 60.9 (-11.7%) | なし |
| NCC 2.4M x 8k (fft corr) | 436.9 | 513.6 | 430.8 (-1.4%) | なし |
| bandpass 2.4M (spectral mask) | 186.5 | 218.5 | 182.0 (-2.4%) | なし |
| stft-equivalent psd 2.4M | 46.3 | 52.8 | 45.8 (-1.1%) | なし |
| gcc_phat 2.4M x 8k | 263.7 | 285.5 | 258.3 (-2.0%) | なし |
| **bench_ncc** | | | | |
| FullNcc | 341.3 ms | 420.0 | 340.1 (-0.4%) | preflight `sum_sq` |
| BandNcc | 2039.7 | 2036.1 | 1949.5 (-4.4%) | preflight/band `sum_sq` |
| GccPhat | 367.8 | 336.0 | 329.8 (-10.3%) | preflight `sum_sq` |
| SpecEnv | 521.3 | 468.7 | 471.7 (-9.5%) | `rms`+`scale`(normalize_gain)・preflight `sum_sq` |
| bandpass(sig) | 179.4 | 190.2 | 186.6 (+4.0%) | なし |
| **bench_correlation** | | | | |
| original radix-2 bandpass 2.4M | 550.2 ms | 540.3 (-1.8%) | — | なし(対照) |
| real spectral-mask bandpass 2.4M | 184.4 | 183.3 (-0.6%) | — | なし |
| five independent bands 2.4M | 936.4 | 953.1 (+1.8%) | — | なし |
| shared forward FFT five bands | 467.9 | 468.9 (+0.2%) | — | なし |
| NCC 2.4M x 8k | 274.2 | 271.5 (-1.0%) | — | なし |
| GCC-PHAT 2.4M x 8k | 261.7 | 257.3 (-1.7%) | — | なし |
| **bench_locate_prepared**(7 rep median) | | | | |
| prepare-cold-two-sources-and-capture | 908.4 ms | 905.5 (-0.3%) | — | `sum_sq`/`rms`/`scale` 経由 |
| band-ncc-uncached-two-pairs | 3552.1 | 3335.8 (-6.1%) | — | preflight/band `sum_sq` |
| band-ncc-prepared-two-pairs | 2777.3 | 2705.1 (-2.6%) | — | 同上 |
| coarse-ncc-two-pairs-proposal-only | 129.6 | 122.1 (-5.8%) | — | ncc(非 vec) |
| full-ncc-two-pairs | 612.7 | 598.9 (-2.3%) | — | preflight `sum_sq` |
| coarse-fine-and-global-evidence | 741.6 | 722.4 (-2.6%) | — | 同上 |
| cold-prepared-band-and-full | 4411.8 | 4371.9 (-0.9%) | — | 同上 |
| uncached-band-and-full | 4006.9 | 3964.0 (-1.1%) | — | 同上 |

判定: 移行経路を含む全行が ±5% 内か改善(最大 -10.3%)。
`bandpass(sig)` +4.0% は非移行行で閾値内。回帰フラグなし。
`max absolute response error` は 1.371e-11 で不変。

### residual 再構成の採否: **採用**

計測プロトコル(design L293-327)に従う。

1. **正しさゲート**: `bench_residual` example で同一 fixture 上の
   要素毎 `|a-b| <= 1e-12*max(1,|a|,|b|)` を確認 —
   2ch×1src: max_diff 2.220e-16 / max_ratio 2.220e-16、
   2ch×2src: max_diff 4.441e-16 / max_ratio 2.220e-16、いずれも
   PASS。`cargo test -p al-locate` 66 件 green
   (subtract_fit/prepare_residual テスト・golden 含む)
2. **性能**(release、A/B 交互計測 15 rep、frames=1_680_000
   ≒ 35s@48k stereo capture、median / run2 median):
   - capture 2ch × source 1ch: legacy 11.918 / 11.398 ms →
     reorg 9.780 / 9.666 ms(**-15〜-18% 改善**)
   - capture 2ch × source 2ch: legacy 13.562 / 13.644 ms →
     reorg 12.632 / 13.459 ms(**-7% / -1.4%**)
   - いずれの形状でも ±5% を超える回帰なし。gather 形は要素毎に
     `src.len()` 分の動的長 zip を回すスカラー内ループで SIMD 化
     できないが、per-source 形は `sub_scaled` の連続ストリーム化で
     source 毎の余分な cap 再走査を補って速い
3. **計測ノイズ注意**: sequential 計測(変形ごとに rep まとめ)では
   2ch×2src で +15〜+37% の偽回帰を示した(run 間 drift)。
   A/B 交互化で消失。結論は交互計測値に基づく
4. **結果**: 採用条件(ゲート合格 + ±5% 超回帰なし)を満たすため
   採用。列 gather dot(旧 :166)は再構成に伴い解消。計測出力は
   `target/bench-residual.txt`、測定器は
   `crates/al-locate/examples/bench_residual.rs` として残す

## 許容比較・数値差分の記録(T2/T3 引継ぎ分含む)

- **float 厳密→許容比較の修正一覧**: T2 の
  `crates/al-locate/tests/locate.rs:327`(`assert_eq!` →
  `abs() < 1e-15`)のみ。T3・T4 での新規修正はなし
- **`normalize_gain` の除算→逆数乗算**(T2、methods.rs:497-502):
  `v / rms` が `dsp::scale(x, r.recip())` により `v * r.recip()`
  となり ulp 級差が生じ得る(再構成許容ポリシーで許容済み)
- **envelope NCC 残存許容逸脱**(T2、methods.rs:539-547):
  `dsp::corr0` 化は周期入力で全 lag が厳密同点(≡1.0)になり
  argmax タイブレークが lag 0 に倒れて pinned golden の曖昧性
  棄却を破壊するため、順序が累算順の ulp に本質依存する当サイトは
  逐次式を維持(残存許容)。詳細は
  IMPL-034EqrFgTWTPu16W3CuG6r
- **residual 減算ループの累算順変更**(本タスク): `v -= (m0·s0[t]
  + m1·s1[t])` 一括減算 → source 毎 `v -= m_s·s[t]` 逐次減算。
  `ResidualEnergy.after` 等の永続化値は ~ulp 級に揺れ得る
  (ADR 許容済み、既存テスト全件非回帰)

## Validation

- `mise exec -- cargo test -p al-math`: vec.rs テスト 14 件 pass
  (等価性 2 件を含む。サイズ行列 0..=33・1k・64k・1_048_576、
  検証経路は pulp features `["std","x86-v3"]` の x86-v3・Scalar)
- `mise exec -- cargo bench -p al-math`: `vec` group が全 8
  プリミティブ × 4 サイズで実行可能、reduction は
  `Throughput::Bytes` 報告
- `mise exec -- cargo test --workspace`: 500 passed / 0 failed
  (golden・Validity ゲート非回帰)
- `mise exec -- cargo build --workspace`: green
- `mise exec -- cargo fmt --all -- --check`: green
- `mise exec -- cargo clippy --workspace --all-targets -- -D
  warnings`: green
- `cargo test -p al-locate`: 66 件 pass(residual 再構成後)

## Risks

- Instant bench は 3 rep・単発の行が大きなノイズを持つ環境
  (+20% 級の run 間変動を観測)であり、置換前後比の判定は
  複数サンプル・median 値に基づく。閾値近傍の行が将来の再計測で
  逆転し得る
- `bench_residual` の代表形状は 2ch×{1,2}src。より多チャンネルの
  source(例: 4ch)では cap 再走査コストが線形に増えるため、
  形状によっては優位が縮小し得る(現行契約の実チャンネル範囲では
  採用判定の範囲内)

## Follow-ups

- T2 からの持越し: design file map への envelope NCC 残存許容
  追記提案、および合議層の手法内曖昧性(同点 argmax)非検出の
  構造的問題(別イシュー候補)
- n=32 での per-call dispatch コスト(~5-13 ns)は spec Non-goal
  として観測のみ。小スライス頻呼出し経路が将来問題になる場合は
  呼出し側 batch 化を検討
