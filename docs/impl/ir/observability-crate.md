---
id: "IMPL-034Eg353ac42Ik7en8rVVT"
type: "impl"
status: "completed"
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
    - "docs/tasks/observability-crate.md"
  implemented-by: []
  depends-on: []
  blocks: []
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
  references:
    - "docs/plans/observability-foundation.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# al-observability crate の新設

## Summary

sink init・format 配線・routing・activation 解釈を所有する専用
crate `crates/al-observability` を新設した(plan T1)。公開 API は
design 規定の薄い2段構成: env/flag の生値を受ける純粋関数
`resolve_activation` と、解釈済み `LogConfig` から global
subscriber を構築して `LogGuard` を返す `init`。

## Context

本 IR は実装完了後に作成した(本来の運用はタスク開始時・最初の
コード変更前に作成する)。対応 task は
`docs/tasks/observability-crate.md`、plan は
`docs/plans/observability-foundation.md`、上位設計は
`docs/designs/observability-foundation.md` と ADR
`docs/adr/observability-dedicated-crate.md`・
`docs/adr/logging-sink-format-activation-contract.md`、検証仕様は
`docs/test-specs/observability-foundation.md`。

## Implementation

### Changes

- root `Cargo.toml`: `workspace.members` に
  `crates/al-observability` を追加、`[workspace.dependencies]` に
  `al-observability`(path)・`tracing = "0.1.44"`・
  `tracing-subscriber = "0.3.23"`(features:
  `fmt`, `env-filter`, `json`, `ansi`)・
  `tracing-appender = "0.2.5"` を追加
- `crates/al-observability/Cargo.toml`・`src/lib.rs` 新設。
  型: `SinkKind { Stderr, File }`、`LogFormat { Text, Json }`、
  `LogConfig { filter: EnvFilter, stderr_format, file_format,
  file_dir: Option<PathBuf> }`、
  `Activation { Disabled, Enabled(LogConfig) }`、
  `LogConfigError { InvalidDirective, UnknownSink, UnknownFormat,
  MalformedFormatDirective }`(生値保持・`Display`)、
  `InitError { AlreadyInitialized }`、`LogGuard`
  (non_blocking `WorkerGuard` を保持し Drop で flush 保証)
- `resolve_activation(log_env, log_flag, format_env,
  format_flag)`: per-concern 全体置換(flag が env を丸ごと
  置換)、env の空文字は未指定扱い・flag の空文字は malformed で
  fail-fast、activation 未指定時の format directive は無視
  (`Disabled`)、既定 OFF。EnvFilter の parse は panic する
  `new` ではなく `try_new` で `InvalidDirective` へ写像
- `init(config)`: per-sink fmt layer を
  `Box<dyn Layer<Registry> + Send + Sync>` の `Vec` に積み、
  単一 `.with()` + 末尾の global `EnvFilter` で重畳して
  `try_init`。file sink は `rolling::Builder`(DAILY・prefix
  `al`・suffix `log`・`max_log_files(7)`)+ `non_blocking` +
  `with_ansi(false)`
- `Builder::build` 呼出し前に `fs::create_dir_all(dir)` を実行
  (`RollingFileAppender` が log dir 作成前に
  `prune_old_logs` を走らせ、fresh `<base_dir>/logs` で stray
  stderr error を出す問題への対処)。`build` 失敗時は stderr に
  warn 行 + stderr-only degrade で `Ok` を返す
- unit test 15 件: `resolve_activation` 全分岐(未指定・env
  空文字・flag 優先・format 単独無効・未知 sink/format・壊れた
  directive・空文字 flag)・format directive parse・file build
  失敗時の degrade 経路

### Decisions During Implementation

- EnvFilter の parse は lenient — `bogus!!!` のような文字列も
  valid target directive として受理される。fail-fast を検証する
  例には実際に `try_new` が失敗する `=info`(target 空)・
  `info=bogus`(未知 level)を使い、test-spec の fail-fast 例も
  実装に合わせて修正した
- text/json の分岐で fmt layer の具象型が変わるため、per-sink
  builder は `Box<dyn Layer>` を返し `Vec` で重畳する形にした
- EnvFilter は per-layer filter ではなく registry 末尾の global
  filter として適用(design 規約)
- `FmtSpan::CLOSE` の span lifecycle event 付与は本タスク時点
  では未設定で、T4(leaf crate span 計装)で「内部に event を
  持たない leaf span が sink に残らない」不備として発覚・追加
  された(経緯は
  `docs/impl/ir/observability-leaf-instrumentation.md` を参照)

## Related Experiments

### Adopted

### Rejected

## Validation

- `cargo test -p al-observability`: unit test 15 件 pass
- `cargo build --workspace`・`cargo clippy`・
  `cargo fmt --check`: green

## Risks

- 同一日の log file への複数 `al` プロセス append の行原子性は
  tracing-appender 依存の既知制約(test-spec Non-goals に明記)

## Follow-ups

- T2(`docs/tasks/observability-cli-wiring.md`)で al-cli へ配線
- T4(`docs/tasks/observability-leaf-instrumentation.md`)で両
  fmt layer に `FmtSpan::CLOSE` を追加済み
