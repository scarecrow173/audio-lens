---
id: "DISC-034GNo8RqBQuP1n40LOrxO"
type: "discovery"
status: "draft"
title: "issue-53 local_lag FFT 厳密再現化"
created: "2026-10-16"
updated: "2026-10-16"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/53"
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
  related:
    - "docs/discovery/locate-consensus-argmax-ambiguity-contract.md"
    - "docs/discovery/fft-rebaseline-post-plan-workspace.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# issue-53 local_lag FFT 厳密再現化

## Exploration Goal

issue #53: `reference_analysis` の `local_lag` が 1 窓あたり
±`lag_max` の全 lag に `corr0` を個別適用し、50 s/48 kHz 入力で
~5G MACs/pair を消費している(reference_analysis 全体 61.5 s の
主要部)。確定した Phase 2 方針は **Option B: FFT 分子 + prefix
二乗和分母で現行 corr0 を厳密再現**する(2026-10-15 ロードマップ
確定決定)。本 discovery は厳密再現に必要な意味解析と実装形状を
確定する。

## Key Issues and Assumptions

### 現行の厳密な意味(crates/al-analysis/src/reference.rs)

`corr_at(x, y, lag)`(x, y は等長 L の窓):

- lag ≥ 0: `corr0(x[..L−lag], y[lag..L])` — overlap 長 n = L−lag
- lag < 0: `corr0(x[|lag|..L], y[..L−|lag|])` — overlap 長 n = L−|lag|
- n == 0(|lag| ≥ L)は corr0 を呼ばず 0.0 を返す
- `corr0` は `ex > 0 && ey > 0` のとき `sxy / √(ex·ey)`、それ以外 0.0
  (al-math `corr0_kernel`、非中心化)

`local_lag` は `lag ∈ [−lag_max, +lag_max]` の全 1025 点(既定
lag_max=512)を走査し、(best lag, best score, 半値幅) を返す。
窓は `lag_window=8192`/`lag_hop=4096` の 50% 重なり。

### FFT 再現の数学的同一性

- **分子**: overlap 和 Σx·y は長さ L の両窓の**線形相互相関**
  cxy[lag] と一致する。FFT 長 N ≥ 2L−1(実装上は next_pow2(2L))
  の相互相関で |lag| ≤ L−1 の全値を一度に得る。差分は浮動小数点
  丸めの和順序のみ(~1e-15 相対)
- **分母**: overlap のエネルギーは prefix 二乗和
  `P[i] = Σ_{k<i} x[k]²` で `ex(lag) = P[L−lag] − P[0]`、
  `ey(lag) = Q[L] − Q[lag]`(lag ≥ 0)。O(L) で全 lag の分母を
  一括構築できる。単調非負の累積で桁落ちリスクは実質なし
  (有効区間は |lag|=512 で窓の 94%)
- **ガード再現**: FFT 分子は丸めで微小な非ゼロになり得るため、
  `ex>0 && ey>0` のガードと |lag| ≥ L → 0.0 を明示的に再現する
  必要がある(分母が 0 のとき出力 0.0 は同じ)

### コスト構造

| | 現行 | FFT 化 |
| --- | --- | --- |
| 1 窓 | 1025 lag × ~8K MAC ≈ 8.4M MAC | FFT N=16384: ~0.5M 演算 + prefix O(L) |
| 50 s/pair(~585 窓) | ~5G MAC | ~0.3G 演算 |

FFT plan(`rustfft`)は窓長同一のため pair 内で再利用可能。
`al-math` の既存 FFT 基盤(#42 で NumPy 比 1.5-3.2x 高速確認済み)
を使う。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A: 既存 NCC 置換 | 実装小 | 分母定義(固定長正規化)・片側 lag が異なり別推定量になる。peak 選択・スコア絶対値・golden 全て再基準化が必要 | no |
| B: FFT 分子 + prefix 分母で corr0 厳密再現 | 意味不変(同一推定量)、~50-100x 見込み、検証は peak 一致+スコア許容誤差 | 実装中規模、丸め差で近接 peak の argmax 入替が理論上あり得る(#59 曖昧性処理に委譲) | yes |
| C: 直接和のまま SIMD のみ | 完全 bit 一致 | corr0 は既に SIMD 化済みで改善余地 ~4-8x に留まる | no |

確定決定(2026-10-15): **Option B**。peak lag 一致 + スコア相対
誤差の閾値評価、tie 近傍の argmax 入替は #59 の曖昧性処理に委譲。
小窓 dispatch は prototype と直接和を複数窓サイズで A/B 実測して
交差があれば閾値を設ける。

## Tentative Conclusions and Hypotheses

- **confirmed**: 分子は線形相互相関として厳密に FFT 計算できる。
  N = next_pow2(2L) で |lag| ≤ L−1 の全域をカバーする
- **confirmed**: 分母は prefix 二乗和で同一 overlap 範囲を厳密に
  再現できる(SIMD corr0 の和順序とは異なるが誤差は ulp 級)
- **confirmed**: 返却値契約 `(best_lag, best_score, half_width)` と
  `corr0` の 0 ガードは維持する。`local_lag` の公開挙動不変
- **hypothesis**: FFT 版は `lag_window=8192` で桁違いに速い。
  小窓(数百サンプル級)では直接和と交差する可能性があり、
  交差の有無と位置は実測で決める
- **hypothesis**: `lag_max < L−1` のとき FFT は全 lag を計算し
  必要範囲だけ読む — 分子は変わらないので問題なし。
  `lag_max ≥ L` は現行同様 |lag| ≥ L を 0.0 で埋める

## Open Questions

- 小窓閾値の有無と値 — prototype 計測で解決(実装フェーズ)

## Promotion Candidates

- [x] spec-doc needed: `local_lag` の FFT 再現契約(peak 一致基準・
  スコア許容誤差・ガード再現・小窓 dispatch 方針)
- [x] adr-doc needed: FFT 相互相関の採用(plan 再利用、N 選択、
  ガード再現の責務配置)

## Sources and Relations

- issue #53 / ロードマップ `docs/roadmap` Phase 2 確定決定
- `crates/al-analysis/src/reference.rs` `corr_at`/`local_lag`
- `crates/al-math/src/vec.rs` `corr0_kernel`
- `docs/discovery/locate-consensus-argmax-ambiguity-contract.md`
  (#59: argmax tie 契約)
- `docs/discovery/fft-rebaseline-post-plan-workspace.md`
  (#42/#52: FFT plan 再利用の実績)
