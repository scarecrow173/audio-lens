//! AudioLens observability foundation。
//!
//! logging の sink init・format 配線・routing・activation 解釈を
//! 所有する。公開 API は薄い2段構成:
//!
//! - `resolve_activation`: `AL_LOG`/`AL_LOG_FORMAT`・
//!   `--log`/`--log-format` の生値を引数で受けて [`Activation`] を
//!   返す純粋関数(env は内部で読まない)
//! - `init`: 解釈済みの [`LogConfig`] を受けて global subscriber
//!   を構築し、file writer の flush を保証する [`LogGuard`] を返す
//!
//! leaf crate は本 crate に依存せず、`tracing` 標準 API で emit のみ
//! を行う。既定は OFF(subscriber 未設定)で、activation 未指定では
//! `init` は呼ばれない。

use std::fmt;
use std::path::PathBuf;

use tracing::Subscriber;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{self, RollingFileAppender, Rotation};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::{Layer, SubscriberExt};
use tracing_subscriber::registry::{LookupSpan, Registry};
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

/// activation 解釈の結果。既定 OFF。
// `EnvFilter` 内包の `LogConfig` が大きいが、activation 解釈結果の
// 公開 API 形態として `Enabled(LogConfig)` を維持する(design
// 規定)。`init` へ受け渡す1回きりの値であり、サイズ差は問題に
// ならない。
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum Activation {
    /// `AL_LOG`/`--log` が未指定(env の空文字も未指定扱い)。
    Disabled,
    /// logging 有効。解釈済みの [`LogConfig`] を保持する。
    Enabled(LogConfig),
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
}

impl fmt::Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyInitialized => {
                write!(f, "log subscriber is already initialized")
            }
        }
    }
}

impl std::error::Error for InitError {}

/// 返却 guard。file sink の `WorkerGuard` を保持し、Drop で flush
/// を保証する。呼出し側は `cmd_*` のスコープ終了まで保持する。
#[derive(Debug)]
pub struct LogGuard {
    // 保持のみで読み取らない。Drop 時に worker が flush される。
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

    Ok(Activation::Enabled(LogConfig {
        filter,
        stderr_format,
        file_format,
        // file sink の集約先は呼出し側(run/resume は
        // `<base_dir>/logs`、inspect は `None`)が設定する。
        file_dir: None,
    }))
}

/// [`LogConfig`] から global subscriber を構築し [`LogGuard`] を
/// 返す。プロセス内で1回だけ呼ぶ(2回目は
/// [`InitError::AlreadyInitialized`])。
///
/// `file_dir` が `Some` で file sink の構築(`rolling::Builder::build`)
/// に失敗した場合、stderr に warn 行を出して stderr-only に
/// degrade し `Ok` を返す — 「log 機構は job を壊さない」不変条件。
pub fn init(config: LogConfig) -> Result<LogGuard, InitError> {
    let mut layers: Vec<Box<dyn Layer<Registry> + Send + Sync>> =
        vec![stderr_layer(config.stderr_format)];
    let mut file_guard = None;
    if let Some(dir) = &config.file_dir {
        // `RollingFileAppender` runs `prune_old_logs` before creating
        // the log directory and prints a stray stderr error when the
        // read fails; create the directory up front so a fresh
        // `<base_dir>/logs` stays quiet. If creation fails, `build`
        // fails too and degrades to stderr-only below.
        let _ = std::fs::create_dir_all(dir);
        match rolling::Builder::new()
            .rotation(Rotation::DAILY)
            .filename_prefix("al")
            .filename_suffix("log")
            .max_log_files(7)
            .build(dir)
        {
            Ok(appender) => {
                let (layer, guard) = file_layer(config.file_format, appender);
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
        .with(config.filter) // EnvFilter は末尾に global filter として
        .try_init()
        .map_err(|_| InitError::AlreadyInitialized)?;
    Ok(LogGuard {
        _file_guard: file_guard,
    })
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

/// stderr sink の fmt layer を構築する。text/json の分岐で具象型が
/// 変わるため boxed layer を返す。
fn stderr_layer<S>(format: LogFormat) -> Box<dyn Layer<S> + Send + Sync>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    // CLOSE span events make leaf-crate `info_span!` instrumentation
    // observable (`AL_LOG=al_locate=debug` shows `locate::*` close
    // events with elapsed time); without them a span with no inner
    // events would leave no trace in either sink.
    let layer = tracing_subscriber::fmt::Layer::default()
        .with_writer(std::io::stderr)
        .with_span_events(FmtSpan::CLOSE);
    match format {
        LogFormat::Text => layer.boxed(),
        LogFormat::Json => layer.json().boxed(),
    }
}

/// file sink の fmt layer を構築する。`non_blocking` writer に包み、
/// flush 保証の `WorkerGuard` を layer とともに返す。
fn file_layer<S>(
    format: LogFormat,
    appender: RollingFileAppender,
) -> (Box<dyn Layer<S> + Send + Sync>, WorkerGuard)
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let layer = tracing_subscriber::fmt::Layer::default()
        .with_writer(writer)
        .with_ansi(false)
        .with_span_events(FmtSpan::CLOSE);
    let layer = match format {
        LogFormat::Text => layer.boxed(),
        LogFormat::Json => layer.json().boxed(),
    };
    (layer, guard)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled(result: Result<Activation, LogConfigError>) -> LogConfig {
        match result {
            Ok(Activation::Enabled(config)) => config,
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
