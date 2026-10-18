---
id: "DISC-034FnccPlanWsReuse0001"
type: "discovery"
status: "active"
title: "Issue 52 NCC plan/workspace 再利用の適用境界"
created: "2026-10-14"
updated: "2026-10-15"
owners: ["scarecrow173"]
relations:
  source:
    - "remaining-issues-roadmap.md"
    - "https://github.com/scarecrow173/audio-lens/issues/52"
  derives-from: []
  derived-by:
    - "../specs/ncc-plan-workspace-reuse.md"
    - "../adr/ncc-plan-workspace-reuse.md"
  references:
    - "../specs/al-math-api-boundary.md"
    - "../designs/al-math-api-boundary.md"
    - "../specs/caller-side-optimizations.md"
---

# Issue 52 NCC plan/workspace 再利用の適用境界

## Exploration Goal

ロードマップ §#52 の2026-10-14確定判断を、専用の spec/ADR に記録する。
Entry Decision: A-4。要求・選択肢・採用判断は確定済みで、API詳細は次の design review が所有する。
dispatch-specific override: 親からの限定文書作成指示に従い、briefing-flow の SUBAGENT-STOP を適用し、profile生成・追加委譲・共有index更新を行わない。

## Findings

`crates/al-math/src/correlation.rs:64-90` は毎回FFT係数と作業bufferを確保する。
`fft.rs:717-726` は不変係数と可変scratchを RealFft に同居させている。
既存 caller-side-optimizations spec はA2 NCC plan再利用を対象外にしており、#52専用chainはない。
既存 backend中立・split re/im契約を維持する。

IPD drift の `cheap.rs:1278-1280` は同長blockを渡すため、NCCの合法なclampにより
候補がlag 0一つになる。さらにindexからmax_lagを引くため、全blockの推定値が同じになりdriftが0になる。
違反の所有者はcallerの検索窓shape/座標であり、NCC clampではない。これは静的診断で、red実行は未実施。

## Decision Inputs

不変 RealFftPlan と排他所有のmutable Workspaceを分離する。基盤は汎用で、NCC/GCC-PHATが
FFT planを共有し、PHAT指定は実行時に行う。free functionを維持する。
移行はband_gain::windowsとcheap IPD driftの2サイトのみ。IPD修正を独立commitで先行する。
TLS、pool、新依存、al-locate移行、FFT kernel交換は対象外。

## Boundary and Open Questions

spec/ADRは本artifactを共通anchorとする。全roadmapのlineageを取り込まない。
shape不一致の扱い、結果bufferの返却方式、具体API名・詳細はdesign reviewで確定する。
spec/ADRの判断は2026-10-15 Judge ACCEPT。動的実証は未実施。ownersはscarecrow173。

## Promotion Candidates

focused spec/ADRをreviewprotocolへ提出し、承認後にgraph routeを再投影する。
