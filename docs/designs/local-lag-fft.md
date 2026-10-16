---
id: "DESIGN-034GNsEgwjRPsPqegmSEWU"
type: "design"
status: "approved"
title: "local_lag FFT 化の設計"
created: "2026-10-16"
updated: "2026-10-16"
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
    - "docs/specs/local-lag-overlap-corr-fft.md"
    - "docs/adr/lag-al-math-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/local-lag-overlap-corr-fft.md"
    - "docs/adr/lag-al-math-plan.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---
# local_lag FFT 化の設計

## Context

`al-analysis::reference::local_lag` は等長 L の窓 `x`,`y` に対し
`lag ∈ [−max_lag, +max_lag]` の各点で `corr0`(overlap 部分の
非中心化正規化相関)を個別に呼ぶ。1 窓 ~8.4M MAC × ~585 窓
(50 s/48 kHz)が reference_analysis の支配コスト。

既存 `al_math::correlation::CorrelationPlan` は片側スライドの
固定長正規化 NCC であり、本要件(両側 lag・overlap 正規化)とは
別の演算契約。新カーネル `OverlapCorrPlan`/`OverlapCorrWorkspace`
を `al_math::correlation` に追加する(ADR-034GNsM3Pltk8Ax6HdXuHe)。

## Scope

### In Scope

- `OverlapCorrPlan`/`OverlapCorrWorkspace` + owned-curve 入口
- `local_lag` を新カーネル経由に切替え、plan を窓ループ外で再利用
- scalar↔dispatch 同等性テスト、corr_at 逐点比較テスト
- 小窓 dispatch 閾値の実測判定

### Out of Scope

- 推定量・出力契約の変更、`lag_window`/`lag_max` 既定値の変更
- locate 側相関パス・CorrelationPool への変更
- 既存 `CorrelationPlan` の API 変更

## Components and Boundaries

- `al_math::correlation::OverlapCorrPlan` — 窓長 L を shape キーに
  持つ不変 plan。内部に `RealFftPlan`(N = next_pow2(2L))を保持
- `al_math::correlation::OverlapCorrWorkspace` — FFT workspace +
  実数バッファ + prefix 二乗和バッファ + 出力曲線バッファ
  (長さ 2·max_lag+1 分)を全確保する排他領域
- `OverlapCorrPlan::curve(&self, ws, x, y, max_lag) -> &[f64]` —
  借用曲線を返す。`curve[i]` は `lag = i − max_lag` の値。
  内部は `dispatch`/`WithSimd` で SIMD カーネルに委譲
- `al_locate::dsp` — `OverlapCorrPlan`/`OverlapCorrWorkspace` を
  re-export する facade(既存の CorrelationPlan re-export と同型)
- `al_analysis::reference::local_lag` — plan+workspace を受け取り
  曲線から `(best_lag, best_score, half_width)` を算出
  (現行と同一のロジック)

## Data and Control Flow

1. `analyze` が窓ループ前に `OverlapCorrPlan::new(lag_window)` と
   workspace を1度構築
2. 各窓で `plan.curve(ws, xa, ya, lag_max)` を呼ぶ:
   - `real` バッファに `x`(先頭 L、残り 0 埋め)→ forward FFT
   - 同様に `y` → forward FFT
   - cross spectrum `conj(X)·Y` を pointwise に計算
   - inverse FFT → `real[k]` = c[k] = Σx[i]·y[i+k](N ≥ 2L−1 で
     線形同値)
   - prefix 二乗和 Px, Py を構築し、lag ごとに
     `ex = lag≥0 ? Px[L−lag] : Px[L]−Px[|lag|]`、
     `ey = lag≥0 ? Py[L]−Py[lag] : Py[L−|lag|]` を得る
   - `|lag| ≥ L` または `ex≤0 || ey≤0` → 0.0、それ以外
     `real[k] / √(ex·ey)` を `out` に書く
3. `local_lag` は借用曲線を走査して best lag/score/半値幅を算出
   (現行ロジックを曲線参照に置き換えるだけ)

## Non-Functional Constraints

- 数値契約: 現行 `corr_at` の逐点再現(丸めの和順序差を除く)。
  peak lag は一致、スコアは相対誤差閾値内(#59 tie 処理に
  近接入替を委譲)
- 性能: 1 窓あたり FFT N=16384 が 3 変換 + O(L)。plan は
  `local_lag` 呼出全体で1度だけ構築
- 借用契約: 返却曲線は ws を借用し、保持したまま ws 再利用不可
- `lag_window = 0` または `2L−1` の 2 冪化が usize overflow する
  形状は constructor で panic/空 shape(既存 plan と同型)

## Risks and Trade-offs

- **近接 peak の argmax 入替**: 分子の和順序が変わるため ulp 差で
  ピーク順位が入れ替わり得る → spec の一致基準(peak 一致+相対
  誤差)と #59 の tie 検出で吸収。実測で入替が golden を壊す
  場合はスコア tolerance を調整
- **prefix 和の桁落ち**: f64 で L≤数M まで単調非負の累積は安全。
  極端な大入力でも overlap 区間は窓の大部分を占めるため実害なし
- **小窓でのオーバーヘッド**: N=2L の FFT が直接和に負ける
  交差があるか実測し、あれば閾値 dispatch を導入する

## Deferred Design Concerns

- 窓 50% 重なり部分の FFT 共有 / 部分 lag のみの変換 | 本変更で
  支配コストは解消見込み | 実測で依然支配的な場合 | 複雑性増

## Verification Notes

- [ ] 合成信号で `curve(lag)` ≡ `corr_at(lag)` を全 lag で比較
- [ ] dispatch vs `pulp::Scalar` の逐点偏差 ≤ ~1e-12
- [ ] `local_lag` 出力が既存 `ref.lag_*` テストで不変
- [ ] `bench` で local_lag 部分の改善と小窓交差を計測
