---
id: "TASK-034z8kB6w92jCsle2QoqKe"
type: "task"
status: "done"
title: "合議受理 + 失敗モードフラグ"
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
    - "docs/tasks/locate-result-schema.md"
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

# 合議受理 + 失敗モードフラグ

## Work

合議受理(≥2 手法の位置一致[許容差設定可] + ピーク/背景閾値)と失敗モードフラグ(無音境界・周期信号・広いピーク・低情報窓・偽相関)を実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/locate-and-tracking.md`
- `docs/specs/source-localization.md`
- `docs/test-specs/locate-method-consensus.md`

## Dependencies

- `docs/tasks/locate-result-schema.md`

## Affected Files

- `crates/al-locate/src/consensus.rs`

## Done When

- [ ] 単一手法のピークは受理されない
- [ ] 棄却時も全手法の証拠と棄却理由が保持される
- [ ] 閾値・許容差は config 経由で設定可能

## Verification

- [ ] 偽ピーク fixture が受理されない
- [ ] `cargo test -p al-locate` パス
