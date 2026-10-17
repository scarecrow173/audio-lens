---
id: "DESIGN-034H77LaRZ9cyfPIcdYe0w"
type: "design"
status: "draft"
title: "Streaming Bandpass 設計 (issue #71 P2)"
created: "2026-10-16"
updated: "2026-10-16"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/71"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/streaming-bandpass-issue-71-p2.md"
    - "../adr/bandfilter-inputcache.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./valid-fft-ncc-plan-issue-71-p1.md"
    - "./adaptive-ncc-dispatch-issue-71-p3.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/streaming-bandpass-issue-71-p2.md"
    - "../adr/bandfilter-inputcache.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Streaming Bandpass 設計 (issue #71 P2)

## Context

`band_gain::analyze` は現行 `InputCache::prepare_with_bands` で
`PreparedChannel{bands: Vec<BandSignal>, coarse, ...}` を構築する。
`bandpass_many` は 1 forward FFT + 帯域ごと IFFT を一括実行し、
全帯域を `Vec<Vec<f64>>` で返すため、呼出側が逐次消費でも全帯域が
生存する。加えて `prepare_with_bands` は未使用の `coarse` 間引き
信号を必ず計算する。

実ジョブ(44.1kHz、n≈2.2M、26 帯域、2 入力 × 2ch)で約 1.8GB が
`analyze` 終了まで同時生存する。消費は帯域 i の (src, cap) ペアを
`windows()` に通して捨てるだけであり、逐次生成でよい。

## Scope

### In Scope

- al-math `filter.rs` への `BandFilter` 追加(forward スペクトル
  保持 + `band(lo, hi)` で mask/IFFT/切り戻し)。
- `bandpass_many` を `BandFilter` へ委譲(filter 仕様の実装を
  一箇所に集約)。
- `al_locate::dsp` の re-export に `BandFilter` を追加
  (band_gain は `al_locate::dsp` 経由が既存の境界)。
- `band_gain::analyze` を帯域ループ内逐次生成へ変更し、
  `InputCache` 不使用にする。
- 数値一致・メモリ削減の検証。

### Out of Scope

- locate 側 `InputCache`/`PreparedChannel`/`prepare_with_bands`
  の変更(coarse は coarse_ncc が使用)。
