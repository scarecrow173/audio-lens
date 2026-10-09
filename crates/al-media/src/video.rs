//! Uncompressed AVI `vids` frame decode (BI_RGB DIB) and the coarse
//! per-frame features persisted as the `video_features` ingest
//! artifact.
//!
//! Scope per `docs/adr/avi.md`: only uncompressed DIB payloads
//! (`biCompression == BI_RGB`, 24/32bpp) are decoded. Anything else —
//! compressed codec fourccs, other bit depths, truncated frame bodies —
//! becomes an explicit `unavailable` artifact record rather than a
//! decode failure.
//!
//! Frames are never retained: decode produces one luma plane at a
//! time, from which the mean luminance and the mean absolute
//! frame-to-frame luma difference are accumulated. Event timestamps
//! are stream-clock ticks supplied by the caller (`movi` order +
//! `strh` clock for AVI) — no nominal-FPS time reconstruction happens
//! here.

use serde::{Deserialize, Serialize};

use crate::wav::{read_u16, read_u32};

/// Codec fourcc as text; `rawvideo` when the compression field is 0.
pub(crate) fn codec_fourcc(codec: u32) -> String {
    if codec == 0 {
        return "rawvideo".into();
    }
    String::from_utf8_lossy(&codec.to_le_bytes())
        .trim_end_matches(['\0', ' '])
        .to_string()
}

/// Pixel layout of an uncompressed DIB frame, parsed from the video
/// stream's `strf` chunk (BITMAPINFOHEADER).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DibFormat {
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels (absolute value of `biHeight`).
    pub height: u32,
    /// Bits per pixel (24 or 32).
    pub bits_per_pixel: u16,
    /// Rows are stored top-down (`biHeight < 0`); bottom-up otherwise.
    pub top_down: bool,
}

impl DibFormat {
    /// Bytes per row including DWORD alignment padding.
    fn row_stride(&self) -> usize {
        let bytes = self.width as usize * (self.bits_per_pixel as usize / 8);
        (bytes + 3) & !3
    }

    /// Bytes one frame body must carry.
    fn frame_bytes(&self) -> usize {
        self.row_stride() * self.height as usize
    }
}

/// Parse the `strf` BITMAPINFOHEADER into a [`DibFormat`]. Returns a
/// human-readable reason — unsupported compression/bit depth and
/// malformed headers alike — because every video-side parse failure
/// surfaces as a `video_features` `unavailable` record, never as a
/// container-level `Malformed`.
fn dib_format(strf: &[u8]) -> Result<DibFormat, String> {
    let err = |m: &str| m.to_string();
    if strf.len() < 40 {
        return Err(err("video strf shorter than BITMAPINFOHEADER"));
    }
    let width = read_u32(strf, 4).map_err(|e| e.to_string())? as i32;
    let height = read_u32(strf, 8).map_err(|e| e.to_string())? as i32;
    let bits_per_pixel = read_u16(strf, 14).map_err(|e| e.to_string())?;
    let compression = read_u32(strf, 16).map_err(|e| e.to_string())?;
    if compression != 0 {
        return Err(format!(
            "compressed video codec {} is not decodable",
            codec_fourcc(compression)
        ));
    }
    if width <= 0 || height == 0 {
        return Err(format!("invalid DIB dimensions {width}x{height}"));
    }
    if !matches!(bits_per_pixel, 24 | 32) {
        return Err(format!("unsupported DIB bit depth {bits_per_pixel}"));
    }
    let fmt = DibFormat {
        width: width as u32,
        height: height.unsigned_abs(),
        bits_per_pixel,
        top_down: height < 0,
    };
    if fmt.frame_bytes() == 0 {
        return Err("zero-sized DIB frame".into());
    }
    Ok(fmt)
}

/// BT.601 luma of one BGR(A) pixel.
fn luma(b: u8, g: u8, r: u8) -> f64 {
    0.299 * f64::from(r) + 0.587 * f64::from(g) + 0.114 * f64::from(b)
}

