//! RIFF/WAVE demuxing and PCM decoding.
//!
//! Only uncompressed PCM (format tag 1) and IEEE float (tag 3) are decoded.
//! Anything else fails explicitly as `decode/input unavailable`.

use crate::metadata::{MediaError, SampleFormat};

/// Decoded audio: pre-normalization PCM plus stream metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedAudio {
    /// Interleaved samples in the source's native scale, stored as f64.
    ///
    /// Integer PCM keeps its exact integer value (u8 is offset to signed),
    /// and IEEE float passes through unchanged. No scaling to a [-1, 1]
    /// range and no gain normalization is applied; `sample_format` records
    /// which scale the values are in. All supported integer widths fit
    /// exactly in an f64, so this is lossless.
    pub samples: Vec<f64>,
    /// Channel count.
    pub channels: u16,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Original bit depth.
    pub bits_per_sample: u16,
    /// Source sample format.
    pub sample_format: SampleFormat,
    /// Codec name for provenance (e.g. `pcm_s16le`).
    pub codec: String,
}

impl DecodedAudio {
    /// Number of frames (samples per channel).
    pub fn frames(&self) -> u64 {
        if self.channels == 0 {
            return 0;
        }
        (self.samples.len() / self.channels as usize) as u64
    }

    /// Duration in seconds, when the sample rate is known.
    pub fn duration_secs(&self) -> Option<f64> {
        if self.sample_rate == 0 {
            return None;
        }
        Some(self.frames() as f64 / f64::from(self.sample_rate))
    }
}

struct FmtChunk {
    format_tag: u16,
    channels: u16,
    sample_rate: u32,
    byte_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
}

fn read_u16(data: &[u8], at: usize) -> Result<u16, MediaError> {
    data.get(at..at + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| MediaError::malformed("truncated u16 field"))
}

fn read_u32(data: &[u8], at: usize) -> Result<u32, MediaError> {
    data.get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| MediaError::malformed("truncated u32 field"))
}

fn parse_fmt(chunk: &[u8]) -> Result<FmtChunk, MediaError> {
    if chunk.len() < 16 {
        return Err(MediaError::malformed("fmt chunk shorter than 16 bytes"));
    }
    Ok(FmtChunk {
        format_tag: read_u16(chunk, 0)?,
        channels: read_u16(chunk, 2)?,
        sample_rate: read_u32(chunk, 4)?,
        byte_rate: read_u32(chunk, 8)?,
        block_align: read_u16(chunk, 12)?,
        bits_per_sample: read_u16(chunk, 14)?,
    })
}

fn decode_samples(
    fmt: &FmtChunk,
    data: &[u8],
) -> Result<(SampleFormat, String, Vec<f64>), MediaError> {
    let (sample_format, codec, bytes_per_sample): (SampleFormat, &str, usize) =
        match (fmt.format_tag, fmt.bits_per_sample) {
            (1, 8) => (SampleFormat::PcmU8, "pcm_u8", 1),
            (1, 16) => (SampleFormat::PcmS16, "pcm_s16le", 2),
            (1, 24) => (SampleFormat::PcmS24, "pcm_s24le", 3),
            (1, 32) => (SampleFormat::PcmS32, "pcm_s32le", 4),
            (3, 32) => (SampleFormat::Float32, "pcm_f32le", 4),
            (3, 64) => (SampleFormat::Float64, "pcm_f64le", 8),
            (tag, bits) => {
                return Err(MediaError::input_unavailable(format!(
                    "unsupported WAV encoding: format_tag={tag} bits={bits}"
                )));
            }
        };

    // Cross-check the declared frame layout: a header that contradicts
    // itself is corrupt, not merely unusual.
    let expected_block = fmt.channels as usize * bytes_per_sample;
    if usize::from(fmt.block_align) != expected_block {
        return Err(MediaError::malformed(format!(
            "block_align {} does not match channels*bytes_per_sample {expected_block}",
            fmt.block_align
        )));
    }
    if u64::from(fmt.byte_rate) != u64::from(fmt.sample_rate) * u64::from(fmt.block_align) {
        return Err(MediaError::malformed(format!(
            "byte_rate {} does not match sample_rate*block_align",
            fmt.byte_rate
        )));
    }

    let frame_bytes = usize::from(fmt.block_align);
    if !data.len().is_multiple_of(frame_bytes) {
        return Err(MediaError::malformed("data chunk ends mid-frame"));
    }
    let mut samples = Vec::with_capacity(data.len() / bytes_per_sample);
    for raw in data.chunks_exact(bytes_per_sample) {
        let v = match bytes_per_sample {
            // 8-bit PCM is unsigned offset binary; keep the decoded value
            // as an exact (signed) integer, unscaled.
            1 if fmt.format_tag == 1 => f64::from(i16::from(raw[0]) - 128),
            2 => f64::from(i16::from_le_bytes([raw[0], raw[1]])),
            3 => {
                let v = i32::from(raw[0]) | (i32::from(raw[1]) << 8) | (i32::from(raw[2]) << 16);
                let v = if v & 0x80_0000 != 0 {
                    v | !0xFF_FFFF
                } else {
                    v
                };
                f64::from(v)
            }
            4 if fmt.format_tag == 1 => {
                f64::from(i32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
            }
            4 => f64::from(f32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]])),
            8 => f64::from_le_bytes([
                raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
            ]),
            _ => unreachable!("bytes_per_sample is fixed per format"),
        };
        samples.push(v);
    }
    Ok((sample_format, codec.to_owned(), samples))
}

