---
id: "DESIGN-034ENh1Caa1pUN8HBonU2b"
type: "design"
status: "approved"
title: "機械的 caller クリーンアップ: bandpass バッチ化・csd インライン化・pass 集約 (A6-A16) の実装設計"
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
    - "../specs/mechanical-caller-cleanup.md"
  implemented-by: []
  depends-on:
    - "./al-math-vec-primitives.md"
    - "./observability-foundation.md"
    - "../adr/al-math-vec-primitives-slice-boundary.md"
    - "../adr/simd-reduction-lane-fma.md"
    - "../adr/stage-timing-manifest-persistence.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./caller-side-optimizations.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/mechanical-caller-cleanup.md"
    - "../discovery/mechanical-caller-cleanup.md"
  derived-by:
    - "../plans/mechanical-caller-cleanup.md"
    - "../test-specs/mechanical-caller-cleanup.md"
    - "../tasks/mech-cleanup-bench-baseline-characterization.md"
    - "../tasks/mech-cleanup-a6-bandpass-batching.md"
    - "../tasks/mech-cleanup-a7-a8-csd-inlining.md"
    - "../tasks/mech-cleanup-a9-a12-cheap-pass-consolidation.md"
    - "../tasks/mech-cleanup-a15-interleave-block-writes.md"
    - "../tasks/mech-cleanup-a16-row-major-fast-paths.md"
    - "../tasks/mech-cleanup-measurement-ir-closeout.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 機械的 caller クリーンアップ: bandpass バッチ化・csd インライン化・pass 集約 (A6-A16) の実装設計

## Context

`../specs/mechanical-caller-cleanup.md`(SPEC-034EMwMaXGBCfumSazX7zs,
approved)が束ねる機械的 caller 側クリーンアップ 6 項目
(A6・A7/A8・A9・A12・A15・A16)の実装形状を定める。site 一覧・
等価クラス・禁止パターン・プロセス契約は spec が権威であり、
本設計が spec と食い違う場合は spec に従う。

全項目が **bit-identical** 等価クラスに着地する
(A16d のみ「identical」= 返却 `(out, region_shape)` の完全一致)。
ADR-034EoHxzsvsHp9BOdCK2Cn(`../adr/simd-reduction-lane-fma.md`)の
tolerance predicate `|a-b| <= 1e-12*max(1,|a|,|b|)` は本バンドル
では一度も適用しない。spec 判断により **ADR は新設しない**
(全項目が既存ポリシの機械的適用)。

設計が解く残件(spec が design へ defer した Open Questions):

- A15 の書き込み chunk 粒度(per-frame staged row vs channel block)
- A16d tensor fast path の境界条件チェック配置
- characterization 捕捉粒度(function-level vs record-level)

## Scope

### In Scope

- **A6**: `reference.rs:742-747` の `.take(8)` ループ + 16 回
  `bandpass` 呼出しを、`&bands[..bands.len().min(8)]` スライス
  への 2 回の `bandpass_many` 呼出しへ置き換え(bit-identical)。
- **A7/A8**: 列挙 7 サイトの `csd` 返却 Vec 受け取りを、
  インライン `conj(x)·y` スカラー積へ置き換え(bit-identical)。
- **A9**: `cheap.rs:632-634`・`reference.rs:653-654` の
  `magnitude()`/`power()`/`psd()` 各 pass を `power()` 一回へ
  集約し、mag/psd を caller 側で導出。`cheap.rs:794-796`
  `mean_row` を同一 row-major pass に融合(bit-identical)。
- **A12**: `cheap.rs:577-603` の distinct count を
  `HashSet<u64>` 事前計数へ替え、gate 外では両 sort と
  f64 実体化を丸ごと skip(record-level bit-identical)。
- **A15**: `normalize.rs:168-171`・`reference.rs:1163-1170` の
  per-sample `extend_from_slice`/`push` と `dsp.rs:17-29` の
  per-sample `push` をブロック書き込みへ(byte/element 完全一致)。
- **A16**: `mean_psd`/`mean_power`/`mean_row` の行優先累積、
  `dom()` の magnitude 行列除去(scratch `mag_sq`)、video
  luma/diff の単一 pass 融合、tensor `read_range` の
  contiguous-row fast path(全て既存クラス内変更)。

### Out of Scope

spec の Out of Scope を継承する。要点: al-math カーネル変更、
公開 API 形状・永続化形式・record 意味の変更、新規 public API
(`StftMatrix` メソッド追加を含む)、al-analysis への直接
`al-math` 依存(`al_locate::dsp` facade 集約を維持)、新規依存、
al-dataset/al-media への新規 dev-dep、`mean_psd` の vec
reduction 化(tolerance-class、本バンドルでは不採用)、envelope
prefix-sum、`matrix_to_tensor` 統合、A2/A5/A10/A13/A14、
マルチスレッド化、golden 意味・Validity しきい値の変更、
互換層(pre-release の replace-only 方針)。

