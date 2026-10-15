---
id: "ADR-034FnccPlanWsReuse0003"
type: "adr"
status: "accepted"
title: "不変 FFT plan と排他 workspace を分離する"
created: "2026-10-14"
updated: "2026-10-15"
owners: ["scarecrow173"]
relations:
  source: []
  related:
    - "../specs/ncc-plan-workspace-reuse.md"
  derives-from:
    - "../discovery/ncc-plan-workspace-reuse.md"
  references:
    - "al-math-api-fft-split-re-im.md"
    - "../specs/al-math-api-boundary.md"
    - "../designs/al-math-api-boundary.md"
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# 不変 FFT plan と排他 workspace を分離する

## Context and Problem Statement

相関の反復呼出しがRealFft係数とscratchを毎回構築する。現RealFftは不変係数と可変scratchを
同居させるため、共有可能資源と排他資源の寿命が分かれていない。
本ADRは2026-10-14のユーザー確定判断を記録するdraftであり、文書承認は別reviewで行う。

## Decision Drivers

- 反復準備・確保を削減し、所有者とRSSを明示する。
- backend中立split re/im境界と現FFT kernelを維持する。
- NCC/GCC-PHAT共通資源を一つの責務で管理する。

## Considered Options

- A: shape keyed thread-local plan/workspace cache。
- B: 不変RealFftPlanと明示的な呼出側/worker別mutable Workspace。
- C: scratch poolからbufferを借用する。

## Decision Outcome

Chosen option: B。FFT長に紐づく不変RealFftPlanをSend+Sync共有可能にし、Workspaceは
呼出側/workerが排他所有する。概念形は `plan.ncc(&mut ws, template, signal)`。
基盤は汎用とし、NCC/GCC-PHATはFFT planを共有する。PHATは実行時指定でplan状態にしない。
free functionを維持し、移行はband_gain::windowsとcheap IPD driftに限定する。
具体API名・shape不一致時の動作・結果借用/所有契約は次のdesign reviewで確定する。

### Consequences

- 不変係数を複数workerで共有でき、mutable buffer競合を型と所有権で防げる。
- FFT/correlation内部の所有境界変更と2callerの寿命設計が必要になる。
- TLSとpoolによる不透明な寿命・ロック管理を導入しない。

### Confirmation

specの等価性、共有plan/独立workspace、反復確保、cold/warm計測を確認する。
性能目標の実測値はdesign/planで固定し、Issue由来回数を実測扱いしない。

## Implementation Plan

- `crates/al-math/src/fft.rs`: 不変係数とscratchの責務を分離し、現kernel/layoutを維持する。
- `crates/al-math/src/correlation.rs`: FFTと相関作業領域を再利用する明示API、free function経路を整合させる。
- `crates/al-analysis/src/cheap.rs`: 先にIPD検索窓shape/lag原点をcallerで修正し、回帰red/greenを独立commitにする。
  NCCの `signal.len()-template.len()` clampは正しいため変更しない。その後固定shapeの反復でreuseする。
- `crates/al-analysis/src/band_gain.rs`: 既存のwindow+2max検索窓・signed lag・極性/tie判定を保ちreuseする。
- テストはal-math相関/FFT既存テストとcheap内既存helperを活用。依存・設定追加はなし。
- 避けるpattern: TLS、pool、caller側重複キャッシュ、al-locate移行、kernel交換、未計測の性能claim。
- リリース前方針に従い新たな互換layer/deprecationは設けない。指定されたfree function自体は維持する。

## Verification

- [ ] IPD異なるsigned lag fixtureのred/greenと一定lag/零探索幅/短入力を確認する。
- [ ] reuseとfree functionのNCC/PHAT、境界、極性、候補方向の等価性を確認する。
- [ ] Send+Sync共有plan/worker別workspaceの逐次・並列同一性を確認する。
- [ ] 2サイトのplan構築/確保数、cold/warm p50/p95・RSSをbaselineと比較し証拠を残す。
- [ ] API詳細がdesign reviewで確定し、独立IPD修正commitが移行に先行する。

## Pros and Cons of the Options

### A: TLS cache

- caller変更は少ない。
- 寿命とスレッド数×shapeのmemory増が不透明で、明示所有判断に反するため棄却。

### B: 明示plan/workspace

- 所有権とRSSが明瞭で、NCC/PHAT共通化と将来worker別workspaceに整合する。
- APIと内部所有境界の変更が必要。

### C: scratch pool

- buffer型をcallerから隠せる。
- ロック競合と借用寿命管理が増え、今回の2サイトには不要なため棄却。

## More Information

共通discovery anchorと関連specにscope・証拠を記録。shapeが入力依存のal-locate移行、
kernel交換は将来別Issue/測定で判断する。ownersは2026-10-15にscarecrow173と設定した。文書承認は今回reviewcheckpointで判断する。


## 2026-10-15 Decision Review

A/B candidates各0、Validator追加0、Judge ACCEPT。coordinatorが意思決定のみ承認。
red/green、等価性・共有・確保削減・性能実証は未実施であり、この承認に含まれない。
