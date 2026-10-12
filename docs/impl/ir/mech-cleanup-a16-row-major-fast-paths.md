---
id: "IMPL-034EQzH5olUVFjLCPhPxt7"
type: "impl"
status: "completed"
title: "機械的 caller クリーンアップ A16: 行優先累積・dom mag_sq scratch・video 融合・tensor fast path"
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
      - type: "code"
        path: "crates/al-analysis/src/expensive/mod.rs"
      - type: "code"
        path: "crates/al-media/src/video.rs"
      - type: "code"
        path: "crates/al-dataset/src/tensor.rs"
      - type: "doc"
        path: "docs/tasks/mech-cleanup-a16-row-major-fast-paths.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/mech-cleanup-a16-row-major-fast-paths.md"
  implemented-by: []
  depends-on:
    - "docs/plans/mechanical-caller-cleanup.md"
    - "docs/impl/ir/mech-cleanup-bench-baseline-characterization.md"
    - "docs/impl/ir/mech-cleanup-a15-interleave-block-writes.md"
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

# 機械的 caller クリーンアップ A16: 行優先累積・dom mag_sq scratch・video 融合・tensor fast path

## Summary

plan T6(design step 6 の A16-opt コミット相当)。4 サブ項目
+ A9 item の `reference.rs` psd→power 側(A16a と同一変更
単位のため design 確定どおり同梱)を適用した。

- **A16a**: `mean_psd_s`/`mean_psd_c`/`mean_power` を列走査
  (`(0..bins).map(|k| rows.map(|r| r[k]).sum() / frames)`)
  から 0.0 始まり・row 昇順・`k` 昇順のスカラー累積 +
  `/ frames` へ。`reference.rs` 側は `ms.psd()`/`mc.psd()`
  を `ms.power()`/`mc.power()` 1 回ずつへ置き換え、
  `row[k] * inv`(`inv = 1.0 / norm`、norm =
  `(0.375 * window) * sample_rate`)で psd 行列を実体化
  しない。`.take(frames)` 防御 bound を保持。vec
  reduction primitive は不使用(加算順維持のため)
- **A16b**: `dom()` の `m.magnitude()` 全行列実体化を、
  再利用 `fft_size` scratch への per-frame `mag_sq` +
  `scratch[k].sqrt()` 累積(`k < half`)へ。argmax 部
  (`skip(1)`・`total_cmp`)は不変
- **A16c**: video の全画素 luma 和と prev diff zip-sum を
  1 pass の `luma_sum`/`diff_sum` へ融合(`p.get(i)` で
  zip の silently-truncate を再現、`None => 0.0`
  リテラル・`pixels == 0` の NaN 系列を保持)
- **A16d**: `read_range` の local walk を最内 dim を行と
  して扱う形へ再構成。行頭で outer dims inside + 最内
  extent 完全包含なら `copy_from_slice` 1 行コピー、部分
  包含は per-element gather fallback

全項目 bit-identical(A16d は返却 `(out, region_shape)`
完全一致)。T5(A15)と同一作業セッションで適用したが
変更ファイルは分離されている。

## Context

task `docs/tasks/mech-cleanup-a16-row-major-fast-paths.md`
(TASK-034EO4yi7Vv8c3JuAdt1Z0)、plan
`docs/plans/mechanical-caller-cleanup.md`(T6)、design
`docs/designs/mechanical-caller-cleanup.md`(A16a–d
sketch・`reference.rs:653-654` 同梱の確定事項)、ADR
`docs/adr/simd-reduction-lane-fma.md`(A16b の
`mag_sq` 必須根拠)に従う。baseline・characterization は
T1、先行 opt は T2–T5(直列適用済み)。ユーザー指示により
**本タスクではコミットを作成していない**(Done When の
履歴順序はコミット実施時に担保が必要)。

## Implementation

### Changes