## Components and Boundaries

### T-baseline: characterization・baseline・コミット順序

spec のプロセス契約(最優先): 各項目について (1) 現行出力を
捕捉する characterization テストが**変更前**に存在し pass、
(2) bench baseline を変更前に採取し変更後に再測、
(3) before/after を `docs/impl/ir/` の Implementation Record に
記録、(4) characterization+baseline コミットが optimization
コミットより**先**に git history へ land すること。

#### characterization 捕捉粒度(deferred 決定)

**決定**: 永続化される値は record/artifact 級で捕捉し、
非永続の中間量はその下流 record で捕捉する。

- persisted(record スカラー・tensor 要素・`.f64le` バイト列):
  `f64::to_bits` / バイト列の完全一致を、決定的 synthetic
  入力で生成した出力へ適用する。期待値は既存 testkit fixture
  (`crates/al-testkit`)経由で再現可能な値に限り、magic
  literal のハードコードは避けて新旧同一経路の A/B 形
  (oracle をテスト内に残す)または record 同士の比較とする。
- 非永続(A7 の累積器): 累積結果が流入する下流 record
  (coherence・h・band gain・`stereo.coherence_mean`)の
  `to_bits` で間接捕捉する。
- 既存カバレッジの再利用: `dsp.rs:196` `deinterleave` 単体
  テスト、`tensor.rs` の `read_range` 単体テスト
  (round-trip/境界エラー)、`locate.rs` の normalized_pcm
  layout テスト群、各 `tests/golden.rs` のシナリオ pin は
  そのまま回帰として効く。これらは**現行出力の値 pin で
  はない**ため、spec 契約上の characterization は別途追加する。

#### characterization テスト配置

| 対象 | 配置 | 捕捉内容 |
| --- | --- | --- |
| A6 band_correlation | `al-analysis/tests/reference.rs` | band_correlation tensor 要素 `to_bits` + `bands < 8` edge(低 sample rate)での `.min(8)` 境界 |
| A7 累積系 | `al-analysis/tests/reference.rs`・`expensive.rs`・cheap 系テスト | 下流 record(h・coherence・band gain・`stereo.coherence_mean`)`to_bits` |
| A8 hft | `al-analysis/tests/reference.rs` | hft tensor 全要素 `to_bits` |
| A9 導出 | cheap 系テスト | `magnitude`/`power`/`psd` 各 tensor + `spec.slope`(mean_row 消費)record `to_bits`、`/ norm` リテラル不在のコード検査 |
| A12 quant | cheap 系テスト | `dig.quant_step_est` value・support・`Validity`: in-gate(2〜1M)と >1M distinct skip の両経路 |
| A15 interleave | `al-locate/tests/locate.rs`・dsp 単体テスト・`al-analysis/tests/reference.rs` | normalized `.f64le` バイト列・`deinterleave` 返却列・residual interleave 要素列の完全一致 |
| A16a mean | `al-analysis/tests/reference.rs`・`expensive.rs` | `band_gains`/`psd_ratio`/`total_power`/`one_sided_power` 等の下流 record `to_bits` |
| A16b dom | `al-analysis/tests/reference.rs` | `ref.harmonic_freq_ratio` record 一致 |
| A16c video | `al-media` 既存テスト配置 | luminance/diff 系列 `to_bits`・先頭 `diff=0.0`・`pixels==0` degenerate 出力 |
| A16d tensor | `al-dataset` `tensor.rs` 単体テスト・`tests/integration.rs` | `(out, region_shape)` 完全一致: fast path・fallback・innermost 部分包含 edge |

#### bench 基盤と baseline 採取

既存基盤を流用し、新規 harness は既存パターンの追加のみ。

- **al-analysis criterion**(dev-dep `criterion = "=0.8.2"` 既存、
  `[[bench]]` stanza を同一パターンで追加可):
  - `benches/` に `band_correlation` arm(A6): 8×`bandpass`
    loop vs `bandpass_many` の section 直接計測。
  - `csd_accum` arm(A7/A8): per-frame csd 累積と単要素
    csd パターンの alloc 削減を固定長 synthetic 入力で計測。
  - `quant_step` arm(A12): >1M distinct を持つ決定的入力で
    現行 double-sort vs HashSet gate を計測。
  - `spectral_pass` arm(A9/A16a/A16b): frames×bins の
    mag_sq pass 数削減・行優先累積・dom scratch を計測。
- **`examples/bench_reference.rs`** + manifest `duration_ms`:
  A6・A9・A16a/b の stage 級代表経路。
- **al-locate `examples/bench_dsp.rs`**(Instant 計時の既存
  example): A15 の deinterleave/interleave が載るなら流用、
  載らなければ既存 example 内の Instant 区間追加に留める。
