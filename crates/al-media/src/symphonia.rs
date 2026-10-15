//! Audio-only decoder backend backed by symphonia.
//!
//! Detects MP4 input and decodes the first decode-capable AAC track.
//! Container facts, timing, and stream probing remain owned by the
//! native backend; this backend only supplies `DecodedAudio` for the
//! registry's audio fallback path.

use std::io::Cursor;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_AAC, DecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::decoder::{
    DecodeRequest, IngestResult, MediaDecoder, MediaProbe, StreamProbe, StreamResult, Unavailable,
};
use crate::format::{MediaFormat, detect_format};
use crate::metadata::{MediaError, SampleFormat};
use crate::wav::DecodedAudio;

pub struct SymphoniaDecoder;

/// 対象 track: demux 結果と選択した track 情報。
struct SelectedTrack {
    id: u32,
    codec_params: symphonia::core::codecs::CodecParameters,
}

fn open_format(source: &[u8]) -> Result<Box<dyn FormatReader>, SymphoniaError> {
    let cursor = Cursor::new(source.to_vec());
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());
    let mut hint = Hint::new();
    hint.with_extension("mp4");
    let fmt_opts = FormatOptions {
        enable_gapless: true,
        ..Default::default()
    };
    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &fmt_opts,
        &MetadataOptions::default(),
    )?;
    Ok(probed.format)
}

fn select_aac_track(format: &dyn FormatReader) -> Option<SelectedTrack> {
    for track in format.tracks() {
        if track.codec_params.codec != CODEC_TYPE_AAC {
            continue;
        }
        // AAC-LC 以外(SBR/PS 等)は decoder 生成で弾かれる。
        if symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .is_ok()
        {
            return Some(SelectedTrack {
                id: track.id,
                codec_params: track.codec_params.clone(),
            });
        }
    }
    None
}

fn probe_error(err: &SymphoniaError) -> MediaError {
    match err {
        SymphoniaError::IoError(e) => MediaError::Io(std::io::Error::new(e.kind(), e.to_string())),
        SymphoniaError::Unsupported(_) | SymphoniaError::LimitError(_) => {
            MediaError::input_unavailable(format!("symphonia: {err}"))
        }
        _ => MediaError::malformed(format!("symphonia: {err}")),
    }
}

