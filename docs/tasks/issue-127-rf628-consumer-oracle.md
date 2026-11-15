---
id: "TASK-034Y19LB0yL31DZhSVjAbk"
type: "task"
status: "in-progress"
title: "Issue #127 RF#628 consumer oracle: 実装タスク"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/628"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-127-rf628-consumer-oracle.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-127-rf628-consumer-oracle.md"
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
    - "docs/test-specs/issue-127-rf628-consumer-oracle.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #127 RF#628 consumer oracle: 実装タスク

## Work

`ruviform-audio-test`（RF#628 deterministic PCM fixture/oracle）を
file-based 解析の独立 consumer oracle として `crates/al-analysis`
に実装する slice。pin を `ac10a2f7` →
`abb9f372d3ee95ee824b3c4f7dc68e8083ce2774` へ lockfile-primary で
bump し、AL-1（f64 fidelity）・AL-2（長尺非回帰）・AL-3
（gap/Validity）の 3 脚を `tests/rf628_oracle.rs` に実装する。
AL-4（clock drift）は Deferred、AL-5（device-pipeline legs）は
NotApplicable として記録する。文書セットは spec/design/plan/task/
test-spec/IR の 6 点 + rev-watch 5 点 + exp 記録。

| # | 内容 | 状態 |
| --- | --- | --- |
| 1 | ruviform main 再解決（`abb9f372`、`src/pipeline/` 存在確認） | 完了 |
| 2 | `ruviform-audio-test` を root `[workspace.dependencies]` + al-analysis `[dev-dependencies]` に追加 | 完了 |
| 3 | `update-ruviform-rev.mjs` → `sync-ui-deps` → lock/rev check Pass（26 package 単一 SHA） | 完了 |
| 4 | `ruviform-audio-core` を al-analysis dev-deps に追加（`SampleRate` 用） | 未着手 |
| 5 | `tests/rf628_oracle.rs` 実装（AL-1/AL-2/AL-3） | 未着手 |
| 6 | rev-watch 5 点 + IR/exp 記録 | 未着手 |
| 7 | index 再生成 + 全検証 lane | 未着手 |
| 8 | review-protocol → commit/push/PR（`Refs #127`） | 未着手 |

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-127-rf628-consumer-oracle.md`
- `docs/specs/issue-127-rf628-consumer-oracle.md`
- `docs/designs/issue-127-rf628-consumer-oracle.md`
- `docs/test-specs/issue-127-rf628-consumer-oracle.md`

## Dependencies

- ruviform private repo への system git 認証（`git-fetch-with-cli`）
- 前提 pin: `ac10a2f7`（PR #213 系で採用済みの直前 pin）

## Affected Files

- `Cargo.toml` / `Cargo.lock` — ruviform pin + dev-dep
- `crates/al-analysis/Cargo.toml` — dev-deps
- `crates/al-analysis/tests/rf628_oracle.rs` — 新規テスト
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/`
  の本 slice 文書と index

## Done When

- [ ] AL-1/AL-2/AL-3 テストが存在し `mise run test al-analysis` で Pass
- [ ] lockfile が単一 SHA `abb9f372` に解決され lock/rev check Pass
- [ ] AL-4 Deferred・AL-5 NotApplicable が test-spec/PR に記録済み
- [ ] rev-watch 5 点に採用判断・差分概要・rollback が記録済み
- [ ] 全検証 lane 実行・記録、PR 作成（`Refs #127`）、Issue は open

## Verification

- `mise run check-ruviform-lock` / `check-ruviform-rev`:
  Pass（実行済み、26 package 単一 SHA）
- `mise run fmt` / `check al-analysis` / `lint al-analysis` /
  `test al-analysis` / `check-cli-headless` / `prek run --all-files`:
  未実行（テスト実装後に実施）
- 詳細は `docs/test-specs/issue-127-rf628-consumer-oracle.md` を正本とする
