//! ISO-BMFF / MP4 demuxing: track metadata, sample-table timing
//! observation, and uncompressed-PCM sample extraction.
//!
//! Scope: demux and observation only — no codec decoding. Per the MP4
//! ADR, compressed tracks are identified and recorded but their samples
//! are never decoded; uncompressed PCM sample entries (`sowt`, `twos`,
//! `lpcm`, `raw `, ISO 23003-5 `ipcm`/`fpcm`) are extracted as part of
//! demuxing and returned under the same pre-normalization contract as
//! WAV: integer-scale `f64` + the matching `SampleFormat` (the
//! downstream `decode_input` divides by `format_scale`, so scaling here
//! would double-normalize).
//!
//! Timing: `observe_timing` requires a monotonic non-decreasing
//! sequence, so only the decode-order DTS series (cumulative `stts`
//! deltas, monotonic by construction) is observed under
//! `sample_table`. `ctts`-derived PTS is recorded separately under
//! `sample_table_pts`, sorted into presentation order so it stays
//! monotonic — a B-frame reorder must not surface as a false
//! discontinuity on the DTS series.
//!
//! Fragmented MP4 (`moof`) is detected and fails explicitly as
//! `decode/input unavailable`. Boxes we do not interpret are skipped
//! and their paths recorded in `skipped_boxes`; structurally required
//! boxes (`moov`, per-`trak` `mdia`/`mdhd`/`hdlr`, `stbl`/`stsd`) are
//! validated strictly and their absence is `Malformed`.

use crate::avi::{StreamTiming, TimestampOrigin};
use crate::metadata::{IntegrityObservation, MediaError, SampleFormat};
use crate::timing::{ClockKind, observe_timing};
use crate::wav::DecodedAudio;

fn be_u16(data: &[u8], at: usize) -> Result<u16, MediaError> {
    data.get(at..at + 2)
        .map(|b| u16::from_be_bytes([b[0], b[1]]))
        .ok_or_else(|| MediaError::malformed("truncated be u16 field"))
}

fn be_u32(data: &[u8], at: usize) -> Result<u32, MediaError> {
    data.get(at..at + 4)
        .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| MediaError::malformed("truncated be u32 field"))
}

fn be_u64(data: &[u8], at: usize) -> Result<u64, MediaError> {
    data.get(at..at + 8)
        .map(|b| u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
        .ok_or_else(|| MediaError::malformed("truncated be u64 field"))
}

fn be_i32(data: &[u8], at: usize) -> Result<i32, MediaError> {
    data.get(at..at + 4)
        .map(|b| i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| MediaError::malformed("truncated be i32 field"))
}

fn be_f64(data: &[u8], at: usize) -> Result<f64, MediaError> {
    data.get(at..at + 8)
        .map(|b| f64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
        .ok_or_else(|| MediaError::malformed("truncated be f64 field"))
}

fn fourcc_text(tag: &[u8; 4]) -> String {
    String::from_utf8_lossy(tag)
        .trim_end_matches(['\0', ' '])
        .to_string()
}

/// A box header within `data[at..limit]`.
struct BoxHeader {
    tag: [u8; 4],
    body_start: usize,
    body_end: usize,
}

/// Read one box header. `size == 1` selects the 64-bit largesize form;
/// `size == 0` means the box runs to the end of its enclosing region.
/// A box that overruns `limit`, a truncated header, or a size smaller
/// than the header itself is `Malformed`.
fn box_header(
    data: &[u8],
    at: usize,
    limit: usize,
    context: &str,
) -> Result<BoxHeader, MediaError> {
    if at + 8 > limit {
        return Err(MediaError::malformed(format!(
            "{context}: truncated box header"
        )));
    }
    let size = be_u32(data, at)? as u64;
    let tag: [u8; 4] = data[at + 4..at + 8].try_into().unwrap();
    let (header, body_len) = match size {
        0 => (8u64, (limit - at) as u64 - 8),
        1 => {
            if at + 16 > limit {
                return Err(MediaError::malformed(format!(
                    "{context}: truncated largesize box header"
                )));
            }
            let large = be_u64(data, at + 8)?;
            if large < 16 {
                return Err(MediaError::malformed(format!(
                    "{context}: box {} largesize smaller than header",
                    fourcc_text(&tag)
                )));
            }
            (16u64, large - 16)
        }
        s if s < 8 => {
            return Err(MediaError::malformed(format!(
                "{context}: box {} size smaller than header",
                fourcc_text(&tag)
            )));
        }
        s => (8u64, s - 8),
    };
    let body_start = at + header as usize;
    let body_end = body_start
        .checked_add(body_len as usize)
        .ok_or_else(|| MediaError::malformed(format!("{context}: box size overflows")))?;
    if body_end > limit {
        return Err(MediaError::malformed(format!(
            "{context}: box {} extends past end of region",
            fourcc_text(&tag)
        )));
    }
    Ok(BoxHeader {
        tag,
        body_start,
        body_end,
    })
}

/// List the immediate child boxes of `data[start..end]`.
fn child_boxes(
    data: &[u8],
    start: usize,
    end: usize,
    context: &str,
) -> Result<Vec<BoxHeader>, MediaError> {
    let mut out = Vec::new();
    let mut at = start;
    while at < end {
        let h = box_header(data, at, end, context)?;
        at = h.body_end;
        out.push(h);
    }
    Ok(out)
}

/// `ftyp` brand information recorded at ingest.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Mp4Brands {
    /// `ftyp` major brand (e.g. `isom`, `mp42`).
    pub major_brand: String,
    /// `ftyp` minor version field.
    pub minor_version: u32,
    /// `ftyp` compatible-brands list.
    pub compatible_brands: Vec<String>,
}

/// Per-track facts declared in `moov` (no sample data decoded).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Mp4Track {
    /// `trak` order index within `moov`.
    pub track_index: u32,
    /// `tkhd` track id, when the box is present.
    pub track_id: Option<u32>,
    /// `hdlr` handler type (`vide`, `soun`, ...).
    pub handler: String,
    /// First `stsd` sample-entry format (codec fourcc, e.g. `mp4a`),
    /// `None` for an empty description.
    pub codec: Option<String>,
    /// `mdhd` media timescale (ticks per second).
    pub timescale: u32,
    /// `mdhd` media duration in timescale ticks.
    pub duration_ticks: u64,
    /// `mdhd` language (ISO-639-2/T packed 15-bit, decoded; `und` when
    /// undeclared).
    pub language: String,
    /// `tkhd` duration on the movie timescale, when present.
    pub movie_duration_ticks: Option<u64>,
    /// Number of samples declared by the sample table.
    pub sample_count: u64,
    /// `stss` sync-sample indices; absent `stss` means all samples are
    /// sync points, recorded as `None`.
    pub sync_samples: Option<Vec<u32>>,
    /// `elst` presence. Edit lists are recorded, not interpreted.
    pub has_edit_list: bool,
    /// Sound sample entry fields for `soun` tracks.
    pub audio: Option<Mp4SoundInfo>,
    /// Visual sample entry dimensions for `vide` tracks.
    pub video: Option<Mp4VideoInfo>,
}

