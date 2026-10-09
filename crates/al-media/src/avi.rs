//! RIFF/AVI demuxing: PCM audio extraction, video stream metadata,
//! timing observation, and uncompressed-frame feature extraction.
//!
//! Scope: legacy AVI (`RIFF 'AVI '`). The audio stream must carry PCM
//! or IEEE-float data (its `strf` chunk is a WAVEFORMATEX, the same
//! layout as a WAV `fmt ` chunk); other audio codecs are recorded as
//! `decode/input unavailable` without failing the demux. The first
//! `vids` stream's declared facts (codec fourcc, dimensions, nominal
//! frame rate, frame count) are recorded, and uncompressed `db`/`dc`
//! frames are decoded to coarse features by [`crate::video`].
//!
//! Timing: legacy AVI carries no per-packet PTS. Each `movi` packet's
//! position on its stream clock is derived from the mux order — the
//! cumulative `strh` units consumed by earlier packets of the same
//! stream — and recorded as a CFR-free [`StreamTiming`] observation
//! (audio packet timestamps and video frame positions alike). The mux
//! order is the primary fact; nothing is reconstructed from a nominal
//! frame rate. The derived timestamp column itself is persisted
//! (`StreamTiming::start_tick` + `stream_timestamps` with origin
//! `movi_order`) so per-packet positions stay addressable from
//! `metadata.json` alone.
//!
//! Partial ingest: a missing or undecodable `auds` stream no longer
//! fails the demux — `AviMedia::audio` is `None` and
//! `audio_unavailable` records the `decode/input unavailable` reason
//! while timing and video features still surface. Structural
//! corruption (bad chunks, size mismatch, missing `movi`) stays
//! `Malformed`.

use crate::metadata::MediaError;
use crate::timing::{ClockKind, TimingObservation, observe_timing};
use crate::video::{
    self, VideoFeatureArtifact, VideoFeatureConfig, VideoFrame, VideoThumbs, codec_fourcc,
};
use crate::wav::{DecodedAudio, decode_waveformatex, read_u16, read_u32};

/// Video stream facts declared by the container (no frame decode).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VideoInfo {
    /// Codec fourcc as text (e.g. `DIB`, `MJPG`), `rawvideo` when the
    /// compression field is zero.
    pub codec: String,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Bits per pixel.
    pub bits_per_pixel: u16,
    /// Declared frame count (`strh` length; falls back to the counted
    /// frame chunks in `movi`).
    pub frames: u64,
    /// Nominal frame rate (`strh` rate/scale), when declared.
    pub fps: Option<f64>,
    /// Duration in seconds derived from frames/fps, when known.
    pub duration_secs: Option<f64>,
}

/// Where a persisted timestamp column came from.
///
/// The label keeps derived timelines honest: a mux-order position and a
/// container-stored timestamp are different facts, and consumers must
/// be able to tell them apart without knowing the container type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimestampOrigin {
    /// Derived from AVI `movi` mux order (cumulative `strh` ticks), not
    /// a container-stored PTS.
    MoviOrder,
    /// MP4 sample-table decode-order DTS (cumulative `stts` deltas).
    SampleTable,
    /// MP4 `ctts`-derived presentation timestamps, sorted into
    /// presentation order so the column stays monotonic.
    SampleTablePts,
}

/// CFR-free timing observation for one declared stream.
///
/// The same container-neutral shape serves every container: which
/// stream, on which clock, where the timestamps came from
/// (`timestamp_origin`), and the observation over them. For AVI the
/// timestamps are in `strh`-rate ticks (one tick = `1/dwRate` seconds):
/// video frame chunks advance the clock by `dwScale` each, `dwSampleSize`
/// streams (e.g. PCM audio) advance by `len*dwScale/dwSampleSize`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StreamTiming {
    /// Stream index in `hdrl` order (the `NN` prefix of its `movi` chunks).
    pub stream: u32,
    /// Stream kind fourcc as text (`vids`, `auds`, ...).
    pub stream_type: String,
    /// How `stream_timestamps` was produced (see [`TimestampOrigin`]).
    pub timestamp_origin: TimestampOrigin,
    /// Initial stream-clock position: `strh` `dwStart` converted to
    /// rate-domain ticks (`dwStart * dwScale`), the base offset of
    /// `stream_timestamps`. Kept even when the stream has no
    /// `movi` packets.
    pub start_tick: i64,
    /// Per-packet timestamps on this stream's clock, in
    /// `observation.timebase_hz` ticks. For `movi_order` origins,
    /// element `i` is the position of this stream's `i`-th `movi`
    /// packet, derived from `movi` mux order (cumulative `strh` ticks
    /// consumed by earlier packets of the same stream, starting at
    /// `start_tick`) — not a container-stored PTS.
    pub stream_timestamps: Vec<i64>,
    /// Observation over the packet timeline; see `crate::timing`.
    pub observation: TimingObservation,
}

