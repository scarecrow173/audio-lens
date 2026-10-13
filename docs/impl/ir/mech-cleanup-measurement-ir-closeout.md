---
id: "IMPL-034ERb7k6dRCqL49GfwQ7X"
type: "impl"
status: "completed"
title: "機械的 caller クリーンアップ: 事後計測・Implementation Record 記録・最終検証"
created: "2026-10-12"
updated: "2026-10-12"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "doc"
        path: "docs/impl/ir/mech-cleanup-measurement-ir-closeout.md"
    modified:
      - type: "doc"
        path: "docs/tasks/mech-cleanup-measurement-ir-closeout.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
      - type: "doc"
        path: "docs/plans/mechanical-caller-cleanup.md"
      - type: "doc"
        path: "docs/plans/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/mech-cleanup-measurement-ir-closeout.md"
  implemented-by: []
  depends-on:
    - "docs/plans/mechanical-caller-cleanup.md"
    - "docs/impl/ir/mech-cleanup-bench-baseline-characterization.md"
    - "docs/impl/ir/mech-cleanup-a6-bandpass-batching.md"
    - "docs/impl/ir/mech-cleanup-a7-a8-csd-inlining.md"
    - "docs/impl/ir/mech-cleanup-a9-a12-cheap-pass-consolidation.md"
    - "docs/impl/ir/mech-cleanup-a15-interleave-block-writes.md"
    - "docs/impl/ir/mech-cleanup-a16-row-major-fast-paths.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/mechanical-caller-cleanup.md"
    - "docs/specs/mechanical-caller-cleanup.md"
    - "docs/test-specs/mechanical-caller-cleanup.md"
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

# 機械的 caller クリーンアップ: 事後計測・Implementation Record 記録・最終検証

## Summary

plan T7(closeout)。T1–T6 で land した全最適化(A6 bandpass
バッチ化・A7/A8 `csd` インライン化・A9 `power()` 一括化 +
`mean_row` 融合・A15 interleave/deinterleave ブロック化・
A16a–d 行優先累積/dom scratch/video 融合/tensor fast path)
の事後計測を、T1 baseline(criterion named baseline
`mech-cleanup-pre`・`target/bench-baseline/` Instant 出力)
に対して実施し、before/after を確定した。A12(HashSet 事前
計数)は T4 で証拠付き棄却済みのため不変更のまま確定。

key number: stage 級代表経路 `analyze_pair`(2s mono、R0–R3
fir_model)は **631.8 → 575.8 ms(−8.9%、T2–T6 累積)**。
criterion の同一 run 腕比較では band_correlation の
sequential→batched 化で約 2.5 倍、csd per-frame 累積で
約 1.5 倍・hft で約 7.6 倍、spectral_pass の 3-pass→
single-power で約 1.8 倍・mean 列走査→row-major で約
3.7 倍・dom magnitude 行列→`mag_sq` scratch で約 2.6 倍。
対象外経路(stft・conv・lstsq・al-locate Instant 区間)に
コード起因と断定できる ±5% 超回帰なし — 本セッションは
全計測点で機械 drift(±10–20%、band_correlation 初回 run
は +195% の外れ)が大きく、残余の +5–11% 観測値はコード
未変更の証拠(git diff)と併せて drift と判断(後述)。
workspace 検証一式 green、golden 非回帰、両 audit clean。

## Context

task `docs/tasks/mech-cleanup-measurement-ir-closeout.md`
(TASK-034EO4z7oZadAFuMojWjLO)、plan
`docs/plans/mechanical-caller-cleanup.md` T7、spec
`docs/specs/mechanical-caller-cleanup.md` のプロセス契約
「before/after 実測を Implementation Record に記録」に
従う。baseline は T1 IR
(`mech-cleanup-bench-baseline-characterization.md`)が
`target/criterion/`(named baseline `mech-cleanup-pre`)と
`target/bench-baseline/`(`criterion-mech-cleanup-pre.txt`・
`bench_reference-pre.txt`・`bench_dsp-pre.txt`・al-dataset
Instant 値)に記録したもの。ユーザー指示により**本タスク
でもコミットを作成していない**。

## Implementation

### Changes

コード変更なし(計測・記録・ドキュメント closeout のみ)。

