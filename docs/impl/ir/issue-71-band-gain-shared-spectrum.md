---
id: "IMPL-byqx6HbPHxQ7pH8caAYCG"
type: "impl"
status: "completed"
title: "Issue #71 band_gain 共有スペクトル化 (ruviform #183 BandSpectrum/BandScratch 適用)"
created: "2026-11-02"
updated: "2026-11-02"
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
  implements: []
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-71-band-gain-pair-al-analysis.md"
    - "docs/impl/ir/ruviform-rev-bump-consumer-regression-verification-issue-127.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/perf-reeval-2026-10.md"
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
    adopted:
      - "入力ごと 1 回の forward FFT を保持する BandSpectrum を band タスク群で共有 + worker-local BandScratch で帯域抽出 (ruviform #183)"
    rejected:
      - "BandFilter 共有 (band_into が &mut self で直列化する。旧 API)"
      - "タスクごと bandpass 再計算の維持 (forward ~100 回/pair の冗長。本件で解消)"
---

# Issue #71 band_gain 共有スペクトル化 (ruviform #183 BandSpectrum/BandScratch 適用)

## Summary

前回の pair 内並列化(`issue-71-band-gain-pair-al-analysis`)で
(channel, band) タスクが `bandpass` を自前で呼ぶ構造になり、
帯域波形生成のために全長入力の forward FFT がタスクあたり 2 回
(= pair あたり ~104 回) 再計算されていた。ruviform #183 で
導入された `BandSpectrum`(不変・Send+Sync の共有 forward FFT
ホルダ)と `BandScratch`(呼出側排他の抽出スクラッチ)を適用し、
入力ごと 1 回の forward FFT を全 band タスクで共有する構造へ
置き換えた。`band_gain` span busy は canonical で
65.4s/2call → 33.7s/2call(~1.94×)を計測。9 ラン golden 比較は
数値 diff 0 件(唯一の差分は pin bump 由来の ruviform rev
provenance 埋め込み)を確認した。

## Context

- 前回記録の「見積もりとの差異」節で、spec 見積もり ~4-7s/pair に
  対し実測 ~14.3s/pair だった主因として、タスク内で再計算する
  forward FFT(~4.19M 点 × ~100 回/pair)と FFT 重タスク同時実行の
  メモリ帯域競合を特定済み。解消は ruviform #183(共有スペクトル +
  分離スクラッチ API)がフォローアップとして記録されていた。
- ruviform #183 は `BandSpectrum::new(input, rate)`(1 回の
  forward FFT を保持、不変・Send+Sync)と
  `spectrum.band_into(&mut BandScratch, lo, hi, out)`(排他
  スクラッチを使う帯域抽出)を提供。`BandScratch` は変換長が同じ
  スペクトル間で使い回せる。`BandFilter::band_into`(旧 API)との
  アルゴリズム差はなく、同じ bin 写像・同一 plan での逆 FFT・
  同じコピーバック規則を共有する(bit-exact)。
- 旧 pin `7c430212` には当該 API が存在しないため、ruviform pin を
  現 origin/main `9d267fdf7aa2b3f1cd250672f32b54813ada8e1d` へ
  bump した(全 ruviform 依存で同一 rev。lock 再解決で 15 箇所の
  source が単一 rev に揃うことを確認)。
- locate 側 `band_ncc` は `InputCache::prepare_with_bands` が
  `bandpass_many`(入力ごと 1 回の forward で全帯域抽出)を既に
  使っており、冗長 forward は残っていない。残対象は reference 側
  `band_gain` のみで、本件はそこに限定した。

## Implementation

### Changes

**`Cargo.toml` / `Cargo.lock`**

- ruviform git 依存の rev を `7c430212...` →
  `9d267fdf7aa2b3f1cd250672f32b54813ada8e1d`(origin/main 先頭)
  へ全箇所一括更新。lock 再解決で ruviform source 15 箇所が単一
  rev に揃うことを確認。consumer 側の API 差分は additive のみ
  (BandSpectrum/BandScratch 追加等)。`ruviform-math` の clippy
  attribute 調整は AudioLens の消費 API に影響なし。

**`crates/al-analysis/src/band_gain.rs`**

