---
id: "DESIGN-034EdQdU4O8Ml4IzUkoFFY"
type: "design"
status: "approved"
title: "observability foundation: sink・activation・計時の配線設計"
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
    - "../specs/observability-foundation.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./overview.md"
    - "./job-orchestration.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/observability-foundation.md"
    - "../adr/observability-dedicated-crate.md"
    - "../adr/logging-sink-format-activation-contract.md"
    - "../adr/stage-timing-manifest-persistence.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# observability foundation: sink・activation・計時の配線設計

## Context

spec `../specs/observability-foundation.md`(approved)と 3 本の ADR
(`../adr/observability-dedicated-crate.md`・
`../adr/logging-sink-format-activation-contract.md`・
`../adr/stage-timing-manifest-persistence.md`)で確定した方針を、
実装可能な配線設計に具体化する。対象は `al-observability` crate の
公開 API、sink 構築の内部構造、al-cli への init 配線、leaf crate の
instrumentation 配置、`StageRecord.duration_ms` の manifest 変更、
依存配線である。

現状の前提:

- 全 crate の Cargo.toml に `tracing`/`log` 系依存は 0 件。
  production 経路の stderr 出力は `crates/al-cli/src/main.rs` の
  `fail()`(`error: {message}` + exit 2)1系統のみ
- al-cli は clap 等を使わない手書き引数解析で、flag 取得は
  `parse_flag`/`flag_present` パターン(`main.rs:170-178`)。値必須
  flag の検査は `--input-ids` と同じ
  `flag_present && parse_flag.is_none()` 形(`main.rs:195`)
- 全ステージ実行の唯一の通過点は
  `crates/al-job/src/runner.rs` の `StageRunner::execute`(210-283)、
  `stage.run(&ctx)` は 257 行目の `match` の対象
- manifest の `StageRecord` は
  `crates/al-job/src/manifest.rs:30-40`。構造体リテラルは
  `manifest.rs:99-104`(`Manifest::new`)・`runner.rs:231-236`
  (`unwrap_or_else` fallback)・`registry.rs:77-82`(テスト)の3箇所

## Scope

### In Scope

- `crates/al-observability` の新設と公開 API
  (`resolve_activation`/`init`/`LogConfig`/`Activation`/`SinkKind`/
  `LogFormat`/エラー型/`LogGuard`)の設計
- stderr/file dual sink の構築詳細(rolling・non_blocking・
  per-sink format・degrade)
- al-cli の各 `cmd_*` への init 配線と `--log`/`--log-format` flag
  解釈、統合テストの env scrub
- 計装対象 crate(al-cli・al-job・al-locate・al-analysis・
  al-media・al-viz・al-dataset)への span/event 配置と命名・field
  規約の具体化
- `StageRecord.duration_ms` の追加と runner 両分岐での設定
- workspace 依存配線(`tracing`/`tracing-subscriber`/
  `tracing-appender`)と criterion の al-math dev-dep 化、samply
  手順文書の配置方針

### Out of Scope

- raw payload capture の opt-in、常時計装プロファイラ(puffin/tracy)
- CI ベンチ・回帰ゲート、既存 examples ベンチの criterion 移行
- `--quiet` 相当の stderr 抑制 flag、stdout への log 出力
- export package への stage duration 同梱
- criterion ベンチ対象・計測項目の選定(別 chain の
  `al-math-simd-reduction` 側で扱う)

## Components and Boundaries

### `crates/al-observability`(新設)

sink init・format 配線・routing・activation 解釈の単一所有境界。
公開 API は「生値を受けて解釈する `resolve_activation`」と
「解釈済み config を受けて subscriber を組んで guard を返す
`init`」の薄い2段に留める。

```rust
// crates/al-observability/src/lib.rs

/// sink 識別子。format directive の語彙としても使う閉集合。
pub enum SinkKind { Stderr, File }

/// sink ごとの出力形式。
pub enum LogFormat { Text, Json }

/// 解釈済みの logging 設定。init の入力。
pub struct LogConfig {
    /// EnvFilter directive 解釈済みのフィルタ。
    pub filter: tracing_subscriber::EnvFilter,
    /// stderr sink の format(既定 Text)。
    pub stderr_format: LogFormat,
    /// file sink の format(既定 Json)。
    pub file_format: LogFormat,
    /// `Some(<base_dir>/logs)` で file sink を組む(run/resume)。
    /// `inspect` では `None`(stderr のみ)。
    pub file_dir: Option<std::path::PathBuf>,
}

/// activation 解釈の結果。
pub enum Activation {
    /// 既定 OFF — init は呼ばない(または no-op)。
    Disabled,
    /// directive 解釈済みの設定で有効化。
    Enabled(LogConfig),
}
```

