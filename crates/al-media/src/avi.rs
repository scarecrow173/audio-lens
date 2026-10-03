//! RIFF/AVI demuxing: PCM audio extraction plus video stream metadata.
//!
//! Scope: legacy AVI (`RIFF 'AVI '`). The audio stream must carry PCM
//! or IEEE-float data (its `strf` chunk is a WAVEFORMATEX, the same
//! layout as a WAV `fmt ` chunk); other audio codecs fail explicitly
//! as `decode/input unavailable`. The video stream is never decoded —
//! only its declared facts (codec fourcc, dimensions, nominal frame
//! rate, frame count) are recorded, matching the ingest spec's
//! container-level observation role.
//!
//! Timing: legacy AVI carries no per-packet PTS. Each `movi` packet's
//! position on its stream clock is derived from the mux order — the
//! cumulative `strh` units consumed by earlier packets of the same
//! stream — and recorded as a CFR-free [`StreamTiming`] observation
//! (audio packet timestamps and video frame positions alike). The mux
//! order is the primary fact; nothing is reconstructed from a nominal
//! frame rate.

use crate::metadata::MediaError;
use crate::timing::{ClockKind, TimingObservation, observe_timing};
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

/// CFR-free timing observation for one declared stream, derived from
/// `movi` packet order and the stream's `strh` clock.
///
/// Timestamps are in `strh`-rate ticks (one tick = `1/dwRate` seconds):
/// video frame chunks advance the clock by `dwScale` each, `dwSampleSize`
/// streams (e.g. PCM audio) advance by `len*dwScale/dwSampleSize`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StreamTiming {
    /// Stream index in `hdrl` order (the `NN` prefix of its `movi` chunks).
    pub stream: u32,
    /// Stream kind fourcc as text (`vids`, `auds`, ...).
    pub stream_type: String,
    /// Observation over the packet timeline; see `crate::timing`.
    pub observation: TimingObservation,
}

