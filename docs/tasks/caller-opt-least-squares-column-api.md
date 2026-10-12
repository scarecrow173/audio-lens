---
id: "TASK-034EJgyjQ0EutoP1M6RqoQ"
type: "task"
status: "todo"
title: "caller 側最適化 A4: LeastSquares 列指向 API 置換と caller 移行"
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
    - "docs/plans/caller-side-optimizations.md"
  implemented-by: []
  depends-on:
    - "docs/plans/caller-side-optimizations.md"
    - "docs/tasks/caller-opt-stft-fft-plan-hoist.md"
  blocks:
    - "docs/tasks/caller-opt-dsp-conv-overlap-save.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/caller-side-optimizations.md"
    - "docs/adr/caller-side-optimizations-structure.md"
    - "docs/adr/simd-reduction-lane-fma.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/caller-side-optimizations.md"
  references: []
  defers: []
  deferred-by: []
---

# caller 側最適化 A4: LeastSquares 列指向 API 置換と caller 移行

## Work

plan T3(A4 opt コミット)。`LeastSquares` に列指向エントリ
`from_columns` を追加して rows API を**置換**(両建てしない、
pre-release 方針)し、`fit_mix` の行マテリアライズ(~8.6M alloc
規模)と linalg 内部の strided 列再収集・行セット clone を除去
する。predicate ゲートは T1 の `fit_mix` characterization と
`mod legacy` oracle が担う。

**`crates/al-math/src/linalg.rs`**:

- `pub fn from_columns(columns: &[&[f64]], lambda: f64) ->
  Option<Self>` を追加。契約は `new` の列向き写し:
  `columns.first()?` で列が1本以上・列長(観測数 m)≥ 列数 n・
  全列等長・全要素有限・λ が非負有限、のいずれかを欠けば
  `None`(「fail, never guess」の Option 契約を維持)
- `LeastSquares::new(&[Vec<f64>])` は除去して `from_columns` に
  置き換える(行・列の二重表現を残さない)
- `least_squares` free fn は列シグネチャ
  `least_squares(columns: &[&[f64]], rhs: &[f64], lambda: f64)`
  に置き換えて存続(al-locate root re-export 公開面を維持)
- 内部表現を列正本へ: `obs_columns`(ridge 適用前の入力列、
  RSS 用に 1 コピーのみ)、`columns`(ridge augmentation 付き
  分解後の列)、`reflectors`。`Factor` は入力列を直接消費し
  strided gather を行わない。augmentation(diag = √λ の `extra`
  要素)は分解内部のみに存在
- `solve(rhs)`: `rhs.len() == m`(観測数)かつ全要素有限で
  なければ `None`。`Apply` は現行どおり `y = rhs` を m+extra に
  zero-pad し反射適用・後退代入で `x` を得る
- `Apply` の RSS: `resid = rhs.to_vec()` に対し
  `for (j,&xj) { sub_scaled_kernel(simd, &mut resid, xj,
  &obs_columns[j]) }` の後 `rss = sum_sq_kernel(&resid)`、
  `rms = (rss / m).sqrt()`。RSS は現行契約どおり ridge 項を
  **含めない**(QR 尾部から RSS を取る実装は禁止 — ADR 確定)
- `condition_number` は無変更(入力は正方行列のまま)
- `mod tests` を列 fixture へ全面書き換え。`mod legacy`(行
  ベース参照)は oracle として残し、テスト内で列→行転置して
  対比する

**`crates/al-locate/src/mixer.rs`**:

- `fit_mix` を `fit_mix(source_channels: &[&[f64]],
  capture_channels: &[&[f64]], ridge: f64) ->
  Option<MixerEstimate>` に変更
- 行マテリアライズ(`mixer.rs:55-57` の `rows` 構築)は除去
- AᵀA 集計を列上の `dsp::dot` に置き換え
  (`ata[i][j] = dsp::dot(source_channels[i],
  source_channels[j])`、実装簡潔さ優先で全要素計算)
- `LeastSquares::from_columns(source_channels, ridge)` を呼ぶ。
  解ループ・`dsp::rms(cap)`・`MixerEstimate` 組立ては不変
- T1 の characterization テストを新シグネチャへ追従(oracle は
  不変、呼出し側のみ `&[&s0[..], &s1[..]]` 形へ)

**caller 移行(design 移行マップどおり)**:

- `reference.rs:731-733`: `src_chs`/`cap_chs` の `to_vec` コピー
  を**除去**し `fit_mix(&src.src, &src.cap, cfg.ridge)` へ
- `reference.rs:882`: 同上(crosstalk)
- `reference.rs:1105`: `fit_mix(&src.src, &r1_refs, …)`。
  `r1_refs: Vec<&[f64]> = r1.iter().map(Vec::as_slice).collect()`
  (参照のみ、データコピーなし)
- `residual.rs`: `subtract_fit` の公開シグネチャ `&[Vec<f64>]`
  は**維持**。内部で `Vec<&[f64]>` 列参照へ変換 —
  `c.len() >= len` は `&c[..len]`(コピーなし)、`c.len() < len`
  のみ zero-pad した所有バッファに逃がす(design の最小化方針)。
  `capture_channels.to_vec()` は維持(residual 出力は所有が必要)
- `stage.rs:753-759`: `cap_window`/`src` を per-channel `to_vec`
  からスライス参照 `Vec<&[f64]>` へ