/// Decode one frame chunk into `out` as a top-down-ordered luma plane
/// (`width * height` f64 values, 0..255 scale). `out` is reused across
/// frames — it is the scratch plane, not a retained frame.
fn decode_luma(dib: &DibFormat, body: &[u8], out: &mut Vec<f64>) -> Result<(), String> {
    let need = dib.frame_bytes();
    if body.len() < need {
        return Err(format!(
            "frame chunk holds {} bytes, need {need} ({}x{} {}bpp)",
            body.len(),
            dib.width,
            dib.height,
            dib.bits_per_pixel
        ));
    }
    let (w, h, stride, step) = (
        dib.width as usize,
        dib.height as usize,
        dib.row_stride(),
        dib.bits_per_pixel as usize / 8,
    );
    out.resize(w * h, 0.0);
    for r in 0..h {
        // BI_RGB stores rows bottom-up unless biHeight was negative.
        let dst_row = if dib.top_down { r } else { h - 1 - r };
        let row = &body[r * stride..r * stride + w * step];
        for (x, px) in row.chunks_exact(step).enumerate() {
            out[dst_row * w + x] = luma(px[0], px[1], px[2]);
        }
    }
    Ok(())
}

/// One frame-carrying `movi` chunk (`db`/`dc`) tagged with its
/// stream-clock position. Non-frame chunks (`pc` etc.) never appear
/// here, so frame indices stay aligned with `luminance`/`diff`.
pub struct VideoFrame<'a> {
    /// Position on the `vids` stream clock, in `timebase_hz` ticks.
    pub time_ticks: i64,
    /// Raw chunk body (one DIB frame).
    pub body: &'a [u8],
}

/// Decode one frame chunk into packed RGB24 (top-down row order), then
/// nearest-neighbour downscale so the result is at most `max_width`
/// pixels wide. Returns `(w, h, rgb)`.
fn decode_rgb_thumb(
    dib: &DibFormat,
    body: &[u8],
    max_width: u32,
) -> Result<(u32, u32, Vec<u8>), String> {
    let need = dib.frame_bytes();
    if body.len() < need {
        return Err(format!(
            "frame chunk holds {} bytes, need {need} ({}x{} {}bpp)",
            body.len(),
            dib.width,
            dib.height,
            dib.bits_per_pixel
        ));
    }
    let (w, h, stride, step) = (
        dib.width,
        dib.height,
        dib.row_stride(),
        dib.bits_per_pixel as usize / 8,
    );
    let scale = w.div_ceil(max_width).max(1);
    let (tw, th) = (w.div_ceil(scale), h.div_ceil(scale));
    let mut rgb = vec![0u8; tw as usize * th as usize * 3];
    for ty in 0..th as usize {
        let sy = ty * scale as usize;
        // BI_RGB stores rows bottom-up unless biHeight was negative.
        let src_row = if dib.top_down {
            sy
        } else {
            h as usize - 1 - sy
        };
        let row = &body[src_row * stride..src_row * stride + w as usize * step];
        for tx in 0..tw as usize {
            let px = &row[tx * scale as usize * step..][..step];
            // DIB pixel order is BGR(A); emit RGB.
            let o = (ty * tw as usize + tx) * 3;
            rgb[o] = px[2];
            rgb[o + 1] = px[1];
            rgb[o + 2] = px[0];
        }
    }
    Ok((tw, th, rgb))
}

/// Event detection tunables; every used value is persisted in the
/// artifact (`config.ingest.video.*` overrides).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VideoFeatureConfig {
    /// Mean absolute luma difference (0..255) between consecutive
    /// frames that marks a `scene_change` candidate.
    pub scene_change_threshold: f64,
    /// Absolute mean-luminance step (0..255) that marks a
    /// `luminance_step` event (flash / fade edge).
    pub luminance_step_threshold: f64,
    /// Mean absolute luma difference floor for a motion run.
    pub motion_threshold: f64,
    /// Consecutive frames over `motion_threshold` that open a
    /// `motion_onset` event.
    pub motion_min_run: usize,
}

impl Default for VideoFeatureConfig {
    fn default() -> Self {
        Self {
            scene_change_threshold: 40.0,
            luminance_step_threshold: 32.0,
            motion_threshold: 4.0,
            motion_min_run: 2,
        }
    }
}

