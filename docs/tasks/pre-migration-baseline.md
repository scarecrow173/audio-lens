---
id: "TASK-034P51k4FtjAZkVTMWcZPl"
type: "task"
status: "in-progress"
title: "pre-migration baseline の実装と実測"
created: "2026-10-30"
updated: "2026-10-31"
owners:
  - "scarecrow173"
relations:
  source:
    - "../plans/implement-pre-migration-baseline.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-pre-migration-baseline.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-pre-migration-baseline.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/pre-migration-baseline.md"
    - "../designs/pre-migration-baseline.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/pre-migration-baseline.md"
  references: []
  defers: []
  deferred-by: []
---

# pre-migration baseline の実装と実測

## Work

`docs/designs/pre-migration-baseline.md` の通り、
`tools/baseline/`(capture/normalize/store/procmon/compare)
と `baseline/` 成果物一式を実装し、run matrix
(canonical + variants×5 + small + resume + aa)を実測して
Issue #98 の baseline を固定する。coverage 台帳で追加が
必要と判定した最小 guard テストのみ `crates/al-*/tests/`
に増補する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-pre-migration-baseline.md`
- `docs/designs/pre-migration-baseline.md`
- `docs/adr/pre-migration-baseline.md`

## Dependencies

- release build(`cargo build --release`)済みであること
- ffmpeg が PATH で利用可能であること(版数は記録)

## Affected Files

- `tools/baseline/`(新規)
- `baseline/`(新規・commit)
- `crates/al-analysis/examples/bench_alloc.rs`(新規)
- `mise.toml`(`baseline` task)
- `crates/al-*/tests/`(coverage 判定分のみ)

## Done When

- [ ] `mise run baseline` で run matrix が実行され
  `baseline/` に成果物一式が生成される
- [ ] `compare.mjs` の canonical vs aa・resume vs canonical
  が契約通りに評価される
- [ ] `compare.mjs --self-test` が意図差分を検出する
- [ ] `baseline/coverage.json` が Issue #98 列挙を全て
  割り当て済み
- [ ] `mise run test` / `lint` / `fmt` が green

## Verification

- [ ] 上記 Done When の全項目を実コマンドで確認
- [ ] spec の Acceptance Criteria を満たす