/// Demuxed AVI: optional decoded audio, declared video metadata,
/// coarse video features, and per-stream packet timing.
#[derive(Debug, Clone)]
pub struct AviMedia {
    /// PCM decoded from the first `auds` stream, when that stream
    /// exists and its codec is decodable.
    pub audio: Option<DecodedAudio>,
    /// Why `audio` is absent (`decode/input unavailable` detail).
    pub audio_unavailable: Option<String>,
    /// First `vids` stream's declared metadata, if present.
    pub video: Option<VideoInfo>,
    /// Coarse features of the first `vids` stream — decoded series or
    /// an explicit `unavailable` record — when a `vids` stream exists.
    pub video_features: Option<VideoFeatureArtifact>,
    /// Downscaled preview thumbnails of that same stream, when it
    /// decoded successfully.
    pub video_thumbs: Option<VideoThumbs>,
    /// Packet timing observations for each `auds`/`vids` stream,
    /// in `hdrl` order. Each keeps its own clock — audio and video
    /// timestamps are never merged.
    pub timing: Vec<StreamTiming>,
}

struct StreamHead {
    fcc_type: [u8; 4],
    scale: u32,
    rate: u32,
    start: u32,
    length: u32,
    sample_size: u32,
    format: Vec<u8>,
}

/// One packet inside the `movi` list, tagged with its stream.
struct MoviPacket<'a> {
    /// `hdrl` stream index (the `NN` prefix of the chunk id).
    stream_no: usize,
    /// Chunk kind (the `id[2..4]` suffix: `wb`, `db`, `dc`, ...).
    kind: [u8; 2],
    body: &'a [u8],
}

/// Iterate `(id, body)` chunk pairs inside a RIFF chunk region.
///
/// Validation mirrors the WAV reader: a chunk body that overruns the
/// region, a missing word-alignment pad byte, or leftover bytes too
/// short to form a header are all `Malformed` — never silently
/// ignored. The iterator stops after the first error.
struct Chunks<'a> {
    rest: &'a [u8],
    context: &'static str,
}

/// One validated `(id, body)` pair or a `Malformed` error.
type ChunkItem<'a> = Result<([u8; 4], &'a [u8]), MediaError>;

fn chunks<'a>(data: &'a [u8], context: &'static str) -> Chunks<'a> {
    Chunks {
        rest: data,
        context,
    }
}

impl<'a> Chunks<'a> {
    fn fail(&mut self, msg: impl Into<String>) -> Option<ChunkItem<'a>> {
        self.rest = &[];
        Some(Err(MediaError::malformed(format!(
            "{}: {}",
            self.context,
            msg.into()
        ))))
    }
}

impl<'a> Iterator for Chunks<'a> {
    type Item = ChunkItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }
        if self.rest.len() < 8 {
            return self.fail("trailing bytes shorter than a chunk header");
        }
        let id: [u8; 4] = self.rest[..4].try_into().unwrap();
        let size = u32::from_le_bytes(self.rest[4..8].try_into().unwrap()) as usize;
        let name = || String::from_utf8_lossy(&id).into_owned();
        let Some(body_end) = 8usize.checked_add(size) else {
            return self.fail(format!("chunk {} size overflows", name()));
        };
        if body_end > self.rest.len() {
            return self.fail(format!("chunk {} body extends past end of region", name()));
        }
        // Chunks are word-aligned: an odd size must be followed by a pad
        // byte — including the last chunk in the region.
        if body_end + (size % 2) > self.rest.len() {
            return self.fail(format!("missing pad byte after odd-sized chunk {}", name()));
        }
        let body = &self.rest[8..body_end];
        self.rest = &self.rest[body_end + (size % 2)..];
        Some(Ok((id, body)))
    }
}

fn parse_strh(body: &[u8], head: &mut StreamHead) -> Result<(), MediaError> {
    // The fixed AVISTREAMHEADER spans dwSampleSize at offset 44.
    if body.len() < 48 {
        return Err(MediaError::malformed("strh chunk shorter than 48 bytes"));
    }
    head.fcc_type.copy_from_slice(&body[..4]);
    head.scale = read_u32(body, 20)?;
    head.rate = read_u32(body, 24)?;
    head.start = read_u32(body, 28)?;
    head.length = read_u32(body, 32)?;
    head.sample_size = read_u32(body, 44)?;
    Ok(())
}

fn video_info(head: &StreamHead, movi_frames: u64) -> Result<VideoInfo, MediaError> {
    let bih = &head.format;
    if bih.len() < 40 {
        return Err(MediaError::malformed(
            "video strf shorter than BITMAPINFOHEADER",
        ));
    }
    let (scale, rate) = (head.scale, head.rate);
    let fps = (scale > 0).then(|| rate as f64 / scale as f64);
    let frames = u64::from(head.length).max(movi_frames);
    Ok(VideoInfo {
        codec: codec_fourcc(read_u32(bih, 16)?),
        width: read_u32(bih, 4)?,
        height: (read_u32(bih, 8)? as i32).unsigned_abs(),
        bits_per_pixel: read_u16(bih, 14)?,
        frames,
        fps,
        duration_secs: fps.map(|f| if f > 0.0 { frames as f64 / f } else { 0.0 }),
    })
}

