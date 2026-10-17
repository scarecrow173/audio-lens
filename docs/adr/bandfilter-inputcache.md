---
id: "ADR-034H775tTm4fDkJtwoGDwL"
type: "adr"
status: "proposed"
title: "BandFilter 型による帯域逐次生成と InputCache バイパス"
created: "2026-10-16"
updated: "2026-10-16"
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
  implements:
    - "../specs/streaming-bandpass-issue-71-p2.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./ncc-valid-fft-plan-gcc-phat-plan.md"
    - "./ncc-plan-time-dispatch-issue-71-p3.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/streaming-bandpass-issue-71-p2.md"
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

# BandFilter 型による帯域逐次生成と InputCache バイパス

## Context and Problem Statement

`band_gain::analyze` は `InputCache::prepare_with_bands` 経由で
26 帯域 × source/capture × 全チャンネルの帯域波形を同時保持する
(実ジョブで ≈1.8GB)。さらに `prepare_with_bands` は band_gain が
使わない `coarse` 間引き信号を入力ごとに計算する。

一方で `band_gain` の消費パターンは「帯域 i の src/cap 波形を
`windows()` に渡して捨てる」の繰り返しであり、全帯域の同時生存は
不要。また `InputCache` のキーは `(id, channel, rate, version,
filter)` で analyze 内では一意であり、hit が発生しない。

どこで逐次化し、filter 仕様の実装をどこに置くかが設計論点。

## Decision Drivers

* ピークメモリ削減(主目的)と unused `coarse` リサンプルの除去
* 帯域ごとの出力を `bandpass_many` と完全一致させる数値契約
* フィルタ仕様の実装を一箇所に集約する(表現ドリフト防止)
* locate 側の `InputCache` 契約を変えない
* FFT・SIMD の責務は al-math に置く既存境界の維持

## Considered Options

* **A: `BandFilter` 型(al-math)+ band_gain は InputCache を使わない**
  forward スペクトルを保持する plan 型。`band(lo, hi)` 呼出ごとに
  mask+IFFT+切り戻し。`bandpass_many` はこれへ委譲。
* **B: `PreparedChannel` 内で帯域を遅延生成する**
  `bands()` を iterator/lazy にし、元信号またはスペクトルを
  PreparedChannel が保持。`prepare_with_bands` 契約の拡張が必要。
* **C: `prepare_with_bands` に `need_coarse` 等のフラグを足すだけ**
  coarse 除去はできるが帯域同時保持は残る。

## Decision Outcome

Chosen option: "A: `BandFilter` 型 + InputCache バイパス", because
帯域逐次化の恩恵が最も大きいのは band_gain であり、その消費者は
cache 再利用の利益を持たない。`BandFilter` は forward FFT・mask・
IFFT という filter 仕様の責務を al-math 内に閉じ、`bandpass_many`
が同じ実装へ委譲するため数値一致が構造上保証される。B は
PreparedChannel の契約拡張(遅延評価・内部可変性・スペクトル保持)
を必要とし、C は主目的のメモリ削減を達成できない。

### Consequences

* Good, because band_gain の帯域同時生存が 26×2ch から現帯域 2
  信号に減り、ピークメモリが大幅に削減される。
* Good, because coarse リサンプルが band_gain 経路から完全に消える。
* Good, because `bandpass_many` の外部契約・出力は不変で、locate 側
  `InputCache` も無変更。
* Bad, because `band()` 呼出ごとに dispatch 経由の IFFT を行うため、
  `bandpass_many` の一括呼出より関数呼出境界が増える(IFFT 自体は
  同数、差は無視できる想定)。
* Bad, because 帯域波形が `Arc<[f64]>` 共有ではなく per-call `Vec`
  になるため、同一帯域を複数回使う呼出側では再 IFFT になる
  (band_gain は各帯域を 1 回だけ消費するため問題ない)。

### Confirmation

* `cargo test -p al-math` で `BandFilter::band` と `bandpass_many`
  の出力一致・scalar↔dispatch 同等を検証。
* `cargo test -p al-analysis` で band_gain 契約テスト全 pass。
* 実サンプル run で reference busy・ピークメモリを計測記録。

## Implementation Plan

* Affected paths: `crates/al-math/src/filter.rs`,
  `crates/al-analysis/src/band_gain.rs`,
  `crates/al-locate/src/dsp.rs`(re-export 更新が必要な場合)
* Patterns to follow: `CorrelationPlan`/`CorrelationWorkspace` と同様に
  「不変 plan + 呼出ごとの計算」の形。`dispatch`/`WithSimd` で
  SIMD 選択。
* Patterns to avoid: `PreparedChannel`/`InputCache` への遅延評価の
  持ち込み(option B)。`band_gain` 内での自前 FFT。
* Dependencies or configuration: なし。

## Verification

* [ ] `cargo test -p al-math -p al-analysis` 全 pass
* [ ] `cargo clippy --workspace --all-targets` clean
* [ ] 実サンプル run で `analysis::reference` busy と RSS を記録

## Pros and Cons of the Options

### Option A(採用)

* Good, because メモリ削減と coarse 除去を同時に達成し、filter
  仕様の単一実装を維持できる。
* Bad, because al-math の公開 API が 1 型増える。

### Option B

* Good, because `prepare_with_bands` の呼出形状を変えずに済む。
* Bad, because PreparedChannel にスペクトル(または元信号)の保持と
  内部可変性が必要になり、cache 共有の不変性が崩れる。locate 側の
  全消費者の契約に波及する。

### Option C

* Good, because 変更が最小。
* Bad, because 主目的の帯域同時保持 ≈1.8GB が残る。

## More Information

* 確定方針: `docs/discovery/reference-analysis-analyze-pair-band-gain.md`
  (P2 Streaming Bandpass)
* 調査: `docs/research/issue-71-band-correlation/algorithm-research-report.md`
