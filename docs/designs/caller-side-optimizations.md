---
id: "DESIGN-034EIsK2SiNLNZYXmumzOe"
type: "design"
status: "approved"
title: "caller 側最適化: FIR 畳み込み FFT 化・STFT plan 再利用・LeastSquares 列指向の実装設計"
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
    - "../specs/caller-side-optimizations.md"
  implemented-by: []
  depends-on:
    - "./al-math-vec-primitives.md"
    - "./observability-foundation.md"
    - "../adr/al-math-vec-primitives-slice-boundary.md"
    - "../adr/simd-reduction-lane-fma.md"
    - "../adr/al-math-api-fft-split-re-im.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./al-math-vec-primitives.md"
    - "./mechanical-caller-cleanup.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/caller-side-optimizations.md"
    - "../adr/caller-side-optimizations-structure.md"
    - "../discovery/caller-side-optimizations.md"
  derived-by:
    - "../plans/caller-side-optimizations.md"
    - "../test-specs/caller-side-optimizations.md"
    - "../tasks/caller-opt-bench-baseline-characterization.md"
    - "../tasks/caller-opt-stft-fft-plan-hoist.md"
    - "../tasks/caller-opt-least-squares-column-api.md"
    - "../tasks/caller-opt-dsp-conv-overlap-save.md"
    - "../tasks/caller-opt-measurement-ir-closeout.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# caller 側最適化: FIR 畳み込み FFT 化・STFT plan 再利用・LeastSquares 列指向の実装設計

## Context

spec `../specs/caller-side-optimizations.md`(approved)と ADR
`../adr/caller-side-optimizations-structure.md`(accepted)で確定した
3施策(A1 fir_model 畳み込み FFT 化・A3 STFT plan 再利用・A4
LeastSquares 列指向)を、実装可能な関数名・シグネチャ・ファイル
マップ・テスト/ベンチ計画に具体化する。棚卸し根拠は
`../discovery/caller-side-optimizations.md`。

契約の権威は上流にある。本設計内の再掲(数値 predicate・
`from_columns` 契約・plan 所有境界・RSS 非 ridge 契約)が
spec/ADR と乖離した場合は spec/ADR を正とし、本設計を追従修正する。

設計が解く残件(discovery の Open Questions 由来):

- overlap-save のブロック長・境界条件の具体値
- dsp 側 plan キャリアの具体形
- `conv`/`fir_model`/`fir_ir` の実名・シグネチャ
- `residual.rs` clone+resize の最小化方針
- characterization テスト・ベンチの具体的構成と baseline 取得手続き

## Scope

### In Scope

- **T-baseline(最適化より先に land する基盤)**:
  characterization テスト群・criterion ベンチ基盤
  (`crates/al-analysis` criterion `=0.8.2` dev-dep + `[[bench]]`
  stanza、`benches/` 新設)、`examples/bench_reference.rs` Instant
  ベンチ新設、baseline 計測手続きとコミット順序
- **A3**: `al_locate::dsp` への `Fft` plan 公開と `stft` の
  plan 巻き上げ(ビット一致)
- **A4**: `LeastSquares::from_columns` 追加と rows API 置換、
  `fit_mix` 列指向化と全 caller 移行、`.to_vec()` コピー除去
- **A1**: `dsp::conv`(overlap-save via `RealFft`)新設、
  `fir_model`/`fir_ir`/`subtract` の名前付き関数切り出し、
  `subtract` の zero-fill 修正、直接畳み込み参照実装の
  test/bench 保持
- テスト計画・ファイルマップ・リスク登録・ロールバック方針

### Out of Scope

