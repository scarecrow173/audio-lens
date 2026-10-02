---
id: "TASK-034z0J8zzSGVEuhTR3yHOM"
type: "task"
status: "todo"
title: "合成 fixture による ingest 結合テスト"
created: "2026-10-02"
updated: "2026-10-02"
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
    - "docs/plans/implement-ingest-av.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-ingest-av.md"
    - "docs/tasks/ingest-stage.md"
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
    - "docs/test-specs/ingest-wav-decode.md"
    - "docs/test-specs/av-timing-observation.md"
  references: []
  defers: []
  deferred-by: []
---

# 合成 fixture による ingest 結合テスト

## Work

合成 WAV(プログラム生成)と合成 VFR タイムスタンプ系列で、
ingest → manifest 登録 → resume の結合テストを実装する。
未対応形式・破損ファイルの明示的失敗もカバーする。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-ingest-av.md`
- `docs/test-specs/ingest-wav-decode.md`
- `docs/test-specs/av-timing-observation.md`

## Dependencies

- `docs/tasks/ingest-stage.md`

## Affected Files

- `crates/al-media/tests/`

## Done When

- [ ] 合成 WAV の run → resume → manifest 検証が通る
- [ ] 同一入力の再現性(ハッシュ一致)が確認される
- [ ] 未対応形式・破損入力が明示的失敗する
- [ ] VFR 系列の観測が辞書注釈と分離して記録される

## Verification

- [ ] `cargo test --workspace` が通る
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` が通る
