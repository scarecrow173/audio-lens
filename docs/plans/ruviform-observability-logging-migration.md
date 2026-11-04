---
id: "PLAN-9PUZEliT1Z1FgKwKE0LHmP"
type: "plan"
status: "approved"
title: "al-observability logging の ruviform-observability 委譲実装計画 (AL-OBS-01)"
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
  implements:
    - "../specs/ruviform-observability-logging-migration.md"
    - "../designs/ruviform-observability-logging-migration.md"
  implemented-by:
    - "../tasks/ruviform-observability-logging-migration.md"
  depends-on:
    - "../designs/ruviform-observability-logging-migration.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./observability-foundation.md"
    - "https://github.com/scarecrow173/ruviform/issues/334"
    - "https://github.com/scarecrow173/ruviform/issues/335"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/ruviform-observability-logging-migration.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ruviform-observability-logging-migration.md"
  references: []
  defers: []
  deferred-by: []
---

# al-observability logging の ruviform-observability 委譲実装計画 (AL-OBS-01)

## Goal

[設計](../designs/ruviform-observability-logging-migration.md)
の契約を、Issue [#138](https://github.com/scarecrow173/audio-lens/issues/138)
の実装順序 (a)–(d) に沿って実装する。golden test で現行挙動を
固定してから stderr sink を ruviform `logging` feature へ委譲し、
file sink は #334 解消まで暫定 local 構築を維持する。

## Scope

- golden test 拡充: retention(7 世代)、file init 失敗
  degrade、format E2E(file=text/stderr=json)、flag>env
  優先、`AL_LOG` 空文字 OFF、file sink の span close event
  観測(暫定期間の leaf span 観測担保)
- `ruviform-observability`(feature `logging`)の依存追加と
  `al-observability::init` の委譲実装
- `LogGuard`/`InitError` の内部表現更新(公開 API 形は維持)
- 検証: verify-ci、golden parity、logging ON/OFF 解析結果
  同一性、`cargo tree` 依存境界、`check-ruviform-lock`

## File Map

| Path | Change | Purpose |
| --- | --- | --- |
| `Cargo.toml` | modify | `ruviform-observability` workspace dep 追加(feature `logging`) |
| `Cargo.lock` | modify | `cargo check` 再解決(ruviform-observability 追加、同一 SHA) |
| `crates/al-observability/Cargo.toml` | modify | `ruviform-observability` dep 追加 |
| `crates/al-observability/src/lib.rs` | modify | `init` を `build_logging_layers` 委譲 + 暫定 local file sink。`InitError::Build` 追加 |
| `crates/al-cli/tests/observability.rs` | modify | golden test 拡充(上記 Scope) |
| `docs/specs/ruviform-observability-logging-migration.md` | add | 本チェーン spec |
| `docs/designs/ruviform-observability-logging-migration.md` | add | 採用設計・gap/暫定方針 |
| `docs/plans/ruviform-observability-logging-migration.md` | add | 本計画 |
| `docs/test-specs/ruviform-observability-logging-migration.md` | add | 検証基準 |
| `docs/tasks/ruviform-observability-logging-migration.md` | add | タスク |
| `docs/impl/ir/` | add | Implementation Record |

## Task Breakdown

1. [tasks/ruviform-observability-logging-migration.md](../tasks/ruviform-observability-logging-migration.md)
   — golden test 拡充 → 依存追加 → `init` 委譲 → 検証までを
   1 タスクとして実施(テスト先行と委譲は同一変更境界で意味を
   持つため分割しない。コミットは「golden test 追加」「委譲
   実装+依存」「文書」の論理単位に分ける)。

## Dependency Notes

- ruviform resolved SHA `8fb665f168f8c1af011af849821a5929e5362629`
  (本タスクで追従済み。`update-ruviform-rev.mjs` + `sync-ui-deps`
  実施済み、同一 source から `ruviform-observability` を解決する)。
- 暫定 file sink は `tracing-appender`/`tracing-subscriber` の
  既存依存を継続使用(新規外部 dep 追加なし)。
- `resolve_activation`・`LogConfig` 公開形・al-cli 呼出しは
  不変のため、`al-cli` 本体の変更は発生しない想定。

## Risk Register

| Risk | Mitigation |
| --- | --- |
| ruviform stderr sink で `FmtSpan::CLOSE` が出ず leaf span 観測が file sink のみに縮退 | file sink は local layer で CLOSE 維持。regression は設計書に明記、ruviform #335 で追跡 |
| `RollingFile` suffix gap で file sink が委譲できない | 暫定 local 構築に留め #334 で追跡。永久重複実装にしない |
| bounded writer の queue 溢れで stderr event が drop する | 4096 capacity・guard drain は ruviform 正本契約。drop は LoggingHealth で観測可能、既定 OFF 不変 |
| golden test が ruviform の stderr 出力形式差分で落ちる | stage event 本体(JSON/text 双方)は同一 formatter。warn 行・error 行は自前文字列で固定済み |
| lockfile 解決が 8fb665f 以降に再追従する | `check-ruviform-lock` 照合。最終検証前に再 merge + 再生成 |

## Rollback

- `git checkout -- Cargo.lock Cargo.toml` + `git checkout --
  crates/al-observability` で現行 self-contained 実装に復帰。
  golden test 追加のみなら機能差分なしで revert 可能。
- ruviform SHA 追従のみ戻す場合は `update-ruviform-rev.mjs
  <旧sha>` + `sync-ui-deps`。

## Verification Matrix

| Acceptance Gate | 検証手段 |
| --- | --- |
| 既定 OFF/priority/format/error parity | `observability.rs` golden test suite + 追加 cases |
| stderr/file/jsonl/daily7 | `log_flag_enables_stderr_and_file_sinks` + retention test |
| inspect stdout/readonly | `inspect_logs_to_stderr_only_and_keeps_stdout_json` |
| file init 失敗 degrade | 新規 golden test(`<base_dir>/logs` を通常 file で占拠) |
| 単一 Host Subscriber・重複 init なし | unit test(2回目 init が AlreadyInitialized)+ try_init 単一箇所 |
| leaf span 観測(file sink CLOSE) | 新規 golden test(file JSON に close event を含む) |
| 解析結果非回帰・f64 精度 | `mise run test` 全件 + `run` の manifest/artifact diff |
| 依存境界 | `cargo tree` で tracing-* が al-observability 経由のみ |
| ruviform 単一 SHA | `mise run check-ruviform-lock` / `check-ruviform-rev` |
| 全体 green | `mise run verify-ci`(fmt→check-ci→lint-ci→test-ci) |

判定基準の詳細は [test-spec](../test-specs/ruviform-observability-logging-migration.md) を参照する。
