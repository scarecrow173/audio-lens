---
id: "ADR-034GNsM3Pltk8Ax6HdXuHe"
type: "adr"
status: "accepted"
title: "等長窓 lag 相関を al-math の専用 plan として提供"
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
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/locate-plan-run-pool.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/local-lag-overlap-corr-fft.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# 等長窓 lag 相関を al-math の専用 plan として提供

## Context and Problem Statement

`local_lag`(issue #53)の厳密再現 FFT 化には「等長窓・両側 lag・
overlap 正規化」の相関曲線カーネルが必要である。既存の
`al_math::correlation::CorrelationPlan` は「固定長 template が
signal 上を片側にスライドする NCC/PHAT」で、正規化の意味も lag
方向も異なるため、そのままでは `corr_at` の推定量を表せない。

カーネルの配置先と API 形状を決める必要がある。

## Decision Drivers

* SIMD 演算は `al-math` に集約し、pulp を内部に留める既存方針
* `al-analysis` は `al-locate::dsp` 経由で演算 primitive を使う
  既存の依存構造(dsp が al-math の facade)
* plan/workspace 分離と借用曲線返却の #52 確立パターン
* scalar 実装と runtime-dispatch 実装の数値同等性テスト基盤

## Considered Options

* `al_math::correlation` に `OverlapCorrPlan`/`OverlapCorrWorkspace`
  を追加し、`dsp` facade 経由で `al-analysis` が使う
* `CorrelationPlan` を拡張して両側・overlap 正規化モードを持たせる
* `al-analysis` 内部に FFT 実装を直接置く(al-math 依存を追加)

## Decision Outcome

Chosen option: "`OverlapCorrPlan` を `al_math::correlation` に追加",
because 等長窓・両側 lag・overlap 正規化は
`CorrelationPlan`(片側スライド・固定長正規化)とは別の演算契約
であり、同一型へのモード追加は「最大 lag = signal−template」の
不変式を曖昧にする。独立型は shape キー `window_len` のみで
invariant が単純で、既存型の契約を侵さない。al-analysis は既存
通り `al_locate::dsp` 経由で re-export を使う(直接の al-math
依存を増やさない)。

### Consequences

* Good, because plan/workspace/借用曲線の #52 パターンをそのまま
  踏襲し、`local_lag` は窓ループ外で plan を構築できる。
* Good, because `CorrelationPlan` の既存契約・テストに変更がない。
* Bad, because al-math に相関系の型が1つ増え、名目上 API 面が
  拡大する。

### Confirmation

* `OverlapCorrPlan::curve` の出力が `corr_at` の全 lag 走査と
  peak lag 一致・スコア許容誤差内であること
* `local_lag` が plan を窓ループ外で構築し再利用すること
* `cargo test --workspace` / clippy / fmt が clean

## Implementation Plan

* Affected paths: `crates/al-math/src/correlation.rs`(新規型)、
  `crates/al-locate/src/dsp.rs`(re-export)、
  `crates/al-analysis/src/reference.rs`(`local_lag` 呼出)
* Patterns to follow: `CorrelationPlan`/`CorrelationWorkspace`、
  `dispatch`/`WithSimd`、借用曲線返却、SIMD カーネルの
  `#[inline(always)]` + `pulp::Scalar` 対比テスト
* Patterns to avoid: `CorrelationPlan` へのモード追加、
  al-analysis からの al-math 直接依存、TLS/グローバル plan
* Dependencies or configuration: なし(新規依存なし)

## Verification

* [ ] OverlapCorr カーネルの corr_at 逐点比較テスト(peak 一致、
  スコア相対誤差 ≤ 1e-9 程度)
* [ ] scalar↔dispatch 偏差 ≤ ~1e-12(既存 correlation テストと
  同じ基準)
* [ ] `ref.lag_*` golden/既存テスト不変
* [ ] `cargo test --workspace`、clippy `-D warnings`、fmt

## Pros and Cons of the Options

### al-math に専用 plan 型を追加

* Good, because 演算契約が独立して明確で、shape キーが `L` 一つ。
  dispatch/SIMD 検証基盤をそのまま使える。
* Bad, because 型が1つ増える。

### CorrelationPlan を拡張

* Good, because 型は増えない。
* Bad, because 「片側スライド・固定長正規化」と「両側 lag・
  overlap 正規化」の混在で不変式が複雑化し、呼出側が意味を
  取り違えやすい。

### al-analysis 内部に直接実装

* Good, because crate 間の変更が最小。
* Bad, because SIMD 演算が al-math 外に漏れ、pulp/dispatch 基盤の
  外で実装することになる — 既存方針に反する。

## More Information

* `docs/discovery/issue-53-local-lag-fft.md`
* `docs/specs/local-lag-overlap-corr-fft.md`
* `docs/adr/locate-plan-run-pool.md`(#40 の plan プール方式)
