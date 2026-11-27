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
    /// 局所失敗の型付き区分（文字列化より前に決定）。
    pub audio_failure: Option<crate::decoder::Unavailable>,
    /// 映像側の局所失敗区分。
    pub video_failure: Option<crate::decoder::Unavailable>,
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

#[derive(Debug)]
pub(crate) struct StreamHead {
    pub(crate) fcc_type: [u8; 4],
    pub(crate) scale: u32,
    pub(crate) rate: u32,
    pub(crate) start: u32,
    pub(crate) length: u32,
    pub(crate) sample_size: u32,
    pub(crate) format: Vec<u8>,
}

const STRH_HEADER_LEN: usize = 48;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StrhDeclarationFields {
    fcc_type: [u8; 4],
    scale: u32,
    rate: u32,
    start: u32,
    length: u32,
    sample_size: u32,
}

fn avi_strh_accepts_body_len(body_len: usize) -> bool {
    body_len >= STRH_HEADER_LEN
}

fn avi_strh_declaration_fields(prefix: &[u8; STRH_HEADER_LEN]) -> StrhDeclarationFields {
    StrhDeclarationFields {
        fcc_type: [prefix[0], prefix[1], prefix[2], prefix[3]],
        scale: u32::from_le_bytes([prefix[20], prefix[21], prefix[22], prefix[23]]),
        rate: u32::from_le_bytes([prefix[24], prefix[25], prefix[26], prefix[27]]),
        start: u32::from_le_bytes([prefix[28], prefix[29], prefix[30], prefix[31]]),
        length: u32::from_le_bytes([prefix[32], prefix[33], prefix[34], prefix[35]]),
        sample_size: u32::from_le_bytes([prefix[44], prefix[45], prefix[46], prefix[47]]),
    }
}

/// One packet inside the `movi` list, tagged with its stream.
#[derive(Debug)]
pub(crate) struct MoviPacket<'a> {
    /// `hdrl` stream index (the `NN` prefix of the chunk id).
    pub(crate) stream_no: usize,
    /// Chunk kind (the `id[2..4]` suffix: `wb`, `db`, `dc`, ...).
    kind: [u8; 2],
    pub(crate) body: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MoviPacketTag {
    stream_no: usize,
    kind: [u8; 2],
}