/// Declared audio facts from a `soun` sample entry.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Mp4SoundInfo {
    /// Declared channel count.
    pub channels: u16,
    /// Declared bits per sample.
    pub bits_per_sample: u16,
    /// Declared sample rate in Hz.
    pub sample_rate: u32,
}

/// Declared video facts from a `vide` sample entry.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Mp4VideoInfo {
    /// Declared frame width in pixels.
    pub width: u16,
    /// Declared frame height in pixels.
    pub height: u16,
}

/// Container-level MP4 facts persisted into `metadata.json`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Mp4Facts {
    /// `ftyp` brands.
    pub brands: Mp4Brands,
    /// `mvhd` movie timescale, when present.
    pub movie_timescale: Option<u32>,
    /// `mvhd` movie duration in movie-timescale ticks, when present.
    pub movie_duration_ticks: Option<u64>,
    /// Per-track declared facts, in `trak` order.
    pub tracks: Vec<Mp4Track>,
    /// Boxes skipped without interpretation, recorded as
    /// `parent/child` fourcc paths (e.g. `moov/trak/edts/elst` is
    /// *not* here — `elst` presence is a declared fact).
    pub skipped_boxes: Vec<String>,
}

/// Demuxed MP4: optional decoded PCM plus declared container facts and
/// per-track sample timing.
#[derive(Debug, Clone)]
pub struct Mp4Media {
    /// Persisted container facts (`mp4` section of `metadata.json`).
    pub facts: Mp4Facts,
    /// PCM extracted from the first PCM-decodable `soun` sample entry,
    /// `None` when every audio track is compressed or there is no
    /// audio track at all. `None` is not an error — it is the partial
    /// ingest case and is paired with `integrity` observations.
    pub audio: Option<DecodedAudio>,
    /// Sample-table timing for each `soun`/`vide` track: decode-order
    /// DTS (`sample_table`) plus presentation-order PTS
    /// (`sample_table_pts`) when a `ctts` exists.
    pub timing: Vec<StreamTiming>,
    /// `decode/input unavailable` observations for tracks whose codec
    /// we cannot decode. These are facts, not stage failures.
    pub integrity: Vec<IntegrityObservation>,
}

/// Parsed sound sample entry.
struct SoundEntry {
    format: [u8; 4],
    channels: u16,
    bits: u16,
    sample_rate: u32,
    /// CoreAudio-style format flags for `lpcm` (v2 entries).
    lpcm_flags: Option<u32>,
    /// `pcmC` (ISO 23003-5) declared flags + sample size.
    pcmc: Option<(u8, u8)>,
}

/// Parsed sample table for one track.
#[derive(Default)]
struct SampleTable {
    /// Decode-order DTS series in timescale ticks (from `stts`).
    dts: Vec<i64>,
    /// Composition offsets in decode order (from `ctts`).
    cts_offsets: Option<Vec<i64>>,
    /// Per-sample sizes in bytes (from `stsz`).
    sizes: Vec<u32>,
    /// Chunk file offsets (from `stco`/`co64`).
    chunk_offsets: Vec<u64>,
    /// `stsc` runs: (first_chunk, samples_per_chunk, sd_index).
    stsc: Vec<(u32, u32, u32)>,
}

/// Decode the mdhd packed 15-bit language field (three 5-bit letters
/// biased by 0x60). `0` maps to `und`.
fn mdhd_language(value: u16) -> String {
    if value == 0 {
        return "und".into();
    }
    let chars: Vec<u8> = [
        (((value >> 10) & 0x1F) + 0x60) as u8,
        (((value >> 5) & 0x1F) + 0x60) as u8,
        ((value & 0x1F) + 0x60) as u8,
    ]
    .into_iter()
    .filter(|c| c.is_ascii_lowercase())
    .collect();
    if chars.len() == 3 {
        String::from_utf8(chars).unwrap_or_else(|_| "und".into())
    } else {
        "und".into()
    }
}

fn parse_ftyp(body: &[u8]) -> Result<Mp4Brands, MediaError> {
    if body.len() < 8 || !body.len().is_multiple_of(4) {
        return Err(MediaError::malformed("ftyp body is not brand-aligned"));
    }
    let major = fourcc_text(&body[0..4].try_into().unwrap());
    let minor_version = be_u32(body, 4)?;
    let compatible_brands = body[8..]
        .as_chunks::<4>()
        .0
        .iter()
        .map(fourcc_text)
        .collect();
    Ok(Mp4Brands {
        major_brand: major,
        minor_version,
        compatible_brands,
    })
}

