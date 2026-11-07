---
id: "DESIGN-uCvLq1YDJau0qmwrQqB52k"
type: "design"
status: "approved"
title: "al-observability logging の ruviform-observability 委譲設計 (AL-OBS-01)"
created: "2026-11-04"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/138"
    - "https://github.com/scarecrow173/ruviform/issues/276"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/ruviform-observability-logging-migration.md"
  implemented-by: []
  depends-on:
    - "../adr/logging-sink-format-activation-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./observability-foundation.md"
    - "./ruviform-migration-responsibility-map.md"
    - "https://github.com/scarecrow173/ruviform/issues/334"
    - "https://github.com/scarecrow173/ruviform/issues/335"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/ruviform-observability-logging-migration.md"
    - "../adr/logging-sink-format-activation-contract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ruviform-observability-logging-migration.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/277"
    - "https://github.com/scarecrow173/ruviform/issues/289"
  defers: []
  deferred-by: []
---

# al-observability logging の ruviform-observability 委譲設計 (AL-OBS-01)

## Context

Issue [#138](https://github.com/scarecrow173/audio-lens/issues/138)
([spec](../specs/ruviform-observability-logging-migration.md))。

現行 `crates/al-observability` は次を所有している。

- `resolve_activation`: `AL_LOG`/`AL_LOG_FORMAT`/`--log`/
  `--log-format` の生値から `Activation` を返す純粋関数。
  優先順位(flag>env 全体置換)、空文字規約、sink/format
  閉集合語彙と fail-fast エラー語彙は製品契約(ADR-034EdkLs…
  の Confirmation 項が error 文字列を含めて固定)。
- `init`: stderr layer(text/json 切替)+ file layer
  (`RollingFileAppender` `al` + `log` suffix・daily・
  `max_log_files(7)` + `non_blocking`)を
  `registry().with(layers).with(EnvFilter).try_init()` で
  global subscriber として登録し、`WorkerGuard` を
  `LogGuard` として返す。両 layer に `FmtSpan::CLOSE` を設定。

Ruviform #276 で `ruviform-observability`(feature `logging`)が
提供する `build_logging` / `build_logging_layers` は、sink 構築・
bounded writer・health・`WriterGuard` drain を担う汎用実装で、
host が Dispatch/layer 組立て・登録を所有する契約を持つ。

### 確認した adapter gap と暫定方針

委譲にあたり Ruviform adapter の公開契約に 2 件の不足を確認し、
各々 Ruviform Issue を起票した(恒久回避策で隠さず追跡する)。

| gap | 影響 | 追跡 | 暫定方針 |
| --- | --- | --- | --- |
| `SinkDestination::RollingFile` に `filename_suffix` がなく `{prefix}.{date}` しか生成できない | `al.YYYY-MM-DD.log` 命名契約(変更禁止仕様 4)を表現不能 | [ruviform#334](https://github.com/scarecrow173/ruviform/issues/334) | file sink は従来どおり local 構築を継続 |
| `fmt` layer の span lifecycle event を設定する経路がなく `FmtSpan::NONE` 固定 | leaf span の close event が sink に残らない(spec Requirements) | [ruviform#335](https://github.com/scarecrow173/ruviform/issues/335) | 下記「既知の一時 regression」参照 |

## Decision

**stderr sink の構築を `build_logging_layers` へ委譲し、file
sink は命名契約維持のため #334 解消まで local 構築を継続する。
両者を単一 `Registry` の layer 群として host(al-cli 経由の
`al-observability::init`)が `try_init` で登録し、guard は
`LogGuard` が ruviform `WriterGuard` と local `WorkerGuard`
を併せ持つ。**

## Components and Boundaries

### §1 `resolve_activation`(製品 adapter・維持)

- 変更しない。env/flag 優先順位・空文字規約・
  `{stderr,file}`/`{text,json}` 閉集合語彙・`LogConfigError`
  の error 文字列は製品契約であり、`run_ingest` 系 CLI
  golden test が exit code/`error:` 行を固定する。
- `LogConfig` の公開形も維持(`filter: EnvFilter`・
  `stderr_format`/`file_format: LogFormat`・
  `file_dir: Option<PathBuf>`)。al-cli は `file_dir` を
  `run`/`resume` で `<base_dir>/logs`、`inspect`/`export` では
  未設定のまま使う現行契約のまま。

### §2 `init`(委譲 + 暫定 local file sink)

```text
init(LogConfig)
  ├─ LoggingConfig { sinks: [SinkConfig {
  │      destination: Stderr, format: map(stderr_format),
  │      filter: <directive>, failure_mode: Required }] }
  │    -> build_logging_layers()
  │       .layers  (ruviform bounded-writer stderr layer,
  │                 per-sink EnvFilter)
  │       .guards  (WriterGuard)
  │       .degraded(Required 指定のため常に空)
  ├─ file_dir: Some(dir) のとき local file layer:
  │     RollingFileAppender(al + log suffix, DAILY, max 7)
  │     + non_blocking + fmt layer(.json()/text, ansi off,
  │     FmtSpan::CLOSE) + EnvFilter::with_filter
  │     -> layers に push、WorkerGuard を保持
  │     build 失敗 -> eprintln!("warn: ...") + degrade
  └─ registry().with(layers).try_init()
       -> LogGuard { ruviform WriterGuard, local WorkerGuard }
```

- **stderr → ruviform**: bounded queue(`DEFAULT_STDERR_QUEUE_
  CAPACITY` = 4096)経由の非同期書込み・drop 計数・guard
  drain(5s bound)は ruviform 正本。filter は per-sink
  `with_filter` 適用で、file layer 側も同じ directive を
  `EnvFilter::with_filter` で適用する — 現行の global filter
  と同じ directive が両 sink に掛かる等価構成。
- **file → local(暫定)**: `filename_prefix("al")` +
  `filename_suffix("log")` + `Rotation::DAILY` +
  `max_log_files(7)` + `non_blocking` + `FmtSpan::CLOSE`。
  #334 解消後は `SinkDestination::RollingFile` へ移し本
  ブロックを撤去する(解消条件)。
- **span event 方針**: local file layer は `FmtSpan::CLOSE`
  を維持するため leaf span の close event は file(JSONL、
  機械集計の主経路)に残り続ける。stderr sink は #335 解消
  まで close event を出さない(下記 regression)。
  → **#335 解消済み(2026-11-07)**: stderr は
  `SinkConfig::span_events = SpanEvents::CLOSE` で復旧。
  後続: [ruviform-logging-span-events-compat](./ruviform-logging-span-events-compat.md)。
- **登録**: `try_init` は従来どおり `al-observability::init`
  内の1回きり。2回目は `InitError::AlreadyInitialized`。
  ruviform 側は `set_global_default` を呼ばない契約のため、
  global 重複 init は構造的に発生しない。

### §3 型・エラー写像

- `al_observability::LogFormat` (Text/Json) →
  `ruviform LogFormat` (Text/JsonLines)。adapter 内の
  1 箇所の写像関数に集約し、他の型を再定義しない。
- `InitError` に ruviform `LoggingBuildError` を包む variant
  を追加(`Build`)。`Display` は `error:` 行規約に合わせた
  人間可読文とし、`AlreadyInitialized` 文言は維持。
- `LogGuard` は opaque を維持: ruviform `WriterGuard` +
  local `WorkerGuard` を保持し Drop で drain/flush。

### §4 依存配線

- `[workspace.dependencies]` に
  `ruviform-observability = { git = "…", features = ["logging"] }`
  を追加(他 ruviform crate と同形。rev は書かず
  Cargo.lock が正本)。
- `al-observability` は `tracing-subscriber`(EnvFilter・
  fmt layer・`try_init`)と `tracing-appender`(暫定 file
  sink)を引き続き直接依存する。leaf crate の依存構図
  (`leaf -> tracing` のみ)は不変であり、`cargo tree` での
  `tracing-subscriber`/`tracing-appender` 経路が
  `al-observability` 経由のみである契約も維持する。

## Data and Control Flow

```text
al run --log "al_locate=debug,info" --base-dir <dir>
  -> parse_flag("--log") / env AL_LOG            (al-cli)
  -> resolve_activation(..)                      (al-observability)
       Activation::Enabled(LogConfig{filter,stderr,file_dir=None})
  -> config.file_dir = Some(<dir>/logs)          (al-cli)
  -> init(config)                                (al-observability)
       build_logging_layers(stderr sink)         (ruviform)
       local file layer (.log suffix)            (al-observability 暫定)
       registry().with([stderr, file]).try_init()
  -> LogGuard { WriterGuard(stderr worker), WorkerGuard(file) }
       ^^^ cmd_* スコープ終了まで保持 → drain+flush
leaf crates emit via tracing::info!/info_span!    (変更不要)
```

## 既知の一時 regression

- **stderr sink の span close event 欠落**: ruviform adapter が
  `FmtSpan` を設定しないため、stderr sink は span lifecycle
  event を出力しない。`al run --log info` の stderr で
  `cli::run`/`stage` span の close 行が現れない。
  緩和: file JSONL sink は local layer で `FmtSpan::CLOSE`
  を維持するため、集計用途の leaf span close event は file
  sink に残る。復旧条件: ruviform #335 で span event 設定が
  提供された時点で stderr sink にも適用する。
  → **解消済み(2026-11-07)**: #335 の `SinkConfig::span_events`
  提供により `SpanEvents::CLOSE` を stderr に適用
  ([ruviform-logging-span-events-compat](./ruviform-logging-span-events-compat.md))。

## Risks and Trade-offs

- **stderr の非同期化**: ruviform stderr sink は bounded
  queue 経由で、`fail()` の `error:` 行(direct eprintln)と
  tracing 行の行順序が従来の同期書込みと変わり得る。
  ADR の「行単位 interleave 共存」規約は満たす。queue 溢れは
  イベントを drop する(tracing-appender の block しない
  方針と同等以上の安全性で、drop は health 計数で観測可能)。
- **global filter → per-sink filter**: `Registry` 末尾の
  `EnvFilter` から各 layer の `with_filter` へ移る。有効時は
  event が per-sink に評価されるため微増コストがあるが、
  directive は同一で観測結果は等価。既定 OFF(未登録)は
  不変のため OFF overhead 契約に影響しない。
- **file sink 二重実装の残存**: #334 解消まで `RollingFile` 系
  構築が製品側に残る。回避策は本設計の暫定ブロックに限定
  し、解消条件・追跡 Issue を本書・コードコメント・
  plan/task に明記する。
- **ruviform-observability の追加で lockfile 解決 SHA が
  変わる**: 本変更自体は `8fb665f…` への追従を含む。同一
  source からの解決のため `check-ruviform-lock` の単一 SHA
  契約は維持される。追従による API 差分は `mise run check`
  で検証済み。

## Deferred Design Concerns

- stderr sink の `FmtSpan::CLOSE` 再有効化(ruviform #335
  解消待ち)。→ **対応済み(2026-11-07)**:
  [ruviform-logging-span-events-compat](./ruviform-logging-span-events-compat.md)。
- file sink の `SinkDestination::RollingFile` 移行と local
  file layer 撤去(ruviform #334 解消待ち)。→ #334 は
  解消済み(`filename_suffix` 提供)。移行自体は未着手の
  別タスクとして残る。
- Collector/OBS-07(#289)導入時の `LoggingHealth` 公開と
  Source 登録。AudioLens に登録可能な Counter/Source 実体が
  現状ないため、本 Issue では `LoggingLayersResult::health`
  を `LogGuard` に保持するに留め公開 API 化しない。
- GUI プロセスの logging 配線(別 Issue)。

## Verification Notes

- 既存 `crates/al-cli/tests/observability.rs` + 本 Issue で追加
  する golden test(retention、file init 失敗 degrade、format
  E2E、flag>env 優先、span close observability)が同一のまま
  PASS する。
- `cargo tree` で `tracing-subscriber`/`tracing-appender` が
  `al-observability` 経由のみで現れること。
- `mise run verify-ci` + `check-ruviform-lock` が green で、
  ruviform 全 crate が単一 SHA(`8fb665f…`)に解決されること。
- Logging ON/OFF で `run` の manifest/artifacts が同一
  (f64 解析値・成果物 hash に差がない)こと。
