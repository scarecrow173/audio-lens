---
id: "IMPL-034EPEQYyirFg9sIVgiZPY"
type: "impl"
status: "completed"
title: "機械的 caller クリーンアップ A7/A8: csd 呼出しの inline conj(x)·y 化"
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
        path: "crates/al-analysis/src/cheap.rs"
      - type: "code"
        path: "crates/al-analysis/src/band_gain.rs"
      - type: "code"
        path: "crates/al-analysis/src/expensive/channel.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/mech-cleanup-a7-a8-csd-inlining.md"
  implemented-by: []
  depends-on:
    - "docs/plans/mechanical-caller-cleanup.md"
    - "docs/impl/ir/mech-cleanup-bench-baseline-characterization.md"
    - "docs/impl/ir/mech-cleanup-a6-bandpass-batching.md"
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

# 機械的 caller クリーンアップ A7/A8: csd 呼出しの inline conj(x)·y 化

## Summary

plan T3(design step 3 の A7A8-opt コミット相当、T2 と同一作業
セッションで実施)。`csd`(`stft.rs`、要素式 `conj(x)·y` =
`(xr*yr + xi*yi, xr*yi - xi*yr)`)を呼ぶ列挙 7 サイトを、
累積ループ内の inline スカラー複素積へ置き換えた。加算順
(bin 昇順・frame 昇順)と各サイトの auto-power 式
(`powi(2)`・`r*r+i*i`)は現行のまま維持し、`c[k]` 経由の
読みだけを inline 式へ置き換えたため累積結果は bit-identical —
下流 record/tensor(h・coherence・band gain・
`stereo.coherence_mean`・hft tensor)の `to_bits`
characterization が opt 後も green。`csd` 関数自体は
`#[cfg(test)]` の oracle が使うため public のまま残し、
consumer 4 ファイルの `use ... csd` のみ prune した。

## Context

task `docs/tasks/mech-cleanup-a7-a8-csd-inlining.md`
(TASK-034EO4hAcMtLkbpxQc8eOZ)、plan
`docs/plans/mechanical-caller-cleanup.md`(T3)、design
`docs/designs/mechanical-caller-cleanup.md`(A7/A8 の 7 サイト
表・A8 sketch・import prune)に従う。baseline・
characterization は T1、先行 opt は T2(A6、同一セッションで
`reference.rs` へ直列適用)。ユーザー指示により**本タスクでは
コミットを作成していない**(Done When の履歴順序はコミット
実施時に担保が必要)。

## Implementation

### Changes

全サイトで引数順は `conj(x)·y` の x = 第 1/2 引数側・
y = 第 3/4 引数側を現行 `csd` 呼出しから機械的に写した
(x/y を取り違えると虚部符号が反転する)。

- `crates/al-analysis/src/reference.rs`(4 サイト + import):
  - `fir_ir`(旧 :183): `cxy` 累積を inline 積へ。`cxx` の
    `powi(2)` 式は現行のまま
  - Welch(旧 :539-545): `pxy` 累積を inline 積へ。
    `pxx`/`pyy` の `r*r+i*i` 式は現行のまま
  - A8 hft(旧 :587-593): 長さ 1 `csd` + `c[0]` を除去し
    design sketch どおりスカラー `(cr, ci)` →
    `((cr*cr + ci*ci).sqrt()) / xs`。`xs` guard と
    else-`0.0` を完全保持(frames×half の微小 alloc を除去)
  - L/R coherence(旧 :872-877): `pab` 累積を inline 積へ
  - `use crate::stft::{...}` から `csd` を除去
- `crates/al-analysis/src/cheap.rs`(stereo pair、旧
  :1105-1120): `pxy.iter().enumerate()` を `0..ma.fft_size`
  の inline 積へ。frames cap `min(64)` を維持。トップレベルの
  `use crate::stft::{...}` から `csd` を除去(test oracle の
  関数内 `use crate::stft::csd` はそのまま)
- `crates/al-analysis/src/band_gain.rs`(coherence、旧 :159):
  `xy` 累積を inline 積へ。`use ... csd` を除去
- `crates/al-analysis/src/expensive/channel.rs`(旧 :102-118):
  全 bin の `csd` 生成をやめ、`for &k in &band_bins` 内で
  その bin のみ積を計算(最大の削減)。`use ... csd` を除去

### Decisions During Implementation

