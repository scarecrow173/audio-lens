---
id: "IMPL-034EPmN4wXkQ8rZs2vDhJf"
type: "impl"
status: "completed"
title: "機械的 caller クリーンアップ: characterization テスト・bench 腕・baseline 採取"
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
    - "docs/tasks/mech-cleanup-bench-baseline-characterization.md"
  implemented-by: []
  depends-on:
    - "docs/plans/mechanical-caller-cleanup.md"
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
    - "docs/adr/simd-reduction-lane-fma.md"
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

# 機械的 caller クリーンアップ: characterization テスト・bench 腕・baseline 採取

## Summary

plan T1(design step 1 の char コミット相当)。A6–A16 の全
最適化適用に先立ち、criterion ベンチ 4 腕・全項目
characterization テスト・A15 Instant 区間・A16d Instant
micro-timing・baseline 実測を land した。等価性は本
バンドルの test-spec 規則どおり `f64::to_bits` /
バイト列 / `(out, region_shape)` の完全一致(tolerance
predicate は不適用)。**最適化コードは一切含まず、
`src/` への差分は `#[cfg(test)] mod tests` 内の追加のみ**
(production 差分ゼロを `git diff` で確認済み)。

## Context

task `docs/tasks/mech-cleanup-bench-baseline-characterization.md`
(TASK-034EO484s25JM8bsb6zUCh)、plan
`docs/plans/mechanical-caller-cleanup.md`(T1)、design
`docs/designs/mechanical-caller-cleanup.md`(characterization
配置表・bench 構成)、test-spec
`docs/test-specs/mechanical-caller-cleanup.md`(完全一致規則)、
spec `docs/specs/mechanical-caller-cleanup.md`(プロセス契約)、
ADR `docs/adr/simd-reduction-lane-fma.md`(数値不変の権威)に
従う。後続の T2–T6 opt タスクは本 baseline・
characterization・bench 腕に依存する。ユーザー指示により
**本タスクではコミットを作成していない**(Done When の
「char/bench コミットが opt より先に land」はコミット実施時に
確認が必要)。

## Implementation

### Changes

- `crates/al-analysis/Cargo.toml`: `[[bench]] name =
  "band_correlation" / "csd_accum" / "quant_step" /
  "spectral_pass" harness = false` stanza ×4(criterion
  `=0.8.2` dev-dep は先行チェーンで導入済み・新規依存なし)
- `crates/al-analysis/benches/band_correlation.rs` 新設:
  A6 腕。`bands = third_octave_bands(40, min(rate/2, 16000))`
  (production と同じ帯域集合)・~2 s @48 kHz LCG 入力で
  `sequential_8x_bandpass`(現行 `.take(8)` × `bandpass`/
  帯域/信号)vs `batched_bandpass_many`(1 forward FFT/
  信号)を計測
- `crates/al-analysis/benches/csd_accum.rs` 新設: A7/A8 腕。
  ~2 s @48 kHz・4096/2048 STFT(45 フレーム × 4096 bin)で
  `per_frame_csd_call`(現行: frame 毎に bins 長
  `Vec<(f64,f64)>` を alloc)vs `per_frame_inline`(同式を
  累積 loop に直接記述)、`hft_len1_csd`(現行: (frame,bin)
  毎の長さ 1 `csd`)vs `hft_scalar_inline`(同式 inline)
- `crates/al-analysis/benches/quant_step.rs` 新設: A12 腕。
  in-gate fixture(1M サンプル・1/16 格子量子化)と
  >1M distinct fixture(1.5M LCG ノイズ)の両経路で、現行
  `uniq` u64 sort+dedup → gate → f64 `total_cmp` sort と
  HashSet `<u64>` 事前計数(bench 内に両経路を記述)を計測
- `crates/al-analysis/benches/spectral_pass.rs` 新設:
  A9/A16a/A16b 腕。~5.5 s @48 kHz(127 フレーム × 4096 bin)
  で `three_passes_mag_pow_psd` vs `single_power_derived`
  (`power()` 一回 + caller 導出、psd norm =
  `0.375*window*rate` の `scale` と同一式)、
  `mean_column_scan` vs `mean_row_major`(同一加算順の
  行優先)、`dom_magnitude_matrix` vs `dom_mag_sq_scratch`
  (`al_locate::dsp::mag_sq` facade 経由)
