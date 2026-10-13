---
id: "TASK-034EO4yi7Vv8c3JuAdt1Z0"
type: "task"
status: "done"
title: "機械的 caller クリーンアップ A16: 行優先累積・dom mag_sq scratch・video 融合・tensor fast path"
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
  implemented-by:
    - "docs/impl/ir/mech-cleanup-a16-row-major-fast-paths.md"
  depends-on:
    - "docs/plans/mechanical-caller-cleanup.md"
    - "docs/tasks/mech-cleanup-a15-interleave-block-writes.md"
  blocks:
    - "docs/tasks/mech-cleanup-measurement-ir-closeout.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/mechanical-caller-cleanup.md"
    - "docs/adr/simd-reduction-lane-fma.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/mechanical-caller-cleanup.md"
  references: []
  defers: []
  deferred-by: []
---

# 機械的 caller クリーンアップ A16: 行優先累積・dom mag_sq scratch・video 融合・tensor fast path

## Work

plan T6 = design の A16-opt コミット(step 6)。
mean 行優先・dom scratch・video 融合・tensor fast path
の 4 サブ項目 + A9 item の `reference.rs:653-654`
psd→power 側(A16a と同一変更単位のため本コミットへ
同梱 — design 確定)を適用する。

**A16a — `mean_psd`/`mean_power` 行優先スカラー累積 +
`reference.rs:653-654` psd→power**:

- `reference.rs:655-660`: `psd_s`/`psd_c` の消費者は
  `mean_psd_*` のみ(grep 確認済み)。`ms.power()`/
  `mc.power()` を 1 回ずつ呼び psd 行列を実体化せず、
  pow 行を row 昇順に走査して `mean_psd[k] += row[k] *
  inv`(`k` 昇順、`inv = 1.0/norm`)、最後に
  `/ frames`。行側の走査範囲は `frames =
  ms.frames().min(mc.frames())`(`:516`)の
  `.take(frames)` 防御 bound を保持する(Aligned 契約
  下では恒等だが faithful transcription のため残す)
- `expensive/mod.rs:648-651`: `power` 行列の
  `mean_power[k] += row[k]` を row 昇順で累積し
  `/ frames`。`total_power`/`one_sided_power` の消費は
  不変
- **スカラー必須**: 0.0 始まり・row 昇順加算を保つため
  vec reduction primitive へ置き換えない(加算順が変わり
  tolerance-class になる — 本バンドル唯一の tolerance
  候補だったが spec が scalar を選択)

**A16b — `dom()` の magnitude 行列除去**:

- `reference.rs:1071-1084`: `m.magnitude()` 全行列の
  実体化をやめ、呼出しごとに再利用する fft_size 長
  scratch 1 本へ `mag_sq(&m.re[f], &m.im[f],
  &mut scratch)` し、`k < half(= m.fft_size/2)` で
  `mean[k] += scratch[k].sqrt()` を累積。argmax 部
  (`skip(1)`・`total_cmp`)は不変
- **禁止**: 手書き `re*re + im*im`(magnitude() は
  `mag_sq → mag_sq_kernel → simd.mul_add_e_f64s`、
  `vec.rs:128-139` 経由で x86-v3 lane は真 FMA・
  Scalar lane と `len % lanes` tail は非融合
  `r*r+i*i` の**位置依存丸め**を持ち、手書きは再現
  不能)。`power()` 再利用も禁止(argmax(Σ√x) ≠
  argmax(Σx))
- `mag_sq` は `al_locate::dsp` facade から import
  (re-export 済み `dsp.rs:35`)。al-analysis へ直接
  `al-math` 依存を追加しない

**A16c — video luma/diff 1 pass 融合**:

- `video.rs:402-441`: 全画素 luma 和(`:427`)と prev
  diff zip-sum(`:428-440`)を 1 pass の別 accumulator
  `luma_sum`/`diff_sum` へ融合(design sketch 通り
  `frame.luma.iter().enumerate()` で `p.get(i)`)。
  `luminance.push(luma_sum / pixels)`・`diff.push(
  if prev.is_some() { diff_sum / pixels } else { 0.0 })`、
  `scratch = prev.replace(frame.luma).unwrap_or_default()`
- **保持条件**: 各和の加算順(luma 配列昇順)は不変で
  bit-identical。先頭 frame の `prev == None` は `0.0`
  リテラルを再現(`0.0/pixels` ではない)。`pixels ==
  0`(`:391`)では `0.0/0.0 = NaN` 系列が現行通り
  出る。diff 側は `zip` の silently-truncate semantics
  を `p.get(i)` で再現 — `p[i]` index 化は禁止
  (failure mode が panic へ変わる)
- ROI 和(`:413-424`)は系列別部分和で対象外

**A16d — tensor `read_range` contiguous-row fast path**:

- `tensor.rs:271-289`: local 座標 walk を最内 dim を
  「行」として扱う形へ再構成。行頭(`local[ndim-1] == 0`)
  で outer dims(`local[0..ndim-1]`)が region 内、かつ
  その chunk 行の最内 extent `[g_lo, g_lo+extent_last)`
  が region 最内 `[region[last][0], region[last][1])` に
  **完全包含**される場合のみ `copy_from_slice` で 1 行
  コピー、それ以外(部分包含を含む)は現行 per-element
  gather へ fallback
