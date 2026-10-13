---
id: "IMPL-034FoT1CalibrateWindowEps"
type: "impl"
status: "draft"
title: "locate argmax 同点検出: tie eps 特性計測(T1)"
created: "2026-10-13"
updated: "2026-10-13"
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
    - "docs/tasks/locate-argmax-tie-eps-characterization.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/locate-consensus-argmax-ambiguity.md"
    - "docs/specs/locate-consensus-ambiguity-contract.md"
    - "docs/test-specs/locate-argmax-tie-eps-calibration.md"
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

# locate argmax 同点検出: tie eps 特性計測(T1)

## Summary

plan T1(実装ゲート)の特性計測。本番コードは一切変更せず、
計測専用テストのみを追加した。実測の結果、
`TIE_REL_EPS`/`TIE_ABS_FLOOR` の実現可能窓が**存在する**ことを
確認し、以下に確定した:

- `TIE_REL_EPS = 1e-11`
- `TIE_ABS_FLOOR = 1e-11`

spec Req 6-7 のゲート `max(dispatch 偏差) << eps << min(マージン)`
に対し、下側 ~4 桁・上側 ~1.7e3 倍のクリアランスを持つ。
エスカレーション(spec Req 7)は**不要**。

spec の候補レンジ(`rel_eps` 相対 1e-9..1e-6、`abs_floor` ~1e-9)
からの下方乖離は実測に基づく意図的な決定: 受理系最小マージン
1.65e-8(`case_loop_material` × `spectral_envelope`)に対し
1e-9 は 16 倍しか離れず薄弱であり、逆に dispatch 偏差は
想定(~1e-13 級)より遥かに小さい(≤ ~5e-16、殆どビット一致)
ことが判明したため、窓の中央付近へ再較正した。

## Context

`docs/tasks/locate-argmax-tie-eps-characterization.md`
(TASK-034Fo9cS08WsbNrLoq6t6a)、design
`docs/designs/locate-consensus-argmax-ambiguity.md` §7、test-spec
`docs/test-specs/locate-argmax-tie-eps-calibration.md` Rule A-C
に従う。判定式は `T = { i : |corr[i]| >= M − tie_window }`、
`tie_window = max(TIE_REL_EPS·M, TIE_ABS_FLOOR)`、`|T| >= 2` で
曖昧(design §1)。

計測経路:

- `crates/al-locate/tests/argmax_tie_characterization.rs` 新設:
  fixture 毎に4メソッドの raw 曲線を production 同一式で生成し、
  `M`、`argmax`、第二順位大きさ、マージン、argmax±1 ショルダー
  デルタ、候補 eps(1e-9..1e-6 rel)×abs_floor 1e-9 と採用値での
  `|T|`、`|corr|` 上位を `--nocapture` で記録する。
  `band_ncc` は `InputCache::prepare().bands()`(`locate_prepared`
  と同一帯域信号)+ `evidence_from_bands` 同型の累積、
  `spectral_envelope` は `methods.rs` の現行逐次式(pre-T3)を
  逐語複製、`different_rates` は `ResamplePlan`(44100→48000)で
  source 正規化を再現
- `crates/al-math/src/correlation.rs` `mod tests`:
  `dispatch(Correlation)` vs `pulp::Scalar.vectorize(Correlation)`
  を phat false/true × n=65536/32768 の実運用 RealFFT サイズで
  A/B し最大偏差を記録
- `crates/al-math/src/filter.rs` `mod tests`: `Envelope` カーネルを
  実運用 fft_size(1024、len 2048/4096/8192 → 実効 n 同)で
  同様に A/B

## Measured: dispatch 偏差(Rule B)

`dispatch(kernel)` vs `pulp::Scalar.vectorize(kernel)` の最大
絶対偏差(本計測環境、AVX 系 dispatch):

| kernel | サイズ | phat | max \|a−b\| |
|--------|--------|------|----------------|
| Correlation | n=65536 (sig 40000 + tpl 4096) | false | 0(bitwise 一致) |
| Correlation | n=65536 | true | 0(bitwise 一致) |
| Correlation | n=32768 (sig 24576 + tpl 8192) | false | 3.47e-18 |
| Correlation | n=32768 | true | 1.74e-18 |
| Envelope | fft_n=2048/8192 | — | 0(bitwise 一致) |
| Envelope | fft_n=4096 | — | 4.44e-16 |

カーネルはレーン数に依存する reduction 順序を避ける設計
(`correlation.rs` コメント参照)のため、実運用サイズでも偏差は
実質 FP 丸め最小級。曲線レベルへの伝播を保守的に見ても
max ~1e-15 級。`band_ncc` の累積曲線は per-band `ncc` 偏差の和で
~1e-17 級、`spectral_envelope` の逐次 NCC は包絡 bin 偏差
(≤4.4e-16)を通じて ~1e-14 級までと見積もる。

**max(dispatch 偏差) ≈ 5e-16(実測)/ ~1e-14(伝播込み保守値)**。

