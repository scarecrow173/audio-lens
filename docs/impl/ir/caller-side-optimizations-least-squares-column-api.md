---
id: "IMPL-034ELdrUTsy60thZlpbYPE"
type: "impl"
status: "completed"
title: "caller-side optimizations: LeastSquares column-oriented API replacement"
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
    - "docs/tasks/caller-opt-least-squares-column-api.md"
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
    - "docs/adr/caller-side-optimizations-structure.md"
    - "docs/test-specs/caller-side-optimizations.md"
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

# caller-side optimizations: LeastSquares column-oriented API replacement

## Summary

plan T3(A4 opt コミット)。`LeastSquares` の公開入口を行指向
`new(&[Vec<f64>])` から列指向 `from_columns(&[&[f64]], λ)` へ
置換し(pre-release 方針で両建てなし)、`fit_mix` を
`&[&[f64]]` シグネチャへ移行した。これにより
`mixer.rs` の per-sample 行マテリアライズ(m×n の `Vec<Vec<f64>>`
行構築)、`linalg.rs` 内部の strided 列再収集と行セットの
`to_vec` clone、`reference.rs` のチャネル `to_vec` コピー、
`stage.rs` の `cap_window` per-channel `to_vec` がすべて除去
された。RSS は ridge 項を含めない現行契約のまま、連続バッファ上の
`sub_scaled_kernel` + `sum_sq_kernel` パスで計算する。

## Context

task `docs/tasks/caller-opt-least-squares-column-api.md`
(TASK-034EJgyjQ0EutoP1M6RqoQ)、design
`docs/designs/caller-side-optimizations.md` の A4 節(列正本・
`obs_columns`・RSS 設計・`fit_mix` シグネチャ・caller 移行
マップ・`subtract_fit` 最小化)、ADR
`docs/adr/caller-side-optimizations-structure.md`
(`from_columns` API 形・rows API 置換・RSS 非 ridge 契約・
fit_mix 移行確定)、test-spec
`docs/test-specs/caller-side-optimizations.md`(Rule D
predicate)に従う。依存タスク T1(bench baseline・
characterization)と T2(stft plan hoist)は land 済み。

## Implementation

### Changes

- `crates/al-math/src/linalg.rs`
  - `LeastSquares::from_columns(columns: &[&[f64]], lambda) ->
    Option<Self>` を追加し `LeastSquares::new` を除去。
    契約は `new` の列向き写し: 列が1本以上・列長(観測数 m)≥
    列数 n・全列等長・全要素有限・λ が非負有限、のいずれかを
    欠けば `None`
  - `least_squares` free fn は列シグネチャ
    `least_squares(columns: &[&[f64]], rhs, lambda)` に置換
    (al-locate root re-export 面は関数名不変で維持)
  - 内部表現を列正本へ: `obs_columns`(ridge 適用前の入力列、
    RSS 用に 1 コピーのみ)、`columns`(√λ 対角を `extra`
    要素として末尾へ追加した augmented 列の分解結果、長さ
    m+extra)、`reflectors`。`Factor` は入力列を `to_vec` で
    直接消費し strided gather を行わない
  - `solve(rhs)` は `rhs.len() == m` かつ全要素有限を要求。
    `Apply` は `y = rhs` を m+extra に zero-pad して反射適用・
    後退代入(不変)
  - `Apply` の RSS: `resid = rhs.to_vec()` に
    `for (j, &xj) { sub_scaled_kernel(simd, &mut resid, xj,
    &obs_columns[j]) }` の後 `sum_sq_kernel(&resid)`、
    `rms = (rss / m).sqrt()`(ridge 項非含有、ADR 確定)
  - `mod tests` を列 fixture へ全面書き換え。`mod legacy`
    (行ベース normal-equations oracle)は残し、テスト内で
    列→行転置(`rows_of`)して対比
  - `condition_number` は無変更(入力は正方行列のまま)
- `crates/al-math/benches/lstsq.rs`: 列シグネチャへ追従。
  benchmark ID `new`(factor 構築)・`solve` を維持し、
  `--baseline lstsq-rows` が行 API baseline と直接 delta を
  報告する形にした。入力データは baseline と同じ値の転置
  (同種の workload)
- `crates/al-locate/src/mixer.rs`
  - `fit_mix(source_channels: &[&[f64]], capture_channels:
    &[&[f64]], ridge) -> Option<MixerEstimate>` に変更
  - per-sample 行 `Vec` 構築(`rows` マテリアライズ)を除去
  - AᵀA 集計を列上の `dsp::dot` に置換
    (`ata[i][j] = dsp::dot(source_channels[i],
    source_channels[j])`、実装簡潔さ優先で全要素計算)
  - `LeastSquares::from_columns(source_channels, ridge)` を呼ぶ。
    解ループ・`dsp::rms(cap)`・`MixerEstimate` 組立ては不変
  - テストは呼出し側のみ `&[&s0[..], …]` / `Vec<&[f64]>` 参照へ
    追従。`correlated_mix_...` の AᵀA 参照も列 `dsp::dot` 累算に
    揃えた(後述)
