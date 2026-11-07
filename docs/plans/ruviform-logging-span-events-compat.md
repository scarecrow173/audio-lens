---
id: "PLAN-SQWHGfXx2Hg0nBJWdUgw2A"
type: "plan"
status: "approved"
title: "SinkConfig::span_events 追従の実装計画 (AL#127)"
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
    - "../specs/ruviform-logging-span-events-compat.md"
    - "../designs/ruviform-logging-span-events-compat.md"
  implemented-by:
    - "../tasks/ruviform-logging-span-events-compat.md"
  depends-on:
    - "../designs/ruviform-logging-span-events-compat.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./ruviform-observability-logging-migration.md"
    - "https://github.com/scarecrow173/ruviform/issues/335"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/ruviform-logging-span-events-compat.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ruviform-logging-span-events-compat.md"
  references: []
  defers: []
  deferred-by: []
---

# SinkConfig::span_events 追従の実装計画 (AL#127)

## Goal

[設計](../designs/ruviform-logging-span-events-compat.md)
どおり、al-observability の stderr `SinkConfig` literal 構築へ
`span_events: SpanEvents::CLOSE` を設定して現公開契約へ適合
させ、lockfile を既定 branch 追従で最新 ruviform main へ更新
する。stderr span close 出力(migration 前挙動)を復旧し、
既存の logging 互換契約を全て保持する。

## Scope

- `Cargo.lock` ruviform resolved SHA を
  `node tools/update-ruviform-rev.mjs <sha>` で ruviform main
  `ad99d18` へ明示更新(追従契約、candidate 固定 pin なし)
- `crates/al-observability/src/lib.rs`: `SpanEvents` import +
  `span_events: SpanEvents::CLOSE` + module doc の #335
  regression 節を解消済みへ更新
- `crates/al-cli/tests/observability.rs`: stderr sink の span
  close record を外部観測する regression test 追加
- `ui/vendor` 再生成(`mise run sync-ui-deps`)と doc チェーン
  (spec/design/plan/test-spec/task/IR)+ 生成 index 再生成

## File Map

| Path | Change | Purpose |
| --- | --- | --- |
| `Cargo.lock` | modify | ruviform 17 crate の resolved SHA `78b4e7e -> ad99d18`(update-ruviform-rev.mjs 経由、手編集なし) |
| `crates/al-observability/src/lib.rs` | modify | stderr SinkConfig に `span_events: SpanEvents::CLOSE` + module doc 更新 |
| `crates/al-cli/tests/observability.rs` | modify | stderr span close 外部観測 regression test 追加 |
| `ui/vendor/` | generated | `sync-ui-deps` で lockfile SHA と同一 rev に再生成(gitignore 済み生成物) |
| `docs/specs|designs|plans|test-specs|tasks/ruviform-logging-span-events-compat.md` | add | 本チェーン |
| `docs/impl/ir/ruviform-logging-span-events-compat.md` | add | 実装記録(検証・SHA・残 gate) |
| `docs/*/README.md` | generated | doc-maintenance の生成 index 再生成 |

## Implementation Order

1. RED 再現: lockfile 更新 → `mise run check-ci` で E0063
   (exit 101)を確認・記録する(済)。
2. `lib.rs` の literal 構築へ field 追加 + module doc 更新。
3. regression test 追加 → 対象 test 単独で GREEN 確認。
4. `sync-ui-deps` → `check-ruviform-lock`/`check-ruviform-rev`
   → `verify-ci` → `check-ui`/`test-ui`/`check-cli-headless`
   → workflow 構契約 task(check-ci-workflow /
   check-ci-tracker-workflows)→ doc audit。
5. IR 作成・index 再生成・独立 review・commit/push/PR。

## Risk Register

| Risk | Mitigation |
| --- | --- |
| lockfile 追従で observability 以外の API 差分が混入 | verify-ci + golden で検証。非等値は PR に明示し独立 review で判断 |
| stderr close record が filter directive で落ちる | record は span の level/target 継承で既存 filter を通る挙動を regression test で固定 |
| vendor 再生成が network fetch に依存 | sync-ui-deps が正規手順。失敗時は guide の復旧表に従う |

## Verification Matrix

| 検証 | コマンド | 担保する契約 |
| --- | --- | --- |
| compile 適合 | `mise run check-ci` | R1(E0063 解消) |
| 全体 gate | `mise run verify-ci` | fmt + check + lint + test(ci profile) |
| stderr span close | `cargo test -p al-cli --test observability` | R2(復旧挙動)・R3(既存挙動群) |
| lock 単一 SHA | `mise run check-ruviform-lock` / `check-ruviform-rev` | R5 |
| UI vendor 整合 | `mise run check-ui` / `test-ui` | R5(vendor REV 一致) |
| headless 境界 | `mise run check-cli-headless` | 依存境界不変 |
| workflow 構造 | `mise run check-ci-workflow` / `check-ci-tracker-workflows` | 非対象だが hygiene として実行 |
| doc audit | `doc-status` lint/audit | チェーン整合 |
