---
id: "IMPL-034EJHfBnQsZuebjx84T7X"
type: "impl"
status: "completed"
title: "caller-side optimizations: bench baseline and characterization"
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
    - "docs/tasks/caller-opt-bench-baseline-characterization.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/caller-side-optimizations.md"
    - "docs/test-specs/caller-side-optimizations.md"
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

# caller-side optimizations: bench baseline and characterization

## Summary

plan T1(design T0 + A1-char・A3-char・A4-char の集約)。全ての最適化
適用に先立ち、criterion ベンチ基盤・Instant ベンチ・
characterization テスト・`subtract` zero-fill 修正を land し、
baseline 実測を記録した。最適化コード(`dsp::conv`・`pub use
Fft`・`from_columns`・`fir_model` 内部の畳み込み置換)は一切
含まない — 本タスクは測定・捕捉基盤のみ。

## Context

task `docs/tasks/caller-opt-bench-baseline-characterization.md`
(TASK-034EJgkPQJmqrMLNdP7lVh)、design
`docs/designs/caller-side-optimizations.md`(T-baseline・
切り出しシグネチャ・コミット順序)、test-spec
`docs/test-specs/caller-side-optimizations.md`(Rule A-F)に従う。
等価性 predicate は ADR-034EoHxzsvsHp9BOdCK2Cn の
`|a-b| <= 1e-12*max(1,|a|,|b|)` を使用。後続の T2–T4 最適化
タスクは本 baseline と characterization に依存する。

## Implementation

### Changes

- `crates/al-analysis/Cargo.toml`: criterion `=0.8.2` dev-dep
  (al-math と同 version・pattern、依存追加として本 IR に記録)+
  `[[bench]] name = "stft" / "conv" harness = false` stanza
- `crates/al-analysis/benches/stft.rs` 新設: group `stft`、
  window/hop 現行設定 4096/2048 固定、決定的 LCG 入力で
  n = 524_288 / 1_048_576 を計測
- `crates/al-analysis/benches/conv.rs` 新設: `conv_direct`
  (design 指定の逐語同一コピー、`x == 0.0` skip 除去)の
  baseline 腕のみ。`dsp::conv` は T4 まで存在しないため腕なし。
  O(n·m) のため n ∈ {65_536, 262_144} × m ∈ {256, 4_096} に
  有界化し `sample_size(10)` + `measurement_time(2s)` を設定
- `crates/al-math/Cargo.toml` + `benches/lstsq.rs`: `[[bench]]
  name = "lstsq"`。rows API(`LeastSquares::new` + `solve`)を
  列数 {1, 2, 4} × 観測数 {65_536, 262_144, 1_048_576} で計測
  (`new`/`solve` 別 arm)。`vec_primitives.rs` とは別 target
- `crates/al-analysis/examples/bench_reference.rs` 新設:
  al-locate `bench_*` examples の Instant ベンチ規約に倣い、
  `stft`・有界 synthetic `analyze_pair`(2s @48k mono、R0–R3・
  fir_model 経路)・`prepare_residual`(A4 前後でシグネチャ不変
  の caller 級計測点)を計測
- `crates/al-analysis/src/reference.rs`: `analyze_pair` 内の
  `fir_model`/`subtract` クロージャを module 内名前付き関数
  `fir_ir`/`fir_model`/`subtract` へ挙動不変で切り出し(design
  指定シグネチャ。`fir_ir` は CSD 累積〜irfft まで、`conv_len`
  キャップと `fr == 0` 後の無条件畳み込みは `fir_model` 側)。
  `subtract` を `m[i.min(m.len())]`(ragged で panic)から
  `m.get(i).copied().unwrap_or(0.0)` の zero-fill へ修正。
  `mod tests` に `conv_direct` 参照実装・predicate ヘルパ・
  `fir_model` characterization(空・n<conv_len・conv_len=4096
  境界・全ゼロ・LCG ノイズ)・`subtract` ragged fixture テスト追加
- `crates/al-analysis/src/stft.rs`: `mod tests` にビット一致
  characterization — 各フレームを test 内で独立に window +
  `dsp::fft`(single-shot)した参照と `f64::to_bits` 比較。
  fixture: 複数フレーム・`samples.len() < window`(0 フレーム)・
  単一フレーム・非2冪 window・全ゼロ。`frames`/`fft_size`/
  `window`/`hop` メタも照合
- `crates/al-locate/src/mixer.rs`: `mod tests` に `fit_mix`
  characterization — `MixerEstimate.matrix`・`fit_quality` を
  test 内行ベース normal-equations oracle(`linalg.rs` `mod
  legacy` と同型、自己完結)と predicate 照合。
  `condition_number` は INFINITY 整合 + 位数(log10 差 ≤0.5)の
  sanity bound のみ

### Decisions During Implementation

- **`condition_number` は singular fixture でも「huge or
  infinite」sanity に留めた**: 実測で、完全に同一な複製 source
  列でも `condition_number` が INFINITY にならないケース
  (seed 200 → 1.77e8)を確認した。Gauss-Jordan `invert` の
  pivot 閾値判定が `x·(1/x)` の ulp に依存するため。A4 で
  AᵀA 集計が列 `dot` 化されると ulp 揺れで INF/有限が
  揺れ得るため、spec どおり sanity bound のみとした
- **`fir_ir` の `fr == 0` 早期 return**: design どおり
  `fir_ir` 内に固定し、長さ `fft_size` のゼロ IR を返す。
  `fir_model` は無条件に畳み込み、結果は `vec![0.0; n]` で
  旧 early return と等価
