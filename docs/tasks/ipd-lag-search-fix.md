---
id: "TASK-034FQrcq7vbkfsYZwC2wrc"
type: "task"
status: "in-progress"
title: "IPD 検索窓と signed lag 原点を修復する"
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

# IPD 検索窓と signed lag 原点を修復する

## Work


1. cheap内に非周期wideband 2区間lag -3/+5の独立回帰を先に追加し、現実装drift0でredを保存する。
2. NCC合法clampを変えずfull-margin検索窓・signed origin helperを修正する。
3. signed helper、固定lag、max0、負設定の既存usize飽和、短入力、exactblock、第一channel pair境界を確認する。
4. targeted cheap回帰green→IPD fix/回帰/irを独立commit。plan APIをこのcommitに混ぜない。
5. fixture/benchmark harnessを用意し、このfix後commitをpinしてbaselineを取得する。
   harness追加は必要なら独立commit。bug修正によるworkload差をreuse効果と混同しない。

## Acceptance Criteria

- [ ] 上記sliceの契約・検証・独立reviewを満たす。
- [ ] 実行結果を対応irへ記録し、上流planの順序/commit境界を維持する。

## Verification

[承認済みplanのVerification Matrix](../plans/implement-ncc-plan-workspace-reuse.md)に従う。
今回同義具体化は2026-10-15 plan Judge ACCEPTに含む。依存/Cargo.lock変更は禁止、mise経由で実行する。
あなたは他者と共存するため所有外の変更をrevertしない。