- `crates/al-locate/src/residual.rs`: `subtract_fit` の公開
  シグネチャ `&[Vec<f64>]` は維持し、内部で `Vec<&[f64]>` 列
  参照へ変換 — `c.len() >= len` は `&c[..len]`(コピーなし)、
  `c.len() < len` のみ zero-pad した所有バッファに逃がす
  (design の最小化方針)。`capture_channels.to_vec()` と
  per-source `sub_scaled` 残差ループは `src_cols` で維持
- `crates/al-locate/src/stage.rs`: mixer fit 導出部で
  `cap_window` を per-channel `to_vec` からスライス参照
  `Vec<&[f64]>` へ、`source.channel_samples` も
  `Vec::as_slice` 参照に変更(データコピーなし)
- `crates/al-locate/src/dsp.rs`: `least_squares` テストを列
  fixture(`&[c0.as_slice(), c1.as_slice()]`)へ追従
- `crates/al-analysis/src/reference.rs`: `fit_mix` 呼出し3箇所を
  移行 — `src_chs`/`cap_chs` の `to_vec` チャネルコピーを除去し
  `src.src`/`src.cap`(`Vec<&[f64]>`)を直接渡す(mixer_fit・
  crosstalk)。R2 残差 fit は `r1_refs: Vec<&[f64]> =
  r1.iter().map(Vec::as_slice).collect()` で参照のみ渡す

### Decisions During Implementation

- **bench ID を `new`/`solve` のまま維持**: criterion baseline
  `lstsq-rows` は `lstsq/new/*`・`lstsq/solve/*` の ID に保存
  されているため、ID を変えると `--baseline` 比較が機能しない。
  workload は同一値の転置のみで計測経路は列 API。doc コメント
  で `new` = factor 構築(`from_columns`)である旨を明記
- **`correlated_mix_...` の参照 AᵀA を列 `dsp::dot` 累算に
  変更**: production が行累算から列 `dsp::dot`(lane
  アキュムレータ)へ変わったため、in-test の行累算 `ata` との
  `assert_eq!` は ulps で割れ得る。テストの意図(診断が ridge
  前の正規行列で ridge に依存しないこと)は `est ==
  regularized` の等価性が担い、値一致側は同じ累算順の参照を
  使う形に揃えた
- **`subtract_fit` は長さ合わせのみ所有**: design の最小化方針
  どおり `c.len() >= len` は `&c[..len]` で借用し、短い列だけ
  clone+resize 相当の padded バッファに逃がす。公開シグネチャ・
  `lib.rs` re-export 面・`stage.rs`/`tests/locate.rs` の caller
  は無変更
- **空の `columns` スライスは `columns.first()?` で `None`**:
  `n == 0` は列0本として弾かれ、旧 `cols == 0` 拒否と同じ
  結果。`m == 0`(長さ0の列)は `m < n ≥ 1` で `None` に
  なり、旧 `vec![vec![]]` ケースと一致

## Related Experiments

### Adopted

<!-- No experiment logs: mechanical task per task-doc. -->

### Rejected

<!-- No rejected experiments. -->

## Bench 結果(baseline `lstsq-rows` との delta)

環境: 同一マシン上の `mise exec -- cargo bench -p al-math
--bench lstsq -- --baseline lstsq-rows`(bench profile =
release + debuginfo)。完全出力は
`target/bench-baseline/criterion-lstsq-a4.txt`(gitignore 済み)。

group `lstsq`、λ=1e-6、median:

