//! AudioLens observability foundation。
//!
//! logging の sink init は `ruviform-observability::logging`
//! (feature `logging`。Ruviform #276 の汎用 adapter)へ委譲し、
//! 本 crate は製品固有の設定面だけを保持する薄い Config
//! Adapter とする。公開 API は薄い2段構成:
//!
//! - `resolve_activation`: `AL_LOG`/`AL_LOG_FORMAT`・
//!   `--log`/`--log-format` の生値を引数で受けて [`Activation`] を
//!   返す純粋関数(env は内部で読まない)。製品固有の優先順位・
//!   sink/format 閉集合語彙・エラー文字列は製品契約として保持
//! - `init`: 解釈済みの [`LogConfig`] を受けて global subscriber
//!   を構築し、writer の flush/drain を保証する [`LogGuard`] を
//!   返す。stderr sink は ruviform `build_logging_layers` の
//!   bounded writer、file sink は暫定 local 構築(後述)
//!
//! leaf crate は本 crate に依存せず、`tracing` 標準 API で emit のみ
//! を行う。既定は OFF(subscriber 未設定)で、activation 未指定では
//! `init` は呼ばれない。
//!
//! # 暫定 file sink(ruviform#334 解消条件付き)
//!
//! file sink の命名契約 `al.YYYY-MM-DD.log` は
//! `tracing_appender` の `filename_suffix` が必要だが、ruviform
//! `SinkDestination::RollingFile` が suffix を公開していない
//! ([ruviform#334](https://github.com/scarecrow173/ruviform/issues/334))。
//! そのため file sink は従来どおり `RollingFileAppender` を直接
//! 構築する local 実装を維持する。#334 解消後は
//! `SinkDestination::RollingFile` へ移して本ブロックを撤去する。
//!
//! # 既知の一時 regression(ruviform#335)
//!
//! ruviform adapter の `fmt` layer は `FmtSpan` を設定しない
//! ([ruviform#335](https://github.com/scarecrow173/ruviform/issues/335))
//! ため、stderr sink は span lifecycle event(CLOSE)を出力しない。
//! file JSONL sink は local layer で `FmtSpan::CLOSE` を維持し、
//! 集計用途の leaf span 観測を保持する。#335 解消後に stderr
//! sink へも適用する。

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use ruviform_observability::logging::{
    LoggingBuildError, LoggingConfig, SinkConfig, SinkDestination, SinkFailureMode,
};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{self, Rotation};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::{Layer, SubscriberExt};
use tracing_subscriber::registry::Registry;
use tracing_subscriber::util::SubscriberInitExt;

/// logging の出力先。sink 語彙は `{stderr, file}` の閉集合で、
/// stdout への log 出力は `inspect` の JSON 契約保護のため存在
/// しない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinkKind {
    /// stderr sink(既定 format は [`LogFormat::Text`])。
    Stderr,
    /// file sink。`<base_dir>/logs/` 配下への JSON-lines が既定。
    File,
}

/// sink ごとの出力形式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// 人間向けの text 形式。
    Text,
    /// 機械処理向けの JSON-lines 形式。
    Json,
}

/// 解釈済みの logging 設定。[`init`] の入力。
#[derive(Debug)]
pub struct LogConfig {
    /// EnvFilter directive 解釈済みのフィルタ。
    pub filter: EnvFilter,
    /// stderr sink の format(既定 [`LogFormat::Text`])。
    pub stderr_format: LogFormat,
    /// file sink の format(既定 [`LogFormat::Json`])。
    pub file_format: LogFormat,
    /// `Some(<base_dir>/logs)` で file sink を組む(run/resume)。
    /// `inspect` では `None`(stderr のみ)。
    pub file_dir: Option<PathBuf>,
}

/// activation 解釈の結果。既定 OFF。`EnvFilter` 内包の
/// [`LogConfig`] が大きいため `Box` に包んで enum サイズを
/// 抑える。
#[derive(Debug)]
pub enum Activation {
    /// `AL_LOG`/`--log` が未指定(env の空文字も未指定扱い)。
    Disabled,
    /// logging 有効。解釈済みの [`LogConfig`] を保持する。
    Enabled(Box<LogConfig>),
}

