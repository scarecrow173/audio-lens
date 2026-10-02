---
id: "TASK-034yZBGf1qCjTi7xP8XFVR"
type: "task"
status: "todo"
title: "run / resume 再現テストの実装"
created: "2026-10-02"
updated: "2026-10-02"
owners: ["scarecrow173"]
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
    - "docs/plans/implement-job-orchestration-foundation.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/stage-runner.md"
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
    - "docs/test-specs/stage-runner-resume.md"
    - "docs/test-specs/job-immutability-provenance.md"
  references: []
  defers: []
  deferred-by: []
---

# run / resume 再現テストの実装

## Work

スタブステージを使った結合テストで、run → 途中失敗 → resume の
再現性と、同一設定の再実行が新ジョブ id を発行することを検証する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/test-specs/job-immutability-provenance.md`
- `docs/test-specs/stage-runner-resume.md`
- `docs/plans/implement-job-orchestration-foundation.md`

## Dependencies

- `docs/tasks/stage-runner.md`

## Affected Files

- `crates/al-job/tests/`(結合テスト)
- テスト用スタブステージ実装

## Done When

- [ ] A → B → C で B 失敗 → resume で B / C のみ再実行される
- [ ] 全成功後の resume が副作用なしで終了する
- [ ] 同一設定の再実行が別ジョブ id を発行する

## Verification

- [ ] `mise exec -- cargo test -p al-job` が通る
