---
id: "TASK-HrjjahBSYbViCD72MweZwV"
type: "task"
status: "done"
title: "al-observability SinkConfig::span_events 追従 (AL#127)"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/ruviform-logging-span-events-compat.md"
    - "../designs/ruviform-logging-span-events-compat.md"
  implemented-by: []
  depends-on:
    - "../designs/ruviform-logging-span-events-compat.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "ruviform-observability-logging-migration.md"
    - "https://github.com/scarecrow173/ruviform/issues/335"
    - "https://github.com/scarecrow173/ruviform/issues/334"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/ruviform-logging-span-events-compat.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ruviform-logging-span-events-compat.md"
  references:
    - "https://github.com/scarecrow173/ruviform/pull/348"
    - "https://github.com/scarecrow173/ruviform/actions/runs/42302538464"
  defers: []
  deferred-by: []
---

# al-observability SinkConfig::span_events 追従 (AL#127)

## Work

Issue [#127](https://github.com/scarecrow173/audio-lens/issues/127)
の追従対象として、[設計](../designs/ruviform-logging-span-events-compat.md)
どおり `SinkConfig` literal 構築を現公開契約へ適合させる。

1. **lockfile 追従**(済): `node tools/update-ruviform-rev.mjs
   ad99d181c34cded1e56489fa9542027fe0dfc966` で ruviform
   全 crate を main HEAD `ad99d18` へ更新。
   `mise run check-ci` が E0063(exit 101)で RED 再現済み。
2. **`init` 修復**: stderr `SinkConfig` に
   `span_events: SpanEvents::CLOSE` を追加(migration 前
   stderr 挙動の復旧。file sink 暫定 local 構築は不変)。
   module doc の #335 regression 節を解消済みへ更新。
3. **regression test**: `crates/al-cli/tests/observability.rs`
   に stderr span close record の外部観測 test を追加。
4. **再生成・検証**: `sync-ui-deps` で vendor 整合、
   `verify-ci` / `check-ruviform-lock` / `check-ruviform-rev` /
   `check-ui` / `test-ui` / `check-cli-headless` /
   `check-ci-workflow` / `check-ci-tracker-workflows`、
   対象 observability test、doc audit。
5. **IR・PR**: `docs/impl/ir/ruviform-logging-span-events-compat.md`
   に検証結果・SHA・残 gate を記録し、`Refs #127` の PR を
   作成(常設 Issue は Close しない)。

## Classification

- `normal-plan-task`

## Upstream Documents

- [docs/specs/ruviform-logging-span-events-compat.md](../specs/ruviform-logging-span-events-compat.md)(本件要求)
- [docs/designs/ruviform-logging-span-events-compat.md](../designs/ruviform-logging-span-events-compat.md)(approved)
- [docs/plans/ruviform-logging-span-events-compat.md](../plans/ruviform-logging-span-events-compat.md)(approved)
- [docs/test-specs/ruviform-logging-span-events-compat.md](../test-specs/ruviform-logging-span-events-compat.md)(検証基準)
- [docs/designs/ruviform-observability-logging-migration.md](../designs/ruviform-observability-logging-migration.md)(回復条件の正本)
- [docs/guides/ruviform-lockfile-update.md](../guides/ruviform-lockfile-update.md)(lockfile 追従手順)

## Dependencies

- `ruviform-observability` @ ruviform main `ad99d18`
  (feature `logging`。`SpanEvents` は #335/PR #348 で追加)
- 新規 crate・feature 追加なし

## Affected Files

- `Cargo.lock`(update-ruviform-rev.mjs 経由)
- `crates/al-observability/src/lib.rs`
- `crates/al-cli/tests/observability.rs`
- `ui/vendor/`(生成物)
- `docs/specs|designs|plans|test-specs|tasks/ruviform-logging-span-events-compat.md`
- `docs/impl/ir/ruviform-logging-span-events-compat.md`

## Done When

- [x] `mise run check-ci`/`verify-ci` が lockfile `ad99d18`
      で green(E0063 解消)
- [x] stderr span close の regression test を含む
      observability test 群が pass、既存 golden 不変
- [x] `check-ruviform-lock`/`check-ruviform-rev` が単一 SHA
      `ad99d18` + vendor REV 一致で pass
- [x] IR に実行 command/OS/toolchain/exit/sourceSHA/lockSHA
      と残 gate を記録