- **al-media(A16c)/al-dataset(A16d)**: criterion/example
  基盤なし・新規 dev-dep 禁止。al-dataset は
  `tests/integration.rs` が tensor round-trip を既に網羅する
  ため、同 test binary 内の Instant micro-timing で計測。
  al-media は既存 example/test に合致するものがあれば
  Instant 区間、なければ **code inspection + workspace bench
  回帰ゲート + manifest `duration_ms`** とし、採用方式を IR に
  明記する(spec の fallback 規定)。
- **baseline**: 最適化コミット前に `cargo bench` /
  example 出力を `target/bench-baseline/` へ記録
  (コミット対象外)。変更後に同一条件で再測し IR に
  before/after を転記する。

#### コミット順序

1. **char — characterization + bench arm**: 上表の
   characterization テスト群と新規 criterion arm(挙動不変)。
   このコミット後に baseline を採取する。
2. **A6-opt**: `bandpass_many` バッチ化。併せて未使用化する
   `reference.rs:13` の `bandpass` import を同一コミットで
   prune する(A6 項の「import prune」参照)。
3. **A7A8-opt**: csd インライン化 7 サイト(同一イディオムの
   ため 1 コミット)。併せて未使用化する consumer file の
   `csd` import を同一コミットで prune する(A7/A8 項の
   「import prune」参照)。
4. **A9A12-opt**: cheap.rs 側 pass 集約 + HashSet 事前計数
   (同一関数群内の機械的変更として束ねる)。
5. **A15-opt**: interleave/deinterleave 3 サイト。
6. **A16-opt**: mean 行優先・dom scratch・video 融合・
   tensor fast path。A9 item の `reference.rs:653-654`
   psd→power 側もこのコミットに含める(A16a と同一変更単位
   のため — item ↔ commit の対応は 1:1 ではない)。
7. 各 `-opt` コミットまたは直後の `docs` コミットで、対応する
   `docs/impl/ir/` へ before/after を記録する。

各 `-opt` は独立適用可能で、任意のコミット単位で revert しても
他項目の等価性に影響しない。

### A6 — band_correlation の bandpass_many バッチ化

`reference.rs:742-747` を次の形状へ置き換える。

```rust
// bands.iter().take(8) 相当を bound-safe なスライスで表現し、
// forward FFT を signal ごとに 1 回へ集約する。
let capped = &bands[..bands.len().min(8)];
let xs = bandpass_many(src_mono, rate, capped);
let ys = bandpass_many(cap_mono, rate, capped);
let band_corrs: Vec<f64> = xs.iter().zip(&ys).map(|(x, y)| corr0(x, y)).collect();
```

- **等価の根拠**: `bandpass` は `bandpass_many(.., &[(lo, hi)])`
  `.pop()` の thin wrapper(`crates/al-math/src/filter.rs:7-11`)
  であり、mask 規則・切り戻し・出力順(bands 順)が同一。
  `corr0` を band 順に適用すれば `band_corrs` は逐次版と
  bit-identical。
- **禁止**: `&bands[..8]` リテラル(`bands < 8` で panic)。
  必ず `bands.len().min(8)` で bound する。
- **依存**: `bandpass_many` は `al_locate::dsp` から既に
  re-export 済み(`dsp.rs:34`)。`use` への追加のみ。
- **import prune(同一コミット必須)**: 本項目の置換で
  `reference.rs:744-745` の `bandpass(` 呼出しは全て消え、
  `reference.rs:13` の `use ... bandpass` が未使用 import
  になる(`cargo clippy -D warnings` で fail する)。
  `bandpass` を use リストから除去し `bandpass_many` へ
  入れ替える。`al-math`/`dsp.rs` 側の `bandpass` 公開面自体は
  残す — 削るのは consumer file の `use` 記述のみ。
- **計測**: plan/forward が 16 → 2。`band_correlation` arm と
  `bench_reference.rs` `duration_ms` で確認。

### A7/A8 — csd 呼出しのインライン `conj(x)·y` 化

`stft.rs:173-179` の `csd` は要素式 `conj(x)·y`、すなわち
`(xr*yr + xi*yi, xr*yi - xi*yr)` を返す(`:177`)。各 caller は
返却 Vec ではなく、同じ式を累積ループ内で直接計算する。

```rust
// パターン(各サイトの変数名に合わせる):
let pr = xr * yr + xi * yi;   // c[k].0 相当
let pi = xr * yi - xi * yr;   // c[k].1 相当
accum[k].0 += pr;
accum[k].1 += pi;
```