impl VideoFeatureConfig {
    /// Overlay `config.ingest.video` onto the defaults.
    pub fn from_job(config: &serde_json::Value) -> Self {
        let c = &config["ingest"]["video"];
        let mut cfg = Self::default();
        if let Some(v) = c["scene_change_threshold"].as_f64() {
            cfg.scene_change_threshold = v;
        }
        if let Some(v) = c["luminance_step_threshold"].as_f64() {
            cfg.luminance_step_threshold = v;
        }
        if let Some(v) = c["motion_threshold"].as_f64() {
            cfg.motion_threshold = v;
        }
        if let Some(v) = c["motion_min_run"].as_u64() {
            cfg.motion_min_run = (v as usize).max(1);
        }
        cfg
    }
}

/// Coarse video event kinds the ingest stage can emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoEventKind {
    /// Frame-to-frame diff over `scene_change_threshold`.
    SceneChange,
    /// Mean-luminance step over `luminance_step_threshold`.
    LuminanceStep,
    /// Start of a sustained frame-diff run (`motion_threshold`,
    /// `motion_min_run`).
    MotionOnset,
}

/// One detected video event on the `vids` stream clock.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VideoEvent {
    /// Event classification.
    pub kind: VideoEventKind,
    /// Frame index into `luminance`/`diff`/`frame_timestamps`.
    pub frame: u32,
    /// Stream-clock timestamp in `timebase_hz` ticks.
    pub time_ticks: i64,
}

/// Decoded per-frame feature series of one `vids` stream. All series
/// are indexed by decoded frame; timestamps live on the same stream
/// clock as `StreamTiming`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoFeatures {
    /// `hdrl` stream index of the decoded `vids` stream.
    pub stream: u32,
    /// Declared codec fourcc (`rawvideo`, `DIB `, ...).
    pub codec: String,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Bits per pixel of the decoded DIB.
    pub bits_per_pixel: u16,
    /// Ticks per second of the stream clock.
    pub timebase_hz: u64,
    /// Stream-clock origin (`strh` `dwStart` in rate ticks).
    pub start_tick: i64,
    /// Stream-clock tick of each decoded frame — the frame-carrying
    /// subset of the stream's packet positions.
    pub frame_timestamps: Vec<i64>,
    /// Mean luma per frame (0..255).
    pub luminance: Vec<f64>,
    /// Mean absolute luma difference to the previous frame
    /// (`diff[0] == 0.0`; aligned with `luminance`).
    pub diff: Vec<f64>,
    /// Detected coarse events.
    pub events: Vec<VideoEvent>,
    /// Detection parameters actually used.
    pub params: VideoFeatureConfig,
}

/// Downscaled RGB24 thumbnail frames for preview/report rendering.
/// Frames are strided so at most [`MAX_THUMB_FRAMES`] are retained and
/// each is nearest-neighbour downscaled to at most [`MAX_THUMB_WIDTH`]
/// pixels wide. Pixel payloads are base64 so the artifact stays JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoThumbs {
    /// Thumbnail width in pixels.
    pub width: u32,
    /// Thumbnail height in pixels.
    pub height: u32,
    /// Ticks per second of the stream clock (same as `VideoFeatures`).
    pub timebase_hz: u64,
    /// Source frame index of each thumbnail (indexes
    /// `frame_timestamps`).
    pub frame_indices: Vec<u32>,
    /// Stream-clock tick of each embedded frame.
    pub frame_times_ticks: Vec<i64>,
    /// One base64-encoded packed RGB24 image per frame
    /// (row-major, top-down, `width * height * 3` bytes decoded).
    pub frames_rgb24_b64: Vec<String>,
}

/// Upper bound on retained thumbnail frames.
pub const MAX_THUMB_FRAMES: usize = 90;
/// Upper bound on thumbnail width in pixels.
pub const MAX_THUMB_WIDTH: u32 = 160;

/// `video_features` artifact payload: either the decoded series or an
/// explicit unavailability record — never silent absence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum VideoFeatureArtifact {
    /// Features decoded from an uncompressed `vids` stream.
    Decoded(VideoFeatures),
    /// The stream exists but cannot be decoded (compressed codec,
    /// unsupported pixel format, truncated frames).
    Unavailable {
        /// `hdrl` stream index of the `vids` stream.
        stream: u32,
        /// Declared codec fourcc as text.
        codec: String,
        /// `decode/input unavailable` detail.
        reason: String,
    },
}