activation 解釈関数(純粋関数 — env を内部で読まず env 値も引数で
受け、unit test 可能にする):

```rust
/// env/flag の生値から activation を解釈する。
///
/// - `log_env` = `AL_LOG`、`log_flag` = `--log`(activation concern)
/// - `format_env` = `AL_LOG_FORMAT`、`format_flag` = `--log-format`
/// - 優先順位は per-concern の全体置換: flag 値があれば env 値を
///   丸ごと置き換える(directive 単位の merge はしない)
/// - 「空文字=未指定」規約は env のみ: env が unset または空文字は
///   未指定扱い。flag 値の空文字(`--log ""`/`--log-format ""`)は
///   malformed として `LogConfigError` で fail-fast する(directive
///   `""` が EnvFilter で ERROR-level を有効化してしまう副作用を
///   防ぐ)
/// - activation が未指定なら format directive は読まず `Disabled`
pub fn resolve_activation(
    log_env: Option<&str>,
    log_flag: Option<&str>,
    format_env: Option<&str>,
    format_flag: Option<&str>,
) -> Result<Activation, LogConfigError>
```

init 関数:

```rust
/// 解釈済み config で global subscriber を構築する。
/// `config.file_dir` が `Some` のとき file sink を組み、init に
/// 失敗した場合は stderr に warn 行を出して stderr-only に
/// degrade する(Err にしない — 「log 機構は job を壊さない」)。
pub fn init(config: LogConfig) -> Result<LogGuard, InitError>

/// 返却 guard。file sink の `WorkerGuard` を保持し、Drop で flush
/// を保証する。呼出し側は `cmd_*` のスコープ終了まで保持する。
pub struct LogGuard { /* Option<non_blocking::WorkerGuard> */ }
```

エラー型(全て `Display` を実装し、呼出し側の `fail()` にそのまま
渡せる。panic はしない):

```rust
pub enum LogConfigError {
    /// EnvFilter directive の parse 失敗。生 directive を含める。
    InvalidDirective(String),
    /// 未知の sink 名(`stdout` を含む)。
    UnknownSink(String),
    /// 未知の format 値。
    UnknownFormat(String),
    /// `<sink>=<format>` 形でない壊れた directive。
    MalformedFormatDirective(String),
}

pub enum InitError {
    /// global subscriber が既に設定済み。設計上到達しない内部エラー。
    AlreadyInitialized,
}
```

format directive(`<sink>=<format>` カンマ区切り)の解釈は
`resolve_activation` 内部で行い、`SinkKind`/`LogFormat` への写像で
未知語彙を `UnknownSink`/`UnknownFormat`、その他の malformed を
`MalformedFormatDirective` に分類する。未指定 sink は既定値
(stderr=Text、file=Json)に留まる。flag の空文字値は `--log ""` を
`InvalidDirective`、`--log-format ""` を
`MalformedFormatDirective` に分類し、呼出し側の `fail()` で
fail-fast する。

### sink 構築(init 内部)

- **layer 構築関数**: format 分岐(Text=既定の fmt / Json=`.json()`)
  で layer の具象型が異なるため、各 sink の layer 構築関数を
  ジェネリック形にして `Layer::boxed()` で型を揃える:

  ```rust
  fn stderr_layer<S>(format: LogFormat)
      -> Box<dyn Layer<S> + Send + Sync>
  where
      S: Subscriber + for<'span> LookupSpan<'span>,
  ```

  file layer 側も同じシグネチャで、内部で writer の構築と
  `WorkerGuard` の取り出しだけを担わせる
- **stderr layer**: `tracing_subscriber::fmt::Layer` に
  `with_writer(std::io::stderr)`。Text は既定の fmt、Json は
  `.json()` を付けてから `Layer::boxed()` する