## Measured: fixture マージン(Rule A)

`margin = M − max_{i≠argmax}|corr[i]|`(argmax のショルダー自身を
含む全サンプル第二順位)。括弧内は M(band_ncc は帯域和のため
~5 まで到達し得る)。全受理系 fixture は全メソッド
`valid-path=true`(既存3分岐を抜けて曖昧判定に到達する曲線)。

### 受理系(マージン下限の拘束側)

| fixture | full_ncc | band_ncc | gcc_phat | spectral_envelope |
|---------|----------|----------|----------|-------------------|
| pin-periodic-control | 9.34e-1 | 2.39 (M=4.99) | 1.00 | **1.07e-7** |
| case_exact_insert | 9.34e-1 | 7.49e-2 (M=0.727) | 9.93e-1 | **3.73e-7** |
| case_repeated_source | **1.75e-6** | 7.64e-4 (M=4.99) | 6.33e-4 | **4.37e-6** |
| case_loop_material | **5.52e-7** | 2.20e-2 (M=0.672) | 2.09e-3 | **1.65e-8** ← 全局最小 |
| case_long_source_linear_drift | 3.38e-4 | 1.42e-1 | 6.19e-3 | 4.41e-4 |
| case_periodic_source | 2.93e-1 | 1.79e-3 | 8.91e-1 | 3.61e-6 |
| polarity_inverted_source_is_still_located | 6.10e-1 | 1.68 (M=3.35) | 2.69e-1 | **6.70e-7** |
| repeated_source_reports_second_occurrence | 1.78e-2 | 3.04e-1 (M=3.65) | 8.21e-3 | 9.08e-6 |
| different_rates_locate_without_false_drift | 2.06e-1 | 1.68 (M=4.82) | 9.07e-1 | 1.05e-2 |

- **min(意味ある分離) = 1.65e-8**(`case_loop_material` ×
  `spectral_envelope`)。拘束の実形はショルダーデルタではなく、
  **ループ素材の正当な複数出現が準同点包絡を持つ近同点**
  (argmax=12000、第二順位=8000 — 別の真出現位置)。
- ショルダーデルタ曝露: `case_long_source_linear_drift` の
  full_ncc で +3.38e-4(第二順位が argmax−1 のショルダー)。
  推定式 `1−cos(2π·fc/fs)` の ~8.6e-3 より浅いが ~1e-4 級には
  達せず、採用 eps(1e-11)からは 4 桁以上の余裕。spectral_envelope
  の hop 疎化曲線では argmax 隣接は未評価 lag(0)のため
  ショルダー曝露は構造的に発生しない
- `spectral_envelope` の慢性曖昧: noise 系入力(ホワイトノイズ
  同士の `pinned_low_coherence` margin 2.75e-6、広帯域チャープ
  `different_rates` margin 1.05e-2、ノイズ埋込み
  `repeated_source_reports_second_occurrence` margin 9.08e-6)で
  **慢性曖昧は観測されなかった**(採用窓で |T|=1)。包絡 NCC は
  広く 0.99+ に張り付くが順位間差は 1e-6 級以上を保つ。ただし
  受理系最小マージン 1.65e-8 はこのメソッド由来であり、
  候補レンジ上端側(1e-7 以上)では `case_loop_material` や
  `polarity_inverted`(6.7e-7)が曖昧化する — eps 上限の実質
  拘束は spectral_envelope が担う

### 拒否系 / 混同リスク

| fixture | full_ncc | band_ncc | gcc_phat | spectral_envelope |
|---------|----------|----------|----------|-------------------|
| pin-periodic | margin **8.88e-16** → \|T\|=28(全て 1024 倍数) | margin 2.58e-4 → \|T\|=1 | margin 1.09e-1 → \|T\|=1 | margin **0.0** → \|T\|=219 |
| pinned_low_coherence_no_inflated_lag | 3.14e-3 → 1 | 2.71e-3 → 1 | 4.83e-3 → 1 | 2.75e-6 → 1 |
| pinned_silence_no_false_peak(両方向) | preflight 棄却のため同点判定へ到達しない(曲線なし) | — | — | — |
| prepared_exact_tie(+/-template) | margin **3.11e-15** → \|T\|=2 @ {0,6952} | 8.11e-6 → 1 | margin **7.99e-15** → \|T\|=2 | 6.98e-6 → 1 |
| prepared_periodic_sin f=0.002 r=512 | 5.92e-6 → 1 | 6.12e-2 → 1 | 7.81e-1 → 1 | 2.73e-5 → 1 |
| prepared_periodic_sin f=0.12 r=512 | 3.63e-6 → 1 | 6.12e-2 → 1 | 5.93e-1 → 1 | 5.08e-4 → 1 |
| prepared_periodic_sin r=513 | 上と同一構造(band_ncc のみ僅差) | | | |