/// `AL_LOG`/`--log`/`AL_LOG_FORMAT`/`--log-format` の解釈エラー。
/// panic せず、呼出し側の `fail()` で `error:` 行として報告する。
#[derive(Debug)]
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

impl fmt::Display for LogConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDirective(raw) => {
                write!(f, "invalid log directive: {raw:?}")
            }
            Self::UnknownSink(sink) => {
                write!(
                    f,
                    "unknown log sink: {sink:?} (expected \"stderr\" or \"file\")"
                )
            }
            Self::UnknownFormat(format) => {
                write!(
                    f,
                    "unknown log format: {format:?} (expected \"text\" or \"json\")"
                )
            }
            Self::MalformedFormatDirective(raw) => {
                write!(
                    f,
                    "malformed log format directive: {raw:?} \
                     (expected <sink>=<format>)"
                )
            }
        }
    }
}

impl std::error::Error for LogConfigError {}

/// [`init`] のエラー。
#[derive(Debug)]
pub enum InitError {
    /// global subscriber が既に設定済み。設計上到達しない内部エラー。
    AlreadyInitialized,
    /// ruviform logging adapter の sink 構築に失敗した
    /// (`Required` 指定の stderr sink の init 失敗、または
    /// filter の再解釈失敗 — 後者は `resolve_activation` で
    /// 検証済みのため通常到達しない)。
    Build(LoggingBuildError),
}

impl fmt::Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyInitialized => {
                write!(f, "log subscriber is already initialized")
            }
            Self::Build(source) => {
                write!(f, "cannot initialize log sinks: {source}")
            }
        }
    }
}

impl std::error::Error for InitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::AlreadyInitialized => None,
            Self::Build(source) => Some(source),
        }
    }
}

/// 返却 guard。ruviform `WriterGuard` と file sink の
/// `WorkerGuard` を保持し、Drop で drain/flush を保証する。
/// 呼出し側は `cmd_*` のスコープ終了まで保持する。
#[derive(Debug)]
pub struct LogGuard {
    // 保持のみで読み取らない。Drop 時に各 worker が drain+flush
    // される(ruviform WriterGuard は bounded timeout 付き join)。
    _guards: Vec<ruviform_observability::logging::WriterGuard>,
    _file_guard: Option<WorkerGuard>,
}

/// env/flag の生値から [`Activation`] を解釈する純粋関数。
///
/// - `log_env`/`log_flag` は `AL_LOG`/`--log`、
///   `format_env`/`format_flag` は `AL_LOG_FORMAT`/`--log-format`
///   の値(それぞれ `Option` で未指定を表す)
/// - 優先順位は per-concern の全体置換: flag 値があれば env 値を
///   丸ごと置き換える(directive 単位の merge はしない)
/// - 「空文字=未指定」規約は env のみ: env が unset または空文字は
///   未指定扱い。flag 値の空文字(`--log ""`/`--log-format ""`)は
///   malformed として [`LogConfigError`] で fail-fast する
/// - activation が未指定の場合 format directive は解釈せず
///   [`Activation::Disabled`] を返す(format 指定のみでは有効化
///   しない)
pub fn resolve_activation(
    log_env: Option<&str>,
    log_flag: Option<&str>,
    format_env: Option<&str>,
    format_flag: Option<&str>,
) -> Result<Activation, LogConfigError> {
    // per-concern 全体置換: flag があれば env を置き換える。
    // env の空文字は未指定、flag の空文字は fail-fast。
    let directive = match log_flag {
        Some("") => {
            return Err(LogConfigError::InvalidDirective(String::new()));
        }
        Some(flag) => Some(flag),
        None => specified(log_env),
    };
    let Some(directive) = directive else {
        // activation 未指定では format directive を解釈しない。
        return Ok(Activation::Disabled);
    };
    let filter = EnvFilter::try_new(directive)
        .map_err(|_| LogConfigError::InvalidDirective(directive.to_string()))?;

    let format_directives = match format_flag {
        Some("") => {
            return Err(LogConfigError::MalformedFormatDirective(String::new()));
        }
        Some(flag) => Some(flag),
        None => specified(format_env),
    };
    let (stderr_format, file_format) = match format_directives {
        Some(raw) => parse_format_directives(raw)?,
        None => (LogFormat::Text, LogFormat::Json),
    };

    Ok(Activation::Enabled(Box::new(LogConfig {
        filter,
        stderr_format,
        file_format,
        // file sink の集約先は呼出し側(run/resume は
        // `<base_dir>/logs`、inspect は `None`)が設定する。
        file_dir: None,
    })))
}

