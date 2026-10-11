---
id: "TASK-034EevbtmmkzTShLCB0Mw0"
type: "task"
status: "done"
title: "al-observability crate の新設"
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
  implemented-by:
    - "docs/impl/ir/observability-crate.md"
  depends-on: []
  blocks:
    - "docs/tasks/observability-cli-wiring.md"
    - "docs/tasks/observability-leaf-instrumentation.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/observability-foundation.md"
    - "docs/adr/observability-dedicated-crate.md"
    - "docs/adr/logging-sink-format-activation-contract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/observability-foundation.md"
  references: []
  defers: []
  deferred-by: []
---

# al-observability crate の新設

## Work

sink init・format 配線・routing・activation 解釈を所有する専用
crate `crates/al-observability` を新設する(plan T1)。公開 API は
design 規定の薄い2段構成とする: env/flag の生値を引数で受ける
純粋関数 `resolve_activation` と、解釈済み `LogConfig` を受けて
subscriber を構築し `LogGuard` を返す `init`。

- 型: `SinkKind { Stderr, File }`、`LogFormat { Text, Json }`、
  `LogConfig { filter: EnvFilter, stderr_format, file_format,
  file_dir: Option<PathBuf> }`、
  `Activation { Disabled, Enabled(LogConfig) }`、
  `LogConfigError { InvalidDirective, UnknownSink, UnknownFormat,
  MalformedFormatDirective }`(各 `String` 保持・`Display`)、
  `InitError { AlreadyInitialized }`、
  `LogGuard`(non_blocking `WorkerGuard` を保持し Drop で
  flush を保証)
- `resolve_activation(log_env, log_flag, format_env,
  format_flag) -> Result<Activation, LogConfigError>`:
  env の空文字は未指定扱い、flag の空文字は malformed で
  fail-fast、flag は env を per-concern 全体置換で上書き、
  activation 未指定時の format directive は無視、既定 OFF
- `init(config) -> Result<LogGuard, InitError>`: per-sink
  layer builder が `Box<dyn Layer<S> + Send + Sync>` を返し、
  `Vec<Box<dyn Layer<Registry>>>` を単一 `.with()` + EnvFilter
  で重畳。file は `rolling::Builder`(DAILY・prefix `al`・
  suffix `log`・`max_log_files(7)`)+ `non_blocking` +
  `with_ansi(false)`。`Builder::build` 失敗時のみ stderr warn
  + stderr-only degrade(`Ok` を返す)

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/observability-foundation.md`
- `docs/designs/observability-foundation.md`
- `docs/adr/observability-dedicated-crate.md`
- `docs/adr/logging-sink-format-activation-contract.md`

## Dependencies

- なし(先行タスクなし。T2・T4 が本タスクに依存する)

## Affected Files

- `Cargo.toml`(root): `workspace.members` に
  `crates/al-observability` 追加、`[workspace.dependencies]` に
  `al-observability`/`tracing`/`tracing-subscriber`(features:
  `fmt`, `env-filter`, `json`, `ansi`)/`tracing-appender` を追加
- `crates/al-observability/Cargo.toml`(新設): tracing /
  tracing-subscriber / tracing-appender を `.workspace = true`
- `crates/al-observability/src/lib.rs`(新設): 型・
  `resolve_activation`・`init`・per-sink layer builder

## Done When

- [ ] `al-observability` が workspace member として build できる
- [ ] `resolve_activation` が全分岐(未指定/空文字 env/flag
  優先/format 単独無視/未知 sink・format/壊れた directive/
  空文字 flag)を正しく解釈する
- [ ] `init` が dual sink + EnvFilter で subscriber を構築し、
  file init 失敗時に warn + stderr-only degrade する
- [ ] file 名が `al.YYYY-MM-DD.log` 形式・保持 7 世代である
- [ ] panic する `EnvFilter::new` を使わず `try_new`/`try_from`
  系で parse 失敗を `Result` で返す

## Verification

- [ ] `mise exec -- cargo test -p al-observability` で unit test
  (resolve_activation 全分岐・format directive parse・
  degrade 経路)が pass
- [ ] `mise exec -- cargo build --workspace` が green
- [ ] `mise exec -- cargo clippy`・`cargo fmt --check` が green