- `crates/al-analysis/src/reference.rs`:
  - `use al_locate::dsp` に `mag_sq` を追加(facade
    re-export 経由、al-math 直接依存なし)
  - 旧 `:655-662`: `let psd_s = ms.psd(); let psd_c =
    mc.psd();` + 列走査 mean を `pow_s`/`pow_c` =
    `power()` 1 回ずつ + `mean_psd_*[k] += row[k] * inv`
    (`.iter().take(frames)` 保持)→ `*v /= frames as f64`
    へ。`norm` は `psd()` 内の
    `window_power() * sample_rate` と同じ
    `(0.375 * ms.window as f64) * ms.sample_rate as f64`
    で評価順も同一
  - 旧 `:1071-1084` `dom()`: `m.magnitude()` を除去し、
    呼出しごとに 1 本の `scratch = vec![0.0; m.fft_size]`
    へ `mag_sq(&m.re[f], &m.im[f], &mut scratch)` して
    `mean[k] += scratch[k].sqrt()`(`k < half =
    m.fft_size / 2`)を累積。手書き `re*re + im*im` なし
  - residual interleave(旧 `:1165-1173`)は A15(T5)で
    別途変更済み
- `crates/al-analysis/src/expensive/mod.rs`(旧 `:648-651`):
  `power` 行列の列走査 mean を `mean_power[k] += row[k]`
  row 昇順スカラー累積 + `/ frames` へ。`total_power`・
  `one_sided_power`・`RegionCtx` 消費は不変
- `crates/al-media/src/video.rs`(旧 `:427-440`):
  `frame.luma.iter().enumerate()` 1 pass で `luma_sum`・
  `diff_sum` を累積。`prev` が `Some` の枝では
  `p.get(i)` で zip truncate を再現、`None` 枝は luma
  のみ走査して `diff` に `0.0` リテラルを push。
  `prev` は `prev.replace(frame.luma)` 前に判定
- `crates/al-dataset/src/tensor.rs`(旧 `:271-289`):
  `local` walk を `local[..last]`/`extent[..last]` の
  outer-dim counter に限定し、行単位で `inside`・
  `src`/`dst` base を hoist。chunk 単位の
  `row_fully_contained`(最内 `[g_lo, g_lo+extent_last)` ⊆
  region 最内)が真なら `copy_from_slice` で 1 行コピー、
  偽(部分包含を含む)なら `0..extent_last` の
  per-element gather へ fallback(最内 stride = 1 を利用)

### Decisions During Implementation

- **A16a の `norm`/`inv` は `ms` から 1 本だけ導出**:
  `ms`/`mc` は同一 `cfg.stft_window`・同一 `rate` で構築
  されるため `norm` は bit 同一。`scale(&power, 1.0/norm)`
  の要素乗算と `row[k] * inv` は同じ演算で bit-identical
- **A16c の `prev` 判定をループ外へ hoist**: design
  sketch は要素ループ内で `if let Some(p) = &prev` を書く
  が、同 cost で同じ semantics のため `if let Some(p)` 外側
  + `else` 側 luma のみループへ整理(加算順はいずれも
  luma 昇順で不変)
- **A16d の `row_fully_contained` は chunk 単位で hoist**:
  `g_lo`/`g_hi` は `local` walk に依らず chunk coords のみ
  で決まるため inner loop の外へ。design の「行頭判定」を
  満たしつつ比較回数を最小化(outer-dim の inside 判定は
  行単位で残る — 部分包含 fallback 側は行内 per-element)
- **A16d の fallback は innermost のみ per-element**:
  outer dims が inside の行に限定されるため、要素単位の
  残り判定は最内 dim のみで現行 `inside` と同値
- **ndim == 1 退化ケース**: `local[..0]` の `increment`
  は即 `false` を返し「outer dims なしの 1 行」として全
  extent を 1 行扱い — 現行 walk と同じ結果
- テスト追加なし: T1 が設置した characterization が
  fast path・fallback・部分包含 edge・両経路
  `(out, region_shape)` 比較を既にカバー
  (`a16d_read_range_full_inner_dim_is_bit_exact`・
  `a16d_read_range_partial_inner_dim_edge_is_bit_exact`・
  `a16d_read_range_micro_timing_baseline` の 3 経路 assert)

## Related Experiments

### Adopted

<!-- No experiment logs: mechanical task per task-doc. -->

### Rejected

<!-- No experiment logs: mechanical task per task-doc. -->

## Bench 計測結果(post-optimization)