/// Parse `mdhd` (v0/v1): timescale, duration, language.
fn parse_mdhd(body: &[u8]) -> Result<(u32, u64, String), MediaError> {
    if body.len() < 24 {
        return Err(MediaError::malformed("mdhd shorter than version-0 body"));
    }
    let version = body[0];
    let (timescale, duration, lang_at) = match version {
        0 => (be_u32(body, 12)?, u64::from(be_u32(body, 16)?), 20),
        1 => {
            if body.len() < 34 {
                return Err(MediaError::malformed("mdhd shorter than version-1 body"));
            }
            (be_u32(body, 20)?, be_u64(body, 24)?, 32)
        }
        v => {
            return Err(MediaError::malformed(format!(
                "unsupported mdhd version {v}"
            )));
        }
    };
    if timescale == 0 {
        return Err(MediaError::malformed("mdhd declares timescale 0"));
    }
    Ok((timescale, duration, mdhd_language(be_u16(body, lang_at)?)))
}

/// Parse `hdlr`: handler type fourcc.
fn parse_hdlr(body: &[u8]) -> Result<[u8; 4], MediaError> {
    if body.len() < 12 {
        return Err(MediaError::malformed("hdlr shorter than handler type"));
    }
    Ok(body[8..12].try_into().unwrap())
}

/// Parse `tkhd` (v0/v1): track id + movie-timescale duration.
fn parse_tkhd(body: &[u8]) -> Result<(u32, u64), MediaError> {
    if body.len() < 24 {
        return Err(MediaError::malformed("tkhd shorter than version-0 body"));
    }
    match body[0] {
        0 => Ok((be_u32(body, 12)?, u64::from(be_u32(body, 20)?))),
        1 => {
            if body.len() < 36 {
                return Err(MediaError::malformed("tkhd shorter than version-1 body"));
            }
            Ok((be_u32(body, 20)?, be_u64(body, 28)?))
        }
        v => Err(MediaError::malformed(format!(
            "unsupported tkhd version {v}"
        ))),
    }
}

/// Parse `mvhd` (v0/v1): movie timescale + duration.
fn parse_mvhd(body: &[u8]) -> Result<(u32, u64), MediaError> {
    if body.len() < 20 {
        return Err(MediaError::malformed("mvhd shorter than version-0 body"));
    }
    match body[0] {
        0 => Ok((be_u32(body, 12)?, u64::from(be_u32(body, 16)?))),
        1 => {
            if body.len() < 32 {
                return Err(MediaError::malformed("mvhd shorter than version-1 body"));
            }
            Ok((be_u32(body, 20)?, be_u64(body, 24)?))
        }
        v => Err(MediaError::malformed(format!(
            "unsupported mvhd version {v}"
        ))),
    }
}

/// `stts`/`ctts` run table: `entry_count` pairs of (count, value).
fn parse_run_table(body: &[u8], signed: bool, name: &str) -> Result<Vec<(u32, i64)>, MediaError> {
    if body.len() < 8 {
        return Err(MediaError::malformed(format!(
            "{name} shorter than entry count"
        )));
    }
    let count = be_u32(body, 4)? as usize;
    if body.len() < 8 + count * 8 {
        return Err(MediaError::malformed(format!(
            "{name} entry list truncated"
        )));
    }
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let at = 8 + i * 8;
        let n = be_u32(body, at)?;
        let v = if signed {
            i64::from(be_i32(body, at + 4)?)
        } else {
            i64::from(be_u32(body, at + 4)?)
        };
        out.push((n, v));
    }
    Ok(out)
}

/// Expand `stts` runs into the per-sample DTS series.
fn dts_series(stts: &[(u32, i64)]) -> Result<Vec<i64>, MediaError> {
    let total: usize = stts.iter().map(|e| e.0 as usize).sum();
    let mut dts = Vec::with_capacity(total);
    let mut t = 0i64;
    for &(count, delta) in stts {
        for _ in 0..count {
            dts.push(t);
            t = t
                .checked_add(delta)
                .ok_or_else(|| MediaError::malformed("stts dts overflows i64"))?;
        }
    }
    Ok(dts)
}

/// Expand `ctts` runs into per-sample composition offsets.
fn cts_series(ctts: &[(u32, i64)]) -> Vec<i64> {
    let mut out = Vec::new();
    for &(count, v) in ctts {
        out.extend(std::iter::repeat_n(v, count as usize));
    }
    out
}

/// Parse `stsz`: uniform sample size or per-sample sizes.
/// `file_len` bounds the declared count: every real sample needs at
/// least one payload byte, so a table cannot name more samples than
/// the file holds.
fn parse_stsz(body: &[u8], file_len: usize) -> Result<Vec<u32>, MediaError> {
    if body.len() < 12 {
        return Err(MediaError::malformed("stsz shorter than header"));
    }
    let uniform = be_u32(body, 4)?;
    let count = be_u32(body, 8)? as usize;
    if count > file_len {
        return Err(MediaError::malformed(
            "stsz declares more samples than bytes in file",
        ));
    }
    if uniform != 0 {
        return Ok(vec![uniform; count]);
    }
    if body.len() < 12 + count * 4 {
        return Err(MediaError::malformed("stsz sample size list truncated"));
    }
    (0..count).map(|i| be_u32(body, 12 + i * 4)).collect()
}

/// Parse `stsc`: (first_chunk, samples_per_chunk, sample_description_index).
fn parse_stsc(body: &[u8]) -> Result<Vec<(u32, u32, u32)>, MediaError> {
    if body.len() < 8 {
        return Err(MediaError::malformed("stsc shorter than entry count"));
    }
    let count = be_u32(body, 4)? as usize;
    if body.len() < 8 + count * 12 {
        return Err(MediaError::malformed("stsc entry list truncated"));
    }
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let at = 8 + i * 12;
        out.push((
            be_u32(body, at)?,
            be_u32(body, at + 4)?,
            be_u32(body, at + 8)?,
        ));
    }
    Ok(out)
}

/// Parse `stss`: sync sample indices.
fn parse_stss(body: &[u8]) -> Result<Vec<u32>, MediaError> {
    if body.len() < 8 {
        return Err(MediaError::malformed("stss shorter than entry count"));
    }
    let count = be_u32(body, 4)? as usize;
    if body.len() < 8 + count * 4 {
        return Err(MediaError::malformed("stss entry list truncated"));
    }
    (0..count).map(|i| be_u32(body, 8 + i * 4)).collect()
}