| サイト | 現行 | 変更 |
| --- | --- | --- |
| `reference.rs:183` (fir_ir) | `csd(&from_stft.re[f], …, &mt.*)` → `cxy` | 全 bin で inline 積、`c[k]` 読みを除去。`cxx` の `powi(2)` 式は現行のまま |
| `reference.rs:539-545` (Welch) | `csd(&ms.*, &mc.*)` → `pxy` | 同上。`pxx`/`pyy` の `r*r+i*i` 式は現行のまま |
| `reference.rs:872-877` (L/R coherence) | `csd(&m0.*, &m1.*)` → `pab` | 同上 |
| `band_gain.rs:159` (coherence) | `csd(&a.*, &b.*)` → `xy` | 同上 |
| `cheap.rs:1105-1120` (stereo pair) | `csd(&ma.*, &mb.*)` → `csd_re/csd_im` | `pxy.iter().enumerate()` を `0..ma.fft_size` の inline 積へ。frames cap `min(64)` 維持 |
| `expensive/channel.rs:102-118` | 全 bin の csd を生成し `band_bins` のみ消費 | `for &k in &band_bins` 内でその bin のみ積を計算(最大の削減) |
| `reference.rs:587-593` (A8 hft) | 長さ 1 の `csd` 呼出し + `c[0]` | スカラー `(cr, ci)` へ |

- **不変条件**: 加算順(bin 昇順・frame 昇順)と各サイトの
  auto-power 式(`powi(2)`/`r*r+i*i`)は現行のまま。**変えるのは
  `c[k]` 参照を inline 式へ置き換える部分だけ**。
- **A8 の形**(`reference.rs:587-593`):

```rust
let xs = ms.re[f][k] * ms.re[f][k] + ms.im[f][k] * ms.im[f][k];
if xs > 0.0 {
    let (xr, xi) = (ms.re[f][k], ms.im[f][k]);
    let (yr, yi) = (mc.re[f][k], mc.im[f][k]);
    let cr = xr * yr + xi * yi;
    let ci = xr * yi - xi * yr;
    ((cr * cr + ci * ci).sqrt()) / xs
} else {
    0.0
}
```

  `c[0]` 相当の式・`xs` ガード・else 0.0 を完全に保持し、
  frames×half ≈ 8.6M の微小 heap alloc を除去する。
- **`csd` 関数は public のまま残す**(`stft.rs` の
  `#[cfg(test)]` が `stft.rs:390` で使用)。削除しない。
- **import prune(同一コミット必須)**: 7 サイトの inline 化で
  全ての production `csd` 呼出しが消え、`reference.rs:18`・
  `cheap.rs:16`・`band_gain.rs:16`・`expensive/channel.rs:9`
  の `use ... csd` が未使用 import になる
  (`cargo clippy -D warnings` で fail する)。各 consumer
  file の use リストから `csd` を除去する。`stft.rs`/
  `dsp.rs` 側の公開面は変えない(残る `csd` 使用は
  `#[cfg(test)]` の `stft.rs:390` のみ)。

### A9 — `power()` 一括化と caller 側導出

`StftMatrix::{psd, magnitude, power}`(`stft.rs:48-89`)はいずれも
frames×bins の `mag_sq` pass を各自行う。caller 側で `power()`
一回から導出する。

```rust
// cheap.rs:632 付近 — m の move(cheap.rs:665)「前」に全て行う。
let pow = m.power();                                   // 唯一の mag_sq pass
let norm = (0.375 * m.window as f64) * m.sample_rate as f64;
let inv = 1.0 / norm;
let half = m.fft_size / 2;
let mut mean_row = vec![0.0; half];
let mag: Vec<Vec<f64>> = pow
    .iter()
    .map(|row| {
        let r: Vec<f64> = row.iter().map(|&v| v.sqrt()).collect();
        for (k, &v) in r.iter().take(half).enumerate() {
            mean_row[k] += v;                       // row 昇順・k < half
        }
        r
    })
    .collect();
for v in &mut mean_row {
    *v /= mag.len() as f64;                           // ÷ len
}
let psd: Vec<Vec<f64>> = pow
    .iter()
    .map(|row| row.iter().map(|&v| v * inv).collect())
    .collect();
```

- **norm 形**: `window_power()`(`stft.rs:41-44`、private)と同じ
  `0.375 * window` × `sample_rate`。`window`/`sample_rate` は
  pub field。psd は **`v * inv`(reciprocal 乗算)のみ**。
  `/ norm` リテラルは禁止(ulp 差)。現行 `psd()` の
  `scale(&power, 1.0/norm, …)` と同じ乗算なので bit-identical。
- **ordering**: `mag`/`psd`/`mean_row` の導出と `m.fft_size`・
  `m.window`・`m.sample_rate` の読みは全て
  `channel_stft.push(Some(m))`(`cheap.rs:665`)より前に完了
  させる。
