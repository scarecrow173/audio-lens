---
id: "SPEC-034H5lyClpFOg52Hv86maq"
type: "spec"
status: "draft"
title: "Valid FFT NCC plan (issue #71 P1)"
created: "2026-10-16"
updated: "2026-10-17"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/71"
    - "../discovery/reference-analysis-analyze-pair-band-gain.md"
    - "../research/issue-71-band-correlation/algorithm-research-report.md"
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
    - "../discovery/issue-40-locate-pair-plan-cache.md"
    - "../discovery/issue-53-local-lag-fft.md"
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

# Valid FFT NCC plan (issue #71 P1)

## What Should Be Built

`CorrelationPlan` の NCC 経路に、有効ラグ(`0..=S−L`)だけを正しく計算すれば
よいことを利用した **Valid FFT** モードを追加する。テンプレートが信号内に
完全に収まるラグのみが対象の場合、FFT 長を現行の
`nextpow2(S+L−1)` から `nextpow2(S)` へ短縮できる。

- NCC 専用の plan 生成経路(例: `CorrelationPlan::new_ncc_valid`)を
  追加し、FFT 長を `nextpow2(S)` とする。`nextpow2(S) ==
  nextpow2(S+L−1)` の形状では同一 plan と等価になり、常に安全側へ
  倒れる。
- `gcc_phat` は従来通り `nextpow2(S+L−1)` を維持する(周波数ビンの
  位相正規化が FFT 長依存のため、短縮すると結果が変わる)。NCC と
  PHAT の plan を分離し、valid plan での `gcc_phat` 呼出を契約違反
  として扱う。
- `CorrelationPool` の形状キーに mode(NCC-valid / 従来)を含め、
  ncc と gcc_phat が異なる FFT 長の plan を誤共有しないようにする。
- `dsp::ncc` / `dsp::gcc_phat` フリー関数は pool 経由と同一 plan を
  使う(両経路の結果一致を維持)。
- `band_gain::windows` の `CorrelationPlan::new` 呼出を NCC-valid
  plan へ切り替える。

## Why Now

実測で locate `band_ncc` が 174s busy(1呼出 ~10.9s、帯域ごと全長 NCC)、
reference `band_gain` が ~23s/pair(窓ごと FFT-NCC ~47,600呼)と確定し、
両者の共通カーネルである `CorrelationPlan` の FFT 長が
`nextpow2(S+L−1)` と過大であることが判明した。等長近傍の形状では
`nextpow2(S)` で半減できる。issue #71 の確定方針 P1。

## Users and Value

- 解析ジョブ実行者: locate(band_ncc/full_ncc)と reference(band_gain)
  の支配的カーネル時間が、推定量を変えずに縮小する。
- 開発者: NCC と GCC-PHAT の plan 契約が明確に分離され、後続の
  dispatch/Blocked FFT(P3)の基盤になる。

## Scope

### In Scope

- `al-math`: NCC-valid 用の plan 生成経路と FFT 長選択
  (`nextpow2(S)`、`S==0` や `S<L` のガードは現行と同じ契約)
- `al-locate`: `CorrelationPool` のキー拡張(mode 区別)、ncc が
  valid plan、gcc_phat が従来 plan を使う
- `al-analysis`: `band_gain::windows` の plan を NCC-valid へ切替
- `dsp::ncc`/`gcc_phat` フリー関数と pool 経路の plan 一致
- 回帰テスト: 全有効ラグ値・peak/tie/順位・scalar↔dispatch
- 実測: 実サンプル run での band_ncc / band_gain busy 比較

### Out of Scope

- GCC-PHAT の FFT 長短縮(契約が変わるため禁止)
- Streaming Bandpass(P2)、適応 dispatch / Blocked FFT(P3)
- 別推定器(multirate/SOS/FIR/ViSQOL/指紋/H1)の導入
- `OverlapCorrPlan`(local_lag、別演算契約で既に最適化済み)

## Requirements

- R1: valid FFT plan は `nextpow2(S)` を FFT 長とし、
  `S==0 || S<L` では現行同様に演算不要 shape とする。
- R2: NCC の返却曲線 `real[..=max_lag]`(`max_lag ≤ S−L` に
  clamp)の全値が、従来 plan の出力と既存 scalar↔dispatch 許容基準内で
  一致すること。wraparound 汚染は返却範囲外にのみ発生することを
  数学的根拠として文書化する。
- R3: `gcc_phat` は従来 FFT 長のままとし、valid plan での phat 実行は
  契約違反(開発時に検出可能な形)とする。
- R4: `CorrelationPool` は (template_len, signal_len, mode) をキーとし、
  ncc と gcc_phat の plan を誤共有しない。`MAX_SHAPES` 上限の挙動は
  維持する。
- R5: `dsp::ncc` は valid plan、`dsp::gcc_phat` は従来 plan を使い、
  pool 経路と一致させる。
- R6: `band_gain::windows` は NCC-valid plan を使い、出力(gain/corr/
  lag/support/reason/Tensor)が従来と一致する。
- R7: scalar 実装と runtime-dispatch 実装の数値同等性を維持する
  (FFT 長の変更は両経路で同一)。

## Acceptance Criteria

- [ ] 全有効ラグ(両端・等長・短 template・K=1)で valid plan と
      従来 plan の NCC 曲線が一致(既存基準の相対/絶対誤差内)。
- [ ] `dsp::ncc` / `pool.ncc` / `band_gain` 経路で同一結果。
- [ ] `dsp::gcc_phat` / `pool.gcc_phat` が従来値と一致(FFT 長不変)。
- [ ] peak argmax・tie 群・順位・曖昧性の回帰が一致。
- [ ] 実サンプル run で band_ncc・band_gain の busy 時間が改善
      (FFT 長半減形状でおおむね半減を確認)し、効かない形状では
      従来と同等。
- [ ] `audit_docs`・clippy・workspace テストが全て clean。

## Deferred Design Concerns

- P2 Streaming Bandpass(帯域逐次生成、~1.8GB メモリ削減):
  P1 の plan 分離後に独立 PR。`InputCache` の責務分割設計が必要。
- P3 適応 dispatch + Blocked FFT: (template_len, signal_len, n_lags,
  memory) による方式選択。形状ベンチで採否を決める。
- PHAT 用 FFT 長の見直し: 契約変更を伴うため恒久的に除外。

## Sources and Relations

- 上流: `docs/discovery/reference-analysis-analyze-pair-band-gain.md`
  (確定方針・計測内訳)、`docs/research/issue-71-band-correlation/` の
  調査レポート(Valid FFT の数学的根拠・OSS 先例・受入試験要件)
- 関連: `issue-40-locate-pair-plan-cache.md`(CorrelationPool)、
  `issue-53-local-lag-fft.md`(OverlapCorrPlan)
