---
id: "SPEC-034FnccPlanWsReuse0002"
type: "spec"
status: "approved"
title: "NCC plan/workspace 再利用と IPD drift 計測の復旧"
created: "2026-10-14"
updated: "2026-10-15"
owners: ["scarecrow173"]
relations:
  source: []
  related:
    - "../adr/ncc-plan-workspace-reuse.md"
  derives-from:
    - "../discovery/ncc-plan-workspace-reuse.md"
  references:
    - "al-math-api-boundary.md"
    - "../adr/al-math-api-fft-split-re-im.md"
    - "caller-side-optimizations.md"
---

# NCC plan/workspace 再利用と IPD drift 計測の復旧

## What Should Be Built

反復相関計算でFFT準備と作業領域を再利用できる明示的な基盤を提供する。
同じ基盤でNCCとGCC-PHATを実行でき、既存free functionも利用できる。
IPD driftの既存caller不具合を先に独立修正する。

## Why Now

ロードマップ#52はband_gain::windows約98K calls、IPD drift約1.8K callsを対象に、
各呼出しで繰り返すFFT準備・確保を削減する。回数はIssue由来で、今回の実測値ではない。
現IPD driftは同長blockのNCCからlag候補を得られず、異なる時間帯のlag変化を報告できない。

## Users and Value

解析利用者は正しいlag driftを得る。実装者は共有可能な不変資源と排他的な可変資源の寿命を
明示でき、将来workerごとに独立した作業領域を持てる。

## Scope

### In Scope

- 汎用FFT plan/workspace基盤とNCC/GCC-PHATの再利用経路。
- band_gain::windowsとcheap.rs IPD driftの2サイト移行。
- IPD検索窓とlag座標のcaller境界修正、回帰検証、独立先行commit。
- 等価性・所有権・確保削減・cold/steady-state計測。

### Out of Scope

- al-locate移行、他callerの一斉移行、TLS cache、scratch pool、新依存。
- FFT kernel交換、rustfft導入、#33/#36/#55の変更、並列実行機能自体。

## Requirements

- 不変planはFFT長に紐づき共有可能、mutable Workspaceは呼出側またはworkerが排他所有する。
- NCCとGCC-PHATはFFT planを共有でき、PHAT正規化は実行時指定とする。
- ncc()/gcc_phat() free functionとbackend中立split re/im境界を維持する。
- NCCの符号、候補lag方向、空入力/短signal/ゼロエネルギー契約を維持する。
- IPD driftは100ms単位の第1channel pairのlag変化を測る。探索窓shapeとindex原点を整合させ、
  同長入力の合法clampを変更して補償しない。

## Acceptance Criteria

- [ ] 独立回帰テストが修正前red、修正後greenとなり、既知の異なるsigned lag間のdriftを報告する。
- [ ] 固定lag、零探索幅、短入力でIPD driftの境界挙動を検証する。signed lag原点も最低意味層で検証する。
- [ ] reuse経路とfree functionをNCC/PHAT双方で独立参照または既存goldenと比較し、既存の許容誤差・候補方向・極性を保つ。
- [ ] 異なるshape、空入力、ゼロエネルギー、連続再利用で前回buffer内容が結果へ混入しない。
- [ ] planのSend+Sync共有をコンパイル検証し、共有planとworker別workspaceの並列結果が逐次結果と一致する。
- [ ] 反復2サイトでplan構築/確保回数をbaselineと比較し、内側ループのplan再構築・scratch確保を除去した証拠を保存する。
- [ ] cold/steady-state、p50/p95、RSSとfixture・環境・shapeを保存し、速度向上を未計測で主張しない。
- [ ] IPD修正がplan API移行と独立した先行commitになり、移行範囲が2サイトに収まる。

## Deferred Design Concerns

shape不一致契約、具体API、結果buffer返却方式、測定fixture/誤差基準の具体化は
次のdesign reviewの必須項目。実装開始前に確定し、新たな依存判断は含めない。

## Sources and Relations

共通anchorに確定判断と静的診断を記録。al-math-api-boundaryは上流制約の参照であり、
既存文書全体をsupersedeしない。今回判断を承認。ownersは2026-10-15にscarecrow173と設定した。


## 2026-10-15 Decision Review

A/B candidates各0、Validator追加0、Judge ACCEPT。coordinatorが意思決定のみ承認。
red/green、等価性・共有・確保削減・性能実証は未実施であり、この承認に含まれない。