| bench | baseline `lstsq-rows` | A4 適用後 | change(criterion) |
| --- | --- | --- | --- |
| new/1x65536 | 4.4535 ms | 1.5616 ms | −65.24%(CI [−66.0%, −64.4%], p = 0.00, improved) |
| solve/1x65536 | 750.53 µs | 697.93 µs | −4.87%(CI [−6.7%, −3.0%], p = 0.00, improved) |
| new/1x262144 | 19.715 ms | 6.4538 ms | −67.27%(CI [−67.7%, −66.7%], p = 0.00, improved) |
| solve/1x262144 | 4.0318 ms | 3.9848 ms | −1.17%(CI [−5.1%, +2.6%], p = 0.54, no change) |
| new/1x1048576 | 83.266 ms | 25.353 ms | −69.55%(CI [−70.4%, −68.7%], p = 0.00, improved) |
| solve/1x1048576 | 20.099 ms | 17.533 ms | −12.77%(CI [−15.4%, −9.8%], p = 0.00, improved) |
| new/2x65536 | 6.2268 ms | 2.7481 ms | −55.87%(CI [−56.7%, −55.1%], p = 0.00, improved) |
| solve/2x65536 | 1.0319 ms | 1.0863 ms | +7.37%(CI [+4.4%, +10.4%], p = 0.00, regressed) |
| new/2x262144 | 29.384 ms | 12.751 ms | −56.60%(CI [−57.0%, −56.2%], p = 0.00, improved) |
| solve/2x262144 | 5.5069 ms | 6.3920 ms | +16.07%(CI [+11.9%, +21.0%], p = 0.00, regressed) |
| new/2x1048576 | 123.45 ms | 57.124 ms | −53.73%(CI [−54.5%, −53.0%], p = 0.00, improved) |
| solve/2x1048576 | 22.260 ms | 24.101 ms | +8.27%(CI [+5.9%, +10.6%], p = 0.00, regressed) |
| new/4x65536 | 10.808 ms | 7.2211 ms | −33.19%(CI [−34.6%, −31.5%], p = 0.00, improved) |
| solve/4x65536 | 2.4524 ms | 1.8009 ms | −26.57%(CI [−28.0%, −25.1%], p = 0.00, improved) |
| new/4x262144 | 50.211 ms | 37.924 ms | −24.47%(CI [−26.0%, −22.7%], p = 0.00, improved) |
| solve/4x262144 | 10.674 ms | 10.746 ms | +0.67%(CI [−2.3%, +3.9%], p = 0.68, no change) |
| new/4x1048576 | 215.23 ms | 143.71 ms | −33.23%(CI [−33.9%, −32.5%], p = 0.00, improved) |
| solve/4x1048576 | 43.263 ms | 41.054 ms | −5.11%(CI [−6.6%, −3.5%], p = 0.00, improved) |

baseline `lstsq-rows` は行 API(`LeastSquares::new` で内部
strided 列再収集 + 行セット `to_vec` clone を含む)の計測。

factor 構築(`new`)は全サイズで −24〜−70% と大幅改善 —
strided gather と行セット clone が消え、入力列の連続
`to_vec` + 追記のみになった効果が支配的。`solve` は 1 列・
4 列では改善するが 2 列では +7〜+16% の回帰を計測 — 旧来の
per-row 2 要素 `dot_kernel` 呼出しより、列 `sub_scaled` パス
(n 回の resid 走査 + sum_sq)のほうが n=2 でメモリ
トラフィックが増えるためと考えられる。RSS 実装は ADR
確定事項(連続バッファ・ridge 項非含有)のままとし、
solve は係数数 ≤ 数 ch の実ワークロードでは factor 構築の
改善がトータルで上回る。caller 側の行マテリアライズ除去
(mixer の ~m×n 行 `Vec` 構築、reference.rs/stage.rs の
チャネルコピー)は本 bench 対象外で、さらなる削減分。

## Validation

- `mise exec -- cargo test -p al-math`: green(列契約・解精度・
  `mod legacy` oracle 対比を含む)
- `mise exec -- cargo test -p al-locate`: green(mixer
  characterization・`dsp.rs` 列 fixture・`subtract_fit` 系を
  含む)
- `mise exec -- cargo test --workspace`: green(全 suite、
  golden 非回帰)
- `mise exec -- cargo build --workspace`: green
- `mise exec -- cargo fmt --all -- --check`: green
- `mise exec -- cargo clippy --workspace --all-targets -- -D
  warnings`: green
- `mise exec -- cargo bench -p al-math --bench lstsq --
  --baseline lstsq-rows`: 完走、delta は上表のとおり
- `mise exec -- cargo tree -p al-analysis --depth 1`: 直接依存に
  al-math なし
- コード検査: `LeastSquares::new`・per-sample 行 `Vec` 構築・
  strided gather・`reference.rs` の `.to_vec()` チャネルコピー・
  `stage.rs` の per-channel `to_vec` の残存なし。
  `subtract_fit`/`prepare_residual` の公開シグネチャと
  `lib.rs` re-export 面・`stage.rs` `prepare_residual` 呼出し・
  `tests/locate.rs`・`residual.rs` テストは無変更

## Risks

- bench delta は同一マシン・同一 bench profile でのみ有効。
  `solve` 側は RSS が行 gather から列 `sub_scaled` パスに
  変わった分の差を含む
- `fit_mix` の AᵀA 累算順変更により `condition_number` の
  低ビットは baseline とずれ得る(述語対象外・sanity bound
  のみ検証済み)。`matrix`/`fit_quality` は characterization
  oracle と 1e-12 predicate 内で一致
- ロールバックは本コミットの revert(独立コミット境界)

## Follow-ups

- T4(A1-opt): `dsp::conv`(overlap-save)+ `fir_model` 置換
  (`docs/tasks/caller-opt-dsp-conv-overlap-save.md`、本タスクに
  depends-on)
- 最終測定タスク(caller-opt-measurement-ir-closeout): 本 IR の
  bench delta を before/after 全比較値として closeout へ転記
