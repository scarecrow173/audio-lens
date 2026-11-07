---
id: "DESIGN-17NiTSZp7kRH6pAW7EFwiO"
type: "design"
status: "approved"
title: "SinkConfig::span_events 追従設計 — stderr span close 復旧 (AL#127)"
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
  implements:
    - "../specs/ruviform-logging-span-events-compat.md"
  implemented-by: []
  depends-on:
    - "../adr/logging-sink-format-activation-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./ruviform-observability-logging-migration.md"
    - "./ruviform-lockfile-source-of-truth.md"
    - "https://github.com/scarecrow173/ruviform/issues/334"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/ruviform-logging-span-events-compat.md"
    - "./ruviform-observability-logging-migration.md"
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

# SinkConfig::span_events 追従設計 — stderr span close 復旧 (AL#127)

## Context

Issue [#127](https://github.com/scarecrow173/audio-lens/issues/127)
(常設 Ruviform 追従 Issue)。[spec](../specs/ruviform-logging-span-events-compat.md)
が記すとおり、ruviform #335 で `SinkConfig` に必須 field
`span_events: SpanEvents` が追加され、AL の literal 構築が
E0063 で compile 不能になった。

[migration 設計](./ruviform-observability-logging-migration.md)
はこの追従を「既知の一時 regression」として予約していた:

- migration 前実装(9f44174)は stderr/file 両 layer に
  `FmtSpan::CLOSE` を設定していた。
- AL#138 の委譲で ruviform adapter が `FmtSpan` を設定
  できず、stderr は span lifecycle event を失った
  (file sink は local layer で `FmtSpan::CLOSE` を維持)。
- 回復条件: 「ruviform #335 で span event 設定が提供された
  時点で stderr sink にも適用する」

`SpanEvents` は `tracing_subscriber::FmtSpan` の crate 固有
対応物(NEW/ENTER/EXIT/CLOSE を bit flag 合成、Default =
NONE で pre-#335 挙動)。CLOSE record は span 自身の
level/target を継承し、per-sink `EnvFilter` を通常 event と
同様に通る。

## Decision

**stderr `SinkConfig` に `span_events: SpanEvents::CLOSE` を
設定し、migration 前の stderr span close 出力を復旧する。
file sink の暫定 local 構築は変更しない。**

- `span_events: SpanEvents::CLOSE` は file 側 `FmtSpan::CLOSE`
  の写しではなく、migration 前 stderr の本来挙動の復旧値。
  NONE(Default)を選ぶと compile は通るが、設計が明記した
  回復条件を満たさず leaf span 観測(AL-OBS-01 要件)の
  暫定 regression を固定化してしまう。
- file sink は local layer(`RollingFileAppender` + suffix
  命名 + `non_blocking` + `FmtSpan::CLOSE`)を維持する。
  ruviform #334 の `filename_suffix` 公開により
  `SinkDestination::RollingFile` への移行は技術的に可能に
  なったが、rotation/lifetime/degradation 契約の移行検証を
  伴う別変更であり、本件の compile 互換修復とは責務が
  異なるため行わない(migration 設計の Deferred Design
  Concerns の独立項目として追跡継続)。
- 依存・公開 API・エラー語彙・`LogGuard` 形は変更しない。
  `InitError`/`Activation`/`LogConfig`/`resolve_activation`
  の契約は不変。

## Components and Boundaries

### `init`(所有境界での修復)

```text
build_logging_layers(&LoggingConfig {
    sinks: [SinkConfig {
        destination: Stderr,
        format: map_log_format(stderr_format),
        span_events: SpanEvents::CLOSE,   // 追加(唯一の変更点)
        filter: <directive>,
        failure_mode: Required,
    }]
})
```

修復位置は violation を所有する literal 構築点
(`crates/al-observability/src/lib.rs` の stderr sink 設定)
に限定する。型写像は `map_log_format` に倣い
`ruviform_observability::logging::SpanEvents` をそのまま使う
— AudioLens 語彙の再定義は製品側に span event 語彙の
公開面が存在しないため不要(YAGNI)。

### module doc 更新

lib.rs module doc の「既知の一時 regression(ruviform#335)」
節を解消済みとして書き換える: stderr sink は
`SinkConfig::span_events = SpanEvents::CLOSE` で span close
record を出力し、file JSONL sink は従来どおり local layer の
`FmtSpan::CLOSE` を持つ。#334 の暫定 file sink 節は維持。

## Data and Control Flow

変更なし(既存 flow を維持)。span_events は sink 構築時の
layer 設定であり、activation/優先順位/登録経路に影響しない。

## Risks and Trade-offs

- **stderr 出力増**: close record 行が stderr に復帰する。
  行単位 interleave 共存規約(ADR)の範囲内であり、
  `error:`/`warn:` plain 行や stdout JSON 契約に影響しない。
  record は span の level/target を継承するため filter
  directive で制御可能。
- **旧 rev 非互換**: `span_events` を持たない ruviform rev
  では compile 不能(E0560)。pre-release 方針で互換層は
  持たず、lockfile 追従と一体で扱う。
- **lockfile 追従の包含変更**: `78b4e7e -> ad99d18` は
  observability 以外の crate 差分(#334/#336/#280 等)も
  含む。AL が直接依存する面への影響は `check-ci`/`verify-ci`
  /golden で検証し、非等値が出れば PR に明示する。

## Verification Notes

- RED: lockfile `ad99d18` 化後の `mise run check-ci` が
  E0063 で exit 101(再現済み)。
- GREEN: `verify-ci` + `check-ruviform-lock`/`rev` + UI lane
  + observability golden が pass。
- stderr span close の外部観測 regression test を
  `crates/al-cli/tests/observability.rs` に追加
  (file 側 `file_sink_emits_span_close_events` と同型の
  外部挙動検証。field 名の鏡写し test ではない)。