/// Advance one stream's clock past a `movi` packet, in `strh`-rate
/// ticks. A `dwSampleSize` stream (PCM audio) covers `len/sample_size`
/// samples of `dwScale` ticks each; every other packet is one frame.
fn tick_advance(head: &StreamHead, body_len: usize) -> Result<i64, MediaError> {
    let t = if head.sample_size > 0 {
        (body_len as u64 * u64::from(head.scale)) / u64::from(head.sample_size)
    } else {
        u64::from(head.scale)
    };
    i64::try_from(t).map_err(|_| MediaError::malformed("stream clock advance overflows i64"))
}

/// Stream-clock position of every `movi` packet, in mux order — the
/// cumulative `strh` ticks consumed by earlier packets of the same
/// stream, offset by `dwStart`. `None` for packets of streams that are
/// neither `vids` nor `auds` (they own no timeline slot). This is the
/// single source of packet positions: `stream_timings` and the video
/// frame extractor both read it, so frame indices and timestamps can
/// never drift apart.
fn packet_ticks(
    streams: &[StreamHead],
    packets: &[MoviPacket<'_>],
) -> Result<Vec<Option<i64>>, MediaError> {
    let mut next = Vec::with_capacity(streams.len());
    for s in streams {
        next.push(
            i64::from(s.start)
                .checked_mul(i64::from(s.scale))
                .ok_or_else(|| MediaError::malformed("stream start offset overflows i64"))?,
        );
    }
    packets
        .iter()
        .map(|p| match streams.get(p.stream_no) {
            Some(head) if matches!(&head.fcc_type, b"vids" | b"auds") => {
                let t = next[p.stream_no];
                next[p.stream_no] = t
                    .checked_add(tick_advance(head, p.body.len())?)
                    .ok_or_else(|| MediaError::malformed("stream clock overflows i64"))?;
                Ok(Some(t))
            }
            _ => Ok(None),
        })
        .collect()
}

/// Build per-stream packet timing observations from the derived
/// per-packet positions. Observation is delegated to `crate::timing` —
/// clocks stay separate and nothing is resampled to a nominal rate.
fn stream_timings(
    streams: &[StreamHead],
    packets: &[MoviPacket<'_>],
    ticks: &[Option<i64>],
) -> Vec<StreamTiming> {
    let mut pts: Vec<Vec<i64>> = streams.iter().map(|_| Vec::new()).collect();
    for (p, tick) in packets.iter().zip(ticks) {
        if let Some(t) = tick {
            pts[p.stream_no].push(*t);
        }
    }
    streams
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            let clock = match &s.fcc_type {
                b"auds" => ClockKind::Audio,
                b"vids" => ClockKind::Video,
                _ => return None,
            };
            // Audio packets are irregularly sized by design; only video
            // frames have a meaningful nominal interval (dwScale ticks).
            let nominal = (clock == ClockKind::Video && s.scale > 0).then_some(f64::from(s.scale));
            let observation = observe_timing(clock, u64::from(s.rate), &pts[i], nominal);
            Some(StreamTiming {
                stream: i as u32,
                stream_type: String::from_utf8_lossy(&s.fcc_type).into_owned(),
                timestamp_origin: TimestampOrigin::MoviOrder,
                start_tick: i64::from(s.start) * i64::from(s.scale),
                stream_timestamps: std::mem::take(&mut pts[i]),
                observation,
            })
        })
        .collect()
}