環境: `mise exec -- cargo bench -p al-analysis --bench
spectral_pass -- --baseline mech-cleanup-pre`(criterion
median、baseline は T1 が保存した `mech-cleanup-pre`)。
生ログは `target/bench-baseline/spectral_pass-mech-cleanup-
a16.txt`。Instant 系は `bench_dsp-a15.txt`・
`bench-reference-t6.txt`・al-dataset release test run。

### criterion — spectral_pass(127 フレーム × 4096 bin)

| arm | baseline (median) | post-change (median) | change |
| --- | --- | --- | --- |
| three_passes_mag_pow_psd(旧形状) | 6.1926 ms | 4.8736 ms | −21.3% |
| single_power_derived(採用形状) | 3.4120 ms | 3.1041 ms | −9.0% |
| mean_column_scan(旧形状) | 553.73 µs | 469.91 µs | −9.7% |
| mean_row_major(採用形状) | 151.44 µs | 141.06 µs | −9.6% |
| dom_magnitude_matrix(A16b 旧) | 1.2490 ms | 981.63 µs | −17.5% |
| dom_mag_sq_scratch(A16b 採用) | 446.68 µs | 377.51 µs | −21.0% |

腕コードは bench ファイル内で自己完結・不変のため、全腕の
一律改善は baseline 採取時との run 環境 drift(前回 T4 run
でも同傾向を記録)。**同一 run 内の腕比較が採用差分の実測**:
今回 run では mean 列走査 470 µs → row-major 141 µs(約
3.3 倍)、dom magnitude 行列 982 µs → mag_sq scratch
378 µs(約 2.6 倍)。baseline 時点の腕比較(554→151 µs・
1249→447 µs)が long-run の採用期待値で、方向は両 run で
一致。

### Instant — bench_reference(3 回以上 run、うち clean 4 run
の mean。生ログ `target/bench-baseline/bench-reference-t6.txt`)

| 計測点 | pre (mean) | post T2+T3 (t5) | post T4 (mean) | post T5+T6 (clean 4 run mean) |
| --- | --- | --- | --- | --- |
| stft 1M samples (4096/2048) | 32.5 ms | ~30.9 ms | ~32.7 ms | ~32.3 ms (33.1/31.5/33.0/31.4) |
| analyze_pair 2s mono (R0–R3, fir_model) | 631.8 ms | ~580.5 ms | ~548.6 ms | ~570.8 ms (582.1/587.2/554.9/559.1) |
| prepare_residual 2s 2src × 2cap | 10.9 ms | ~7.1 ms | ~7.8 ms | ~7.8 ms (8.1/7.2/8.2/7.8) |

- 初回 2 run は環境 noise(873.4 ms・1384.9 ms)で除外し、
  clean 4 run のみを mean 化。pre 比 `analyze_pair` は
  −9.6%(631.8 → 570.8 ms、T2–T6 累積)。t4 比では
  548.6 → 570.8 ms と見かけ回帰するが、t4/t5/t6 で run
  環境 drift(不変計測点 `prepare_residual` が 7.1–7.8 ms
  で変動、`stft` も 30.3–37.2 ms)が大きく、A16 単独効果は
  この計測点では noise 以内。spectral_pass 同一 run の腕
  比較(mean 3.3 倍・dom 2.6 倍)が stage 級効果の根拠

### Instant — al-dataset A16d micro-timing(release、
`tests/integration.rs` の Instant 区間、`--nocapture`)

T1 baseline は `docs/impl/ir/mech-cleanup-bench-baseline-
characterization.md` 記録値。post は同一 release test
binary の 3 連続 run(初回のみ noise を除外)。

| region | pre (1 run) | post (3 run 帯域) | delta |
| --- | --- | --- | --- |
| contiguous_rows [100,2000)×[0,256)(486_400 vals) | 17.431 ms | 15.09–15.30 ms | 約 −13% |
| partial_inner [100,2000)×[64,160)(182_400 vals) | 7.5101 ms | 6.31–6.74 ms | 約 −11–16% |
| one_chunk [0,64)×[0,64)(4_096 vals) | 183.7 µs | 144.6–150.0 µs | 約 −18–21% |

`read_range` の計時には chunk file I/O + f64le parse が
支配的に含まれるため、gather→block copy の改善分は読み出し
全体の 1 割強に留まる。partial_inner は完全包含 chunk が
fast path へ、非包含 chunk が per-element fallback へ振り
分けられる混合経路。