- **file layer**: `tracing_appender::rolling::Builder::new()` に
  `rotation(Rotation::DAILY)` + `filename_prefix("al")` +
  `filename_suffix("log")` + `max_log_files(7)` を設定して
  `build(dir)`(素の `rolling::daily` では `al.YYYY-MM-DD.log`
  形式にならない)。`tracing_appender::non_blocking(writer)` で
  `(NonBlocking, WorkerGuard)` を得て(`non_blocking` 自体は失敗
  しない)、writer を `with_writer` で fmt Layer に渡し
  `with_ansi(false)` + text/JSON 切替、guard は `LogGuard` に
  格納する
- **重畳**: box 化した layer を `Vec<Box<dyn Layer<Registry> +
  Send + Sync>>` に集めて1回の `.with()` で重畳し(`Vec<L>` は
  `L: Layer<S>` なら `Layer<S>` を実装する)、EnvFilter は別の
  `.with()` で global filter として載せる:

  ```rust
  let layers: Vec<Box<dyn Layer<Registry> + Send + Sync>> =
      vec![stderr_layer(config.stderr_format) /* , file_layer(..) */];
  tracing_subscriber::registry()
      .with(layers)
      .with(config.filter) // EnvFilter は末尾に global filter として
      .try_init()
  ```

  `.with(stderr_layer).with(file_layer)` のように box 化 layer を
  個別に重畳する形は型が成立しない(2個目の `.with` は
  `Layer<Layered<...>>` を要求し `Box<dyn Layer<Registry>>` は
  それを満たさない)ため、Vec 重畳を正規形とする。`try_init()`
  (panic する `init`/`set_global_default` の unwrap は禁止)で
  global default に設定し、失敗を `InitError` に写像する
- **degrade**: degrade 判定対象は `rolling::Builder::build`
  (`io::Result` を返す唯一の失敗点)のみ。`non_blocking` は
  `(NonBlocking, WorkerGuard)` を返すだけで失敗しない。
  `file_dir` が `Some` で `build` が失敗した場合、
  `eprintln!("warn: cannot initialize file log sink at {dir}:
  {e}")` を出して stderr layer のみの subscriber を組み `Ok` を
  返す
- **sink 構成**: `run`/`resume` は file+stderr、`inspect` は stderr
  のみ(`file_dir: None`)。file sink は activation 有効時にのみ
  組まれるため、既定 OFF では `<base_dir>/logs/` に file が作られ
  ない

### al-cli 配線

`crates/al-cli/src/main.rs` に logging init の小さなヘルパを1本
追加し、各 `cmd_*` で `--base-dir` 取得直後に呼ぶ:

```rust
/// `--log`/`--log-format` と AL_LOG/AL_LOG_FORMAT を解釈して
/// logging を初期化する。`file_sink` は file sink を組むか
/// (run/resume=true、inspect=false)。
fn init_logging(
    args: &[String],
    base_dir: &Path,
    file_sink: bool,
) -> Result<Option<al_observability::LogGuard>, String> {
    if flag_present(args, "--log") && parse_flag(args, "--log").is_none() {
        return Err("--log requires a value".into());
    }
    if flag_present(args, "--log-format") && parse_flag(args, "--log-format").is_none() {
        return Err("--log-format requires a value".into());
    }
    let activation = al_observability::resolve_activation(
        std::env::var("AL_LOG").ok().as_deref(),
        parse_flag(args, "--log"),
        std::env::var("AL_LOG_FORMAT").ok().as_deref(),
        parse_flag(args, "--log-format"),
    )
    .map_err(|e| e.to_string())?;
    match activation {
        al_observability::Activation::Disabled => Ok(None),
        al_observability::Activation::Enabled(mut config) => {
            if file_sink {
                config.file_dir = Some(base_dir.join("logs"));
            }
            al_observability::init(config)
                .map(Some)
                .map_err(|e| e.to_string())
        }
    }
}
```

- `cmd_run`/`cmd_resume`/`cmd_inspect` の先頭付近(`--base-dir` の
  `parse_flag` 直後)で `let _log_guard = match init_logging(...) {
  Ok(g) => g, Err(e) => return fail(e) };` とし、guard を cmd 関数の
  スコープ終了まで保持して non_blocking writer の flush を保証する