- **`half` の重複定義を避ける**: sketch で導入する
  `let half = m.fft_size / 2` と既存 `cheap.rs:668` の
  `let half = mag[0].len() / 2` は同一値(`mag` の行長 =
  `fft_size`)。`:668` 側を削除し、push 前に定義した `half`
  を再利用する(shadow・重複定義にしない)。
- **`mean_row` 融合**(`cheap.rs:794-796`): 現行の
  `mag.iter().map(|r| r[b]).sum() / mag.len()` と、加算対象
  (`sqrt` 済み要素)・加算順(row 昇順)・除算が同一で
  bit-identical。`spec.slope` 以降の消費点は変えない。
- **reference.rs:653-654**: `psd_s`/`psd_c` の消費者は
  `mean_psd_*` のみ(grep 済み)。A16a と併せて、
  `ms.power()`/`mc.power()` 一回ずつ + 行優先累積で
  `mean_psd_*[k] += row[k] * inv` とし、psd 行列自体は
  実体化しない(`row[k] * inv` が `scale` 出力要素と同じ値)。
- **単一消費者の例外(変えない)**: `cheap.rs:942`
  `mm.magnitude()`、`reference.rs:1189` `rm.psd()`。
  `reference.rs:1071` の `m.magnitude()` は A16b で除去。

### A12 — distinct count の HashSet 事前計数

`cheap.rs:577-603` を次の形状へ。

```rust
let distinct: std::collections::HashSet<u64> =
    samples.iter().map(|v| v.to_bits()).collect();
let quant = if distinct.len() >= 2 && distinct.len() <= 1_000_000 {
    let mut sorted: Vec<f64> =
        distinct.iter().map(|b| f64::from_bits(*b)).collect();
    sorted.sort_by(|a, b| a.total_cmp(b));
    sorted
        .windows(2)
        .map(|w| w[1] - w[0])
        .filter(|d| *d > 0.0)
        .fold(f64::INFINITY, f64::min)
} else {
    f64::NAN
};
// support には distinct.len() as u64 を使う(現行 uniq.len() と同値)
```

- **等価の根拠**: `to_bits` キーの dedup count は `uniq.len()`
  と一致。gate 内では `total_cmp` sort 後の `windows(2)` 最小
  正差が同じ集合・同じ順序で求まるため `quant`・support・
  `Validity` は record-level で bit-identical。gate 外(運用上
  >1M)では u64 sort・f64 実体化・f64 sort のすべてを skip。
- **禁止**: distinct count の近似(support が変わるため)。
  `HashSet` は std のみ(新規依存なし)。
- **characterization は両経路必須**: in-gate と >1M distinct
  skip を別 fixture で pin する。

### A15 — interleave/deinterleave のブロック書き込み化

**chunk 粒度の決定(deferred)**: 書き込み側は「staged row +
ブロック転送」、読み出し側は「channel-block 単位の strided
収集」とする。frame-major の配置順はいずれも不変。

- `normalize.rs:168-171`(interleave → bytes): 1 frame 分の
  channel サンプルを再利用 row scratch に詰め、frame ごとに
  1 回 `extend_from_slice` する。per-sample の 8B extend
  (~17M 規模)を per-frame extend へ削減。

```rust
let ch = channels.len();
let mut bytes = Vec::with_capacity(params.frames as usize * ch * 8);
let mut row = vec![0u8; ch * 8];
for frame in 0..params.frames as usize {
    for (c, channel) in channels.iter().enumerate() {
        row[c * 8..c * 8 + 8].copy_from_slice(&channel[frame].to_le_bytes());
    }
    bytes.extend_from_slice(&row);
}
```

- `reference.rs:1163-1170`(residual interleave → `Vec<f64>`):
  `n * channels` に事前確保し indexed write(frame-major)。

```rust
let c = residual.len();
let mut interleaved = vec![0.0; n * c];
for i in 0..n {
    for (ch_idx, ch) in residual.iter().enumerate() {
        interleaved[i * c + ch_idx] = ch[i];
    }
}
```

- `dsp.rs:17-29`(deinterleave): 現行は frame-major に
  `out[ch].push(v)`。channel-block 単位で strided gather し、
  各 channel row を連続 write する(`chunks_exact` により
  trailing partial frame drop も同一)。

```rust
let out: Vec<Vec<f64>> = (0..channels)
    .map(|ch| {
        interleaved
            .chunks_exact(channels)
            .map(|frame| frame[ch])
            .collect()
    })
    .collect();
```

- **等価の根拠**: 配置問題であり値は変わらない。byte 列・
  要素列が完全一致すれば bit-identical。frame-major の
  interleave 順(transpose しない)を両方向で維持する。

### A16 — 行優先累積・dom scratch・video 融合・tensor fast path

#### A16a: `mean_psd`/`mean_power` の行優先スカラー累積