/// Parse `stco` (u32) / `co64` (u64) chunk offset tables.
fn parse_chunk_offsets(body: &[u8], wide: bool) -> Result<Vec<u64>, MediaError> {
    if body.len() < 8 {
        return Err(MediaError::malformed("stco/co64 shorter than entry count"));
    }
    let count = be_u32(body, 4)? as usize;
    let width = if wide { 8 } else { 4 };
    if body.len() < 8 + count * width {
        return Err(MediaError::malformed("stco/co64 offset list truncated"));
    }
    (0..count)
        .map(|i| {
            if wide {
                be_u64(body, 8 + i * 8)
            } else {
                Ok(u64::from(be_u32(body, 8 + i * 4)?))
            }
        })
        .collect()
}

/// Per-chunk sample layout: for each chunk, its sample-description
/// index and the byte ranges of its samples (as offsets into the file).
struct ChunkMap {
    /// `(file_offset, len, sd_index)` per sample in decode order.
    samples: Vec<(u64, u32, u32)>,
}

/// Expand `stsc` into a per-chunk (samples_per_chunk, sd_index) vector.
fn chunk_samples(stsc: &[(u32, u32, u32)], chunks: usize) -> Result<Vec<(u32, u32)>, MediaError> {
    if stsc.is_empty() {
        return Err(MediaError::malformed("stsc has no entries"));
    }
    if stsc[0].0 != 1 {
        return Err(MediaError::malformed(
            "stsc first entry must start at chunk 1",
        ));
    }
    let mut out = Vec::with_capacity(chunks);
    for (i, &(first, spc, sd)) in stsc.iter().enumerate() {
        let next_first = stsc.get(i + 1).map(|e| e.0).unwrap_or(chunks as u32 + 1);
        if first == 0 || next_first <= first {
            return Err(MediaError::malformed(
                "stsc first_chunk values are not ascending",
            ));
        }
        for _ in first..next_first.min(chunks as u32 + 1) {
            out.push((spc, sd));
        }
    }
    if out.len() != chunks {
        return Err(MediaError::malformed(
            "stsc does not cover all chunk offsets",
        ));
    }
    Ok(out)
}

/// Build the sample → file-offset map from the sample table.
fn build_chunk_map(table: &SampleTable) -> Result<ChunkMap, MediaError> {
    let per_chunk = chunk_samples(&table.stsc, table.chunk_offsets.len())?;
    let mut samples = Vec::with_capacity(table.sizes.len());
    let mut idx = 0usize;
    for (chunk_i, &(spc, sd)) in per_chunk.iter().enumerate() {
        let mut offset = table.chunk_offsets[chunk_i];
        for _ in 0..spc {
            let Some(&size) = table.sizes.get(idx) else {
                return Err(MediaError::malformed(
                    "stsc declares more samples than stsz",
                ));
            };
            samples.push((offset, size, sd));
            offset = offset
                .checked_add(u64::from(size))
                .ok_or_else(|| MediaError::malformed("chunk offset overflows"))?;
            idx += 1;
        }
    }
    if idx != table.sizes.len() {
        return Err(MediaError::malformed(
            "stsz declares more samples than stsc maps",
        ));
    }
    Ok(ChunkMap { samples })
}

/// Parse a sound sample entry body (fields after the 8-byte
/// size/format header): reserved(6)+dref(2)+version(2)+rev(2)+
/// vendor(4)+channels(2)+samplesize(2)+compressionid(2)+packetsize(2)+
/// samplerate(4). v1 adds 16 bytes, v2 a 36-byte structure carrying the
/// lpcm format flags. `ipcm`/`fpcm` entries carry a `pcmC` child box.
fn parse_sound_entry(body: &[u8], format: [u8; 4]) -> Result<SoundEntry, MediaError> {
    if body.len() < 28 {
        return Err(MediaError::malformed(
            "sound sample entry shorter than base fields",
        ));
    }
    let version = be_u16(body, 8)?;
    let mut entry = SoundEntry {
        format,
        channels: be_u16(body, 16)?,
        bits: be_u16(body, 18)?,
        sample_rate: be_u32(body, 24)? >> 16,
        lpcm_flags: None,
        pcmc: None,
    };
    let children_at = match version {
        0 => 28,
        1 => 44,
        2 => {
            if body.len() < 64 {
                return Err(MediaError::malformed(
                    "version-2 sound sample entry truncated",
                ));
            }
            let rate = be_f64(body, 32)?;
            if rate.is_finite() && rate >= 0.0 {
                entry.sample_rate = rate as u32;
            }
            entry.channels = be_u32(body, 40)? as u16; // numAudioChannels
            entry.bits = be_u32(body, 48)? as u16; // bitsPerChannel
            entry.lpcm_flags = Some(be_u32(body, 52)?);
            64
        }
        v => {
            return Err(MediaError::malformed(format!(
                "unsupported sound sample entry version {v}"
            )));
        }
    };
    // Child boxes (pcmC for ISO 23003-5 ipcm/fpcm, esds, wave, ...).
    if body.len() > children_at {
        for h in child_boxes(body, children_at, body.len(), "sound sample entry")? {
            if &h.tag == b"pcmC" {
                let b = &body[h.body_start..h.body_end];
                // ISO/IEC 23003-5 pcmC: FullBox header (4) +
                // format_flags (1) + PCM_sample_size (1) = 6 bytes.
                if b.len() >= 6 {
                    entry.pcmc = Some((b[4], b[5]));
                }
            }
        }
    }
    Ok(entry)
}

/// PCM byte layout derived from a sound sample entry.
struct PcmLayout {
    float: bool,
    signed: bool,
    big_endian: bool,
    bits: u16,
    channels: u16,
    sample_rate: u32,
}

