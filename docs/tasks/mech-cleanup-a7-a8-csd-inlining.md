---
id: "TASK-034EO4hAcMtLkbpxQc8eOZ"
type: "task"
status: "todo"
title: "機械的 caller クリーンアップ A7/A8: csd 呼出しの inline conj(x)·y 化"
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
    - "docs/plans/mechanical-caller-cleanup.md"
  implemented-by: []
  depends-on:
    - "docs/plans/mechanical-caller-cleanup.md"
    - "docs/tasks/mech-cleanup-a6-bandpass-batching.md"
  blocks:
    - "docs/tasks/mech-cleanup-a9-a12-cheap-pass-consolidation.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/mechanical-caller-cleanup.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/mechanical-caller-cleanup.md"
  references: []
  defers: []
  deferred-by: []
---

# 機械的 caller クリーンアップ A7/A8: csd 呼出しの inline conj(x)·y 化

## Work

plan T3 = design の A7A8-opt コミット(step 3)。`csd`
(`stft.rs:173-179`、要素式 `conj(x)·y` =
`(xr*yr + xi*yi, xr*yi - xi*yr)`)を呼ぶ列挙 7 サイトを、
累積ループ内の inline スカラー複素積へ置き換える(同一
イディオムのため 1 コミット)。`c[k]` 経由の読みのみを
inline 式へ置き換え、加算順(bin 昇順・frame 昇順)と
各サイトの auto-power 式(`powi(2)`・`r*r+i*i`)は
現行のまま維持する。

サイト一覧(design の表、file:line は現 HEAD で再確認):

| Site | csd → 累積器 | 条件 |
|------|--------------|------|
| `reference.rs:183` (fir_ir) | `cxy` | 全 bin で inline 積、`cxx` の `powi(2)` 式は現行のまま |
| `reference.rs:539-545` (Welch) | `pxy` | 同上、`pxx`/`pyy` の `r*r+i*i` 式は現行のまま |
| `reference.rs:872-877` (L/R coherence) | `pab` | 同上 |
| `band_gain.rs:159` (coherence) | `xy` | 同上 |
| `cheap.rs:1105-1120` (stereo pair) | `csd_re`/`csd_im` | `pxy.iter().enumerate()` を `0..ma.fft_size` の inline 積へ。frames cap `min(64)` 維持 |
| `expensive/channel.rs:102-118` | `sxy` | `for &k in &band_bins` 内でその bin のみ積を計算(全 bin の csd 生成をやめる — 最大の削減) |
| `reference.rs:587-593` (A8 hft) | スカラー `(cr, ci)` | 長さ 1 の `csd` 呼出し + `c[0]` を除去し `((cr*cr + ci*ci).sqrt()) / xs`(現行 :593 と同じ式)。`xs` guard と else-`0.0` を正確に保持 |

- **手続き**: 引数順の取り違え(`conj(x)·y` の x/y)を
  防ぐため、現行の `csd` 呼出し引数と戻り値使用箇所を
  機械的に写す。auto-power 式を「統一」しない
- **`csd` 関数は public のまま残す**(`stft.rs:390` の
  `#[cfg(test)]` が使用)。削除しない
- **import prune(同一コミット必須)**: 全 production
  `csd` 呼出しが消えるため、`reference.rs:18`・
  `cheap.rs:16`・`band_gain.rs:16`・
  `expensive/channel.rs:9` の use リストから `csd` を
  除去する(残らなければ clippy `-D warnings` で fail)。
  `stft.rs`/`dsp.rs` の公開面は変えない
- 等価性ゲート: T1 の characterization(下流 record
  h・coherence・band gain・`stereo.coherence_mean`・hft
  tensor の `to_bits`)が opt 前後で green
- 再計測: `csd_accum` criterion 腕を `--baseline` で
  比較し before/after を `docs/impl/ir/` へ転記

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/mechanical-caller-cleanup.md`(T3)
- `docs/designs/mechanical-caller-cleanup.md`(A7/A8 の 7
  サイト表・A8 sketch・import prune)
- `docs/specs/mechanical-caller-cleanup.md`(A7/A8 要件)

## Dependencies

- `docs/tasks/mech-cleanup-bench-baseline-characterization.md`
  (T1: characterization + `csd_accum` 腕 + baseline)
- `docs/tasks/mech-cleanup-a6-bandpass-batching.md`
  (T2: `reference.rs` の先行 opt — 直列適用)

## Affected Files

- `crates/al-analysis/src/reference.rs`(`:183`・
  `:539-545`・`:872-877`・`:587-593` 置換 + `:18`
  `csd` import prune)
- `crates/al-analysis/src/cheap.rs`(`:1105-1120` stereo +
  `:16` `csd` import prune)
- `crates/al-analysis/src/band_gain.rs`(`:159` + `:16`
  `csd` import prune)
- `crates/al-analysis/src/expensive/channel.rs`
  (`:102-118` `band_bins` のみ積 + `:9` `csd` import
  prune)
- `docs/impl/ir/<record>.md`(A7/A8 before/after 転記)

## Done When

- [ ] 列挙 7 サイトに `csd(` 呼出しが残らず、各累積器が
  inline `conj(x)·y` 積で同じ加算順・auto-power 式を
  維持している
- [ ] `expensive/channel.rs` が `band_bins` の bin のみ
  積を計算する(全 bin の csd 生成が消えている)
- [ ] A8 が長さ 1 `csd` + `c[0]` をスカラー `(cr, ci)`
  + `((cr*cr + ci*ci).sqrt()) / xs` へ置き換え、`xs`
  guard・else-`0.0` を保持している
- [ ] `cheap.rs` stereo 経路の `min(64)` frames cap が
  保持されている
- [ ] consumer file 4 箇所の `csd` import が prune 済みで、
  `stft.rs` の `csd` 定義は public のまま
- [ ] T1 の A7/A8 characterization(下流 record・hft
  tensor `to_bits`)が opt 前後両時点で green
- [ ] `csd_accum` 腕の before/after が IR に記録されている
- [ ] 本コミットが T1 の char コミット・T2 の A6-opt
  コミットより後の git 履歴にある

## Verification

- [ ] `mise exec -- cargo test -p al-analysis` が green
  (reference・expensive・cheap の characterization 含む)
- [ ] `rg 'csd\(' crates/al-analysis/src/` が 0 件
  (`stft.rs` の定義と `#[cfg(test)]` 使用のみ残る)
- [ ] `rg 'use.*csd' crates/al-analysis/src/` が 0 件
- [ ] `mise exec -- cargo bench -p al-analysis --bench
  csd_accum -- --baseline <name>` で比較が実行できる
- [ ] `mise exec -- cargo clippy --workspace --all-targets
  -- -D warnings`・`mise exec -- cargo fmt --all --
  --check` が green(未使用 import が残らない)
- [ ] `mise exec -- cargo test --workspace` が green
  (golden 含む)

## Out of Scope

- `csd` 関数本体の変更・削除(`stft.rs:390` test が使用)
- A9/A12(T4)・A15(T5)・A16(T6)
- `stft.rs`/`dsp.rs` の公開面変更