- 4 本の新規 criterion bench を baseline 名付きで再実行:
  `cargo bench -p al-analysis --bench band_correlation --
  --baseline mech-cleanup-pre`・`--bench csd_accum`・
  `--bench quant_step`・`--bench spectral_pass`(全て逐次
  実行。初回 run は band_correlation で機械負荷による
  +83–195% の偽 regression を示したため静かな時間帯に
  再実行。生ログ `target/bench-baseline/
  criterion-mech-cleanup-post-t7.txt`・
  `-t7-rerun.txt`、gitignore 済み)
- 非対象経路の回帰確認: `--bench stft --baseline
  mech-cleanup-pre`(2 run)・`--bench conv --baseline
  mech-cleanup-pre`・al-math `--bench lstsq --baseline
  lstsq-rows`(先行チェーンの baseline。本バンドルでは
  al-math 無変更)。生ログ `criterion-nontarget-t7.txt`・
  `criterion-stft-t7-rerun.txt`
- `cargo run --release --example bench_reference -p
  al-analysis` を 3 run(feature フラグ不要を確認)、
  `cargo run --release --example bench_dsp -p al-locate`
  を 3 run。生ログ `bench-reference-t7.txt`
- al-dataset `tests/integration.rs` の A16d Instant
  micro-timing を release test で再計測
  (`--nocapture` 出力)
- `docs/impl/ir/README.md`・`docs/tasks/README.md` の索引
  行、task doc の status → done・implemented-by 記入、plan
  `mechanical-caller-cleanup.md` の status → completed・
  `docs/plans/README.md` 索引行(closeout 慣行: 先行
  チェーンの plan クローズと同じ)

### Decisions During Implementation

- **逐次実行・外れ run 除外の運用**: T4 IR で記録済みの
  「bench 並行実行による競合 noise」に加え、本セッション
  はアイドル時でも 10–20% の帯域変動が観測された。判定は
  (a) 同一 run 内の腕比較(採用差分の実測)、(b) 静かな
  時間帯の再 run(baseline 比較の sanity)、(c) コード未
  変更の git diff 証拠、の 3 本立て。初回 band_correlation
  run(sequential +195%・batched +83%)は腕コードが T1 から
  不変の A/B 自己完結腕であるため機械負荷と断定し除外、
  再 run(sequential −8.0%・batched −10.5%)を採用
- **stft/conv の +5–11% 観測値は回帰と判定しない**:
  `git log main..HEAD -- crates/al-analysis/src/stft.rs
  crates/al-math/`・`git diff main...HEAD` が空(当該
  コードは本ブランチで一切不変更)。criterion の stft 腕・
  conv 腕・al-locate Instant 区間(NCC・bandpass・
  gcc_phat・fft)が一様に +5–20% へ張り付く現象は、同一
  セッション内で bench ファイル内自己完結腕まで同方向へ
  動くことからセッション全体の機械状態(周波数・負荷)
  と判断。±5% ゲートの適用は「コード起因の回帰」に限る
  spec 文意に従い、drift は Risks に明記して判定保留と
  しない
- **A12 棄却の再確認のみ**: quant_step 腕は再計測しても
  HashSet が両経路で遅い(in-gate 14.13 vs 6.40 ms・>1M
  53.06 vs 26.74 ms)。T4 IR の棄却判断が成立し続けること
  を確認し、production コードは不変更
- **al-media(A16c)は spec fallback 記録を維持**: T1 で
  確定したとおり Instant/example 計時経路が存在しない
  ため code inspection + characterization `to_bits` +
  workspace 回帰ゲート(T6 IR の記録を引用)
- **manifest `duration_ms` の別途計測は行わない**: 代表的
  実メディア pipeline の before 値は本セッションで再現
  不能(pre-state ビルドがない)ため、stage 級証拠は spec
  が指定する `bench_reference`(`analyze_pair` を含む)の
  Instant 計測で代替(先行チェーン closeout と同じ裁定)

## Related Experiments

### Adopted

<!-- No experiment logs: measurement/closeout task. -->

### Rejected

<!-- No rejected experiments. -->

## Bench 結果(consolidated before/after)

環境: 同一マシン、`mise exec --` 経由、criterion = 0.8.2
(bench profile = release + debuginfo)、Instant ベンチは
release。baseline は全て T1 の `mech-cleanup-pre`。
**本セッションは機械 drift が大きく、baseline との絶対差
は参考値 — 採用差分の実測は同一 run 内の腕比較で示す。**