/// CoreAudio lpcm flag bits (QT sound sample description v2
/// `formatSpecificFlags`, also used by `pcmC`-style layouts).
const LPCM_FLOAT: u32 = 1;
const LPCM_BIG_ENDIAN: u32 = 2;
const LPCM_SIGNED: u32 = 4;
const LPCM_PACKED: u32 = 8;
const LPCM_NON_INTERLEAVED: u32 = 32;

/// Classify a sound sample entry as decodable PCM. Returns `None` for
/// compressed or undecodable layouts — an observation, not an error.
fn pcm_layout(entry: &SoundEntry) -> Option<PcmLayout> {
    let base = |float, signed, big_endian, bits| {
        Some(PcmLayout {
            float,
            signed,
            big_endian,
            bits,
            channels: entry.channels,
            sample_rate: entry.sample_rate,
        })
    };
    match &entry.format {
        // QuickTime legacy PCM fourccs.
        b"sowt" => base(false, true, false, entry.bits),
        b"twos" => base(false, true, true, entry.bits),
        b"in24" => base(false, true, true, 24),
        b"in32" => base(false, true, true, 32),
        b"fl32" => base(true, true, true, 32),
        b"fl64" => base(true, true, true, 64),
        b"raw " => base(false, false, false, 8),
        // ISO/IEC 23003-5 uncompressed audio: pcmC carries layout.
        b"ipcm" => {
            // pcmC format_flags bit0: 1 = little-endian, 0 = big-endian.
            let (flags, size) = entry.pcmc?;
            base(false, true, flags & 1 == 0, u16::from(size))
        }
        b"fpcm" => {
            let (flags, size) = entry.pcmc?;
            base(true, true, flags & 1 == 0, u16::from(size))
        }
        b"lpcm" => {
            // Flags from a v2 entry; classic lpcm defaults to
            // big-endian signed packed integer.
            let f = entry
                .lpcm_flags
                .unwrap_or(LPCM_BIG_ENDIAN | LPCM_SIGNED | LPCM_PACKED);
            // Unpacked or non-interleaved lpcm needs repacking we do
            // not implement — record as undecodable, not malformed.
            if f & LPCM_PACKED == 0 || f & LPCM_NON_INTERLEAVED != 0 {
                return None;
            }
            base(
                f & LPCM_FLOAT != 0,
                f & LPCM_SIGNED != 0,
                f & LPCM_BIG_ENDIAN != 0,
                entry.bits,
            )
        }
        _ => None,
    }
}

/// Decode raw PCM bytes to integer-scale `f64` under `layout`, keeping
/// the WAV pre-normalization contract: unsigned input is offset to a
/// signed integer scale (u8 → −128..127 like WAV `PcmU8`; wider
/// unsigned input maps onto the signed format of the same width so
/// `format_scale` still normalizes correctly).
fn decode_pcm_bytes(
    layout: &PcmLayout,
    data: &[u8],
) -> Result<(Vec<f64>, SampleFormat), MediaError> {
    let width = match (layout.float, layout.bits) {
        (true, 32) => 4,
        (true, 64) => 8,
        (false, 8) => 1,
        (false, 16) => 2,
        (false, 24) => 3,
        (false, 32) => 4,
        (float, bits) => {
            return Err(MediaError::input_unavailable(format!(
                "unsupported PCM width {}-bit float={float}",
                bits
            )));
        }
    };
    if !data.len().is_multiple_of(width) {
        return Err(MediaError::input_unavailable(
            "PCM sample data ends mid-sample",
        ));
    }
    let signed_int = |raw: u64, bits: u16| -> f64 {
        let shift = 64 - bits;
        ((raw << shift) as i64 >> shift) as f64
    };
    let unsigned_int = |raw: u64, bits: u16| -> f64 { raw as f64 - (1u64 << (bits - 1)) as f64 };
    let mut samples = Vec::with_capacity(data.len() / width);
    for s in data.chunks_exact(width) {
        let v = if layout.float {
            if width == 4 {
                let b: [u8; 4] = s.try_into().unwrap();
                f64::from(if layout.big_endian {
                    f32::from_be_bytes(b)
                } else {
                    f32::from_le_bytes(b)
                })
            } else {
                let b: [u8; 8] = s.try_into().unwrap();
                if layout.big_endian {
                    f64::from_be_bytes(b)
                } else {
                    f64::from_le_bytes(b)
                }
            }
        } else {
            let raw = if layout.big_endian {
                s.iter().fold(0u64, |v, &b| (v << 8) | u64::from(b))
            } else {
                s.iter().rev().fold(0u64, |v, &b| (v << 8) | u64::from(b))
            };
            if layout.signed {
                signed_int(raw, layout.bits)
            } else {
                unsigned_int(raw, layout.bits)
            }
        };
        samples.push(v);
    }
    let format = match (layout.float, layout.signed, layout.bits) {
        (true, _, 32) => SampleFormat::Float32,
        (true, _, 64) => SampleFormat::Float64,
        // 8-bit unsigned keeps the WAV PcmU8 label; wider unsigned was
        // offset onto the signed scale above and reports as signed.
        (false, false, 8) => SampleFormat::PcmU8,
        (false, true, 8) => SampleFormat::PcmS8,
        (false, _, 16) => SampleFormat::PcmS16,
        (false, _, 24) => SampleFormat::PcmS24,
        (false, _, 32) => SampleFormat::PcmS32,
        _ => SampleFormat::Other,
    };
    Ok((samples, format))
}

/// Codec name for provenance, matching the WAV `pcm_*` convention.
fn pcm_codec_name(layout: &PcmLayout, format: SampleFormat) -> String {
    let endian = if layout.big_endian { "be" } else { "le" };
    match format {
        SampleFormat::PcmU8 => "pcm_u8".into(),
        SampleFormat::PcmS8 => "pcm_s8".into(),
        SampleFormat::Float32 => format!("pcm_f32{endian}"),
        SampleFormat::Float64 => format!("pcm_f64{endian}"),
        SampleFormat::PcmS16 => format!("pcm_s16{endian}"),
        SampleFormat::PcmS24 => format!("pcm_s24{endian}"),
        SampleFormat::PcmS32 => format!("pcm_s32{endian}"),
        SampleFormat::Other => format!("pcm_{}", layout.bits),
    }
}

