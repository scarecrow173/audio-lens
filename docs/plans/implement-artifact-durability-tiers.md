---
id: "PLAN-034GsQ4Xf3tqGpUXx6bUK2"
type: "plan"
status: "approved"
title: "Implement artifact durability tiers and resume verification"
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
    - "docs/specs/durability-tiered-artifact-writes.md"
    - "docs/adr/artifact-durability-tiers-relaxed-bulk-writes.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/artifact-durability-tiers-resume-verification.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/durability-tiers-resume-verification.md"
  references: []
  defers: []
  deferred-by: []
---

# Implement artifact durability tiers and resume verification

## Goal

fsutil に `write_atomic`/`rename_atomic` を追加して bulk データ書込
を relaxed tier に切り替え、`StageRunner::resume` に Completed
stage の artifact 実在検証を入れて欠落 stage を再実行へ降格する。

## Scope

単一実装ストリーム。al-job(fsutil + runner 検証)→ producer の
等級切替と params 記録(al-media/al-locate/al-dataset)の順に
依存するため、テストスペックと3タスクに分割する。

## Upstream Documents

- `docs/specs/durability-tiered-artifact-writes.md`
- `docs/adr/artifact-durability-tiers-relaxed-bulk-writes.md`

## Design Inputs

- `docs/designs/artifact-durability-tiers-resume-verification.md`

## File Map

- `crates/al-job/src/fsutil.rs` — `write_atomic`/`rename_atomic` 追加、
  `write_durable` と tmp/cleanup 規律を共有
- `crates/al-job/src/runner.rs` — resume 時の artifact 実在検証と
  Completed → Pending 降格
- `crates/al-job/tests/resume.rs` — 検証テスト追加、StubStage を
  実ファイル書込に修正
- `crates/al-media/src/stage.rs` — decoded.f64le を `write_atomic`、
  params に byte_len
- `crates/al-locate/src/normalize.rs` — normalized PCM を
  `write_atomic`、params に byte_len
- `crates/al-dataset/src/table.rs` — chunk を `write_atomic`
- `crates/al-dataset/src/tensor.rs` — chunk を `write_atomic`
- `crates/al-dataset/src/audio.rs` — sync_all 除去 + `rename_atomic`
- `crates/al-dataset/src/stage.rs` — `analysis_dataset` record に
  `params.files` 列挙

## Implementation Sequence

- [ ] T1: fsutil に `write_atomic`/`rename_atomic` + 単体テスト
- [ ] T2: runner resume 検証(存在/byte_len/files)+ 降格 +
      resume.rs の stub 修正と新テスト
- [ ] T3: producer 等級切替(relaxed 経路へ)と params メタ記録

## Dependencies and Risks

- T2 の stub 修正は既存 resume テストの期待値に影響する
  (Completed stage の artifact が実在する契約になる)。
- T3 は T1 に依存(fsutil API)。T2 は T1 と独立で先行可能。
- `analysis_dataset` の `files` 列挙は `writer.finish()` 後の
  `analysis/` 再帰走査で生成する(chunk 数千でも stat 不要、
  列挙のみ)。

## Task Breakdown

- bundled test-spec: `durability-tiers-resume-verification`
- T1 `fsutil-atomic-write-primitives` — write_atomic/rename_atomic
- T2 `resume-artifact-verification` — runner 検証 + 降格 + テスト
- T3 `relaxed-tier-callsite-migration` — 等級切替 + params 記録

## Verification

- [ ] `cargo test --workspace` 全件パス
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo fmt --all -- --check` clean
- [ ] artifact 欠落/切り詰め → resume で stage 再実行のテスト
- [ ] `audit_docs.js --type all` no findings

## Open Questions / Gaps

- なし(design の未決項は spec/ADR で確定済み)。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: T2 は T1 と並行可能、T3 は T1 依存
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- relaxed 書込が fsync を呼ばないこと、記録系が write_durable を
  維持すること、降格で除去した artifact id が下流 lineage に
  dangling 参照として残る許容範囲を重点確認。
