//! Ruviform media-I/O バックエンドへのアダプタ(Issue #100)。
//!
//! container の demux/decode は Ruviform backend
//! (WAV/ISOBMFF は `ruviform-io-symphonia`、AVI は `ruviform-io-avi`)
//! に委譲し、AudioLens 側は既存の観測契約 — `IngestResult` /
//! `StreamProbe` / `StreamTiming` / `Mp4Facts` / integrity
//! observation — への接続を担う。`NativeAudioFrame` の非破壊
//! (native scale f64)経路のみを使い、f32 正規化は導入しない。
//!
//! 暫定境界: container の事実観測パーサ (wav/avi/mp4.rs) は
//! AudioLens の永続契約が要求する詳細(skipped boxes・edit list・
//! sync samples・chunk facts)を Ruviform の現行観測面
//! (StreamFacts/ContainerInfo)がまだ提供しないため残置し、
//! decoder のみを Ruviform に移す。汎用パーサの最終撤去は
//! Issue #104 で追跡する。

use std::io::Cursor;

use ruviform_media_core::{Fact, StreamId, StreamKind};
use ruviform_media_io::{Decoder, Demuxer, Error, ErrorKind, Frame, Packet};

use crate::decoder::{
    DecodeRequest, IngestResult, MediaDecoder, MediaProbe, NativeDecoder, Unavailable,
};
use crate::format::{MediaFormat, detect_format};
use crate::metadata::{IntegrityObservation, MediaError, SampleFormat};
use crate::wav::DecodedAudio;

/// The pinned Ruviform revision — recorded in provenance so manifests
/// keep the exact backend revision in use.
pub const RUVIFORM_REV: &str = "e467ccba8e1cf9d18982387827b61275b4bf3f4c";

/// Normalize a Ruviform boundary error into the ingest error contract:
/// `InputUnavailable` for capability gaps, `Malformed` for corrupt
/// input, `Io` for source failures.
fn map_error(error: Error) -> MediaError {
    match error.kind() {
        ErrorKind::Unsupported => MediaError::input_unavailable(error.to_string()),
        ErrorKind::Io => MediaError::Io(std::io::Error::other(error.to_string())),
        ErrorKind::InvalidData
        | ErrorKind::InvalidValue
        | ErrorKind::TimeOverflow
        | ErrorKind::UnexpectedEos => MediaError::malformed(error.to_string()),
        _ => MediaError::malformed(error.to_string()),
    }
}

/// Ruviform demuxers kept concrete: `make_native_decoder` /
/// `make_decoder` are backend entry points, not `Demuxer` trait items.
enum RuvDemux {
    Symphonia(ruviform_io_symphonia::SymphoniaDemuxer),
    Avi(ruviform_io_avi::AviDemuxer),
}

impl RuvDemux {
    fn as_demuxer(&mut self) -> &mut dyn Demuxer {
        match self {
            Self::Symphonia(d) => d,
            Self::Avi(d) => d,
        }
    }
}

fn open_demux(format: MediaFormat, source: &[u8]) -> Result<RuvDemux, MediaError> {
    let cursor = Cursor::new(source.to_vec());
    match format {
        MediaFormat::Wav | MediaFormat::Mp4 => ruviform_io_symphonia::open_demuxer(cursor)
            .map(RuvDemux::Symphonia)
            .map_err(map_error),
        MediaFormat::Avi => ruviform_io_avi::open_demuxer(cursor)
            .map(RuvDemux::Avi)
            .map_err(map_error),
    }
}

fn make_native_decoder(demux: &RuvDemux, stream: StreamId) -> Result<Box<dyn Decoder>, MediaError> {
    match demux {
        RuvDemux::Symphonia(d) => d
            .make_native_decoder(stream)
            .map(|d| Box::new(d) as Box<dyn Decoder>)
            .map_err(map_error),
        RuvDemux::Avi(d) => d
            .make_native_decoder(stream)
            .map(|d| Box::new(d) as Box<dyn Decoder>)
            .map_err(map_error),
    }
}

