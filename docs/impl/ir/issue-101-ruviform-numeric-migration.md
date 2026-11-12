---
id: "IMPL-hL1u8vsDDTmk6RIHcU4cH4"
type: "impl"
status: "completed"
title: "Issue #101 al-math / al-locate::dsp 汎用 f64 カーネルの Ruviform 切替記録"
created: "2026-10-31"
updated: "2026-10-31"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/101"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/ruviform-dsp-kernel-switch.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
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

# Issue #101 al-math / al-locate::dsp 汎用 f64 カーネルの Ruviform 切替記録

- 対象 Issue: audio-lens #101
- 基準 baseline: Issue #98 成果物(`baseline/`)
- Ruviform rev: `f3f474b1734af05ca2435a322f935805010ba8b6`(Issue #150/#152/#153 全マージ後の main HEAD)
- 前提比較記録(ruviform 側): `issue-150-al-math-comparison-20261031.md` /
  `issue-152-al-math-comparison-20261031.md` / `issue-153-al-analysis-al-math-comparison-20261031.md`
- 本記録の差分出力: `docs/impl/ir/issue-101-baseline/`(`tools/baseline/compare.mjs` の 9 ラン分)

## Summary

task `docs/tasks/ruviform-dsp-kernel-switch.md` に従い、AudioLens 内の
汎用 f64 カーネル(al-math・al-locate::dsp・al-analysis の inline 実装)を
Ruviform `ruviform-numeric` の API へ切替えた。`al-math` クレートは全呼出の
置換完了後に削除し、`al-locate::dsp` は `deinterleave` と旧 panic 契約を
再現する薄い facade のみを残した。Issue #98 の凍結 golden との 9 ラン比較で
構造・イベント判定の差分はなく、残差は SIMD/累積順序由来の tolerance/value
差のみであった。

## Context

- task: `docs/tasks/ruviform-dsp-kernel-switch.md`(front matter `implements`)
- 基準 baseline: Issue #98 成果物(`baseline/` の凍結 golden)
- 前提となる ruviform 側比較記録: `issue-150-al-math-comparison-20261031.md` /
  `issue-152-al-math-comparison-20261031.md` /
  `issue-153-al-analysis-al-math-comparison-20261031.md`
- 責務境界の正本: #97 責務マップ(残留処理の判定根拠は
  Implementation > Remaining Responsibilities in AudioLens 節)

## Implementation

### Changes

| AudioLens 旧 API(al-math / al-locate::dsp / al-analysis) | Ruviform API | 検証根拠 |
| --- | --- | --- |
| `fft` free fn / `FftPlan` / `FftWorkspace` | `ruviform_numeric::fft::{fft, FftPlan, FftWorkspace}` | #150 比較(約 1e-13, SIMD ordering 差) |
| `hann_window` / `apply_window` | `Window::Hann`(`0.5 - 0.5*cos(2πn/N)`, 周期式)・plan 内適用 | 同式を直接確認 |
| `corr0` / `CorrelationPlan` / `NccWorkspace` / `ncc` | `corr::{corr0, CorrelationPlan, NccWorkspace}`(direct/FFT route, K≤2048, C=8) | #152 比較(argmax 完全一致) |
| `gcc_phat` / `overlap_corr_curve` / `curve` | `corr::{gcc_phat, overlap_corr_curve}` / `NccCurve.values` | #152 比較 |
| `BandFilter` / `bandpass` / `bandpass_many` | `bands::{BandFilter, bandpass, bandpass_many}` | #153 比較(bit-exact) |
| `resample` / `ResamplePlan` | `resample::ResamplePlan`(`polyphase-blackman-sinc` v1, provenance 値保持) | #150 比較(bit-exact) |
| `dot` / `sum_sq` / `scale` / `axpy` / `conv` | `vec::{dot, sum_sq, scale, axpy}` / `conv::conv` | #150 比較 |
| `LeastSquares` / `condition_number` | `linalg::{LeastSquares, condition_number}` | #150 比較 |
| `al_analysis::stft::{stft, power_spectrum, band_power, envelope, csd, complex_coherence, third_octave_bands}` | `stft::{StftPlan, StftWorkspace}` / `bands::{band_power, third_octave_bands, envelope}` / `xspec::{csd, complex_coherence}` | #153 比較(bit-exact) |
| al-analysis 内 inline Welch 累積(band_gain/cheap/reference) | `xspec::CrossSpectrum` + `xspec::transfer_h1` | #153 比較(bit-exact) |

