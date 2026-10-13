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

#[cfg(test)]
use crate::dib::{decode_luma, decode_rgb_thumb, dib_format};
use crate::metadata::MediaError;

/// Codec fourcc as text; `rawvideo` when the compression field is 0.
pub(crate) fn codec_fourcc(codec: u32) -> String {
    if codec == 0 {
        return "rawvideo".into();
    }
    String::from_utf8_lossy(&codec.to_le_bytes())
        .trim_end_matches(['\0', ' '])
        .to_string()
}

pub use crate::dib::DibFormat;
/// One frame-carrying `movi` chunk (`db`/`dc`) tagged with its
/// stream-clock position. Non-frame chunks (`pc` etc.) never appear
/// here, so frame indices stay aligned with `luminance`/`diff`.
pub struct VideoFrame<'a> {
    /// Position on the `vids` stream clock, in `timebase_hz` ticks.
    pub time_ticks: i64,
    /// Raw chunk body (one DIB frame).
    pub body: &'a [u8],
}

/// Event detection tunables; every used value is persisted in the
/// artifact (`config.ingest.video.*` overrides).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// 名前付き矩形。名前は marker と紐付ける安定キー。
    pub rois: Vec<VideoRoi>,
}

impl Default for VideoFeatureConfig {
    fn default() -> Self {
        Self {
            scene_change_threshold: 40.0,
            luminance_step_threshold: 32.0,
            motion_threshold: 4.0,
            motion_min_run: 2,
            rois: Vec::new(),
        }
    }
}

impl VideoFeatureConfig {
    /// Overlay `config.ingest.video` onto the defaults.
    pub fn from_job(config: &serde_json::Value) -> Result<Self, String> {
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
        if let Some(rois) = c.get("rois") {
            cfg.rois = serde_json::from_value(rois.clone())
                .map_err(|e| format!("invalid ingest.video.rois: {e}"))?;
        }
        cfg.validate_rois(None)?;
        Ok(cfg)
    }

    fn validate_rois(&self, dimensions: Option<(u32, u32)>) -> Result<(), String> {
        let mut names = std::collections::HashSet::new();
        for roi in &self.rois {
            let r = &roi.rect;
            if roi.name.trim().is_empty() || !names.insert(&roi.name) {
                return Err("ROI names must be nonempty and unique".into());
            }
            let right = r.x.checked_add(r.width);
            let bottom = r.y.checked_add(r.height);
            if r.width == 0
                || r.height == 0
                || right.is_none()
                || bottom.is_none()
                || dimensions.is_some_and(|(w, h)| right.unwrap() > w || bottom.unwrap() > h)
            {
                return Err(format!(
                    "invalid or out-of-frame ROI rectangle: {}",
                    roi.name
                ));
            }
        }
        Ok(())
    }
}

/// 左上原点のピクセル矩形。[x, x+width) × [y, y+height)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VideoRoiRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// marker との対応付けに使用できる ROI 名とその観測範囲。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VideoRoi {
    pub name: String,
    pub rect: VideoRoiRect,
}

/// フレーム時刻列と同じ index の ROI 平均輝度系列。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoiLuminance {
    pub roi: VideoRoi,
    pub luminance: Vec<f64>,
}

/// 汎用映像イベント種別(ingest の検出イベントとユーザー供給マーカー)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoEventKind {
    /// ユーザー供給の汎用 visual marker(analysis で追加)。
    UserMarker,
    /// Frame-to-frame diff over `scene_change_threshold`.
    SceneChange,
    /// Mean-luminance step over `luminance_step_threshold`.
    LuminanceStep,
    /// Start of a sustained frame-diff run (`motion_threshold`,
    /// `motion_min_run`).
    MotionOnset,
}

/// One detected video event on the `vids` stream clock.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoEvent {
    /// None は全フレーム、Some は名前付き ROI のイベント。
    pub roi_name: Option<String>,
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
    /// ROI 別の輝度。空なら従来の全フレーム観測だけ。
    pub roi_luminance: Vec<RoiLuminance>,
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
            roi_name: None,
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
    /// 局所失敗は capability と破損を区別する。
    pub unavailable: Option<crate::decoder::Unavailable>,
}