- `dsp.rs:114` テスト: `least_squares(&[c0.as_slice(),
  c1.as_slice()], &rhs, 0.0)` の列 fixture へ追従
- `benches/lstsq.rs`: 列シグネチャへ追従し、`--baseline
  lstsq-rows` で T1 baseline と比較して delta を IR に追記
- 無変更を維持: `subtract_fit`/`prepare_residual` の公開
  シグネチャ(`lib.rs:29-30` re-export 面・`stage.rs:947`・
  `tests/locate.rs:300`・`residual.rs` テスト `:269,306`)、
  `lib.rs` の `least_squares` re-export 行(名前は同じ)

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/caller-side-optimizations.md`(T3)
- `docs/designs/caller-side-optimizations.md`(列正本・
  `obs_columns`・RSS 設計・`fit_mix` シグネチャ・caller 移行
  マップ・`subtract_fit` 最小化)
- `docs/adr/caller-side-optimizations-structure.md`(`from_columns`
  API 形・rows API 置換・RSS 非 ridge 契約・fit_mix 移行確定)
- `docs/adr/simd-reduction-lane-fma.md`(predicate 適用の権威)

## Dependencies

- `docs/tasks/caller-opt-stft-fft-plan-hoist.md`(直列化は plan
  Implementation Sequence の決定)
- 実質的には T1 の `fit_mix` characterization・lstsq rows API
  baseline・`mod legacy` oracle に依存
- T4 は本タスクに依存する

## Affected Files

- `crates/al-math/src/linalg.rs`(`from_columns` 追加・`new` 除去・
  `least_squares` 列シグネチャ・`obs_columns`・RSS
  `sub_scaled`/`sum_sq` 化・テスト列 fixture 化)
- `crates/al-math/benches/lstsq.rs`(列シグネチャ追従)
- `crates/al-locate/src/mixer.rs`(`fit_mix` 列シグネチャ・
  行マテリアライズ除去・AᵀA 列 `dot` 化・テスト追従)
- `crates/al-locate/src/residual.rs`(`subtract_fit` 内部の列
  参照変換)
- `crates/al-locate/src/stage.rs`(`:753-759` スライス参照化)
- `crates/al-locate/src/dsp.rs`(`:114` テスト列 fixture 化)
- `crates/al-analysis/src/reference.rs`(`fit_mix` 呼出し×3 移行・
  `.to_vec()` 除去)
- `docs/impl/ir/<record>.md`(lstsq bench delta の追記)

## Done When

- [ ] `LeastSquares::from_columns(&[&[f64]], λ)` が公開 API と
  して存在し `new` 相当契約(等長・有限・観測数≥列数・非負
  有限 λ → `None`)を列向きに持ち、`new` と行 `least_squares`
  が除去されている
- [ ] `solve` が `rhs.len() == 観測数` を要求し違反で `None`
- [ ] `LeastSquares` が行セットを保持せず ridge 適用前の入力列を
  1 コピーのみ保持し、RSS が `sub_scaled`/`sum_sq` で ridge 項
  非含有のまま計算されている(コード検査)
- [ ] `mixer.rs` の per-sample 行 `Vec` 構築と `linalg.rs` の
  strided gather・行 clone がコード検査で除去されている。
  `fit_mix` 呼出し全箇所が列シグネチャを使う
- [ ] `reference.rs` の `.to_vec()` チャネルコピーが除去され
  `src.src`/`src.cap` の `Vec<&[f64]>` が直接渡されている
- [ ] `subtract_fit`/`prepare_residual` の公開シグネチャ
  `&[Vec<f64>]` が不変で、`lib.rs` re-export 面・
  `stage.rs:947`・`tests/locate.rs:300`・`residual.rs` テスト
  が無変更
- [ ] T1 の `fit_mix` characterization が新シグネチャ追従後も
  predicate 内で green(`condition_number` は sanity bound)
- [ ] linalg 既存解精度テストが列 fixture 書き換え後も pass し、
  `mod legacy` oracle 対比が存在する
- [ ] lstsq bench が列シグネチャで走り、baseline 比較 delta が
  IR に追記済み

## Verification

- [ ] `mise exec -- cargo test -p al-math` が green(列契約・
  解精度・`mod legacy` 対比を含む)
- [ ] `mise exec -- cargo test -p al-locate` が green(mixer
  characterization・`dsp.rs:114`・`subtract_fit` 系を含む)
- [ ] `mise exec -- cargo test -p al-math -p al-locate -p
  al-analysis` と `mise exec -- cargo test --workspace` が
  green(golden テストを含む)
- [ ] `mise exec -- cargo bench -p al-math --bench lstsq --
  --baseline lstsq-rows` が実行でき delta を IR に記録
- [ ] `mise exec -- cargo clippy --workspace --all-targets -- -D
  warnings`・`mise exec -- cargo fmt --all -- --check` が green
- [ ] `rg "LeastSquares::new|fit_mix\(&\[s0"` 等で行 API・行
  マテリアライズの残存がないことを検査
- [ ] `mise exec -- cargo tree -p al-analysis --depth 1` の直接
  依存に al-math が現れない

## Out of Scope

- `condition_number` の入力形式変更(正方行列のまま)や述語的
  等価検証(sanity bound のみ)
- `subtract_fit`/`prepare_residual` の公開シグネチャ変更
- conv・stft 等 A1/A3 の変更