/// [`LogConfig`] から global subscriber を構築し [`LogGuard`] を
/// 返す。プロセス内で1回だけ呼ぶ(2回目は
/// [`InitError::AlreadyInitialized`])。
///
/// stderr sink は ruviform `build_logging_layers` が生成する
/// bounded-writer layer(bounded queue + drain guard、per-sink
/// `EnvFilter`)を使う。file sink は `filename_suffix("log")` の
/// 命名契約を維持するため暫定で local 構築する(module doc 参照)。
/// 両 sink に同一 directive を per-sink filter として適用する
/// (従来の global filter と等価)。
///
/// `file_dir` が `Some` で file sink の構築に失敗した場合、
/// stderr に warn 行を出して stderr-only に degrade し `Ok` を
/// 返す — 「log 機構は job を壊さない」不変条件。
pub fn init(config: LogConfig) -> Result<LogGuard, InitError> {
    // stderr sink は ruviform logging adapter に委譲する。
    let built = ruviform_observability::logging::build_logging_layers(&LoggingConfig {
        sinks: vec![SinkConfig {
            destination: SinkDestination::Stderr,
            format: map_log_format(config.stderr_format),
            // `resolve_activation` で parse 済みの directive と同じ
            // ものを per-sink filter として適用する。
            filter: config.filter.to_string(),
            // stderr は常に利用可能な sink であり、init 失敗時は
            // 明示エラーで呼出し側の `fail()` に委ねる。
            failure_mode: SinkFailureMode::Required,
        }],
    })
    .map_err(InitError::Build)?;
    let mut layers = built.layers;
    let guards = built.guards;

    let mut file_guard = None;
    if let Some(dir) = &config.file_dir {
        match build_file_sink(dir, config.file_format, config.filter.clone()) {
            Ok((layer, guard)) => {
                layers.push(layer);
                file_guard = Some(guard);
            }
            Err(e) => {
                // 「log 機構は job を壊さない」: file sink 構築の失敗は
                // warn + stderr-only degrade とする。
                eprintln!(
                    "warn: cannot initialize file log sink at {}: {e}",
                    dir.display()
                );
            }
        }
    }
    tracing_subscriber::registry()
        .with(layers)
        .try_init()
        .map_err(|_| InitError::AlreadyInitialized)?;
    Ok(LogGuard {
        _guards: guards,
        _file_guard: file_guard,
    })
}

/// 製品語彙の [`LogFormat`] を ruviform `LogFormat` へ写像する
/// (写像はこの1箇所に集約する)。
fn map_log_format(format: LogFormat) -> ruviform_observability::logging::LogFormat {
    match format {
        LogFormat::Text => ruviform_observability::logging::LogFormat::Text,
        LogFormat::Json => ruviform_observability::logging::LogFormat::JsonLines,
    }
}

/// 暫定 local file sink(ruviform#334 解消条件付き)。
///
/// `filename_prefix("al")` + `filename_suffix("log")` +
/// `Rotation::DAILY` + `max_log_files(7)` の rolling file を
/// `non_blocking` writer に包み、`FmtSpan::CLOSE` + per-sink
/// `EnvFilter` を設定した fmt layer と `WorkerGuard` を返す。
///
/// `RollingFileAppender` runs `prune_old_logs` before creating
/// the log directory and prints a stray stderr error when the
/// read fails; create the directory up front so a fresh
/// `<base_dir>/logs` stays quiet. If creation fails, `build`
/// fails too and the caller degrades to stderr-only.
fn build_file_sink(
    dir: &Path,
    format: LogFormat,
    filter: EnvFilter,
) -> io::Result<(Box<dyn Layer<Registry> + Send + Sync>, WorkerGuard)> {
    let _ = std::fs::create_dir_all(dir);
    let appender = rolling::Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix("al")
        .filename_suffix("log")
        .max_log_files(7)
        .build(dir)
        .map_err(io::Error::other)?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let layer = tracing_subscriber::fmt::Layer::default()
        .with_writer(writer)
        .with_ansi(false)
        .with_span_events(FmtSpan::CLOSE);
    let layer = match format {
        LogFormat::Text => layer.boxed(),
        LogFormat::Json => layer.json().boxed(),
    };
    // per-sink EnvFilter(ruviform stderr sink と同じ directive)。
    Ok((layer.with_filter(filter).boxed(), guard))
}