- `reference.rs:655-660`: A9 の `pow` 行を row 昇順に走査し
  `mean_psd[k] += row[k] * inv`(`k` 昇順)。最後に
  `/ frames`。行側の走査範囲は
  `frames = ms.frames().min(mc.frames())`(`:516`)の
  `.take(frames)` bound を保持する — `Aligned` 契約下では
  恒等だが、防御 bound は faithful transcription のため
  そのまま残す。
- `expensive/mod.rs:648-651`: `power` 行列の
  `mean_power[k] += row[k]` を row 昇順で累積し `/ frames`。
  `total_power`/`one_sided_power` の消費は不変。
- **スカラー必須**: 0.0 始まりの row 昇順加算を保つため vec
  reduction primitive へは置き換えない(加算順が変わり
  tolerance-class になる — 本バンドルの唯一の tolerance 候補
  だったが spec が scalar を選択)。
- `cheap.rs:794-796` の `mean_row` は A9 の row-major pass に
  融合済み(上記)。

#### A16b: `dom()` の magnitude 行列除去

`reference.rs:1071-1084`。`m.magnitude()` 全行列の実体化を
やめ、fft_size 長 scratch 1 本へ `mag_sq` して `sqrt` 累積する。

```rust
let dom = |m: &crate::stft::StftMatrix| -> f64 {
    let half = m.fft_size / 2;
    let mut mean = vec![0.0; half];
    let mut scratch = vec![0.0; m.fft_size];   // 呼出しごとに 1 本
    for f in 0..m.frames() {
        mag_sq(&m.re[f], &m.im[f], &mut scratch);
        for k in 0..half {
            mean[k] += scratch[k].sqrt();
        }
    }
    mean.iter()
        .enumerate()
        .skip(1)
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(b, _)| b as f64 * bin_hz)
        .unwrap_or(0.0)
};
```

- **等価の根拠**: `magnitude()` の要素は `mag_sq → mag_sq_kernel
  → simd.mul_add_e_f64s`(`vec.rs:128-139`)経由で、x86-v3 lane
  は真 FMA・Scalar lane と `len % lanes` tail は非融合
  `r*r+i*i` という**位置依存の丸め**を持つ。手書きの
  `re*re + im*im` はこの配置依存を再現できず**禁止**。
  `mag_sq` 自体を呼べば要素値・累積順とも一致。
- `power()` 再利用は不可(argmax(Σ√x) ≠ argmax(Σx))。
- `mag_sq` は `al_locate::dsp` facade から import する
  (re-export 済み `dsp.rs:35`)。al-analysis へ直接の
  `al-math` 依存を足さない。
- argmax 部(`skip(1)`・`total_cmp`)は不変。

#### A16c: video luma/diff 融合

`video.rs:402-441` の全画素 luma 和(`:427`)と prev との diff
zip-sum(`:428-440`)を 1 pass の別 accumulator へ融合する。

```rust
let mut luma_sum = 0.0;
let mut diff_sum = 0.0;
for (i, &a) in frame.luma.iter().enumerate() {
    luma_sum += a;
    if let Some(p) = &prev {
        // 現行 zip と同じ silently-truncate semantics:
        // p が短い場合は index せず skip する(p[i] は panic へ
        // failure mode が変わるため禁止)
        if let Some(&b) = p.get(i) {
            diff_sum += (a - b).abs();
        }
    }
}
luminance.push(luma_sum / pixels as f64);
diff.push(if prev.is_some() { diff_sum / pixels as f64 } else { 0.0 });
scratch = prev.replace(frame.luma).unwrap_or_default();
```

- **保持条件**: 各和の加算順(luma 配列の昇順)は不変で
  bit-identical。先頭 frame の `prev == None` 分岐は
  `diff = 0.0` リテラルを再現する(`0.0/pixels` ではない —
  `pixels == 0` でも 0.0)。`pixels == 0`(`:391`)では
  `luma = 0.0/0.0 = NaN` 系列が現行通り出る。diff 側は現行
  `zip` の silently-truncate semantics を `p.get(i)` で
  再現する — `p[i]` index 化は長さ不一致時に panic へ
  failure mode が変わるため禁止(decoder 契約下到達不能
  だが、同 cost で semantics を保持する)。ROI 和
  (`:413-424`)は系列別部分和で対象外。

#### A16d: tensor `read_range` contiguous-row fast path

`tensor.rs:271-289` の local 座標 walk を、最内 dim を
「行」として扱う形へ再構成する。

- **境界チェック配置の決定(deferred)**: per-chunk の
  `local` walk 内で、outer dims(`local[0..ndim-1]`)が region
  内に収まる行単位で判定する。最内 dim の走査開始点
  (`local[ndim-1] == 0` の行頭)で、その chunk 行の最内
  extent `[g_lo, g_lo + extent_last)` が region の最内
  `[region[last][0], region[last][1])` に**完全包含**される
  なら `copy_from_slice` で 1 行コピー、そうでなければ
  (部分包含を含め)現行の per-element gather へ fallback。
