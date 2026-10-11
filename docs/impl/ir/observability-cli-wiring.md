---
id: "IMPL-034Eg35uKyFu2Dmhclk8F5"
type: "impl"
status: "completed"
title: "al-cli の logging 配線"
created: "2026-10-11"
updated: "2026-10-11"
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
    - "docs/tasks/observability-cli-wiring.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/observability-crate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/observability-foundation.md"
    - "docs/adr/logging-sink-format-activation-contract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/observability-foundation.md"
  references:
    - "docs/plans/observability-foundation.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# al-cli の logging 配線

## Summary

`al-observability`(T1)を al-cli に配線した(plan T2)。flag
parse・init 位置・dispatch span・env scrub を design どおりに
実装し、logging 系統合 test と metadata-only regression test を
`tests/observability.rs` に追加した。

## Context

本 IR は実装完了後に作成した(本来の運用はタスク開始時・最初の
コード変更前に作成する)。対応 task は
`docs/tasks/observability-cli-wiring.md`、plan は
`docs/plans/observability-foundation.md`、上位設計は
`docs/designs/observability-foundation.md` と ADR
`docs/adr/logging-sink-format-activation-contract.md`、検証仕様は
`docs/test-specs/observability-foundation.md`。前提は T1
(`docs/impl/ir/observability-crate.md`)の API と workspace dep。

## Implementation

### Changes

- `crates/al-cli/Cargo.toml`: `al-observability`・`tracing` を
  `[dependencies]`、`tracing-subscriber` を `[dev-dependencies]`
  に追加(capture subscriber test 用)
- `crates/al-cli/src/main.rs`:
  - `init_logging(args, base_dir, file_sink) ->
    Result<Option<LogGuard>, String>` ヘルパを追加。
    `flag_present && parse_flag.is_none()` チェックで値なし
    `--log`/`--log-format` を `"--log requires a value"` /
    `"--log-format requires a value"` の `Err` として返し、
    呼出し側の `fail()` で `error:` 行 + exit 2 に載せる。
    `AL_LOG`/`AL_LOG_FORMAT` env と flag 生値を
    `resolve_activation` に渡し、`Enabled` なら `file_sink`
    に応じて `config.file_dir = Some(base_dir/logs)` を設定
    して `init` を呼ぶ。`LogConfigError`/`InitError` は
    `e.to_string()` で `fail()` へ
  - `cmd_run`/`cmd_resume` は `init_logging(.., true)`(file +
    stderr)、`cmd_inspect` は `init_logging(.., false)`
    (stderr のみ。read-only のため file sink を建てない)。
    返却 guard を cmd スコープ全体で保持し、早期 `fail()`
    でも Drop で flush される
  - dispatch span `cli::run`/`cli::resume`/`cli::inspect` は
    init 成功後に生成(init 前の span は inert で `record` が
    no-op になる design の順序規約)。`job_id` は job 確定後に
    `span.record` で後付け(`cli::run` は `len` = input 数も
    record)
  - `USAGE` に `--log`・`--log-format`・`AL_LOG`・
    `AL_LOG_FORMAT` を追記
- `crates/al-cli/tests/cli.rs`: `al_command() -> Command`
  ヘルパを導入し、全 `Command::new(al)` 呼出しを集約して
  `AL_LOG`/`AL_LOG_FORMAT` を `env_remove` で scrub
  (edition 2024 で `std::env::set_var` が unsafe のため子
  プロセス側を scrub)
- `crates/al-cli/tests/observability.rs` 新設(6 test):
  `log_flag_enables_stderr_and_file_sinks`(両 sink に stage
  event)・`al_log_env_activates_logging_without_flags`
  (env のみで活性化)・`logging_off_creates_no_log_files`
  (既定 OFF + `--log-format` 単独では活性化しない)・
  `inspect_logs_to_stderr_only_and_keeps_stdout_json`
  (inspect は stderr のみ・stdout JSON 維持)・
  `malformed_log_options_fail_fast`(値なし・空文字・未知
  sink/format・壊れた directive が `error:` + exit 2、
  `al --log x run` は unknown subcommand)・
  `emitted_fields_stay_metadata_sized`(自前 `Layer` の
  capture subscriber で emit される全 field が ≤1 KiB の
  metadata-only であることを assert)

### Decisions During Implementation

- `init_logging` は `Result<_, String>` を返し、既存の
  `fail()`(`error:` 行 + exit 2)規約にそのまま乗せる形に
  した。observability 側で exit 経路を持たせない
- file sink の有無は `init_logging` の `file_sink` bool で
  呼出し側が宣言する形にし、`file_dir` の設定責務を al-cli
  側(`<base_dir>/logs`)に置いた
- metadata-only test は `tracing::subscriber::with_default` +
  自前 `FieldSizeLayer` で event/span attribute の render 済み
  field サイズを収集し、≤1 KiB を assert する design 規定の
  方式で実装(規約違反の検出を emit 内容に直接掛ける)

## Related Experiments

### Adopted

### Rejected

## Validation

- `cargo test -p al-cli`: `tests/observability.rs` 6 test を
  含め green
- `al run --log info` で stderr(text)と
  `<base_dir>/logs/al.YYYY-MM-DD.log`(JSON-lines)の両 sink
  に stage 完了 event を確認
- `cargo tree --workspace --edges normal` で
  `tracing-subscriber`/`tracing-appender` が
  `al-observability` 経由のみに現れることを確認(al-cli の
  `tracing-subscriber` は dev-dep)
- `cargo build --workspace`・`cargo clippy`・
  `cargo fmt --check`: green

## Risks

- `stderr=json` 指定時は tracing 行と `fail()` の plaintext
  error 行が混在し stderr 全体は機械 parse できない(test-spec
  Non-goals の既知制約)

## Follow-ups

- なし(T4 の leaf span 計装・T3 の stage 計時は本配線の上で
  観測される)
