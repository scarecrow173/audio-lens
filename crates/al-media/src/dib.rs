//! Native BI_RGB pixel decode。共通特徴抽出には decoded plane を渡す。
use crate::metadata::MediaError;
use crate::video::{
    DecodedVideoFrame, MAX_THUMB_FRAMES, MAX_THUMB_WIDTH, VideoExtraction, VideoFeatureConfig,
    VideoFrame, VideoFrameInfo, codec_fourcc, extract_decoded_features, unavailable_extraction,
};
use crate::wav::{read_u16, read_u32};

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
    pub(crate) fn row_stride(&self) -> usize {
        let bytes = self.width as usize * (self.bits_per_pixel as usize / 8);
        (bytes + 3) & !3
    }

    /// Bytes one frame body must carry.
    pub(crate) fn frame_bytes(&self) -> usize {
        self.row_stride() * self.height as usize
    }
}

/// BITMAPINFOHEADER を解析し、unsupported と malformed を型で保持する。
/// 局所失敗は video unavailable として独立した観測を保持する。
pub(crate) fn dib_format(strf: &[u8]) -> Result<DibFormat, MediaError> {
    let err = MediaError::malformed;
    if strf.len() < 40 {
        return Err(err("video strf shorter than BITMAPINFOHEADER"));
    }
    let width = read_u32(strf, 4)? as i32;
    let height = read_u32(strf, 8)? as i32;
    let bits_per_pixel = read_u16(strf, 14)?;
    let compression = read_u32(strf, 16)?;
    if compression != 0 {
        return Err(MediaError::input_unavailable(format!(
            "compressed video codec {} is not decodable",
            codec_fourcc(compression)
        )));
    }
    if width <= 0 || height == 0 {
        return Err(MediaError::malformed(format!(
            "invalid DIB dimensions {width}x{height}"
        )));
    }
    if !matches!(bits_per_pixel, 24 | 32) {
        return Err(MediaError::input_unavailable(format!(
            "unsupported DIB bit depth {bits_per_pixel}"
        )));
    }
    let fmt = DibFormat {
        width: width as u32,
        height: height.unsigned_abs(),
        bits_per_pixel,
        top_down: height < 0,
    };
    if fmt.frame_bytes() == 0 {
        return Err(MediaError::malformed("zero-sized DIB frame"));
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
pub(crate) fn decode_luma(dib: &DibFormat, body: &[u8], out: &mut Vec<f64>) -> Result<(), String> {
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

/// Decode one frame chunk into packed RGB24 (top-down row order), then
/// nearest-neighbour downscale so the result is at most `max_width`
/// pixels wide. Returns `(w, h, rgb)`.
pub(crate) fn decode_rgb_thumb(
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
    let unavailable = |error| unavailable_extraction(stream, &codec, error);
    let dib = match dib_format(strf) {
        Ok(d) => d,
        Err(error) => return unavailable(error),
    };

    // The declared frame size must fit inside the largest frame chunk
    // before any plane is allocated — corrupt BITMAPINFOHEADER
    // dimensions otherwise drive an unbounded allocation.
    let max_body = frames.iter().map(|f| f.body.len()).max().unwrap_or(0);
    if dib.frame_bytes() > max_body {
        return unavailable(MediaError::malformed(format!(
            "DIB frame needs {} bytes, largest frame chunk holds {}",
            dib.frame_bytes(),
            max_body
        )));
    }
    let thumb_step = frames.len().div_ceil(MAX_THUMB_FRAMES).max(1);
    let decoded = |i: usize, mut luma: Vec<f64>| {
        let frame = &frames[i];
        decode_luma(&dib, frame.body, &mut luma).map_err(MediaError::malformed)?;
        let preview = if i.is_multiple_of(thumb_step) {
            decode_rgb_thumb(&dib, frame.body, MAX_THUMB_WIDTH).ok()
        } else {
            None
        };
        Ok(DecodedVideoFrame {
            time_ticks: frame.time_ticks,
            luma,
            preview,
        })
    };
    extract_decoded_features(
        VideoFrameInfo {
            stream,
            codec,
            width: dib.width,
            height: dib.height,
            bits_per_pixel: dib.bits_per_pixel,
            timebase_hz,
            start_tick,
        },
        frames.len(),
        decoded,
        cfg,
    )
}