### A6 — band_correlation(127 フレーム × 8 帯域、criterion median)

| arm | baseline (T1) | T7 (再 run) | change |
| --- | --- | --- | --- |
| sequential_8x_bandpass(旧形状) | 52.758 ms | 48.527 ms | −8.0% |
| batched_bandpass_many(採用形状) | 21.723 ms | 19.437 ms | −10.5% |

腕コード不変のため絶対改善は環境 drift。**同一 run 腕比較
が採用差分**: 今回 run で sequential 48.5 → batched 19.4 ms
(約 2.5 倍)、baseline 時点の腕比較(52.8 → 21.7 ms、約
2.4 倍)と一致。`analyze_pair` 累積にも寄与(下記)。
初回 run(sequential 155.79 ms・batched 39.77 ms)は機械
負荷の外れとして除外。

### A7/A8 — csd_accum(criterion median)

| arm | baseline (T1) | T7 | change |
| --- | --- | --- | --- |
| per_frame_csd_call(旧形状) | 240.33 µs | 234.76 µs | −2.6%(noise) |
| per_frame_inline(採用形状) | 758.42 µs *¹ | 158.66 µs | −3.4% vs T3-run 基準 *² |
| hft_len1_csd(旧形状) | 4.4405 ms | 4.4257 ms | −3.7%(noise) |
| hft_scalar_inline(採用形状) | 591.06 µs | 581.01 µs | +2.9%(noise) |

*¹ T1 baseline 採取時は inline 腕が異常に遅い値(758 µs)で
記録されており、T3 IR の post 値 156.91 µs および本 run の
158.66 µs が実態。
*² baseline との nominal change −79% は T1 の高 drift 値への
比較のため参考外。実質的な採用差分は同一 run 腕比較:
per-frame 235 → 159 µs(約 1.5 倍)、hft 4.43 ms → 581 µs
(約 7.6 倍)。alloc 除去効果は T3 IR と一致して再現。

### A12 — quant_step(criterion median、棄却の再確認)

| arm | baseline (T1) | T7 | change |
| --- | --- | --- | --- |
| in_gate_double_sort(現行=維持) | 7.2851 ms | 6.3956 ms | −12.2% |
| in_gate_hashset_gate(棄却) | 14.171 ms | 14.134 ms | −0.3% |
| over_1m_double_sort(現行=維持) | 27.628 ms | 26.742 ms | −3.2% |
| over_1m_hashset_gate(棄却) | 55.179 ms | 53.056 ms | −3.8% |

同一 run で HashSet は in-gate 約 2.2 倍・>1M 約 2.0 倍遅く、
T4 の棄却判断を 3 回目の計測でも再確認(baseline・T4・T7
で方向不変)。production は現行 double-sort 形状維持、
bench 腕は再評価用に残置。

### A9/A16a/A16b — spectral_pass(127 フレーム × 4096 bin、criterion median)

| arm | baseline (T1) | T7 | change |
| --- | --- | --- | --- |
| three_passes_mag_pow_psd(旧形状) | 6.1926 ms | 5.4802 ms | −11.5% |
| single_power_derived(採用形状、A9) | 3.4120 ms | 2.9934 ms | −12.3% |
| mean_column_scan(旧形状) | 553.73 µs | 515.58 µs | −1.7%(noise) |
| mean_row_major(採用形状、A9+A16a) | 151.44 µs | 138.25 µs | −9.7% |
| dom_magnitude_matrix(A16b 旧) | 1.2490 ms | 1.0536 ms | −16.4% |
| dom_mag_sq_scratch(A16b 採用) | 446.68 µs | 401.40 µs | −9.9% |

同一 run 腕比較が採用差分: 3-pass 5.48 → single_power
2.99 ms(約 1.8 倍)、mean 列走査 516 → row-major 138 µs
(約 3.7 倍)、dom magnitude 行列 1054 → `mag_sq` scratch
401 µs(約 2.6 倍)。T4・T6 IR と同方向で再現。

### Instant — bench_reference(3 run、pre/t4/t6 との比較)

