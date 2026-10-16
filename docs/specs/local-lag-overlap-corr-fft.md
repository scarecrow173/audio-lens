---
id: "SPEC-034GNsDtc88g3RPbhoW9RS"
type: "spec"
status: "draft"
title: "local_lag オーバーラップ正規化相関の FFT 化"
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
    - "docs/discovery/issue-53-local-lag-fft.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines:
    - "docs/discovery/issue-53-local-lag-fft.md"
  refined-by: []
  derives-from:
    - "docs/discovery/issue-53-local-lag-fft.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# local_lag オーバーラップ正規化相関の FFT 化

## What Should Be Built

`reference_analysis` の `local_lag` が行う ±`lag_max` 全 lag の
`corr0` 走査を、FFT 相互相関(分子)+ prefix 二乗和(分母)による
**同一推定量の高速計算**に置き換える。

等長 L の窓 `x`, `y` に対し、現行 `corr_at` と同一の曲線
`c[lag]`(lag ∈ [−lag_max, +lag_max]、overlap 長 L−|lag| で
正規化、`ex=0 || ey=0` で 0.0)を O(L log L) で返すカーネルを
`al-math` に追加し、`local_lag` は plan を窓ループ外で構築して
再利用する。

## Why Now

50 s/48 kHz 入力で `local_lag` は ~585 窓 × 1025 lag × ~8K MAC
≈ ~5G MACs/pair を消費し、reference_analysis(実測 61.5 s)の
主要部を占める。Phase 2 確定決定(2026-10-15)で Option B
(厳密再現 FFT 化)の採用が確定している。

## Users and Value

- AudioLens ユーザー: reference_analysis の所要時間を削減
- 保守担当: 推定量の意味は不変で、評価・golden 契約を維持したまま
  高速化できる

## Scope

### In Scope

- `al-math` に等長窓・両側 lag・overlap 正規化の相関曲線
  カーネル(plan + workspace + owned-curve 入口)を追加
- `local_lag` を新カーネル経由に切替え、FFT plan を窓ループ
  外で再利用
- peak lag 一致とスコア相対誤差の検証、実測による小窓 dispatch
  閾値の要否判定

### Out of Scope

- `local_lag` の推定量・出力契約(`(lag, score, width)`、
  `lag_median`/`lag_mad`/`p05`/`p95` への流れ)の変更
- locate 側の相関パス(NCC/PHAT、CorrelationPool)への変更
- corr0 自体の定義変更、中心化相関への切替
- `lag_window`/`lag_hop`/`lag_max` の既定値変更

## Requirements

- 曲線値は現行 `corr_at` の逐点再現: lag ≥ 0 は
  `corr0(x[..L−lag], y[lag..L])`、lag < 0 は
  `corr0(x[|lag|..L], y[..L−|lag|])` と等しい(浮動小数点
  丸めの和順序差を除く)
- `|lag| ≥ L` および `ex=0 || ey=0` の点は 0.0 を返す
- FFT 長 N は 2L−1 以上の 2 のべき乗で wraparound を防ぐ
- FFT plan/workspace は形状 `(L)` キーで再利用可能とし、
  `local_lag` 呼出ごとの再構築を避ける
- scalar 実装と runtime-dispatch 実装は数値同等
- 小窓閾値は推測で置かず、prototype の実測で交差が確認された
  場合のみ導入する

## Acceptance Criteria

- [ ] 新カーネルの出力曲線が現行 `corr_at` の全 lag 走査結果と
  peak lag 一致・スコア相対誤差が閾値内(合成・実信号 fixture)
- [ ] `local_lag` を通じた `ref.lag_*` 出力が既存テスト/golden で
  不変(許容誤差規約内)
- [ ] dispatch 版と scalar 版の偏差が既存 correlation と同程度
  (≤ ~1e-12)
- [ ] 50 s 級入力での実測改善を記録(local_lag 部分の時間)
- [ ] 小窓 dispatch の要否を実測で判定し、採用する場合は閾値を
  記録する
- [ ] `cargo test --workspace`、clippy、fmt が clean

## Deferred Design Concerns

- 窓ごとの FFT を更に削る方向(50% 重なり窓の共有、部分 lag
  範囲のみの変換) | 現行変更で既に支配的コストは解消される見込み
  | 実測で local_lag が依然支配的な場合 | 追加複雑性

## Sources and Relations

- `docs/discovery/issue-53-local-lag-fft.md`(意味解析と選択肢)
- issue #53、Phase 2 確定決定(2026-10-15)
- `docs/discovery/fft-rebaseline-post-plan-workspace.md`
  (#42/#52 の plan 再利用実績)