- flag はサブコマンド後の args でのみ解釈する既存規約に従い、
  `al --log x run` は従来どおり unknown subcommand
- `al run --log --stages ...` のように次 token が flag の場合は
  その token が値として飲み込まれ directive parse で失敗する
  (`error:` 行 + exit 2。ADR 確定済みの挙動)
- `USAGE` に `--log <directive>`・`--log-format <sink>=<format>[,
  <sink>=<format>...]`・`AL_LOG`・`AL_LOG_FORMAT` を追記する
- 統合テスト `crates/al-cli/tests/cli.rs`: 子プロセス側の
  `AL_LOG`/`AL_LOG_FORMAT` を `Command::env_remove` で除去する。
  `Command::new(al)` が15箇所程度に分散しているため、各 site への
  `env_remove` 追加ではなく `Command` 生成を返す小さな helper
  (例 `fn al_command() -> Command`)に集約して scrub を一箇所に
  置く(edition 2024 で `std::env::set_var` は unsafe のため
  親側ではなく子プロセスの環境を scrub する規約)

### instrumentation 配線

`crates/al-job/src/runner.rs` の `execute` 内で `stage.run(&ctx)` を
stage 境界 span で囲み、`std::time::Instant` で計時して成否両分岐で
event emit + `record.duration_ms` 設定を行う:

```rust
let stage_name = serde_json::to_value(*stage_kind)
    .ok()
    .and_then(|v| v.as_str().map(str::to_owned))
    .unwrap_or_else(|| format!("{stage_kind:?}"));
let span = tracing::info_span!("stage",
    stage = %stage_name, job_id = %manifest.job_id);
let _enter = span.enter();
let started = std::time::Instant::now();
match stage.run(&ctx) {
    Ok(artifacts) => {
        let duration_ms = started.elapsed().as_millis() as u64;
        tracing::info!(stage = %stage_name, job_id = %manifest.job_id,
            duration_ms, "stage completed");
        // artifacts 登録 + record.status = Completed +
        // record.duration_ms = Some(duration_ms)
    }
    Err(err) => {
        let duration_ms = started.elapsed().as_millis() as u64;
        tracing::error!(stage = %stage_name, job_id = %manifest.job_id,
            duration_ms, error = %err.message, "stage failed");
        // record.status = Failed + record.error +
        // record.duration_ms = Some(duration_ms) → save → return
    }
}
```

- span の enter guard は event emit を含む match 終了まで保持し、
  event が stage span 配下にネストするようにする
- **span 生成順序(patterns to follow)**: dispatch span・stage
  span・内部 span は subscriber init(`try_init` 成功)の後に
  生成する。init 前に作られた span は default dispatcher 側で
  inert(disabled span)として確定し、init 後も `span.record` 等が
  no-op のままになる。各 `cmd_*` では `init_logging` の呼出しを
  dispatch span 生成より必ず先に行う
- `stage.run` が panic した場合は unwind で計時注入の適用範囲外
  (`duration_ms`・event とも記録されない。spec 確定済み)
- stage 名は `serde_json::to_value` による snake_case kind 文字列
  (al-cli `StubStage` と同じ変換規約)

### manifest 変更

- `crates/al-job/src/manifest.rs` の `StageRecord` に
  `#[serde(default)] pub duration_ms: Option<u64>` を追加
  (missing→None は serde 既定だが、旧 manifest 読込互換の意図明示
  として付ける。ADR 確定済み)。未実行・旧 manifest 由来は `None`
- `Manifest::new` の初期化リテラルと `runner.rs` の
  `unwrap_or_else` fallback リテラルに `duration_ms: None` を追加
- `registry.rs:77` のテスト用 `StageRecord` リテラルに
  `duration_ms: None` を追加
- 意味は「その stage の最新実行の経過時間(ms)」。resume で再実行
  された stage は新値で上書き(合算しない)、Completed/Skipped で
  実行されない stage は既存値を保持する
- manifest 書込みは既存の `manifest.save` タイミング(各 stage
  遷移後)に乗せ、新しい永続化経路は増やさない

### span 配置マップ