- characterization テスト(全 24 本、期待値は決定的
  fixture + test 内 oracle、magic literal なし):
  - `crates/al-analysis/tests/reference.rs`
    `mod mechanical_cleanup`(8 本): A6 band_correlation
    tensor `to_bits` + `ref.band_correlation_min` record・
    `bands < 8` edge(rate=400)、A7/A8 Welch 消費 tensor
    (transfer_h・coherence・transfer_h_t)、A7
    `ref.band_gain_m2` `params.coherence`、A7 stereo
    `lr_band_coherence` tensor + `ref.lr_lowband_coherence`、
    A16a `band_gain_psd_ratio` tensor/records + `psd_ratio`
    tensor、A16b `ref.harmonic_freq_ratio`(+`src_hz`/
    `cap_hz` params)、A15 residual interleave(R0 は
    capture そのものの frame-major 再構成で完全一致、
    全ステージは deinterleave→ch0→`stft_re` tensor 一致で
    layout を pin)
  - `crates/al-analysis/tests/expensive.rs`
    `mod mechanical_cleanup`(2 本): A7
    `exp.freq_lr_coherence`(channel.rs の band_bins 累積
    oracle)、A16a `mean_power`/`total_power`/
    `one_sided_power` 消費 record(`exp.cutoff_hz`・
    `exp.thd_ratio`・`exp.spectral_hole_ratio`)
  - `crates/al-analysis/src/cheap.rs` `mod tests`(5 本):
    A9 magnitude/power/psd 各 tensor 要素 `to_bits` +
    `spec.slope` record、A12 `dig.quant_step_est`
    value/support/Validity — in-gate(1/16 格子)と
    >1M distinct skip(1.3M LCG)の別 fixture、A7 stereo
    `stereo.coherence_mean`
  - `crates/al-locate/src/dsp.rs` `mod tests`(2 本): A15
    `deinterleave` 要素列完全一致(1/2/6ch・trailing
    partial frame drop・`channels == 0`・空入力)+
    `.f64le` バイト列 round-trip bit 一致
  - `crates/al-locate/tests/locate.rs`(1 本): A15
    `prepare_input` の normalized `.f64le` バイト列
    完全一致(frame-major・channel-inner・LE)
  - `crates/al-media/src/video.rs` `mod tests`(3 本):
    A16c luminance/diff 系列 `to_bits`・`diff[0] == 0.0`・
    長さ不一致時の `zip` 打ち切り・`pixels == 0`
    degenerate(NaN 系列)pin
  - `crates/al-dataset/src/tensor.rs` `mod tests`(2 本):
    A16d `(out, region_shape)` 完全一致 — innermost 完全
    包含(全行 contiguous)と部分包含 edge(左右端・
    1 列・1 セル)を row-major oracle で pin
  - `crates/al-dataset/tests/integration.rs`(1 本): A16d
    `Instant` micro-timing 区間(新規 dev-dep なし、
    timing は `eprintln` 出力・assert は完全一致のみ)
- `crates/al-locate/examples/bench_dsp.rs`: **A15 Instant
  区間追加**(deinterleave 現行 vs channel-block gather、
  interleave frame-major vs indexed write vs staged-row
  `extend_from_slice`、2ch × 1M フレーム)
- `crates/al-analysis/examples/bench_reference.rs`: 確認のみ
  — `analyze_pair` 経路が A6・A7/A8・A15 residual・A16a/b
  の stage 級経路を既にカバーしているため変更不要

### Decisions During Implementation

- **`bench_dsp` の A15 計測方式は「区間追加」に決定**:
  既存の計時経路(fft/NCC/bandpass/stft-equivalent/
  gcc_phat)に deinterleave/interleave は載っていなかった
  ため、plan の fallback 規定どおり Instant 区間を追加した
- **al-media(A16c)の計測方式は spec fallback に決定**:
  al-media には Instant/example 計時経路が存在しない。
  spec/plan の fallback 規定に従い、**code inspection +
  workspace bench 回帰ゲート**を採用(T6 適用時は stage 級
  manifest `duration_ms` も補助指標として使える)。
  characterization 側は `mod tests` の系列 `to_bits` で担保
- **al-dataset(A16d)は `tests/integration.rs` 内 Instant
  micro-timing を採用**(plan 指定どおり新規 dev-dep なし。
  timing は gate せず eprintln 出力)
- **`csd_accum` の inline 腕は zip 形へ書き換え**:
  clippy `needless_range_loop`(-D warnings)で
  `for k in 0..bins { pxy[k] += c[k] }` が指摘されたため、
  `pxy.iter_mut().zip(&c)` の同義形へ変更。baseline は
  修正後の腕形状で再採取済み(下記参照)
- **characterization oracle は production を逐語転記**:
  `welch()`/`band_gain_coherence_oracle()`/
  `luma_diff_oracle()`/`region_oracle()`/`quant_oracle()` 等は
  現行実装の式・加算順をそのまま test 側へ複写。public API
  (`csd`・`bandpass`・`StftMatrix`・`deinterleave`・
  `fit_mix`)は opt 後も残るため、opt 前後の両時点で同じ
  oracle が使える
