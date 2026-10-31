//! `DatasetReader` 上の `ruviform-viz` port 実装。表示に必要な
//! 範囲だけを range read で読み、全量ロードしない。
//!
//! - `DatasetWaveform`: `inputs/<id>.wav` → [`WaveformSource`]
//! - `DatasetSpectrum`: `intermediate/<input>/ch<N>/stft_re|stft_im`
//!   (永続複素 STFT) → [`SpectrumSource`]。STFT の再計算は行わない。

use std::ops::Range;
use std::sync::Arc;

use al_dataset::reader::DatasetReader;
use ruviform_viz::error::{Result, VizError};
use ruviform_viz::source::{
    BinAxis, PcmBlock, SpectrumData, SpectrumInfo, SpectrumSource, SpectrumTile, WaveformInfo,
    WaveformSource,
};
use ruviform_viz::timeline::FrameGrid;

fn source_err(err: impl std::fmt::Display) -> VizError {
    VizError::Source(err.to_string())
}

/// `inputs` ロールの WAV artifact を range read する
/// [`WaveformSource`]。`DatasetReader` は read が全て `&self` で
/// Sync なので `Arc` 共有で port 契約を満たす。
pub struct DatasetWaveform {
    reader: Arc<DatasetReader>,
    name: String,
    info: WaveformInfo,
}

impl DatasetWaveform {
    /// `inputs/<name>.wav` を開く。`name` は input_id。
    /// フレーム数は WAV ヘッダ、rate/channels は同梱の
    /// `<name>.metadata.json` から取得する。
    pub fn open(reader: Arc<DatasetReader>, name: &str) -> std::io::Result<Self> {
        let frames = reader.audio_frames("inputs", name)?;
        let meta = reader.read_json("inputs", &format!("{name}.metadata"))?;
        // ingest metadata は `audio.{sample_rate,channels}` に保持する。
        let channels = meta["audio"]["channels"].as_u64().unwrap_or(0) as usize;
        let sample_rate = meta["audio"]["sample_rate"].as_u64().unwrap_or(0) as u32;
        if channels == 0 || sample_rate == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("audio metadata for {name} lacks channels/sample_rate"),
            ));
        }
        Ok(Self {
            reader,
            name: name.to_string(),
            info: WaveformInfo {
                sample_rate,
                channels,
                frames,
            },
        })
    }
}

impl WaveformSource for DatasetWaveform {
    fn info(&self) -> WaveformInfo {
        self.info
    }

    fn read(&self, start: u64, frames: usize) -> Result<PcmBlock> {
        if frames == 0 || start >= self.info.frames {
            return Ok(PcmBlock {
                start,
                frames: 0,
                samples: Vec::new(),
            });
        }
        let end = start.saturating_add(frames as u64).min(self.info.frames);
        let audio = self
            .reader
            .read_audio_range("inputs", &self.name, start, end)
            .map_err(source_err)?;
        Ok(PcmBlock {
            start,
            frames: (end - start) as usize,
            samples: audio.samples,
        })
    }
}

/// `intermediate/<input>/ch<N>/stft_re|stft_im` を領域読みする
/// [`SpectrumSource`]。永続された複素 STFT をそのまま使い、
/// STFT の再計算は行わない(再解析禁止の契約)。
///
/// tensor は `[frame][bin]` (frame-major) で bin は `fft_size` 全域、
/// frame `f` は sample `f * hop` 始点 — `cheap::stft` の規約を
/// `FrameGrid`/`BinAxis` にそのまま写す。
pub struct DatasetSpectrum {
    reader: Arc<DatasetReader>,
    /// `intermediate` 配下の tensor 名プレフィックス
    /// (例: `<input>/ch0`)。
    prefix: String,
    info: SpectrumInfo,
}

impl DatasetSpectrum {
    /// `intermediate/<input_id>/ch<channel>` の複素 STFT を開く。
    /// grid は tensor shape と dataset config
    /// (`analysis.cheap.stft_*`)から導出される。
    pub fn open(
        reader: Arc<DatasetReader>,
        input_id: &str,
        channel: usize,
    ) -> std::io::Result<Self> {
        let prefix = format!("{input_id}/ch{channel}");
        let re = reader.open_tensor("intermediate", &format!("{prefix}/stft_re"))?;
        let im = reader.open_tensor("intermediate", &format!("{prefix}/stft_im"))?;
        let shape = re.meta().shape.clone();
        if shape.len() != 2 || im.meta().shape != shape {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("stft tensors for {prefix} missing or shape-mismatched"),
            ));
        }
        let (window, hop) = stft_grid(&reader);
        let bins = shape[1] as usize;
        let sample_rate = audio_sample_rate(&reader, input_id).unwrap_or(48_000);
        Ok(Self {
            reader,
            prefix,
            info: SpectrumInfo {
                frames: shape[0],
                bins,
                grid: FrameGrid::new(window, hop, 0).map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
                })?,
                // 永続 tensor は `fft_size` 全域の bin を保持する
                // (`StftMatrix` 規約)。bin 間隔は rate / fft_size。
                axis: BinAxis::Uniform {
                    first_hz: 0.0,
                    step_hz: sample_rate as f64 / bins.max(1) as f64,
                },
                sample_rate,
            },
        })
    }
}

impl SpectrumSource for DatasetSpectrum {
    fn info(&self) -> SpectrumInfo {
        self.info.clone()
    }

    fn read_tile(&self, frames: Range<u64>, bins: Range<usize>) -> Result<SpectrumTile> {
        let f_end = frames.end.min(self.info.frames);
        let f_start = frames.start.min(f_end);
        let b_end = bins.end.min(self.info.bins);
        let b_start = bins.start.min(b_end);
        let n_frames = (f_end - f_start) as usize;
        let n_bins = b_end - b_start;
        let data = if n_frames == 0 || n_bins == 0 {
            SpectrumData::Complex {
                re: Vec::new(),
                im: Vec::new(),
            }
        } else {
            let region = [[f_start, f_end], [b_start as u64, b_end as u64]];
            let (re, _) = self
                .reader
                .read_tensor("intermediate", &format!("{}/stft_re", self.prefix), &region)
                .map_err(source_err)?;
            let (im, _) = self
                .reader
                .read_tensor("intermediate", &format!("{}/stft_im", self.prefix), &region)
                .map_err(source_err)?;
            SpectrumData::Complex { re, im }
        };
        Ok(SpectrumTile {
            frame_start: f_start,
            frames: n_frames,
            bin_start: b_start,
            bins: n_bins,
            data,
        })
    }
}

/// dataset config から STFT の window/hop を取得する
/// (`analysis.cheap.stft_*` の解釈)。
fn stft_grid(reader: &DatasetReader) -> (u64, u64) {
    let cheap = &reader.manifest().config["analysis"]["cheap"];
    let window = cheap["stft_window"].as_u64().unwrap_or(2048);
    let hop = cheap["stft_hop"].as_u64().unwrap_or(1024);
    (window, hop)
}

/// `inputs/<id>.metadata.json` の `audio.sample_rate` を読む。
fn audio_sample_rate(reader: &DatasetReader, input_id: &str) -> Option<u32> {
    let meta = reader
        .read_json("inputs", &format!("{input_id}.metadata"))
        .ok()?;
    meta["audio"]["sample_rate"].as_u64().map(|v| v as u32)
}