/// Demuxed AVI: decoded audio plus declared video metadata.
#[derive(Debug, Clone)]
pub struct AviMedia {
    /// PCM decoded from the first `auds` stream.
    pub audio: DecodedAudio,
    /// First `vids` stream's declared metadata, if present.
    pub video: Option<VideoInfo>,
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

fn fourcc(codec: u32) -> String {
    let b = codec.to_le_bytes();
    if codec == 0 {
        return "rawvideo".into();
    }
    String::from_utf8_lossy(&b)
        .trim_end_matches(['\0', ' '])
        .to_string()
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
        codec: fourcc(read_u32(bih, 16)?),
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
fn tick_advance(head: &StreamHead, body_len: usize) -> i64 {
    if head.sample_size > 0 {
        body_len as i64 * i64::from(head.scale) / i64::from(head.sample_size)
    } else {
        i64::from(head.scale)
    }
}

/// Build per-stream packet timing observations from `movi` mux order.
///
/// A packet's position is the cumulative stream time consumed by that
/// stream's earlier packets, offset by `strh` `dwStart`. Observation is
/// delegated to `crate::timing` — clocks stay separate and nothing is
/// resampled to a nominal rate.
fn stream_timings(streams: &[StreamHead], packets: &[MoviPacket<'_>]) -> Vec<StreamTiming> {
    let mut next: Vec<i64> = streams
        .iter()
        .map(|s| i64::from(s.start) * i64::from(s.scale))
        .collect();
    let mut pts: Vec<Vec<i64>> = streams.iter().map(|_| Vec::new()).collect();
    for p in packets {
        let Some(head) = streams.get(p.stream_no) else {
            continue;
        };
        if !matches!(&head.fcc_type, b"vids" | b"auds") {
            continue;
        }
        pts[p.stream_no].push(next[p.stream_no]);
        next[p.stream_no] += tick_advance(head, p.body.len());
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
            Some(StreamTiming {
                stream: i as u32,
                stream_type: String::from_utf8_lossy(&s.fcc_type).into_owned(),
                observation: observe_timing(clock, u64::from(s.rate), &pts[i], nominal),
            })
        })
        .collect()
}

/// Demux an AVI file: decode the first PCM audio stream and record the
/// first video stream's declared metadata plus per-stream packet timing.
pub fn decode_avi(data: &[u8]) -> Result<AviMedia, MediaError> {
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

    // Only the first `auds` stream is decoded; its `NNwb` packets are
    // concatenated in mux order so interleaved packets of other streams
    // never leak into the PCM.
    let audio_idx = streams
        .iter()
        .position(|s| &s.fcc_type == b"auds")
        .ok_or_else(|| MediaError::input_unavailable("AVI has no audio stream"))?;
    let mut audio_pcm: Vec<u8> = Vec::new();
    for p in &movi_packets {
        if p.stream_no == audio_idx && &p.kind == b"wb" {
            audio_pcm.extend_from_slice(p.body);
        }
    }
    // A structurally valid AVI whose audio codec is not PCM/IEEE-float
    // stays `decode/input unavailable`; genuine corruption stays
    // `Malformed`. Only the message gains AVI context.
    let audio =
        decode_waveformatex(&streams[audio_idx].format, &audio_pcm).map_err(|e| match e {
            MediaError::InputUnavailable { reason } => {
                MediaError::input_unavailable(format!("AVI audio stream: {reason}"))
            }
            MediaError::Malformed { reason } => {
                MediaError::malformed(format!("AVI audio stream: {reason}"))
            }
            other => other,
        })?;

    let video = streams
        .iter()
        .position(|s| &s.fcc_type == b"vids")
        .map(|vidx| {
            let counted = movi_packets
                .iter()
                .filter(|p| p.stream_no == vidx && matches!(&p.kind, b"db" | b"dc"))
                .count() as u64;
            video_info(&streams[vidx], counted)
        })
        .transpose()?;

    let timing = stream_timings(&streams, &movi_packets);

    Ok(AviMedia {
        audio,
        video,
        timing,
    })
}

/// Specification for [`write_avi`]: a CFR uncompressed-video + PCM-audio
/// AVI suitable for fixtures and samples.
pub struct AviSpec {
    /// Frame width in pixels (must satisfy `width * 3 % 4 == 0` for
    /// unpadded BGR24 rows).
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Frames per second (integer CFR).
    pub fps: u32,
    /// BGR24 frames (bottom-up rows, `width*height*3` bytes each).
    pub frames: Vec<Vec<u8>>,
    /// Interleaved PCM s16 audio bytes.
    pub audio: Vec<u8>,
    /// Audio channel count.
    pub channels: u16,
    /// Audio sample rate.
    pub sample_rate: u32,
}

fn push_chunk(out: &mut Vec<u8>, id: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(id);
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    if body.len() % 2 == 1 {
        out.push(0);
    }
}

fn push_list(out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(b"LIST");
    out.extend_from_slice(&((body.len() + 4) as u32).to_le_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(body);
}

/// Serialize an [`AviSpec`] into a legacy AVI file.
pub fn write_avi(spec: &AviSpec) -> Vec<u8> {
    let usec_per_frame = 1_000_000u32 / spec.fps.max(1);
    let block_align = spec.channels * 2;
    let byte_rate = spec.sample_rate * u32::from(block_align);
    let frame_bytes = (spec.width * spec.height * 3) as usize;

    // avih
    let mut avih = Vec::with_capacity(56);
    avih.extend_from_slice(&usec_per_frame.to_le_bytes());
    avih.extend_from_slice(&0u32.to_le_bytes()); // max bytes/sec
    avih.extend_from_slice(&0u32.to_le_bytes()); // padding granularity
    avih.extend_from_slice(&0x10u32.to_le_bytes()); // AVIF_HASINDEX
    avih.extend_from_slice(&(spec.frames.len() as u32).to_le_bytes());
    avih.extend_from_slice(&0u32.to_le_bytes()); // initial frames
    avih.extend_from_slice(&2u32.to_le_bytes()); // streams
    avih.extend_from_slice(&0u32.to_le_bytes()); // suggested buffer
    avih.extend_from_slice(&spec.width.to_le_bytes());
    avih.extend_from_slice(&spec.height.to_le_bytes());
    avih.extend_from_slice(&[0u8; 16]);

    // video strh (fccType 'vids', handler 'DIB ')
    let mut vstrh = Vec::with_capacity(56);
    vstrh.extend_from_slice(b"vids");
    vstrh.extend_from_slice(b"DIB ");
    vstrh.extend_from_slice(&0u32.to_le_bytes()); // flags
    vstrh.extend_from_slice(&0u16.to_le_bytes()); // priority
    vstrh.extend_from_slice(&0u16.to_le_bytes()); // language
    vstrh.extend_from_slice(&0u32.to_le_bytes()); // initial frames
    vstrh.extend_from_slice(&1u32.to_le_bytes()); // scale
    vstrh.extend_from_slice(&spec.fps.to_le_bytes()); // rate
    vstrh.extend_from_slice(&0u32.to_le_bytes()); // start
    vstrh.extend_from_slice(&(spec.frames.len() as u32).to_le_bytes()); // length
    vstrh.extend_from_slice(&(frame_bytes as u32).to_le_bytes()); // suggested buffer
    vstrh.extend_from_slice(&u32::MAX.to_le_bytes()); // quality
    vstrh.extend_from_slice(&0u32.to_le_bytes()); // sample size
    vstrh.extend_from_slice(&0i16.to_le_bytes()); // rcFrame left/top
    vstrh.extend_from_slice(&0i16.to_le_bytes());
    vstrh.extend_from_slice(&(spec.width as i16).to_le_bytes());
    vstrh.extend_from_slice(&(spec.height as i16).to_le_bytes());

    // video strf: BITMAPINFOHEADER
    let mut vstrf = Vec::with_capacity(40);
    vstrf.extend_from_slice(&40u32.to_le_bytes());
    vstrf.extend_from_slice(&(spec.width as i32).to_le_bytes());
    vstrf.extend_from_slice(&(spec.height as i32).to_le_bytes());
    vstrf.extend_from_slice(&1u16.to_le_bytes()); // planes
    vstrf.extend_from_slice(&24u16.to_le_bytes()); // bit count
    vstrf.extend_from_slice(&0u32.to_le_bytes()); // compression = BI_RGB
    vstrf.extend_from_slice(&(frame_bytes as u32).to_le_bytes());
    vstrf.extend_from_slice(&[0u8; 16]); // resolution + palette

    let mut vstrl = Vec::new();
    push_chunk(&mut vstrl, b"strh", &vstrh);
    push_chunk(&mut vstrl, b"strf", &vstrf);

    // audio strh (fccType 'auds')
    let audio_frames = spec.audio.len() as u32 / u32::from(block_align).max(1);
    let mut astrh = Vec::with_capacity(56);
    astrh.extend_from_slice(b"auds");
    astrh.extend_from_slice(&0u32.to_le_bytes()); // handler
    astrh.extend_from_slice(&0u32.to_le_bytes());
    astrh.extend_from_slice(&0u16.to_le_bytes());
    astrh.extend_from_slice(&0u16.to_le_bytes());
    astrh.extend_from_slice(&0u32.to_le_bytes());
    astrh.extend_from_slice(&u32::from(block_align).to_le_bytes()); // scale
    astrh.extend_from_slice(&byte_rate.to_le_bytes()); // rate
    astrh.extend_from_slice(&0u32.to_le_bytes()); // start
    astrh.extend_from_slice(&audio_frames.to_le_bytes()); // length
    astrh.extend_from_slice(&0u32.to_le_bytes());
    astrh.extend_from_slice(&u32::MAX.to_le_bytes());
    astrh.extend_from_slice(&u32::from(block_align).to_le_bytes()); // sample size
    astrh.extend_from_slice(&[0u8; 8]); // rcFrame

    // audio strf: WAVEFORMATEX (PCM s16)
    let mut astrf = Vec::with_capacity(18);
    astrf.extend_from_slice(&1u16.to_le_bytes());
    astrf.extend_from_slice(&spec.channels.to_le_bytes());
    astrf.extend_from_slice(&spec.sample_rate.to_le_bytes());
    astrf.extend_from_slice(&byte_rate.to_le_bytes());
    astrf.extend_from_slice(&block_align.to_le_bytes());
    astrf.extend_from_slice(&16u16.to_le_bytes());
    astrf.extend_from_slice(&0u16.to_le_bytes()); // cbSize

    let mut astrl = Vec::new();
    push_chunk(&mut astrl, b"strh", &astrh);
    push_chunk(&mut astrl, b"strf", &astrf);

    let mut hdrl = Vec::new();
    push_chunk(&mut hdrl, b"avih", &avih);
    push_list(&mut hdrl, b"strl", &vstrl);
    push_list(&mut hdrl, b"strl", &astrl);

    let mut movi = Vec::new();
    for f in &spec.frames {
        push_chunk(&mut movi, b"00db", f);
    }
    push_chunk(&mut movi, b"01wb", &spec.audio);

    let mut body = Vec::new();
    push_list(&mut body, b"hdrl", &hdrl);
    push_list(&mut body, b"movi", &movi);

    let mut out = Vec::with_capacity(body.len() + 12);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(body.len() as u32 + 4).to_le_bytes());
    out.extend_from_slice(b"AVI ");
    out.extend_from_slice(&body);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let frame = vec![7u8; 64 * 48 * 3];
        let audio: Vec<u8> = (0..960i16)
            .flat_map(|v| (v % 200 - 100).to_le_bytes())
            .collect();
        write_avi(&AviSpec {
            width: 64,
            height: 48,
            fps: 10,
            frames: vec![frame; 30],
            audio,
            channels: 1,
            sample_rate: 48_000,
        })
    }

    #[test]
    fn roundtrip_decodes_audio_and_video_meta() {
        let bytes = fixture();
        let m = decode_avi(&bytes).unwrap();
        assert_eq!(m.audio.sample_rate, 48_000);
        assert_eq!(m.audio.channels, 1);
        assert_eq!(m.audio.samples.len(), 960);
        let v = m.video.unwrap();
        assert_eq!(v.codec, "rawvideo");
        assert_eq!(v.frames, 30);
        assert_eq!(v.fps, Some(10.0));
        assert_eq!((v.width, v.height), (64, 48));
    }

    #[test]
    fn missing_audio_stream_fails_explicitly() {
        let mut spec_bytes = fixture();
        // Corrupt the audio stream's fccType so no 'auds' stream exists.
        let pos = spec_bytes.windows(4).position(|w| w == b"auds").unwrap();
        spec_bytes[pos] = b'x';
        let e = decode_avi(&spec_bytes).unwrap_err();
        assert!(matches!(e, MediaError::InputUnavailable { .. }));
    }

    #[test]
    fn truncated_movi_is_malformed() {
        let e = decode_avi(b"RIFF\x04\0\0\0AVI ").unwrap_err();
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

    fn strh_body(fcc: &[u8; 4], scale: u32, rate: u32, length: u32, sample_size: u32) -> Vec<u8> {
        let mut b = vec![0u8; 56];
        b[..4].copy_from_slice(fcc);
        b[20..24].copy_from_slice(&scale.to_le_bytes());
        b[24..28].copy_from_slice(&rate.to_le_bytes());
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
        push_chunk(&mut astrl, b"strh", &strh_body(b"auds", 2, 96_000, 0, 2));
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
        let e = decode_avi(&bytes).unwrap_err();
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
        let e = decode_avi(&bytes).unwrap_err();
        assert!(matches!(e, MediaError::Malformed { .. }));
    }

    #[test]
    fn chunk_body_overrunning_region_is_malformed() {
        let avi = avi_with_tail(b"JUNK\x10\0\0\0ab");
        let e = decode_avi(&avi).unwrap_err();
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
        let e = decode_avi(&avi_with_tail(&tail)).unwrap_err();
        assert!(matches!(e, MediaError::Malformed { .. }));
    }

    #[test]
    fn trailing_garbage_is_malformed() {
        let mut tail = Vec::new();
        push_chunk(&mut tail, b"JUNK", &[1, 2, 3, 4]);
        tail.extend_from_slice(b"ab"); // shorter than a chunk header
        let e = decode_avi(&avi_with_tail(&tail)).unwrap_err();
        assert!(matches!(e, MediaError::Malformed { .. }));
    }

    #[test]
    fn unsupported_audio_codec_is_input_unavailable() {
        // format_tag 2 (ADPCM): a structurally valid AVI with an
        // unsupported codec is `input unavailable`, not `malformed`.
        let avi = audio_only_avi(2, &[0u8; 64]);
        let e = decode_avi(&avi).unwrap_err();
        assert!(matches!(e, MediaError::InputUnavailable { .. }));
    }

    #[test]
    fn only_first_auds_stream_is_decoded() {
        // Stream 0: video. Stream 1: mono s16 audio (the one to decode).
        // Stream 2: stereo s16 audio — its packets must never leak into
        // stream 1's PCM.
        let audio_a: Vec<u8> = (0..40i16).flat_map(|v| (v + 100).to_le_bytes()).collect();
        let audio_b: Vec<u8> = (0..80i16).flat_map(|v| (v - 500).to_le_bytes()).collect();
        let m = decode_avi(&multi_stream_avi(&audio_a, &audio_b)).unwrap();

        assert_eq!(m.audio.channels, 1);
        let expected: Vec<f64> = (0..40i16).map(|v| f64::from(v + 100)).collect();
        assert_eq!(m.audio.samples, expected);
        // Stream 2's packets keep their own timing entry, never PCM.
        let s2 = m.timing.iter().find(|t| t.stream == 2).unwrap();
        assert_eq!(s2.observation.timestamp_count, 1);
    }

    #[test]
    fn packet_timing_is_observed_per_stream_clock() {
        let m = decode_avi(&fixture()).unwrap();
        assert_eq!(m.timing.len(), 2);

        // Video stream 0: 30 frames of 1 tick at a 10 Hz clock.
        let v = m.timing.iter().find(|t| t.stream_type == "vids").unwrap();
        assert_eq!(v.observation.clock, ClockKind::Video);
        assert_eq!(v.observation.timebase_hz, 10);
        assert_eq!(v.observation.timestamp_count, 30);
        assert!(v.observation.intervals.iter().all(|&i| i == 1));
        assert_eq!(v.observation.drift_ppm, Some(0.0));

        // Audio stream 1: a single wb packet covering its PCM, on the
        // byte-rate clock (block_align scale, byte_rate rate).
        let a = m.timing.iter().find(|t| t.stream_type == "auds").unwrap();
        assert_eq!(a.observation.clock, ClockKind::Audio);
        assert_eq!(a.observation.timebase_hz, 96_000);
        assert_eq!(a.observation.timestamp_count, 1);

        // Interleaved packet order maps to cumulative stream positions:
        // stream 1's two wb packets sit at 0 and len(first) ticks.
        let audio_a: Vec<u8> = (0..40i16).flat_map(|v| (v + 100).to_le_bytes()).collect();
        let half = audio_a.len() / 2;
        let m2 = decode_avi(&multi_stream_avi(&audio_a, &[7u8; 32])).unwrap();
        let a1 = m2.timing.iter().find(|t| t.stream == 1).unwrap();
        assert_eq!(a1.observation.timestamp_count, 2);
        assert_eq!(a1.observation.intervals, vec![half as i64]);
    }

    /// Two-audio-stream AVI (video + mono auds + stereo auds) with
    /// stream 1's audio split across two interleaved `01wb` packets.
    fn multi_stream_avi(audio_a: &[u8], audio_b: &[u8]) -> Vec<u8> {
        let half = audio_a.len() / 2;
        let mut hdrl = Vec::new();
        let mut vstrl = Vec::new();
        push_chunk(&mut vstrl, b"strh", &strh_body(b"vids", 1, 10, 2, 0));
        push_chunk(&mut vstrl, b"strf", &[0u8; 40]);
        push_list(&mut hdrl, b"strl", &vstrl);
        for (strh, strf) in [
            (
                strh_body(b"auds", 2, 96_000, 0, 2),
                wavefmt_body(1, 1, 48_000, 16),
            ),
            (
                strh_body(b"auds", 4, 192_000, 0, 4),
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
