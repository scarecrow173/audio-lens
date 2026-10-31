---
id: "TASK-034P3P0JW4zZM5GdZlp1WA"
type: "task"
status: "todo"
title: "旧基盤撤去と最終検証(AL#104)"
created: "2026-10-30"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/104"
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
    - "docs/tasks/ruviform-viz-retirement.md"
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

# 旧基盤撤去と最終検証(AL#104)

## Work

移行完了後の最終ゲート。二重実装・未使用 crate・暫定
adapter/比較フラグの撤去、性能・精度の最終確認、依存・
ライセンス・CI 監査、クロスリポジトリの Issue/ADR/DAG 整合を
行う。Issue
[#104](https://github.com/scarecrow173/audio-lens/issues/104) を正本とする。

- plan の「二重実装撤去条件」1〜4 に従い、条件を満たす旧実装
  (`al-math` カーネル、`al-media` 汎用 I/O、暫定 adapter)を撤去
- `al-math` 溶解・`al-media` 縮小の残責務を責務マップと照合
- `al-observability` の `ruviform-observability` 委譲可否を監査
- 例外として残すものは Issue + 理由を必須とする

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/ruviform-foundation-migration.md`
- `docs/designs/ruviform-migration-responsibility-map.md`
- Issue [#104](https://github.com/scarecrow173/audio-lens/issues/104)

## Dependencies

- `docs/tasks/ruviform-viz-retirement.md`(全先行完了ゲートを
  推移的に包含)

## Affected Files

- `crates/al-math`(撤去候補)
- `crates/al-media`(縮小後の残量整理)
- `crates/al-observability`(委譲可否の監査対象)
- 暫定 adapter/比較フラグの残存箇所

## Done When

- [ ] 二重実装撤去条件を満たす旧実装が撤去され、例外には Issue
      と理由がある
- [ ] 性能・精度の最終確認に説明不能な退行がない
- [ ] #104 の完了条件を満たす(依存・ライセンス・CI・DAG 監査)

## Verification

- [ ] 未使用 crate/重複実装が残っていない
- [ ] `mise run test` 相当・最終性能/精度確認が通る