/// Canonical (f32) decoder — the route for codecs like AAC whose
/// decoded form is not the source's lossless PCM encoding.
fn make_decoder(demux: &RuvDemux, stream: StreamId) -> Result<Box<dyn Decoder>, MediaError> {
    match demux {
        RuvDemux::Symphonia(d) => d
            .make_decoder(stream)
            .map(|d| Box::new(d) as Box<dyn Decoder>)
            .map_err(map_error),
        RuvDemux::Avi(d) => d
            .make_decoder(stream)
            .map(|d| Box::new(d) as Box<dyn Decoder>)
            .map_err(map_error),
    }
}

/// Map the decoder's native sample format onto the ingest contract.
/// `f64` storage keeps every ≤32-bit integer encoding exact.
fn sample_format(format: ruviform_audio_core::SampleFormat) -> SampleFormat {
    use ruviform_audio_core::SampleFormat as F;
    match format {
        F::U8 => SampleFormat::PcmU8,
        F::S8 => SampleFormat::PcmS8,
        F::S16 | F::U16 => SampleFormat::PcmS16,
        F::S24 | F::U24 => SampleFormat::PcmS24,
        F::S32 | F::U32 => SampleFormat::PcmS32,
        F::F32 => SampleFormat::Float32,
        F::F64 => SampleFormat::Float64,
        _ => SampleFormat::Other,
    }
}

/// Codec name for provenance. `pcm_*` names keep the container's
/// endianness where it is recoverable; compressed codecs use the
/// canonical Ruviform codec name (`aac`, ...).
fn codec_name(
    container: MediaFormat,
    info: &ruviform_media_io::StreamInfo,
    format: SampleFormat,
    bits: u16,
) -> String {
    use ruviform_media_io::CodecId;
    match info.codec.codec {
        CodecId::Pcm => {
            let endian = match container {
                MediaFormat::Wav => "le",
                _ => match info.facts.codec_tag {
                    Some(ruviform_media_io::CodecTag::FourCc(tag)) => match &tag {
                        b"sowt" => "le",
                        _ => "be",
                    },
                    _ => "le",
                },
            };
            match (format, bits) {
                (SampleFormat::PcmU8, _) => "pcm_u8".into(),
                (SampleFormat::PcmS8, _) => "pcm_s8".into(),
                (SampleFormat::PcmS16, _) => format!("pcm_s16{endian}"),
                (SampleFormat::PcmS24, _) => format!("pcm_s24{endian}"),
                (SampleFormat::PcmS32, _) => format!("pcm_s32{endian}"),
                (SampleFormat::Float32, _) => format!("pcm_f32{endian}"),
                (SampleFormat::Float64, _) => format!("pcm_f64{endian}"),
                _ => "pcm".into(),
            }
        }
        codec => codec.name().into(),
    }
}

/// Whether a Ruviform stream is audio-typed.
fn is_audio(info: &ruviform_media_io::StreamInfo) -> bool {
    info.stream.kind == StreamKind::Audio
}

/// Run the demux → native-decode pipeline for one stream, collecting
/// `NativeAudioFrame`s into a single interleaved f64 sample vector and
/// the first frame's PTS ticks (with provenance).
fn decode_audio_native(
    demux: &mut RuvDemux,
    stream: StreamId,
    packets: Vec<Packet>,
) -> Result<(Vec<f64>, u64), MediaError> {
    let mut decoder = make_native_decoder(demux, stream)?;
    let mut samples = Vec::new();
    let mut first_pts: Option<u64> = None;
    let collect = |frame: Frame,
                   samples: &mut Vec<f64>,
                   first_pts: &mut Option<u64>|
     -> Result<(), MediaError> {
        match frame {
            Frame::NativeAudio(frame) => {
                if first_pts.is_none() {
                    *first_pts = Some(frame.pts().ticks.max(0) as u64);
                }
                samples.extend_from_slice(frame.samples());
                Ok(())
            }
            _ => Err(MediaError::malformed(
                "native decoder emitted a non-native frame",
            )),
        }
    };
    for packet in packets {
        decoder.send_packet(packet).map_err(map_error)?;
        while let Some(frame) = decoder.receive_frame().map_err(map_error)? {
            collect(frame, &mut samples, &mut first_pts)?;
        }
    }
    decoder.drain().map_err(map_error)?;
    while let Some(frame) = decoder.receive_frame().map_err(map_error)? {
        collect(frame, &mut samples, &mut first_pts)?;
    }
    Ok((samples, first_pts.unwrap_or(0)))
}

