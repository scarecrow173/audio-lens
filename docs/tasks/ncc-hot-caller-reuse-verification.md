---
id: "TASK-034FQrcP4zy8KVn51A8pyX"
type: "task"
status: "todo"
title: "2hot callerを再利用に移し確保と速度を検証する"
created: "2026-10-14"
updated: "2026-10-14"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-ncc-plan-workspace-reuse.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-ncc-plan-workspace-reuse.md"
    - "docs/tasks/ncc-plan-workspace-foundation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/ncc-plan-workspace-reuse.md"
  references: []
  defers: []
  deferred-by: []
---

# 2hot callerを再利用に移し確保と速度を検証する

## Work


1. band_gain windowsとIPD helperのcurve生成だけreuseへ移し、loop外所有・loop内borrow消費を確認する。
2. 2site既存挙動の回帰、loop内plan/scratch確保0、逐次/worker別ws結果一致を確認する。
3. 同fixture/toolchain/host/設定でbaselineとpostのcold/steady p50/p95/RSSを比較する。
4. targeted tests、workspace fmt/clippy、docs audit/diffcheck、独立完成reviewを行う。
5. 修正可能な非Blockerを直し、移行/測定/対応irをcommitしてGraph commit gateを確認する。

3slicesは依存順のみ、並列実装しない。独立reviewと必要な読取調査は並列可。

## Acceptance Criteria

- [ ] 上記sliceの契約・検証・独立reviewを満たす。
- [ ] 実行結果を対応irへ記録し、上流planの順序/commit境界を維持する。

## Verification

[承認済みplanのVerification Matrix](../plans/implement-ncc-plan-workspace-reuse.md)に従う。
今回同義具体化は2026-10-15 plan Judge ACCEPTに含む。依存/Cargo.lock変更は禁止、mise経由で実行する。
あなたは他者と共存するため所有外の変更をrevertしない。