- 周期 fixture の同点構造: `pin-periodic` の full_ncc は
  **分離ピークの厳密同点**(全て周期 1024 倍数、margin ~9e-16
  — 周期窓がサンプル完全一致するため)。plateau ではない。
  `spectral_envelope` は hop 評価 lag 219 箇所が厳密 1.0。
  `prepared_periodic_sin` は近同点の**分離ピーク**構造
  (上位 ~6 サンプルが 1e-6〜1e-5 内に密集、plateau ではない)
- **重要な実測事実(T2 への影響)**: `pin-periodic` で厳密同点
  となるのは `full_ncc` と `spectral_envelope` のみ。
  `band_ncc`(margin 2.58e-4)と `gcc_phat`(margin 1.09e-1)は
  採用窓どころか候補レンジ全域で同点とならない — 帯域フィルタ
  は有限区間のスペクトルマスク(ゼロ埋め円周畳み込み)でエッジ
  過渡が厳密周期を崩し、GCC-PHAT は全信号変換で lag 毎に
  スペクトル寄与が異なるため。plan Verification の
  「pin-periodic で全4メソッド merged 証拠が
  AmbiguousAlignment」という想定は計測と矛盾する: T2 の pin は
  「full_ncc + spectral_envelope が AmbiguousAlignment、
  band_ncc/gcc_phat は Valid で投票し合意不足」へ修正が必要。
  結果として `evaluate` は「投票非空かつ agreeing <
  min_agreeing_methods」分岐に落ち、不受理 + validity
  `ambiguous_alignment` + "only N method(s) agree" 文言の
  現行 pin は維持される見込み

## eps 確定

窓条件 `max(dispatch 偏差) << tie_window << min(受理系マージン)`:

- 下側: max dispatch 偏差 ~5e-16(実測)/ ~1e-14(保守値)
- 上側: min 受理系マージン 1.65e-8(M≈1 の spectral_envelope
  曲線で絶対値として拘束)

採用値:

```rust
const TIE_REL_EPS: f64 = 1e-11;
const TIE_ABS_FLOOR: f64 = 1e-11;
```

| 判定 | 値 | クリアランス |
|------|-----|--------------|
| tie_window(M≈1) vs dispatch 偏差 | 1e-11 vs ~5e-16 | ~2e4 倍(>4桁) |
| tie_window(M≈1) vs min 受理マージン | 1e-11 vs 1.65e-8 | ~1.65e3 倍(>3桁) |
| tie_window(M≈5, band_ncc) vs band min margin | 5e-11 vs 2.2e-2 | ~4e8 倍 |

- 厳密同点(fixture の margin ≤ ~1e-14)は採用窓で確実に捕捉
  される(pin-periodic |T|=28/219、prepared_exact_tie |T|=2 を
  実測確認)。受理系 fixture は全て |T|=1
- 判定反転の irreducible 不安定帯 `eps ± dispatch_deviation`:
  帯幅 ~1e-15 絶対(tie_window 境界 ± ~5e-16 内の |corr| のみが
  dispatch 間で所属を反転し得る)
- spec 候補レンジ(rel 1e-9..1e-6 / floor ~1e-9)からの乖離理由:
  レンジ下端 1e-9 は min マージン 1.65e-8 に 16 倍しか離れず、
  「<<」の要件に薄弱。dispatch 偏差が実質ゼロ級である実測により
  下方の余地が大きいため、両側の余裕を最大化する中央寄りの
  1e-11 を採用。abs_floor は M≈0 近傍の退化判定を同スケール
  1e-11 で定義する(それ以下の M を持つ曲線は `M − window < 0`
  で全サンプルが T に入り曖昧 — 弱スコアで投票しないため
  受理結果へ影響しない、design §2 記載の意味論)

## Verification

- `mise exec -- cargo test -p al-locate --test
  argmax_tie_characterization -- --nocapture`: 全 fixture の
  マージン表を出力し、受理系の |T|==1(かつ margin > 10×
  window の余裕 pin)、pin-periodic の |T|=28/1/1/219・同点位置
  の周期倍数、prepared_exact_tie の |T|=2@{0,6952}、
  prepared_periodic_sin / pinned_low_coherence の |T|=1 を
  アサートして green
- `mise exec -- cargo test -p al-math dispatch_deviation -- --
  nocapture`: 上記偏差表を出力し、偏差 < 1e-12(採用 eps の
  1/10 以下)をアサートして green

## Open items / 留意事項

- `best_window_match` の同点ガードが適用されるウィンドウ NCC
  曲線(anchor_window=512 の短窓)は T1 の計測対象外(タスク文の
  fixture 範囲外)。短窓曲線はより浅いマージンを持ち得るが、
  同点時の挙動は `continue`(寄与なし)であり受理系証拠の
  曖昧化ではない。design §5 の退化受諾と併せて T2 で観測する
- `spectral_envelope` の hop 疎化曲線の未評価 lag(0.0)は
  `M ≲ TIE_ABS_FLOOR` の曲線のみ T に入る — 弱スコア曲線では
  全サンプル同点となり得るが投票しないため無害(design §2)
