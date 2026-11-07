//! Source view model — bounded waveform/spectrum read と
//! TimeMap/PTS の写像。
//!
//! `DatasetSession::waveform`/`spectrum` が返す bounded source を
//! `ruviform-viz` の `WaveformSeries`/`Spectrogram` へ束ねる。
//! 読み出しは常に範囲指定で、長尺 Dataset の全量ロードをしない。

use std::ops::Range;
use std::sync::Arc;

use al_locate::LocateResult;
use al_media::{StreamTiming, TimestampOrigin, TimingObservation};
use al_service::dataset::DatasetSession;
use ruviform_viz::source::SpectrumSource;
use ruviform_viz::{SampleTimebase, WaveformSource};
use ruviform_viz::{Spectrogram, Tile, TileRequest, ViewBin, WaveformSeries};

use super::ViewError;

/// 波形の bounded view。`start_s`/`end_s`(秒)と表示 bins 上限で
/// `WaveformSeries::view` へ写す。`SampleTimebase` で
/// seconds↔samples を変換できる。
pub struct WaveformView {
    series: WaveformSeries,
    timebase: SampleTimebase,
    /// 総フレーム数。
    pub frames: u64,
    /// チャネル数。
    pub channels: usize,
    /// サンプルレート。
    pub sample_rate: u32,
}

impl WaveformView {
    /// `DatasetSession` の `inputs/<input_id>` source を開く。
    ///
    /// # Errors
    /// `input_id` が存在しない、または metadata が欠ける場合。
    pub fn open(session: &DatasetSession, input_id: &str) -> Result<Self, ViewError> {
        let source: Arc<dyn WaveformSource> = Arc::new(session.waveform(input_id)?);
        Self::try_from_source(source)
    }

    /// 既存の `WaveformSource` から作る(fixture 用)。
    ///
    /// # Errors
    /// `sample_rate == 0` の場合。
    pub fn try_from_source(source: Arc<dyn WaveformSource>) -> Result<Self, ViewError> {
        let info = source.info();
        if info.sample_rate == 0 {
            return Err(ViewError::Invalid(
                "waveform source lacks sample_rate".into(),
            ));
        }
        Ok(Self {
            series: WaveformSeries::on_demand(source),
            timebase: SampleTimebase::new(info.sample_rate).expect("sample_rate validated nonzero"),
            frames: info.frames,
            channels: info.channels,
            sample_rate: info.sample_rate,
        })
    }

    /// `SampleTimebase`(seconds↔samples 換算)。
    pub fn timebase(&self) -> &SampleTimebase {
        &self.timebase
    }

    /// `window` が実際に読む frame 範囲。空区間は 1 frame へ広げ、
    /// 表示 viewport と bins の範囲を一致させる。
    #[must_use]
    pub fn read_frames(&self, start_s: f64, end_s: f64) -> Range<u64> {
        let range = self.frame_range(start_s, end_s);
        range.start..range.end.max(range.start.saturating_add(1))
    }

    /// `[start_s, end_s)` を frame 範囲へ写す。両端は `frames` へ
    /// clamp し、`end` は `start` 以上に保つ。
    fn frame_range(&self, start_s: f64, end_s: f64) -> Range<u64> {
        // `seconds_to_sample` は巨大な秒入力を u64::MAX へ saturate
        // する — start も frames へ clamp しないと `start + 1` が
        // 整数 overflow で panic する(ticks() と同じ両端 clamp)。
        let start = self
            .timebase
            .seconds_to_sample(start_s.max(0.0))
            .min(self.frames);
        let end = self
            .timebase
            .seconds_to_sample(end_s.max(0.0))
            .min(self.frames)
            .max(start);
        start..end
    }

    /// `channel` の `[start_s, end_s)` を最大 `bins` 個の
    /// `ViewBin` へ集約して返す。読み出しは範囲のみ。
    ///
    /// # Errors
    /// 範囲外の channel、または source の読み出しエラー。
    pub fn window(
        &self,
        channel: usize,
        start_s: f64,
        end_s: f64,
        bins: usize,
    ) -> Result<Vec<ViewBin>, ViewError> {
        if channel >= self.channels {
            return Err(ViewError::Invalid(format!(
                "channel {channel} out of {}",
                self.channels
            )));
        }
        let range = self.read_frames(start_s, end_s);
        self.series
            .view(channel, range.start, range.end, bins.max(1))
            .map_err(|e| ViewError::Invalid(e.to_string()))
    }
}

/// スペクトルの bounded view。`TileRequest` 経由で frame/bin 範囲の
/// tile を読む(全 frame の一括取得はしない)。
pub struct SpectrumView {
    spectrogram: Spectrogram,
    /// `SpectrumInfo` の写し(frame/bin/grid/axis)。
    pub info: ruviform_viz::SpectrumInfo,
}

impl SpectrumView {
    /// `DatasetSession` の `intermediate/<input_id>/ch<channel>`
    /// source を開く。
    ///
    /// # Errors
    /// tensor が存在しない、または grid が導出不能な場合。
    pub fn open(
        session: &DatasetSession,
        input_id: &str,
        channel: usize,
    ) -> Result<Self, ViewError> {
        let source: Arc<dyn SpectrumSource> = Arc::new(session.spectrum(input_id, channel)?);
        Ok(Self {
            spectrogram: Spectrogram::new(source.clone()),
            info: source.info(),
        })
    }

    /// `intermediate` tensor の tile を読む。
    ///
    /// # Errors
    /// `req` が source 範囲を超える場合(source が clamp する)。
    pub fn tile(&self, req: &TileRequest) -> Result<Tile, ViewError> {
        self.spectrogram
            .tile(req)
            .map_err(|e| ViewError::Invalid(e.to_string()))
    }
}

