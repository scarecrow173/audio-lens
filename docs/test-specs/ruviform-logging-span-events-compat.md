---
id: "TSPEC-Jpr62rfkrRNiHtvEJn74bF"
type: "test-spec"
status: "approved"
title: "SinkConfig::span_events 追従の検証 (AL#127)"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "ruviform-observability-logging-migration.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/ruviform-logging-span-events-compat.md"
    - "../plans/ruviform-logging-span-events-compat.md"
  derived-by: []
  verifies:
    - "../specs/ruviform-logging-span-events-compat.md"
    - "../designs/ruviform-logging-span-events-compat.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/335"
    - "https://github.com/scarecrow173/ruviform/actions/runs/42302538464"
  defers: []
  deferred-by: []
---

# SinkConfig::span_events 追従の検証 (AL#127)

## Purpose

al-observability が ruviform `SinkConfig` の現公開契約
(`span_events` 必須 field)で compile し、migration 前挙動
だった stderr の span close record 出力を復旧しつつ、
[既存 test-spec](./ruviform-observability-logging-migration.md)
と [ADR](../adr/logging-sink-format-activation-contract.md)
の logging 互換契約が全て維持されることを保証する。
暫定 regression とされた「stderr は close event を出さない」
規定は本変更で取り消される。

## Feature

`al` CLI logging の stderr sink が、有効化時に span close
record(span 自身の level/target・`time.busy`/`time.idle`
field 付き)を出力すること。型境界では `SinkConfig`
literal 構築が現公開契約で compile 可能であること
(field 名の写しではなく、契約適合の型検査として
`check-ci` が担保)。

## Rules

- Rule: `run --log info` の stderr に span close record が
  出力される(`time.busy` を含む close 行)。migration 前
  実装が両 sink に設定していた `FmtSpan::CLOSE` 相当の復旧
  であり、file JSONL 側の close 出力規約は不変。
- Rule: close record は span 自身の level/target を継承して
  per-sink `EnvFilter` を通るため、`--log` directive で
  絞った span は record も絞られる(既存 filter 契約の
  一部であり、新規の制御語彙は増やさない)。
- Rule: 既存契約の非 regress — 既定 OFF・stdout JSON 保護・
  flag>env 全体置換・format 独立切替・`al.YYYY-MM-DD.log`
  daily/7 世代・file init 失敗 degrade・fail-fast error 語彙・
  `init` 1回きり・metadata-only 規約は
  [migration test-spec](./ruviform-observability-logging-migration.md)
  の Rules をそのまま検証対象とする。

## Examples

- `run --log info` → stderr に `close`/`time.busy` を含む
  span close 行が現れ、`logs/al.YYYY-MM-DD.log` にも従来
  どおり `"time.busy"` を含む close record が残る。
- `run`(logging 無効)→ stderr close record も含め tracing
  行は出ない(既定 OFF 不変)。
- `inspect --log info` → stdout は有効 JSON のまま、
  stderr sink のみ close record を出力する。

## Verification Commands

- `mise run check-ci`(RED: 更新後 rev で E0063 → 修復後
  exit 0)
- `cargo test -p al-cli --test observability`(標的 regression)
- `mise run verify-ci`、`mise run check-ruviform-lock`、
  `mise run check-ruviform-rev`、`mise run check-ui`、
  `mise run test-ui`、`mise run check-cli-headless`
