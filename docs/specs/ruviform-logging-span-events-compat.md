---
id: "SPEC-GAQPlJtIjsPj8TdfBwrfIw"
type: "spec"
status: "approved"
title: "al-observability の SinkConfig::span_events 追従 (AL#127 / Ruviform #335)"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/335"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../designs/ruviform-observability-logging-migration.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "ruviform-observability-logging-migration.md"
    - "observability-foundation.md"
    - "ruviform-lockfile-source-of-truth.md"
    - "../adr/logging-sink-format-activation-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/ruviform-observability-logging-migration.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ruviform-logging-span-events-compat.md"
  references:
    - "https://github.com/scarecrow173/ruviform/pull/348"
    - "https://github.com/scarecrow173/ruviform/actions/runs/42302538464"
    - "https://github.com/scarecrow173/ruviform/issues/334"
  defers: []
  deferred-by: []
---

# al-observability の SinkConfig::span_events 追従 (AL#127 / Ruviform #335)

## What Should Be Built

`crates/al-observability` の `init` が literal 構築する
ruviform `SinkConfig` を、ruviform #335(PR
[#348](https://github.com/scarecrow173/ruviform/pull/348)、
`33d93dd`)で追加された必須 field `span_events: SpanEvents`
を含む現公開契約へ適合させる。併せて `Cargo.lock` の ruviform
resolved SHA を追従契約(既定 branch 追従・lockfile 正本)に
従い最新 main へ明示更新する。

これは AL#127(Ruviform 追従の常設 tracking Issue)に対応する
互換修復であり、追跡済みの scope 外既存 Consumer 追従漏れの
解消である。Ruviform PR #392(workflow/evidence/docs 変更)は
この field を導入していない。

## Why It's Needed

consumer-verify run
[42302538464](https://github.com/scarecrow173/ruviform/actions/runs/42302538464)
(candidate `30205f2b` + AL main `04f1861` + SV main `d8bf8ca`)
で AL check-ci lane が `E0063: missing field 'span_events' in
initializer of 'SinkConfig'`(crates/al-observability/src/lib.rs:264)
で失敗し、AL 後続 lane が skipped・evidence fail・summary fail
となった。preflight・SHA 照合・SV 全 lane は pass であり、
失敗原因は AL 側の literal 構築が新必須 field を持たない
ことのみである。local 再現: lockfile を ruviform main
`ad99d18` へ更新後 `mise run check-ci` が同一 E0063 で
exit 101(Windows / rust 1.99.0)。

## Requirements

- **R1**: `SinkConfig` literal 構築に `span_events` field を
  設定し、現公開契約で compile 可能にする。
- **R2**: stderr sink の `span_events` 値は、file sink の
  `FmtSpan::CLOSE` の写しや逆値の推測で決めない。
  コード/履歴/設計から特定した stderr の本来挙動 —
  migration 前実装(`9f44174`)では stderr layer にも
  `FmtSpan::CLOSE` が設定されていた — を復旧する
  `SpanEvents::CLOSE` とする。migration 設計の回復条件
  「ruviform #335 で span event 設定が提供された時点で
  stderr sink にも適用する」が成立したため。
- **R3**: 既存契約を保持する — stderr/file の filter・
  format 切替、stdout 保護(sink 語彙閉集合・inspect JSON
  契約)、`AL_LOG` 既定 OFF、file の `al.YYYY-MM-DD.log`
  命名・daily rotation・`max_log_files(7)` 保持・init 失敗
  warn+degrade、`init` 1回きり・`AlreadyInitialized`。
- **R4**: file sink は暫定 local 構構築を維持する
  (ruviform #334 の `RollingFile` 移行は別の deferred
  concern であり本件では行わない)。新 Logger 実装・別
  Toolkit/GUI 移行・Ruviform 側変更で compile 穴を隠さない。
- **R5**: `Cargo.lock` の ruviform resolved SHA は既定
  branch 追従契約に従い `update-ruviform-rev.mjs` で最新
  main へ更新し、`check-ruviform-lock` の単一 SHA 契約と
  `ui/vendor` の REV 整合を維持する。特定 candidate SHA
  への恒久 pin・依存凍結は行わない。新規 crate 追加なし。

## Acceptance Criteria

- lockfile 更新後 rev(`ad99d18`)で `mise run check-ci` /
  `mise run verify-ci` が green(E0063 解消)。
- `run --log info` の stderr に span close record
  (`time.busy`/`time.idle` 付き)が出力され、file JSONL 側の
  close 出力・既存 golden(test-spec 準拠)が不変。
- `mise run check-ruviform-lock` / `check-ruviform-rev` が
  単一 SHA + vendor REV 一致で pass。
- consumer-verify 向け: 修復 head が candidate rev
  (`30205f2b` 相当: `span_events` を含む任意の ruviform rev)
  で compile 可能。`span_events` を持たない旧 rev への
  後方互換は本プロジェクトの pre-release 方針により持たない
  (成果物は再生成する方針、AGENTS.md 開発原則参照)。

## Out of Scope

- file sink の `SinkDestination::RollingFile` 移行(ruviform
  #334 は解消済みだが、migration は別タスクの deferred
  concern)。
- Ruviform 側の変更・PR #392 workflow 修復(別 worker 担当)。
- GUI プロセスの logging 配線・LoggingHealth 公開。