/// Decode the first decodable audio stream of `source` through
/// Ruviform. Returns the `DecodedAudio` plus the stream's
/// `StreamFacts.index` (container-order index) for observation joins.
fn ruviform_audio(format: MediaFormat, source: &[u8]) -> Result<(DecodedAudio, u32), MediaError> {
    let mut demux = open_demux(format, source)?;
    let (stream_id, index, info) = {
        let d = demux.as_demuxer();
        let info = d
            .streams()
            .iter()
            .find(|s| is_audio(s) && s.facts.decodable.is_known())
            .cloned()
            .ok_or_else(|| {
                MediaError::input_unavailable("Ruviform backend reports no decodable audio stream")
            })?;
        (info.stream.id, info.facts.index, info)
    };
    // The packet feed filters to the audio stream — the demux walks
    // the container once while other streams' packets are dropped.
    let mut packets = Vec::new();
    loop {
        match demux.as_demuxer().read_packet() {
            Ok(Some(p)) => {
                if p.stream == stream_id {
                    packets.push(p);
                }
            }
            Ok(None) => break,
            Err(e) => return Err(map_error(e)),
        }
    }
    // The non-destructive native path is used when the stream declares
    // a native audio encoding — PCM keeps its exact integer/f64 scale.
    // Codecs without a lossless native form (AAC: `native_format` is
    // `NotDeclared`) decode through the canonical f32 frame path and
    // are labelled `Float32`, matching the ingest contract the old
    // symphonia fallback established.
    let native_fmt = match info.codec.native_format.get() {
        Some(ruviform_media_io::CodecFormat::Audio(f)) => Some(*f),
        _ => None,
    };
    let canonical = |demux: &mut RuvDemux,
                     stream: StreamId,
                     packets: Vec<Packet>|
     -> Result<Vec<f64>, MediaError> {
        let mut decoder = make_decoder(demux, stream)?;
        let mut samples = Vec::new();
        let collect = |frame: Frame, samples: &mut Vec<f64>| {
            if let Frame::Audio(frame) = frame {
                let buffer = frame.buffer();
                for i in 0..buffer.frames() {
                    for c in 0..buffer.channels() {
                        samples.push(f64::from(buffer.channel(c).map_or(0.0, |ch| ch[i])));
                    }
                }
            }
        };
        for packet in packets {
            decoder.send_packet(packet).map_err(map_error)?;
            while let Some(frame) = decoder.receive_frame().map_err(map_error)? {
                collect(frame, &mut samples);
            }
        }
        decoder.drain().map_err(map_error)?;
        while let Some(frame) = decoder.receive_frame().map_err(map_error)? {
            collect(frame, &mut samples);
        }
        Ok(samples)
    };
    let (samples, fmt, bits) = match native_fmt {
        Some(f) => match decode_audio_native(&mut demux, stream_id, packets.clone()) {
            Ok((samples, _first_pts)) if !samples.is_empty() => (
                samples,
                sample_format(f.sample_format),
                f.sample_format.bits_per_sample(),
            ),
            _ => (
                canonical(&mut demux, stream_id, packets)?,
                SampleFormat::Float32,
                32,
            ),
        },
        None => (
            canonical(&mut demux, stream_id, packets)?,
            SampleFormat::Float32,
            32,
        ),
    };
    if samples.is_empty() {
        return Err(MediaError::input_unavailable(
            "Ruviform audio stream decoded to no samples",
        ));
    }
    let (sample_rate, channels) = match info.codec.format {
        ruviform_media_io::CodecFormat::Audio(f) => {
            (f.sample_rate.get(), f.layout.channels() as u16)
        }
        _ => (0, 0),
    };
    if channels == 0 || sample_rate == 0 {
        return Err(MediaError::input_unavailable(
            "Ruviform audio format lacks rate/channels",
        ));
    }
    Ok((
        DecodedAudio {
            samples,
            channels,
            sample_rate,
            bits_per_sample: bits,
            sample_format: fmt,
            codec: codec_name(format, &info, fmt, bits),
        },
        index,
    ))
}

