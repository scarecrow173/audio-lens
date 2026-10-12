---
id: "IMPL-034EPEQoc16OG8lQeyK30c"
type: "impl"
status: "completed"
title: "機械的 caller クリーンアップ A6: band_correlation の bandpass_many バッチ化"
created: "2026-10-12"
updated: "2026-10-12"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "code"
        path: "crates/al-analysis/src/reference.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/mech-cleanup-a6-bandpass-batching.md"
  implemented-by: []
  depends-on:
    - "docs/plans/mechanical-caller-cleanup.md"
    - "docs/impl/ir/mech-cleanup-bench-baseline-characterization.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/mechanical-caller-cleanup.md"
    - "docs/test-specs/mechanical-caller-cleanup.md"
    - "docs/specs/mechanical-caller-cleanup.md"
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

# 機械的 caller クリーンアップ A6: band_correlation の bandpass_many バッチ化

## Summary

plan T2(design step 2 の A6-opt コミット相当、T3 と同一作業
セッションで実施)。`reference.rs` の band_correlation 構築を
`bands.iter().take(8)` + 16 回の `bandpass` 呼出し(src/cap ×
8 band)から、`&bands[..bands.len().min(8)]` の bound-safe
cap slice + `bandpass_many` 2 呼出し(src/cap 各 1 forward
FFT)へ置き換えた。`bandpass` は `bandpass_many(.., &[(lo,
hi)])` `.pop()` の thin wrapper で mask 規則・切り戻し・出力
順が同一のため `band_corrs` は bit-identical — characterization
(tensor 要素 `to_bits` + `bands < 8` edge)が opt 後も green。

## Context

task `docs/tasks/mech-cleanup-a6-bandpass-batching.md`
(TASK-034EO4gPvPkx3voaC6KOlY)、plan
`docs/plans/mechanical-caller-cleanup.md`(T2)、design
`docs/designs/mechanical-caller-cleanup.md`(A6 sketch・import
prune・計測方法)に従う。baseline・characterization は T1
(`docs/impl/ir/mech-cleanup-bench-baseline-characterization.md`)
が供給。ユーザー指示により**本タスクではコミットを作成して
いない**(Done When の履歴順序はコミット実施時に担保が必要)。

## Implementation

### Changes

- `crates/al-analysis/src/reference.rs`:
  - band_correlation 構築を design sketch どおり
    `let capped = &bands[..bands.len().min(8)];` →
    `bandpass_many(src_mono, rate, capped)`・
    `bandpass_many(cap_mono, rate, capped)` →
    `xs.iter().zip(&ys).map(|(x, y)| corr0(x, y))` へ置換。
    `&bands[..8]` リテラルは使わず `.min(8)` で bound
    (`bands < 8` の edge は T1 characterization が pin)
  - `use al_locate::dsp::{...}` リストの `bandpass` を
    `bandpass_many` へ入れ替え(未使用 import による clippy
    `-D warnings` fail を防止)。`al-math`/`dsp.rs` 側の
    `bandpass` 公開面は不変

### Decisions During Implementation

- **`band_corrs` の型を明示**: `Vec::new()` + `push` から
  `collect()` への変更で要素型推論が必要になったため
  `let band_corrs: Vec<f64>` と宣言。`corr0(x, y)` は
  `&Vec<f64>` → `&[f64]` の deref coercion でそのまま通る
- **`use` リストを複数行へ展開**: `bandpass_many` 追加で
  rustfmt の 1 行上限を超えたため fmt 準拠の複数行形へ

## Related Experiments

### Adopted

<!-- No experiment logs: mechanical task per task-doc. -->

### Rejected

<!-- No rejected experiments. -->

## Bench 計測結果(post-optimization)

環境: `mise exec -- cargo bench -p al-analysis --bench
band_correlation -- --baseline mech-cleanup-pre`(criterion
median、baseline は T1 が保存した `mech-cleanup-pre`)。
stage 級経路は `mise exec -- cargo run --release --example
bench_reference -p al-analysis` を再実行し
`target/bench-baseline/bench_reference-pre.txt` と比較。

### criterion — band_correlation(8 帯域、~2 s @48 kHz)

| arm | baseline (median) | post-change (median) | change |
| --- | --- | --- | --- |
| sequential_8x_bandpass | 52.758 ms | 49.468 ms | −6.2% |
| batched_bandpass_many | 21.723 ms | 20.838 ms | −4.1% |

腕形状は bench ファイル内で自己完結するため両腕とも微小な
run 間変動のみ。採用差分(sequential → batched、= production
が今回取った形状)は baseline 時点の腕比較 52.758 → 21.723 ms
(約 2.4 倍、forward FFT 16 → 2 回)どおり。

### Instant — bench_reference(`bench_reference-pre.txt` との比較)

| 計測点 | pre (mean) | post (mean) |
| --- | --- | --- |
| stft 1M samples (4096/2048) | 32.5 ms | 30.9 ms |
| analyze_pair 2s mono (R0–R3, fir_model) | 631.8 ms | 561.8 ms |
| prepare_residual 2s 2src × 2cap | 10.9 ms | 8.3 ms |

`analyze_pair` −11% は A6 と同時適用した A7/A8(T3、
`docs/impl/ir/mech-cleanup-a7-a8-csd-inlining.md`)の合成効果を
含む点に注意 — 本 run は両 opt 適用後の stage 級計測。

## Validation

- `mise exec -- cargo test -p al-analysis -p al-locate`: green —
  al-analysis 139(lib 93 + avsync 7 + expensive 8 + golden 19 +
  reference 12)、al-locate 71。A6 characterization
  (`a6_band_correlation_tensor_is_bit_exact`・
  `a6_band_correlation_bands_below_eight_edge`)を含む全
  mechanical_cleanup テストが green
- `mise exec -- cargo test --workspace`: green(golden 含む)
- `mise exec -- cargo clippy --workspace --all-targets --
  -D warnings`: green(未使用 import なし)
- `mise exec -- cargo fmt --all -- --check`: green
- `rg 'bandpass\(' crates/al-analysis/src/reference.rs`:
  0 件(production;`expensive/tests.rs` の test 用呼出しは
  既存のまま対象外)
- `rg 'bandpass_many' crates/al-analysis/src/reference.rs`:
  use + 2 呼出し
- `rg 'bands\[\.\.8\]' crates/`: 0 件(`.min(8)` bound のみ)

## Risks

- criterion 値は本マシンの単発 run。±5% 程度の環境ノイズを
  含み得る(sequential 腕 −6.2% はコード不変のため純粋な揺らぎ)
- `bench_reference` の `analyze_pair` 改善は A7/A8 との合成であり、
  A6 単独の stage 級寄与は分離していない

## Follow-ups

- T3(A7/A8-opt): `csd` inline 化 — 同一セッションで適用済み、
  IR は `docs/impl/ir/mech-cleanup-a7-a8-csd-inlining.md`
- T7(measurement-ir-closeout): 全 bench の最終比較・char→opt
  履歴順序の確認(コミット実施時)