span 名は全て `&'static str` リテラル。stage 境界は固定名
`"stage"` + field、stage 内部のサブ単位は `<crate>::<unit>` 名。
EnvFilter の target は span 名ではなく module path(`al_locate`
等)であり、`AL_LOG=al_locate=debug` は crate 単位の絞込みになる。
field 語彙は `job_id`・`stage`・`duration_ms`・`len`・`sha256`・
`input_id`・`method` を共通利用し、run 単位の span/event には
`job_id` を必須 field とする。

| crate | 配置箇所 | span 名 | 主な field |
| --- | --- | --- | --- |
| al-cli | 各 `cmd_*` 本体を囲む dispatch span | `cli::run` / `cli::resume` / `cli::inspect` | `job_id`(判明後 `span.record` で追記)、`len`(入力数) |
| al-job | `StageRunner::execute` の `stage.run` | `stage` | `stage`, `job_id`, event に `duration_ms` |
| al-locate | 各 `LocateMethod` 実装(`methods.rs`)の `locate`/`locate_prepared` 入口 | `locate::full_ncc`, `locate::band_ncc`, `locate::gcc_phat`, `locate::spectral_envelope` | `method`, `len`(template/signal 長) |
| al-analysis | `cheap::analyze_input` 呼出し(stage.rs) / `reference::analyze_pair` 呼出し / `expensive::analyze_region` 呼出し + 各 family 呼出し(`expensive/channel.rs` 等) | `analysis::cheap`, `analysis::reference`, `analysis::region`, `analysis::<family>`(channel・codec・dynamics・ir・noise・timing・transfer) | `input_id`, `method`, `len`, `duration_ms` |
| al-media | `IngestStage::ingest_input` の入力単位 / `DecoderRegistry::decode`・`MediaDecoder::probe` | `media::ingest`, `media::decode`, `media::probe` | `input_id`, `sha256`, `len`(byte 数), `method`(decoder id) |
| al-viz | `VisualizationStage::run` の生成物単位 / `ExportStage::run` の package・golden | `viz::<product>`(timeline・views・report・summary・tiles・lod), `viz::package`, `viz::golden` | `len`, `duration_ms` |
| al-dataset | `DatasetStage::run` の書込み単位 | `dataset::write` | `input_id`, `len`, `duration_ms` |

### 依存配線

- root `Cargo.toml`: `workspace.members` に
  `"crates/al-observability"` を追加。`[workspace.dependencies]` に
  次を追加(add 時点で crates.io 公開から ≥7 日経過した版を pin):
  - `al-observability = { path = "crates/al-observability" }`
  - `tracing`(leaf crate 群 + al-cli + al-observability が参照)
  - `tracing-subscriber`(feature: `fmt`, `env-filter`, `json`,
    `ansi` — `json` は file sink の JSON-lines に必須、`ansi` は
    `with_ansi` メソッドが `ansi` feature 依存のため必須)
  - `tracing-appender`(al-observability のみ)
- `crates/al-observability/Cargo.toml`: `tracing`・
  `tracing-subscriber`・`tracing-appender` を `.workspace = true` で
  参照
- leaf crate(al-cli・al-job・al-locate・al-analysis・al-media・
  al-viz・al-dataset)の `[dependencies]` に `tracing.workspace =
  true` を追加。al-cli はさらに `al-observability.workspace = true`
- `crates/al-cli` の `[dev-dependencies]` に
  `tracing-subscriber.workspace = true` を追加する。metadata-only
  capture test(後述)が capture subscriber + 自前 Layer を組む
  ために必要。dev-dep のため「leaf crate は normal dep で
  `tracing-subscriber` に依存しない」不変条件は維持される
- al-core・al-math・al-testkit には tracing 系依存を追加しない
  (計装対象外。al-math の criterion dev-dep は次項のとおり
  bench 用途のみ)
- `crates/al-math`: dev-dependency に criterion を追加(≥7d 経過版
  を pin、採用版と公開日は Implementation Record に記録)。
  `[[bench]]` で `harness = false` を設定し、bench target として
  `crates/al-math/benches/dsp_smoke.rs` を新設する。既存の公開
  カーネル(`al_math::filter::spectral_envelope` または
  `al_math::correlation::ncc` のような既存 dispatch 経路の
  カーネル)に対する最小 criterion ベンチを1本置く。vec
  プリミティブ本番ベンチ(計測対象・項目の選定を含む)は次
  chain の `al-math-simd-reduction` で追加するため、ここでは
  smoke 用の1本に留める