/// Ruviform backend: decode crosses to `ruviform-io-*` backends while
/// container-fact observation stays on the AudioLens parsers.
pub struct RuviformDecoder;

impl MediaDecoder for RuviformDecoder {
    fn id(&self) -> &'static str {
        "ruviform"
    }
    fn version(&self) -> &'static str {
        RUVIFORM_REV
    }
    fn detect(&self, source: &[u8]) -> Option<MediaFormat> {
        detect_format(source)
    }

    /// Probe facts reuse the AudioLens container parsers (the persisted
    /// observation contract). Ruviform stream capabilities then update
    /// each stream's `decode` flag — e.g. an MP4 AAC track the native
    /// parser could only observe now reports decodable.
    fn probe(&self, source: &[u8]) -> Result<MediaProbe, MediaError> {
        let span = tracing::info_span!("media::probe", method = self.id(), len = source.len());
        let _enter = span.enter();
        let mut probe = NativeDecoder.probe(source)?;
        probe.backend = self.id().into();
        let Ok(mut demux) = open_demux(probe.format, source) else {
            // The AudioLens parser accepted the container but the
            // Ruviform backend cannot open it — a capability fact, not
            // a probe failure. Streams keep their native capability
            // flags; decode() will surface the backend error.
            return Ok(probe);
        };
        let streams: Vec<(u32, StreamKind, Fact<()>)> = {
            let d = demux.as_demuxer();
            d.streams()
                .iter()
                .map(|s| (s.facts.index, s.stream.kind, s.facts.decodable))
                .collect()
        };
        let mut audio_seen = false;
        let mut video_seen = false;
        for facts in &mut probe.streams {
            let ruv = streams.iter().find(|(i, _, _)| *i == facts.stream);
            match (facts.kind.as_str(), ruv) {
                ("audio", Some((_, StreamKind::Audio, decodable))) => {
                    if !audio_seen && decodable.is_known() {
                        facts.decode = Ok(());
                    }
                    audio_seen = true;
                }
                ("video", Some((_, StreamKind::Video, decodable))) => {
                    if !video_seen && decodable.is_known() {
                        facts.decode = Ok(());
                    }
                    video_seen = true;
                }
                _ => {}
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
        // Facts/timing stay on the AudioLens observation path; the
        // payload decode crosses to Ruviform. AVI video features are
        // still produced by the native DIB path (gate 3, tracked for
        // #104) — recorded in provenance.
        let mut fact_request = request.clone();
        fact_request.audio = false;
        let mut result = NativeDecoder.decode(source, &native_probe(probe), &fact_request)?;
        result.provenance.settings["ruviform"] = serde_json::json!({
            "rev": RUVIFORM_REV,
            "io": ["ruviform-io-symphonia", "ruviform-io-avi"],
        });
        if matches!(probe.format, MediaFormat::Avi) && request.video {
            result.provenance.settings["ruviform"]["video_path"] = "native_dib".into();
        }
        if !request.audio {
            return Ok(result);
        }
        let first_audio_stream = probe
            .streams
            .iter()
            .find(|s| s.kind == "audio")
            .map(|s| s.stream);
        let mut selected_audio = first_audio_stream;
        match ruviform_audio(probe.format, source) {
            Ok((audio, index)) => {
                let audio_stream = probe
                    .streams
                    .iter()
                    .find(|s| s.kind == "audio" && s.stream == index)
                    .map(|s| s.stream)
                    .or(first_audio_stream);
                selected_audio = audio_stream;
                // MP4 edit lists: `elst` media_time is the media start
                // offset (e.g. AAC encoder delay). Ruviform decoders
                // leave it unapplied, so the AudioLens observation
                // contract applies it here — the same media-time →
                // sample-rate conversion the old fallback used.
                let mut audio = audio;
                if let (MediaFormat::Mp4, Some(mp4)) = (probe.format, result.mp4.as_ref())
                    && let Some(track) = mp4.tracks.iter().find(|t| t.track_index == index)
                {
                    let timescale = u64::from(track.timescale).max(1);
                    let offset = track
                        .edit_media_time_ticks
                        .unwrap_or(0)
                        .min(track.duration_ticks);
                    let rate = u64::from(audio.sample_rate);
                    let front = offset * rate / timescale;
                    let playable = (track.duration_ticks - offset) * rate / timescale;
                    let ch = usize::from(audio.channels);
                    let start = (front as usize) * ch;
                    let len = (playable as usize) * ch;
                    if start < audio.samples.len() {
                        audio.samples.drain(..start);
                    } else {
                        audio.samples.clear();
                    }
                    audio.samples.truncate(len);
                }
                result.audio = Some(audio);
                result
                    .provenance
                    .streams
                    .push(crate::decoder::StreamResult {
                        stream: audio_stream,
                        kind: "audio".into(),
                        decoded: true,
                        unavailable: None,
                    });
                // Other audio tracks stay unselected — the same fact
                // the native MP4 parser recorded for secondary `soun`
                // tracks. `soun` entries precede the synthesized video
                // observations, matching trak-order emission.
                let mut secondary = Vec::new();
                for s in probe
                    .streams
                    .iter()
                    .filter(|s| s.kind == "audio" && s.stream != index)
                {
                    let failure = Unavailable::capability(
                        "secondary audio track; first decodable track already selected",
                    );
                    let detail = if let Some(mp4) = result.mp4.as_ref() {
                        let codec = mp4
                            .tracks
                            .iter()
                            .find(|t| t.track_index == s.stream)
                            .and_then(|t| t.codec.clone())
                            .unwrap_or_default();
                        serde_json::json!({
                            "track_index": s.stream,
                            "handler": "soun",
                            "codec": codec,
                        })
                    } else {
                        serde_json::json!({ "track_index": s.stream })
                    };
                    secondary.push(IntegrityObservation::decode_unavailable(
                        "audio", &failure, detail,
                    ));
                    result
                        .provenance
                        .streams
                        .push(crate::decoder::StreamResult {
                            stream: Some(s.stream),
                            kind: "audio".into(),
                            decoded: false,
                            unavailable: Some(failure),
                        });
                }
                result.integrity.splice(0..0, secondary);
            }
            Err(e) => {
                let failure = Unavailable::from_error(&e);
                // Temporary comparison route: fall back to the native
                // decoders when the Ruviform backend cannot read this
                // input, and record the fallback reason in provenance.
                let mut native_integrity = Vec::new();
                let mut native_stream = None;
                let native = match probe.format {
                    MediaFormat::Wav => crate::wav::decode_wav(source).ok(),
                    MediaFormat::Mp4 => match crate::mp4::decode_mp4(source) {
                        Ok(media) => {
                            native_integrity = media.integrity;
                            native_stream = media.audio_stream;
                            media.audio
                        }
                        Err(_) => None,
                    },
                    MediaFormat::Avi => crate::avi::avi_layout(source)
                        .and_then(|layout| {
                            crate::avi::decode_layout(layout, &request.video_config, true, false)
                        })
                        .ok()
                        .and_then(|media| media.audio),
                };
                match native {
                    Some(audio) => {
                        selected_audio = native_stream.or(first_audio_stream);
                        result.integrity.extend(native_integrity);
                        result.audio = Some(audio);
                        result.provenance.settings["ruviform"]["audio_fallback"] = serde_json::json!({
                            "backend": "native",
                            "reason": failure.reason,
                        });
                        result
                            .provenance
                            .streams
                            .push(crate::decoder::StreamResult {
                                stream: selected_audio,
                                kind: "audio".into(),
                                decoded: true,
                                unavailable: None,
                            });
                    }
                    None => {
                        // Per-track unavailability mirrors the MP4
                        // observation contract (one entry per `soun`
                        // track); other containers keep a single
                        // backend-scoped observation.
                        // `soun` entries precede the backend-generated
                        // video observations, matching the trak-order
                        // emission of the native MP4 parser.
                        let mut track_entries = Vec::new();
                        if let Some(mp4) = result.mp4.as_ref() {
                            for track in mp4.tracks.iter().filter(|t| t.handler == "soun") {
                                track_entries.push(IntegrityObservation::decode_unavailable(
                                    "audio",
                                    &failure,
                                    serde_json::json!({
                                        "track_index": track.track_index,
                                        "handler": "soun",
                                        "codec": track.codec.clone().unwrap_or_default(),
                                    }),
                                ));
                            }
                        }
                        let observed = !track_entries.is_empty();
                        result.integrity.splice(0..0, track_entries);
                        if !observed {
                            result
                                .integrity
                                .push(IntegrityObservation::decode_unavailable(
                                    "audio",
                                    &failure,
                                    serde_json::json!({"backend": "ruviform"}),
                                ));
                        }
                        result
                            .provenance
                            .streams
                            .push(crate::decoder::StreamResult {
                                stream: first_audio_stream,
                                kind: "audio".into(),
                                decoded: false,
                                unavailable: Some(failure.clone()),
                            });
                        // Remaining audio streams are unselected — the
                        // native unselected-stream contract carries each
                        // stream's own decode verdict.
                        for s in probe
                            .streams
                            .iter()
                            .filter(|s| s.kind == "audio" && Some(s.stream) != first_audio_stream)
                        {
                            result
                                .provenance
                                .streams
                                .push(crate::decoder::StreamResult {
                                    stream: Some(s.stream),
                                    kind: "audio".into(),
                                    decoded: false,
                                    unavailable: Some(s.decode.clone().err().unwrap_or_else(
                                        || {
                                            Unavailable::capability(
                                                "stream not selected by ingest selection policy",
                                            )
                                        },
                                    )),
                                });
                        }
                        result.audio_unavailable = Some(failure.reason);
                    }
                }
            }
        }
        // `provenance.streams` keeps the native emission contract:
        // requested-kind entries come first (audio before video), then
        // unselected streams in probe order. The fact decode only emits
        // video-side entries, so the audio entries appended above are
        // re-ordered here.
        let selected_video = probe
            .streams
            .iter()
            .find(|s| s.kind == "video")
            .map(|s| s.stream);
        order_stream_results(
            &mut result.provenance.streams,
            probe,
            selected_audio,
            selected_video,
        );
        Ok(result)
    }
}

/// Order `provenance.streams` the way `NativeDecoder::decode` emits
/// them: the selected audio entry, then the selected video entry, then
/// every unselected stream in probe order.
fn order_stream_results(
    streams: &mut [crate::decoder::StreamResult],
    probe: &MediaProbe,
    selected_audio: Option<u32>,
    selected_video: Option<u32>,
) {
    let rank = |stream: Option<u32>, kind: &str| {
        probe
            .streams
            .iter()
            .position(|s| Some(s.stream) == stream && s.kind == kind)
            .unwrap_or(usize::MAX)
    };
    streams.sort_by_key(|s| {
        if s.kind == "audio" && s.stream == selected_audio {
            (0, 0usize)
        } else if s.kind == "video" && s.stream == selected_video {
            (0, 1)
        } else {
            (1, rank(s.stream, s.kind.as_str()))
        }
    });
}

/// Re-wrap a Ruviform-labelled probe so `NativeDecoder::decode` accepts
/// it (its pin requires `probe.backend == "native"`).
fn native_probe(probe: &MediaProbe) -> MediaProbe {
    let mut p = probe.clone();
    p.backend = "native".into();
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::{DecodeRequest, MediaDecoder};

    fn wav16() -> Vec<u8> {
        let cursor = std::io::Cursor::new(Vec::new());
        let mut w =
            crate::wav::WavWriter::new(cursor, crate::wav::WavEncoding::PcmS16, 1, 48_000).unwrap();
        w.write_samples(&[0.0, 100.0, -200.0, 300.0]).unwrap();
        w.finish().unwrap().into_inner()
    }

    #[test]
    fn ruviform_decodes_wav_pcm() {
        let data = wav16();
        let backend = RuviformDecoder;
        let probe = backend.probe(&data).expect("probe");
        assert_eq!(probe.backend, "ruviform");
        let result = backend
            .decode(&data, &probe, &DecodeRequest::default())
            .expect("decode");
        let audio = result.audio.expect("audio decoded");
        assert_eq!(audio.samples, vec![0.0, 100.0, -200.0, 300.0]);
        assert_eq!(audio.sample_format, SampleFormat::PcmS16);
    }

    #[test]
    fn backend_id_is_ruviform() {
        assert_eq!(RuviformDecoder.id(), "ruviform");
    }
}
