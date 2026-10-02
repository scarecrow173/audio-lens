---
id: "TASK-034z8kJaSUxUrTrHj1cMG7"
type: "task"
status: "done"
title: "residual-search: 2段階探索 + 減算来歴"
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
    - "docs/tasks/locate-mixer.md"
    - "docs/tasks/locate-consensus.md"
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
    - "docs/test-specs/locate-tracking-residual.md"
  references: []
  defers: []
  deferred-by: []
---

# residual-search: 2段階探索 + 減算来歴

## Work

支配的ソースの 2 段階残差探索を実装。減算来歴(親ソース・alignment id・推定ゲイン/チャンネル変換[混合行列]・減算前後エネルギー・減算確度・子 locate)を保持する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/locate-and-tracking.md`
- `docs/specs/locate-tracking.md`

## Dependencies

- `docs/tasks/locate-mixer.md`
- `docs/tasks/locate-consensus.md`

## Affected Files

- `crates/al-locate/src/residual.rs`

## Done When

- [ ] 支配的 BGM 減算後に静かな素材が受理される
- [ ] 減算来歴の全項目が永続化される

## Verification

- [ ] 残差探索 fixture が静かな素材を受理する
- [ ] `cargo test -p al-locate` パス