- **A15 residual interleave の pin 方法**: 中間残差チャンネルは
  public 出力に現れないため、R0(capture そのもの)は
  frame-major 直接再構成で全要素 `to_bits`、全ステージは
  `deinterleave` → ch0 → `stft` が persisted `stft_re`
  tensor と一致することで layout を pin した
- **bench 実行は `--bench <name>` 単位**(先行チェーン IR と
  同じ理由): `cargo bench -p al-analysis -- --save-baseline`
  は lib unittest harness にも引数が渡り
  `Unrecognized option` で失敗するため、6 bench target を
  個別実行して baseline `mech-cleanup-pre` を保存した

## Related Experiments

### Adopted

<!-- No experiment logs: mechanical task per task-doc. -->

### Rejected

<!-- No rejected experiments. -->

## Baseline 計測結果(pre-optimization)

環境: `mise exec -- cargo bench -p al-analysis --bench <name> --
--save-baseline mech-cleanup-pre`(bench profile = release +
debuginfo)、criterion baseline は `target/criterion/<group>/
<arm>/mech-cleanup-pre/` 配下、完全出力・Instant 出力は
`target/bench-baseline/` 配下(ともに gitignore 済み)。
値は criterion 報告の median。

### criterion — band_correlation(A6、~2 s @48 kHz、8 帯域)

| arm | time (median) |
| --- | --- |
| sequential_8x_bandpass(現行) | 52.758 ms |
| batched_bandpass_many(候補) | 21.723 ms |

### criterion — csd_accum(A7/A8、45 フレーム × 4096 bin)

| arm | time (median) |
| --- | --- |
| per_frame_csd_call(現行) | 241.63 µs |
| per_frame_inline(候補) | 169.73 µs |
| hft_len1_csd(現行) | 4.5943 ms |
| hft_scalar_inline(候補) | 567.39 µs |

※ 本ファイルの数値は clippy 修正後の腕形状で再採取した run
(`target/bench-baseline/csd_accum-mech-cleanup-pre.txt`)。
`criterion-mech-cleanup-pre.txt` 内の csd_accum 区間は修正前
(index 形 inline = 758.42 µs)のため参照しないこと。

### criterion — quant_step(A12、両経路)

| arm | time (median) |
| --- | --- |
| in_gate_double_sort(現行・1M) | 7.2851 ms |
| in_gate_hashset_gate(候補) | 14.171 ms |
| over_1m_double_sort(現行・1.5M/>1M distinct) | 27.628 ms |
| over_1m_hashset_gate(候補) | 55.179 ms |

観察: この入力規模では HashSet 事前計数は sort+dedup より
遅い(候補腕が両経路で現行を下回らなかった)。A12 候補形状の
採否は T4 で再評価すること(本タスクは計測基盤のみで、腕の
優劣を確定しない)。

### criterion — spectral_pass(A9/A16a/A16b、127 フレーム × 4096 bin)

| arm | time (median) |
| --- | --- |
| three_passes_mag_pow_psd(現行) | 6.1926 ms |
| single_power_derived(候補) | 3.4120 ms |
| mean_column_scan(現行) | 553.73 µs |
| mean_row_major(候補) | 151.44 µs |
| dom_magnitude_matrix(現行) | 1.2490 ms |
| dom_mag_sq_scratch(候補) | 446.68 µs |

### criterion — 既存腕(conv / stft、baseline `mech-cleanup-pre`)

| arm | time (median) |
| --- | --- |
| conv_direct n65536_m256 | 3.5971 ms |
| conv_direct n262144_m256 | 14.341 ms |
| conv_direct n65536_m4096 | 61.168 ms |
| conv_direct n262144_m4096 | 251.47 ms |
| dsp_conv n65536_m256 | 741.03 µs |
| dsp_conv n262144_m256 | 3.0274 ms |
| dsp_conv n1048576_m256 | 12.369 ms |
| dsp_conv n65536_m4096 | 1.2382 ms |
| dsp_conv n262144_m4096 | 4.8530 ms |
| dsp_conv n1048576_m4096 | 18.906 ms |
| stft 4096/2048 n=524288 | 11.630 ms |
| stft 4096/2048 n=1048576 | 23.901 ms |

### Instant — bench_dsp A15 区間(`target/bench-baseline/bench_dsp-pre.txt`)

| 計測点 | mean (5 rep) |
| --- | --- |
| deinterleave 2ch × 1M(現行 element loop) | 7.1 ms |
| deinterleave 2ch × 1M(channel-block gather) | 6.0 ms |
| interleave 2ch × 1M(frame-major loop) | 5.2 ms |
| interleave 2ch × 1M(indexed write) | 5.2 ms |
| interleave 2ch × 1M(staged-row extend) | 7.2 ms |