- src は `Σ local[d]*ext_strides[d]`(+ 行内 offset は 0)、
  dst は `Σ (global_d - region[d][0])*region_strides[d]`。
  両 layout とも最内 stride = 1 なので完全包含なら常に
  連続区間になる。outer-dim の src/dst base は行ループの
  外へ hoist してよい(値は不変)。
- `increment` は最内 dim を除く outer dims へ適用する形に
  限定する(`local[..ndim-1]`/`extent[..ndim-1]`)。
- **fallback 維持**: 部分包含 edge(行の一部だけ region 内)
  は per-element gather が `inside` 判定を要素単位で行う。
  返却 `(out, region_shape)` は両経路で完全一致。

### ファイルマップ

| ファイル | 変更 |
| --- | --- |
| `crates/al-analysis/src/reference.rs` | A6 バッチ化・A7×3 サイト・A8 hft・A9+A16a(psd_s/c→power+行優先)・A15 residual interleave・A16b dom scratch |
| `crates/al-analysis/src/cheap.rs` | A9 pass 集約 + mean_row 融合・A12 HashSet・A7 stereo csd |
| `crates/al-analysis/src/band_gain.rs` | A7 coherence `xy` |
| `crates/al-analysis/src/expensive/channel.rs` | A7 sxy(band_bins のみ) |
| `crates/al-analysis/src/expensive/mod.rs` | A16a mean_power 行優先 |
| `crates/al-locate/src/normalize.rs` | A15 staged-row interleave |
| `crates/al-locate/src/dsp.rs` | A15 channel-block deinterleave |
| `crates/al-media/src/video.rs` | A16c luma/diff 融合 |
| `crates/al-dataset/src/tensor.rs` | A16d read_range fast path |
| `crates/al-analysis/benches/`(+`Cargo.toml` stanza) | T-baseline の新規 arm |
| 各 `tests/`・`docs/impl/ir/` | characterization・IR |

変更しない: `stft.rs` の `csd`/`psd`/`magnitude`/`power`
(公開 API、内部は不変)、`al-math` 全般、`filter.rs`、
`vec.rs`、`matrix_to_tensor`、永続化形式、record 意味。

### テスト計画(対象ごとのゲート)

1. **A6 — band_corrs 等価**: `to_bits` 要素一致 +
   `bands < 8` edge + `bandpass_many` 呼出しが 2 回・
   `.min(8)` bound のコード検査。
2. **A7/A8 — 累積等価**: 下流 record/tensor の `to_bits`
   一致 + 列挙サイトに `csd` 呼出しが残らないコード検査。
3. **A9 — 導出等価**: mag/pow/psd tensor・`spec.slope`
   `to_bits` 一致、`/ norm` リテラル不在、`:665` より前の
   導出完了のコード検査。
4. **A12 — quant 両経路**: `dig.quant_step_est` の
   value/support/`Validity` が in-gate・>1M skip で一致。
5. **A15 — byte/element 一致**: `.f64le` バイト列・
   deinterleave 列・residual interleave 列の完全一致。
6. **A16 — record 等価**: `mean_psd`/`mean_power` 系・
   `ref.harmonic_freq_ratio`・video luma/diff `to_bits` 一致、
   tensor 両経路一致、`mag_sq` scratch 使用・手書き
   `r*r+i*i` 不在のコード検査。
7. **bench delta**: `target/bench-baseline/` との比較を IR に
   記録。非対象経路に ±5% を超える回帰がないこと。
8. **全体**: `cargo test --workspace`・clippy・fmt・golden
   (`tests/golden.rs`)が green。

## Data and Control Flow

```text
[A6]   analyze_pair → bands.min(8) slice
         → bandpass_many(src) / bandpass_many(cap)   # forward 2回
         → zip → corr0 → band_corrs

[A7/8] per-frame loop → conj(x)·y inline (bin 昇順)
         → cxx/cxy・pxy・pab・xy・csd_re/im・sxy・hft 要素

[A9]   m.power() 1 pass
         ├→ mag  (sqrt / row-major, mean_row 融合, k<half, ÷len)
         ├→ psd  (* inv)                              # cheap
         └→ mean_psd_* (row[k]*inv 行優先累積 ÷frames) # reference
         ※ すべて channel_stft.push(Some(m)) より前

[A12]  samples → HashSet<u64> count
         ├ 2 ≤ n ≤ 1M → sort(total_cmp) → min positive diff
         └ otherwise  → NaN / NotApplicable (両 sort skip)

[A15]  interleave: frame → staged row → extend_from_slice
       residual : frame-major indexed write (n*c 事前確保)
       deinterleave: channel-block strided gather → 連続 write

[A16b] dom(): per-frame mag_sq → scratch[k].sqrt() 累積 (k<half)
       → skip(1) argmax → harmonic_freq_ratio
[A16c] frame.luma 1 pass → luma_sum + diff_sum(prev Some)
[A16d] chunk local walk → outer-dim inside + 最内完全包含
         → copy_from_slice / else per-element gather
```