- **`subtract` zero-fill は `map_or` 外形を維持**:
  `model.get(c).map_or(0.0, |m| m.get(i).copied().unwrap_or(0.0))`
  とし、指定置換を最小差分で適用(旧 `m[i.min(m.len())]` は
  `i >= m.len()` で `m[m.len()]` へ index して panic する
  実質バグだった)
- **`bench` 実行は `--bench <name>` 単位**: `cargo bench -p
  al-analysis -- --save-baseline` は lib unittest harness にも
  引数が渡り `Unrecognized option` で失敗する。`--bench stft`・
  `--bench conv` を別実行して baseline `pre` を保存した

## Related Experiments

### Adopted

<!-- No experiment logs: mechanical task per task-doc. -->

### Rejected

<!-- No rejected experiments. -->

## Baseline 計測結果(pre-optimization)

環境: 同一マシン上の `mise exec -- cargo bench --benches`
(release + debuginfo)、criterion `--save-baseline pre` /
`lstsq-rows` は `target/criterion/` 配下、Instant/完全出力は
`target/bench-baseline/` 配下(ともに gitignore 済み)。

### criterion — stft(group `stft`、window/hop 4096/2048、baseline `pre`)

| 信号長 | time (median) |
| --- | --- |
| 524_288 | 13.840 ms |
| 1_048_576 | 26.973 ms |

### criterion — conv_direct(group `conv`、baseline `pre`)

| n | m=256 | m=4_096 |
| --- | --- | --- |
| 65_536 | 4.1349 ms | 62.115 ms |
| 262_144 | 16.576 ms | 254.28 ms |

`dsp::conv` 腕は T4 で追加し同一 fixture・baseline `pre` と A/B。

### criterion — lstsq rows API(group `lstsq`、baseline `lstsq-rows`)

`LeastSquares::new`(構築)/ `solve`(求解)、median:

| 列数 x 観測数 | new | solve |
| --- | --- | --- |
| 1 x 65_536 | 4.4535 ms | 750.53 µs |
| 1 x 262_144 | 19.715 ms | 4.0318 ms |
| 1 x 1_048_576 | 83.266 ms | 20.099 ms |
| 2 x 65_536 | 6.2268 ms | 1.0319 ms |
| 2 x 262_144 | 29.384 ms | 5.5069 ms |
| 2 x 1_048_576 | 123.45 ms | 22.260 ms |
| 4 x 65_536 | 10.808 ms | 2.4524 ms |
| 4 x 262_144 | 50.211 ms | 10.674 ms |
| 4 x 1_048_576 | 215.23 ms | 43.263 ms |

T3(列 API 置換)時は同一 fixture で列シグネチャへ追従し
baseline `lstsq-rows` と比較する。

### Instant — bench_reference(`target/bench-baseline/bench-reference.txt`)

| 計測点 | mean (3 rep) |
| --- | --- |
| stft 1M samples (4096/2048) | 35.7 ms |
| analyze_pair 2s mono (R0-R3, fir_model) | 691.2 ms |
| prepare_residual 2s 2src x 2cap | 19.3 ms |

## Validation

- `mise exec -- cargo test -p al-math -p al-locate -p al-analysis`:
  green(新規 characterization 含む。`fir_model_matches_
  fir_ir_truncated_conv_direct`・`subtract_zero_fills_short_
  or_missing_model`・`stft_is_bit_exact_*` ×5・
  `fit_mix_matches_normal_equations_oracle`)
- `mise exec -- cargo test --workspace`: green(39 suite、
  golden テスト非回帰)
- `mise exec -- cargo build --workspace`: green
- `mise exec -- cargo fmt --all -- --check`: green
- `mise exec -- cargo clippy --workspace --all-targets -- -D
  warnings`: green
- `mise exec -- cargo bench -p al-analysis -- --list`: stft・conv
  ベンチが列挙される
- `mise exec -- cargo bench -p al-math --bench lstsq`: 完走、
  `--save-baseline lstsq-rows` で baseline 保存済み
- `mise exec -- cargo run --example bench_reference --release -p
  al-analysis`: 完走、出力は
  `target/bench-baseline/bench-reference.txt`
- `mise exec -- cargo tree -p al-analysis --depth 1`: 直接依存に
  al-math なし(al-core・al-dataset・al-job・al-locate・
  al-media・serde・serde_json・tracing;dev-deps は
  al-testkit・criterion)

## Risks

- `conv_direct` bench は debug ビルドではなく bench profile
  (release+debuginfo)で記録。同一 profile での再計測のみ
  before/after 比較に有効
- Instant ベンチは 3 rep 平均・単発のため ±5% 以上の環境ノイズを
  含み得る(VEC-IR 先例では ±20% 級の run 間変動あり)。
  回帰判定は複数サンプルで確認すること
- `fir_model` characterization は切り出し直後・`dsp::conv`
  置換後の両時点で同一テストが green である前提。A4 後の
  `fit_mix` characterization は列シグネチャへの追従が必要
  (T3 で test 内 fixture も列化)

## Follow-ups

- T2(A3-opt): `dsp::Fft` re-export + stft plan 巻き上げ —
  `stft_is_bit_exact_*` がゲート
- T3(A4-opt): `from_columns` + rows API 置換 + caller 移行 +
  `benches/lstsq.rs` の列シグネチャ追従
- T4(A1-opt): `dsp::conv`(overlap-save)+ `fir_model` 畳み込み
  置換 + `benches/conv.rs` への `dsp::conv` 腕追加(同一
  fixture で baseline `pre` と A/B)+ `dsp.rs` tests への
  `conv_direct` 第3コピー
- 最終測定タスク(caller-opt-measurement-ir-closeout):
  before/after の全比較値を IR closeout へ転記
