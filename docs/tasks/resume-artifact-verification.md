---
id: "TASK-034GsRMzsYQltCJEffwBfb"
type: "task"
status: "done"
title: "resume-artifact-verification"
created: "2026-10-15"
updated: "2026-10-15"
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
    - "docs/plans/implement-artifact-durability-tiers.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-artifact-durability-tiers.md"
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
    - "docs/test-specs/durability-tiers-resume-verification.md"
  references: []
  defers: []
  deferred-by: []
---

# resume-artifact-verification

## Work

`StageRunner::resume` に Completed/Skipped stage の artifact 実在検証を 追加する。ファイルは実在+`params.byte_len` 一致、ディレクトリは `params.files` 列挙の全実在。失敗時は Pending 降格+artifact レコード除去+manifest save。既存 resume.rs の StubStage は path 登録のみでファイルを 書かないため、実ファイル書込に修正する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-artifact-durability-tiers.md`
- `docs/designs/artifact-durability-tiers-resume-verification.md`

## Dependencies

- なし(T1 と並行可能)

## Affected Files

- `crates/al-job/src/runner.rs`
- `crates/al-job/tests/resume.rs`

## Done When

- [ ] artifact 欠落/byte_len 不一致/files 欠落で stage が Pending に
      降格し再実行される
- [ ] 降格で除去した artifact レコードが manifest.artifacts から消える
- [ ] 全 artifact 実在の resume は no-op

## Verification

- [ ] `cargo test -p al-job` がパスする
- [ ] 欠落/切り詰め/dir-files の3ケースの新テストがある
