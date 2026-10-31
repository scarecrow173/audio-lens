---
id: "TASK-034P3P1fwfyIxni1zVuomQ"
type: "task"
status: "todo"
title: "ruviform-exec による stage 内並列化の評価(AL#54)"
created: "2026-10-30"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/54"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/ruviform-foundation-migration.md"
  implemented-by: []
  depends-on:
    - "docs/plans/ruviform-foundation-migration.md"
    - "docs/tasks/ruviform-dependency-ci.md"
    - "docs/tasks/ruviform-migration-baseline.md"
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
---

# ruviform-exec による stage 内並列化の評価(AL#54)

## Work

`ruviform-exec` の OfflineDriver(チャンク駆動オフライン実行・
進捗・キャンセル・ワーカープール)を AudioLens の stage 内
並列化に適用できるかを実測評価し、採否を決める。Issue
[#54](https://github.com/scarecrow173/audio-lens/issues/54) を正本とする。
本 plan 上はクリティカルパスに入れない独立評価。

- 逐次参照等価の決定性契約・チャンク独立の呼出側契約を
  AL 側 stage が満たせるか確認
- 適合する stage で性能効果を実測し採否を記録

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/ruviform-foundation-migration.md`
- `docs/designs/ruviform-migration-responsibility-map.md` §5
- Issue [#54](https://github.com/scarecrow173/audio-lens/issues/54)

## Dependencies

- `docs/tasks/ruviform-dependency-ci.md`(Ruviform 依存の導入後)
- `docs/tasks/ruviform-migration-baseline.md`(性能計測は #98
  baseline を基準とする)

## Affected Files

- `crates/al-job` または stage 実装(評価対象の並列化箇所)

## Done When

- [ ] OfflineDriver の適合性評価と採否の記録がある
- [ ] 採用時は決定性契約違反がないことが検証される

## Verification

- [ ] 並列化評価対象での決定性・性能実測
