---
id: "TASK-034z8kK3Fu6B5tJ0qnORg0"
type: "task"
status: "done"
title: "locate 結合テスト(合成 fixture)"
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
    - "docs/tasks/locate-stage-cli.md"
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
    - "docs/test-specs/locate-tracking-residual.md"
  references: []
  defers: []
  deferred-by: []
---

# locate 結合テスト(合成 fixture)

## Work

`validation-strategy` の Locate fixture を `crates/al-locate/tests/` に実装:ゲイン変更挿入・EQ 済み挿入・複数出現・線形ドリフト・スキップ+再開・マルチch ダウンミックス(極性反転)・支配的 BGM 後の残差受理。両 test-spec の全 Example を網羅する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/test-specs/locate-method-consensus.md`
- `docs/test-specs/locate-tracking-residual.md`
- `docs/specs/validation-strategy.md`

## Dependencies

- `docs/tasks/locate-stage-cli.md`

## Affected Files

- `crates/al-locate/tests/`

## Done When

- [ ] 両 test-spec の全 Example に対応するテストがある
- [ ] `cargo test --workspace` パス
- [ ] `cargo clippy --workspace` clean

## Verification

- [ ] 上記が全て通る