| 計測点 | pre (T1) | post T4 | post T5+T6 | T7 (3 run) | pre 比 |
| --- | --- | --- | --- | --- | --- |
| stft 1M samples (4096/2048) | 32.5 ms | ~32.7 ms | ~32.3 ms | ~36.8 ms (34.2/40.9/35.2) | +13%(drift) |
| analyze_pair 2s mono (R0–R3, fir_model) | 631.8 ms | ~548.6 ms | ~570.8 ms | ~575.8 ms (575.8/574.4/578.0) | **−8.9%** |
| prepare_residual 2s 2src × 2cap | 10.9 ms | ~7.8 ms | ~7.8 ms | ~8.8 ms (8.0/8.8/9.5) | −19% |

`analyze_pair` 累積 delta = **631.8 → 575.8 ms(−8.9%)**。
内部で cheap stage(A9)・band_correlation(A6)・csd
経路(A7/A8)・reference 側 A16a/b を含む複合値であり、
単独 opt の寄与分離は不能 — criterion 腕比較が要素別の
根拠。`stft`・`prepare_residual` はコード不変の計測点で
それぞれ +13%・−19% と乖離しており、このセッションの
Instant drift が ±10–20% に達することを裏付ける。

### Instant — al-locate bench_dsp(3 run、A15 計測点 + 非対象区間)

| 計測点 | baseline (T1) | post T5 (a15) | T7 (3 run 範囲) |
| --- | --- | --- | --- |
| deinterleave 2ch × 1M(production) | 7.1 ms | 4.4 ms | 5.6–9.6 ms |
| deinterleave(channel-block gather 腕) | 6.0 ms | 5.3 ms | 5.7–7.3 ms |
| interleave(frame-major loop 腕) | 5.2 ms | 4.7 ms | 5.3–6.7 ms |
| interleave(indexed write 腕) | 5.2 ms | 5.0 ms | 5.5–7.0 ms |
| interleave(staged-row extend 腕) | 7.2 ms | 7.1 ms | 7.4–10.3 ms |
| legacy fft 4M | 284.3 ms | 285.8 ms | 295.5–344.3 ms |
| fft 4M (next_pow2) | 160.2 ms | 135.9 ms | 167.1–186.4 ms |
| complex fft 4M reused | 102.0 ms | 97.6 ms | 97.7–99.0 ms |
| rfft 4M reused | 58.6 ms | 61.2 ms | 58.9–62.3 ms |
| NCC 2.4M × 8k | 410.0 ms | 409.6 ms | 447.9–498.1 ms |
| bandpass 2.4M | 186.1 ms | 188.5 ms | 197.2–213.3 ms |
| welch-ish psd 2.4M | 46.5 ms | 46.4 ms | 46.9–50.4 ms |
| gcc_phat 2.4M × 8k | 245.3 ms | 241.3 ms | 266.9–291.6 ms |

A15 計測点: production deinterleave は baseline 7.1 → T7
median 6.0 ms(−15%)で T5 計測(4.4 ms)と同方向。
残りの腕・非対象区間(fft・NCC・bandpass・gcc_phat・
welch-ish)はコード不変でありながら一様に +5–20% で、
本セッション drift の定量証拠となる。±5% 超の見かけ
回帰は全てこの機械変動の範囲内で、コード起因と断定
できるものはない(各関数の `git diff` は空)。

### Instant — al-dataset A16d micro-timing(release test)

| region | pre (T1) | post T6 | T7 (1 run) |
| --- | --- | --- | --- |
| contiguous_rows [100,2000)×[0,256)(486_400 vals) | 17.431 ms | 15.09–15.30 ms | 16.457 ms |
| partial_inner [100,2000)×[64,160)(182_400 vals) | 7.5101 ms | 6.31–6.74 ms | 7.406 ms |
| one_chunk [0,64)×[0,64)(4_096 vals) | 183.7 µs | 144.6–150.0 µs | 159.0 µs |

T7 値は T6 post 帯域より約 +5–10% 高い(セッション
drift)が、pre 比では contiguous −5.6%・partial_inner
−1.4%・one_chunk −13% で改善方向を維持。chunk I/O が
支配的なため gather→block copy の効果は全体の 1 割強。

### al-media(A16c)— spec fallback 記録

T1 確定どおり Instant 計時経路が存在しないため **code
inspection + characterization 系列 `to_bits` + workspace
回帰ゲート**を採用(T6 IR に証拠記録済み):