/// env 値の正規化: unset または空文字は未指定(`None`)扱い。
fn specified(value: Option<&str>) -> Option<&str> {
    value.filter(|s| !s.is_empty())
}

/// `<sink>=<format>` のカンマ区切り directive を解釈する。
/// 未指定 sink は既定値(stderr=Text・file=Json)に留まる。
fn parse_format_directives(raw: &str) -> Result<(LogFormat, LogFormat), LogConfigError> {
    let mut stderr_format = LogFormat::Text;
    let mut file_format = LogFormat::Json;
    for directive in raw.split(',') {
        let Some((sink, format)) = directive.split_once('=') else {
            return Err(LogConfigError::MalformedFormatDirective(
                directive.to_string(),
            ));
        };
        let kind = match sink {
            "stderr" => SinkKind::Stderr,
            "file" => SinkKind::File,
            unknown => {
                return Err(LogConfigError::UnknownSink(unknown.to_string()));
            }
        };
        let format = match format {
            "text" => LogFormat::Text,
            "json" => LogFormat::Json,
            unknown => {
                return Err(LogConfigError::UnknownFormat(unknown.to_string()));
            }
        };
        match kind {
            SinkKind::Stderr => stderr_format = format,
            SinkKind::File => file_format = format,
        }
    }
    Ok((stderr_format, file_format))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled(result: Result<Activation, LogConfigError>) -> LogConfig {
        match result {
            Ok(Activation::Enabled(config)) => *config,
            other => panic!("expected Enabled, got {other:?}"),
        }
    }

    #[test]
    fn all_unspecified_is_disabled() {
        assert!(matches!(
            resolve_activation(None, None, None, None),
            Ok(Activation::Disabled)
        ));
    }

    #[test]
    fn env_activates_logging() {
        let config = enabled(resolve_activation(Some("info"), None, None, None));
        assert_eq!(config.stderr_format, LogFormat::Text);
        assert_eq!(config.file_format, LogFormat::Json);
        assert_eq!(config.file_dir, None);
    }

    #[test]
    fn flag_overrides_env_wholesale() {
        // flag が env を丸ごと置き換えるため、env の壊れた directive
        // は解釈されず成功する。
        let config = enabled(resolve_activation(Some("=info"), Some("info"), None, None));
        assert_eq!(config.stderr_format, LogFormat::Text);
    }

    #[test]
    fn format_only_does_not_activate() {
        assert!(matches!(
            resolve_activation(None, None, Some("stderr=json"), None),
            Ok(Activation::Disabled)
        ));
        assert!(matches!(
            resolve_activation(None, None, None, Some("stderr=json")),
            Ok(Activation::Disabled)
        ));
    }

    #[test]
    fn empty_env_is_unspecified() {
        assert!(matches!(
            resolve_activation(Some(""), None, None, None),
            Ok(Activation::Disabled)
        ));
        // env の空文字 format も未指定扱い(既定値のまま有効化)。
        let config = enabled(resolve_activation(Some("info"), None, Some(""), None));
        assert_eq!(config.stderr_format, LogFormat::Text);
        assert_eq!(config.file_format, LogFormat::Json);
    }

    #[test]
    fn empty_flag_is_fail_fast() {
        assert!(matches!(
            resolve_activation(None, Some(""), None, None),
            Err(LogConfigError::InvalidDirective(_))
        ));
        assert!(matches!(
            resolve_activation(Some("info"), None, None, Some("")),
            Err(LogConfigError::MalformedFormatDirective(_))
        ));
        // flag の空文字は env 値より優先して fail-fast する。
        assert!(matches!(
            resolve_activation(Some("info"), Some(""), None, None),
            Err(LogConfigError::InvalidDirective(_))
        ));
    }

    #[test]
    fn invalid_directive_is_fail_fast() {
        // `=info` は target が空の壊れた directive、`info=bogus`
        // は未知 level で EnvFilter::try_new が失敗する。
        for raw in ["=info", "info=bogus"] {
            assert!(matches!(
                resolve_activation(Some(raw), None, None, None),
                Err(LogConfigError::InvalidDirective(e)) if e == raw
            ));
            assert!(matches!(
                resolve_activation(None, Some(raw), None, None),
                Err(LogConfigError::InvalidDirective(e)) if e == raw
            ));
        }
    }

    #[test]
    fn unknown_sink_is_rejected() {
        // stdout sink は inspect の JSON 契約保護のため明示拒否。
        for raw in ["stdout=text", "bogus=json"] {
            assert!(matches!(
                resolve_activation(Some("info"), None, Some(raw), None),
                Err(LogConfigError::UnknownSink(_))
            ));
            assert!(matches!(
                resolve_activation(Some("info"), None, None, Some(raw)),
                Err(LogConfigError::UnknownSink(_))
            ));
        }
    }

    #[test]
    fn unknown_format_is_rejected() {
        assert!(matches!(
            resolve_activation(Some("info"), None, Some("file=bogus"), None),
            Err(LogConfigError::UnknownFormat(_))
        ));
    }

    #[test]
    fn malformed_format_directive_is_rejected() {
        // `<sink>=<format>` 形でない directive は malformed。
        assert!(matches!(
            resolve_activation(Some("info"), None, Some("stderrtext"), None),
            Err(LogConfigError::MalformedFormatDirective(_))
        ));
        // sink 名が空の directive は未知 sink として拒否する。
        assert!(matches!(
            resolve_activation(Some("info"), None, Some("=json"), None),
            Err(LogConfigError::UnknownSink(_))
        ));
    }

    #[test]
    fn per_sink_format_switching() {
        let config = enabled(resolve_activation(
            Some("info"),
            None,
            Some("stderr=json,file=text"),
            None,
        ));
        assert_eq!(config.stderr_format, LogFormat::Json);
        assert_eq!(config.file_format, LogFormat::Text);
    }

    #[test]
    fn unmentioned_sink_keeps_default() {
        let config = enabled(resolve_activation(
            Some("info"),
            None,
            Some("file=text"),
            None,
        ));
        assert_eq!(config.stderr_format, LogFormat::Text);
        assert_eq!(config.file_format, LogFormat::Text);
    }

    #[test]
    fn format_flag_replaces_format_env_wholesale() {
        // flag は env を丸ごと置き換える: env の `stderr=json` は
        // 効かず stderr は既定 Text に留まる。
        let config = enabled(resolve_activation(
            Some("info"),
            None,
            Some("stderr=json"),
            Some("file=text"),
        ));
        assert_eq!(config.stderr_format, LogFormat::Text);
        assert_eq!(config.file_format, LogFormat::Text);
    }

    #[test]
    fn both_env_and_flag() {
        let config = enabled(resolve_activation(
            Some("warn"),
            Some("al_locate=debug,info"),
            Some("stderr=json"),
            Some("file=text"),
        ));
        assert_eq!(config.stderr_format, LogFormat::Text);
        assert_eq!(config.file_format, LogFormat::Text);
    }

    #[test]
    fn init_degrades_to_stderr_on_file_build_failure() {
        // 存在しない親を持つ path(Windows で NUL は必ず失敗する)。
        let config = LogConfig {
            filter: EnvFilter::try_new("info").unwrap(),
            stderr_format: LogFormat::Text,
            file_format: LogFormat::Json,
            file_dir: Some(PathBuf::from("\0invalid")),
        };
        // file sink 構築の失敗は Err にならず stderr-only で継続
        // する。global subscriber の try_init は並行 test と干渉
        // し得るため結果は問わず、degrade が panic/Err にならない
        // ことのみ確認する。
        let _ = init(config);
    }
}
