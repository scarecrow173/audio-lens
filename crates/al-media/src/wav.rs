//! RIFF/WAVE demuxing and PCM decoding.
//!
//! Only uncompressed PCM (format tag 1) and IEEE float (tag 3) are decoded.
//! Anything else fails explicitly as `decode/input unavailable`.

use std::io::{self, Read, Seek, SeekFrom, Write};

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

pub(crate) struct FmtChunk {
    format_tag: u16,
    channels: u16,
    sample_rate: u32,
    byte_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
}

pub(crate) fn read_u16(data: &[u8], at: usize) -> Result<u16, MediaError> {
    data.get(at..at + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| MediaError::malformed("truncated u16 field"))
}

pub(crate) fn read_u32(data: &[u8], at: usize) -> Result<u32, MediaError> {
    data.get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| MediaError::malformed("truncated u32 field"))
}

pub(crate) fn parse_fmt(chunk: &[u8]) -> Result<FmtChunk, MediaError> {
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

pub(crate) fn decode_samples(
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

/// Located WAV structure: parsed `fmt ` plus the `data` chunk's byte
/// range, so sample decoding can target a sub-range without holding
/// the whole file.
struct WavLayout {
    fmt: FmtChunk,
    data_offset: u64,
    data_len: u64,
}

/// Walk RIFF chunk headers from a seekable source. Only the `fmt `
/// body is read; `data` and unknown chunks are skipped by offset.
fn wav_layout<R: Read + Seek>(r: &mut R) -> Result<WavLayout, MediaError> {
    let file_len = r
        .seek(SeekFrom::End(0))
        .map_err(|e| MediaError::malformed(format!("cannot seek WAV source: {e}")))?;
    let read_at = |r: &mut R, at: u64, buf: &mut [u8]| -> Result<(), MediaError> {
        r.seek(SeekFrom::Start(at))
            .and_then(|_| r.read_exact(buf))
            .map_err(|e| MediaError::malformed(format!("cannot read WAV at {at}: {e}")))
    };
    let mut head = [0u8; 12];
    read_at(r, 0, &mut head).map_err(|_| MediaError::input_unavailable("not a RIFF/WAVE file"))?;
    if &head[0..4] != b"RIFF" || &head[8..12] != b"WAVE" {
        return Err(MediaError::input_unavailable("not a RIFF/WAVE file"));
    }
    // The RIFF size field declares the whole file minus its own header.
    let riff_size = u64::from(u32::from_le_bytes(head[4..8].try_into().unwrap()));
    if file_len < 12 || riff_size != file_len - 8 {
        return Err(MediaError::malformed(
            "RIFF size does not match file length",
        ));
    }

    // A fmt body beyond this is not a PCM header anyone produces.
    const MAX_FMT_BYTES: u64 = 4096;
    let mut fmt: Option<FmtChunk> = None;
    let mut pcm: Option<(u64, u64)> = None;
    let mut offset = 12u64;
    while offset + 8 <= file_len {
        let mut h = [0u8; 8];
        read_at(r, offset, &mut h)?;
        let size = u64::from(u32::from_le_bytes(h[4..8].try_into().unwrap()));
        let body_start = offset + 8;
        let body_end = body_start
            .checked_add(size)
            .ok_or_else(|| MediaError::malformed("chunk size overflow"))?;
        if body_end > file_len {
            return Err(MediaError::malformed("chunk extends past end of file"));
        }
        // An odd-sized chunk must be followed by its word-alignment pad
        // byte — including the last chunk.
        if body_end + (size % 2) > file_len {
            return Err(MediaError::malformed(
                "missing pad byte after odd-sized chunk",
            ));
        }
        match &h[0..4] {
            // Duplicate fmt/data are rejected rather than silently
            // last-wins: dropping an earlier `data` chunk would discard
            // part of the input, breaking the no-loss decode contract.
            b"fmt " => {
                if fmt.is_some() {
                    return Err(MediaError::malformed("duplicate fmt chunk"));
                }
                if size > MAX_FMT_BYTES {
                    return Err(MediaError::malformed("fmt chunk unreasonably large"));
                }
                let mut body = vec![0u8; size as usize];
                read_at(r, body_start, &mut body)?;
                fmt = Some(parse_fmt(&body)?);
            }
            b"data" => {
                if pcm.is_some() {
                    return Err(MediaError::malformed("duplicate data chunk"));
                }
                pcm = Some((body_start, size));
            }
            _ => {}
        }
        // Chunks are word-aligned: an odd size is followed by a pad byte.
        offset = body_end + (size % 2);
    }
    if offset != file_len {
        return Err(MediaError::malformed(
            "trailing bytes after last RIFF chunk",
        ));
    }

    let fmt = fmt.ok_or_else(|| MediaError::malformed("missing fmt chunk"))?;
    if fmt.channels == 0 {
        return Err(MediaError::malformed("fmt declares zero channels"));
    }
    if fmt.sample_rate == 0 {
        return Err(MediaError::malformed("fmt declares zero sample rate"));
    }
    let (data_offset, data_len) = pcm.ok_or_else(|| MediaError::malformed("missing data chunk"))?;
    if !data_len.is_multiple_of(u64::from(fmt.block_align)) {
        return Err(MediaError::malformed("data chunk ends mid-frame"));
    }
    Ok(WavLayout {
        fmt,
        data_offset,
        data_len,
    })
}

fn decoded(fmt: &FmtChunk, body: &[u8]) -> Result<DecodedAudio, MediaError> {
    let (sample_format, codec, samples) = decode_samples(fmt, body)?;
    Ok(DecodedAudio {
        samples,
        channels: fmt.channels,
        sample_rate: fmt.sample_rate,
        bits_per_sample: fmt.bits_per_sample,
        sample_format,
        codec,
    })
}

/// Decode a WAVEFORMATEX body + raw sample bytes (the layout shared by
/// WAV `fmt `/`data` chunks and AVI audio `strf`/`wb` chunks).
pub(crate) fn decode_waveformatex(
    fmt_bytes: &[u8],
    data: &[u8],
) -> Result<DecodedAudio, MediaError> {
    decoded(&parse_fmt(fmt_bytes)?, data)
}

/// Decode a RIFF/WAVE file into pre-normalization PCM.
pub fn decode_wav(data: &[u8]) -> Result<DecodedAudio, MediaError> {
    let layout = wav_layout(&mut io::Cursor::new(data))?;
    let body = &data[layout.data_offset as usize..(layout.data_offset + layout.data_len) as usize];
    decoded(&layout.fmt, body)
}

/// Number of PCM frames in the file, from the header alone.
pub fn wav_frame_count<R: Read + Seek>(r: &mut R) -> Result<u64, MediaError> {
    let layout = wav_layout(r)?;
    Ok(layout.data_len / u64::from(layout.fmt.block_align))
}

/// Decode frames `[start_frame, end_frame)` without loading the whole
/// file. Out-of-range or empty ranges fail explicitly.
pub fn read_wav_range<R: Read + Seek>(
    r: &mut R,
    start_frame: u64,
    end_frame: u64,
) -> Result<DecodedAudio, MediaError> {
    let layout = wav_layout(r)?;
    let block = u64::from(layout.fmt.block_align);
    let total = layout.data_len / block;
    if start_frame >= end_frame || end_frame > total {
        return Err(MediaError::malformed(format!(
            "frame range [{start_frame}, {end_frame}) outside {total} frames"
        )));
    }
    let byte_len = (end_frame - start_frame) * block;
    r.seek(SeekFrom::Start(layout.data_offset + start_frame * block))
        .map_err(|e| MediaError::malformed(format!("cannot seek WAV data: {e}")))?;
    let mut buf = vec![0u8; byte_len as usize];
    r.read_exact(&mut buf)
        .map_err(|e| MediaError::malformed(format!("cannot read WAV data: {e}")))?;
    decoded(&layout.fmt, &buf)
}

/// Output encoding for [`WavWriter`].
///
/// Input samples are `f64` in the *native scale* of the chosen encoding
/// — the same convention [`decode_wav`] uses for output. Integer
/// encodings expect integer-scale values (clamped to the target width,
/// rounded to nearest); `Float32` casts directly. Encoding a decoded
/// file in the same format is therefore lossless.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavEncoding {
    /// Signed 16-bit little-endian PCM.
    PcmS16,
    /// Signed 24-bit little-endian PCM.
    PcmS24,
    /// Signed 32-bit little-endian PCM.
    PcmS32,
    /// IEEE 754 32-bit float.
    Float32,
}

impl WavEncoding {
    fn format_tag(self) -> u16 {
        match self {
            Self::Float32 => 3,
            _ => 1,
        }
    }

    fn bits_per_sample(self) -> u16 {
        match self {
            Self::PcmS16 => 16,
            Self::PcmS24 => 24,
            Self::PcmS32 | Self::Float32 => 32,
        }
    }

    fn encode_sample(self, v: f64, out: &mut Vec<u8>) {
        match self {
            Self::PcmS16 => {
                let s = v.round().clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16;
                out.extend_from_slice(&s.to_le_bytes());
            }
            Self::PcmS24 => {
                let s = v.round().clamp(-8_388_608.0, 8_388_607.0) as i32;
                let b = s.to_le_bytes();
                out.extend_from_slice(&b[..3]);
            }
            Self::PcmS32 => {
                let s = v.round().clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32;
                out.extend_from_slice(&s.to_le_bytes());
            }
            Self::Float32 => out.extend_from_slice(&(v as f32).to_le_bytes()),
        }
    }
}

/// Streams `f64` samples into a RIFF/WAVE file.
///
/// The header is written up front with placeholder sizes and patched
/// on [`WavWriter::finish`], so arbitrarily long streams never need a
/// full-size output buffer in memory. Durability (fsync + rename) is
/// the caller's job — see `al_job::fsutil::rename_durable`.
pub struct WavWriter<W: Write + Seek> {
    out: W,
    encoding: WavEncoding,
    channels: u16,
    data_len: u64,
}

impl<W: Write + Seek> WavWriter<W> {
    /// Write the RIFF/`fmt `/`data` headers and get ready for samples.
    pub fn new(
        mut out: W,
        encoding: WavEncoding,
        channels: u16,
        sample_rate: u32,
    ) -> io::Result<Self> {
        if channels == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "channels must be > 0",
            ));
        }
        if sample_rate == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "sample_rate must be > 0",
            ));
        }
        let bits = encoding.bits_per_sample();
        // Compute in u32 so absurd channel counts fail explicitly
        // instead of wrapping into a self-inconsistent header.
        let block_align = u32::from(channels)
            .checked_mul(u32::from(bits / 8))
            .filter(|v| *v <= u32::from(u16::MAX))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("channels {channels} x {}-bit exceeds WAV block_align", bits),
                )
            })? as u16;
        let byte_rate = sample_rate
            .checked_mul(u32::from(block_align))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("sample_rate {sample_rate} x block_align {block_align} overflows u32"),
                )
            })?;
        // Placeholder sizes; patched on finish.
        out.write_all(b"RIFF")?;
        out.write_all(&0u32.to_le_bytes())?;
        out.write_all(b"WAVE")?;
        out.write_all(b"fmt ")?;
        out.write_all(&16u32.to_le_bytes())?;
        out.write_all(&encoding.format_tag().to_le_bytes())?;
        out.write_all(&channels.to_le_bytes())?;
        out.write_all(&sample_rate.to_le_bytes())?;
        out.write_all(&byte_rate.to_le_bytes())?;
        out.write_all(&block_align.to_le_bytes())?;
        out.write_all(&bits.to_le_bytes())?;
        out.write_all(b"data")?;
        out.write_all(&0u32.to_le_bytes())?;
        Ok(Self {
            out,
            encoding,
            channels,
            data_len: 0,
        })
    }

    /// Encode and append `samples` (interleaved f64 in the encoding's
    /// native scale). Length must be a multiple of `channels`.
    pub fn write_samples(&mut self, samples: &[f64]) -> io::Result<()> {
        if !samples.len().is_multiple_of(usize::from(self.channels)) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "sample count is not a multiple of channels",
            ));
        }
        let mut buf = Vec::with_capacity(samples.len() * 3);
        for &v in samples {
            self.encoding.encode_sample(v, &mut buf);
        }
        self.out.write_all(&buf)?;
        self.data_len += buf.len() as u64;
        Ok(())
    }

    /// Patch the RIFF and `data` size fields and return the writer.
    /// Chunks are word-aligned, so an odd `data` size is followed by a
    /// pad byte that counts toward the RIFF size. The stream is *not*
    /// flushed beyond what `out` does; the caller must still
    /// `sync_all`/rename for durability.
    pub fn finish(mut self) -> io::Result<W> {
        if self.data_len > u64::from(u32::MAX - 37) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "data chunk exceeds RIFF 32-bit size limit",
            ));
        }
        let pad = self.data_len % 2;
        if pad == 1 {
            self.out.seek(SeekFrom::End(0))?;
            self.out.write_all(&[0u8])?;
        }
        self.out.seek(SeekFrom::Start(4))?;
        self.out
            .write_all(&((36 + self.data_len + pad) as u32).to_le_bytes())?;
        self.out.seek(SeekFrom::Start(40))?;
        self.out.write_all(&(self.data_len as u32).to_le_bytes())?;
        Ok(self.out)
    }
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
    fn range_read_matches_full_decode() {
        let frames: Vec<i16> = vec![100, -200, 300, -400, 500, -600, 700, -800];
        let wav = make_wav(2, 48000, 16, &frames); // 4 stereo frames
        let mut c = io::Cursor::new(&wav);
        assert_eq!(wav_frame_count(&mut c).unwrap(), 4);
        let part = read_wav_range(&mut c, 1, 3).unwrap();
        assert_eq!(part.channels, 2);
        assert_eq!(part.frames(), 2);
        assert_eq!(part.samples, vec![300.0, -400.0, 500.0, -600.0]);
    }

    #[test]
    fn range_read_rejects_out_of_range() {
        let wav = make_wav(1, 48000, 16, &[1, 2, 3]);
        let mut c = io::Cursor::new(&wav);
        assert!(read_wav_range(&mut c, 2, 9).is_err());
        assert!(read_wav_range(&mut c, 2, 2).is_err());
        assert!(read_wav_range(&mut c, 0, 0).is_err());
        // The tail range is still allowed.
        let tail = read_wav_range(&mut c, 1, 3).unwrap();
        assert_eq!(tail.samples, vec![2.0, 3.0]);
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
    fn rejects_duplicate_data_chunk() {
        let mut wav = make_wav(1, 48000, 16, &[1, 2]);
        // A second data chunk would silently override the first; that
        // discards input bytes and must be rejected.
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&2u32.to_le_bytes());
        wav.extend_from_slice(&3i16.to_le_bytes());
        let riff = (wav.len() - 8) as u32;
        wav[4..8].copy_from_slice(&riff.to_le_bytes());
        assert!(matches!(
            decode_wav(&wav),
            Err(MediaError::Malformed { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_fmt_chunk() {
        let wav = make_wav(1, 48000, 16, &[1, 2]);
        let mut with_dup = Vec::new();
        with_dup.extend_from_slice(&wav[..36]); // RIFF header + first fmt
        with_dup.extend_from_slice(&wav[12..36]); // duplicate fmt
        with_dup.extend_from_slice(&wav[36..]); // data chunk
        let riff = (with_dup.len() - 8) as u32;
        with_dup[4..8].copy_from_slice(&riff.to_le_bytes());
        assert!(matches!(
            decode_wav(&with_dup),
            Err(MediaError::Malformed { .. })
        ));
    }

    #[test]
    fn rejects_zero_sample_rate() {
        let mut wav = make_wav(1, 48000, 16, &[1, 2]);
        // fmt body starts at byte 20; sample_rate at +4, byte_rate at +8.
        wav[24..28].copy_from_slice(&0u32.to_le_bytes());
        wav[28..32].copy_from_slice(&0u32.to_le_bytes());
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

    fn encode(encoding: WavEncoding, channels: u16, rate: u32, samples: &[f64]) -> Vec<u8> {
        let cursor = io::Cursor::new(Vec::new());
        let mut w = WavWriter::new(cursor, encoding, channels, rate).unwrap();
        w.write_samples(samples).unwrap();
        w.finish().unwrap().into_inner()
    }

    #[test]
    fn s16_encode_decodes_back_exactly() {
        let samples = [16384.0, -16384.0, 0.0, 32767.0];
        let wav = encode(WavEncoding::PcmS16, 2, 48000, &samples);
        let audio = decode_wav(&wav).unwrap();
        assert_eq!(audio.sample_format, SampleFormat::PcmS16);
        assert_eq!(audio.channels, 2);
        assert_eq!(audio.samples, samples);
    }

    #[test]
    fn s24_and_s32_round_trip() {
        for (enc, expected) in [
            (WavEncoding::PcmS24, SampleFormat::PcmS24),
            (WavEncoding::PcmS32, SampleFormat::PcmS32),
        ] {
            let samples = [8_388_000.0, -8_388_000.0, 1.0];
            let wav = encode(enc, 1, 44100, &samples);
            let audio = decode_wav(&wav).unwrap();
            assert_eq!(audio.sample_format, expected);
            assert_eq!(audio.samples, samples);
        }
    }

    #[test]
    fn float32_round_trip_keeps_native_scale() {
        let samples = [0.5_f64, -0.25, 1.0];
        let wav = encode(WavEncoding::Float32, 1, 48000, &samples);
        let audio = decode_wav(&wav).unwrap();
        assert_eq!(audio.sample_format, SampleFormat::Float32);
        assert_eq!(audio.samples, samples);
    }

    #[test]
    fn integer_encode_clamps_to_range() {
        let wav = encode(WavEncoding::PcmS16, 1, 48000, &[40_000.0, -40_000.0]);
        let audio = decode_wav(&wav).unwrap();
        assert_eq!(audio.samples, vec![32767.0, -32768.0]);
    }

    #[test]
    fn rejects_misaligned_sample_count_and_bad_params() {
        let cursor = io::Cursor::new(Vec::new());
        assert!(WavWriter::new(cursor, WavEncoding::PcmS16, 0, 48000).is_err());
        let cursor = io::Cursor::new(Vec::new());
        assert!(WavWriter::new(cursor, WavEncoding::PcmS16, 1, 0).is_err());
        let cursor = io::Cursor::new(Vec::new());
        let mut w = WavWriter::new(cursor, WavEncoding::PcmS16, 2, 48000).unwrap();
        assert!(w.write_samples(&[1.0]).is_err());
    }
}
