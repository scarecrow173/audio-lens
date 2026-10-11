---
id: "TASK-034EeviFOOrOAjrYXMGXHO"
type: "task"
status: "todo"
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
    - "docs/plans/observability-foundation.md"
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
  references: []
  defers: []
  deferred-by: []
---

# al-cli の logging 配線

## Work

`al-observability`(T1)を al-cli に配線する(plan T2)。flag
parse・init 位置・dispatch span・env scrub を design どおりに
実装する。

- `--log <directive>`(値必須)・`--log-format
  <sink>=<format>[,...]`(値必須)を既存の
  `parse_flag`/`flag_present` パターンで parse する。flag が
  存在して値が無い場合は `--input-ids` と同じ
  `flag_present && parse_flag.is_none()` チェックで
  `error: --log requires a value`(同様に `--log-format`)
  + exit 2。flag はサブコマンド後の args でのみ解釈する
  (`al --log x run` は従来どおり unknown subcommand)
- `fn init_logging(args, base_dir, file_sink) ->
  Result<Option<LogGuard>, String>` ヘルパを `main.rs` に追加。
  値必須チェック後、`AL_LOG`/`AL_LOG_FORMAT` env と flag 生値を
  `resolve_activation` に渡し、`Enabled` なら `file_sink` に
  応じて `file_dir = Some(base_dir/logs)` を設定して `init`
  を呼ぶ。error は `e.to_string()` で `fail()` へ
- 各 `cmd_*`(`run`/`resume`/`inspect`)で `--base-dir` 検証
  直後に `init_logging` を呼び、返却 guard を cmd スコープ
  全体で保持する(早期 `fail()` でも Drop で flush)。
  `run`/`resume` は file+stderr、`inspect` は stderr のみ
- dispatch span(`cli::run`/`cli::resume`/`cli::inspect`)は
  init 成功後に生成する(init 前の span は inert で
  `span.record` が no-op になる design の順序規約)。`job_id`
  は `span.record` で後付けする
- `USAGE` に `--log`・`--log-format`・`AL_LOG`・`AL_LOG_FORMAT`
  を追記する
- `crates/al-cli/tests/cli.rs` に `al_command() -> Command`
  ヘルパを導入し、全 `Command::new(al)` 呼出しで
  `AL_LOG`/`AL_LOG_FORMAT` を `env_remove` する(edition 2024
  で `std::env::set_var` が unsafe のため子プロセス側を
  scrub)

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/observability-foundation.md`
- `docs/designs/observability-foundation.md`
- `docs/adr/logging-sink-format-activation-contract.md`

## Dependencies

- `docs/tasks/observability-crate.md`(T1):
  `al-observability` の API と workspace dep が前提

## Affected Files

- `crates/al-cli/Cargo.toml`: `al-observability`・`tracing`
  を `.workspace = true`、`tracing-subscriber` を
  `[dev-dependencies]` に(capture subscriber test 用)
- `crates/al-cli/src/main.rs`: flag parse・`init_logging`・
  各 `cmd_*` の init + dispatch span・USAGE 追記
- `crates/al-cli/tests/cli.rs`: `al_command()` env scrub
  ヘルパ、logging 系統合 test、metadata-only capture test

## Done When

- [ ] `run`/`resume` で `--log`/`AL_LOG` 指定時に stderr+file
  の両 sink に event が出る。`inspect` は stderr のみで
  `<base_dir>/logs/` に file が作られない
- [ ] 値なし `--log`/`--log-format`・未知 sink/format・壊れた
  directive・空文字 flag が `error:` 行 + exit 2 で fail-fast
  し panic しない。`al --log x run` は unknown subcommand
- [ ] dispatch span が init 後に生成され、guard が cmd スコープ
  全体で保持される
- [ ] logging 有効時でも `al inspect` stdout が有効 JSON のまま
- [ ] 統合 test が `al_command()` で env scrub 済みで、
  metadata-only regression test(capture subscriber + 自前
  Layer で field サイズ ≤1 KiB を assert)が存在し pass する

## Verification

- [ ] `mise exec -- cargo test -p al-cli` が green
- [ ] `al run --log info` で stderr と
  `<base_dir>/logs/al.YYYY-MM-DD.log` の両 sink に stage 完了
  event が観測できる
- [ ] `mise exec -- cargo tree --workspace --edges normal` で
  `tracing-subscriber`/`tracing-appender` が `al-observability`
  経由のみに現れる
- [ ] `mise exec -- cargo clippy`・`cargo fmt --check` が green
