---
id: "TASK-034AaG1gz22ttDMRbpSJsW"
type: "task"
status: "done"
title: "golden: video/A-V + コンテナケース行列"
created: "2026-10-04"
updated: "2026-10-04"
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
    - "docs/plans/golden-validation.md"
  implemented-by: []
  depends-on:
    - "docs/plans/golden-validation.md"
    - "docs/tasks/al-testkit-crate.md"
    - "docs/tasks/testkit-expect-helpers.md"
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
    - "docs/test-specs/golden-validation.md"
  references: []
  defers: []
  deferred-by: []
---

# golden: video/A-V + コンテナケース行列

## Work

spec の video/A-V ケース行列を実装する(CFR/VFR タイムスタンプ、PCM パディング/dwStart 由来の既知 A/V offset、drift、dup/drop メタデータ)。パラメタ化 AVI/MP4 writer を使う。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/golden-validation.md`
- `docs/test-specs/golden-validation.md`

## Dependencies

- `docs/tasks/al-testkit-crate.md`
- `docs/tasks/testkit-expect-helpers.md`

## Affected Files

- `crates/al-media/tests/golden.rs`

## Done When

- [ ] spec 列挙の video/A-V ケースがテストとして存在する
- [ ] VFR が nominal FPS 演算なしで観測される

## Verification

- [ ] `cargo test -p al-media`
