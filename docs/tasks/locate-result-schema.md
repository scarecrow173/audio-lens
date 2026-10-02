---
id: "TASK-034z8kAE3mkxbYycE0kq7l"
type: "task"
status: "todo"
title: "LocateResult スキーマ + AlignmentId 発行"
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
    - "docs/plans/locate-and-tracking.md"
  implemented-by: []
  depends-on:
    - "docs/plans/locate-and-tracking.md"
    - "docs/tasks/locate-methods.md"
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
    - "docs/test-specs/locate-method-consensus.md"
  references: []
  defers: []
  deferred-by: []
---

# LocateResult スキーマ + AlignmentId 発行

## Work

spec の出力必須項目を持つ `LocateResult` スキーマと `AlignmentId` 発行を実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/locate-and-tracking.md`
- `docs/specs/source-localization.md`
- `docs/adr/multi-method-locate-consensus.md`

## Dependencies

- `docs/tasks/locate-methods.md`

## Affected Files

- `crates/al-locate/src/result.rs`(`AlignmentId`、`LocateResult`、artifact role: `locate_result`/`locate_evidence`/`time_map`/`residual`)

## Done When

- [ ] `LocateResult` がソース素材 id・ソース区間・キャプチャ区間・手法ごとの証拠・手法一致数・位置のばらつき・確度/validity・アルゴリズム/バージョン/パラメータ・alignment id を持つ
- [ ] `AlignmentId` が一意に発行され serde ラウンドトリップする

## Verification

- [ ] `cargo test -p al-locate` パス
