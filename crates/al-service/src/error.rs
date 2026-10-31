//! サービス層の失敗分類。GUI/CLI は `code()` で IPC/終了コードへ
//! 対応付け、メッセージをそのまま表示する。

use std::fmt;

/// サービス操作の失敗。`kind` は原因の分類で、UI の表示分岐に使う。
#[derive(Debug)]
pub struct ServiceError {
    kind: ErrorKind,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ErrorKind {
    /// 引数・パラメータ不正。
    Invalid,
    /// 対象(job・dataset・artifact)が存在しない。
    NotFound,
    /// I/O または永続フォーマットの失敗(破損・schema 不一致を含む)。
    Io,
    /// stage 実行の失敗(manifest に記録済み)。
    Stage,
    /// キャンセル要求により中断した(失敗ではない)。
    Cancelled,
}

impl ServiceError {
    /// `invalid` 分類のエラーを作る。
    pub fn invalid(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Invalid,
            message: message.into(),
        }
    }

    /// `not_found` 分類のエラーを作る。
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::NotFound,
            message: message.into(),
        }
    }

    /// キャンセル中断を表すエラーを作る。
    pub fn cancelled() -> Self {
        Self {
            kind: ErrorKind::Cancelled,
            message: "cancelled".into(),
        }
    }

    /// 分類コード: `invalid` / `not_found` / `io` / `stage` /
    /// `cancelled`。IPC の error.code 相当として使う。
    pub fn code(&self) -> &'static str {
        match self.kind {
            ErrorKind::Invalid => "invalid",
            ErrorKind::NotFound => "not_found",
            ErrorKind::Io => "io",
            ErrorKind::Stage => "stage",
            ErrorKind::Cancelled => "cancelled",
        }
    }

    /// 表示用メッセージ。
    pub fn message(&self) -> &str {
        &self.message
    }

    /// stage 失敗からの変換(内部利用)。
    pub(crate) fn from_stage(err: al_job::StageError) -> Self {
        Self {
            kind: ErrorKind::Stage,
            message: format!("{:?}: {}", err.stage, err.message),
        }
    }
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ServiceError {}

impl From<std::io::Error> for ServiceError {
    fn from(err: std::io::Error) -> Self {
        let kind = if err.kind() == std::io::ErrorKind::NotFound {
            ErrorKind::NotFound
        } else {
            ErrorKind::Io
        };
        Self {
            kind,
            message: err.to_string(),
        }
    }
}
