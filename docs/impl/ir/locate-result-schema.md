---
id: "IMPL-034z8DdZyFszV4L4Bvi5M0"
type: "impl"
status: "completed"
title: "locate-result-schema"
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
    - "docs/tasks/locate-result-schema.md"
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

# locate-result-schema

## Summary

`LocateResult` スキーマと `AlignmentId`(`ALN-` + base62×22)を
実装。spec の出力必須項目(ソース素材 id・ソース区間・キャプチャ区間・
手法ごとの証拠・手法一致数・位置のばらつき・確度/validity・棄却理由・
アルゴリズム id/バージョン・パラメータ)をすべて保持する。

## Implementation

### Changes

- `crates/al-core/src/ids.rs`: `AlignmentId` 追加
- `crates/al-locate/src/result.rs`: `SampleRange` / `LocateResult`

### Decisions During Implementation

- `accepted` フラグで「合議受理」と「証拠はあるが棄却」を区別。
  棄却結果も削除せず永続化する

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Context

task `docs/tasks/locate-result-schema.md`、plan
`docs/plans/locate-and-tracking.md`。commit `d14cfae`。

## Validation

- `cargo test -p al-locate` 全 pass

## Risks

- なし

## Follow-ups

- なし