- `windows()` の NCC plan 共有・Blocked FFT・適応 dispatch(P3)。
- `window_anchors_in`(issue #76)。
- coherence STFT など analyze の他セクション。

## Components and Boundaries

### `BandFilter`(al-math `filter.rs`)

forward スペクトルと IFFT 用スクラッチを保持する不変 plan。

```rust
pub struct BandFilter {
    plan: RealFft,
    bin_hz: f64,
    len: usize,
    spectrum_re: Vec<f64>,  // n/2+1
    spectrum_im: Vec<f64>,  // n/2+1
    masked_re: Vec<f64>,    // n/2+1 (band() のスクラッチ)
    masked_im: Vec<f64>,
    real: Vec<f64>,         // n (IFFT 出力バッファ)
}
```

- `BandFilter::new(samples: &[f64], sample_rate: u32) -> Self`
  - `n = samples.len().next_power_of_two()`、ゼロ埋めして 1 回だけ
    `forward_with_simd`。内部は `dispatch(ForwardKernel)`。
  - `samples.is_empty()` では `len = 0`・空スペクトルで構築し、
    `band()` は空 `Vec` を返す(現行 `bandpass_many` の早期 return と
    同じ観測結果)。
- `BandFilter::band(&mut self, lo_hz: f64, hi_hz: f64) -> Vec<f64>`
  - `masked_*` を 0 埋め → `lower_bin` で半開帯域の bin 範囲を求めて
    スペクトルからコピー → `inverse_with_simd` → `real[..len]`
    をコピーして返す。内部は `dispatch(InverseKernel)`。
  - `&mut self` はスクラッチ共有のため。返却 `Vec` のみが生存資源。

内部の SIMD カーネルは既存 `Bandpass`/`Envelope` と同じ
`WithSimd` パターン。mask+IFFT の本体は `Bandpass::with_simd` と
`BandFilter::band` の両方から呼ぶ共通内部関数とし、bin 判定
(`lower_bin`)・境界規則・切り戻しを単一実装にする。

### `bandpass_many` の委譲

```rust
pub fn bandpass_many(samples, rate, bands) -> Vec<Vec<f64>> {
    dispatch(Bandpass { samples, sample_rate, bands })
}
```

`Bandpass::with_simd` 内で forward 1 回 + 帯域ごとの共通内部関数を
呼ぶ構造に整理し、出力・空入力挙動・`hi.is_nan()` 規則は不変。
外部から見える差分はない。

### `band_gain::analyze` の再構成

現行:

```rust
let a = cache.prepare_with_bands("source", ch, rate, alignment, x, &bands);
let b = cache.prepare_with_bands("capture", ch, rate, alignment, y, &bands);
let all: Vec<_> = a.bands().iter().zip(b.bands())
    .map(|(((lo,_),x),(_,y))| windows(x, y, window, hop, max, guard(lo)))
    .collect();
```

変更後:

```rust
let mut fx = BandFilter::new(x, rate);
let mut fy = BandFilter::new(y, rate);
let all: Vec<_> = bands.iter().map(|&(lo, hi)| {
    let xb = fx.band(lo, hi);
    let yb = fy.band(lo, hi);
    windows(&xb, &yb, window, hop, max, guard(lo))
}).collect();
```

- `InputCache` の import・生成を band_gain から除去。
  `prepare_with_bands` の coarse/radius/FilterKey 構築が丸ごと消える。
- `coherence(x, y, ...)` は帯域波形を使わず既存のまま。
- 帯域順・`guard = ceil(rate/lo)`・`all` の要素順は不変。

## Data and Control Flow

チャンネルごと:

1. `BandFilter::new(x)` / `BandFilter::new(y)` — 各 1 forward FFT、
   スペクトル 2 本が生存(n≈4.2M の実ジョブで ~34MB×2)。
2. 帯域 i: `fx.band`/`fy.band` → 帯域波形 2 本だけ生存
   (~17MB×2)→ `windows()` → `Vec<Window>` のみ `all` に残す。
3. `all` は従来通り select/aggregate/record/tensor へ。

生存するのはスペクトル 2 + 現帯域 2 + `all`(Window は ~48B ×
26×915 ≈ 1.1MB)。現行の帯域波形 26×2ch×2 入力 ≈1.8GB に対し
~100MB 規模になる見込み。

## Non-Functional Constraints

- 帯域ごとの出力は `bandpass_many` とビット一致(同一 forward・
  同一 mask・同一 IFFT・同一切り戻し)。
- scalar↔runtime-dispatch 同等:`band()` も `dispatch` 経由。
- `prepare_with_bands`/`PreparedChannel`/`InputCache` の契約は
  変更しない(locate 側が使用中)。
- band_gain の出力(record・tensor・帯域順・guard・400Hz anchor・
  polarity・delta/reference)は不変。

## Risks and Trade-offs

- **`band()` 呼出ごとの dispatch オーバーヘッド**: arch 検出は
  キャッシュされ軽微。IFFT 回数は現行と同じ(26/入力/ch)。
- **scratch の使い回しと `&mut`**: `band()` が `&mut self` を取る
  ため並列で同一フィルタを呼べない。band_gain の呼出は逐次であり、
  並列化が必要なら入力 × ch 単位で別 `BandFilter` を持てばよい
  (forward FFT は入力ごと 1 回で済む)。
- **同一帯域の再呼出で再 IFFT**: band_gain は各帯域を 1 回消費。
  複数回消費する将来の呼出側では `bandpass_many` 選択の余地を残す
  (両 API を残す設計)。
- **alloc churn**: `band()` が返却 `Vec` を都度確保(帯域あたり
  ~17MB)。同サイズの繰り返し確保は allocator が再利用し、IFFT
  コストに対し微小。`all` への保持がないため同時生存は 2 本。

## Deferred Design Concerns

- locate 側 `PreparedChannel` の逐次化/スペクトル保持: locate の
  BandNcc は 5 帯域で小さく、coarse 共有・`release` 意味論・残差
  version 管理が絡む。#76/#54 検討時に再評価。
- `windows()` NCC plan の帯域間共有: 現行は `windows()` 内で
  帯域ごとに `CorrelationPlan::new_ncc` を構築している(同一形状
  なのに 26 回)。P3 の dispatch 設計と一緒に扱う。
- `BandFilter` の `Arc` 共有/複数消費者対応: 現需要なし(YAGNI)。

## Verification Notes

- [ ] al-math テスト: `BandFilter::band` 各帯域が `bandpass_many`
      対応帯域と一致(複数形状・NaN hi・空入力)。
- [ ] scalar↔dispatch: `band()` の両経路で一致(既存基準 1e-10)。
- [ ] al-analysis `cargo test` 全 pass(band_gain 契約不変)。
- [ ] メモリ: thread-local counting allocator(先例
      `al-math/tests/ncc_workspace.rs`)で帯域ループ中の live bytes
      が `2*len` 帯域 + スペクトル/スクラッチ程度に収まることを検証
      (26 帯域分の同時生存がないこと)。
- [ ] 実サンプル run で `analysis::reference` busy と RSS を計測記録。