`al-math` クレートは全呼出の置換完了後に削除。`al-locate::dsp` は AudioLens 固有の
`deinterleave` と、旧来の panic 契約(shape 不整合・非 2 冪長など)を `expect` で再現する
薄い facade のみを残す。複素スペクトルの公開型は tuple `Vec` から `(Vec<f64>, Vec<f64>)`
の split re/im レイアウトへ移行(中間の zip/unzip adapter を置かない)。

### Preserved Numeric Contracts

- FFT: forward=負号回転・無正規化、inverse=正号回転・1/N 正規化、非 2 冪は panic 契約維持
- Real FFT: 非負周波数 `N/2+1` bins
- NCC: direct/FFT ルーティング閾値 `K<=2048`・コスト比 `C=8` を Ruviform 側で一致確認
- Welch: `Σw²` による window power 正規化・cross-spectrum は和を保持し coherence/H1 で
  正規化が相殺される構造を維持。`reference.rs` では平均化→coherence の演算順序を
  変更していない(fp の丸め差が増えないよう `/frames` を `*(1/frames)` に置き換えない)
- ゼロエネルギー時は NaN ではなく 0 を返す既存契約を維持(`transfer_h1` 等)
- STFT: 完全窓のみ、`fft_size = next_power_of_two(window)`、`sample_rate` は
  記録のみ(0 でも計算には使わない)→ plan には `max(1)` を渡し挙動を維持
- conv: 空入力の早期 return セマンティクスが旧実装と同一であることを確認
- resample provenance: algorithm `polyphase-blackman-sinc` / version `1` を保持

### Remaining Responsibilities in AudioLens

- `al-locate::dsp::deinterleave`: チャネル分離は AudioLens の入力境界の責務
- `al-locate::dsp` の panic 契約 facade: 旧 API の shape 違反=panic という
  呼出側契約を維持する変換層(新規処理は持たない)
- `al-analysis::stft` の `StftMatrix`/`stft` 関数: Dataset 永続化形式と
  AudioLens 側の集約境界。内部カーネルのみ Ruviform
