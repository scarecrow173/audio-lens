---
id: "TSPEC-YLw9YwXZe7FPiRD7fbuVwg"
type: "test-spec"
status: "approved"
title: "ruviform-observability logging 委譲の検証 (AL-OBS-01)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/138"
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
    - "observability-foundation.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/ruviform-observability-logging-migration.md"
    - "../plans/ruviform-observability-logging-migration.md"
  derived-by: []
  verifies:
    - "../specs/ruviform-observability-logging-migration.md"
    - "../designs/ruviform-observability-logging-migration.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/334"
    - "https://github.com/scarecrow173/ruviform/issues/335"
  defers: []
  deferred-by: []
---

# ruviform-observability logging 委譲の検証 (AL-OBS-01)

## Purpose

Issue #138 の委譲後も、AudioLens の logging 互換契約(既定
OFF・優先順位・sink/format・stdout 保護・daily 7 世代命名・
error/degrade)が golden 挙動どおり維持されることを保証する。
検証対象は [設計](../designs/ruviform-observability-logging-migration.md)
/ [計画](../plans/ruviform-observability-logging-migration.md)
の契約と [ADR](../adr/logging-sink-format-activation-contract.md)
の Confirmation 項。

## Feature

`al` CLI の logging 有効化・sink routing・ファイル命名/
retention・失敗時 degrade の外部観測挙動

## Rules

- Rule: `AL_LOG`/`--log` 未指定(および `AL_LOG` 空文字)では
  stderr に tracing 行が出ず `<base_dir>/logs` が生成されない。
  `--log-format` 単独でも有効化されない。
- Rule: `--log <directive>` または `AL_LOG` で有効化され、
  stderr(text)と `<base_dir>/logs/al.YYYY-MM-DD.log`
  (JSON-lines)の両 sink に stage event が出力される。
- Rule: flag は env を per-concern 全体置換する(`--log` は
  `AL_LOG` を、`--log-format` は `AL_LOG_FORMAT` を)。壊れた
  env directive + 有効な flag で run が成功する。
- Rule: `--log-format`/`AL_LOG_FORMAT` で sink ごとの format
  が独立切替(`stderr=json`・`file=text`)し、未指定 sink は
  既定値(stderr=text・file=json)を保つ。
- Rule: `al inspect` は `--log` 有効時でも stdout が有効 JSON
  のままで `<base_dir>/logs` を作らない。
- Rule: file 名は `al.YYYY-MM-DD.log` 形式で、保持は最大
  7 世代(超過分は古い順に削除される)。
- Rule: `<base_dir>/logs` を生成できない環境(同名の通常
  file 占拠等)では stderr に `warn: cannot initialize file
  log sink` を出して stderr-only に degrade し、job は成功する。
- Rule: 値なし `--log`/`--log-format`・空文字 flag・未知
  sink(`stdout` を含む)・未知 format・壊れた directive は
  `error:` 行 + exit 2 で fail-fast し panic しない。
  `al --log x run` は unknown subcommand のまま。
- Rule: file sink の JSON-lines に、内部 event を持たない
  span の close event(span lifecycle)が残る(leaf span
  観測の暫定担保。stderr sink は ruviform #335 解消まで
  close event を出さない既知 regression)。
- Rule: `init` の2回目呼出しは `AlreadyInitialized` で失敗し、
  global subscriber 登録は `try_init` 1箇所に限定される。
- Rule: logging ON/OFF で `run` の manifest・成果物・f64
  解析値が同一である。
- Rule: metadata-only 規約(field < 1 KiB)の回帰 test が
  引き続き PASS する。

## Examples

- `run --log info` → stderr に `stage completed`、
  `logs/al.YYYY-MM-DD.log` に `"stage":"ingest"` +
  `duration_ms` を含む JSON 行。
- `logs/` 配下に `al.2020-01-0*.log` 9 file を事前配置して
  `run --log info` → 実行後 `al.*.log` は最大 7 件に刈られる。
- `<base_dir>/logs` を通常 file として作成 → `run --log info`
  が exit 0 で stderr に `warn: cannot initialize file log sink`
  を含む。
- `AL_LOG="=info"` + `--log info` → flag 優先で run 成功
  (env の壊れた directive は評価されない)。
- `run --log info --log-format stderr=json` → stderr の各
  tracing 行が JSON として parse 可能(ただし `fail()`/
  `warn:` の plain 行は対象外)。
- `al-observability` unit test: `init` 2回目が
  `Err(AlreadyInitialized)`。
