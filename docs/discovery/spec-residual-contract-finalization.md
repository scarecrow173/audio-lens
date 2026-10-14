---
id: "DISC-034FPdDmmo6fGWovNellpV"
type: "discovery"
status: "active"
title: "Issue 33 確定仕様の適用境界"
created: "2026-10-14"
updated: "2026-10-14"
owners: []
relations:
  source:
    - "remaining-issues-roadmap.md"
    - "https://github.com/scarecrow173/audio-lens/issues/33"
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
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/spec-residual-contract-finalization.md"
    - "../adr/spec-residual-contract-finalization.md"
  verifies: []
  verified-by: []
  references:
    - "../tasks/spec-residual-decisions.md"
  defers: []
  deferred-by: []
---

# Issue 33 確定仕様の適用境界

## Exploration Goal

2026-10-14 に確定した #33 判断を、旧仕様の決定履歴を保持して正式仕様へ適用する。

## Findings

旧 task は done。sample_rate は実装済みであり、candidate table は stage.rs 446–504 にて
空の場合も保存する。一方、旧仕様は FLAC 保存を defer として保持している。
今回は WAV のみを正式仕様にするため、旧判断とは別の適用単位が必要。

## Decision Inputs

fused 実行を維持し候補データは独立参照・空保存。WAV のみ、Float64 PCM と来歴を維持。
StageKind / manifest / resume 境界を維持。入力 FLAC decode は出力 encode と独立。
新依存はなし。将来の圧縮は容量・時間・I/O 実測と精度保持を条件に別 Issue。

## Boundary and Open Questions

ユーザー未決事項はない。旧文書は今回適用対象として references で辿る。
旧 overview/roadmap 全体の lineage を今回 chain に取り込むと既存複数 plan が focus を
曖昧にするため、本 discovery を今回 spec/ADR の共通 anchor にする。
これは旧判断の無効化や他 Issue の再承認ではない。

## Promotion Candidates

同名 spec/ADR が判断契約を所有する。次は docs-only design / plan / task として適用・検証する。