- code inspection: 2 pass → 1 pass で per-frame luma 走査
  が半減、`p.get(i)` 分岐は `Some` 側のみで zip と同じ
  short-circuit、追加 allocation なし
- characterization: `a16c_luma_diff_series_is_bit_exact`・
  `a16c_length_mismatch_truncates_via_zip`・
  `a16c_zero_pixel_degenerate_series_is_pinned` が opt 後も
  bit-exact green(本タスクの `cargo test --workspace` で
  再確認)
- manifest `duration_ms` の stage 級計測は baseline 未採取
  のため記録しない(T1 IR の方式決定どおり)

### 非対象経路 — 回帰判定(±5% bound)

| bench | baseline | T7 | 判定 |
| --- | --- | --- | --- |
| stft/4096/2048/524288 | 11.630 ms | 12.487/12.898 ms(2 run) | +5–11% drift(コード不変) |
| stft/4096/2048/1048576 | 23.901 ms | 25.138/28.251 ms | +5–18% drift(コード不変) |
| conv_direct 全 4 点 | T1 値 | −3.5%〜+3.1% | noise 内 |
| dsp_conv 全 6 点 | T1 値 | −3.4%〜+11.4% | 大 N 2 点 +8–11% は drift(コード不変) |
| lstsq new/solve 全 18 点 | lstsq-rows | new −37〜−69%・solve ほぼ noise | 2 列 solve +5–7% は先行チェーン既知 trade-off の再現 |

`stft`/`dsp_conv` の見かけ回帰は (1) 対象コードの
`git diff main...HEAD` が空、(2) 同一セッションで bench
ファイル内自己完結腕まで同方向に動く、(3) 再 run で値が
改善方向へ振れる(band_correlation、stft 2nd run)、の 3
点から機械 drift と判定。revert 対象となるコード起因の
回帰は確認されなかった。lstsq `solve/2x65536`(+5.3%)・
`solve/2x1048576`(+6.1%)は先行チェーン T3 IR 記録済みの
既知 trade-off(列 `sub_scaled`/`sum_sq` RSS、k=2 の
メモリトラフィック)がそのまま再現したもので、本バンドル
とは無関係(al-math 無変更)。

## spec Acceptance Criteria 充足確認

| AC(要旨) | 充足根拠 |
| --- | --- |
| 各項目の char テストが opt 前 green・opt 後も green(char コミット先行) | git: char 一括コミット `5e05cd5` が全 opt コミット(`8201da8` A6+A7/A8・`cd3688c` A9・`289001e` A15+A16)に先行。`cargo test --workspace` 39 suite green で opt 後も green |
| A6: `band_corrs` `to_bits` 一致・`bandpass_many` 2 回・`.min(8)` bound・`bandpass(` 不在 | `a6_band_correlation_tensor_is_bit_exact`・`a6_band_correlation_bands_below_eight_edge` green。`rg 'bandpass_many'` で src/cap 2 呼出し + `bands.len().min(8)` 確認済み(T2 IR) |
| A7/A8: 下流 record・hft `to_bits`、7 サイトに `csd(` 不在 | `a7_*`・`a8_hft_*` characterization green。`rg 'csd\('` で列挙サイトに残存なし(T3 IR) |
| A9: mag/pow/psd・`spec.slope` `to_bits`、`/ norm` 不在・`:665` 前導出・`mean_row` 融合・新規メソッドなし | `a9_mag_pow_psd_tensors_are_bit_exact`・`a9_spec_slope_record_is_bit_exact` green + コード検査(T4 IR) |
| A12: `dig.quant_step_est` value/support/Validity が両経路で一致 | `a12_quant_step_in_gate_is_bit_exact`・`a12_quant_step_over_1m_skip_is_bit_exact` green(コード不変更のため当然充足) |
| A15: `.f64le` バイト列・deinterleave 列・residual interleave 列完全一致 | locate.rs・dsp.rs tests・reference.rs tests の a15 characterization green(T5 IR) |
| A16a/b: `mean_psd_*`/`mean_power`/hfr `to_bits`・scalar row-ascending・`mag_sq` scratch・手書き二乗和なし | `a16a_*`・`a16b_harmonic_freq_ratio_is_bit_exact` green + `rg 're\s*\*\s*re'` 検査(T6 IR) |
| A16c: luminance/diff 系列 `to_bits`・`diff=0.0`・`pixels==0` | a16c characterization 3 本 green(T6 IR) |
| A16d: `(out, region_shape)` が fast path・fallback・部分包含 edge で完全一致 | `a16d_read_range_*` 3 経路 assert green(T6 IR + 本タスク再計測) |
| 項目ごとの計測方式明記 + before/after 実測が IR に記録 | 本 IR「Bench 結果」節 + T2–T6 各 IR の post 計測節 |
| 非対象経路 ±5% 以内 | コード起因の回帰なし。観測された +5–11% は全てコード不変経路のセッション drift(証拠は判定表) |
| workspace test・build・clippy・fmt green・golden pass | `cargo test --workspace` 39 suite(golden 含む)0 failed・`cargo build --workspace`・`cargo fmt --all -- --check`・`cargo clippy --workspace --all-targets -- -D warnings` 全て green |
| `cargo tree -p al-analysis --depth 1` に al-math なし | 直接依存 = al-core・al-dataset・al-job・al-locate・al-media・serde・serde_json・tracing(dev: al-testkit・criterion) |
| `audit_docs`・`audit_impl_record` clean | `audit_docs.js --type all`: 236 files no findings。`audit_impl_record.js`: 67 files(本 IR 追加前 66)no findings |