- samply 手順: `docs/guides/` を新設し、
  `docs/guides/profiling-samply.md` に samply の使い方(導入
  方法・`[profile.release] debug = true` の設定・実行例・
  `.samply` 出力の見方)を記述する(CPU sampling、外部ツール
  随時、計装不要。文書の作成自体は次 task の作業)

## Data and Control Flow

- **activation → init**: 各 `cmd_*` が argv(`parse_flag`)と
  env(`std::env::var`)の生値を `init_logging` へ渡す →
  `resolve_activation` が per-concern 全体置換で directive を決め、
  `EnvFilter::try_new`/`try_from` 系で parse(parse 失敗・未知
  sink/format・壊れた directive は `LogConfigError` → `fail()` の
  `error:` 行 + exit 2)→ `init` が EnvFilter + stderr layer
  (+ file layer)を組み `try_init` → `LogGuard` を cmd スコープで
  保持。`Disabled` なら subscriber を組まず leaf crate の emit は
  no-op のまま
- **file degrade**: file sink の構築(`rolling::Builder::build`、
  `io::Result`)が失敗した場合のみ `eprintln!` の warn 行 +
  stderr-only subscriber。job 実行は継続(`non_blocking` は失敗
  しないため degrade 対象外)
- **stage 計時**: `execute` が stage ごとに `info_span!("stage")` を
  enter → `Instant::now` → `stage.run` → 成否両分岐で
  `elapsed().as_millis()` を `record.duration_ms` に設定し、
  `info!`/`error!` event を emit → 既存の `manifest.save` で
  永続化。log OFF でも manifest への記録は常時行われる
- **stdout 非汚染**: tracing 行は stderr と file のみ。`fail()` の
  `error:` 行とは stderr 上で行単位 interleave で共存する

## Non-Functional Constraints

- 既定 OFF で実用上 zero overhead(subscriber 未設定時の tracing
  no-op コストのみ)。OFF 時に `<base_dir>/logs/` を作らない
- `run`/`resume` の key=value 行と `inspect` の stdout JSON schema
  を変更しない(`inspect` には `StageRecord.duration_ms` field 追加
  のみ差分許容。`Option::None` は `null` で serialize される)
- file sink init 失敗は job をエラーにしない(「log 機構は job を
  壊さない」不変条件)
- metadata-only: field は len・件数・hash・ID・stage 名・method 名
  のみ。raw PCM・波形データ・巨大バッファは記録しない
- span 名は `&'static str` で実行時動的生成をしない
- 旧 manifest(`duration_ms` なし)の `Manifest::load` 成功を維持
- 依存方向は `al-cli → al-observability → tracing-subscriber/
  tracing-appender` と `leaf → tracing` の2系統のみ。leaf crate は
  `al-observability`・`tracing-subscriber`・`tracing-appender` に
  normal dep として直接依存しない(al-cli の
  `tracing-subscriber` は dev-dep でのみ許容)

## Risks and Trade-offs

- `non_blocking` writer の flush は `LogGuard`(内部の
  `WorkerGuard`)の Drop 依存 → guard を cmd 関数スコープで保持する
  配線を必須とし、`init` の戻り値型を `LogGuard` にして drop 忘れ
  を型で防ぐ。早期 `return fail(e)` でも Drop で flush される
- `EnvFilter` の parse エラーは生 directive 文字列を保持しない
  ため、`resolve_activation` 側で生値を error message に含める
  (デバッグ時に入力値が見えない事態を避ける)
- 同一 `<base_dir>` で複数 `al` プロセスが同日 log file に append
  し得る(ADR 確定済みの既知制約。行書込み原子性は
  tracing-appender 依存、他プロセス open 中の retention 削除は
  Windows で失敗し得る)→ 対策せず既知制約として引き継ぐ
- `stderr=json` 指定を許容するが `fail()` の `error:` 行は
  plaintext のまま残るため stderr は JSON-lines として parse 不能
  → JSON 機械処理の責務は file sink のみ(ADR 確定済み)
- `activation` 未指定時に format directive を解釈しない設計では、
  `AL_LOG_FORMAT` に壊れた値が残っていても既定 OFF の実行は
  影響を受けない。誤指定の気づきやすさより既定 OFF の堅牢性を優先
  する(spec の「`--log-format` 単独では有効化されない」の解釈)
