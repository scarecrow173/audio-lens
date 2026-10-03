---
id: "IMPL-034z8DenxqaJmDmtNFrAok"
type: "impl"
status: "completed"
title: "locate-consensus"
created: "2026-10-02"
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
    - "docs/tasks/locate-consensus.md"
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

# locate-consensus

## Summary

合議受理を `consensus::evaluate` として実装。`min_agreeing_methods`
(デフォルト 2)以上の独立手法が `position_tolerance` 内で一致し、
スコア/ピーク対背景比の閾値を超えた場合のみ受理。競合クラスタは
`AmbiguousAlignment` で明示棄却。

## Implementation

### Changes

- `crates/al-locate/src/consensus.rs`: `ConsensusConfig` + `evaluate`

### Decisions During Implementation

- クラスタリングは位置ソート+クラスタ幅による近接併合(新規投票は
  クラスタ先頭から `position_tolerance` 以内の場合のみ併合し、
  クラスタ内の全ペア差が tolerance を超えないことを保証する。
  レビュー修正: 以前は隣接差のみを見る single-link で、連鎖的に
  tolerance を超える広がりを 1 クラスタに吸収していた)。
  最大クラスタが採用候補、第2クラスタも閾値を満たす場合は
  曖昧として棄却(無言の選択をしない)
- 偶数クラスタの代表位置は下側中央値を採用

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Context

task `docs/tasks/locate-consensus.md`、test-spec
`docs/test-specs/locate-method-consensus.md`、ADR
`docs/adr/multi-method-locate-consensus.md`。commit `d14cfae`。

## Validation

- `cargo test -p al-locate` 全 pass(2 手法一致で受理 / 単一手法の
  偽ピークは棄却 / 投票ゼロ棄却 / 競合クラスタは曖昧)

## Risks

- デフォルト閾値(position_tolerance=256 等)は未調整の暫定値。
  config で上書き可能

## Follow-ups

- なし