## Non-Functional Constraints

- **数値契約**: 全項目 bit-identical(A16d は返却値完全一致)。
  tolerance predicate は適用しない。`mean_psd`/`mean_power`/
  `mean_row` は scalar row-ascending 累積を維持する。
- **禁止パターン**(spec 確定): `&bands[..8]` リテラル、
  `/ norm` 除算、dom() 内の手書き `re*re+im*im`、A12 の近似
  distinct count、`cheap.rs:665` の `m` move 後の派生読み、
  新規 public API(`StftMatrix` メソッド含む)、al-analysis の
  直接 `al-math` 依存、新規依存、al-dataset/al-media の新規
  dev-dep。
- **プロセス契約**: char+baseline コミットが opt コミットに
  先行し git history で検証可能であること。IR に項目ごとの
  計測方式と before/after を記録すること。
- **facade 集約**: `mag_sq`/`bandpass_many`/`scale` は
  `al_locate::dsp` 経由のみ。

## Risks and Trade-offs

| リスク | 緩和・ロールバック |
| --- | --- |
| A6: `.min(8)` を忘れ `bands < 8` で panic | char テストに低 rate edge を含める。単一ブロックなので revert 容易 |
| A7/A8: inline 式の引数順取り違え(`conj(x)·y` の x/y)・auto-power 式の不用意な統一 | 下流 record `to_bits` ゲート。サイトごとに現行式を写すだけの機械的変更。サイト単位で revert 可 |
| A9: `* inv` でなく `/ norm` を書く・move 後に m を読む | コード検査 + `to_bits` ゲート。導出は独立式なので差し戻し容易 |
| A9+A16a: psd 行列非実体化で隠れた消費者を落とす | `psd_s`/`psd_c` の消費者は `mean_psd_*` のみと grep 確認済み。コンパイルが即座に検出 |
| A12: HashSet で順序非決定に見える懸念 | 値へ影響するのは `total_cmp` sort 後の集合のみで順序は一意。両経路 char 済み |
| A15: staged/indexed write の境界ずれ | バイト列完全一致テスト。各サイト独立で revert 可 |
| A16b: 手書き `r*r+i*i` で lane/tail 丸めずれ | `mag_sq` 呼出し必須を設計に明記 + record 一致 + コード検査 |
| A16c: `None=>0.0`/`pixels==0` の分岐欠落 | 系列 `to_bits` + degenerate fixture で pin |
| A16d: fast path の境界判定誤り | 完全包含のみ fast path・部分包含は現行 gather へ fallback。両経路の完全一致テスト |
| 全体: 測定対象外経路の回帰 | baseline 比較 ±5% ゲート + workspace bench 回帰 |

## Deferred Design Concerns

spec の deferred を継承する(本設計で新たな defer はない)。

- A2 `ncc` の `RealFft` plan 再利用(新規 API surface を要し
  別チェーンへ)、A5 `local_lag→ncc`(数値契約の判断要)。
- `mean_psd` の vec reduction 化(tolerance-class; 将来の
  別バンドルで再検討。再着手条件: reduction lane/FMA 順序を
  固定する primitive 契約の整備)。
- envelope prefix-sum・`matrix_to_tensor` 統合・
  A10/A13/A14・residual-gather/lod 系項目。

## Verification Notes

- [ ] char コミットが全 `-opt` コミットより先に git history にある。
- [ ] `&bands[..bands.len().min(8)]` のみで `bandpass_many` 呼出しは 2 回。
- [ ] 列挙 7 サイトに `csd(` 呼出しが残らず、`stft.rs` の `csd` 定義は public のまま。
- [ ] psd 導出が `* (1.0/norm)` 形のみで `/ norm` リテラルがない。
- [ ] cheap.rs の全導出が `channel_stft.push(Some(m))` より前。
- [ ] `mean_psd`/`mean_power`/`mean_row` が scalar row-ascending 累積。
- [ ] `dom()` が `mag_sq` + 再利用 scratch を使い、手書き `re*re+im*im` がない。
- [ ] A12 が gate 外で両 sort・f64 実体化を skip し、support が `distinct.len()`。
- [ ] `.f64le` バイト列・deinterleave 列・residual interleave 列・
      tensor `(out, region_shape)` が完全一致テストを持つ。
- [ ] `target/bench-baseline/` に baseline があり、各 IR に
      計測方式と before/after が記録されている。
- [ ] `cargo test --workspace`・clippy・fmt・golden が green。
