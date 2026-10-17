---
id: "SPEC-034H76gCpx4TVbIGDVGOPZ"
type: "spec"
status: "draft"
title: "Streaming Bandpass (issue #71 P2)"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./valid-fft-ncc-plan-issue-71-p1.md"
    - "./adaptive-ncc-dispatch-issue-71-p3.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/reference-analysis-analyze-pair-band-gain.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../research/issue-71-band-correlation/algorithm-research-report.md"
    - "../research/issue-71-band-correlation/oss-deep-dive.md"
  defers: []
  deferred-by: []
---

# Streaming Bandpass (issue #71 P2)

## What Should Be Built

`band_gain` が全帯域波形を同時保持する構造を、帯域ごとに
「mask → IFFT → 窓解析 → 破棄」する逐次生成へ変える。

- al-math に `BandFilter` 型を追加する。1 回の forward FFT で
  スペクトルを保持し、`band(&mut self, lo_hz, hi_hz)` 呼出ごとに
  mask + IFFT + 元長切り戻しを行い、その帯域波形だけを返す。
- 既存 `bandpass_many` は `BandFilter` へ委譲し、帯域ごとの出力を
  収集する形に変更する。フィルタ仕様の実装は `BandFilter` 一箇所に
  集約し、表現ドリフトを持ち込まない。
- `band_gain::analyze` は `InputCache::prepare_with_bands` を使わず、
  入力 × チャンネルごとに `BandFilter` を構築して帯域ループ内で
  `band()` → `windows()` → ドロップする。

## Why Now

- `prepare_with_bands` 経由では 26 帯域 × source/capture × 2ch の
  全帯域波形(≈1.8GB)が `analyze` 終了まで同時生存する。#54 の
  stage 並列化のメモリ予算と衝突し得る。
- `band_gain` は `prepare_with_bands` の `coarse` 間引き信号・
  `coarse_rate`・`radius` を一切使わないのに、入力ごとに
  リサンプルを計算している(推定 0.3-0.7s/入力)。
- `band_gain` の `InputCache` はキーが `(id, channel, rate,
  alignment, bands)` で一意であり、analyze 内で hit が発生しない。
  キャッシュ経由の利益がない。

## Users and Value

- AudioLens の運用者: 実サンプル run のピークメモリ削減と、
  reference_analysis の busy 削減(coarse 除去分)。

## Scope

### In Scope

- `al-math::filter` への `BandFilter` 追加と `bandpass_many` の委譲。
- `band_gain::analyze` の逐次生成化(`InputCache` 不使用)。
- band_gain 経路での `coarse` リサンプル除去(使わないため)。
- 実測によるメモリ・busy の検証。

### Out of Scope

- locate 側の `InputCache::prepare`/`prepare_with_bands`(coarse は
  coarse_ncc メソッドが使用。帯域数も 5 で小さい)。
- `windows()` の NCC plan 共有化・Blocked FFT(P3 の範囲)。
- `window_anchors_in`(issue #76)。
- coherence/STFT など他セクションの変更。

## Requirements

- `BandFilter::band(lo, hi)` の出力は同一入力に対する
  `bandpass_many` の対応帯域出力と完全一致する(同一 forward FFT、
  同一 mask 規則、同一 IFFT、同一切り戻し規則)。
- 空入力では `band()` が空ベクトルを返し、`bandpass_many` も
  帯域ごとの空ベクトルを従来通り返す。
- `hi_hz` が NaN の場合など既存の bin 判定規則(`lower_bin` の
  半開帯域・Nyquist bin 規則)をそのまま再利用する。
- `band_gain` の出力(record・tensor・帯域順・guard・400Hz anchor・
  polarity・delta/reference)は従来と一致する。
- scalar↔runtime-dispatch の同等性を維持する(`band()` も dispatch
  経由で SIMD 選択する)。
- `prepare_with_bands` の契約は変更しない(locate 側の消費者を
  壊さない)。

## Acceptance Criteria

- [ ] `BandFilter::band` の各帯域出力が `bandpass_many` の対応帯域と
      一致(既存の scalar↔dispatch テストと同じ基準)。
- [ ] `band_gain` 既存テスト全 pass(合成遅延・polarity・
      silence/search_edge・guard 等の契約不変)。
- [ ] 帯域波形の同時生存が 2 信号(src/cap の現帯域)までに削減
      されることを alloc 計測または同等の方法で検証。
- [ ] 実サンプル run で `analysis::reference` busy とピークメモリを
      計測・記録。

## Deferred Design Concerns

- locate 側 `PreparedChannel` の帯域逐次化: locate の BandNcc は
  5 帯域のみでメモリ影響が小さく、coarse 共有・残差版管理など
  別設計論点がある。本変更では触らない。#76 や #54 の検討時に
  再評価する。

## Sources and Relations

- 確定方針(P2): `docs/discovery/reference-analysis-analyze-pair-band-gain.md`
- P1 仕様: `docs/specs/valid-fft-ncc-plan-issue-71-p1.md`
- 調査: `docs/research/issue-71-band-correlation/algorithm-research-report.md`
  (Streaming Bandpass = 帯域ごと IFFT→窓解析→破棄、主成果はメモリ)