- `analyze` 冒頭で `inputs = src[..n_chan] ++ cap[..n_chan]` の各
  入力に対し `BandSpectrum::new` を `run_pool` で並列構築(入力
  ごと 1 回の forward FFT。従来は band タスク内で `bandpass` が
  毎回行っていた)。`split_at(n_chan)` で `src_spectra` /
  `cap_spectra` に分け、タスク側は `&BandSpectrum` を共有参照する。
- `Ncc` ワーカーコンテキストに `scratch: Option<(usize,
  BandScratch)>` を追加。`Ncc::band` は最初の抽出時にスペクトルの
  変換長で `BandScratch` を構築し、以後は同変換長の限り全
  スペクトルで使い回す(src/cap で fft_size が違う場合のみ再構築)。
- band タスクは `ncc.band(src_spectra[ch], lo, hi, &mut x)` /
  `ncc.band(cap_spectra[ch], lo, hi, &mut y)` で帯域波形を生成し、
  後続の `windows_with`(guard・corr0・探索端除外・polarity 等)
  は一切変更しない。タスク列構成(先頭 coherence + channel-major
  band)と ordered slot 収集も不変で、結果の収集順・浮動小数点
  演算順は逐次版と同一。
- 回帰テスト `shared_spectrum_band_extraction_is_bit_exact_with_
  bandpass` を追加: 1 つの `Ncc` scratch で 2 入力・複数帯域に
  抽出した帯域波形が `bandpass`(従来の帯域ごと forward FFT 経路)
  と全要素一致することを直接固定する。

### Decisions During Implementation

- **forward FFT を band タスクの外側へ**: `BandSpectrum::new` は
  入力数(2ch×2=4 個)だけで済み、band タスク(52 個)は抽出のみ
  を行う。スペクトル構築も `run_pool` で並列化し、逐次区間を残さ
  ない。`BandSpectrum` は不変・Send+Sync なので全 band タスクが
  `&` で共有できる。
- **scratch は worker ctx へ**: `BandScratch` は `&mut` が必要な
  排他領域。`run_pool` の worker ctx(ワーカー起動時 1 回構築)に
  保持させることで、タスクごとの再確保を避けつつ Send 境界を
  満たす。src/cap で変換長が異なる入力(44.1kHz 不等長など)にも
  対応するため、保持キーは変換長とした。
- **bandpass_many ではなく BandSpectrum**: locate 側の
  `bandpass_many` は「1 forward → 全帯域抽出」を 1 入力に対し
  逐次行う。band_gain は (channel, band) を並列タスクにするため、
  抽出をタスクごと独立に行える `band_into` が必要で、これが
  ruviform #183 の提供するちょうど良い境界だった。

## Related Experiments

### Adopted

- `BandSpectrum` + per-worker `BandScratch` による共有 forward FFT
  構造。`band_into` の抽出規則は旧 `BandFilter::band_into` と
  アルゴリズム同一で、回帰テストで `bandpass` との全要素一致
  (bit-exact)を固定した。

### Rejected

- **`BandFilter` 共有**(旧 API): `band_into` が `&mut self` のため
  共有は直列化。ruviform #183 で解消済みのため再検討不要。
- **band タスク内 `bandpass` 維持**: forward ~104 回/pair の冗長が
  残り、本件の解消対象そのもの。

## Validation

- [x] `cargo check --workspace --all-targets` / `cargo build
  --workspace` / `cargo test --workspace`(46 バイナリ全 pass、
  `band_gain` テスト 5 件含む)/ `cargo clippy --workspace -- -D
  warnings` / `cargo fmt --check` 全 pass(新 pin 対応込み)
- [x] `mise run fmt` / `check` / `lint` / `test` / `build` /
  `verify-toolchain` / `check-cli-headless` 全 pass
  (`crates/al-gui/assets/app.js` の Biome `useTemplate` 指摘は
  warning として残るが未変更ファイルの既存事項。新規証跡 JSON
  4 件の末尾改行を `biome check --write` で修正)
- [x] 回帰テスト:
  `shared_spectrum_band_extraction_is_bit_exact_with_bandpass`
  (BandSpectrum 抽出波形 = `bandpass` 出力の全要素一致、
  複数帯域・複数スペクトルで scratch 使い回し込み) +
  既存 `independent_band_delays_and_channel_polarity_are_recovered`
  / `top_half_is_selected_per_polarity_with_time_ties_and_
  true_median` / `silence_short_input_and_search_edge_are_explicit`
  / `unrelated_loud_capture_never_becomes_supported_frequency_
  response` / `a7_band_gain_m2_coherence_params_are_bit_exact` /
  `a16a_band_gain_psd_ratio_is_bit_exact` 全 pass
