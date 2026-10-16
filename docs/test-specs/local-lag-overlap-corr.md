---
id: "TSPEC-034GNwf4Cw9BJVBt8akOXm"
type: "test-spec"
status: "draft"
title: "OverlapCorr/local_lag 検証と小窓計測"
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
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/local-lag-overlap-corr-fft.md"
  derived-by: []
  verifies:
    - "docs/designs/local-lag-fft.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# OverlapCorr/local_lag 検証と小窓計測

## Purpose

`OverlapCorrPlan` が現行 `corr_at` と同一の推定量を返すこと、
および `local_lag` の公開契約 `(lag, score, width)` が不変で
あることを保証する。`local_lag` の意味を変える変更
(中心化、固定長正規化への置換など)が入ったときに検出するための
spec であり、別推定量が正当化されたときのみ retire する。

## Feature

等長窓の両側 lag 相関曲線: lag ∈ [−max_lag, +max_lag] の各点で
overlap 長 L−|lag| の非中心化正規化相関を返し、best lag・
score・半値幅を導出する。

## Rules

- Rule A: 曲線値は `corr_at` の逐点再現 — 各 lag で
  `|new − old|` が相対誤差閾値(1e-9)以内、かつ peak lag が一致する
- Rule B: ガード再現 — overlap エネルギーが 0 の lag、および
  `|lag| ≥ L` の lag は 0.0 を返す
- Rule C: 借用契約 — `curve` の返却スライスは workspace を借用し、
  保持したまま workspace を再利用できない(compile-fail 契約は
  CorrelationPlan と同型)
- Rule D: dispatch 同等性 — `pulp::Scalar` で vectorize した結果と
  runtime-dispatch 結果の逐点偏差が 1e-12 未満
- Rule E: `local_lag` の出力 `(best_lag, best_score, half_width)`
  は曲線スキャンのみから算出し、既存呼出と同じ解釈を持つ

## Examples

- 既知オフセットを埋め込んだ合成窓(x = noise、y = x を k サンプル
  シフト)で、new curve の argmax が `corr_at` 走査の argmax と
  一致し、全 lag のスコア差 ≤ 1e-9 相対
- 無音窓(全 0)で全 lag が 0.0
- `max_lag ≥ L` を指定した場合、`|lag| ≥ L` の要素が 0.0 で
  panic しない
- `local_lag` を通じた `ref.lag_*` が既存テストの期待値を維持

## Guarantees

- FFT 化後も `ref.lag_median`/`lag_mad`/`p05`/`p95` の意味と
  値域は不変(許容誤差内)
- peak lag が既存出力と一致する(fixture 単位)
- 50 s/48 kHz 級の入力で local_lag 部分の計測時間が低下する

## Non-goals

- `lag` 推定の精度向上や推定量の改善を検証しない(意味不変が前提)
- locate 側(NCC/PHAT)の出力契約は別 spec の範囲
- 半値幅が丸め差で ±1 変わる端点ケースは「peak 一致+スコア
  許容」の範囲とし、bit 一致は要求しない

## Risk

- これが落ちた場合: FFT 化の取り違え(窓外データの混入、
  wraparound、負 lag の符号反転)で `ref.lag_*` が silently
  変わる。detect 契約を守るため最小限の逐点比較は必須
