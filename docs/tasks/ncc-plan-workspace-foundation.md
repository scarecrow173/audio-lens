---
id: "TASK-034FQrcCGZZgIfcHyHbt58"
type: "task"
status: "todo"
title: "FFT と相関の不変planと排他workspaceを分離する"
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
    - "docs/tasks/ipd-lag-search-fix.md"
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

# FFT と相関の不変planと排他workspaceを分離する

## Work


1. plan Send+Sync、shape/pow2不一致、borrowed output、既存oracle等価性とfreshthread allocation/dropのテストを作る。
2. immutable FFT係数とpacked scratch/large skew stageを分離しconstructorで全確保する。
3. StageRun本体を共通generic primitiveへ抽出し、explicit pathは外側dispatchのsimdを直接渡す。
   RealTransformの2方向はexplicit pathへ、legacy Fft TLS pathは既存dispatchを維持する。
4. CorrelationPlan/Workspaceを実装し、NCC/PHAT共通kernelにfreewrapperを統合する。
5. 下記matrixで数値/境界/firstcall/dropを検証、math変更と対応証拠を論理commitする。

## Acceptance Criteria

- [ ] 上記sliceの契約・検証・独立reviewを満たす。
- [ ] 実行結果を対応irへ記録し、上流planの順序/commit境界を維持する。

## Verification

[承認済みplanのVerification Matrix](../plans/implement-ncc-plan-workspace-reuse.md)に従う。
今回同義具体化は2026-10-15 plan Judge ACCEPTに含む。依存/Cargo.lock変更は禁止、mise経由で実行する。
あなたは他者と共存するため所有外の変更をrevertしない。
