---
id: "SPEC-034HiHWQhvLOEFEnriCF0A"
type: "spec"
status: "draft"
title: "shape-adaptive NCC dispatch (issue #71 P3)"
created: "2026-10-17"
updated: "2026-10-17"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./valid-fft-ncc-plan-issue-71-p1.md"
    - "./streaming-bandpass-issue-71-p2.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/reference-analysis-analyze-pair-band-gain.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../research/issue-71-band-correlation/algorithm-research-report.md"
  defers: []
  deferred-by: []
---

# shape-adaptive NCC dispatch (issue #71 P3)

## What Should Be Built

`CorrelationPlan::new_ncc` が (template_len, signal_len) から導かれる
有効ラグ数 `K = signal_len − template_len` を用い、NCC の分子を

- **直接 SIMD 経路**(小 K:`acc[k] += a[i]·b[k+i]` の axpy 蓄積形)
- **Valid FFT 経路**(P1 の `nextpow2(S)` plan、大 K)

のいずれかへ **plan 構築時に**選択する shape-adaptive dispatch。
全 `pool.ncc`/`plan.ncc` 呼出は `max_lag = S − L` で呼ばれており、
(L, S) から K は一意に決まるため plan-time で完全に決定できる。

選択はベンチ実測で校正したコストモデルによる:
直接法 ≈ `L·(K+1)` MAC、FFT ≈ `n·log2(n)`(n = `nextpow2(S)`)。
現形状での採否測定(2026-10-17、release):

| 形状 | L | K | FFT | direct | 採用 |
| --- | --- | --- | --- | --- | --- |
| band_ncc/full_ncc (44.1k) | 2,205,000 | 236 | 228ms | 61ms | **direct (~3.7×)** |
| coarse_ncc (1/4 rate) | 551,250 | 59 | 48ms | 5.9ms | **direct (~8×)** |
| band_gain window | 4,410 | 3,528 | 165µs | 2,854µs | FFT |
| window_anchors narrow 小 | 4,410 | 8,820 | 336µs | 7,630µs | FFT |
| window_anchors narrow 中 | 4,410 | 61,740 | 3.6ms | 76.4ms | FFT |
| tiny (8kHz) | 800 | 638 | 38µs | 53µs | FFT |

直接法の出力は Valid FFT と**数値契約同一**(同一の窓エネルギー
prefix-sum・同一 `normalize` 規則・同一 early-zero・同一戻り範囲
`0..=max_lag`、誤差 ~1e-15)。直接法 plan は FFT バッファを
確保しない(NccDirect mode)ため workspace も縮小する。

## Why Now

- P1 で FFT 長半減後も NCC の残コストは band_ncc ~6.3s/呼出
  (26 帯域 × ~240ms)。K=236 と極小で、直接法は帯域あたり
  ~61ms — **band_ncc busy をさらに ~3.7× 削減**する余地がある。
- #76(anchor 探索)の narrow NCC でも、狭い範囲(K 小)では
  直接法が効き、さらに FFT plan 不要化で pool の plan 再構築
  コスト自体も軽減できる可能性がある(#76 側の実装は別 issue)。
- #71 の確定方針で P3 = 「適応 dispatch + Blocked FFT(実測採否)」
  と決定済み。Blocked FFT は形状ベンチの結果**不採用**(下記
  Out of Scope)。

## Users and Value

- AudioLens の解析パイプライン運用者: locate ステージの band_ncc /
  full_ncc / coarse_ncc busy 削減、および pool が抱える FFT
  workspace メモリの削減(direct plan は FFT バッファ不要)。
- 出力契約は不変のため、下流の証拠・テンソル・golden 検証は
  そのまま維持される。

## Scope

### In Scope

- `al-math::correlation` に直接法 NCC カーネル(axpy 蓄積形)を追加
- `CorrelationPlan::new_ncc` の plan-time dispatch(新
  `PlanMode::NccDirect`、コストモデル閾値はベンチ校正値を定数化)
- `CorrelationWorkspace` は direct plan で FFT バッファを確保しない
- pool(`CorrelationPool`)は無変更で自動的に恩恵(キー形状が
  そのまま直接法に落ちる)
- scalar↔runtime-dispatch 同等性・全有効ラグ一致の回帰テスト
- 形状ベンチ結果と閾値根拠の記録(discovery/task)

### Out of Scope

- **Blocked FFT(不採用)**: P1 後の実形状で、ブロック化が FFT を
  上回る隙間がベンチに存在しない(S≈L の band_ncc は直接法、
  小さい窓形状は Valid FFT が既に最適域)。複雑さに見合わない。
- `window_anchors_in` の構造最適化(#76)。本 Phase は pool.ncc に
  到達した時点の plan 選択のみ変え、探索構造は変えない。
- GCC-PHAT・overlap_corr・STFT/coherence 等他カーネル。
- 別推定器・新特徴量(#71 スコープ外の確定事項)。

## Requirements

- `PlanMode::NccDirect` plan の `ncc` は、同一入力に対して
  `PlanMode::NccValid` plan の `ncc` と全有効ラグで
  許容基準(<1e-10 系の既存 scalar↔dispatch 基準に準ずる)内一致
- `new_ncc` の mode 選択は (template_len, signal_len) のみに依存し、
  呼出時 max_lag によらない(全呼出が `max_lag = S−L` のため)
- `template_energy == 0` の early-zero、無音窓の 0 出力、
  `max_lag` clamp(`min(max_lag, S−L)`)、空 shape(n=0)の
  挙動を direct 経路でも同一に保つ
- direct plan の workspace は FFT 関連バッファを確保しない
- `gcc_phat` は direct plan でも panic(現行の NccValid と同じ契約)

## Acceptance Criteria

- [ ] 形状ベンチの全 6 形状で選択が実測最速側に一致する
- [ ] direct 経路の出力が Valid FFT plan と全有効ラグ一致
      (argmax・tie・順位を含む、誤差 ~1e-13 未満)
- [ ] scalar ↔ runtime-dispatch 同等
- [ ] band_ncc/full_ncc/coarse 形状で direct が選択され、
      band_gain 窓・narrow・tiny 形状で FFT が維持されることを
      テストで pin(mode 選択の単体テスト)
- [ ] 実サンプル run で `locate::band_ncc`/`full_ncc` busy の
      before/after を記録
- [ ] 全テスト・clippy・doc audit pass

## Deferred Design Concerns

- 閾値定数は現環境の校正値。別 CPU 特性での再校正は、モデルが
  実形状を誤分類した事例が出た時点で再検討する(誤分類しても
  正しさは変わらず性能のみ)。
- `window_anchors_in` で narrow NCC が direct に落ちる形状が
  存在する場合の効果測定は #76 の計装で行う。

## Sources and Relations

- `../discovery/reference-analysis-analyze-pair-band-gain.md`
  (確定方針の P3 定義、P1/P2 実測)
- `../research/issue-71-band-correlation/algorithm-research-report.md`
  (direct/Blocked/Valid の比較枠組み、SciPy choose_conv_method 先例)
- P1 spec `./valid-fft-ncc-plan-issue-71-p1.md`(plan/mode 分離の
  先行構造)、P2 spec `./streaming-bandpass-issue-71-p2.md`