- src offset = `Σ local[d]*ext_strides[d]`(行内 offset
  0)、dst = `Σ (global_d - region[d][0]) *
  region_strides[d]`。両 layout とも最内 stride = 1 なので
  完全包含なら常に連続区間。outer-dim の src/dst base
  は行ループ外へ hoist してよい(値不変)
- `increment` は outer dims(`local[..ndim-1]`/
  `extent[..ndim-1]`)へ限定する
- 返却 `(out, region_shape)` は両経路で完全一致

等価性ゲート: T1 の characterization(`mean_psd_*`/
`mean_power`/`band_gains`/`psd_ratio`/`total_power`/
`one_sided_power`/`harmonic_freq_ratio` record、video
luminance/diff 系列、tensor `(out, region_shape)` 両経路)
が opt 前後で green。T1 で捕捉した現行 gather oracle と
の両経路比較へ A16d テストを拡張する。

再計測: `spectral_pass` criterion 腕・`bench_reference`
(A16a/b の stage 級経路)・al-dataset Instant
micro-timing・al-media 計測方式(T1 確認結果 —
Instant 区間があればそれ、なければ code inspection +
workspace bench 回帰ゲート + manifest `duration_ms`)
を再実行し、採用方式と before/after を
`docs/impl/ir/` へ転記。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/mechanical-caller-cleanup.md`(T6)
- `docs/designs/mechanical-caller-cleanup.md`(A16a–d
  sketch・行頭完全包含判定・禁止パターン・
  `reference.rs:653-654` 同梱の確定事項)
- `docs/specs/mechanical-caller-cleanup.md`(A16 要件)
- `docs/adr/simd-reduction-lane-fma.md`(A16b の
  `mag_sq` lane/tail 丸め根拠)

## Dependencies

- `docs/tasks/mech-cleanup-bench-baseline-characterization.md`
  (T1: characterization + `spectral_pass` 腕 + Instant
  経路 + baseline)
- `docs/tasks/mech-cleanup-a15-interleave-block-writes.md`
  (T5: `reference.rs` の先行 opt — 直列適用)

## Affected Files

- `crates/al-analysis/src/reference.rs`(`:653-654`
  psd→power + `:655-660` 行優先 + `:1071-1084` dom
  scratch + `mag_sq` import)
- `crates/al-analysis/src/expensive/mod.rs`(`:648-651`
  mean_power 行優先)
- `crates/al-media/src/video.rs`(`:402-441` 1 pass 融合)
- `crates/al-dataset/src/tensor.rs`(`:271-289` fast path)
- `crates/al-dataset/tests/integration.rs` または
  `tensor.rs` `mod tests`(両経路比較への拡張)
- `docs/impl/ir/<record>.md`(A16 before/after 転記)

## Done When

- [ ] `mean_psd_*`/`mean_power` が scalar row-ascending
  累積(0.0 始まり・row 昇順・`k` 昇順・`/ frames`)
  で、`.take(frames)` 防御 bound を保持し、vec
  reduction primitive を使っていない
- [ ] `reference.rs:653-654` が `power()` 1 回ずつ +
  `row[k] * inv` で psd 行列を実体化していない
- [ ] `dom()` が再利用 `mag_sq` scratch + `sqrt` 累積
  (`k < half`)で、手書き `re*re+im*im` がなく、
  argmax 部(`skip(1)`・`total_cmp`)が不変
- [ ] `mag_sq` が `al_locate::dsp` facade 経由で
  import され、al-analysis の直接 `al-math` 依存がない
- [ ] video が 1 pass 融合で `p.get(i)` 使用・先頭
  `diff=0.0` リテラル・`pixels==0` NaN 系列を保持
- [ ] tensor fast path が行頭完全包含に限定され部分
  包含が per-element gather fallback へ落ちる
- [ ] T1 の A16 系 characterization が opt 前後両時点
  で green(record/系列/`(out, region_shape)`)
- [ ] 各計測経路の before/after と採用方式が IR に
  記録されている
- [ ] 本コミットが先行する全 char/opt コミットより後の
  git 履歴にある

## Verification

- [ ] `mise exec -- cargo test -p al-analysis`・
  `cargo test -p al-media`・`cargo test -p al-dataset`
  が green
- [ ] `rg 're\s*\*\s*re|im\s*\*\s*im' crates/al-analysis/
  src/reference.rs` の `dom()` 周辺に手書き二乗和が
  ない(コード検査)
- [ ] `mise exec -- cargo bench -p al-analysis --bench
  spectral_pass -- --baseline <name>` で比較が実行できる
- [ ] `mise exec -- cargo run --example bench_reference
  --release -p al-analysis` が完走し baseline との差分
  を確認済み
- [ ] `mise exec -- cargo tree -p al-analysis --depth 1`
  の直接依存に al-math が現れない
- [ ] `mise exec -- cargo clippy --workspace --all-targets
  -- -D warnings`・`mise exec -- cargo fmt --all --
  --check` が green
- [ ] `mise exec -- cargo test --workspace` が green
  (golden 含む)

## Out of Scope

- `mean_psd`/`mean_power`/`mean_row` の vec reduction 化
  (tolerance-class、別バンドル)
- `StftMatrix` への新規メソッド追加(禁止)
- ROI 和(`video.rs:413-424`)・単一消費者例外
  (`cheap.rs:942`・`reference.rs:1189`)
- al-dataset/al-media の新規 dev-dep・bench harness 追加