impl MediaDecoder for SymphoniaDecoder {
    fn id(&self) -> &'static str {
        "symphonia"
    }

    // Cargo.toml の symphonia pin と対応付ける。
    fn version(&self) -> &'static str {
        "0.5.5"
    }

    fn detect(&self, source: &[u8]) -> Option<MediaFormat> {
        match detect_format(source) {
            Some(MediaFormat::Mp4) => Some(MediaFormat::Mp4),
            _ => None,
        }
    }

    /// 最小構成の probe — registry の fallback 経路で decode の内部整合
    /// 確認に使うだけで、結果の `streams` は採用されない。
    fn probe(&self, source: &[u8]) -> Result<MediaProbe, MediaError> {
        let format = open_format(source).map_err(|e| probe_error(&e))?;
        let streams = format
            .tracks()
            .iter()
            .map(|track| {
                let p = &track.codec_params;
                let decodable = p.codec == CODEC_TYPE_AAC
                    && symphonia::default::get_codecs()
                        .make(p, &DecoderOptions::default())
                        .is_ok();
                StreamProbe {
                    stream: track.id,
                    kind: if p.codec != symphonia::core::codecs::CODEC_TYPE_NULL {
                        "audio"
                    } else {
                        "other"
                    }
                    .into(),
                    codec: (p.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
                        .then(|| "aac".into()),
                    sample_rate: p.sample_rate,
                    channels: p.channels.map(|c| c.count() as u16),
                    bits_per_sample: p.bits_per_sample.map(|b| b as u16),
                    channel_layout: "unknown",
                    timebase_hz: p
                        .time_base
                        .map(|tb| u64::from(tb.denom) / u64::from(tb.numer).max(1))
                        .unwrap_or(1),
                    packet_lengths: vec![],
                    decode: if decodable {
                        Ok(())
                    } else {
                        Err(Unavailable::capability(
                            "codec decode is unavailable for symphonia backend",
                        ))
                    },
                }
            })
            .collect();
        Ok(MediaProbe {
            format: MediaFormat::Mp4,
            streams,
            timing: vec![],
            mp4: None,
            source_hash: al_core::sha256_hex(source),
            backend: self.id().into(),
        })
    }

    fn decode(
        &self,
        source: &[u8],
        probe: &MediaProbe,
        request: &DecodeRequest,
    ) -> Result<IngestResult, MediaError> {
        if probe.backend != self.id() || probe.source_hash != al_core::sha256_hex(source) {
            return Err(MediaError::malformed(
                "probe does not refer to the pinned backend/input",
            ));
        }
        let mut result = IngestResult {
            container: "mp4",
            ..Default::default()
        };
        if !request.audio {
            return Ok(result);
        }
        let mut format = open_format(source).map_err(|e| probe_error(&e))?;
        let Some(selected) = select_aac_track(format.as_ref()) else {
            result.provenance.streams.push(StreamResult {
                stream: None,
                kind: "audio".into(),
                decoded: false,
                unavailable: Some(Unavailable::capability("no decode-capable AAC track")),
            });
            return Ok(result);
        };
        let mut decoder = symphonia::default::get_codecs()
            .make(&selected.codec_params, &DecoderOptions::default())
            .map_err(|e| probe_error(&e))?;

        let mut samples: Vec<f64> = Vec::new();
        let mut channels: u16 = 0;
        let mut sample_rate: u32 = 0;
        let mut first_pts: Option<u64> = None;
        let mut trimmed_front: u64 = 0;
        let mut trimmed_back: u64 = 0;
        let mut skipped_packets: u64 = 0;

        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(SymphoniaError::IoError(_)) => break,
                Err(SymphoniaError::ResetRequired) => break,
                Err(e) => return Err(probe_error(&e)),
            };
            if packet.track_id() != selected.id {
                continue;
            }
            if first_pts.is_none() {
                // gapless 有効時の ts は encoder delay 補正済み。
                first_pts = Some(packet.ts());
            }
            // gapless 有効時は demuxer が trim_start/trim_end を設定し、
            // decoder が priming/padding を除去する。
            trimmed_front += u64::from(packet.trim_start());
            trimmed_back += u64::from(packet.trim_end());
            match decoder.decode(&packet) {
                Ok(decoded) => {
                    let spec = *decoded.spec();
                    let capacity = decoded.capacity() as u64;
                    let mut buf = SampleBuffer::<f64>::new(capacity, spec);
                    buf.copy_interleaved_ref(decoded);
                    samples.extend_from_slice(buf.samples());
                    if channels == 0 {
                        channels = spec.channels.count() as u16;
                        sample_rate = spec.rate;
                    }
                }
                Err(SymphoniaError::DecodeError(_)) => {
                    skipped_packets += 1;
                }
                Err(SymphoniaError::IoError(_)) | Err(SymphoniaError::ResetRequired) => break,
                Err(e) => return Err(probe_error(&e)),
            }
        }
        if samples.is_empty() || channels == 0 || sample_rate == 0 {
            result.provenance.streams.push(StreamResult {
                stream: Some(selected.id),
                kind: "audio".into(),
                decoded: false,
                unavailable: Some(Unavailable::capability(format!(
                    "AAC track decoded to no frames (skipped packets: {skipped_packets})"
                ))),
            });
            return Ok(result);
        }

        result.audio = Some(DecodedAudio {
            samples,
            channels,
            sample_rate,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float32,
            codec: "aac".into(),
        });
        result.provenance.streams.push(StreamResult {
            stream: Some(selected.id),
            kind: "audio".into(),
            decoded: true,
            unavailable: None,
        });
        result.provenance.settings = serde_json::json!({
            "symphonia": {
                "track_id": selected.id,
                "first_pts": first_pts,
                "trimmed_front": trimmed_front,
                "trimmed_back": trimmed_back,
                "skipped_packets": skipped_packets,
            }
        });
        Ok(result)
    }
}
