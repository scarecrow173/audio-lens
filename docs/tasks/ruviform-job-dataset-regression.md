---
id: "TASK-034P3OZ5AfOy7hwO4pHUHy"
type: "task"
status: "todo"
title: "Job・Dataset・Provenance 回帰(AL#103)"
created: "2026-10-30"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/103"
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
    - "docs/tasks/ruviform-media-io-switch.md"
    - "docs/tasks/ruviform-dsp-kernel-switch.md"
    - "docs/tasks/ruviform-domain-kernel-separation.md"
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

# Job・Dataset・Provenance 回帰(AL#103)

## Work

切替後の pipeline 全体で、不変 Job・Manifest・Analysis Dataset・
Provenance・再開が回帰なく動くことを実ジョブで検証する。Issue
[#103](https://github.com/scarecrow173/audio-lens/issues/103) を正本とする。

- ingest〜export の実ジョブで Dataset 生成物・再開・部分失敗を
  回帰検証
- Provenance に Ruviform の revision/backend/パラメータを記録
  できることを確認
- GUI/headless サービスが同一 Dataset を再解析なしで読める
  query/data port の前提を検証

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/ruviform-foundation-migration.md`
- `docs/designs/ruviform-migration-responsibility-map.md` §5/§6/§7
- Issue [#103](https://github.com/scarecrow173/audio-lens/issues/103)

## Dependencies

- `docs/tasks/ruviform-media-io-switch.md`
- `docs/tasks/ruviform-dsp-kernel-switch.md`
- `docs/tasks/ruviform-domain-kernel-separation.md`

## Affected Files

- `crates/al-job`(回帰検証・必要なら provenance 記録の拡張)
- `crates/al-dataset`(provenance 追跡・reader/query の回帰)
- `crates/al-cli`(実ジョブの入口)

## Done When

- [ ] #103 の完了条件を満たす(実ジョブ回帰・再開・provenance
      追跡可能性)
- [ ] 永続 Dataset が再解析なしに範囲読出し・絞込みできる
      reader/query port 前提が検証される(GUI 実側の確認は
      #105 で実施)

## Verification

- [ ] 実ジョブの ingest〜export 回帰・再開・部分失敗検証
- [ ] provenance に Ruviform revision/backend/params が記録される
- [ ] `mise run test` 相当が通る