/// Demux an AVI file: decode the first PCM audio stream when possible,
/// record the first video stream's declared metadata plus per-stream
/// packet timing, and extract coarse features from uncompressed `vids`
/// frames. `video_cfg` carries the event-detection thresholds persisted
/// into the `video_features` artifact.
///
/// A missing or undecodable `auds` stream is *not* an error — the
/// container's observations (metadata, timing, video features) are
/// valid regardless, so `AviMedia::audio` stays `None` with the reason
/// in `audio_unavailable`. Errors mean the container itself is
/// unusable: bad RIFF/chunk structure or a missing `movi` list.
pub fn decode_avi(data: &[u8], video_cfg: &VideoFeatureConfig) -> Result<AviMedia, MediaError> {
    if data.len() < 12 || &data[0..4] != b"RIFF" || &data[8..12] != b"AVI " {
        return Err(MediaError::input_unavailable("not a RIFF/AVI file"));
    }
    // The RIFF size field declares the whole file minus its own header.
    let riff_size = u64::from(read_u32(data, 4)?);
    if riff_size != (data.len() - 8) as u64 {
        return Err(MediaError::malformed(
            "RIFF size does not match file length",
        ));
    }

    let mut streams: Vec<StreamHead> = Vec::new();
    let mut movi_packets: Vec<MoviPacket<'_>> = Vec::new();
    let mut movi_found = false;

    for chunk in chunks(&data[12..], "RIFF body") {
        let (id, body) = chunk?;
        if &id == b"LIST" {
            if body.len() < 4 {
                return Err(MediaError::malformed("LIST chunk shorter than list type"));
            }
            match &body[..4] {
                b"hdrl" => {
                    for sub in chunks(&body[4..], "hdrl list") {
                        let (sid, sbody) = sub?;
                        if &sid != b"LIST" {
                            continue;
                        }
                        if sbody.len() < 4 {
                            return Err(MediaError::malformed("LIST chunk shorter than list type"));
                        }
                        if &sbody[..4] != b"strl" {
                            continue;
                        }
                        let mut head = StreamHead {
                            fcc_type: [0; 4],
                            scale: 0,
                            rate: 0,
                            start: 0,
                            length: 0,
                            sample_size: 0,
                            format: Vec::new(),
                        };
                        let mut has_strh = false;
                        for inner in chunks(&sbody[4..], "strl list") {
                            let (cid, cbody) = inner?;
                            match &cid {
                                b"strh" => {
                                    parse_strh(cbody, &mut head)?;
                                    has_strh = true;
                                }
                                b"strf" => head.format = cbody.to_vec(),
                                _ => {}
                            }
                        }
                        if has_strh {
                            streams.push(head);
                        }
                    }
                }
                b"movi" => {
                    movi_found = true;
                    for inner in chunks(&body[4..], "movi list") {
                        let (cid, cbody) = inner?;
                        // Chunk ids are `NNdb`/`NNdc` (video) / `NNwb` (audio).
                        if !cid[0].is_ascii_digit() || !cid[1].is_ascii_digit() {
                            continue;
                        }
                        let stream_no = ((cid[0] - b'0') as usize) * 10 + (cid[1] - b'0') as usize;
                        movi_packets.push(MoviPacket {
                            stream_no,
                            kind: [cid[2], cid[3]],
                            body: cbody,
                        });
                    }
                }
                _ => {}
            }
        }
    }
    if !movi_found {
        return Err(MediaError::malformed("missing movi list"));
    }

    let ticks = packet_ticks(&streams, &movi_packets)?;
    let timing = stream_timings(&streams, &movi_packets, &ticks);

    // Only the first `auds` stream is decoded; its `NNwb` packets are
    // concatenated in mux order so interleaved packets of other streams
    // never leak into the PCM. Any decode-level failure — no audio
    // stream, an unsupported codec, corrupt stream headers — is
    // recorded as `audio_unavailable` instead of aborting the demux:
    // metadata, timing, and video observations stay valid on their own.
    let (audio, audio_unavailable) = match streams.iter().position(|s| &s.fcc_type == b"auds") {
        Some(audio_idx) => {
            let mut audio_pcm: Vec<u8> = Vec::new();
            for p in &movi_packets {
                if p.stream_no == audio_idx && &p.kind == b"wb" {
                    audio_pcm.extend_from_slice(p.body);
                }
            }
            // An auds stream with no wb payloads is undecodable input,
            // not a zero-length decode.
            match if audio_pcm.is_empty() {
                Err(MediaError::input_unavailable(
                    "audio stream carries no wb packets",
                ))
            } else {
                decode_waveformatex(&streams[audio_idx].format, &audio_pcm)
            } {
                Ok(a) => (Some(a), None),
                Err(e) => (None, Some(format!("AVI audio stream: {e}"))),
            }
        }
        None => (None, Some("AVI has no audio stream".into())),
    };

    let vidx = streams.iter().position(|s| &s.fcc_type == b"vids");
    // A corrupt/truncated video `strf` degrades like the audio path:
    // `video` metadata is dropped and `extract_features` records an
    // explicit `unavailable` — it is not a fatal container error.
    let video = vidx.and_then(|vidx| {
        let counted = movi_packets
            .iter()
            .filter(|p| p.stream_no == vidx && matches!(&p.kind, b"db" | b"dc"))
            .count() as u64;
        video_info(&streams[vidx], counted).ok()
    });

    // Frame-carrying chunks (`db`/`dc`) are the only ones handed to the
    // decoder; non-frame chunks (`pc` palette changes etc.) keep their
    // timeline slot in `ticks` but never shift the frame index.
    let video_extraction = vidx.map(|vidx| {
        let head = &streams[vidx];
        let frames: Vec<VideoFrame<'_>> = movi_packets
            .iter()
            .enumerate()
            .filter(|(_, p)| p.stream_no == vidx && matches!(&p.kind, b"db" | b"dc"))
            .map(|(i, p)| VideoFrame {
                time_ticks: ticks[i].unwrap_or(0),
                body: p.body,
            })
            .collect();
        video::extract_features(
            vidx as u32,
            &head.format,
            u64::from(head.rate),
            i64::from(head.start) * i64::from(head.scale),
            &frames,
            video_cfg,
        )
    });
    let (video_features, video_thumbs) = match video_extraction {
        Some(e) => (Some(e.features), e.thumbs),
        None => (None, None),
    };

    Ok(AviMedia {
        audio,
        audio_unavailable,
        video,
        video_features,
        video_thumbs,
        timing,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_testkit::media::{AviSpec, push_chunk, push_list, write_avi};

    fn fixture() -> Vec<u8> {
        let frame = vec![7u8; 64 * 48 * 3];
        let audio: Vec<u8> = (0..960i16)
            .flat_map(|v| (v % 200 - 100).to_le_bytes())
            .collect();
        write_avi(&AviSpec::cfr(64, 48, 10, vec![frame; 30], audio, 1, 48_000))
    }

    fn decode(data: &[u8]) -> Result<AviMedia, MediaError> {
        decode_avi(data, &VideoFeatureConfig::default())
    }

    #[test]
    fn roundtrip_decodes_audio_and_video_meta() {
        let bytes = fixture();
        let m = decode(&bytes).unwrap();
        let audio = m.audio.expect("PCM decoded");
        assert_eq!(audio.sample_rate, 48_000);
        assert_eq!(audio.channels, 1);
        assert_eq!(audio.samples.len(), 960);
        let v = m.video.unwrap();
        assert_eq!(v.codec, "rawvideo");
        assert_eq!(v.frames, 30);
        assert_eq!(v.fps, Some(10.0));
        assert_eq!((v.width, v.height), (64, 48));
        // Uncompressed vids frames also produce a decoded feature set.
        let crate::video::VideoFeatureArtifact::Decoded(f) =
            m.video_features.as_ref().expect("video_features present")
        else {
            panic!("expected decoded video features")
        };
        assert_eq!(f.luminance.len(), 30);
        assert_eq!(f.frame_timestamps.len(), 30);
    }

    #[test]
    fn truncated_video_strf_degrades_instead_of_failing_demux() {
        // A `vids` strf shorter than BITMAPINFOHEADER is a stream-level
        // failure, not container corruption: video metadata drops to
        // None, video_features records `unavailable`, and audio PCM +
        // timing survive — the same boundary as a corrupt audio strf.
        let frame = vec![7u8; 64 * 48 * 3];
        let audio: Vec<u8> = (0..960i16)
            .flat_map(|v| (v % 200 - 100).to_le_bytes())
            .collect();
        let mut spec = AviSpec::cfr(64, 48, 10, vec![frame; 4], audio, 1, 48_000);
        spec.streams[0].format = vec![0u8; 20];
        let m = decode(&write_avi(&spec)).unwrap();
        assert!(m.video.is_none());
        assert!(m.audio.is_some());
        assert!(matches!(
            m.video_features,
            Some(crate::video::VideoFeatureArtifact::Unavailable { .. })
        ));
    }

    #[test]
    fn missing_audio_stream_is_partial_ingest_not_error() {
        let mut spec_bytes = fixture();
        // Corrupt the audio stream's fccType so no 'auds' stream exists.
        let pos = spec_bytes.windows(4).position(|w| w == b"auds").unwrap();
        spec_bytes[pos] = b'x';
        let m = decode(&spec_bytes).unwrap();
        assert!(m.audio.is_none());
        assert!(m.audio_unavailable.is_some());
        // Video metadata, timing, and frame features still surface.
        assert!(m.video.is_some());
        assert!(m.video_features.is_some());
        assert!(m.timing.iter().any(|t| t.stream_type == "vids"));
    }

    #[test]
    fn truncated_movi_is_malformed() {
        let e = decode(b"RIFF\x04\0\0\0AVI ").unwrap_err();
        assert!(matches!(e, MediaError::Malformed { .. }));
    }

    /// Wrap `tail` in a RIFF/AVI header whose declared size matches the
    /// actual file length, so inner chunk-boundary errors surface
    /// individually instead of tripping the RIFF size check first.
    fn avi_with_tail(tail: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(tail.len() + 12);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&((tail.len() + 4) as u32).to_le_bytes());
        out.extend_from_slice(b"AVI ");
        out.extend_from_slice(tail);
        out
    }

    fn strh_body(
        fcc: &[u8; 4],
        scale: u32,
        rate: u32,
        start: u32,
        length: u32,
        sample_size: u32,
    ) -> Vec<u8> {
        let mut b = vec![0u8; 56];
        b[..4].copy_from_slice(fcc);
        b[20..24].copy_from_slice(&scale.to_le_bytes());
        b[24..28].copy_from_slice(&rate.to_le_bytes());
        b[28..32].copy_from_slice(&start.to_le_bytes());
        b[32..36].copy_from_slice(&length.to_le_bytes());
        b[44..48].copy_from_slice(&sample_size.to_le_bytes());
        b
    }

    fn wavefmt_body(format_tag: u16, channels: u16, sample_rate: u32, bits: u16) -> Vec<u8> {
        let block_align = channels * (bits / 8);
        let mut b = Vec::new();
        b.extend_from_slice(&format_tag.to_le_bytes());
        b.extend_from_slice(&channels.to_le_bytes());
        b.extend_from_slice(&sample_rate.to_le_bytes());
        b.extend_from_slice(&(sample_rate * u32::from(block_align)).to_le_bytes());
        b.extend_from_slice(&block_align.to_le_bytes());
        b.extend_from_slice(&bits.to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes());
        b
    }

    /// An AVI whose only stream is one `auds` stream carrying `pcm`
    /// (its strh block align / byte rate match `wavefmt` semantics).
    fn audio_only_avi(format_tag: u16, pcm: &[u8]) -> Vec<u8> {
        let strf = wavefmt_body(format_tag, 1, 48_000, 16);
        let mut astrl = Vec::new();
        push_chunk(&mut astrl, b"strh", &strh_body(b"auds", 2, 96_000, 0, 0, 2));
        push_chunk(&mut astrl, b"strf", &strf);
        let mut hdrl = Vec::new();
        push_list(&mut hdrl, b"strl", &astrl);
        let mut movi = Vec::new();
        push_chunk(&mut movi, b"00wb", pcm);
        let mut body = Vec::new();
        push_list(&mut body, b"hdrl", &hdrl);
        push_list(&mut body, b"movi", &movi);
        avi_with_tail(&body)
    }

    #[test]
    fn riff_size_mismatch_is_malformed() {
        let mut bytes = fixture();
        bytes[4] = bytes[4].wrapping_add(8);
        let e = decode(&bytes).unwrap_err();
        assert!(matches!(e, MediaError::Malformed { .. }));
    }

    #[test]
    fn valid_chunks_then_cut_off_is_malformed() {
        // The earlier decoder kept whatever decoded before the cut.
        // With a repaired RIFF size the failure must come from the
        // chunk-boundary check, not the outer size check.
        let mut bytes = fixture();
        bytes.truncate(bytes.len() - 100);
        let riff = (bytes.len() - 8) as u32;
        bytes[4..8].copy_from_slice(&riff.to_le_bytes());
        let e = decode(&bytes).unwrap_err();
        assert!(matches!(e, MediaError::Malformed { .. }));
    }

    #[test]
    fn chunk_body_overrunning_region_is_malformed() {
        let avi = avi_with_tail(b"JUNK\x10\0\0\0ab");
        let e = decode(&avi).unwrap_err();
        assert!(matches!(e, MediaError::Malformed { .. }));
    }

    #[test]
    fn missing_pad_byte_is_malformed() {
        // Odd-sized chunk whose body reaches the file end without its
        // required word-alignment pad byte.
        let mut tail = Vec::new();
        push_chunk(&mut tail, b"JUNK", &[1, 2, 3, 4]);
        tail.extend_from_slice(b"JUNK");
        tail.extend_from_slice(&3u32.to_le_bytes());
        tail.extend_from_slice(&[9, 9, 9]);
        let e = decode(&avi_with_tail(&tail)).unwrap_err();
        assert!(matches!(e, MediaError::Malformed { .. }));
    }

    #[test]
    fn trailing_garbage_is_malformed() {
        let mut tail = Vec::new();
        push_chunk(&mut tail, b"JUNK", &[1, 2, 3, 4]);
        tail.extend_from_slice(b"ab"); // shorter than a chunk header
        let e = decode(&avi_with_tail(&tail)).unwrap_err();
        assert!(matches!(e, MediaError::Malformed { .. }));
    }

    #[test]
    fn unsupported_audio_codec_is_partial_ingest_not_error() {
        // format_tag 2 (ADPCM): a structurally valid AVI with an
        // unsupported codec keeps its observations — audio is simply
        // absent with the `decode/input unavailable` reason recorded.
        let avi = audio_only_avi(2, &[0u8; 64]);
        let m = decode(&avi).unwrap();
        assert!(m.audio.is_none());
        assert!(
            m.audio_unavailable
                .as_deref()
                .is_some_and(|r| r.contains("decode/input unavailable"))
        );
        // The audio stream's timing observation still surfaces.
        assert_eq!(m.timing.len(), 1);
        assert_eq!(m.timing[0].stream_type, "auds");
    }

    #[test]
    fn only_first_auds_stream_is_decoded() {
        // Stream 0: video. Stream 1: mono s16 audio (the one to decode).
        // Stream 2: stereo s16 audio — its packets must never leak into
        // stream 1's PCM.
        let audio_a: Vec<u8> = (0..40i16).flat_map(|v| (v + 100).to_le_bytes()).collect();
        let audio_b: Vec<u8> = (0..80i16).flat_map(|v| (v - 500).to_le_bytes()).collect();
        let m = decode(&multi_stream_avi(&audio_a, &audio_b)).unwrap();
        let audio = m.audio.expect("PCM decoded");

        assert_eq!(audio.channels, 1);
        let expected: Vec<f64> = (0..40i16).map(|v| f64::from(v + 100)).collect();
        assert_eq!(audio.samples, expected);
        // Stream 2's packets keep their own timing entry, never PCM.
        let s2 = m.timing.iter().find(|t| t.stream == 2).unwrap();
        assert_eq!(s2.observation.timestamp_count, 1);
    }

    #[test]
    fn packet_timing_is_observed_per_stream_clock() {
        let m = decode(&fixture()).unwrap();
        assert_eq!(m.timing.len(), 2);

        // Video stream 0: 30 frames of 1 tick at a 10 Hz clock.
        let v = m.timing.iter().find(|t| t.stream_type == "vids").unwrap();
        assert_eq!(v.observation.clock, ClockKind::Video);
        assert_eq!(v.observation.timebase_hz, 10);
        assert_eq!(v.observation.timestamp_count, 30);
        assert!(v.observation.intervals.iter().all(|&i| i == 1));
        assert_eq!(v.observation.drift_ppm, Some(0.0));
        // The derived per-packet column is persisted, not just the
        // aggregate observation: one entry per packet, dwStart-based,
        // with its container-neutral origin label.
        assert_eq!(v.start_tick, 0);
        assert_eq!(v.timestamp_origin, TimestampOrigin::MoviOrder);
        assert_eq!(v.stream_timestamps, (0..30).collect::<Vec<i64>>());

        // Audio stream 1: a single wb packet covering its PCM, on the
        // byte-rate clock (block_align scale, byte_rate rate).
        let a = m.timing.iter().find(|t| t.stream_type == "auds").unwrap();
        assert_eq!(a.observation.clock, ClockKind::Audio);
        assert_eq!(a.observation.timebase_hz, 96_000);
        assert_eq!(a.observation.timestamp_count, 1);
        assert_eq!(a.start_tick, 0);
        assert_eq!(a.stream_timestamps, vec![0]);

        // Interleaved packet order maps to cumulative stream positions:
        // stream 1's two wb packets sit at 0 and len(first) ticks.
        let audio_a: Vec<u8> = (0..40i16).flat_map(|v| (v + 100).to_le_bytes()).collect();
        let half = audio_a.len() / 2;
        let m2 = decode(&multi_stream_avi(&audio_a, &[7u8; 32])).unwrap();
        let a1 = m2.timing.iter().find(|t| t.stream == 1).unwrap();
        assert_eq!(a1.observation.timestamp_count, 2);
        assert_eq!(a1.observation.intervals, vec![half as i64]);
        assert_eq!(a1.stream_timestamps, vec![0, half as i64]);
    }

    #[test]
    fn derived_timestamps_keep_dwstart_offset() {
        // vids strh: dwStart=5, dwScale=2 on a 10 Hz clock → the derived
        // timeline must start at 10 rate-domain ticks, and every packet
        // position keeps that offset.
        let mut hdrl = Vec::new();
        let mut vstrl = Vec::new();
        push_chunk(&mut vstrl, b"strh", &strh_body(b"vids", 2, 10, 5, 3, 0));
        push_chunk(&mut vstrl, b"strf", &[0u8; 40]);
        push_list(&mut hdrl, b"strl", &vstrl);
        let mut astrl = Vec::new();
        push_chunk(&mut astrl, b"strh", &strh_body(b"auds", 2, 96_000, 0, 0, 2));
        push_chunk(&mut astrl, b"strf", &wavefmt_body(1, 1, 48_000, 16));
        push_list(&mut hdrl, b"strl", &astrl);
        let mut movi = Vec::new();
        push_chunk(&mut movi, b"00db", &[0u8; 4]);
        push_chunk(&mut movi, b"00db", &[0u8; 4]);
        push_chunk(&mut movi, b"00db", &[0u8; 4]);
        push_chunk(&mut movi, b"01wb", &[0u8; 64]);
        let mut body = Vec::new();
        push_list(&mut body, b"hdrl", &hdrl);
        push_list(&mut body, b"movi", &movi);
        let m = decode(&avi_with_tail(&body)).unwrap();

        let v = m.timing.iter().find(|t| t.stream_type == "vids").unwrap();
        assert_eq!(v.start_tick, 10); // dwStart * dwScale
        assert_eq!(v.stream_timestamps, vec![10, 12, 14]);
        assert_eq!(v.observation.intervals, vec![2, 2]);
    }

    /// BITMAPINFOHEADER body for uncompressed-video fixtures.
    fn bih(width: i32, height: i32, bpp: u16, compression: u32) -> Vec<u8> {
        let mut b = vec![0u8; 40];
        b[4..8].copy_from_slice(&width.to_le_bytes());
        b[8..12].copy_from_slice(&height.to_le_bytes());
        b[14..16].copy_from_slice(&bpp.to_le_bytes());
        b[16..20].copy_from_slice(&compression.to_le_bytes());
        b
    }

    #[test]
    fn non_frame_vids_chunks_dont_desync_frame_indices() {
        // movi: db, pc, db — the `pc` palette chunk keeps a timeline
        // slot but must never count as a frame.
        let frame_a = vec![0u8; 8]; // 2x1 24bpp black (stride 8)
        let frame_b = vec![200u8; 8];
        let mut hdrl = Vec::new();
        let mut vstrl = Vec::new();
        push_chunk(&mut vstrl, b"strh", &strh_body(b"vids", 1, 10, 0, 3, 0));
        push_chunk(&mut vstrl, b"strf", &bih(2, 1, 24, 0));
        push_list(&mut hdrl, b"strl", &vstrl);
        let mut astrl = Vec::new();
        push_chunk(&mut astrl, b"strh", &strh_body(b"auds", 2, 96_000, 0, 0, 2));
        push_chunk(&mut astrl, b"strf", &wavefmt_body(1, 1, 48_000, 16));
        push_list(&mut hdrl, b"strl", &astrl);
        let mut movi = Vec::new();
        push_chunk(&mut movi, b"00db", &frame_a);
        push_chunk(&mut movi, b"00pc", &[0u8; 8]); // palette change: a packet, not a frame
        push_chunk(&mut movi, b"00db", &frame_b);
        push_chunk(&mut movi, b"01wb", &[0u8; 64]);
        let mut body = Vec::new();
        push_list(&mut body, b"hdrl", &hdrl);
        push_list(&mut body, b"movi", &movi);
        let m = decode(&avi_with_tail(&body)).unwrap();

        // Every vids packet owns a timeline slot — including `pc`.
        let v = m.timing.iter().find(|t| t.stream_type == "vids").unwrap();
        assert_eq!(v.stream_timestamps, vec![0, 1, 2]);
        // But only db/dc chunks produce frames — each tagged with its
        // own stream-clock position, not a sequential index.
        let crate::video::VideoFeatureArtifact::Decoded(f) =
            m.video_features.as_ref().expect("video_features")
        else {
            panic!("expected decoded video features")
        };
        assert_eq!(f.frame_timestamps, vec![0, 2]);
        assert_eq!(f.luminance.len(), 2);
        assert_eq!(f.diff.len(), 2);
        // The cut at frame 1 sits at tick 2 (after the pc slot), and
        // the event carries that stream-clock time.
        assert!(
            f.events
                .iter()
                .any(|e| e.kind == crate::video::VideoEventKind::SceneChange && e.time_ticks == 2)
        );
    }

    #[test]
    fn compressed_vids_is_explicit_unavailable() {
        // MJPG-compressed vids: metadata and timing are recorded, and
        // video_features says `unavailable` instead of failing.
        let mut hdrl = Vec::new();
        let mut vstrl = Vec::new();
        push_chunk(&mut vstrl, b"strh", &strh_body(b"vids", 1, 10, 0, 1, 0));
        push_chunk(
            &mut vstrl,
            b"strf",
            &bih(2, 1, 24, u32::from_le_bytes(*b"MJPG")),
        );
        push_list(&mut hdrl, b"strl", &vstrl);
        let mut astrl = Vec::new();
        push_chunk(&mut astrl, b"strh", &strh_body(b"auds", 2, 96_000, 0, 0, 2));
        push_chunk(&mut astrl, b"strf", &wavefmt_body(1, 1, 48_000, 16));
        push_list(&mut hdrl, b"strl", &astrl);
        let mut movi = Vec::new();
        push_chunk(&mut movi, b"00dc", &[0u8; 16]);
        push_chunk(&mut movi, b"01wb", &[0u8; 64]);
        let mut body = Vec::new();
        push_list(&mut body, b"hdrl", &hdrl);
        push_list(&mut body, b"movi", &movi);
        let m = decode(&avi_with_tail(&body)).unwrap();

        assert!(m.audio.is_some());
        assert_eq!(m.video.as_ref().unwrap().codec, "MJPG");
        assert!(matches!(
            m.video_features,
            Some(crate::video::VideoFeatureArtifact::Unavailable { .. })
        ));
    }

    /// Two-audio-stream AVI (video + mono auds + stereo auds) with
    /// stream 1's audio split across two interleaved `01wb` packets.
    fn multi_stream_avi(audio_a: &[u8], audio_b: &[u8]) -> Vec<u8> {
        let half = audio_a.len() / 2;
        let mut hdrl = Vec::new();
        let mut vstrl = Vec::new();
        push_chunk(&mut vstrl, b"strh", &strh_body(b"vids", 1, 10, 0, 2, 0));
        push_chunk(&mut vstrl, b"strf", &[0u8; 40]);
        push_list(&mut hdrl, b"strl", &vstrl);
        for (strh, strf) in [
            (
                strh_body(b"auds", 2, 96_000, 0, 0, 2),
                wavefmt_body(1, 1, 48_000, 16),
            ),
            (
                strh_body(b"auds", 4, 192_000, 0, 0, 4),
                wavefmt_body(1, 2, 48_000, 16),
            ),
        ] {
            let mut astrl = Vec::new();
            push_chunk(&mut astrl, b"strh", &strh);
            push_chunk(&mut astrl, b"strf", &strf);
            push_list(&mut hdrl, b"strl", &astrl);
        }
        let mut movi = Vec::new();
        push_chunk(&mut movi, b"00db", &[0u8; 4]);
        push_chunk(&mut movi, b"01wb", &audio_a[..half]);
        push_chunk(&mut movi, b"02wb", audio_b);
        push_chunk(&mut movi, b"00db", &[0u8; 4]);
        push_chunk(&mut movi, b"01wb", &audio_a[half..]);
        let mut body = Vec::new();
        push_list(&mut body, b"hdrl", &hdrl);
        push_list(&mut body, b"movi", &movi);
        avi_with_tail(&body)
    }
}