- locate の anchor 選択・skip/drift・validity/confidence・候補/consensus 解釈、
  解析 stage の意味論は本 Issue のスコープ外(#102)

## Related Experiments

### Adopted

- なし — 移行方式は task `docs/tasks/ruviform-dsp-kernel-switch.md` と
  ruviform 側比較記録(#150/#152/#153)で確定済みであり、方式選択のための
  実験的比較は行わなかった。関連する `docs/impl/exp/*.jsonl` の実験記録は
  存在しない。

### Rejected

- なし

## Validation

### Golden Comparison (tools/baseline)

9 ラン(`canonical`/`aa`/`resume`/`small`/`variant-*`×5)を新実装で capture し、
凍結 golden(`baseline/runs/<name>`)と `compare.mjs`(rel 1e-9 / abs 1e-12)で比較。
差分の実体は `issue-101-baseline/*.json|txt`。

- 差分 kind は `tolerance` / `value` のみ。`missing` / `added` / `type` / 行数差は 0 —
  構造・イベント判定・候補行の変更なし
- 実測最大値: 相対 ≈1.9e-8(`transfer_h_t` fingerprint `sum`, 2.7e8 規模の累積和で絶対差 5.1)。
  `values/*` 要素は最大でも相対 ~2e-7 以下・典型 ~1e-9。`candidates.rows` の `measured`
  は ~1e-17 相対の最終桁差
- 絶対 1e-12 閾値を超える残差系 fingerprint `sum`(~1e-12〜1e-14)は物理的に 0 の和であり、
  累積順序差による最終桁の符号反転
- `canonical` / `aa` / `resume` の 3 ランの差分ファイルは byte 一致 — 新実装は決定的
- 原因は scalar(ruviform-numeric)対 SIMD(pulp x86-v3 dispatch)の累積順序差であり、
  ruviform 側比較記録(#150 FFT 系 ~1e-13, #152 argmax 一致, #153 bit-exact)と整合

### Performance Comparison

`small` ラン(IN-#61/#62 capture + IN-#63/#64 source、Issue #71/#76 系サンプル)
の stage 時間:

| stage | 旧 ms | 新 ms | 差 |
| --- | --- | --- | --- |
| ingest | 600 | 605 | +0.8% |
| locate | 110678 | 107527 | **-2.8%** |
| cheap_analysis | 2369 | 2523 | +6.5% |
| reference_analysis | 2669 | 3330 | +24.8% |
| expensive_analysis | 1283 | 1521 | +18.6% |
| dataset | 158 | 166 | +5.1% |
| visualization | 1286 | 1294 | +0.6% |
| export | 5569 | 5563 | -0.1% |
| **合計** | **124612** | **122529** | **-1.7%** |

- 支配 stage の `locate`(全体の 89%)は plan 再利用経路の効果でむしろ 2.8% 改善
- `reference_analysis` +24.8%(約 +0.66s)の内訳: `band_gain`(3rd-octave band ×
  窓 NCC)が analyze_pair の約 380ms/pair、および `f0` 推定の `corr0` sweep が
  scalar dot/sum_sq へ置き換わった影響
- microbench(同一マシン): `analyze_pair` +33%, `stft`(1M samples)+25%,
  `prepare_residual` -25%。カーネル単位では `corr0` sweep 4.0x(絶対量 ~24ms/pair)、
  NCC plan +23%, overlap_corr +32%, bandpass_many +27%, gcc_phat +21%, conv +45%,
  BandFilter +33% — scalar vs SIMD の per-call コスト(ruviform 側比較記録の
  1.2–2.1x と整合)
- 結論: e2e 合計は -1.7% でネット中立。パイプライン上の性能退行は無いが、
  カーネル単位の退行は scalar 化に起因する既知のトレードオフ

#### Measurement Noise

負荷下(ビルド並走)で capture したランは stage 時間が数倍に膨らむ
(例: reference_analysis 23s)。上表は負荷なし再計測の値を採用した。

### Verification Results

- `cargo fmt --all`: clean
- `cargo test --workspace`: 全 pass(golden 19 件・al-locate/al-media/doc tests 含む)
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
  (`tools/cinematic-sword-cutscene` の Biome 警告は既存・無関係)
- `cargo build --release`(al bin): pass
- `mise run check-cli-headless`: pass(al-math を HEADLESS_CRATES から除去)
- `mise run verify-toolchain`: pass
- `Cargo.lock` から `al-math` / `pulp` が消え、`ruviform-numeric` が追加済み

## Risks

- scalar(ruviform-numeric)化に伴うカーネル単位の性能退行: `corr0` sweep は
  per-call ~4x、`analyze_pair` +33%、`reference_analysis` stage +24.8%
  (約 +0.66s)。e2e 合計は -1.7% でネット中立だが stage 単位の退行は残る
  (詳細は Validation > Performance Comparison)。
- 累積順序差による残差: golden 差分は tolerance/value のみだが、絶対
  1e-12 閾値を超える残差系 fingerprint `sum`(~1e-12〜1e-14)は物理的に
  0 の和の最終桁符号反転であり、bit 一致は期待できない。
- 性能計測は負荷なし条件の値。ビルド並走などの負荷下では stage 時間が
  数倍に膨らむため、計測値は条件依存である。

## Follow-ups

Ruviform 側への Issue 起票案(起票の実施は本記録の範囲外):

1. ruviform-numeric の scalar NCC/dot 系カーネル(corr0 sweep・窓 NCC 大量呼出)の
   SIMD/vectorize 対応 — AudioLens の reference_analysis で約 +25% の stage 退行が
   観測され、per-call 4x(corr0 sweep)~1.2x(plan NCC)が原因。f64 精度契約を
   維持したまま SIMD ordering を導入できるか検討
2. `xspec::CrossSpectrum` に band 区間限定の累積 API 追加の検討 — 現状は
   `band_power` 後段で帯域抽出しており、帯域限定 accumulate で呼出側の
   帯域ループを削れる可能性