pub use crate::dib::extract_features;

/// 一枚分だけの decoded frame。pixel decoder と特徴抽出の境界。
pub struct DecodedVideoFrame {
    pub time_ticks: i64,
    pub luma: Vec<f64>,
    pub preview: Option<(u32, u32, Vec<u8>)>,
}

pub struct VideoFrameInfo {
    pub stream: u32,
    pub codec: String,
    pub width: u32,
    pub height: u32,
    pub bits_per_pixel: u16,
    pub timebase_hz: u64,
    pub start_tick: i64,
}

pub(crate) fn unavailable_extraction(
    stream: u32,
    codec: &str,
    error: MediaError,
) -> VideoExtraction {
    VideoExtraction {
        features: VideoFeatureArtifact::Unavailable {
            stream,
            codec: codec.into(),
            reason: error.to_string(),
        },
        thumbs: None,
        unavailable: Some(crate::decoder::Unavailable::from_error(&error)),
    }
}

/// Codec に依存せず top-down luma と元 clock から特徴を逐次算出する。
/// 比較を終えた前々フレームの buffer を decoder に返し、二枚を交互に使う。
pub fn extract_decoded_features(
    info: VideoFrameInfo,
    frame_count: usize,
    mut decode_frame: impl FnMut(usize, Vec<f64>) -> Result<DecodedVideoFrame, MediaError>,
    cfg: &VideoFeatureConfig,
) -> VideoExtraction {
    let VideoFrameInfo {
        stream,
        codec,
        width,
        height,
        bits_per_pixel,
        timebase_hz,
        start_tick,
    } = info;
    if let Err(reason) = cfg.validate_rois(Some((width, height))) {
        return unavailable_extraction(stream, &codec, MediaError::input_unavailable(reason));
    }
    let mut roi_luminance: Vec<RoiLuminance> = cfg
        .rois
        .iter()
        .map(|roi| RoiLuminance {
            roi: roi.clone(),
            luminance: Vec::with_capacity(frame_count),
        })
        .collect();
    let pixels = width as usize * height as usize;
    let mut prev: Option<Vec<f64>> = None;
    let mut scratch = Vec::new();
    let mut frame_timestamps = Vec::with_capacity(frame_count);
    let mut luminance = Vec::with_capacity(frame_count);
    let mut diff = Vec::with_capacity(frame_count);
    // Stride the thumbnail set so long captures stay bounded.
    let mut thumb_indices = Vec::new();
    let mut thumb_times = Vec::new();
    let mut thumb_frames = Vec::new();
    let (mut tw, mut th) = (0u32, 0u32);
    for i in 0..frame_count {
        let frame = match decode_frame(i, scratch) {
            Ok(frame) => frame,
            Err(error) => return unavailable_extraction(stream, &codec, error),
        };
        if let Some((w, h, rgb)) = frame.preview {
            (tw, th) = (w, h);
            thumb_indices.push(i as u32);
            thumb_times.push(frame.time_ticks);
            thumb_frames.push(al_core::b64::encode(&rgb));
        }
        for series in &mut roi_luminance {
            let r = &series.roi.rect;
            let mut sum = 0.0;
            for y in r.y..r.y + r.height {
                let start = y as usize * width as usize + r.x as usize;
                sum += frame.luma[start..start + r.width as usize]
                    .iter()
                    .sum::<f64>();
            }
            series
                .luminance
                .push(sum / (u64::from(r.width) * u64::from(r.height)) as f64);
        }
        frame_timestamps.push(frame.time_ticks);
        // One fused pass over `luma`: luminance and prev-diff in
        // separate accumulators, same ascending element order as
        // the split sums. `p.get(i)` keeps `zip`'s
        // silently-truncate semantics — indexing `p[i]` would turn
        // a length mismatch into a panic.
        let mut luma_sum = 0.0;
        let mut diff_sum = 0.0;
        if let Some(p) = &prev {
            for (i, &a) in frame.luma.iter().enumerate() {
                luma_sum += a;
                if let Some(&b) = p.get(i) {
                    diff_sum += (a - b).abs();
                }
            }
        } else {
            for &a in &frame.luma {
                luma_sum += a;
            }
        }
        luminance.push(luma_sum / pixels as f64);
        diff.push(if prev.is_some() {
            diff_sum / pixels as f64
        } else {
            // The first frame has no predecessor.
            0.0
        });
        scratch = prev.replace(frame.luma).unwrap_or_default();
    }
    let mut events = detect_events(&luminance, &diff, &frame_timestamps, cfg);
    for series in &roi_luminance {
        for (i, pair) in series.luminance.windows(2).enumerate() {
            let i = i + 1;
            if (pair[1] - pair[0]).abs() >= cfg.luminance_step_threshold {
                events.push(VideoEvent {
                    kind: VideoEventKind::LuminanceStep,
                    frame: i as u32,
                    time_ticks: frame_timestamps[i],
                    roi_name: Some(series.roi.name.clone()),
                });
            }
        }
    }
    events.sort_by_key(|e| e.time_ticks);
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
            width,
            height,
            bits_per_pixel,
            timebase_hz,
            start_tick,
            frame_timestamps,
            luminance,
            diff,
            events,
            params: cfg.clone(),
            roi_luminance,
        }),
        thumbs,
        unavailable: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luma_planes_are_recycled_after_previous_frame_comparison() {
        let info = VideoFrameInfo {
            stream: 0,
            codec: "rawvideo".into(),
            width: 2,
            height: 2,
            bits_per_pixel: 24,
            timebase_hz: 30,
            start_tick: 0,
        };
        let mut pointers = Vec::new();
        let mut capacities = Vec::new();
        let extraction = extract_decoded_features(
            info,
            8,
            |i, mut luma: Vec<f64>| {
                capacities.push(luma.capacity());
                luma.resize(4, 0.0);
                luma.fill(i as f64);
                pointers.push(luma.as_ptr());
                Ok(DecodedVideoFrame {
                    time_ticks: i as i64,
                    luma,
                    preview: None,
                })
            },
            &VideoFeatureConfig::default(),
        );
        let VideoFeatureArtifact::Decoded(features) = extraction.features else {
            panic!("decoded")
        };
        assert_eq!(
            features.luminance,
            (0..8).map(|i| i as f64).collect::<Vec<_>>()
        );
        assert_eq!(features.diff, vec![0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]);
        assert!(capacities[2..].iter().all(|c| *c >= 4));
        for i in 2..8 {
            assert_eq!(pointers[i], pointers[i - 2]);
        }
    }

    #[test]
    fn roi_tracks_local_step_on_stream_clock() {
        let cfg = VideoFeatureConfig::from_job(&serde_json::json!({
            "ingest": {"video": {"rois": [
                {"name": "字幕", "rect": {"x": 0, "y": 0, "width": 1, "height": 1}},
                {"name": "背景", "rect": {"x": 1, "y": 1, "width": 1, "height": 1}}
            ]}}
        }))
        .unwrap();
        let frames = [vec![0.0; 4], vec![100.0, 0.0, 0.0, 0.0]];
        let out = extract_decoded_features(
            VideoFrameInfo {
                stream: 0,
                codec: "rawvideo".into(),
                width: 2,
                height: 2,
                bits_per_pixel: 24,
                timebase_hz: 10,
                start_tick: 7,
            },
            2,
            |i, _scratch| {
                Ok(DecodedVideoFrame {
                    time_ticks: 7 + i as i64 * 3,
                    luma: frames[i].clone(),
                    preview: None,
                })
            },
            &cfg,
        );
        let body = serde_json::to_value(&out.features).unwrap();
        assert_eq!(body["roi_luminance"][0]["roi"]["name"], "字幕");
        assert_eq!(
            body["roi_luminance"][0]["luminance"],
            serde_json::json!([0.0, 100.0])
        );
        assert_eq!(
            body["roi_luminance"][1]["luminance"],
            serde_json::json!([0.0, 0.0])
        );
        assert_eq!(body["luminance"], serde_json::json!([0.0, 25.0]));
        assert_eq!(body["events"].as_array().unwrap().len(), 1);
        assert_eq!(body["events"][0]["roi_name"], "字幕");
        assert_eq!(body["events"][0]["time_ticks"], 10);
    }

    #[test]
    fn invalid_roi_config_is_explicit() {
        let roi =
            serde_json::json!({"name": "字幕", "rect": {"x": 0, "y": 0, "width": 1, "height": 1}});
        for rois in [
            serde_json::json!(null),
            serde_json::json!({}),
            serde_json::json!([roi.clone(), roi.clone()]),
            serde_json::json!([{ "name": " ", "rect": roi["rect"] }]),
            serde_json::json!([{ "name": "字幕", "rect": {"x": 0, "y": 0, "width": 0, "height": 1} }]),
            serde_json::json!([{ "name": "字幕", "rect": {"x": u32::MAX, "y": 0, "width": 1, "height": 1} }]),
            serde_json::json!([{ "name": "字幕", "rect": {"x": -1, "y": 0, "width": 1, "height": 1} }]),
        ] {
            assert!(
                VideoFeatureConfig::from_job(
                    &serde_json::json!({"ingest": {"video": {"rois": rois}}})
                )
                .is_err()
            );
        }
        let cfg = VideoFeatureConfig {
            rois: vec![VideoRoi {
                name: "外側".into(),
                rect: VideoRoiRect {
                    x: 2,
                    y: 0,
                    width: 1,
                    height: 1,
                },
            }],
            ..Default::default()
        };
        let out = extract_features(
            0,
            &bih(2, 2, 24, 0),
            10,
            0,
            &[frame(0, &dib_body(2, 2, &[0, 0], false))],
            &cfg,
        );
        assert!(
            matches!(out.features, VideoFeatureArtifact::Unavailable { ref reason, .. } if reason.contains("外側"))
        );
    }

    #[test]
    fn roi_coordinates_follow_display_rows_with_padding() {
        let cfg = VideoFeatureConfig {
            rois: vec![VideoRoi {
                name: "上段".into(),
                rect: VideoRoiRect {
                    x: 1,
                    y: 0,
                    width: 1,
                    height: 1,
                },
            }],
            ..Default::default()
        };
        for top_down in [true, false] {
            let strf = bih(2, if top_down { -2 } else { 2 }, 24, 0);
            let body = dib_body(2, 2, &[70, 210], top_down);
            let out = extract_features(0, &strf, 10, 0, &[frame(0, &body)], &cfg);
            let VideoFeatureArtifact::Decoded(f) = out.features else {
                panic!("decode")
            };
            assert!((f.roi_luminance[0].luminance[0] - 70.0).abs() < 1e-9);
        }
    }

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

    // ==== mechanical-caller-cleanup characterization (A16c) ====
    // TASK-034EO484s25JM8bsb6zUCh (plan T1). Pins the CURRENT
    // per-frame `luminance`/`diff` series of
    // `extract_decoded_features` — mean-of-all-elements luminance,
    // `diff[0] == 0.0`, mean |a-b| over the `zip` of current and
    // previous planes divided by `pixels` — bit-exact via
    // `f64::to_bits` against an oracle re-implementing the current
    // formulas (test-spec Rules C/D/E). No tolerance predicate.

    fn video_info(width: u32, height: u32) -> VideoFrameInfo {
        VideoFrameInfo {
            stream: 0,
            codec: "rawvideo".into(),
            width,
            height,
            bits_per_pixel: 24,
            timebase_hz: 30,
            start_tick: 0,
        }
    }

    fn decoded_frames(out: &VideoExtraction) -> &VideoFeatures {
        let VideoFeatureArtifact::Decoded(f) = &out.features else {
            panic!("expected decoded features, got {:?}", out.features)
        };
        f
    }

    /// Oracle for the current per-frame accumulation: luminance is
    /// `sum(luma) / pixels`; `diff[0] == 0.0`; `diff[i]` is the
    /// mean |a-b| over `luma.zip(prev)` (which silently truncates
    /// when lengths differ) divided by `pixels`.
    fn luma_diff_oracle(frames: &[Vec<f64>], pixels: usize) -> (Vec<f64>, Vec<f64>) {
        let mut luminance = Vec::with_capacity(frames.len());
        let mut diff = Vec::with_capacity(frames.len());
        for (i, luma) in frames.iter().enumerate() {
            luminance.push(luma.iter().sum::<f64>() / pixels as f64);
            diff.push(if i == 0 {
                0.0
            } else {
                luma.iter()
                    .zip(frames[i - 1].iter())
                    .map(|(&a, &b)| (a - b).abs())
                    .sum::<f64>()
                    / pixels as f64
            });
        }
        (luminance, diff)
    }

    fn assert_bits(actual: &[f64], expected: &[f64], what: &str) {
        assert_eq!(
            actual.len(),
            expected.len(),
            "{what} length {} vs {}",
            actual.len(),
            expected.len()
        );
        for (i, (&a, &e)) in actual.iter().zip(expected).enumerate() {
            assert_eq!(a.to_bits(), e.to_bits(), "{what}[{i}]");
        }
    }

    #[test]
    fn a16c_luma_diff_series_is_bit_exact() {
        // Deterministic varying-content frames (4x3).
        let pixels = 12usize;
        let frames: Vec<Vec<f64>> = (0..6)
            .map(|i| {
                (0..pixels)
                    .map(|p| ((i * 7 + p * 3) % 13) as f64 * 8.0 - 48.0)
                    .collect()
            })
            .collect();
        let (el, ed) = luma_diff_oracle(&frames, pixels);
        let out = extract_decoded_features(
            video_info(4, 3),
            frames.len(),
            |i, _scratch| {
                Ok(DecodedVideoFrame {
                    time_ticks: i as i64,
                    luma: frames[i].clone(),
                    preview: None,
                })
            },
            &VideoFeatureConfig::default(),
        );
        let f = decoded_frames(&out);
        assert_bits(&f.luminance, &el, "luminance");
        assert_bits(&f.diff, &ed, "diff");
        assert_eq!(f.diff[0].to_bits(), 0.0f64.to_bits(), "diff[0]");
        assert_eq!(f.frame_timestamps, (0..6).collect::<Vec<i64>>());
    }

    #[test]
    fn a16c_length_mismatch_truncates_via_zip() {
        // The decoder may hand back a plane whose length differs
        // from `pixels`: current code does NOT validate it — the
        // `zip` truncates the diff while luminance still divides by
        // `pixels`. Pin that exact behavior.
        let frames: Vec<Vec<f64>> = vec![vec![10.0; 4], vec![20.0; 2], vec![30.0; 4]];
        let pixels = 4usize;
        let (el, ed) = luma_diff_oracle(&frames, pixels);
        let out = extract_decoded_features(
            video_info(2, 2),
            frames.len(),
            |i, _scratch| {
                Ok(DecodedVideoFrame {
                    time_ticks: i as i64,
                    luma: frames[i].clone(),
                    preview: None,
                })
            },
            &VideoFeatureConfig::default(),
        );
        let f = decoded_frames(&out);
        assert_bits(&f.luminance, &el, "luminance");
        assert_bits(&f.diff, &ed, "diff");
        // diff[1] only spans min(2,4)=2 terms.
        assert_eq!(
            f.diff[1].to_bits(),
            (2.0f64 * 10.0 / pixels as f64).to_bits()
        );
    }

    #[test]
    fn a16c_zero_pixel_degenerate_series_is_pinned() {
        // `width * height == 0` → luminance is `0.0 / 0.0` = NaN on
        // every frame; `diff[0] == 0.0`, later diffs are NaN as
        // well (`sum / 0`). Exact bit pattern of quiet NaN.
        let frames: Vec<Vec<f64>> = vec![Vec::new(); 3];
        let (el, ed) = luma_diff_oracle(&frames, 0);
        let out = extract_decoded_features(
            video_info(0, 0),
            frames.len(),
            |i, _scratch| {
                Ok(DecodedVideoFrame {
                    time_ticks: i as i64,
                    luma: Vec::new(),
                    preview: None,
                })
            },
            &VideoFeatureConfig::default(),
        );
        let f = decoded_frames(&out);
        assert_bits(&f.luminance, &el, "luminance");
        assert_bits(&f.diff, &ed, "diff");
        assert!(f.luminance.iter().all(|v| v.is_nan()));
        assert_eq!(f.diff[0].to_bits(), 0.0f64.to_bits());
        assert!(f.diff[1..].iter().all(|v| v.is_nan()));
    }
}