fn avi_movi_packet_tag(id: [u8; 4]) -> Option<MoviPacketTag> {
    if !id[0].is_ascii_digit() || !id[1].is_ascii_digit() {
        return None;
    }
    let stream_no = usize::from(id[0] - b'0') * 10 + usize::from(id[1] - b'0');
    Some(MoviPacketTag {
        stream_no,
        kind: [id[2], id[3]],
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MoviEntryDecision {
    Packet(MoviPacketTag),
    Ignore,
    RejectShortListType,
    RejectRecList,
}

fn avi_movi_entry_decision(id: [u8; 4], list_type_view: Option<[u8; 4]>) -> MoviEntryDecision {
    if &id == b"LIST" {
        return match list_type_view {
            None => MoviEntryDecision::RejectShortListType,
            Some(list_type) if &list_type == b"rec " => MoviEntryDecision::RejectRecList,
            Some(_) => MoviEntryDecision::Ignore,
        };
    }
    match avi_movi_packet_tag(id) {
        Some(tag) => MoviEntryDecision::Packet(tag),
        None => MoviEntryDecision::Ignore,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MoviScanReject {
    ShortListType,
    RecList,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MoviScanStep {
    Collect(MoviPacketTag),
    Skip,
    Stop(MoviScanReject),
}

fn avi_movi_scan_step(decision: MoviEntryDecision) -> MoviScanStep {
    match decision {
        MoviEntryDecision::Packet(tag) => MoviScanStep::Collect(tag),
        MoviEntryDecision::Ignore => MoviScanStep::Skip,
        MoviEntryDecision::RejectShortListType => MoviScanStep::Stop(MoviScanReject::ShortListType),
        MoviEntryDecision::RejectRecList => MoviScanStep::Stop(MoviScanReject::RecList),
    }
}

trait AviStreamTypeView {
    fn fcc_type(&self) -> [u8; 4];
}

impl AviStreamTypeView for StreamHead {
    fn fcc_type(&self) -> [u8; 4] {
        self.fcc_type
    }
}

impl AviStreamTypeView for [u8; 4] {
    fn fcc_type(&self) -> [u8; 4] {
        *self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AviStreamTypeClass {
    Audio,
    Video,
    Other,
}

fn avi_stream_type_class(fcc_type: [u8; 4]) -> AviStreamTypeClass {
    if &fcc_type == b"auds" {
        AviStreamTypeClass::Audio
    } else if &fcc_type == b"vids" {
        AviStreamTypeClass::Video
    } else {
        AviStreamTypeClass::Other
    }
}

fn avi_stream_type_at<T: AviStreamTypeView>(streams: &[T], stream_no: usize) -> Option<[u8; 4]> {
    streams.get(stream_no).map(|stream| stream.fcc_type())
}

fn avi_stream_type_class_at<T: AviStreamTypeView>(
    streams: &[T],
    stream_no: usize,
) -> Option<AviStreamTypeClass> {
    avi_stream_type_at(streams, stream_no).map(avi_stream_type_class)
}

fn avi_first_stream_of_class<T: AviStreamTypeView>(
    streams: &[T],
    class: AviStreamTypeClass,
) -> Option<usize> {
    streams
        .iter()
        .position(|stream| avi_stream_type_class(stream.fcc_type()) == class)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AviPacketKindRole {
    AudioPayload,
    VideoFrame,
    NonPayload,
}

fn avi_packet_kind_role(kind: [u8; 2]) -> AviPacketKindRole {
    if &kind == b"wb" {
        AviPacketKindRole::AudioPayload
    } else if matches!(&kind, b"db" | b"dc") {
        AviPacketKindRole::VideoFrame
    } else {
        AviPacketKindRole::NonPayload
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AviPacketStreamMeaning {
    AudioPayload,
    AudioTimelineOnly,
    VideoFrame,
    VideoTimelineOnly,
    NoTimelineSlot,
}

fn avi_packet_stream_meaning<T: AviStreamTypeView>(
    streams: &[T],
    stream_no: usize,
    kind: [u8; 2],
) -> AviPacketStreamMeaning {
    match avi_stream_type_class_at(streams, stream_no) {
        Some(AviStreamTypeClass::Audio) => {
            if avi_packet_kind_role(kind) == AviPacketKindRole::AudioPayload {
                AviPacketStreamMeaning::AudioPayload
            } else {
                AviPacketStreamMeaning::AudioTimelineOnly
            }
        }
        Some(AviStreamTypeClass::Video) => {
            if avi_packet_kind_role(kind) == AviPacketKindRole::VideoFrame {
                AviPacketStreamMeaning::VideoFrame
            } else {
                AviPacketStreamMeaning::VideoTimelineOnly
            }
        }
        _ => AviPacketStreamMeaning::NoTimelineSlot,
    }
}

fn avi_packet_owns_timeline_slot<T: AviStreamTypeView>(streams: &[T], stream_no: usize) -> bool {
    matches!(
        avi_stream_type_class_at(streams, stream_no),
        Some(AviStreamTypeClass::Audio | AviStreamTypeClass::Video)
    )
}

fn avi_packet_is_selected_audio_payload<T: AviStreamTypeView>(
    streams: &[T],
    stream_no: usize,
    kind: [u8; 2],
    selected_stream: usize,
) -> bool {
    stream_no == selected_stream
        && avi_packet_stream_meaning(streams, stream_no, kind)
            == AviPacketStreamMeaning::AudioPayload
}

fn avi_packet_is_selected_video_frame<T: AviStreamTypeView>(
    streams: &[T],
    stream_no: usize,
    kind: [u8; 2],
    selected_stream: usize,
) -> bool {
    stream_no == selected_stream
        && avi_packet_stream_meaning(streams, stream_no, kind) == AviPacketStreamMeaning::VideoFrame
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AviChunkBoundaryError {
    SizeOverflow,
    ExtendsPastEnd,
    MissingPad,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AviChunkAdvance {
    body_start: u64,
    body_end: u64,
    next_offset: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AviChunkPosition {
    HeaderAvailable,
    End,
    TrailingBytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AviChunkStep {
    Advance(AviChunkAdvance),
    Invalid(AviChunkBoundaryError),
    End,
    TrailingBytes,
}

fn avi_chunk_position(offset: u64, region_len: u64) -> AviChunkPosition {
    if offset == region_len {
        return AviChunkPosition::End;
    }
    if offset > region_len || region_len - offset < 8 {
        return AviChunkPosition::TrailingBytes;
    }
    AviChunkPosition::HeaderAvailable
}

fn avi_chunk_step(offset: u64, region_len: u64, size: u32) -> AviChunkStep {
    match avi_chunk_position(offset, region_len) {
        AviChunkPosition::End => return AviChunkStep::End,
        AviChunkPosition::TrailingBytes => return AviChunkStep::TrailingBytes,
        AviChunkPosition::HeaderAvailable => {}
    }

    let body_start = offset + 8;
    let Some(body_end) = body_start.checked_add(u64::from(size)) else {
        return AviChunkStep::Invalid(AviChunkBoundaryError::SizeOverflow);
    };
    if body_end > region_len {
        return AviChunkStep::Invalid(AviChunkBoundaryError::ExtendsPastEnd);
    }
    // Chunks are word-aligned: an odd size must be followed by a pad
    // byte — including the last chunk in the region.
    let pad = u64::from(size & 1);
    let Some(next_offset) = body_end.checked_add(pad) else {
        return AviChunkStep::Invalid(AviChunkBoundaryError::MissingPad);
    };
    if next_offset > region_len {
        return AviChunkStep::Invalid(AviChunkBoundaryError::MissingPad);
    }
    AviChunkStep::Advance(AviChunkAdvance {
        body_start,
        body_end,
        next_offset,
    })
}

/// Iterate `(id, body)` chunk pairs inside a RIFF chunk region.
///
/// Validation mirrors the WAV reader: a chunk body that overruns the
/// region, a missing word-alignment pad byte, or leftover bytes too
/// short to form a header are all `Malformed` — never silently
/// ignored. The iterator stops after the first error.
struct Chunks<'a> {
    rest: &'a [u8],
    offset: u64,
    region_len: u64,
    context: &'static str,
}

/// One validated `(id, body)` pair or a `Malformed` error.
type ChunkItem<'a> = Result<([u8; 4], &'a [u8]), MediaError>;

fn chunks<'a>(data: &'a [u8], context: &'static str) -> Chunks<'a> {
    Chunks {
        rest: data,
        offset: 0,
        region_len: data.len() as u64,
        context,
    }
}

impl<'a> Chunks<'a> {
    fn fail(&mut self, msg: impl Into<String>) -> Option<ChunkItem<'a>> {
        self.rest = &[];
        self.offset = self.region_len;
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
        match avi_chunk_position(self.offset, self.region_len) {
            AviChunkPosition::End => return None,
            AviChunkPosition::TrailingBytes => {
                return self.fail("trailing bytes shorter than a chunk header");
            }
            AviChunkPosition::HeaderAvailable => {}
        }
        let id: [u8; 4] = self.rest[..4].try_into().unwrap();
        let size = u32::from_le_bytes(self.rest[4..8].try_into().unwrap());
        let name = || String::from_utf8_lossy(&id).into_owned();
        let advance = match avi_chunk_step(self.offset, self.region_len, size) {
            AviChunkStep::Advance(advance) => advance,
            AviChunkStep::Invalid(AviChunkBoundaryError::SizeOverflow) => {
                return self.fail(format!("chunk {} size overflows", name()));
            }
            AviChunkStep::Invalid(AviChunkBoundaryError::ExtendsPastEnd) => {
                return self.fail(format!("chunk {} body extends past end of region", name()));
            }
            AviChunkStep::Invalid(AviChunkBoundaryError::MissingPad) => {
                return self.fail(format!("missing pad byte after odd-sized chunk {}", name()));
            }
            AviChunkStep::End => return None,
            AviChunkStep::TrailingBytes => {
                return self.fail("trailing bytes shorter than a chunk header");
            }
        };
        let body_len = (advance.body_end - advance.body_start) as usize;
        let consumed = (advance.next_offset - self.offset) as usize;
        let body = &self.rest[8..8 + body_len];
        self.rest = &self.rest[consumed..];
        self.offset = advance.next_offset;
        Some(Ok((id, body)))
    }
}

fn parse_strh(body: &[u8], head: &mut StreamHead) -> Result<(), MediaError> {
    // The fixed AVISTREAMHEADER spans dwSampleSize at offset 44.
    if !avi_strh_accepts_body_len(body.len()) {
        return Err(MediaError::malformed("strh chunk shorter than 48 bytes"));
    }
    let mut prefix = [0u8; STRH_HEADER_LEN];
    prefix.copy_from_slice(&body[..STRH_HEADER_LEN]);
    let fields = avi_strh_declaration_fields(&prefix);
    head.fcc_type = fields.fcc_type;
    head.scale = fields.scale;
    head.rate = fields.rate;
    head.start = fields.start;
    head.length = fields.length;
    head.sample_size = fields.sample_size;
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
/// samples of `dwScale` ticks each; every other packet consumes one slot.
/// 非フレーム vids (`pc` 等)も slot を消費する。frame index は db/dc のみを
/// 数えるため index×scale と時刻を同一視せず、persisted ticks を使用する。
fn tick_advance(
    head: &StreamHead,
    body_len: usize,
    remainder: &mut u128,
) -> Result<i64, MediaError> {
    let t = if head.sample_size > 0 {
        // 分子の余りを stream ごとに保持し、パケット境界で時計を丸めない。
        let numerator = body_len as u128 * u128::from(head.scale) + *remainder;
        *remainder = numerator % u128::from(head.sample_size);
        numerator / u128::from(head.sample_size)
    } else {
        u128::from(head.scale)
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
    let mut remainders = vec![0; streams.len()];
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
            Some(head) if avi_packet_owns_timeline_slot(streams, p.stream_no) => {
                let t = next[p.stream_no];
                next[p.stream_no] = t
                    .checked_add(tick_advance(
                        head,
                        p.body.len(),
                        &mut remainders[p.stream_no],
                    )?)
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
            let clock = match avi_stream_type_class(s.fcc_type) {
                AviStreamTypeClass::Audio => ClockKind::Audio,
                AviStreamTypeClass::Video => ClockKind::Video,
                AviStreamTypeClass::Other => return None,
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

pub(crate) struct AviLayout<'a> {
    pub(crate) streams: Vec<StreamHead>,
    pub(crate) packets: Vec<MoviPacket<'a>>,
    ticks: Vec<Option<i64>>,
    pub(crate) timing: Vec<StreamTiming>,
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
    decode_layout(avi_layout(data)?, video_cfg, true, true)
}

pub(crate) fn avi_layout(data: &[u8]) -> Result<AviLayout<'_>, MediaError> {
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
                        let list_type_view = if &cid == b"LIST" {
                            cbody.get(..4).map(|bytes| bytes.try_into().unwrap())
                        } else {
                            None
                        };
                        match avi_movi_scan_step(avi_movi_entry_decision(cid, list_type_view)) {
                            MoviScanStep::Stop(MoviScanReject::ShortListType) => {
                                return Err(MediaError::malformed(
                                    "movi LIST shorter than list type",
                                ));
                            }
                            MoviScanStep::Stop(MoviScanReject::RecList) => {
                                return Err(MediaError::input_unavailable(
                                    "OpenDML/nested rec LIST is not supported",
                                ));
                            }
                            MoviScanStep::Skip => {}
                            // Chunk ids are `NNdb`/`NNdc` (video) / `NNwb` (audio).
                            MoviScanStep::Collect(tag) => {
                                movi_packets.push(MoviPacket {
                                    stream_no: tag.stream_no,
                                    kind: tag.kind,
                                    body: cbody,
                                });
                            }
                        }
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

    Ok(AviLayout {
        streams,
        packets: movi_packets,
        ticks,
        timing,
    })
}

pub(crate) fn decode_layout(
    layout: AviLayout<'_>,
    video_cfg: &VideoFeatureConfig,
    audio_requested: bool,
    video_requested: bool,
) -> Result<AviMedia, MediaError> {
    let AviLayout {
        streams,
        packets: movi_packets,
        ticks,
        timing,
    } = layout;
    // Only the first `auds` stream is decoded; its `NNwb` packets are
    // concatenated in mux order so interleaved packets of other streams
    // never leak into the PCM. Any decode-level failure — no audio
    // stream, an unsupported codec, corrupt stream headers — is
    // recorded as `audio_unavailable` instead of aborting the demux:
    // metadata, timing, and video observations stay valid on their own.
    let (audio, audio_unavailable, audio_failure) = if !audio_requested {
        (None, None, None)
    } else {
        match avi_first_stream_of_class(&streams, AviStreamTypeClass::Audio) {
            Some(audio_idx) => {
                let mut audio_pcm: Vec<u8> = Vec::new();
                for p in &movi_packets {
                    if avi_packet_is_selected_audio_payload(
                        &streams,
                        p.stream_no,
                        p.kind,
                        audio_idx,
                    ) {
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
                    Ok(a) => (Some(a), None, None),
                    Err(e) => (
                        None,
                        Some(format!("AVI audio stream: {e}")),
                        Some(crate::decoder::Unavailable::from_error(&e)),
                    ),
                }
            }
            None => (
                None,
                Some("AVI has no audio stream".into()),
                Some(crate::decoder::Unavailable::capability(
                    "AVI has no audio stream",
                )),
            ),
        }
    };

    let vidx = avi_first_stream_of_class(&streams, AviStreamTypeClass::Video);
    // A corrupt/truncated video `strf` degrades like the audio path:
    // `video` metadata is dropped and `extract_features` records an
    // explicit `unavailable` — it is not a fatal container error.
    let video = vidx.and_then(|vidx| {
        let counted = movi_packets
            .iter()
            .filter(|p| avi_packet_is_selected_video_frame(&streams, p.stream_no, p.kind, vidx))
            .count() as u64;
        video_info(&streams[vidx], counted).ok()
    });

    // Frame-carrying chunks (`db`/`dc`) are the only ones handed to the
    // decoder; non-frame chunks (`pc` palette changes etc.) keep their
    // timeline slot in `ticks` but never shift the frame index.
    let video_extraction = vidx.filter(|_| video_requested).map(|vidx| {
        let head = &streams[vidx];
        let frames: Vec<VideoFrame<'_>> = movi_packets
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                avi_packet_is_selected_video_frame(&streams, p.stream_no, p.kind, vidx)
            })
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
    let (video_features, video_thumbs, video_failure) = match video_extraction {
        Some(e) => (Some(e.features), e.thumbs, e.unavailable),
        None => (None, None, None),
    };

    Ok(AviMedia {
        audio,
        audio_unavailable,
        audio_failure,
        video_failure,
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

    pub(super) fn strh_body(
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
    fn movi_packet_tags_preserve_stream_and_opaque_kind() {
        let mut hdrl = Vec::new();
        let mut vstrl = Vec::new();
        push_chunk(&mut vstrl, b"strh", &strh_body(b"vids", 1, 10, 0, 1, 0));
        push_chunk(&mut vstrl, b"strf", &bih(2, 1, 24, 0));
        push_list(&mut hdrl, b"strl", &vstrl);
        let mut movi = Vec::new();
        push_chunk(&mut movi, b"99zz", &[7u8, 8]);
        push_chunk(&mut movi, b"a0wb", &[1u8, 2]);
        push_chunk(&mut movi, b"0awb", &[3u8, 4]);
        push_chunk(&mut movi, b"00pc", &[5u8, 6]);
        let mut body = Vec::new();
        push_list(&mut body, b"hdrl", &hdrl);
        push_list(&mut body, b"movi", &movi);
        let bytes = avi_with_tail(&body);
        let layout = avi_layout(&bytes).unwrap();

        assert_eq!(layout.packets.len(), 2);
        assert_eq!(layout.packets[0].stream_no, 99);
        assert_eq!(layout.packets[0].kind, *b"zz");
        assert_eq!(layout.packets[0].body, &[7u8, 8]);
        assert_eq!(layout.packets[1].stream_no, 0);
        assert_eq!(layout.packets[1].kind, *b"pc");
        assert_eq!(layout.packets[1].body, &[5u8, 6]);
    }

    fn avi_with_movi_entries(entries: &[u8]) -> Vec<u8> {
        let mut body = Vec::new();
        push_list(&mut body, b"movi", entries);
        avi_with_tail(&body)
    }

    #[test]
    fn exact_rec_movi_list_is_input_unavailable() {
        let mut movi = Vec::new();
        push_list(&mut movi, b"rec ", &[]);
        let bytes = avi_with_movi_entries(&movi);
        let e = avi_layout(&bytes).err().unwrap();
        match e {
            MediaError::InputUnavailable { reason } => {
                assert_eq!(reason, "OpenDML/nested rec LIST is not supported");
            }
            other => panic!("expected input unavailable, got {other:?}"),
        }
    }

    #[test]
    fn short_movi_list_is_malformed() {
        let mut movi = Vec::new();
        push_chunk(&mut movi, b"LIST", b"rec");
        let bytes = avi_with_movi_entries(&movi);
        let e = avi_layout(&bytes).err().unwrap();
        match e {
            MediaError::Malformed { reason } => {
                assert_eq!(reason, "movi LIST shorter than list type");
            }
            other => panic!("expected malformed, got {other:?}"),
        }
    }

    #[test]
    fn non_rec_movi_list_exact_complement_is_ignored_before_packets() {
        for list_type in [b"Rec ", b"recX", b"rec\0"] {
            let mut movi = Vec::new();
            push_list(&mut movi, list_type, b"tail");
            push_chunk(&mut movi, b"zzwb", &[1u8]);
            push_chunk(&mut movi, b"00wb", &[2u8, 3]);
            let bytes = avi_with_movi_entries(&movi);
            let layout = avi_layout(&bytes).unwrap();

            assert_eq!(layout.packets.len(), 1);
            assert_eq!(layout.packets[0].stream_no, 0);
            assert_eq!(layout.packets[0].kind, *b"wb");
            assert_eq!(layout.packets[0].body, &[2u8, 3]);
        }
    }

    #[test]
    fn movi_scan_collection_preserves_packet_order_across_ignores() {
        let mut movi = Vec::new();
        push_chunk(&mut movi, b"02wb", &[2u8]);
        push_chunk(&mut movi, b"JUNK", &[9u8]);
        push_list(&mut movi, b"zzzz", b"tail");
        push_chunk(&mut movi, b"00db", &[0u8, 1]);
        push_chunk(&mut movi, b"a0wb", &[8u8]);
        push_chunk(&mut movi, b"01pc", &[1u8]);
        let bytes = avi_with_movi_entries(&movi);
        let layout = avi_layout(&bytes).unwrap();

        assert_eq!(layout.packets.len(), 3);
        assert_eq!(layout.packets[0].stream_no, 2);
        assert_eq!(layout.packets[0].kind, *b"wb");
        assert_eq!(layout.packets[0].body, &[2u8]);
        assert_eq!(layout.packets[1].stream_no, 0);
        assert_eq!(layout.packets[1].kind, *b"db");
        assert_eq!(layout.packets[1].body, &[0u8, 1]);
        assert_eq!(layout.packets[2].stream_no, 1);
        assert_eq!(layout.packets[2].kind, *b"pc");
        assert_eq!(layout.packets[2].body, &[1u8]);
    }

    #[test]
    fn movi_first_reject_determines_error_and_stops_scan() {
        let mut movi = Vec::new();
        push_chunk(&mut movi, b"00wb", &[1u8]);
        push_chunk(&mut movi, b"LIST", b"rec");
        push_list(&mut movi, b"rec ", &[]);
        push_chunk(&mut movi, b"01wb", &[2u8]);
        let bytes = avi_with_movi_entries(&movi);
        let e = avi_layout(&bytes).err().unwrap();
        match e {
            MediaError::Malformed { reason } => {
                assert_eq!(reason, "movi LIST shorter than list type");
            }
            other => panic!("expected malformed short LIST, got {other:?}"),
        }

        let mut movi = Vec::new();
        push_chunk(&mut movi, b"00wb", &[1u8]);
        push_list(&mut movi, b"rec ", &[]);
        push_chunk(&mut movi, b"LIST", b"rec");
        push_chunk(&mut movi, b"01wb", &[2u8]);
        let bytes = avi_with_movi_entries(&movi);
        let e = avi_layout(&bytes).err().unwrap();
        match e {
            MediaError::InputUnavailable { reason } => {
                assert_eq!(reason, "OpenDML/nested rec LIST is not supported");
            }
            other => panic!("expected input unavailable rec LIST, got {other:?}"),
        }
    }

    #[test]
    fn movi_entry_decision_non_list_packet_ignores_list_type_view() {
        for list_type_view in [None, Some(*b"rec "), Some(*b"zzzz")] {
            assert_eq!(
                avi_movi_entry_decision(*b"99pc", list_type_view),
                MoviEntryDecision::Packet(MoviPacketTag {
                    stream_no: 99,
                    kind: *b"pc",
                })
            );
            assert_eq!(
                avi_movi_entry_decision(*b"a0wb", list_type_view),
                MoviEntryDecision::Ignore
            );
        }
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

#[cfg(kani)]
mod proofs {
    use super::*;

    #[kani::proof]
    fn avi_chunk_step_exact() {
        let offset: u64 = kani::any();
        let region_len: u64 = kani::any();
        let size: u32 = kani::any();
        let step = avi_chunk_step(offset, region_len, size);

        let end_expected_position = offset == region_len;
        let header_available = offset < region_len && region_len - offset >= 8;
        let trailing_expected = !end_expected_position && !header_available;
        let body_start_expected = offset.checked_add(8);
        let body_end_expected = if header_available {
            body_start_expected.and_then(|start| start.checked_add(u64::from(size)))
        } else {
            None
        };
        let pad = u64::from(size & 1);
        let next_expected = body_end_expected.and_then(|end| end.checked_add(pad));
        let valid_expected = header_available
            && body_end_expected.is_some_and(|end| end <= region_len)
            && next_expected.is_some_and(|next| next <= region_len);
        let invalid_expected = header_available && !valid_expected;

        assert_eq!(matches!(step, AviChunkStep::End), end_expected_position);
        assert_eq!(
            matches!(step, AviChunkStep::TrailingBytes),
            trailing_expected
        );
        assert_eq!(matches!(step, AviChunkStep::Advance(_)), valid_expected);
        assert_eq!(matches!(step, AviChunkStep::Invalid(_)), invalid_expected);

        match step {
            AviChunkStep::Advance(plan) => {
                assert!(header_available);
                assert_eq!(plan.body_start, offset + 8);
                assert_eq!(plan.body_end, plan.body_start + u64::from(size));
                assert_eq!(plan.next_offset, plan.body_end + pad);
                assert!(plan.body_end <= region_len);
                assert!(plan.next_offset <= region_len);
                assert!(plan.next_offset > offset);
                assert!(plan.next_offset - offset >= 8);
                assert!(region_len - plan.next_offset < region_len - offset);
            }
            AviChunkStep::Invalid(error) => {
                assert!(header_available);
                let size_overflow = header_available && body_end_expected.is_none();
                let extends_past_end =
                    header_available && body_end_expected.is_some_and(|end| end > region_len);
                let missing_pad = header_available
                    && match body_end_expected {
                        Some(end) if end <= region_len => match next_expected {
                            Some(next) => next > region_len,
                            None => true,
                        },
                        _ => false,
                    };
                assert_eq!(error == AviChunkBoundaryError::SizeOverflow, size_overflow);
                assert_eq!(
                    error == AviChunkBoundaryError::ExtendsPastEnd,
                    extends_past_end
                );
                assert_eq!(error == AviChunkBoundaryError::MissingPad, missing_pad);
            }
            AviChunkStep::End => {
                assert_eq!(offset, region_len);
            }
            AviChunkStep::TrailingBytes => {
                assert!(offset != region_len);
                assert!(offset > region_len || region_len - offset < 8);
            }
        }
    }

    #[kani::proof]
    fn avi_movi_packet_tag_exact() {
        let id: [u8; 4] = kani::any();
        let tag = avi_movi_packet_tag(id);
        let first_digit = id[0].is_ascii_digit();
        let second_digit = id[1].is_ascii_digit();
        let accepted_expected = first_digit && second_digit;

        assert_eq!(tag.is_some(), accepted_expected);
        assert_eq!(tag.is_none(), !accepted_expected);
        if let Some(tag) = tag {
            let tens = usize::from(id[0].wrapping_sub(b'0'));
            let ones = usize::from(id[1].wrapping_sub(b'0'));
            assert_eq!(tag.stream_no, tens * 10 + ones);
            assert!(tag.stream_no <= 99);
            assert_eq!(tag.kind[0], id[2]);
            assert_eq!(tag.kind[1], id[3]);
        } else {
            assert!(!first_digit || !second_digit);
        }
    }

    #[kani::proof]
    fn avi_movi_entry_decision_exact() {
        let id: [u8; 4] = kani::any();
        let list_type_view: Option<[u8; 4]> = kani::any();
        let decision = avi_movi_entry_decision(id, list_type_view);

        let is_list = id == *b"LIST";
        let rec_view = list_type_view == Some(*b"rec ");
        let short_expected = is_list && list_type_view.is_none();
        let rec_expected = is_list && rec_view;
        let list_ignore_expected = is_list && list_type_view.is_some() && !rec_view;
        let tag = avi_movi_packet_tag(id);
        let packet_expected = !is_list && tag.is_some();
        let non_list_ignore_expected = !is_list && tag.is_none();
        let ignore_expected = list_ignore_expected || non_list_ignore_expected;

        assert_eq!(
            matches!(decision, MoviEntryDecision::RejectShortListType),
            short_expected
        );
        assert_eq!(
            matches!(decision, MoviEntryDecision::RejectRecList),
            rec_expected
        );
        assert_eq!(
            matches!(decision, MoviEntryDecision::Packet(_)),
            packet_expected
        );
        assert_eq!(
            matches!(decision, MoviEntryDecision::Ignore),
            ignore_expected
        );
        assert_eq!(
            short_expected as u8
                + rec_expected as u8
                + packet_expected as u8
                + ignore_expected as u8,
            1
        );

        if is_list {
            assert!(!matches!(decision, MoviEntryDecision::Packet(_)));
            if let Some(list_type) = list_type_view {
                assert_eq!(
                    matches!(decision, MoviEntryDecision::RejectRecList),
                    list_type == *b"rec "
                );
                assert_eq!(
                    matches!(decision, MoviEntryDecision::Ignore),
                    list_type != *b"rec "
                );
            }
        }

        if let MoviEntryDecision::Packet(packet_tag) = decision {
            assert!(!is_list);
            assert!(id[0].is_ascii_digit());
            assert!(id[1].is_ascii_digit());
            let tens = usize::from(id[0].wrapping_sub(b'0'));
            let ones = usize::from(id[1].wrapping_sub(b'0'));
            assert_eq!(packet_tag.stream_no, tens * 10 + ones);
            assert!(packet_tag.stream_no <= 99);
            assert_eq!(packet_tag.kind[0], id[2]);
            assert_eq!(packet_tag.kind[1], id[3]);
        }
    }

    type SymbolicMoviEntry = ([u8; 4], Option<[u8; 4]>);
    const SCAN_ENTRY_COUNT: usize = 3;
    const EMPTY_MOVI_TAG: MoviPacketTag = MoviPacketTag {
        stream_no: 0,
        kind: [0, 0],
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct MoviScanCollectionOutcome {
        scanned_entries: u8,
        packet_count: u8,
        packet_tags: [MoviPacketTag; SCAN_ENTRY_COUNT],
        stop: Option<MoviScanReject>,
    }

    fn empty_scan_collection_outcome() -> MoviScanCollectionOutcome {
        MoviScanCollectionOutcome {
            scanned_entries: 0,
            packet_count: 0,
            packet_tags: [EMPTY_MOVI_TAG; SCAN_ENTRY_COUNT],
            stop: None,
        }
    }

    fn store_scan_packet(outcome: &mut MoviScanCollectionOutcome, tag: MoviPacketTag) {
        match outcome.packet_count {
            0 => outcome.packet_tags[0] = tag,
            1 => outcome.packet_tags[1] = tag,
            2 => outcome.packet_tags[2] = tag,
            _ => unreachable!("three scan entries cannot produce more than three packets"),
        }
        outcome.packet_count += 1;
    }

    fn apply_production_scan_entry(
        outcome: &mut MoviScanCollectionOutcome,
        entry: SymbolicMoviEntry,
    ) {
        outcome.scanned_entries += 1;
        let decision = avi_movi_entry_decision(entry.0, entry.1);
        match avi_movi_scan_step(decision) {
            MoviScanStep::Collect(tag) => store_scan_packet(outcome, tag),
            MoviScanStep::Skip => {}
            MoviScanStep::Stop(reject) => outcome.stop = Some(reject),
        }
    }

    fn production_scan_collection_model(
        entries: [SymbolicMoviEntry; SCAN_ENTRY_COUNT],
    ) -> MoviScanCollectionOutcome {
        let mut outcome = empty_scan_collection_outcome();
        apply_production_scan_entry(&mut outcome, entries[0]);
        if outcome.stop.is_some() {
            return outcome;
        }
        apply_production_scan_entry(&mut outcome, entries[1]);
        if outcome.stop.is_some() {
            return outcome;
        }
        apply_production_scan_entry(&mut outcome, entries[2]);
        outcome
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ExpectedMoviScanEntry {
        Collect(MoviPacketTag),
        Skip,
        Stop(MoviScanReject),
    }

    fn expected_movi_scan_entry(
        id: [u8; 4],
        list_type_view: Option<[u8; 4]>,
    ) -> ExpectedMoviScanEntry {
        if id == *b"LIST" {
            return match list_type_view {
                None => ExpectedMoviScanEntry::Stop(MoviScanReject::ShortListType),
                Some(list_type) if list_type == *b"rec " => {
                    ExpectedMoviScanEntry::Stop(MoviScanReject::RecList)
                }
                Some(_) => ExpectedMoviScanEntry::Skip,
            };
        }
        if id[0].is_ascii_digit() && id[1].is_ascii_digit() {
            let stream_no =
                usize::from(id[0].wrapping_sub(b'0')) * 10 + usize::from(id[1].wrapping_sub(b'0'));
            return ExpectedMoviScanEntry::Collect(MoviPacketTag {
                stream_no,
                kind: [id[2], id[3]],
            });
        }
        ExpectedMoviScanEntry::Skip
    }

    fn apply_expected_scan_entry(
        outcome: &mut MoviScanCollectionOutcome,
        entry: ExpectedMoviScanEntry,
    ) {
        outcome.scanned_entries += 1;
        match entry {
            ExpectedMoviScanEntry::Collect(tag) => store_scan_packet(outcome, tag),
            ExpectedMoviScanEntry::Skip => {}
            ExpectedMoviScanEntry::Stop(reject) => outcome.stop = Some(reject),
        }
    }

    fn expected_scan_collection(
        entries: [ExpectedMoviScanEntry; SCAN_ENTRY_COUNT],
    ) -> MoviScanCollectionOutcome {
        let mut outcome = empty_scan_collection_outcome();
        apply_expected_scan_entry(&mut outcome, entries[0]);
        if outcome.stop.is_some() {
            return outcome;
        }
        apply_expected_scan_entry(&mut outcome, entries[1]);
        if outcome.stop.is_some() {
            return outcome;
        }
        apply_expected_scan_entry(&mut outcome, entries[2]);
        outcome
    }

    fn decision_matches_expected_entry(
        decision: MoviEntryDecision,
        expected: ExpectedMoviScanEntry,
    ) -> bool {
        match (decision, expected) {
            (MoviEntryDecision::Packet(actual), ExpectedMoviScanEntry::Collect(expected)) => {
                actual == expected
            }
            (MoviEntryDecision::Ignore, ExpectedMoviScanEntry::Skip) => true,
            (
                MoviEntryDecision::RejectShortListType,
                ExpectedMoviScanEntry::Stop(MoviScanReject::ShortListType),
            ) => true,
            (
                MoviEntryDecision::RejectRecList,
                ExpectedMoviScanEntry::Stop(MoviScanReject::RecList),
            ) => true,
            _ => false,
        }
    }

    #[kani::proof]
    fn avi_movi_scan_collection_bounded_exact() {
        let entries: [SymbolicMoviEntry; SCAN_ENTRY_COUNT] = [
            (kani::any(), kani::any()),
            (kani::any(), kani::any()),
            (kani::any(), kani::any()),
        ];
        let production_decisions = [
            avi_movi_entry_decision(entries[0].0, entries[0].1),
            avi_movi_entry_decision(entries[1].0, entries[1].1),
            avi_movi_entry_decision(entries[2].0, entries[2].1),
        ];
        let expected_entries = [
            expected_movi_scan_entry(entries[0].0, entries[0].1),
            expected_movi_scan_entry(entries[1].0, entries[1].1),
            expected_movi_scan_entry(entries[2].0, entries[2].1),
        ];
        assert!(decision_matches_expected_entry(
            production_decisions[0],
            expected_entries[0]
        ));
        assert!(decision_matches_expected_entry(
            production_decisions[1],
            expected_entries[1]
        ));
        assert!(decision_matches_expected_entry(
            production_decisions[2],
            expected_entries[2]
        ));

        let actual = production_scan_collection_model(entries);
        let expected = expected_scan_collection(expected_entries);
        assert_eq!(actual.scanned_entries, expected.scanned_entries);
        assert_eq!(actual.packet_count, expected.packet_count);
        assert_eq!(actual.packet_tags, expected.packet_tags);
        assert_eq!(actual.stop, expected.stop);
    }

    #[kani::proof]
    fn avi_strh_declaration_bounded_exact() {
        let body_len: usize = kani::any();
        let prefix: [u8; STRH_HEADER_LEN] = kani::any();

        let accepted = avi_strh_accepts_body_len(body_len);
        assert_eq!(accepted, body_len >= STRH_HEADER_LEN);
        assert_eq!(!accepted, body_len < STRH_HEADER_LEN);

        let fields = avi_strh_declaration_fields(&prefix);
        assert_eq!(
            fields.fcc_type,
            [prefix[0], prefix[1], prefix[2], prefix[3]]
        );
        assert_eq!(
            fields.scale,
            u32::from_le_bytes([prefix[20], prefix[21], prefix[22], prefix[23]])
        );
        assert_eq!(
            fields.rate,
            u32::from_le_bytes([prefix[24], prefix[25], prefix[26], prefix[27]])
        );
        assert_eq!(
            fields.start,
            u32::from_le_bytes([prefix[28], prefix[29], prefix[30], prefix[31]])
        );
        assert_eq!(
            fields.length,
            u32::from_le_bytes([prefix[32], prefix[33], prefix[34], prefix[35]])
        );
        assert_eq!(
            fields.sample_size,
            u32::from_le_bytes([prefix[44], prefix[45], prefix[46], prefix[47]])
        );
    }

    const STREAM_TYPE_COUNT: usize = 3;
    type SymbolicStreamTypeTable = [[u8; 4]; STREAM_TYPE_COUNT];

    fn expected_stream_type_class(fcc_type: [u8; 4]) -> AviStreamTypeClass {
        if fcc_type == *b"auds" {
            AviStreamTypeClass::Audio
        } else if fcc_type == *b"vids" {
            AviStreamTypeClass::Video
        } else {
            AviStreamTypeClass::Other
        }
    }

    fn assert_stream_type_class_exact(fcc_type: [u8; 4]) {
        let actual = avi_stream_type_class(fcc_type);
        let expected = expected_stream_type_class(fcc_type);
        assert_eq!(actual, expected);
        assert_eq!(actual == AviStreamTypeClass::Audio, fcc_type == *b"auds");
        assert_eq!(actual == AviStreamTypeClass::Video, fcc_type == *b"vids");
        assert_eq!(
            actual == AviStreamTypeClass::Other,
            fcc_type != *b"auds" && fcc_type != *b"vids"
        );
        assert_eq!(
            (actual == AviStreamTypeClass::Audio) as u8
                + (actual == AviStreamTypeClass::Video) as u8
                + (actual == AviStreamTypeClass::Other) as u8,
            1
        );
    }

    fn expected_first_stream_of_class(
        stream_types: &SymbolicStreamTypeTable,
        class: AviStreamTypeClass,
    ) -> Option<usize> {
        if expected_stream_type_class(stream_types[0]) == class {
            Some(0)
        } else if expected_stream_type_class(stream_types[1]) == class {
            Some(1)
        } else if expected_stream_type_class(stream_types[2]) == class {
            Some(2)
        } else {
            None
        }
    }

    fn assert_first_stream_selection(
        stream_types: &SymbolicStreamTypeTable,
        class: AviStreamTypeClass,
    ) {
        let actual = avi_first_stream_of_class(&stream_types[..], class);
        let expected = expected_first_stream_of_class(stream_types, class);
        assert_eq!(actual, expected);
        let classes = [
            expected_stream_type_class(stream_types[0]),
            expected_stream_type_class(stream_types[1]),
            expected_stream_type_class(stream_types[2]),
        ];
        match actual {
            Some(0) => assert_eq!(classes[0], class),
            Some(1) => {
                assert_ne!(classes[0], class);
                assert_eq!(classes[1], class);
            }
            Some(2) => {
                assert_ne!(classes[0], class);
                assert_ne!(classes[1], class);
                assert_eq!(classes[2], class);
            }
            None => {
                assert_ne!(classes[0], class);
                assert_ne!(classes[1], class);
                assert_ne!(classes[2], class);
            }
            Some(_) => unreachable!("fixed three-stream table selection is in range or absent"),
        }
    }

    fn expected_packet_kind_role(kind: [u8; 2]) -> AviPacketKindRole {
        if kind == *b"wb" {
            AviPacketKindRole::AudioPayload
        } else if kind == *b"db" || kind == *b"dc" {
            AviPacketKindRole::VideoFrame
        } else {
            AviPacketKindRole::NonPayload
        }
    }

    fn expected_packet_stream_meaning(
        stream_types: &SymbolicStreamTypeTable,
        stream_no: usize,
        kind: [u8; 2],
    ) -> AviPacketStreamMeaning {
        if stream_no >= STREAM_TYPE_COUNT {
            return AviPacketStreamMeaning::NoTimelineSlot;
        }
        match expected_stream_type_class(stream_types[stream_no]) {
            AviStreamTypeClass::Audio => {
                if kind == *b"wb" {
                    AviPacketStreamMeaning::AudioPayload
                } else {
                    AviPacketStreamMeaning::AudioTimelineOnly
                }
            }
            AviStreamTypeClass::Video => {
                if kind == *b"db" || kind == *b"dc" {
                    AviPacketStreamMeaning::VideoFrame
                } else {
                    AviPacketStreamMeaning::VideoTimelineOnly
                }
            }
            AviStreamTypeClass::Other => AviPacketStreamMeaning::NoTimelineSlot,
        }
    }

    #[kani::proof]
    fn avi_stream_kind_classification_bounded_exact() {
        let stream_types: SymbolicStreamTypeTable = [kani::any(), kani::any(), kani::any()];
        let stream_no: usize = kani::any();
        let kind: [u8; 2] = kani::any();

        assert_stream_type_class_exact(stream_types[0]);
        assert_stream_type_class_exact(stream_types[1]);
        assert_stream_type_class_exact(stream_types[2]);

        let in_range = stream_no < STREAM_TYPE_COUNT;
        let actual_type = avi_stream_type_at(&stream_types[..], stream_no);
        let actual_class_at = avi_stream_type_class_at(&stream_types[..], stream_no);
        assert_eq!(actual_type.is_some(), in_range);
        assert_eq!(actual_class_at.is_some(), in_range);
        let expected_class_at = if in_range {
            let expected_type = stream_types[stream_no];
            assert_eq!(actual_type, Some(expected_type));
            let expected_class = expected_stream_type_class(expected_type);
            assert_eq!(actual_class_at, Some(expected_class));
            Some(expected_class)
        } else {
            assert_eq!(actual_type, None);
            assert_eq!(actual_class_at, None);
            None
        };

        assert_first_stream_selection(&stream_types, AviStreamTypeClass::Audio);
        assert_first_stream_selection(&stream_types, AviStreamTypeClass::Video);

        let actual_kind_role = avi_packet_kind_role(kind);
        let expected_kind_role = expected_packet_kind_role(kind);
        assert_eq!(actual_kind_role, expected_kind_role);
        assert_eq!(
            actual_kind_role == AviPacketKindRole::AudioPayload,
            kind == *b"wb"
        );
        assert_eq!(
            actual_kind_role == AviPacketKindRole::VideoFrame,
            kind == *b"db" || kind == *b"dc"
        );
        assert_eq!(
            actual_kind_role == AviPacketKindRole::NonPayload,
            kind != *b"wb" && kind != *b"db" && kind != *b"dc"
        );
        assert_eq!(
            (actual_kind_role == AviPacketKindRole::AudioPayload) as u8
                + (actual_kind_role == AviPacketKindRole::VideoFrame) as u8
                + (actual_kind_role == AviPacketKindRole::NonPayload) as u8,
            1
        );

        let actual_meaning = avi_packet_stream_meaning(&stream_types[..], stream_no, kind);
        let expected_meaning = expected_packet_stream_meaning(&stream_types, stream_no, kind);
        assert_eq!(actual_meaning, expected_meaning);
        assert_eq!(
            actual_meaning == AviPacketStreamMeaning::AudioPayload,
            expected_class_at == Some(AviStreamTypeClass::Audio) && kind == *b"wb"
        );
        assert_eq!(
            actual_meaning == AviPacketStreamMeaning::AudioTimelineOnly,
            expected_class_at == Some(AviStreamTypeClass::Audio) && kind != *b"wb"
        );
        assert_eq!(
            actual_meaning == AviPacketStreamMeaning::VideoFrame,
            expected_class_at == Some(AviStreamTypeClass::Video)
                && (kind == *b"db" || kind == *b"dc")
        );
        assert_eq!(
            actual_meaning == AviPacketStreamMeaning::VideoTimelineOnly,
            expected_class_at == Some(AviStreamTypeClass::Video)
                && kind != *b"db"
                && kind != *b"dc"
        );
        assert_eq!(
            actual_meaning == AviPacketStreamMeaning::NoTimelineSlot,
            expected_class_at.is_none() || expected_class_at == Some(AviStreamTypeClass::Other)
        );
        assert_eq!(
            (actual_meaning == AviPacketStreamMeaning::AudioPayload) as u8
                + (actual_meaning == AviPacketStreamMeaning::AudioTimelineOnly) as u8
                + (actual_meaning == AviPacketStreamMeaning::VideoFrame) as u8
                + (actual_meaning == AviPacketStreamMeaning::VideoTimelineOnly) as u8
                + (actual_meaning == AviPacketStreamMeaning::NoTimelineSlot) as u8,
            1
        );

        let actual_owner = avi_packet_owns_timeline_slot(&stream_types[..], stream_no);
        assert_eq!(
            actual_owner,
            expected_meaning != AviPacketStreamMeaning::NoTimelineSlot
        );

        let first_audio = avi_first_stream_of_class(&stream_types[..], AviStreamTypeClass::Audio);
        if let Some(audio_idx) = first_audio {
            let actual_selected =
                avi_packet_is_selected_audio_payload(&stream_types[..], stream_no, kind, audio_idx);
            assert_eq!(
                actual_selected,
                stream_no == audio_idx && expected_meaning == AviPacketStreamMeaning::AudioPayload
            );
        } else {
            assert_ne!(expected_meaning, AviPacketStreamMeaning::AudioPayload);
        }

        let first_video = avi_first_stream_of_class(&stream_types[..], AviStreamTypeClass::Video);
        if let Some(video_idx) = first_video {
            let actual_selected =
                avi_packet_is_selected_video_frame(&stream_types[..], stream_no, kind, video_idx);
            assert_eq!(
                actual_selected,
                stream_no == video_idx && expected_meaning == AviPacketStreamMeaning::VideoFrame
            );
        } else {
            assert_ne!(expected_meaning, AviPacketStreamMeaning::VideoFrame);
        }
    }
}

#[cfg(test)]
mod boundary_tests {
    use super::tests::strh_body;
    use super::*;

    fn classification_stream_head(fcc_type: [u8; 4], scale: u32) -> StreamHead {
        StreamHead {
            fcc_type,
            scale,
            rate: 1,
            start: 0,
            length: 0,
            sample_size: 0,
            format: Vec::new(),
        }
    }

    #[test]
    fn stream_kind_classification_covers_lookup_selection_payload_and_ownership() {
        let types = [*b"xxxx", *b"auds", *b"vids"];
        assert_eq!(avi_stream_type_at(&types, 0), Some(*b"xxxx"));
        assert_eq!(avi_stream_type_at(&types, 1), Some(*b"auds"));
        assert_eq!(avi_stream_type_at(&types, 2), Some(*b"vids"));
        assert_eq!(avi_stream_type_at(&types, 3), None);
        assert_eq!(avi_stream_type_at(&types, usize::MAX), None);
        assert_eq!(
            avi_first_stream_of_class(&types, AviStreamTypeClass::Audio),
            Some(1)
        );
        assert_eq!(
            avi_first_stream_of_class(&types, AviStreamTypeClass::Video),
            Some(2)
        );
        assert_eq!(
            avi_first_stream_of_class(&types, AviStreamTypeClass::Other),
            Some(0)
        );

        assert_eq!(
            avi_packet_stream_meaning(&types, 1, *b"wb"),
            AviPacketStreamMeaning::AudioPayload
        );
        assert_eq!(
            avi_packet_stream_meaning(&types, 1, *b"zz"),
            AviPacketStreamMeaning::AudioTimelineOnly
        );
        assert_eq!(
            avi_packet_stream_meaning(&types, 2, *b"db"),
            AviPacketStreamMeaning::VideoFrame
        );
        assert_eq!(
            avi_packet_stream_meaning(&types, 2, *b"dc"),
            AviPacketStreamMeaning::VideoFrame
        );
        assert_eq!(
            avi_packet_stream_meaning(&types, 2, *b"pc"),
            AviPacketStreamMeaning::VideoTimelineOnly
        );
        assert_eq!(
            avi_packet_stream_meaning(&types, 0, *b"wb"),
            AviPacketStreamMeaning::NoTimelineSlot
        );
        assert_eq!(
            avi_packet_stream_meaning(&types, 3, *b"wb"),
            AviPacketStreamMeaning::NoTimelineSlot
        );
        assert!(avi_packet_owns_timeline_slot(&types, 1));
        assert!(avi_packet_owns_timeline_slot(&types, 2));
        assert!(!avi_packet_owns_timeline_slot(&types, 0));
        assert!(!avi_packet_owns_timeline_slot(&types, 3));

        assert!(avi_packet_is_selected_audio_payload(&types, 1, *b"wb", 1));
        assert!(!avi_packet_is_selected_audio_payload(&types, 2, *b"wb", 1));
        assert!(!avi_packet_is_selected_audio_payload(&types, 1, *b"pc", 1));
        assert!(avi_packet_is_selected_video_frame(&types, 2, *b"db", 2));
        assert!(avi_packet_is_selected_video_frame(&types, 2, *b"dc", 2));
        assert!(!avi_packet_is_selected_video_frame(&types, 2, *b"pc", 2));
        assert!(!avi_packet_is_selected_video_frame(&types, 1, *b"db", 2));
    }

    #[test]
    fn strh_declaration_rejects_short_bodies() {
        for len in [0usize, 1, STRH_HEADER_LEN - 1] {
            let body = vec![0xaa; len];
            let mut head = classification_stream_head([0; 4], 0);
            let err = parse_strh(&body, &mut head).unwrap_err();
            match err {
                MediaError::Malformed { reason } => {
                    assert_eq!(reason, "strh chunk shorter than 48 bytes");
                }
                _ => panic!("short strh body must be malformed"),
            }
        }
    }

    #[test]
    fn strh_declaration_extracts_le_fields_and_ignores_unobserved_and_trailing_bytes() {
        let fcc = [0x74, 0x00, 0xff, 0x73];
        let mut body = strh_body(
            &fcc,
            0x0102_0304,
            0x0506_0708,
            0x090a_0b0c,
            0x0d0e_0f10,
            0x1112_1314,
        );
        for (index, byte) in body.iter_mut().enumerate() {
            if (4..20).contains(&index) || (36..44).contains(&index) || index >= STRH_HEADER_LEN {
                *byte = 0xa5u8.wrapping_add(index as u8);
            }
        }

        let mut head = classification_stream_head([0; 4], 0);
        parse_strh(&body, &mut head).unwrap();
        assert_eq!(head.fcc_type, fcc);
        assert_eq!(head.scale, 0x0102_0304);
        assert_eq!(head.rate, 0x0506_0708);
        assert_eq!(head.start, 0x090a_0b0c);
        assert_eq!(head.length, 0x0d0e_0f10);
        assert_eq!(head.sample_size, 0x1112_1314);

        let mut exact_head = classification_stream_head([0; 4], 0);
        parse_strh(&body[..STRH_HEADER_LEN], &mut exact_head).unwrap();
        assert_eq!(exact_head.fcc_type, head.fcc_type);
        assert_eq!(exact_head.scale, head.scale);
        assert_eq!(exact_head.rate, head.rate);
        assert_eq!(exact_head.start, head.start);
        assert_eq!(exact_head.length, head.length);
        assert_eq!(exact_head.sample_size, head.sample_size);
    }

    #[test]
    fn packet_ticks_track_existing_media_streams_and_skip_missing_or_other_streams() {
        let streams = vec![
            classification_stream_head(*b"auds", 2),
            classification_stream_head(*b"xxxx", 7),
            classification_stream_head(*b"vids", 3),
        ];
        let bodies = [[1u8], [2u8], [3u8], [4u8], [5u8]];
        let packets = [
            MoviPacket {
                stream_no: 0,
                kind: *b"zz",
                body: &bodies[0],
            },
            MoviPacket {
                stream_no: 1,
                kind: *b"wb",
                body: &bodies[1],
            },
            MoviPacket {
                stream_no: 2,
                kind: *b"pc",
                body: &bodies[2],
            },
            MoviPacket {
                stream_no: 3,
                kind: *b"wb",
                body: &bodies[3],
            },
            MoviPacket {
                stream_no: 0,
                kind: *b"db",
                body: &bodies[4],
            },
        ];

        let ticks = packet_ticks(&streams, &packets).unwrap();
        assert_eq!(ticks, vec![Some(0), None, Some(0), None, Some(2)]);
    }

    #[test]
    fn avi_movi_packet_tag_preserves_decimal_stream_and_kind_order() {
        assert_eq!(
            avi_movi_packet_tag(*b"00wb"),
            Some(MoviPacketTag {
                stream_no: 0,
                kind: *b"wb",
            })
        );
        assert_eq!(
            avi_movi_packet_tag(*b"99dc"),
            Some(MoviPacketTag {
                stream_no: 99,
                kind: *b"dc",
            })
        );
        assert_eq!(
            avi_movi_packet_tag(*b"09pc"),
            Some(MoviPacketTag {
                stream_no: 9,
                kind: *b"pc",
            })
        );
        assert_eq!(
            avi_movi_packet_tag(*b"42az"),
            Some(MoviPacketTag {
                stream_no: 42,
                kind: *b"az",
            })
        );
        assert_eq!(
            avi_movi_packet_tag(*b"42za"),
            Some(MoviPacketTag {
                stream_no: 42,
                kind: *b"za",
            })
        );
        assert_eq!(
            avi_movi_packet_tag([b'4', b'2', 0xff, b'z']),
            Some(MoviPacketTag {
                stream_no: 42,
                kind: [0xff, b'z'],
            })
        );
    }

    #[test]
    fn avi_movi_packet_tag_rejects_non_digit_prefix_regardless_of_kind() {
        assert_eq!(avi_movi_packet_tag(*b"a0wb"), None);
        assert_eq!(avi_movi_packet_tag(*b"0awb"), None);
        assert_eq!(avi_movi_packet_tag(*b"/0db"), None);
        assert_eq!(avi_movi_packet_tag(*b"0:dc"), None);
        assert_eq!(avi_movi_packet_tag(*b"a0pc"), None);
        assert_eq!(avi_movi_packet_tag(*b"0azz"), None);
    }

    #[test]
    fn avi_chunk_step_accepts_zero_even_and_odd_sizes_exact_end() {
        assert_eq!(
            avi_chunk_step(0, 8, 0),
            AviChunkStep::Advance(AviChunkAdvance {
                body_start: 8,
                body_end: 8,
                next_offset: 8,
            })
        );
        assert_eq!(
            avi_chunk_step(0, 12, 4),
            AviChunkStep::Advance(AviChunkAdvance {
                body_start: 8,
                body_end: 12,
                next_offset: 12,
            })
        );
        assert_eq!(
            avi_chunk_step(0, 12, 3),
            AviChunkStep::Advance(AviChunkAdvance {
                body_start: 8,
                body_end: 11,
                next_offset: 12,
            })
        );
    }

    #[test]
    fn avi_chunk_step_classifies_terminal_invalid_and_large_offsets() {
        assert_eq!(avi_chunk_step(0, 0, 0), AviChunkStep::End);
        assert_eq!(avi_chunk_step(12, 12, 0), AviChunkStep::End);
        assert_eq!(avi_chunk_step(5, 12, 0), AviChunkStep::TrailingBytes);
        assert_eq!(avi_chunk_step(13, 12, 0), AviChunkStep::TrailingBytes);
        assert_eq!(
            avi_chunk_step(0, 12, 5),
            AviChunkStep::Invalid(AviChunkBoundaryError::ExtendsPastEnd)
        );
        assert_eq!(
            avi_chunk_step(0, 11, 3),
            AviChunkStep::Invalid(AviChunkBoundaryError::MissingPad)
        );
        assert_eq!(
            avi_chunk_step(u64::MAX - 8, u64::MAX, 1),
            AviChunkStep::Invalid(AviChunkBoundaryError::SizeOverflow)
        );
    }

    #[test]
    fn chunks_iterator_applies_pad_and_nested_region_offsets() {
        let mut inner = Vec::new();
        inner.extend_from_slice(b"JUNK");
        inner.extend_from_slice(&3u32.to_le_bytes());
        inner.extend_from_slice(b"abc");
        inner.push(0);
        inner.extend_from_slice(b"FREE");
        inner.extend_from_slice(&0u32.to_le_bytes());

        // Nested LIST iteration starts after the four-byte form type.
        let mut nested_region = Vec::new();
        nested_region.extend_from_slice(b"strl");
        nested_region.extend_from_slice(&inner);
        let mut iter = chunks(&nested_region[4..], "nested");
        let (id, body) = iter.next().unwrap().unwrap();
        assert_eq!(&id, b"JUNK");
        assert_eq!(body, b"abc");
        let (id, body) = iter.next().unwrap().unwrap();
        assert_eq!(&id, b"FREE");
        assert!(body.is_empty());
        assert!(iter.next().is_none());
    }
}