/// Serialized `video_features` artifact body: the feature set tagged
/// with the input it was measured from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoFeatureReport {
    /// Manifest input id.
    pub input_id: String,
    /// Decoded series or explicit unavailability.
    pub features: VideoFeatureArtifact,
}

/// Derive the coarse event list from the decoded series. Events carry
/// stream-clock ticks; nothing is derived from a nominal frame rate.
fn detect_events(
    luminance: &[f64],
    diff: &[f64],
    frame_timestamps: &[i64],
    cfg: &VideoFeatureConfig,
) -> Vec<VideoEvent> {
    let mut events = Vec::new();
    let push = |events: &mut Vec<VideoEvent>, kind: VideoEventKind, frame: usize| {
        events.push(VideoEvent {
            kind,
            frame: frame as u32,
            time_ticks: frame_timestamps[frame],
        });
    };
    for i in 1..luminance.len() {
        if diff[i] >= cfg.scene_change_threshold {
            push(&mut events, VideoEventKind::SceneChange, i);
        }
        if (luminance[i] - luminance[i - 1]).abs() >= cfg.luminance_step_threshold {
            push(&mut events, VideoEventKind::LuminanceStep, i);
        }
    }
    // Motion onset: the first frame of a run of at least
    // `motion_min_run` consecutive diffs over `motion_threshold`,
    // entered from a quiet frame.
    let run = cfg.motion_min_run.max(1);
    for i in 1..diff.len() {
        if diff[i - 1] >= cfg.motion_threshold {
            continue;
        }
        let end = (i + run).min(diff.len());
        if end - i < run {
            continue;
        }
        if diff[i..end].iter().all(|&d| d >= cfg.motion_threshold) {
            push(&mut events, VideoEventKind::MotionOnset, i);
        }
    }
    events.sort_by_key(|e| e.time_ticks);
    events
}

/// Output of [`extract_features`]: the coarse feature series plus
/// downscaled preview thumbnails (`None` when the stream cannot be
/// decoded at all).
#[derive(Debug)]
pub struct VideoExtraction {
    /// Decoded series or explicit unavailability record.
    pub features: VideoFeatureArtifact,
    /// Preview thumbnails; absent on decode failure.
    pub thumbs: Option<VideoThumbs>,
}

