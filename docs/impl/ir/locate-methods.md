---
id: "IMPL-034z8DdANt5T1siWquvDRj"
type: "impl"
status: "completed"
title: "locate-methods"
created: "2026-10-02"
updated: "2026-10-02"
owners: []
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
    - "docs/tasks/locate-methods.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# locate-methods

## Summary

`LocateMethod` trait と 4 手法(full_ncc / band_ncc / gcc_phat /
spectral_envelope)を実装し、候補・スコア・ピーク対背景比・ピーク幅・
第1/第2ピーク比・validity・棄却理由を `MethodEvidence` として生成する。

## Implementation

### Changes

- `crates/al-locate/src/candidate.rs`: `LocateCandidate` /
  `MethodEvidence` 型
- `crates/al-locate/src/methods.rs`: trait + 4 実装 +
  `default_methods()`

### Decisions During Implementation

- 各手法は独立に validity を返し、棄却も証拠として保持
  (consensus が全証拠を受け取る契約)
- ピーク対背景比は median |corr| ベース。DC バイアス信号では
  背景が歪むためテスト fixture はゼロ平均ノイズを使用

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Context

task `docs/tasks/locate-methods.md`、plan
`docs/plans/locate-and-tracking.md`。commit `d14cfae`。

## Validation

- `cargo test -p al-locate` 全 pass(known offset 検出・無音/短入力の
  明示的棄却を含む)

## Risks

- spectral_envelope は近似手法で位置精度が他手法より粗い。
  合議の位置許容差内に収まるかは fixture 基準で検証済み

## Follow-ups

- なし