/// Extract the bytes of every sample mapped to `sd_index`, in decode
/// order, concatenated — the PCM sample stream.
fn extract_sample_bytes(data: &[u8], map: &ChunkMap, sd_index: u32) -> Result<Vec<u8>, MediaError> {
    let mut out = Vec::new();
    let mut total: u64 = 0;
    for &(offset, size, sd) in &map.samples {
        if sd != sd_index {
            continue;
        }
        let end = offset
            .checked_add(u64::from(size))
            .ok_or_else(|| MediaError::malformed("sample offset overflows"))?;
        // Real PCM samples are disjoint ranges of the same file, so a
        // declared total above the file length can only come from
        // overlapping/corrupt tables — reject before the output
        // buffer grows without bound.
        total = total
            .checked_add(u64::from(size))
            .ok_or_else(|| MediaError::malformed("sample byte total overflows"))?;
        if total > data.len() as u64 {
            return Err(MediaError::malformed(
                "declared sample bytes exceed file length",
            ));
        }
        let body = data
            .get(offset as usize..end as usize)
            .ok_or_else(|| MediaError::input_unavailable("sample extends past end of file"))?;
        out.extend_from_slice(body);
    }
    Ok(out)
}

/// Parse one `trak`'s children into track facts + sample table.
struct ParsedTrak {
    track: Mp4Track,
    handler: [u8; 4],
    sound: Vec<SoundEntry>,
    /// `stsd` entry fourccs in declaration order.
    entry_formats: Vec<[u8; 4]>,
    table: SampleTable,
    has_table: bool,
}

fn parse_stsd(body: &[u8], trak: &mut ParsedTrak) -> Result<(), MediaError> {
    if body.len() < 8 {
        return Err(MediaError::malformed("stsd shorter than entry count"));
    }
    let count = be_u32(body, 4)? as usize;
    for h in child_boxes(body, 8, body.len(), "stsd")? {
        let ebody = &body[h.body_start..h.body_end];
        trak.entry_formats.push(h.tag);
        match &trak.handler {
            b"soun" => trak.sound.push(parse_sound_entry(ebody, h.tag)?),
            b"vide" if ebody.len() >= 28 => {
                // VisualSampleEntry: width/height at body offset 24/26.
                trak.track.video = Some(Mp4VideoInfo {
                    width: be_u16(ebody, 24)?,
                    height: be_u16(ebody, 26)?,
                });
            }
            _ => {}
        }
    }
    if trak.entry_formats.len() != count {
        return Err(MediaError::malformed(
            "stsd entry count does not match its boxes",
        ));
    }
    trak.track.codec = trak.entry_formats.first().map(fourcc_text);
    Ok(())
}

fn parse_stbl(
    data: &[u8],
    body_start: usize,
    body_end: usize,
    trak: &mut ParsedTrak,
    skipped: &mut Vec<String>,
) -> Result<(), MediaError> {
    let mut stsd_found = false;
    let mut stts = None;
    let mut ctts = None;
    let mut stss = None;
    let mut stsz = None;
    let mut stsc = None;
    let mut offsets = None;
    for h in child_boxes(data, body_start, body_end, "stbl")? {
        let b = &data[h.body_start..h.body_end];
        match &h.tag {
            b"stsd" => {
                stsd_found = true;
                parse_stsd(b, trak)?;
            }
            b"stts" => stts = Some(parse_run_table(b, false, "stts")?),
            b"ctts" => {
                let signed = b.first().copied() == Some(1);
                ctts = Some(parse_run_table(b, signed, "ctts")?);
            }
            b"stss" => stss = Some(parse_stss(b)?),
            b"stsz" => stsz = Some(parse_stsz(b, data.len())?),
            b"stsc" => stsc = Some(parse_stsc(b)?),
            b"stco" => offsets = Some(parse_chunk_offsets(b, false)?),
            b"co64" => offsets = Some(parse_chunk_offsets(b, true)?),
            other => skipped.push(format!("moov/trak/mdia/minf/stbl/{}", fourcc_text(other))),
        }
    }
    if !stsd_found {
        return Err(MediaError::malformed("stbl is missing stsd"));
    }
    trak.track.sync_samples = stss;
    // The locating tables are all-or-nothing: any one without the
    // others means samples cannot be addressed — structural corruption.
    let have = [
        stts.is_some(),
        stsz.is_some(),
        stsc.is_some(),
        offsets.is_some(),
    ];
    if have.iter().any(|h| *h) && have.iter().any(|h| !*h) {
        return Err(MediaError::malformed(
            "incomplete sample table (stts/stsz/stsc/stco must appear together)",
        ));
    }
    if let (Some(stts), Some(stsz), Some(stsc), Some(offsets)) = (stts, stsz, stsc, offsets) {
        // Run tables expand before use; a declared total above the
        // file length can only come from a corrupt box.
        let declared: u64 = stts.iter().map(|e| u64::from(e.0)).sum();
        let declared_cts: u64 = ctts
            .as_ref()
            .map_or(0, |c| c.iter().map(|e| u64::from(e.0)).sum());
        if declared > data.len() as u64 || declared_cts > data.len() as u64 {
            return Err(MediaError::malformed(
                "stts/ctts declares more samples than bytes in file",
            ));
        }
        let table = SampleTable {
            dts: dts_series(&stts)?,
            cts_offsets: ctts.map(|c| cts_series(&c)),
            sizes: stsz,
            chunk_offsets: offsets,
            stsc,
        };
        if table.dts.len() != table.sizes.len() {
            return Err(MediaError::malformed(
                "stts sample count does not match stsz",
            ));
        }
        if table
            .cts_offsets
            .as_ref()
            .is_some_and(|cts| cts.len() != table.sizes.len())
        {
            return Err(MediaError::malformed(
                "ctts sample count does not match stsz",
            ));
        }
        if let Some(sync) = &trak.track.sync_samples {
            let n = table.sizes.len() as u32;
            if sync.iter().any(|&i| i == 0 || i > n) {
                return Err(MediaError::malformed("stss index outside sample range"));
            }
        }
        trak.track.sample_count = table.sizes.len() as u64;
        trak.table = table;
        trak.has_table = true;
    }
    Ok(())
}

