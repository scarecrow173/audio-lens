//! RIFF/AVI demuxing: PCM audio extraction plus video stream metadata.
//!
//! Scope: legacy AVI (`RIFF 'AVI '`). The audio stream must carry PCM
//! or IEEE-float data (its `strf` chunk is a WAVEFORMATEX, the same
//! layout as a WAV `fmt ` chunk); other audio codecs fail explicitly
//! as `decode/input unavailable`. The video stream is never decoded —
//! only its declared facts (codec fourcc, dimensions, nominal frame
//! rate, frame count) are recorded, matching the ingest spec's
//! container-level observation role.

use crate::metadata::MediaError;
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

/// Demuxed AVI: decoded audio plus declared video metadata.
#[derive(Debug, Clone)]
pub struct AviMedia {
    /// PCM decoded from the first `auds` stream.
    pub audio: DecodedAudio,
    /// First `vids` stream's declared metadata, if present.
    pub video: Option<VideoInfo>,
}

struct StreamHead {
    fcc_type: [u8; 4],
    scale: u32,
    rate: u32,
    length: u32,
    format: Vec<u8>,
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

/// Iterate `(id, body)` chunk pairs inside a RIFF body region.
fn chunks(mut data: &[u8]) -> Vec<([u8; 4], &[u8])> {
    let mut out = Vec::new();
    while data.len() >= 8 {
        let id: [u8; 4] = data[..4].try_into().unwrap();
        let size = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
        if data.len() < 8 + size {
            break; // truncated tail — caller validates required chunks
        }
        out.push((id, &data[8..8 + size]));
        data = &data[(8 + size + (size % 2)).min(data.len())..];
    }
    out
}

fn parse_strh(body: &[u8]) -> Result<(u32, u32, u32), MediaError> {
    if body.len() < 36 {
        return Err(MediaError::malformed("strh chunk shorter than 36 bytes"));
    }
    Ok((
        read_u32(body, 20)?, // dwScale
        read_u32(body, 24)?, // dwRate
        read_u32(body, 32)?, // dwLength
    ))
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

/// Demux an AVI file: decode the first PCM audio stream and record the
/// first video stream's declared metadata.
pub fn decode_avi(data: &[u8]) -> Result<AviMedia, MediaError> {
    if data.len() < 12 || &data[0..4] != b"RIFF" || &data[8..12] != b"AVI " {
        return Err(MediaError::input_unavailable("not a RIFF/AVI file"));
    }
    let mut streams: Vec<StreamHead> = Vec::new();
    let mut audio_pcm: Vec<u8> = Vec::new();
    let mut video_frame_chunks: Vec<u64> = Vec::new();
    let mut movi_found = false;

    for (id, body) in chunks(&data[12..]) {
        if &id == b"LIST" && body.len() >= 4 {
            match &body[..4] {
                b"hdrl" => {
                    for (sid, sbody) in chunks(&body[4..]) {
                        if &sid != b"LIST" || sbody.len() < 4 || &sbody[..4] != b"strl" {
                            continue;
                        }
                        let mut head = StreamHead {
                            fcc_type: [0; 4],
                            scale: 0,
                            rate: 0,
                            length: 0,
                            format: Vec::new(),
                        };
                        let mut has_strh = false;
                        for (cid, cbody) in chunks(&sbody[4..]) {
                            match &cid {
                                b"strh" => {
                                    if cbody.len() < 4 {
                                        return Err(MediaError::malformed(
                                            "strh chunk shorter than fccType",
                                        ));
                                    }
                                    head.fcc_type.copy_from_slice(&cbody[..4]);
                                    (head.scale, head.rate, head.length) = parse_strh(cbody)?;
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
                    for (cid, cbody) in chunks(&body[4..]) {
                        // Chunk ids are `NNdb`/`NNdc` (video) / `NNwb` (audio).
                        if !cid[0].is_ascii_digit() || !cid[1].is_ascii_digit() {
                            continue;
                        }
                        let stream_no = ((cid[0] - b'0') as usize) * 10 + (cid[1] - b'0') as usize;
                        match &cid[2..4] {
                            b"wb" => audio_pcm.extend_from_slice(cbody),
                            b"db" | b"dc" => video_frame_chunks.push(stream_no as u64),
                            _ => {}
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

    let audio_head = streams
        .iter()
        .find(|s| &s.fcc_type == b"auds")
        .ok_or_else(|| MediaError::input_unavailable("AVI has no audio stream"))?;
    let audio = decode_waveformatex(&audio_head.format, &audio_pcm)
        .map_err(|e| MediaError::malformed(format!("AVI audio stream: {e}")))?;

    let video = streams
        .iter()
        .position(|s| &s.fcc_type == b"vids")
        .map(|vidx| {
            let counted = video_frame_chunks
                .iter()
                .filter(|&&n| n == vidx as u64)
                .count() as u64;
            video_info(&streams[vidx], counted)
        })
        .transpose()?;

    Ok(AviMedia { audio, video })
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
}