### Instant — bench_reference(`target/bench-baseline/bench_reference-pre.txt`)

| 計測点 | mean (3 rep) |
| --- | --- |
| stft 1M samples (4096/2048) | 32.5 ms |
| analyze_pair 2s mono (R0–R3, fir_model) | 631.8 ms |
| prepare_residual 2s 2src × 2cap | 10.9 ms |

### Instant — al-dataset A16d micro-timing(release、`--nocapture` 出力)

| region | time |
| --- | --- |
| contiguous_rows [100,2000)×[0,256)(486_400 vals) | 17.431 ms |
| partial_inner [100,2000)×[64,160)(182_400 vals) | 7.5101 ms |
| one_chunk [0,64)×[0,64)(4_096 vals) | 183.7 µs |

## Validation

- `mise exec -- cargo test -p al-analysis -p al-locate -p
  al-media -p al-dataset`: green — al-analysis 139(lib 93 +
  avsync 7 + expensive 8 + golden 19 + reference 12)、
  al-dataset 33(lib 20 + integration 13)、al-locate 71
  (lib 38 + golden 16 + locate 12 + prepared 5)、al-media
  120(lib 69 + mp4 12 + registry 10 + ingest 29)。新規
  characterization 24 本を含む
- `mise exec -- cargo bench -p al-analysis -- --list`:
  `band_correlation`・`csd_accum`・`quant_step`・
  `spectral_pass` が列挙される(既存 `conv`・`stft` と併記)
- `mise exec -- cargo bench -p al-analysis --bench <name> --
  --save-baseline mech-cleanup-pre`(6 target): 完走、
  `target/criterion/**/mech-cleanup-pre/` に保存済み
- `mise exec -- cargo run --release --example bench_dsp -p
  al-locate`・`--example bench_reference -p al-analysis`:
  完走、出力は `target/bench-baseline/bench_dsp-pre.txt`・
  `bench_reference-pre.txt`
- `mise exec -- cargo test -p al-dataset --release --test
  integration a16d -- --nocapture`: A16d Instant 出力取得
- `mise exec -- cargo clippy --workspace --all-targets -- -D
  warnings`: green
- `mise exec -- cargo fmt --all -- --check`: green
- `mise exec -- cargo tree -p al-analysis --depth 1`: 直接依存に
  al-math なし(al-core・al-dataset・al-job・al-locate・
  al-media・serde・serde_json・tracing;dev-deps は
  al-testkit・criterion)
- `git diff`: production `src/` への差分は `#[cfg(test)]
  mod tests` 内の追加のみ、production code diff なしを確認

## Risks

- criterion 値は本マシンの単発 run。候補腕の比較判定は T5/T7
  の再計測で同一 baseline 名・同一 profile に限り有効
- Instant 系は 3–5 rep 平均・環境ノイズ ±5% 以上を含み得る
- `quant_step` の HashSet 候補は baseline で現行を下回った。
  T4 で候補形状(HashSet 容量・count-only 経路・別 gate
  方式)を再検討する余地を残す
- `per_frame_inline`(A7 候補)は zip 形で計測しており、
  T3 の実装 shape(累積位置・iter 形)が変わると差が縮まる
  可能性がある。再現性のため腕コードは bench ファイルに
  自己完結で残している
- 本タスクはコミットを作らず報告のみ(char+opt の履歴順序は
  実際にコミットする際に担保が必要)

## Follow-ups

- T2(A6-opt): `bandpass_many` 化 — A6 characterization
  (band_correlation tensor/edge)がゲート、bench 腕
  `band_correlation` で A/B
- T3(A7/A8-opt): `csd` caller inline 化 ×7 サイト + hft —
  `csd_accum` 腕 + Welch/stereo/band_gain/lr_band_coherence/
  `exp.freq_lr_coherence`/`stereo.coherence_mean` の各
  characterization がゲート
- T4(A9/A12-opt): `power()` 一元化 + HashSet gate —
  `spectral_pass`/`quant_step` 腕 + mag/pow/psd tensor・
  `spec.slope`・`dig.quant_step_est` 両経路ゲート
- T5(A15-opt): interleave/deinterleave ブロック化 —
  `.f64le` バイト列・deinterleave 列・residual interleave
  列ゲート + `bench_dsp` A15 区間再計測
- T6(A16-opt): 行優先累積・dom scratch・video 融合・
  `read_range` fast path — A16a–d 各 characterization +
  `spectral_pass`・A16d Instant 再計測
- T7: 全 bench `--baseline mech-cleanup-pre` 比較・IR
  転記・char→opt 履歴順序の最終確認(コミット作成時)