/// Decode a RIFF/WAVE file into pre-normalization PCM.
pub fn decode_wav(data: &[u8]) -> Result<DecodedAudio, MediaError> {
    if data.len() < 12 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return Err(MediaError::input_unavailable("not a RIFF/WAVE file"));
    }
    // The RIFF size field declares the whole file minus its own header.
    let riff_size = read_u32(data, 4)? as usize;
    if riff_size != data.len() - 8 {
        return Err(MediaError::malformed(
            "RIFF size does not match file length",
        ));
    }

    let mut fmt: Option<FmtChunk> = None;
    let mut pcm: Option<&[u8]> = None;
    let mut offset = 12usize;
    while offset + 8 <= data.len() {
        let id = &data[offset..offset + 4];
        let size = read_u32(data, offset + 4)? as usize;
        let body_start = offset + 8;
        let body_end = body_start
            .checked_add(size)
            .ok_or_else(|| MediaError::malformed("chunk size overflow"))?;
        if body_end > data.len() {
            return Err(MediaError::malformed("chunk extends past end of file"));
        }
        // An odd-sized chunk must be followed by its word-alignment pad
        // byte — including the last chunk.
        if body_end + (size % 2) > data.len() {
            return Err(MediaError::malformed(
                "missing pad byte after odd-sized chunk",
            ));
        }
        let body = &data[body_start..body_end];
        match id {
            b"fmt " => fmt = Some(parse_fmt(body)?),
            b"data" => pcm = Some(body),
            _ => {}
        }
        // Chunks are word-aligned: an odd size is followed by a pad byte.
        offset = body_end + (size % 2);
    }
    if offset != data.len() {
        return Err(MediaError::malformed(
            "trailing bytes after last RIFF chunk",
        ));
    }

    let fmt = fmt.ok_or_else(|| MediaError::malformed("missing fmt chunk"))?;
    if fmt.channels == 0 {
        return Err(MediaError::malformed("fmt declares zero channels"));
    }
    let pcm = pcm.ok_or_else(|| MediaError::malformed("missing data chunk"))?;
    let (sample_format, codec, samples) = decode_samples(&fmt, pcm)?;

    Ok(DecodedAudio {
        samples,
        channels: fmt.channels,
        sample_rate: fmt.sample_rate,
        bits_per_sample: fmt.bits_per_sample,
        sample_format,
        codec,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_wav(channels: u16, rate: u32, bits: u16, frames: &[i16]) -> Vec<u8> {
        let block_align = channels * (bits / 8);
        let data_len = frames.len() as u32 * (bits as u32 / 8);
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVE");
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes()); // PCM
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&rate.to_le_bytes());
        out.extend_from_slice(&(rate * u32::from(block_align)).to_le_bytes());
        out.extend_from_slice(&block_align.to_le_bytes());
        out.extend_from_slice(&bits.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        for s in frames {
            out.extend_from_slice(&s.to_le_bytes());
        }
        out
    }

    #[test]
    fn decodes_16bit_stereo() {
        let wav = make_wav(2, 48000, 16, &[16384, -16384, 0, 32767]);
        let audio = decode_wav(&wav).unwrap();
        assert_eq!(audio.channels, 2);
        assert_eq!(audio.sample_rate, 48000);
        assert_eq!(audio.frames(), 2);
        // Pre-normalization: raw integer values are kept exactly.
        assert_eq!(audio.samples[0], 16384.0);
        assert_eq!(audio.samples[1], -16384.0);
        assert_eq!(audio.samples[3], 32767.0);
    }

    #[test]
    fn decodes_8bit_unsigned_offset_binary() {
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&40u32.to_le_bytes()); // file len 48 minus 8
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes()); // mono
        wav.extend_from_slice(&8000u32.to_le_bytes());
        wav.extend_from_slice(&8000u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&8u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&3u32.to_le_bytes());
        wav.extend_from_slice(&[0u8, 128, 255]);
        wav.push(0); // word-align pad
        let audio = decode_wav(&wav).unwrap();
        assert_eq!(audio.sample_format, SampleFormat::PcmU8);
        assert_eq!(audio.samples, vec![-128.0, 0.0, 127.0]);
    }

    #[test]
    fn rejects_data_chunk_ending_mid_frame() {
        let mut wav = make_wav(1, 48000, 16, &[1, 2, 3]);
        // Corrupt the data chunk size to an odd length (1.5 frames).
        let data_len_at = wav.len() - 10;
        wav[data_len_at] = 5;
        wav.truncate(wav.len() - 1);
        assert!(matches!(
            decode_wav(&wav),
            Err(MediaError::Malformed { .. })
        ));
    }

    #[test]
    fn rejects_missing_pad_byte_at_eof() {
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&39u32.to_le_bytes()); // len 47 minus 8
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&8000u32.to_le_bytes());
        wav.extend_from_slice(&8000u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&8u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&3u32.to_le_bytes());
        wav.extend_from_slice(&[0u8, 128, 255]);
        // No pad byte; the file ends mid-chunk-padding.
        assert!(matches!(
            decode_wav(&wav),
            Err(MediaError::Malformed { .. })
        ));
    }

    #[test]
    fn rejects_trailing_bytes_after_last_chunk() {
        let mut wav = make_wav(1, 48000, 16, &[1, 2]);
        wav.extend_from_slice(&[0xAA; 5]); // 5 bytes too short for a header
        // Keep RIFF size consistent so only the trailing bytes are wrong.
        let riff = (wav.len() - 8) as u32;
        wav[4..8].copy_from_slice(&riff.to_le_bytes());
        assert!(matches!(
            decode_wav(&wav),
            Err(MediaError::Malformed { .. })
        ));
    }

    #[test]
    fn rejects_inconsistent_riff_size() {
        let mut wav = make_wav(1, 48000, 16, &[1, 2]);
        wav[4] = 0; // declared RIFF size far too small
        assert!(matches!(
            decode_wav(&wav),
            Err(MediaError::Malformed { .. })
        ));
    }

    #[test]
    fn rejects_inconsistent_block_align() {
        let mut wav = make_wav(2, 48000, 16, &[1, 2]);
        // block_align field inside the fmt chunk (offset 12 within body,
        // body starts at byte 28).
        wav[28 + 12] = 99;
        assert!(matches!(
            decode_wav(&wav),
            Err(MediaError::Malformed { .. })
        ));
    }

    #[test]
    fn rejects_non_riff() {
        assert!(decode_wav(b"NOPE........").is_err());
        assert!(decode_wav(b"").is_err());
    }

    #[test]
    fn rejects_missing_fmt() {
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&12u32.to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&0u32.to_le_bytes());
        assert!(matches!(
            decode_wav(&wav),
            Err(MediaError::Malformed { .. })
        ));
    }

    #[test]
    fn rejects_truncated_chunk() {
        let mut wav = make_wav(1, 48000, 16, &[1, 2]);
        wav.truncate(wav.len() - 3);
        assert!(decode_wav(&wav).is_err());
    }
}
