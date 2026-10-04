---
id: "TASK-034AaFByhnoqebVdh0M46q"
type: "task"
status: "done"
title: "video/A-V 結合テスト(合成 AVI)"
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
    - "docs/plans/video-av-features.md"
  implemented-by: []
  depends-on:
    - "docs/plans/video-av-features.md"
    - "docs/tasks/video-features-ingest.md"
    - "docs/tasks/avsync-analysis.md"
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
    - "docs/test-specs/video-av-features.md"
  references: []
  defers: []
  deferred-by: []
---

# video/A-V 結合テスト(合成 AVI)

## Work

シーンチェンジ・輝度変化・既知 A/V オフセット(PCM パディング型と `dwStart≠0` 型の両方)を持つ合成 AVI fixture を作り、ingest→cheap→avsync の E2E を検証する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/video-av-features.md`
- `docs/test-specs/video-av-features.md`

## Dependencies

- `docs/tasks/video-features-ingest.md`
- `docs/tasks/avsync-analysis.md`

## Affected Files

- `crates/al-media/tests/`
- `crates/al-analysis/tests/`
- `crates/al-cli/tests/cli.rs`

## Done When

- [ ] 合成 AVI のシーンチェンジが特徴系列に記録される
- [ ] 既知 A/V オフセットが delta として回復される

## Verification

- [ ] `cargo test -p al-media`
- [ ] `cargo test -p al-analysis`
- [ ] `cargo test -p al-cli`