spec の Out of Scope を全て継承する: stft フレームバッファの
巻き上げ/再利用、A2 ncc plan 再利用・A5 `local_lag`→`ncc`
(issue #42 系別チェーン)、al-math カーネル内部
(`Fft`/`RealFft`/vec プリミティブ/QR 数学)の変更、
マルチスレッド化、al-analysis への al-math 直接依存、golden
意味・Validity 閾値の変更、互換層/deprecation 期間。

## Components and Boundaries

### T-baseline: characterization テストとベンチ基盤

全ての最適化適用に先立ち、現行出力を捕捉するテストと計測基盤を
先に land させる(spec プロセス契約)。コミット順序は下記
「コミット順序と baseline 取得手続き」を参照。

#### characterization テスト配置

| 対象 | 配置 | 捕捉内容 |
| --- | --- | --- |
| `fir_model`(A1) | `crates/al-analysis/src/reference.rs` `mod tests` | 名前付き関数へ切り出した `fir_model` の出力を、同じく切り出した `fir_ir` の IR と test 内 `conv_direct`(直接畳み込み参照実装)の合成で照合。fixture: 空・`n < conv_len`・`conv_len = 4096` 境界・全ゼロ・決定的ノイズ(LCG)。比較は要素毎 predicate `|a-b| <= 1e-12 * max(1.0,|a|,|b|)` |
| `subtract` zero-fill(A1 付随) | 同上 | `subtract` を名前付き関数へ切り出し、ragged な channel 長(model が residual より短い)fixture で panic せず残りが zero-fill されることを検証 |
| `stft`(A3) | `crates/al-analysis/src/stft.rs` `mod tests` | 決定的 fixture(sin 混合 + LCG ノイズ)の `StftMatrix` を、各フレームを test 内で独立に window + `dsp::fft`(single-shot 経路)して得た参照と `f64::to_bits` 比較でビット一致照合。fixture: 通常複数フレーム・`samples.len() < window`(0フレーム)・単一フレーム・非2冪 window・全ゼロ |
| `fit_mix`(A4) | `crates/al-locate/src/mixer.rs` `mod tests` | `MixerEstimate.matrix`・`fit_quality` を test 内の行ベース normal-equations 参照実装(自己完結 oracle、`linalg.rs` `mod legacy` と同型)で predicate 内照合。`condition_number` は述語対象外とし、有限/非有限(INFINITY 整合)と位数一致の粗い sanity bound のみ検証 |
| `linalg` 列置換(A4) | `crates/al-math/src/linalg.rs` `mod tests` | `from_columns` が `new` 相当契約(列等長・有限・観測数≥列数・非負有限 λ → `None`)を列向きに持つこと、`solve` が `rhs.len()==観測数` を要求すること、既存解精度テストが列 fixture に書き換え後も pass すること。`mod legacy`(行ベース参照)は oracle として残し、テスト内で列→行転置して対比する |

#### criterion ベンチ基盤

`crates/al-analysis/Cargo.toml` に al-math `Cargo.toml` と同じ
pattern で追加(新しい依存カテゴリではないが、依存追加として
Implementation Record に記録する):

```toml
[dev-dependencies]
criterion = "=0.8.2"

[[bench]]
name = "stft"
harness = false

[[bench]]
name = "conv"
harness = false
```

- `crates/al-analysis/benches/stft.rs`: `stft()` を決定的入力で
  計測。サイズは window/hop を現行設定(4096/2048)に固定し、
  信号長を ~0.5M・~1M samples で走査。group `stft`、
  `black_box` で最適化除去を防止
- `crates/al-analysis/benches/conv.rs`: T0 時点では
  `conv_direct`(直接畳み込み参照実装、bench ファイル内に
  自己完結で保持)の baseline 腕のみを置く — `dsp::conv` は
  A1-opt で初めて存在するため T0 に含めるとベンチが
  コンパイルできない。直接畳み込みは A1 変更前の本番経路
  そのものであり、conv の baseline はこれを計測して
  `target/criterion/` に記録する。A1-opt コミットで
  `dsp::conv` 腕を追加し同一 fixture で計測する。FFT 経路は
  n ∈ {65_536, 262_144, 1_048_576} × kernel ∈ {256, 4096}。
  legacy direct conv は O(n·m) のため有界化: n ≤ 262_144 かつ
  criterion `sample_size(10)` + 短い `measurement_time` を設定
  (n=262_144・m=4096 で ~1G MACs/iter 規模)。同サイズ点で
  直接 A/B、1M 点は FFT 経路の scaling 観測用
- `crates/al-math/benches/lstsq.rs` 新設 + `[[bench]]` stanza
  (criterion dev-dep は導入済み、新規依存なし): `LeastSquares`
  構築 + `solve` を列数 {1, 2, 4} × 観測数 {65_536, 262_144,
  1_048_576} で計測。入力列はベンチ側で1回生成し `b.iter` の
  計測対象外とする。`vec_primitives.rs` とは別 target とする
  (責務分離: ベクトルプリミティブと lstsq は別関心)

#### `examples/bench_reference.rs`

`crates/al-analysis/examples/bench_reference.rs` を新設し、
al-locate `bench_*` examples の Instant ベンチ規約に倣う。
既存 examples は `stft`/`fir_model` 経路/`fit_mix` をカバー
しないため新設する。計測対象:

- `stft`: 現行設定(4096/2048)の決定的入力
- `analyze_pair`: 有界な synthetic fixture(数秒規模の LCG
  ノイズ + 構成済み mix)で `fir_model` を含む R0–R3 全経路
- `prepare_residual`(al-locate pub、シグネチャ不変): A4 の
  caller 級計測点として `fit_mix` を内部経由で計測 — 公開
  シグネチャ `&[Vec<f64>]` が不変のため、A4 前後で同一
  harness で比較可能

`fit_mix` 直接計測は採用しない: A4 でシグネチャが変わるため
before/after で harness 自体が変わり、等価な比較軸にならない。

#### コミット順序と baseline 取得手続き

各最適化は独立に適用可能な単位(spec プロセス契約)。推奨順序と
各単位内のコミット境界:

1. **T0 — ベンチ基盤コミット**: `Cargo.toml` dev-dep + stanza、
   `benches/stft.rs`・`benches/conv.rs`(`conv_direct` baseline
   腕のみ — `dsp::conv` は A1-opt まで存在しない)・
   `benches/lstsq.rs`、`examples/bench_reference.rs` を land。
   baseline を実行し `target/criterion/`(criterion 出力)と
   `target/bench-baseline/`(Instant 出力をファイルリダイレクト
   で保存)に記録、実測値を Implementation Record(ドラフト)へ
2. **A1-char — characterization コミット**: `fir_model`/
   `fir_ir`/`subtract` の名前付き関数切り出し(挙動不変の純粋
   リファクタ)+ `subtract` zero-fill 修正 + characterization
   テスト + `conv_direct` 参照実装。zero-fill 修正はこの
   コミットに同梱し A1 適用より先に land(spec 確定事項)
3. **A1-opt — 最適化コミット**: `dsp::conv` 追加(dsp 側
   equivalence テスト付き)+ `fir_model` の畳み込み置換 +
   `benches/conv.rs` への `dsp::conv` 腕追加(T0 で記録した
   `conv_direct` baseline と同一 harness で A/B)
4. **A3-char**: stft ビット一致 characterization テスト
5. **A3-opt**: dsp `Fft` 公開 + `stft` plan 巻き上げ
6. **A4-char**: mixer/linalg characterization テスト + lstsq
   ベンチ baseline 計測(rows API 上で)
7. **A4-opt**: `from_columns` 追加 + rows API 置換 + caller
   移行 + bench ファイルの列シグネチャ追従

各 opt コミット後に同一条件で bench を再実行し、before/after
実測を Implementation Record(`docs/impl/ir/`)に記録する。
criterion の baseline 保存は `--save-baseline <name>` /
`--baseline <name>` で管理する(出力は `target/criterion/`)。

### A3 — STFT の plan 巻き上げ

**plan キャリア**: `al_math::fft::Fft` は既に `pub` の plan 型
(`new(n)` + `transform(&self, re, im, inverse)`)。dsp 側は薄い
キャリアを新設せず、re-export で公開する — `pub use` による
dsp 集約は既存の `pub use al_math::correlation::{...}` と同じ
pattern で、`Fft` は変換・スレッド間で再利用可能な不変係数の
型としてそのまま露出してよい(ADR「re-export または等価の薄い
キャリア」の re-export 選択)。

```rust
// crates/al-locate/src/dsp.rs
pub use al_math::fft::Fft;
```

`crates/al-locate/src/lib.rs` の mirror 規約に倣い、既存
`pub use dsp::{...}` 行に `Fft` を追加する(`conv` も同様 —
「dsp re-export と crate root mirror」参照)。

**stft の変更**(`crates/al-analysis/src/stft.rs`):

```rust
use al_locate::dsp::{Fft, mag_sq, scale};

pub fn stft(samples: &[f64], window: usize, hop: usize, sample_rate: u32) -> StftMatrix {
    let fft_size = window.next_power_of_two();
    let win = hann(window);
    let mut re = Vec::new();
    let mut im = Vec::new();
    if samples.len() >= window && window > 0 && hop > 0 {
        let plan = Fft::new(fft_size);
        for start in (0..=samples.len() - window).step_by(hop) {
            let mut fre = vec![0.0; fft_size];
            let mut fim = vec![0.0; fft_size];
            for (i, &v) in samples[start..start + window].iter().enumerate() {
                fre[i] = v * win[i];
            }
            plan.transform(&mut fre, &mut fim, false);
            re.push(fre);
            im.push(fim);
        }
    }
    StftMatrix { /* 不変 */ }
}
```

- `Fft::new` はガード内に置く: `window == 0` のとき
  `next_power_of_two() == 1` で `Fft::new(1)` は構築自体は
  可能だが、現行と同じく「フレームが走る場合のみ plan を
  構築する」意味に揃える
- フレームバッファ `vec![0.0; fft_size]` ×2 は `re.push(fre)` で
  出力行に move されるため現行のまま残る(巻き上げ対象外)
- `win` は既にループ外で1回構築済み — windowing 戦略は変更不要
- `stft` シグネチャ不変 → 全 caller(`reference.rs` ×~6、
  `cheap.rs:609,940`、`expensive/mod.rs:632`・`codec.rs:235`・
  `channel.rs:83,89,370,371`・`tests.rs:289`、
  `band_gain.rs:151,152`、`stage.rs:1130`(test)、`stft.rs`
  内部テスト)無修正 — signature 不変のため影響なし
- `dsp::fft` の公開シグネチャ不変 → single-shot caller
  (`codec.rs:97`、`dynamics.rs:191`、`ir.rs:177`、`noise.rs:140`、
  `reference.rs:1075` の irfft、`expensive/tests.rs`)無修正
- **ビット一致の根拠**: `Fft` の twiddle は `Fft::new` で決定的に
  生成され、`transform` はバッファを全上書きする。同一カーネル・
  同一入力の再利用で出力はビット一致。`transform` 内部の
  dispatch も呼出し毎に同じ ISA を選択する
- **不変条件(コード検査)**: stft ループ内に `Fft::new` がない
  こと、`dsp::fft` シグネチャが不変であること

### A4 — 列指向 `LeastSquares` と `fit_mix` 移行

#### `al_math::linalg` の変更

```rust
pub struct LeastSquares {
    obs_columns: Vec<Vec<f64>>, // ridge 適用前の入力列(1 コピーのみ、RSS 用)
    columns: Vec<Vec<f64>>,     // ridge augmentation 付き分解後の列(R 情報)
    reflectors: Vec<Vec<f64>>,
}

impl LeastSquares {
    /// 有限・等長の列、非負有限の ridge、観測数(列長) >= 列数を要求する。
    pub fn from_columns(columns: &[&[f64]], lambda: f64) -> Option<Self>;
    /// rhs.len() == 観測数(ridge augmentation 後の列長ではない)を要求。
    pub fn solve(&self, rhs: &[f64]) -> Option<(Vec<f64>, f64)>;
}

/// 単一 rhs の ridge 最小二乗求解。
pub fn least_squares(
    columns: &[&[f64]],
    rhs: &[f64],
    lambda: f64,
) -> Option<(Vec<f64>, f64)>;
```

- `from_columns` の契約は `new` の列向き写し:
  `columns.first()?` で列が1本以上・列長(観測数 m)≥ 列数 n・
  全列が等長・全要素有限・λ が非負有限、のいずれかを欠けば
  `None`。「fail, never guess」の Option 契約を維持
- `LeastSquares::new(&[Vec<f64>])` は**除去**し `from_columns`
  に置き換える(pre-release 方針、行・列の二重表現を残さない)
- `Factor`(`WithSimd`): 入力列を直接消費し strided gather を
  行わない。分解用列は `columns[j].to_vec()` に ridge 要素
  (diag = √λ を `extra` 要素として末尾に追加)を付けた augmented
  列(長さ m+extra)とする — augmentation は分解内部のみに存在
- `LeastSquares` は ridge 適用前の入力列を1コピーだけ所有する
  (`obs_columns`、現行 `rows.to_vec()` の置き換え)
- `solve`: `rhs.len() == m`(観測数)かつ全要素有限でなければ
  `None`。`Apply` は現行どおり `y = rhs` を m+extra に
  zero-pad し反射適用・後退代入で `x` を得る
- `Apply` の RSS: `resid = rhs.to_vec()` に対し
  `for (j,&xj) { sub_scaled_kernel(simd, &mut resid, xj, &obs_columns[j]) }`
  の後 `rss = sum_sq_kernel(&resid)`、`rms = (rss / m).sqrt()`。
  連続バッファ上の `sub_scaled`/`sum_sq`(`residual.rs` の
  per-source 先例と同形)。RSS は現行契約どおり ridge 項を
  **含めない** — QR 尾部から RSS を取る実装は `residual_rms`/
  `fit_quality` の意味を変えるため禁止(ADR 確定)
- `condition_number` は無変更(入力は正方行列のまま、mixer が
  列 `dot` 集計した AᵀA を渡す)

#### `fit_mix` のシグネチャと内部

```rust
pub fn fit_mix(
    source_channels: &[&[f64]],
    capture_channels: &[&[f64]],
    ridge: f64,
) -> Option<MixerEstimate>;
```

- 行マテリアライズ(`mixer.rs:55-57` の `rows` 構築)は除去
- AᵀA 集計は列上の `dsp::dot` に置き換える:
  `ata[i][j] = dsp::dot(source_channels[i], source_channels[j])`
  (対称行列のため i≤j のみ計算して写すか、全要素計算 —
  実装簡潔さ優先で全要素、n_src ≤ 数ch で誤差)
  — 累算順が変わるため predicate 適用対象
- `LeastSquares::from_columns(source_channels, ridge)` を呼ぶ
- 解ループ・`dsp::rms(cap)`・`MixerEstimate` 組立ては不変

#### caller 移行マップ

| 箇所 | 現行 | 移行 |
| --- | --- | --- |
| `reference.rs:731-732` | `src_chs`/`cap_chs` = `to_vec` コピー | **除去**。`src.src`/`src.cap`(`Vec<&[f64]>`)をそのまま渡す |
| `reference.rs:733` | `fit_mix(&src_chs, &cap_chs, …)` | `fit_mix(&src.src, &src.cap, cfg.ridge)` |
| `reference.rs:882` | 同上(crosstalk) | 同上 |
| `reference.rs:1105` | `fit_mix(&src_chs, &r1, …)` | `fit_mix(&src.src, &r1_refs, …)`。`r1_refs: Vec<&[f64]> = r1.iter().map(Vec::as_slice).collect()`(参照のみ、データコピーなし) |
| `residual.rs:153-161` | 全 src を clone+resize して `&[Vec<f64>]` で fit_mix | `subtract_fit` の公開シグネチャ `&[Vec<f64>]` は**維持**。内部で `Vec<&[f64]>` 列参照へ変換: `c.len() >= len` は `&c[..len]`(コピーなし)、`c.len() < len` のみ zero-pad した所有バッファに逃がす(後述) |
| `residual.rs:162` | `capture_channels.to_vec()` | **維持**(residual 出力バッファは所有が必要、API 形状コピーではない) |
| `stage.rs:753-759` | `cap_window` per-channel `to_vec` + fit_mix | `cap_window: Vec<&[f64]> = capture.channel_samples.iter().map(|c| &c[start..start+len]).collect()` と `src_cols: Vec<&[f64]> = source.channel_samples.iter().map(Vec::as_slice).collect()` でスライス参照へ |
| `dsp.rs:114`(test) | `least_squares(&rows, &rhs, 0.0)` | 列 fixture に書き換え(`least_squares(&[c0.as_slice(), c1.as_slice()], &rhs, 0.0)`。`&[f64]` 要素に `&c0` の Vec→slice 強制を頼らず `as_slice()`/`&s0[..]` 形で明示) |
| `linalg.rs` tests | `LeastSquares::new(&rows, λ)` 系 | 列 fixture へ全面書き換え。`mod legacy` は oracle として残し列→行転置で対比 |
| `mixer.rs` tests | `fit_mix(&[s0, s1], &[cap0, cap1], …)` | `&[&[f64]]` シグネチャへ追従(`&[&s0[..], &s1[..]]` 形) |

`subtract_fit` の最小化方針(ADR「評価の上で最小化」の解釈):

```rust
let len = capture_channels.first()?.len();
let padded: Vec<Vec<f64>> = source_channels
    .iter()
    .map(|c| {
        let mut v = Vec::new();
        if c.len() < len {
            v = c.clone();
            v.resize(len, 0.0);
        }
        v
    })
    .collect();
let src_cols: Vec<&[f64]> = source_channels
    .iter()
    .zip(&padded)
    .map(|(c, p)| if c.len() < len { p.as_slice() } else { &c[..len] })
    .collect();
let cap_cols: Vec<&[f64]> = capture_channels.iter().map(Vec::as_slice).collect();
let est = fit_mix(&src_cols, &cap_cols, ridge)?;
```

等長・長い入力はコピーを完全に除去し、短い入力のみ所有
バッファに逃がす(現行 `resize` と同じ長さ合わせ意味)。
`sub_scaled` 残差ループは `src_cols` を使う(等長契約に適合)。
`subtract_fit`/`prepare_residual` の公開シグネチャ
`&[Vec<f64>]` は不変 — `lib.rs` re-export 面と
`stage.rs:947`・`tests/locate.rs:300`・`residual.rs` テスト
`:269,:306` は無変更。

### A1 — `fir_model` の FFT 畳み込み化

#### 関数切り出し(characterization コミット、挙動不変)

`analyze_pair` 内の `fir_model` クロージャ(`reference.rs:1052`)
と `subtract` クロージャ(`reference.rs:1041`)を module 内の
名前付き関数へ切り出す(private + 同一ファイルの `#[cfg(test)]`
から到達。`reference.rs` には既に `mod tests` があり
`aligned()` ヘルパもある):

```rust
/// Welch 推定の impulse response(from_stft → to の CSD 平均を逆変換)。
/// 返り値は from_stft.fft_size 長の実数列(未 truncate)。
/// fr == 0(フレームなし)なら同長のゼロ埋め IR を返す。
fn fir_ir(
    from_stft: &StftMatrix,
    to: &[f64],
    window: usize,
    hop: usize,
    rate: u32,
) -> Vec<f64>;

/// from の信号に impulse response を畳み込み from 長のモデル列を返す。
/// conv_len = ir.len().min(4096) のキャップと「出力 = 線形畳み込みの
/// 先頭 n 要素」の現行意味を維持する。
fn fir_model(
    from: &[f64],
    to: &[f64],
    from_stft: &StftMatrix,
    cfg: &ReferenceConfig,
    rate: u32,
) -> Vec<f64>;

/// residual - model のチャネル毎要素減算。model 側が短い要素は
/// zero-fill(`m.get(i).copied().unwrap_or(0.0)`)。
fn subtract(residual: &[Vec<f64>], model: &[Vec<f64>]) -> Vec<Vec<f64>>;
```

- `fir_ir` は CSD 累積・`hre`/`him` 構築・`fft(&mut hre, &mut
  him, true)` の irfft までを担い、畳み込みは含まない。
  `conv_len` キャップは `fir_model` 側で `&ir[..ir.len().min(4096)]`
  として適用する — 畳み込み経路の切替点を `dsp::conv` の呼出し
  1箇所に集約する
- `fir_model` 内の STFT は `stft(to, cfg.stft_window,
  cfg.stft_hop, rate)`(`fir_ir` 内部)と `from_stft`(
  `analyze_pair` が既に `ms` として計算済みのものを渡す)
- `fr == 0` の早期 return は `fir_ir` 内に固定する: `mt` は
  `fir_ir` 内部で `stft(to, …)` として計算するため `fir_model`
  側で `mt.frames()` を判定できない。`fir_ir` は `fr == 0` の
  とき長さ `from_stft.fft_size` のゼロ埋め IR をそのまま返し、
  `fir_model` は無条件に `dsp::conv` へ進む(ゼロ IR の畳み込み
  結果は `vec![0.0; n]` で、現行の早期 return と等価)

#### `dsp::conv` の設計

```rust
// crates/al-locate/src/dsp.rs
/// 線形畳み込みの先頭 `signal.len()` 要素を返す(overlap-save、
/// al_math::fft::RealFft)。kernel が空なら signal 長のゼロ列、
/// signal が空なら空ベクトル — 両早期 return はブロック長の
/// 計算より先に置く(`m == 0` では `m - 1` がアンダーフロー)。
pub fn conv(signal: &[f64], kernel: &[f64]) -> Vec<f64>;
```

- **配置**: `al_locate::dsp`(ADR 確定)。`deinterleave`/`fft` と
  同じ dsp 手書き関数の系として、内部で `al_math::fft::RealFft`
  を呼ぶ。al-analysis は `dsp::conv` 経由で到達し al-math 直接
  依存は発生しない。`lib.rs` の dsp mirror にも `conv` を追加
- **方式**: overlap-save。まず空入力の早期 return を行う:
  `kernel.is_empty()` → `vec![0.0; signal.len()]`、
  `signal.is_empty()` → `Vec::new()`(docstring 記載どおり、
  B/L 計算より必ず先 — `m = 0` では `m - 1` が usize で
  アンダーフローする)。次に `m = kernel.len()`、ブロック長
  `B = (2 * m).next_power_of_two().max(1024)`(2のべき乗・
  `RealFft::new` 契約)。segment 長 `L = B - m + 1`
  - `m` ≤ 4096 の実用域で B ≤ 8192、バッファは `B` と `B/2+1`
    規模の数本に有界(全長 spectral path の ~8M 要素変換を回避)
  - `.max(1024)` は小 kernel での per-block 呼出し・dispatch
    オーバーヘッドを潰す下限(任意値でよい。1K で L≈1023)
- **plan 再利用**: `RealFft::new(B)` を1本構築し、H の forward・
  各ブロックの forward/inverse で使い回す
  (`bandpass_many` が1 plan を forward/inverse で使い回す先例)
- **前計算**: `h` を `B` に zero-pad → `plan.forward` で
  `H = (hre, him)`(長さ `B/2+1`)を1回だけ取得
- **ブロックループ**(疑似コード):

  ```text
  if kernel.is_empty(): return vec![0.0; signal.len()]   // m-1 アンダーフロー回避
  if signal.is_empty(): return vec![]
  m = kernel.len(); B = (2*m).next_power_of_two().max(1024); L = B - m + 1
  xbuf = [0; B]                      // 初回の overlap 部はゼロ
  for off in (0..n).step_by(L):
      take = min(L, n - off)
      xbuf.copy_within(B-(m-1).., 0) // 前ブロック末尾 m-1 を先頭へ繰越
      xbuf[m-1 .. m-1+take] = signal[off..off+take]
      xbuf[m-1+take .. B] = 0        // 末尾 zero-pad
      forward(xbuf) -> X; Z = X·H(複素乗算); inverse(Z) -> obuf
      y[off..off+take] = obuf[m-1 .. m-1+take]  // 先頭 m-1 を discard
  ```

  - **M-1 discard**: 先頭 m-1 出力は circular wrap-around の
    影響区間のため破棄し、残り `take` 要素だけを `y` へ写す
  - **zero-pad 戦略**: 入力側は「前ブロック末尾 m-1 + 新規 take」
    のみ書き残りはゼロ。末尾ブロックの短い `take` も同じ処理で
    よい(パディング分は discard 相当)
  - **正規化**: `RealFft::inverse` が 1/N を含むため乗算後の
    逆変換で正しい線形畳み込み値が得られる(追加の scaling
    コードは不要)
  - **スペクトル複素乗算**: `zre[k] = xre·hre − xim·him`、
    `zim[k] = xre·him + xim·hre` を `B/2+1` bins 上でループ。
    vec プリミティブに複素乗算はないため dsp 内で素直な
    スカラループとする(ブロックあたり B/2+1 要素、FFT 自体が
    支配的でプリミティブ化の対象外)
- **`conv_len ≤ 4096` キャップ**: `fir_model` 側が IR を
  `&ir[..ir.len().min(4096)]` に切ってから `dsp::conv` に渡す。
  `conv` 自体にキャップは持たせない(純粋な畳み込み関数のまま)
- **先頭 n 要素(尾部打ち切り)**: `conv` の出力長 = `signal
  .len()` でそのまま現行意味を満たす
- **zero-skip は削除**: 現行ループの `if x == 0.0 { continue }`
  は直接畳み込みの走査短縮であり、FFT 経路には skip の概念が
  ない。`x == 0.0` の項は直接畳み込みでも `y[i+k] += 0.0` と
  同じ(数値寄与ゼロ)のため等価 — ±0.0 のビット差のみ残り得る
  が、比較は値 predicate であり問題にならない。sparse 入力への
  最適化意味は FFT 化で不要になるため持ち越さない
- **`subtract` zero-fill**: `m[i.min(m.len())]` →
  `m.get(i).copied().unwrap_or(0.0)`。Aligned shared-length 契約
  (`stage.rs:283-295`)の下で本番到達不能の構造的防御であり、
  characterization コミットに同梱して A1 適用より先に land
- **直接畳み込み参照実装 `conv_direct`**: test/bench コード内に
  自己完結で保持。`reference.rs` tests・`dsp.rs` tests・
  `benches/conv.rs` の3箇所に短い同一実装を置く(共有公開面に
  出さない — crate を跨ぐ test ユーティリティのためだけに公開
  API を増やさない)。3箇所は共有ヘルパ化せず逐語同一
  (verbatim)のコピーとする。現行 `fir_model` ループの
  `if x == 0.0 { continue }` スキップは参照実装では意図的に
  落とす: 有限データでは `xv == 0.0` の項の積は ±0.0 で
  `y[i+k] += ±0.0` が `y` をビット不変に保つため skip 有無は
  ビット等価であり、skip は走査短縮だけの最適化で参照実装の
  意味に影響しない(参照実装は簡潔さ優先):

  ```rust
  fn conv_direct(x: &[f64], h: &[f64]) -> Vec<f64> {
      let mut y = vec![0.0; x.len()];
      for (i, &xv) in x.iter().enumerate() {
          let maxk = h.len().min(x.len() - i);
          for (k, &hv) in h.iter().take(maxk).enumerate() {
              y[i + k] += xv * hv;
          }
      }
      y
  }
  ```

### ファイルマップ

| ファイル | 変更 |
| --- | --- |
| `crates/al-math/src/linalg.rs` | `from_columns` 追加・`new` 除去・`least_squares` 列シグネチャ・`obs_columns` 保持・RSS を `sub_scaled`/`sum_sq` 化・テスト列 fixture 化 |
| `crates/al-math/benches/lstsq.rs` | 新設(lstsq criterion ベンチ) |
| `crates/al-math/Cargo.toml` | `[[bench]] name = "lstsq"` 追加 |
| `crates/al-locate/src/dsp.rs` | `pub use al_math::fft::Fft`・`conv` 追加・`:114` テスト列 fixture 化・`conv_direct`(test)追加・conv equivalence テスト |
| `crates/al-locate/src/lib.rs` | dsp mirror 行に `Fft`・`conv` を追加 |
| `crates/al-locate/src/mixer.rs` | `fit_mix` 列シグネチャ・行マテリアライズ除去・AᵀA を列 `dot` 化・characterization テスト |
| `crates/al-locate/src/residual.rs` | `subtract_fit` 内部の列参照変換(公開シグネチャ不変) |
| `crates/al-locate/src/stage.rs` | `:753-759` cap_window/src のスライス参照化 |
| `crates/al-analysis/src/reference.rs` | `fir_model`/`fir_ir`/`subtract` 切り出し・zero-fill・`.to_vec()` 除去・`fit_mix` 呼出し×3 移行・characterization テスト・`conv_direct`(test) |
| `crates/al-analysis/src/stft.rs` | `Fft` import・plan 巻き上げ・ビット一致 characterization テスト |
| `crates/al-analysis/Cargo.toml` | criterion `=0.8.2` dev-dep + `[[bench]]` stanza ×2 |
| `crates/al-analysis/benches/stft.rs` | 新設 |
| `crates/al-analysis/benches/conv.rs` | 新設(T0 は `conv_direct` baseline 腕のみ、A1-opt で `dsp::conv` 腕追加) |
| `crates/al-analysis/examples/bench_reference.rs` | 新設(stft・analyze_pair・prepare_residual) |
| `docs/impl/ir/<record>.md` | before/after 実測の記録(Implementation Record) |

変更しない: `codec.rs`・`dynamics.rs`・`ir.rs`・`noise.rs`・
`expensive/tests.rs`(`dsp::fft` single-shot)、`cheap.rs:940`
(stft caller、シグネチャ不変)、`lib.rs` re-export 面の
`least_squares` 行(名前は同じ)、`tests/locate.rs:300`・
`residual.rs` テスト `:269,:306`(`&[Vec<f64>]` 維持)。

### テスト計画(対象ごとのゲート)

1. **A1 — conv 等価**: `dsp.rs` tests で `conv(x,h)` と
   `conv_direct` を predicate 比較。fixture: 空・`n < m`・
   `m` ∈ {1, 3, 256, 4096}・`n` ∈ {1, 小, 非ブロック倍数, 1M級}
   ・全ゼロ・決定的ノイズ(LCG)
2. **A1 — fir_model characterization**: `fir_ir` で得た IR の
   `conv_len` 切詰めを `conv_direct` に通した参照と
   `fir_model` 出力を predicate 比較(切り出し直後・置換後の
   両時点で同一テストが green)。エッジ: 空・`n < conv_len`・
   `conv_len = 4096` 境界・全ゼロ・代表ノイズ
3. **A1 — subtract zero-fill**: ragged チャネル fixture
   (`aligned()` ヘルパに長さの異なる channel を渡す、または
   `subtract` 直接呼出し)で panic せず短い model 側が
   zero-fill されること。本番到達不能の構造的防御の検証
4. **A3 — stft ビット一致**: 決定的 fixture で `StftMatrix.re`/
   `im` 全要素と `frames`/`fft_size`/`window`/`hop` メタが、
   test 内参照(window + `dsp::fft` をフレーム毎に適用)と
   `to_bits` 比較で一致。エッジ: `samples.len() < window`・
   単一フレーム・非2冪 window・全ゼロ。コード検査で
   「ループ内に `Fft::new` がない」を確認
5. **A4 — linalg 契約**: `from_columns` が列向き `new` 契約
   (等長・有限・観測数≥列数・非負有限 λ → `None`)を持つ。
   既存解精度テストを列 fixture に書き換えて継続 pass。
   `least_squares` 列シグネチャの `dsp.rs:114` 追従テスト
6. **A4 — fit_mix characterization**: `MixerEstimate.matrix`・
   `fit_quality` が test 内行ベース oracle と predicate 内一致。
   `condition_number` は INFINITY 整合 + 位数一致の sanity のみ
7. **全体**: `mise exec -- cargo test -p al-math -p al-locate
   -p al-analysis`・`cargo test --workspace`・`cargo clippy
   --workspace --all-targets -- -D warnings`・
   `cargo fmt --all -- --check` が green、golden テスト
   (`al-analysis/tests/golden.rs`・`al-locate/tests/golden.rs`)
   pass、`cargo tree -p al-analysis --depth 1` で al-math
   直接依存がない
8. **bench delta**: criterion delta と `bench_reference` 実測が
   ノイズ幅(目安 ±5%)を超える回帰を示さない — before/after
   を Implementation Record に記録

## Data and Control Flow

```text
[A1] analyze_pair
  → fir_model(from, to, ms, cfg, rate)
      → fir_ir(ms, to, cfg.stft_window, cfg.stft_hop, rate)
          → stft(to, …) → CSD 累積 → fft(inverse) → hre
      → dsp::conv(from, &ir[..min(4096)])
          → RealFft::new(B) 1本 → H 前計算 → overlap-save ブロックループ
          → 先頭 n 要素の線形畳み込み(= 現行 direct conv と同じ意味)

[A3] stft(samples, window, hop, rate)
  → Fft::new(fft_size) 1回(ガード内・ループ外)
  → per-frame: vec![0.0; fft_size] ×2 に window 適用 → plan.transform
  → re.push(fre)/im.push(fim)(バッファは出力行に move)

[A4] fit_mix(&[&[f64]] sources, &[&[f64]] captures, ridge)
  → AᵀA = 列 dsp::dot 集計 → condition_number(&ata)
  → LeastSquares::from_columns(sources, ridge)
      → augmented 列で Householder QR(従来同じカーネル)
      → obs_columns 1 コピー保持
  → per capture: solve(rhs)
      → reflector 適用 → 後退代入 → x
      → RSS = rhs − Σ x_j·obs_column_j を sub_scaled/sum_sq
```

## Non-Functional Constraints

- **数値契約**: A3 は `StftMatrix` 全出力ビット一致。A1/A4 は
  要素毎 `|a - b| <= 1e-12 * max(1.0, |a|, |b|)`(ADR-034EoHxzsvsHp9BOdCK2Cn
  の recomposition tolerance 適用)。永続化値
  (`ResidualEnergy`・`MixerEstimate`・feature record)は
  tolerance 内で変動し得るが golden テスト pass 必須
- **依存**: `crates/al-analysis` に al-math 直接依存を追加
  しない(dsp ファサード一元化)。criterion `=0.8.2` dev-dep は
  al-math と同 version・pattern
- **カーネル不変**: `Fft`/`RealFft`/vec プリミティブ/QR 数学は
  変更しない。変更面は caller + `linalg` API 置換のみ
- **公開シグネチャ**: `dsp::fft`・`stft`・`subtract_fit`/
  `prepare_residual` は不変。`fit_mix`・`least_squares` は
  列シグネチャへ置換(互換層なし、pre-release 方針)
- **メモリ**: overlap-save はブロックバッファ数本(B ≤ 8192 で
  数十KB)に有界。`LeastSquares` は augment 前列を1コピーのみ
- **panic 契約**: `conv` の内部 `RealFft::new(B)` は B を2の
  べき乗に構成するため不変条件で保証(公開関数の入力からは
  panic しない — 空入力は早期 return)。`from_columns`/
  `solve`/`fit_mix` は `Option` 契約で `None` を返す

## Risks and Trade-offs

| リスク | 緩和・ロールバック |
| --- | --- |
| overlap-save の境界条件バグ(edge ブロック・`n < m`・非倍数) | `conv_direct` との fixture 行列で A/B 担保。問題時は `fir_model` を `conv_direct` 呼出しに戻すだけでロールバック可(畳み込み呼出しは1箇所) |
| A3 plan 巻き上げで twiddle 再利用の仮定が崩れる(将来 `Fft` が内部状態を持つ場合) | `Fft::transform` は `&self` で現行は不変係数のみ参照 — ビット一致テストが検出する。ロールバックは `dsp::fft` 呼出しに戻す1行 |
| A4 置換の diff が大きい(linalg テスト全書換) | 列 fixture への機械的置換 + `mod legacy` oracle 維持で数学的正しさは担保。ロールバックは A4-opt コミットの revert(独立コミット境界) |
| `condition_number` が AᵀA 集計順変更で閾値超過的に揺れる | 述語対象外として sanity bound(INFINITY 整合・位数一致)に限定 — spec/ADR 確定済み |
| bench baseline の手順ブレ(ノイズによる誤判定) | 同一条件で `--save-baseline`/`--baseline` を使い ±5% ノイズ幅を超える回帰のみ問題視。Instant ベンチは複数回中央値 |
| `subtract` zero-fill が実は到達する経路を隠す | Aligned shared-length 契約(`stage.rs:283-295`)を根拠に記録済み。ragged fixture テストが防御を明示 |

## Deferred Design Concerns

spec の Deferred Design Concerns をそのまま継承する(spec 側を正とする):

- A2 `ncc` plan 再利用・A5 `local_lag`→`ncc` 置換 — issue #42 系
  の別チェーン
- stft フレームバッファの巻き上げ・出力行バッファ再利用 —
  バッファが出力そのものであり別境界
- criterion ベンチの CI 回帰ゲート — CI 基盤未整備、整備時に
  baseline 比較を再検討
- 複素乗算プリミティブ(`conv` 内スペクトル乗算の SIMD 化)—
  vec プリミティブ集合への追加は別チェーン、スカラループで
  十分と判断(FFT が支配的)

## Verification Notes

- [ ] `dsp::Fft` が re-export され `stft` ループ内に `Fft::new`
  がない。`dsp::fft` シグネチャと single-shot caller が不変で、
  stft 出力が固定 fixture でビット一致するテストが pass する
- [ ] `LeastSquares::from_columns` が公開 API として存在し
  `new` 相当契約を列向きに持ち、`new`/行 `least_squares` が
  除去されている。`solve` が `rhs.len()==観測数` を要求する
- [ ] `mixer.rs` の per-sample 行 `Vec` 構築と `linalg.rs` の
  strided 列再収集・行 clone がコード検査で除去されている。
  `fit_mix` 呼出し全箇所が列シグネチャを使う
- [ ] `dsp::conv` が存在し overlap-save(B 有界・M-1 discard・
  inverse 1/N 正規化・単一 plan 再利用)で実装され、
  `conv_direct` 参照が test/bench 内に保持される。`fir_model`
  が名前付き関数で直接畳み込みループを持たない
- [ ] `subtract` が `m.get(i).copied().unwrap_or(0.0)` に
  置き換わり ragged fixture で panic しないテストがある。
  zero-fill 修正が A1 適用より先のコミットに land している
- [ ] characterization テストが各最適化コミットに先行して存在し、
  opt 前後で green である(コミット履歴で確認)
- [ ] criterion ベンチ(stft・conv・lstsq)と
  `examples/bench_reference` の before/after 実測が
  Implementation Record に記録され ±5% を超える回帰がない
- [ ] `cargo test -p al-math -p al-locate -p al-analysis`・
  `--workspace`・`clippy`・`fmt --check` が green で golden
  テストが pass、`cargo tree -p al-analysis --depth 1` の直接
  依存に al-math がない