fn parse_trak(
    data: &[u8],
    body_start: usize,
    body_end: usize,
    track_index: u32,
    skipped: &mut Vec<String>,
) -> Result<ParsedTrak, MediaError> {
    let mut trak = ParsedTrak {
        track: Mp4Track {
            track_index,
            track_id: None,
            handler: String::new(),
            codec: None,
            timescale: 0,
            duration_ticks: 0,
            language: "und".into(),
            movie_duration_ticks: None,
            sample_count: 0,
            sync_samples: None,
            has_edit_list: false,
            audio: None,
            video: None,
        },
        handler: [0; 4],
        sound: Vec::new(),
        entry_formats: Vec::new(),
        table: SampleTable::default(),
        has_table: false,
    };
    let mut mdhd_seen = false;
    let mut hdlr_seen = false;
    let mut stbl_seen = false;
    for h in child_boxes(data, body_start, body_end, "trak")? {
        match &h.tag {
            b"tkhd" => {
                let (id, dur) = parse_tkhd(&data[h.body_start..h.body_end])?;
                trak.track.track_id = Some(id);
                trak.track.movie_duration_ticks = Some(dur);
            }
            b"edts" => {
                // Edit lists are recorded, not interpreted.
                for c in child_boxes(data, h.body_start, h.body_end, "edts")? {
                    if &c.tag == b"elst" {
                        trak.track.has_edit_list = true;
                    } else {
                        skipped.push(format!("moov/trak/edts/{}", fourcc_text(&c.tag)));
                    }
                }
            }
            b"mdia" => {
                // Two passes: hdlr may legally follow minf, and stsd
                // parsing needs the handler type.
                let mdia_boxes = child_boxes(data, h.body_start, h.body_end, "mdia")?;
                for m in &mdia_boxes {
                    match &m.tag {
                        b"mdhd" => {
                            let (ts, dur, lang) = parse_mdhd(&data[m.body_start..m.body_end])?;
                            trak.track.timescale = ts;
                            trak.track.duration_ticks = dur;
                            trak.track.language = lang;
                            mdhd_seen = true;
                        }
                        b"hdlr" => {
                            trak.handler = parse_hdlr(&data[m.body_start..m.body_end])?;
                            trak.track.handler = fourcc_text(&trak.handler);
                            hdlr_seen = true;
                        }
                        _ => {}
                    }
                }
                for m in &mdia_boxes {
                    match &m.tag {
                        b"mdhd" | b"hdlr" => {}
                        b"minf" => {
                            for f in child_boxes(data, m.body_start, m.body_end, "minf")? {
                                if &f.tag == b"stbl" {
                                    stbl_seen = true;
                                    parse_stbl(data, f.body_start, f.body_end, &mut trak, skipped)?;
                                } else {
                                    skipped.push(format!(
                                        "moov/trak/mdia/minf/{}",
                                        fourcc_text(&f.tag)
                                    ));
                                }
                            }
                        }
                        other => skipped.push(format!("moov/trak/mdia/{}", fourcc_text(other))),
                    }
                }
            }
            other => skipped.push(format!("moov/trak/{}", fourcc_text(other))),
        }
    }
    if !mdhd_seen {
        return Err(MediaError::malformed("trak mdia is missing mdhd"));
    }
    if !hdlr_seen {
        return Err(MediaError::malformed("trak mdia is missing hdlr"));
    }
    if !stbl_seen {
        return Err(MediaError::malformed("trak minf is missing stbl"));
    }
    if let Some(sound) = trak.sound.first() {
        trak.track.audio = Some(Mp4SoundInfo {
            channels: sound.channels,
            bits_per_sample: sound.bits,
            sample_rate: sound.sample_rate,
        });
    }
    Ok(trak)
}