- boxed layer による型消去の動的 dispatch は微小コスト → per-sink
  format の表現力を優先

## Deferred Design Concerns

- raw payload capture の opt-in | log で原因追跡に raw データが
  必要な事案が出た時点 | 再現に別手段(手動再実行・artifact 検査)
  が要る
- CI ベンチ・回帰ゲート | CI 基盤自体が未整備 | criterion の
  baseline 比較を載せるか CI 整備時に再検討
- 常時計装プロファイラ(puffin/tracy) | sampling で分解能が不足
  する調査が出た時点 | ホットスポット内訳が samply の分解能に
  依存する
- export package への stage duration 同梱 | package は job manifest
  を含まず analysis dataset のみをコピーする境界を維持 | package
  側で実行時間を表示したい要求が出た時点で job manifest コピー
  追加を検討
- `stage` event の span ネスト構造の厳密化(span close 時刻で
  duration を取る Layer 方式等) | 現設計の `duration_ms` field で
  要求を満たす | 解析ツール側が span 経過時間を使いたい要求が
  出た時点で再検討

## Verification Notes

- [ ] `AL_LOG`/`--log` 未指定(既定 OFF)で
  `mise exec -- cargo test --workspace` が全件 pass し、
  `run`/`resume` の stdout 形式が不変、`inspect` の stdout が
  `duration_ms` field 追加のみの差分
- [ ] `--log` または `AL_LOG` 指定時、`run`/`resume` で stderr と
  `<base_dir>/logs/al.YYYY-MM-DD.log` の両 sink に stage 完了
  event が出、`inspect` では stderr のみで `<base_dir>/logs/` に
  file が作られない
- [ ] `AL_LOG` 空文字は未指定扱いで OFF、`--log-format` 単独では
  有効化されない
- [ ] 値なし `--log`/`--log-format` が `error: --log requires a
  value`/`error: --log-format requires a value` + exit 2。空文字値
  (`--log ""`/`--log-format ""`)・未知の sink 名(`stdout` 含む)・
  未知 format 値・壊れた directive が `error:` 行 + exit 2 で
  fail-fast し panic しない(`AL_LOG`/`AL_LOG_FORMAT` の空文字は
  未指定扱いで OFF のまま)
- [ ] `--log-format`/`AL_LOG_FORMAT` で per-sink format が独立に
  切り替わり file=JSON-lines + stderr=text の同時併用が動作する。
  flag が env を per-concern 全体置換で上書きする
- [ ] `AL_LOG=al_locate=debug` で `locate::full_ncc` 等の内部 span
  event が観測できるなど、計装対象各 crate の内部 span が観測
  できる
- [ ] file sink init 失敗環境で job が成功し、stderr warn 行 +
  stderr-only degrade になる
- [ ] metadata-only 規約の regression test(capture subscriber +
  自前 Layer で event field を収集し、1 KiB 超の field 値や既知の
  巨大データ field が無いことを assert)が al-cli の統合テスト
  (`crates/al-cli/tests/` 配下、al-cli の dev-dep に
  `tracing-subscriber` を追加して配置)に存在し pass する
- [ ] `manifest.json` の完了 `StageRecord` に `duration_ms` が
  `Some` で記録され(log 無効時を含む)、`duration_ms` を持たない
  旧 manifest の `Manifest::load` が成功する
- [ ] resume で再実行 stage の `duration_ms` が上書きされ、未実行
  stage の値が保持される test
- [ ] `mise exec -- cargo tree --workspace --edges normal` で
  normal dep のみ確認し、`tracing-subscriber`/`tracing-appender`
  が `al-observability` 経由のみに現れ、leaf crate が直接依存
  しない(al-cli の `tracing-subscriber` は dev-dep のため
  `--edges normal` では現れない前提に合わせる)
- [ ] 統合テストが `Command::env_remove` で `AL_LOG`/
  `AL_LOG_FORMAT` を子プロセス側から除去している
- [ ] `mise exec -- cargo bench -p al-math` で
  `crates/al-math/benches/dsp_smoke.rs` の criterion ベンチが
  実行でき、採用版と公開日が記録されている。samply 手順文書が
  `docs/guides/profiling-samply.md` に存在する