/// Decode `frames` (the `db`/`dc` chunks of one `vids` stream, in mux
/// order, already tagged with stream-clock ticks) into a coarse
/// feature set plus strided preview thumbnails. Every video-side
/// failure becomes [`VideoFeatureArtifact::Unavailable`].
pub fn extract_features(
    stream: u32,
    strf: &[u8],
    timebase_hz: u64,
    start_tick: i64,
    frames: &[VideoFrame<'_>],
    cfg: &VideoFeatureConfig,
) -> VideoExtraction {
    let codec = if strf.len() >= 20 {
        codec_fourcc(read_u32(strf, 16).unwrap_or(0))
    } else {
        codec_fourcc(0)
    };
    let unavailable = |reason: String| VideoExtraction {
        features: VideoFeatureArtifact::Unavailable {
            stream,
            codec: codec.clone(),
            reason,
        },
        thumbs: None,
    };
    let dib = match dib_format(strf) {
        Ok(d) => d,
        Err(reason) => return unavailable(reason),
    };

    // The declared frame size must fit inside the largest frame chunk
    // before any plane is allocated — corrupt BITMAPINFOHEADER
    // dimensions otherwise drive an unbounded allocation.
    let max_body = frames.iter().map(|f| f.body.len()).max().unwrap_or(0);
    if dib.frame_bytes() > max_body {
        return unavailable(format!(
            "DIB frame needs {} bytes, largest frame chunk holds {}",
            dib.frame_bytes(),
            max_body
        ));
    }
    let pixels = dib.width as usize * dib.height as usize;
    let mut luma_plane = Vec::with_capacity(pixels);
    let mut prev: Option<Vec<f64>> = None;
    let mut frame_timestamps = Vec::with_capacity(frames.len());
    let mut luminance = Vec::with_capacity(frames.len());
    let mut diff = Vec::with_capacity(frames.len());
    // Stride the thumbnail set so long captures stay bounded.
    let thumb_step = frames.len().div_ceil(MAX_THUMB_FRAMES).max(1);
    let mut thumb_indices = Vec::new();
    let mut thumb_times = Vec::new();
    let mut thumb_frames = Vec::new();
    let (mut tw, mut th) = (0u32, 0u32);
    for (i, frame) in frames.iter().enumerate() {
        if let Err(reason) = decode_luma(&dib, frame.body, &mut luma_plane) {
            return unavailable(reason);
        }
        if i % thumb_step == 0 {
            // Thumbnail decode failure skips that frame only — the
            // preview degrades but the feature series stays whole.
            if let Ok((w, h, rgb)) = decode_rgb_thumb(&dib, frame.body, MAX_THUMB_WIDTH) {
                (tw, th) = (w, h);
                thumb_indices.push(i as u32);
                thumb_times.push(frame.time_ticks);
                thumb_frames.push(al_core::b64::encode(&rgb));
            }
        }
        frame_timestamps.push(frame.time_ticks);
        luminance.push(luma_plane.iter().sum::<f64>() / pixels as f64);
        diff.push(match &prev {
            Some(p) => {
                luma_plane
                    .iter()
                    .zip(p.iter())
                    .map(|(&a, &b)| (a - b).abs())
                    .sum::<f64>()
                    / pixels as f64
            }
            // The first frame has no predecessor.
            None => 0.0,
        });
        prev = Some(std::mem::take(&mut luma_plane));
    }
    let events = detect_events(&luminance, &diff, &frame_timestamps, cfg);
    let thumbs = (!thumb_frames.is_empty()).then_some(VideoThumbs {
        width: tw,
        height: th,
        timebase_hz,
        frame_indices: thumb_indices,
        frame_times_ticks: thumb_times,
        frames_rgb24_b64: thumb_frames,
    });
    VideoExtraction {
        features: VideoFeatureArtifact::Decoded(VideoFeatures {
            stream,
            codec,
            width: dib.width,
            height: dib.height,
            bits_per_pixel: dib.bits_per_pixel,
            timebase_hz,
            start_tick,
            frame_timestamps,
            luminance,
            diff,
            events,
            params: *cfg,
        }),
        thumbs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// BITMAPINFOHEADER body for a BI_RGB DIB (`height` signed so
    /// top-down fixtures are expressible).
    fn bih(width: i32, height: i32, bpp: u16, compression: u32) -> Vec<u8> {
        let mut b = vec![0u8; 40];
        b[0..4].copy_from_slice(&40u32.to_le_bytes());
        b[4..8].copy_from_slice(&width.to_le_bytes());
        b[8..12].copy_from_slice(&height.to_le_bytes());
        b[12..14].copy_from_slice(&1u16.to_le_bytes()); // planes
        b[14..16].copy_from_slice(&bpp.to_le_bytes());
        b[16..20].copy_from_slice(&compression.to_le_bytes());
        b
    }

    /// A 24bpp DIB body for `width x height` where row `y` (counted
    /// from the TOP) is filled with `rows[y]`. Rows are stored
    /// bottom-up with DWORD padding unless `top_down`.
    fn dib_body(width: usize, height: usize, rows: &[u8], top_down: bool) -> Vec<u8> {
        assert_eq!(rows.len(), height);
        let stride = (width * 3 + 3) & !3;
        let mut body = vec![0u8; stride * height];
        for r in 0..height {
            let dst = if top_down { r } else { height - 1 - r };
            for x in 0..width {
                let v = rows[dst];
                body[r * stride + x * 3] = v; // B
                body[r * stride + x * 3 + 1] = v; // G
                body[r * stride + x * 3 + 2] = v; // R
            }
        }
        body
    }

    fn frame(tick: i64, body: &[u8]) -> VideoFrame<'_> {
        VideoFrame {
            time_ticks: tick,
            body,
        }
    }

    #[test]
    fn bottom_up_and_top_down_decode_identically() {
        // Distinct per-row luma so row order is observable.
        let rows = [30u8, 90, 200];
        for top_down in [false, true] {
            let s = bih(4, if top_down { -3 } else { 3 }, 24, 0);
            let body = dib_body(4, 3, &rows, top_down);
            let mut plane = Vec::new();
            decode_luma(&dib_format(&s).unwrap(), &body, &mut plane).unwrap();
            // Row 0 of the decoded plane is the top row (luma 30).
            assert!((plane[0] - 30.0).abs() < 1.0);
            assert!((plane[8] - 200.0).abs() < 1.0);
        }
    }

    #[test]
    fn padded_24bpp_rows_decode() {
        // Width 2 → 6 data bytes padded to 8; the pad bytes must not
        // be read as pixels.
        let strf = bih(2, 2, 24, 0);
        let dib = dib_format(&strf).unwrap();
        assert_eq!(dib.row_stride(), 8);
        // Two top rows of luma 100 and luma 200 (bottom-up storage).
        let body = dib_body(2, 2, &[100, 200], false);
        assert_eq!(body.len(), 16);
        let mut plane = Vec::new();
        decode_luma(&dib, &body, &mut plane).unwrap();
        assert_eq!(plane.len(), 4);
        assert!((plane[0] - 100.0).abs() < 1.0);
        assert!((plane[3] - 200.0).abs() < 1.0);
    }

    #[test]
    fn thirtytwo_bpp_decodes() {
        let strf = bih(2, 1, 32, 0);
        let dib = dib_format(&strf).unwrap();
        // BGRX pixels; the 4th byte is skipped.
        let body = [0u8, 0, 255, 0xAA, 0, 0, 0, 0x55];
        let mut plane = Vec::new();
        decode_luma(&dib, &body, &mut plane).unwrap();
        assert!((plane[0] - 255.0 * 0.299).abs() < 0.5);
        assert_eq!(plane[1], 0.0);
    }

    #[test]
    fn compressed_or_odd_dib_is_unavailable_not_decoded() {
        let frames = [frame(0, &[0u8; 8])];
        // MJPG fourcc in biCompression.
        let mjpg = bih(4, 4, 24, u32::from_le_bytes(*b"MJPG"));
        let out = extract_features(0, &mjpg, 10, 0, &frames, &VideoFeatureConfig::default());
        assert!(matches!(
            out.features,
            VideoFeatureArtifact::Unavailable { .. }
        ));
        assert!(out.thumbs.is_none());
        if let VideoFeatureArtifact::Unavailable { codec, .. } = &out.features {
            assert_eq!(codec, "MJPG");
        }
        // 16bpp BI_RGB is not a supported pixel format.
        let bpp16 = bih(4, 4, 16, 0);
        let out = extract_features(0, &bpp16, 10, 0, &frames, &VideoFeatureConfig::default());
        assert!(matches!(
            out.features,
            VideoFeatureArtifact::Unavailable { .. }
        ));
        // Truncated frame body.
        let ok = bih(4, 4, 24, 0);
        let out = extract_features(0, &ok, 10, 0, &frames, &VideoFeatureConfig::default());
        assert!(matches!(
            out.features,
            VideoFeatureArtifact::Unavailable { .. }
        ));
    }

    /// Fill one row of a 4-wide 24bpp padded frame body with `v`.
    fn row_body(top_row: u8, bottom_row: u8) -> Vec<u8> {
        let stride = (4 * 3 + 3) & !3;
        let mut body = vec![0u8; stride * 2];
        // Bottom-up storage: the first stored row is the bottom row.
        for px in body[..stride].as_chunks_mut::<3>().0 {
            px.fill(bottom_row);
        }
        for px in body[stride..].as_chunks_mut::<3>().0 {
            px.fill(top_row);
        }
        body
    }

    #[test]
    fn known_frames_produce_known_features_and_events() {
        // Luma series: 0, 0, 36, 34, ~29, ~15, ~15, ~250 —
        // a luminance step (32..40) at f2, a quiet frame at f3,
        // sustained small diffs from f4 (motion onset), and a big
        // black→white cut at f7 (scene change + luminance step).
        let strf = bih(4, 2, 24, 0);
        let bodies = [
            row_body(0, 0),     // f0 black
            row_body(0, 0),     // f1 black
            row_body(36, 36),   // f2 step +36
            row_body(34, 34),   // f3 near-identical (diff 2 < motion floor)
            row_body(10, 0),    // f4 half-height bar (diff ~29)
            row_body(0, 20),    // f5 bar moved (diff ~15)
            row_body(10, 0),    // f6 bar back (diff ~15)
            row_body(255, 255), // f7 white flash (diff ~250)
        ];
        let frames: Vec<VideoFrame> = bodies
            .iter()
            .enumerate()
            .map(|(i, b)| frame(i as i64, b))
            .collect();
        let out = extract_features(0, &strf, 10, 0, &frames, &VideoFeatureConfig::default());
        let VideoFeatureArtifact::Decoded(f) = out.features else {
            panic!("expected decoded features")
        };
        assert_eq!(f.luminance.len(), 8);
        assert!(f.luminance[0].abs() < 1e-9);
        assert!((f.luminance[2] - 36.0).abs() < 1e-9);
        assert_eq!(f.diff[0], 0.0);
        assert!((f.diff[2] - 36.0).abs() < 1e-9);
        let has = |kind: VideoEventKind, frame: u32| {
            f.events.iter().any(|e| e.kind == kind && e.frame == frame)
        };
        // Step at f2: above luminance_step (32) but below scene (40).
        assert!(has(VideoEventKind::LuminanceStep, 2));
        assert!(!has(VideoEventKind::SceneChange, 2));
        // The step itself must not open a motion run (f3 is quiet).
        assert!(!has(VideoEventKind::MotionOnset, 2));
        // Sustained diffs from f4 open a motion onset at f4 (tick 4).
        assert!(has(VideoEventKind::MotionOnset, 4));
        assert!(
            f.events
                .iter()
                .any(|e| e.kind == VideoEventKind::MotionOnset && e.time_ticks == 4)
        );
        // The white cut at f7 is both a scene change and a step.
        assert!(has(VideoEventKind::SceneChange, 7));
        assert!(has(VideoEventKind::LuminanceStep, 7));
        // The adopted thresholds are persisted with the artifact.
        assert_eq!(f.params.scene_change_threshold, 40.0);
        assert_eq!(f.timebase_hz, 10);
        // Preview thumbnails cover every frame (8 < MAX_THUMB_FRAMES).
        let t = out.thumbs.expect("decoded stream keeps thumbnails");
        assert_eq!(t.frame_indices.len(), 8);
        assert_eq!(t.frame_times_ticks, (0..8).collect::<Vec<i64>>());
        assert!(t.width <= MAX_THUMB_WIDTH);
        assert_eq!(t.timebase_hz, 10);
    }

    #[test]
    fn thumbnails_decode_rgb_in_top_down_order() {
        // 2x2 bottom-up BGR24: stored rows are bottom, top. Stored
        // bottom row is red (B=0,G=0,R=255), top row is green.
        let dib = dib_format(&bih(2, 2, 24, 0)).unwrap();
        let stride = dib.row_stride();
        let mut body = vec![0u8; stride * 2];
        for px in body[..stride].as_chunks_mut::<3>().0 {
            px.copy_from_slice(&[0, 0, 255]); // bottom row = red
        }
        for px in body[stride..].as_chunks_mut::<3>().0 {
            px.copy_from_slice(&[0, 255, 0]); // top row = green
        }
        let (w, h, rgb) = decode_rgb_thumb(&dib, &body, MAX_THUMB_WIDTH).unwrap();
        assert_eq!((w, h), (2, 2));
        assert_eq!(&rgb[0..3], &[0, 255, 0]); // top-left = green
        assert_eq!(&rgb[6..9], &[255, 0, 0]); // bottom-left = red
    }

    #[test]
    fn artifact_serializes_status_tag() {
        let strf = bih(2, 1, 24, 0);
        let body = dib_body(2, 1, &[77], false);
        let out = extract_features(
            0,
            &strf,
            10,
            0,
            &[frame(0, &body)],
            &VideoFeatureConfig::default(),
        );
        let report = VideoFeatureReport {
            input_id: "IN-x".into(),
            features: out.features,
        };
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["features"]["status"], "decoded");
        let back: VideoFeatureReport = serde_json::from_value(json).unwrap();
        assert_eq!(back, report);
    }
}