- [x] 9 ラン golden 比較: 同一ブランチ pre/post(変更前 binary で
  `baseline/runs/<name>-prespec` を capture し、変更後と
  `compare.mjs` 閾値 rel=1e-9/abs=1e-12 で比較)。**全 run で数値・
  構造・key diff 0 件**。唯一の差分は pin bump で変わる ruviform rev
  の provenance 埋め込み(`params.ruviform.rev` /
  `params.decoder.version` 系の `a=7c430212` → `b=9d267fd` value
  diff。これは pin bump の証跡として意図された記録で、解析結果の
  契約面ではない)
  - canonical 998 / variant-desync 786 / variant-clipping 965 /
    variant-reverb 968 / variant-filter 989 / variant-clicks 995 /
    small 1205 / resume 998 / aa 998 件、全件 provenance rev のみ
  - resume vs canonical(post)、aa vs canonical(post): それぞれ
    **0 diff**(resume の中断・再開経路と aa の決定性も不変)
  - peak RSS(1s ポーリング下限推定): canonical 6.36GB → 6.85GB、
    variant-desync 4.68GB → 4.35GB、small 7.04GB → 7.23GB。
    増減交じりで系統的増加なし(共有スペクトルは入力 4 本あわせて
    ~130MB 級で、locate 支配の e2e ピークに埋没)
- [x] 性能計測(`tools/perf-reeval/capture.mjs`、同一ツール同一
  手順で pre/post を取得。post は
  `perf-reeval-2026-10/{canonical,small}-postspec.json`)

  | run | 指標 | pre(prespec) | post(postspec) | 変化 |
  | --- | --- | --- | --- | --- |
  | canonical | e2e wall | 198.3s | 175.0s | −11.8% |
  | canonical | reference_analysis stage | 47.5s | 31.7s | −33.3% |
  | canonical | `analysis::reference` span busy | 89.1s/2call | 57.3s/2call | −35.7% |
  | canonical | `analysis::band_gain` span busy | 65.4s/2call | 33.7s/2call | **−48.4%** |
  | small | e2e wall | 109.6s | 107.7s | −1.7% |
  | small | reference_analysis stage | 2.79s | 2.26s | −19.0% |
  | small | `analysis::reference` span busy | 3.64s/4call | 2.85s/4call | −21.7% |
  | small | `analysis::band_gain` span busy | 1.62s/3call | 1.03s/3call | **−36.4%** |

  band_gain busy がほぼ半減したのは、帯域波形生成の forward FFT
  が「(channel, band) タスクごと 2 回 ≈ 104 回/pair」から
  「入力ごと 1 回 = 4 回/pair」に減った分。残る ~16.9s/pair は
  窓 NCC ループ本体(25,917 windows/ch)で、本件のスコープ外。
  e2e の改善(~23s)は reference stage の短縮(~15.8s)と一致
  する(残差は locate 側の変動)。環境は同一ホスト同一手順で
  pre/post 自己比較。

## Risks

- `BandSpectrum` は forward FFT 結果を入力ごと保持する(入力 1 本
  あたり複素数 ~2.1M 点 + plan、4 入力で ~130MB 級)。ピーク
  メモリは帯域波形全保持(旧 P2 で排除した 1.8GB 級)よりずっと
  小さい。golden capture の peak RSS は増減交じりで系統的増加なし。
- `Ncc::band` の scratch キーは変換長のみ。同一変換長で異なる
  入力・帯域への使い回しは upstream 契約(`BandScratch` は
  変換長のみを検証する排他領域)として正当。

## Follow-ups

- 窓 NCC ループ本体(25,917 windows/ch、band_gain 残 ~16.9s/pair
  の支配項)の構造的最適化(window-NCC バッチ化等)は別 Issue。
  band_ncc/full_ncc の P3 適応 dispatch も継続検討。
- locate stage(canonical ~102s)が依然 e2e の支配項。#54 stage 内
  並列化と合わせて残課題。
