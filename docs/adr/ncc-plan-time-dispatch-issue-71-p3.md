---
id: "ADR-034HiHXhFVGpbv13OYyDf1"
type: "adr"
status: "accepted"
title: "直接法 NCC の plan-time dispatch(issue #71 P3)"
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
    - "./bandfilter-inputcache.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/adaptive-ncc-dispatch-issue-71-p3.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../research/issue-71-band-correlation/algorithm-research-report.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# 直接法 NCC の plan-time dispatch(issue #71 P3)

## Context and Problem Statement

#71 P1 で NCC 専用 Valid FFT plan(`nextpow2(S)`)を導入したが、
`band_ncc`/`full_ncc`/`coarse_ncc` が要求するのは有効ラグ
`K = S − L` のみで、実ジョブでは K≈236 と極小。分子
`num[k] = Σ_i a[i]·b[k+i]` を直接計算すれば K·L の MAC で済み、
小 K では 3 回の大 FFT(n≈4.19M)より桁違いに安い。

一方、`band_gain` の窓 NCC(L=4410, K=3528)や anchor 探索の
narrow 形状では Valid FFT が依然速い(ベンチ: FFT 165µs vs
direct 2.85ms)。**形状によって最適経路が入れ替わる**ため、
dispatch が必要。

## Decision Drivers

* 全 NCC 呼出が `max_lag = S − L` で行われ、K は plan 構築時の
  (L, S) から一意に決まる — 呼出側の変更なしに plan-time で
  方式を選べる
* 数値契約(分子 → prefix-sum 窓エネルギー → `normalize`)を
  両経路で同一に保てること(誤差 ~1e-15 を実測済み)
* `CorrelationPool` や `dsp::ncc` 等の既存呼出を**無変更**で
  恩恵を受けられること(選択は plan 内部の責務)
* direct plan は FFT plan/バッファを持たず、pool が保持する
  workspace メモリも削減できること
* 誤選択しても正しさは変わらず性能のみ — 閾値はベンチ校正値で
  十分(SAFETY ではなく性能判断)

## Considered Options

* **A. plan-time dispatch(採用)**: `new_ncc` が
  `L·(K+1) ≤ C·n·log2(n)` のコストモデルで `PlanMode::NccDirect`
  を選択。`Correlation::run` が mode で分岐し、分子だけ直接計算、
  正規化は共通コード。workspace は direct plan で FFT バッファを
  確保しない。
* B. 呼出時 dispatch: `Correlation::run` 内で max_lag を見て分岐。
  plan は常に FFT を持ち、direct 選択時に FFT バッファが
  無駄に確保される(pool 内で ~170MB/shape)。選択は実質 (L,S)
  一意なので呼出時判断の利点がない。
* C. 別の公開 API(`ncc_direct` 等)を呼出側が選ぶ: 全呼出側に
  形状判断が漏れ、pool 経由では効かない。責務の層違い。
* D. Blocked FFT(帯状タイル化): P1 後の実形状では
  `nextpow2(S)` plan が既に最小域であり、ベンチで効く隙間なし。
  不採用。

## Decision Outcome

Chosen option: "A. plan-time dispatch"。K は (L,S) 一意なので
plan 構築が自然な選択点であり、workspace の FFT バッファ省略と
併せてメモリも削減できる。数値契約は分子以降の共通パイプ
(cumulative → normalize)を両経路で共有して維持する。

### Consequences

* Good: band_ncc/full_ncc/coarse 形状で ~4〜8× の分子計算削減、
  pool の workspace メモリ削減、呼出側・下流契約は完全に不変。
* Bad: 2 つの NCC 実装を維持する。閾値は環境校正値で、別 CPU では
  誤分類し得る(性能のみ、正しさ不変)。
* Neutral: #76(narrow anchor 探索)には plan 選択経由で間接的に
  効く可能性があるが、anchor 構造自体は変えない。

### Confirmation

* `new_ncc` の mode 選択を単体テストで pin(6 代表形状)
* direct ⇔ Valid FFT の全有効ラグ一致テスト(argmax/tie 含む)
* scalar ↔ runtime-dispatch 同等(<1e-10)
* 実サンプル run で band_ncc busy を実測比較

## Implementation Plan

* Affected paths: `crates/al-math/src/correlation.rs`
  (`PlanMode::NccDirect`、`new_ncc` の選択、`Correlation::run` の
  direct 分岐、workspace 条件付き確保)。呼出側は無変更。
* Patterns to follow: P1 の plan/mode 分離、SIMD カーネルは
  `WithSimd` + `dispatch` 経由、prefix-sum+normalize は共通関数を
  再利用。
* Patterns to avoid: 呼出側での方式選択、public API 追加、
  FFT workspace の常時確保。
* Dependencies or configuration: なし。

## Verification

* [ ] 全 6 形状で mode 選択が実測最速側に一致する pin テスト
* [ ] direct⇔FFT 全有効ラグ一致・scalar↔dispatch <1e-10
* [ ] `cargo test -p al-math -p al-locate -p al-analysis` 全 pass
* [ ] `cargo clippy --workspace --all-targets` clean
* [ ] 実サンプル run で band_ncc/full_ncc busy 改善を記録

## Pros and Cons of the Options

### Option A(plan-time dispatch)

* Good: 選択が (L,S) 一意で plan が唯一の判断主体。workspace も
  形状に応じて縮小でき、pool は透過的に恩恵を受ける。
* Bad: PlanMode が 3 つになる。

### Option B(呼出時 dispatch)

* Good: plan 変更が最小。
* Bad: FFT バッファが direct 形状でも確保される(pool で ~170MB
  /shape の無駄)。選択は max_lag を見ても結局 (L,S) 一意。

### Option C(別 API)

* Good: 実装が単純。
* Bad: pool/stage/band_gain 等の全呼出側に形状判断が漏れ、
  責務の層を破る。pool 経由の経路が恩恵を受けられない。

### Option D(Blocked FFT)

* Good: 理論上の FFT 演算量削減余地。
* Bad: P1 で有効ラグ域 plan にした時点で実形状の FFT 長は
  最小域にあり、ベンチで逆転する隙間がない。複雑さに見合わない。

## More Information

- `../specs/adaptive-ncc-dispatch-issue-71-p3.md`(要件・ベンチ表)
- `../discovery/reference-analysis-analyze-pair-band-gain.md`
  (確定方針・P1/P2 実測)
- `../research/issue-71-band-correlation/algorithm-research-report.md`