### al-media(A16c)— spec fallback 記録

T1 で確認済みのとおり al-media に Instant/example 計時経路は
存在しないため、plan/spec の fallback 規定どおり **code
inspection + characterization 系列 `to_bits` + workspace
回帰ゲート**を採用:

- code inspection: 2 pass → 1 pass で per-frame の luma 走査
  が半減、`p.get(i)` 分岐は `Some` 側のみで zip と同じ
  short-circuit。追加の allocation・呼出しなし
- characterization: `a16c_luma_diff_series_is_bit_exact`・
  `a16c_length_mismatch_truncates_via_zip`・
  `a16c_zero_pixel_degenerate_series_is_pinned` が opt 後も
  bit-exact green
- workspace 回帰ゲート: `cargo test --workspace` green・
  `cargo clippy -D warnings` clean。stage 級 manifest
  `duration_ms` の計測は baseline 未採取のため記録しない
  (T1 IR の方式決定どおり)

## Validation

- `mise exec -- cargo test -p al-analysis`: green — 139 件
  (lib 93 + avsync 7 + expensive 8 + golden 19 +
  reference 12)。`a16a_band_gain_psd_ratio_is_bit_exact`・
  `a16a_mean_power_consumers_are_bit_exact`・
  `a16b_harmonic_freq_ratio_is_bit_exact` を含む
- `mise exec -- cargo test -p al-media`: green — A16c の
  系列 `to_bits` 3 本を含む
- `mise exec -- cargo test -p al-dataset`: green — A16d の
  full-inner/partial-inner bit-exact + micro-timing 経路の
  `(vals, rshape)` assert を含む
- `mise exec -- cargo test --workspace`: green(39 スイート
  全件、golden 含む)
- `rg 're\s*\*\s*re|im\s*\*\s*im' crates/al-analysis/src/
  reference.rs`: `dom()` 周辺に手書き二乗和なし(残留する
  1 箇所は `:915` `lr_band_coherence` のスカラー和であり
  A16b 対象外・変更なし)
- `grep -n "\.psd()\|\.magnitude()" crates/al-analysis/src/
  reference.rs`: production は `rm.psd()`(単一消費者例外、
  現 `:1218`)のみ。`m.magnitude()` は `dom()` から除去済み
- `mise exec -- cargo tree -p al-analysis --depth 1`: 直接
  依存に al-math なし(al-core・al-dataset・al-job・
  al-locate・al-media・serde・serde_json・tracing)
- `mise exec -- cargo bench -p al-analysis --bench
  spectral_pass -- --baseline mech-cleanup-pre`: 全腕完走、
  比較出力を `spectral_pass-mech-cleanup-a16.txt` へ保存
- `mise exec -- cargo run --release --example
  bench_reference -p al-analysis`: 6 run 完走(うち noise
  2 run 除外)、`bench-reference-t6.txt` へ保存
- `mise exec -- cargo test -p al-dataset --release --test
  integration a16d -- --nocapture`: Instant 出力取得
- `mise exec -- cargo clippy --workspace --all-targets --
  -D warnings`・`mise exec -- cargo fmt --all -- --check`:
  green

## Risks

- criterion・Instant 値は本マシンの run。baseline 採取時と
  の環境 drift が大きく(不変腕でも −10〜−21% 変動)、
  絶対差より同一 run 内の腕比較を信頼すること。T4 IR と
  同じ注意
- `bench_reference` の `analyze_pair` は stage 級で複数
  要因(A6/A7/A9/A15/A16)が混ざるため、T6 単独の寄与は
  分離不能。spectral_pass の腕比較が代替根拠
- A16d fast path は「outer dims inside + 最内完全包含」に
  限定されており、outer 部分包含(行の一部だけ region 外)
  では行ごと skip 判定となる。返却は両経路で完全一致を
  characterization が pin
- `dom()` の `mag_sq` scratch は呼出し(closure 呼び)ごと
  に 1 本の確保。`dom` は ms/mc の 2 回のみ呼ばれるため
  追加の共有化は不要(YAGNI)

## Follow-ups

- T7(measurement-ir-closeout): 全 bench の最終比較・
  char→opt 履歴順序の確認(コミット実施時)・`audit_docs`・
  `audit_impl_record` の最終 clean 確認
