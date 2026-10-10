//! Backend 選択と共通 ingest 結果。成果物の登録は stage の責務。

use crate::avi::{StreamTiming, VideoInfo};
use crate::format::{MediaFormat, detect_format};
use crate::metadata::{IntegrityObservation, MediaError};
use crate::mp4::Mp4Facts;
use crate::video::{VideoFeatureArtifact, VideoFeatureConfig, VideoThumbs};
use crate::wav::DecodedAudio;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnavailableKind {
    Capability,
    Malformed,
    Io,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Unavailable {
    pub kind: UnavailableKind,
    pub reason: String,
}

impl Unavailable {
    pub fn capability(reason: impl Into<String>) -> Self {
        Self {
            kind: UnavailableKind::Capability,
            reason: reason.into(),
        }
    }

    pub(crate) fn from_error(error: &MediaError) -> Self {
        Self {
            kind: match error {
                MediaError::InputUnavailable { .. } => UnavailableKind::Capability,
                MediaError::Malformed { .. } => UnavailableKind::Malformed,
                MediaError::Io(_) => UnavailableKind::Io,
            },
            reason: error.to_string(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DecodeRequest {
    pub audio: bool,
    pub video: bool,
    pub timing: bool,
    /// None は既存の選択規則。任意 stream decode は提供しない。
    pub audio_stream: Option<u32>,
    pub video_stream: Option<u32>,
    pub video_config: VideoFeatureConfig,
}

impl Default for DecodeRequest {
    fn default() -> Self {
        Self {
            audio: true,
            video: true,
            timing: true,
            audio_stream: None,
            video_stream: None,
            video_config: VideoFeatureConfig::default(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StreamProbe {
    pub stream: u32,
    pub kind: String,
    pub codec: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub bits_per_sample: Option<u16>,
    /// Channel 順は保持するが layout は推測しない。
    pub channel_layout: &'static str,
    pub timebase_hz: u64,
    pub packet_lengths: Vec<u64>,
    pub decode: Result<(), Unavailable>,
}

#[derive(Debug, Clone)]
pub struct MediaProbe {
    pub format: MediaFormat,
    pub streams: Vec<StreamProbe>,
    pub timing: Vec<StreamTiming>,
    pub mp4: Option<Mp4Facts>,
    pub source_hash: String,
    pub backend: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StreamResult {
    pub stream: Option<u32>,
    pub kind: String,
    pub decoded: bool,
    pub unavailable: Option<Unavailable>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BackendAttempt {
    pub backend: String,
    pub version: String,
    pub executed: bool,
    pub outcome: String,
    pub reason: Option<Unavailable>,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct DecoderProvenance {
    pub backend: String,
    pub version: String,
    pub source_hash: String,
    pub request: Option<DecodeRequest>,
    pub streams: Vec<StreamResult>,
    pub attempts: Vec<BackendAttempt>,
    pub fallback: bool,
    pub settings: serde_json::Value,
}

#[derive(Default)]
pub struct IngestResult {
    pub audio: Option<DecodedAudio>,
    pub streams: Vec<StreamProbe>,
    pub container: &'static str,
    pub video: Option<VideoInfo>,
    pub av_timing: Vec<StreamTiming>,
    pub video_features: Option<VideoFeatureArtifact>,
    pub video_thumbs: Option<VideoThumbs>,
    pub audio_unavailable: Option<String>,
    pub mp4: Option<Mp4Facts>,
    pub integrity: Vec<IntegrityObservation>,
    pub fallback_codec: Option<String>,
    pub provenance: DecoderProvenance,
}

pub trait MediaDecoder: Send + Sync {
    fn id(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn detect(&self, source: &[u8]) -> Option<MediaFormat>;
    /// PCM/frame を生成せず同じ構造検証を再利用する。
    fn probe(&self, source: &[u8]) -> Result<MediaProbe, MediaError>;
    fn decode(
        &self,
        source: &[u8],
        probe: &MediaProbe,
        request: &DecodeRequest,
    ) -> Result<IngestResult, MediaError>;
}

/// 登録順は固定。native は常に先頭、同じ ID は一度だけ登録する。
pub struct DecoderRegistry {
    backends: Vec<Box<dyn MediaDecoder>>,
}

impl DecoderRegistry {
    pub fn native() -> Self {
        Self {
            backends: vec![Box::new(NativeDecoder)],
        }
    }

    pub fn register(&mut self, backend: Box<dyn MediaDecoder>) -> Result<(), MediaError> {
        if self.backends.iter().any(|b| b.id() == backend.id()) {
            return Err(MediaError::input_unavailable(
                "duplicate decoder backend ID",
            ));
        }
        self.backends.push(backend);
        Ok(())
    }

    pub fn resolve(&self, source: &[u8]) -> Option<&dyn MediaDecoder> {
        self.backends
            .iter()
            .find(|b| b.detect(source).is_some())
            .map(|b| b.as_ref())
    }

    pub fn decode(
        &self,
        source: &[u8],
        request: &DecodeRequest,
    ) -> Result<IngestResult, MediaError> {
        // フェーズ1は native のみ。部分観測の合成・外部 fallback 許可は後続決定。
        let backend = self
            .resolve(source)
            .ok_or_else(|| MediaError::input_unavailable("unknown format"))?;
        let probe = backend.probe(source)?;
        let mut result = backend.decode(source, &probe, request)?;
        result.provenance.backend = backend.id().into();
        result.provenance.version = backend.version().into();
        result.provenance.source_hash = probe.source_hash;
        result.provenance.request = Some(request.clone());
        result.provenance.attempts.push(BackendAttempt {
            backend: backend.id().into(),
            version: backend.version().into(),
            executed: true,
            outcome: if result.audio.is_some()
                || matches!(
                    result.video_features,
                    Some(VideoFeatureArtifact::Decoded(_))
                ) {
                "decoded"
            } else {
                "timing_only"
            }
            .into(),
            reason: result
                .provenance
                .streams
                .iter()
                .find_map(|s| s.unavailable.clone()),
        });
        Ok(result)
    }
}

pub struct NativeDecoder;

impl MediaDecoder for NativeDecoder {
    fn id(&self) -> &'static str {
        "native"
    }
    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
    fn detect(&self, source: &[u8]) -> Option<MediaFormat> {
        detect_format(source)
    }

    fn probe(&self, source: &[u8]) -> Result<MediaProbe, MediaError> {
        let format = self
            .detect(source)
            .ok_or_else(|| MediaError::input_unavailable("unknown format"))?;
        let mut probe = MediaProbe {
            format,
            streams: vec![],
            timing: vec![],
            mp4: None,
            source_hash: al_core::sha256_hex(source),
            backend: self.id().into(),
        };
        match format {
            MediaFormat::Wav => {
                let layout = crate::wav::wav_layout(&mut std::io::Cursor::new(source))?;
                probe.streams.push(audio_probe(
                    0,
                    &layout.fmt,
                    u64::from(layout.fmt.sample_rate),
                ));
            }
            MediaFormat::Avi => {
                let layout = crate::avi::avi_layout(source)?;
                let mut first_audio = true;
                let mut first_video = true;
                for (index, head) in layout.streams.iter().enumerate() {
                    let stream = index as u32;
                    if &head.fcc_type == b"auds" {
                        let mut facts = match crate::wav::parse_fmt(&head.format) {
                            Ok(fmt) => audio_probe(stream, &fmt, u64::from(head.rate)),
                            Err(e) => unknown_stream(
                                stream,
                                "audio",
                                u64::from(head.rate),
                                Unavailable::from_error(&e),
                            ),
                        };
                        if !first_audio {
                            facts.decode = Err(Unavailable::capability(
                                "only first auds stream is selected",
                            ));
                        }
                        first_audio = false;
                        facts.packet_lengths = layout
                            .packets
                            .iter()
                            .filter(|p| p.stream_no == index)
                            .map(|p| p.body.len() as u64)
                            .collect();
                        probe.streams.push(facts);
                    } else if &head.fcc_type == b"vids" {
                        let decode = crate::dib::dib_format(&head.format)
                            .map(|_| ())
                            .map_err(|e| Unavailable::from_error(&e));
                        let mut facts = unknown_stream(
                            stream,
                            "video",
                            u64::from(head.rate),
                            Unavailable::capability("only first vids stream is selected"),
                        );
                        facts.codec = head.format.get(16..20).map(|b| {
                            crate::video::codec_fourcc(u32::from_le_bytes(b.try_into().unwrap()))
                        });
                        if first_video {
                            facts.decode = decode;
                        }
                        first_video = false;
                        facts.packet_lengths = layout
                            .packets
                            .iter()
                            .filter(|p| p.stream_no == index)
                            .map(|p| p.body.len() as u64)
                            .collect();
                        probe.streams.push(facts);
                    }
                }
                probe.timing = layout.timing;
            }
            MediaFormat::Mp4 => {
                let media = crate::mp4::probe_mp4(source)?;
                for track in &media.facts.tracks {
                    let mut facts = unknown_stream(
                        track.track_index,
                        match track.handler.as_str() {
                            "soun" => "audio",
                            "vide" => "video",
                            _ => "other",
                        },
                        u64::from(track.timescale),
                        Unavailable::capability(
                            "codec decode is unavailable; existing PCM extraction only",
                        ),
                    );
                    facts.codec = track.codec.clone();
                    if let Some(a) = &track.audio {
                        facts.sample_rate = Some(a.sample_rate);
                        facts.channels = Some(a.channels);
                        facts.bits_per_sample = Some(a.bits_per_sample);
                    }
                    facts.packet_lengths = media
                        .packet_lengths
                        .get(track.track_index as usize)
                        .into_iter()
                        .flatten()
                        .map(|n| u64::from(*n))
                        .collect();
                    if media.pcm_tracks.contains(&track.track_index) {
                        facts.decode = Ok(());
                    }
                    probe.streams.push(facts);
                }
                probe.timing = media.timing;
                probe.mp4 = Some(media.facts);
            }
        }
        Ok(probe)
    }

    fn decode(
        &self,
        source: &[u8],
        probe: &MediaProbe,
        request: &DecodeRequest,
    ) -> Result<IngestResult, MediaError> {
        if probe.backend != self.id()
            || probe.source_hash != al_core::sha256_hex(source)
            || self.detect(source) != Some(probe.format)
        {
            return Err(MediaError::malformed(
                "probe does not refer to the pinned backend/input",
            ));
        }
        let mut selected_mp4_audio = None;
        let mut result = match probe.format {
            MediaFormat::Wav => IngestResult {
                container: "wav",
                audio: if request.audio {
                    Some(crate::wav::decode_wav(source)?)
                } else {
                    None
                },
                ..Default::default()
            },
            MediaFormat::Avi => {
                let media = crate::avi::decode_layout(
                    crate::avi::avi_layout(source)?,
                    &request.video_config,
                    request.audio,
                    request.video,
                )?;
                let mut result = IngestResult {
                    container: "avi",
                    audio: media.audio,
                    audio_unavailable: media.audio_unavailable,
                    video: media.video,
                    av_timing: media.timing,
                    video_features: media.video_features,
                    video_thumbs: media.video_thumbs,
                    ..Default::default()
                };
                if let Some(failure) = media.audio_failure {
                    result.provenance.streams.push(StreamResult {
                        stream: probe
                            .streams
                            .iter()
                            .find(|s| s.kind == "audio")
                            .map(|s| s.stream),
                        kind: "audio".into(),
                        decoded: false,
                        unavailable: Some(failure),
                    });
                }
                if let Some(failure) = media.video_failure {
                    result.provenance.streams.push(StreamResult {
                        stream: probe
                            .streams
                            .iter()
                            .find(|s| s.kind == "video")
                            .map(|s| s.stream),
                        kind: "video".into(),
                        decoded: false,
                        unavailable: Some(failure),
                    });
                }
                result
            }
            MediaFormat::Mp4 => {
                let media = if request.audio {
                    crate::mp4::decode_mp4(source)?
                } else {
                    crate::mp4::probe_mp4(source)?
                };
                selected_mp4_audio = media.audio_stream;
                let fallback_codec = media
                    .facts
                    .tracks
                    .iter()
                    .find(|t| t.handler == "soun")
                    .and_then(|t| t.codec.clone())
                    .or_else(|| media.facts.tracks.iter().find_map(|t| t.codec.clone()));
                IngestResult {
                    container: "mp4",
                    audio: media.audio,
                    av_timing: media.timing,
                    mp4: Some(media.facts),
                    integrity: media.integrity,
                    fallback_codec,
                    ..Default::default()
                }
            }
        };
        if !request.audio {
            result.audio = None;
            result.audio_unavailable = None;
        }
        if !request.video {
            result.video_features = None;
            result.video_thumbs = None;
        }
        if !request.timing {
            result.av_timing.clear();
        }
        for (kind, requested, selection, decoded) in [
            (
                "audio",
                request.audio,
                request.audio_stream,
                result.audio.is_some(),
            ),
            (
                "video",
                request.video,
                request.video_stream,
                matches!(
                    result.video_features,
                    Some(VideoFeatureArtifact::Decoded(_))
                ),
            ),
        ] {
            if !requested {
                continue;
            }
            let stream = if kind == "audio" && probe.format == MediaFormat::Mp4 {
                selected_mp4_audio
                    .and_then(|id| probe.streams.iter().find(|s| s.stream == id))
                    .or_else(|| probe.streams.iter().find(|s| s.kind == kind))
            } else {
                probe.streams.iter().find(|s| s.kind == kind)
            };
            if selection.is_some() && selection != stream.map(|s| s.stream) {
                return Err(MediaError::input_unavailable(
                    "arbitrary stream selection is unavailable in native phase 1",
                ));
            }
            if !result.provenance.streams.iter().any(|s| s.kind == kind) {
                let unavailable = (!decoded).then(|| {
                    stream
                        .and_then(|s| s.decode.clone().err())
                        .unwrap_or_else(|| {
                            Unavailable::capability(format!("{kind} decode unavailable"))
                        })
                });
                result.provenance.streams.push(StreamResult {
                    stream: stream.map(|s| s.stream),
                    kind: kind.into(),
                    decoded,
                    unavailable,
                });
            }
        }
        for stream in &probe.streams {
            let requested = (stream.kind == "audio" && request.audio)
                || (stream.kind == "video" && request.video);
            if requested
                && !result
                    .provenance
                    .streams
                    .iter()
                    .any(|s| s.stream == Some(stream.stream) && s.kind == stream.kind)
            {
                result.provenance.streams.push(StreamResult {
                    stream: Some(stream.stream),
                    kind: stream.kind.clone(),
                    decoded: false,
                    unavailable: Some(stream.decode.clone().err().unwrap_or_else(|| {
                        Unavailable::capability("stream not selected by native phase 1 policy")
                    })),
                });
            }
        }
        if let Some(audio) = &result.audio
            && (audio.channels == 0
                || audio.sample_rate == 0
                || !audio
                    .samples
                    .len()
                    .is_multiple_of(usize::from(audio.channels)))
        {
            return Err(MediaError::malformed(
                "decoded PCM violates rate/channel/frame layout",
            ));
        }
        result.streams = probe.streams.clone();
        result.provenance.settings = serde_json::json!({ "selection": "first_auds_first_vids_or_first_pcm_mp4", "transforms": [], "pcm": "unscaled_interleaved_f64le", "channel_layout": "unknown" });
        Ok(result)
    }
}

fn unknown_stream(
    stream: u32,
    kind: &str,
    timebase_hz: u64,
    unavailable: Unavailable,
) -> StreamProbe {
    StreamProbe {
        stream,
        kind: kind.into(),
        codec: None,
        sample_rate: None,
        channels: None,
        bits_per_sample: None,
        channel_layout: "unknown",
        timebase_hz,
        packet_lengths: vec![],
        decode: Err(unavailable),
    }
}

fn audio_probe(stream: u32, fmt: &crate::wav::FmtChunk, timebase_hz: u64) -> StreamProbe {
    let layout = crate::wav::sample_layout(fmt);
    StreamProbe {
        stream,
        kind: "audio".into(),
        codec: layout.as_ref().ok().map(|(_, codec, _)| codec.clone()),
        sample_rate: Some(fmt.sample_rate),
        channels: Some(fmt.channels),
        bits_per_sample: Some(fmt.bits_per_sample),
        channel_layout: "unknown",
        timebase_hz,
        packet_lengths: vec![],
        decode: layout.map(|_| ()).map_err(|e| Unavailable::from_error(&e)),
    }
}