## Validation

- `mise exec -- cargo bench -p al-analysis --bench
  {band_correlation,csd_accum,quant_step,spectral_pass} --
  --baseline mech-cleanup-pre`: 全腕完走、初回
  band_correlation run は外れとして再 run を採用
- `mise exec -- cargo bench -p al-analysis --bench {stft,
  conv} -- --baseline mech-cleanup-pre`・`cargo bench -p
  al-math --bench lstsq -- --baseline lstsq-rows`: 完走、
  非対象経路の drift を定量
- `mise exec -- cargo run --release --example
  bench_reference -p al-analysis`: 3 run 完走(feature
  フラグ不要)。`bench_dsp -p al-locate`: 3 run 完走
- `mise exec -- cargo test -p al-dataset --release --test
  integration a16d -- --nocapture`: Instant 出力取得
- `mise exec -- cargo test --workspace`: green(39 suite
  全件、golden 含む、0 failed)
- `mise exec -- cargo build --workspace`: green
- `mise exec -- cargo fmt --all -- --check`: green
- `mise exec -- cargo clippy --workspace --all-targets --
  -D warnings`: green
- `mise exec -- cargo tree -p al-analysis --depth 1`: 直接
  依存に al-math なし
- `git log --oneline`: char コミット `5e05cd5` が opt
  コミット `8201da8`(A6+A7/A8)・`cd3688c`(A9)・
  `289001e`(A15+A16)のすべてに先行
- `node .agents/skills/doc-status/scripts/audit_docs.js
  --type all`・`node .agents/skills/impl-doc/scripts/
  audit_impl_record.js`: 共に no findings(本 IR・索引
  更新を含む最終状態で再実行済み)

## Risks

- 本セッションの計測値は drift が大きい run であり、
  baseline `mech-cleanup-pre` との絶対差は信頼度が低い。
  採用差分は同一 run 腕比較・T2–T6 各セッションの post
  値との方向一致性・コード diff 証拠の 3 点で支持するが、
  `stft` の +5–11% のように ±5% ゲートを機械的に適用
  すると偽 regression と読める点が残る — 異常セッション
  であったことを明示しておく(将来の再計測で正常化する
  見込み。CI 回帰ゲート導入時の閾値設計の参考情報)
- `bench_dsp` の Instant 値は 3 run でも 5.6–9.6 ms と幅が
  大きく、A15 の確定的な after 値は T5 IR の 4.4 ms を
  正確値として扱う
- `analyze_pair` の −8.9% は複合 delta で、項目別寄与は
  criterion 腕比較でのみ分離可能
- manifest `duration_ms` の実メディア before/after は未
  取得(bench_reference が代替の stage 級証拠)

## Follow-ups

- なし(チェーン closeout)。A12 は棄却確定で bench 腕のみ
  残置(将来の再評価用)。A2/A5/A10/A13/A14 等の残項目は
  spec の Deferred/Out of Scope どおり別バンドルへ