/// Decode an ISO-BMFF / MP4 input: container facts, per-track sample
/// timing, and PCM from the first PCM-decodable `soun` entry.
///
/// Compressed audio is not a failure: `audio` is `None` and each
/// undecodable `soun` track contributes a `decode/input unavailable`
/// integrity observation. Structural corruption stays `Malformed`;
/// fragmented MP4 is `InputUnavailable` (explicitly unsupported).
pub fn decode_mp4(data: &[u8]) -> Result<Mp4Media, MediaError> {
    if data.len() < 8 || &data[4..8] != b"ftyp" {
        return Err(MediaError::input_unavailable(
            "not an ISO-BMFF file (no leading ftyp box)",
        ));
    }
    let mut brands = None;
    let mut movie = None;
    let mut traks = Vec::new();
    let mut skipped = Vec::new();
    let mut moov_seen = false;
    let mut at = 0usize;
    while at < data.len() {
        let h = box_header(data, at, data.len(), "file")?;
        at = h.body_end;
        match &h.tag {
            b"ftyp" => {
                if brands.is_some() {
                    return Err(MediaError::malformed("duplicate ftyp box"));
                }
                brands = Some(parse_ftyp(&data[h.body_start..h.body_end])?);
            }
            b"moof" | b"mfra" | b"styp" => {
                return Err(MediaError::input_unavailable(
                    "fragmented MP4 is not supported",
                ));
            }
            b"moov" => {
                if moov_seen {
                    return Err(MediaError::malformed("duplicate moov box"));
                }
                moov_seen = true;
                for c in child_boxes(data, h.body_start, h.body_end, "moov")? {
                    match &c.tag {
                        b"mvhd" => {
                            let (ts, dur) = parse_mvhd(&data[c.body_start..c.body_end])?;
                            movie = Some((ts, dur));
                        }
                        b"trak" => traks.push(parse_trak(
                            data,
                            c.body_start,
                            c.body_end,
                            traks.len() as u32,
                            &mut skipped,
                        )?),
                        other => skipped.push(format!("moov/{}", fourcc_text(other))),
                    }
                }
            }
            // Media data, padding, and anything else: not interpreted.
            other => skipped.push(fourcc_text(other)),
        }
    }
    if !moov_seen {
        return Err(MediaError::malformed("missing moov box"));
    }
    if traks.is_empty() {
        return Err(MediaError::malformed("moov has no trak boxes"));
    }
    let brands = brands.ok_or_else(|| MediaError::malformed("missing ftyp box"))?;

    // Timing: decode-order DTS is the observed series; ctts-derived
    // PTS is sorted into presentation order under its own label so a
    // B-frame reorder never reaches observe_timing's monotonicity
    // contract.
    let mut timing = Vec::new();
    for trak in &traks {
        let clock = match &trak.handler {
            b"soun" => ClockKind::Audio,
            b"vide" => ClockKind::Video,
            _ => continue,
        };
        if !trak.has_table {
            continue;
        }
        let dts = &trak.table.dts;
        timing.push(StreamTiming {
            stream: trak.track.track_index,
            stream_type: trak.track.handler.clone(),
            timestamp_origin: TimestampOrigin::SampleTable,
            start_tick: 0,
            stream_timestamps: dts.clone(),
            observation: observe_timing(clock, u64::from(trak.track.timescale), dts, None),
        });
        if let Some(offsets) = &trak.table.cts_offsets {
            let mut pts: Vec<i64> = dts
                .iter()
                .zip(offsets.iter())
                .map(|(d, o)| {
                    d.checked_add(*o)
                        .ok_or_else(|| MediaError::malformed("ctts pts overflows i64"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            pts.sort_unstable();
            timing.push(StreamTiming {
                stream: trak.track.track_index,
                stream_type: trak.track.handler.clone(),
                timestamp_origin: TimestampOrigin::SampleTablePts,
                start_tick: 0,
                stream_timestamps: pts.clone(),
                observation: observe_timing(clock, u64::from(trak.track.timescale), &pts, None),
            });
        }
    }

    // PCM extraction: the first PCM-decodable soun sample entry wins
    // (matching AVI's first-auds rule). Undecodable soun entries are
    // integrity observations, never stage failures.
    let mut audio = None;
    let mut integrity = Vec::new();
    for (ti, trak) in traks.iter().enumerate() {
        if &trak.handler != b"soun" {
            continue;
        }
        if audio.is_some() {
            // Later soun tracks stay metadata+timing only, but the
            // fact they were not decoded is still an observation so
            // every undecoded entry leaves a record.
            if !trak.sound.is_empty() {
                integrity.push(IntegrityObservation {
                    kind: "decode/input unavailable".into(),
                    detail: serde_json::json!({
                        "track_index": ti as u32,
                        "handler": "soun",
                        "reason": "secondary soun track; first PCM-decodable track already selected",
                    }),
                });
            }
            continue;
        }
        let chosen = trak
            .sound
            .iter()
            .enumerate()
            .find_map(|(i, e)| pcm_layout(e).map(|l| (i as u32 + 1, e, l)));
        let Some((sd_index, entry, layout)) = chosen else {
            let codec = trak
                .entry_formats
                .first()
                .map(fourcc_text)
                .unwrap_or_default();
            integrity.push(IntegrityObservation {
                kind: "decode/input unavailable".into(),
                detail: serde_json::json!({
                    "track_index": ti as u32,
                    "handler": "soun",
                    "codec": codec,
                    "reason": "compressed or unsupported audio codec is not decoded",
                }),
            });
            continue;
        };
        if !trak.has_table {
            // PCM entry but nothing to extract — an observation, not
            // corruption (an empty track is legal).
            integrity.push(IntegrityObservation {
                kind: "decode/input unavailable".into(),
                detail: serde_json::json!({
                    "track_index": ti as u32,
                    "handler": "soun",
                    "codec": fourcc_text(&entry.format),
                    "reason": "PCM sample entry has no sample table",
                }),
            });
            continue;
        }
        if layout.channels == 0 || layout.sample_rate == 0 {
            integrity.push(IntegrityObservation {
                kind: "decode/input unavailable".into(),
                detail: serde_json::json!({
                    "track_index": ti as u32,
                    "handler": "soun",
                    "codec": fourcc_text(&entry.format),
                    "reason": "PCM entry declares zero channels or sample rate",
                }),
            });
            continue;
        }
        // A decodable layout can still fail at the data layer
        // (unsupported width, mid-sample truncation): that is a
        // decode/input observation, while structural table corruption
        // stays Malformed — the same boundary as compressed tracks.
        let extracted = build_chunk_map(&trak.table)
            .and_then(|map| extract_sample_bytes(data, &map, sd_index))
            .and_then(|bytes| decode_pcm_bytes(&layout, &bytes));
        match extracted {
            Ok((samples, sample_format)) => {
                audio = Some(DecodedAudio {
                    samples,
                    channels: layout.channels,
                    sample_rate: layout.sample_rate,
                    bits_per_sample: layout.bits.max(entry.bits),
                    sample_format,
                    codec: pcm_codec_name(&layout, sample_format),
                });
            }
            Err(MediaError::InputUnavailable { reason }) => {
                integrity.push(IntegrityObservation {
                    kind: "decode/input unavailable".into(),
                    detail: serde_json::json!({
                        "track_index": ti as u32,
                        "handler": "soun",
                        "codec": fourcc_text(&entry.format),
                        "reason": reason,
                    }),
                });
            }
            Err(e) => return Err(e),
        }
    }

    let tracks: Vec<Mp4Track> = traks.into_iter().map(|t| t.track).collect();
    Ok(Mp4Media {
        facts: Mp4Facts {
            brands,
            movie_timescale: movie.map(|m| m.0),
            movie_duration_ticks: movie.map(|m| m.1),
            tracks,
            skipped_boxes: skipped,
        },
        audio,
        timing,
        integrity,
    })
}