- **各サイトの変数名は design の pattern に統一**: inline 積は
  `let (xr, xi) = (…); let (yr, yi) = (…);` + 直接累積の形で
  統一し、`stft.rs:177` の要素式 `(xr*yr + xi*yi, xr*yi -
  xi*yr)` を逐語転記。auto-power 式は「統一」せず現行のまま
- **A8 は sketch どおりスカラー変数化**: `c[0]` 参照を残さず
  `(cr, ci)` を直接 sqrt 式へ入れた
- **`cheap.rs` の use リストは rustfmt へ委譲**: `csd` 除去で
  1 行に収まるようになったため `cargo fmt --check` 準拠の
  単一行形へ

## Related Experiments

### Adopted

<!-- No experiment logs: mechanical task per task-doc. -->

### Rejected

<!-- No rejected experiments. -->

## Bench 計測結果(post-optimization)

環境: `mise exec -- cargo bench -p al-analysis --bench
csd_accum -- --baseline mech-cleanup-pre`(criterion median、
baseline は T1 が保存した `mech-cleanup-pre` — 値は
`target/bench-baseline/csd_accum-mech-cleanup-pre.txt` の
clippy 修正後腕形状の run)。

### criterion — csd_accum(45 フレーム × 4096 bin)

| arm | baseline (median) | post-change (median) | change |
| --- | --- | --- | --- |
| per_frame_csd_call(現行形状) | 241.63 µs | 235.30 µs | −2.9% |
| per_frame_inline(採用形状) | 169.73 µs | 156.91 µs | −4.7% |
| hft_len1_csd(現行形状) | 4.5943 ms | 4.2590 ms | −7.3% |
| hft_scalar_inline(採用形状) | 567.39 µs | 585.86 µs | +5.5% |

腕形状は bench ファイル内で自己完結するため baseline 間の
差は run 間変動(`hft_scalar_inline` +5.5% は outlier 13%
混在のノイズで、採用形状間の優劣には影響しない)。採用差分:
per-frame 累積は 235.30 → 156.91 µs(約 33% 削減)、hft は
4.2590 ms → 585.86 µs(約 86% 削減)。さらに
`expensive/channel.rs` は全 bin の `csd` Vec 生成自体が消え
`band_bins` のみ積を計算するため、bench 腕が捉えない削減が
stage 経路に乗る(`bench_reference` の `analyze_pair` 計測は
`docs/impl/ir/mech-cleanup-a6-bandpass-batching.md` 参照 —
両 opt 合成で 631.8 → 561.8 ms)。

## Validation

- `mise exec -- cargo test -p al-analysis -p al-locate`: green —
  al-analysis 139(lib 93 + avsync 7 + expensive 8 + golden 19 +
  reference 12)、al-locate 71。A7/A8 characterization
  (`a7_a8_welch_consumers_are_bit_exact`・
  `a7_lr_band_coherence_tensor_is_bit_exact`・
  `a7_band_gain_m2_coherence_params_are_bit_exact`・
  `a7_freq_lr_coherence_is_bit_exact`・
  `a7_stereo_coherence_mean_is_bit_exact`)を含む全
  mechanical_cleanup テストが green
- `mise exec -- cargo test --workspace`: green(golden 含む)
- `mise exec -- cargo clippy --workspace --all-targets --
  -D warnings`: green(未使用 import なし)
- `mise exec -- cargo fmt --all -- --check`: green
- `rg 'csd\(' crates/al-analysis/src/`: production 呼出し 0 件
  (`stft.rs` の定義・`stft.rs`/`cheap.rs` test oracle のみ残存)
- `rg 'use.*csd' crates/al-analysis/src/`: consumer file の
  production import 0 件(`cheap.rs` test 内の scoped
  `use crate::stft::csd` のみ残存)

## Risks

- criterion 値は本マシンの単発 run。`hft_scalar_inline` +5.5%
  のように採用腕でも ±5% の揺らぎを含み得る
- `expensive/channel.rs` の band_bins-only 化は bench 腕で直接
  計測していない(bench の per-frame 腕は全 bin 走査のまま。
  stage 級効果は `analyze_pair` の合成計測に含まれる)

## Follow-ups

- T4(A9/A12-opt): `power()` 一元化 + HashSet gate —
  `spectral_pass`/`quant_step` 腕 + 各 characterization がゲート
- T7(measurement-ir-closeout): 全 bench の最終比較・char→opt
  履歴順序の確認(コミット実施時)
