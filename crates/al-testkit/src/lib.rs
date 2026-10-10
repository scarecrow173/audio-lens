//! `al-testkit`: ゴールデン / 合成 fixture の共有基盤。
//!
//! dev-dependency 専用の補助クレート。決定的(シード固定)な信号
//! 生成・変換、コンテナ fixture(AVI / WAV バイト列)、期待値検証
//! ヘルパを提供する。本番コードはこのクレートに依存しない。
//! 全てプログラム生成かつ外部アセット非依存。

pub mod expect;
pub mod media;
pub mod mp4;
pub mod signal;
pub mod temp;
