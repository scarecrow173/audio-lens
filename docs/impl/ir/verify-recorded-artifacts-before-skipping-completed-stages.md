---
id: "IMPL-034GthAYnQ02kqeLlH7e6vxx"
type: "impl"
status: "completed"
title: "Verify recorded artifacts before skipping completed stages"
created: "2026-10-15"
updated: "2026-10-15"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "code"
        path: "crates/al-job/src/runner.rs"
      - type: "code"
        path: "crates/al-job/tests/resume.rs"
      - type: "code"
        path: "crates/al-media/tests/ingest.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/resume-artifact-verification.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/artifact-durability-tiers-resume-verification.md"
    - "docs/specs/durability-tiered-artifact-writes.md"
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

# Verify recorded artifacts before skipping completed stages

## Summary

`StageRunner::resume` が `StageStatus::Completed` / `Skipped` を無条件に
信用していたのを改め、manifest に記録された artifact の実在検証を
追加した。検証に失敗した stage は Pending に降格し、自身の artifact
記録を manifest から除去して durable に保存した上で再実行される。

## Context

- task: `docs/tasks/resume-artifact-verification.md`
- design: `docs/designs/artifact-durability-tiers-resume-verification.md`
- spec: `docs/specs/durability-tiered-artifact-writes.md`

relaxed 等級の non-durable 書込み導入により、Completed でも成果物が
失われ得るようになったため、resume 境界での存在検証が必須。

## Implementation

### Changes

- 検証は `stage.artifacts` に記録された artifact のみを対象とし、
  `checked_job_path` で job dir 外参照を拒否する。空 path の入力
  宣言は検証しない。
- file artifact: 存在 + regular file を要求。`params.byte_len` が
  あれば実サイズと一致することを要求。
- dir artifact: `params.files` の相対パス全件の存在を要求。
- 失敗時は当該 stage のみ Pending 降格、artifact 参照・error・
  duration をクリアし、stale な ArtifactRecord を manifest から除去。
  下流 stage は自身の artifact が検証を通る限り維持される
  (lineage の dangling 参照は spec で許容済み)。
- 降格があれば durable manifest 経路で保存してから実行に進む。

### Decisions During Implementation

- `byte_len`/`files` を持たない既存 manifest は存在検証のみに
  フォールバック(pre-release 方針、互換マイグレーションなし)。
- 既存テストの stub stage が path を登録しつつファイルを書かない
  状態だったため、`crates/al-job/tests/resume.rs` と
  `crates/al-media/tests/ingest.rs` の stub を実ファイル書込に修正。
  これは新契約(Completed は記録 artifact の実在を要求)への
  整合であって検証の緩和ではない。

## Related Experiments

### Adopted

- なし。

### Rejected

- なし。

## Validation

- `cargo test -p al-job` — resume 系 17 件 pass(欠落・byte_len
  切詰め・dir member 欠落の再実行テストを含む)。
- `cargo test -p al-media` — ingest resume テスト pass。
- `cargo test --workspace` — 42 バイナリ全件 pass、0 失敗。

## Risks

- 欠落検出で降格した stage の artifact を参照する下流 Completed
  stage の lineage は dangling 参照になり得る。spec で許容済み、
  `lineage_of` は欠損レコードを静かに skip する。

## Follow-ups

- なし。