/// source↔capture の sample 写像。`LocateResult` の
/// `capture_range`/`source_range` と両レートから offset+rate の
/// 1 次写像を構成する(GUI 側で alignment を再計算しない)。
#[derive(Debug, Clone)]
pub struct TimeMap {
    /// `capture_range.start`(capture samples)。
    capture_start: u64,
    /// `source_range.start`(source samples)。
    source_start: u64,
    /// `source_rate / capture_rate`。
    rate: f64,
    /// capture 側 sample rate。
    pub capture_rate: u32,
    /// source 側 sample rate。
    pub source_rate: u32,
    /// 写像の根拠となった alignment。
    pub alignment_id: String,
    /// 合議採用済みか。
    pub accepted: bool,
}

impl TimeMap {
    /// `LocateResult` と両 input の sample rate から作る。
    #[must_use]
    pub fn from_result(result: &LocateResult, source_rate: u32, capture_rate: u32) -> Self {
        Self {
            capture_start: result.capture_range.start,
            source_start: result.source_range.start,
            rate: source_rate as f64 / capture_rate.max(1) as f64,
            capture_rate,
            source_rate,
            alignment_id: result.alignment_id.as_str().to_string(),
            accepted: result.accepted,
        }
    }

    /// capture 側 sample を source 側秒へ写す。
    /// `capture_range` より前の位置は線形外挿する(負の delta)。
    #[must_use]
    pub fn capture_sample_to_source_seconds(&self, capture_sample: u64) -> f64 {
        let delta = capture_sample as f64 - self.capture_start as f64;
        (self.source_start as f64 + delta * self.rate) / self.source_rate as f64
    }
}

/// `StreamTiming` の bounded 写し — stream メタ + PTS window
/// 読み出し。全 `stream_timestamps` を保持せず index 範囲で読む。
/// `timestamp_origin`/`observation`(gap・duplicate・drop 候補)は
/// producer が計算した typed 値をそのまま露出する — GUI 側で
/// discontinuity/drop を再検出しない。
pub struct StreamTimeView {
    timing: StreamTiming,
}

impl StreamTimeView {
    /// `StreamTiming` から作る。
    #[must_use]
    pub fn new(timing: StreamTiming) -> Self {
        Self { timing }
    }

    /// ingest metadata JSON の `av_timing` から作る。
    /// `av_timing` は stream 毎(`SampleTable`/`SampleTablePts` 系なら
    /// track 毎)の `Vec<StreamTiming>` で永続化される
    /// (al-media `stage.rs`)。origin/stream で使い分けられるよう
    /// 全件を view として返す。
    ///
    /// # Errors
    /// `av_timing` が無い、または `Vec<StreamTiming>` として
    /// 解釈不能な場合。
    pub fn from_metadata(meta: &serde_json::Value) -> Result<Vec<Self>, ViewError> {
        let raw = meta
            .get("av_timing")
            .ok_or_else(|| ViewError::NotFound("metadata lacks av_timing".into()))?;
        let timings: Vec<StreamTiming> = serde_json::from_value(raw.clone())
            .map_err(|e| ViewError::Invalid(format!("av_timing is not Vec<StreamTiming>: {e}")))?;
        Ok(timings.into_iter().map(Self::new).collect())
    }

    /// 内包する `StreamTiming` への参照(origin・observation 全量)。
    pub fn timing(&self) -> &StreamTiming {
        &self.timing
    }

    /// timestamp 列の由来(`movi_order`/`sample_table`/`sample_table_pts`)。
    /// 派生 timeline と container 保存 PTS を区別する正本ラベル。
    pub fn timestamp_origin(&self) -> TimestampOrigin {
        self.timing.timestamp_origin
    }

    /// `TimingObservation`(intervals・discontinuities・duplicates・
    /// drop_candidates・drift_ppm)。
    pub fn observation(&self) -> &TimingObservation {
        &self.timing.observation
    }

    /// stream index。
    pub fn stream(&self) -> u32 {
        self.timing.stream
    }

    /// stream 種別(`vids`/`auds` 等)。
    pub fn stream_type(&self) -> &str {
        &self.timing.stream_type
    }

    /// `timebase_hz`(tick 間隔の逆数)。
    pub fn timebase_hz(&self) -> u64 {
        self.timing.observation.timebase_hz
    }

    /// `start_tick`。
    pub fn start_tick(&self) -> i64 {
        self.timing.start_tick
    }

    /// 保持している timestamp 総数。
    pub fn len(&self) -> usize {
        self.timing.stream_timestamps.len()
    }

    /// timestamp が空か。
    pub fn is_empty(&self) -> bool {
        self.timing.stream_timestamps.is_empty()
    }

    /// index 範囲 `[start, end)` の PTS window を返す(bounded)。
    /// `start`/`end` は `len` へ clamp し、`start > len` でも
    /// panic せず空 slice を返す(空 stream `ticks(1, 2)` 等)。
    #[must_use]
    pub fn ticks(&self, start: usize, end: usize) -> &[i64] {
        let start = start.min(self.timing.stream_timestamps.len());
        let end = end.min(self.timing.stream_timestamps.len()).max(start);
        &self.timing.stream_timestamps[start..end]
    }

    /// index `i` の timestamp を秒へ換算する。`timebase_hz == 0`
    /// の malformed stream には `None`(inf/NaN を捏造しない)。
    #[must_use]
    pub fn seconds_at(&self, i: usize) -> Option<f64> {
        if self.timing.observation.timebase_hz == 0 {
            return None;
        }
        self.timing
            .stream_timestamps
            .get(i)
            .map(|t| *t as f64 / self.timing.observation.timebase_hz as f64)
    }
}
