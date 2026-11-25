//! MP4/AVI container property suite and Kani proof harness (Issue #256):
//! `decode_mp4`/`decode_avi`/`detect_format` never panic on arbitrary or
//! mutated bytes, `write_mp4`/`write_avi` output round-trips through the
//! decoders with the declared container facts intact, compressed or
//! missing audio stays an integrity observation instead of a demux
//! failure, and `detect_format` follows the documented signature rules.
//! The `#[cfg(kani)]` module holds bounded proofs that only `cargo kani`
//! compiles.

use std::panic::{AssertUnwindSafe, catch_unwind};

use al_media::decoder::UnavailableKind;
use al_media::{
    ClockKind, MediaFormat, SampleFormat, TimestampOrigin, VideoFeatureConfig, decode_avi,
    decode_mp4, detect_format,
};
use al_testkit::media::{AviSpec, AviStreamSpec, MoviPacket, pcm_s16le, write_avi};
use al_testkit::mp4::{Mp4EntrySpec, Mp4Spec, Mp4TableSpec, Mp4TrackSpec, mp4_language, write_mp4};
use proptest::prelude::*;

#[cfg(kani)]
mod kani_stub_models;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// `fourcc_text` semantics restated as the model: lossy text with
/// trailing NUL/space padding removed.
fn tag_text(tag: &[u8; 4]) -> String {
    String::from_utf8_lossy(tag)
        .trim_end_matches(['\0', ' '])
        .to_owned()
}

/// `detect_format`'s documented signature rules restated as the model:
/// `RIFF`+`WAVE`/`AVI ` form type wins, `ftyp` at offset 4 means MP4,
/// anything else — including shorter inputs — is `None`.
fn expected_detect(data: &[u8]) -> Option<MediaFormat> {
    if data.len() >= 12 && &data[0..4] == b"RIFF" {
        return match &data[8..12] {
            b"WAVE" => Some(MediaFormat::Wav),
            b"AVI " => Some(MediaFormat::Avi),
            _ => None,
        };
    }
    if data.len() >= 8 && &data[4..8] == b"ftyp" {
        return Some(MediaFormat::Mp4);
    }
    None
}

/// Decode-order DTS of a single uniform `stts` run.
fn expected_dts(count: u32, delta: u32) -> Vec<i64> {
    (0..count)
        .map(|i| i64::from(i) * i64::from(delta))
        .collect()
}

// ---------------------------------------------------------------------------
// MP4 spec generation
// ---------------------------------------------------------------------------

/// Printable fourccs for `ftyp` brands — recorded as text, so they must
/// round-trip through `fourcc_text` (no trailing NUL/space).
fn brand() -> impl Strategy<Value = [u8; 4]> {
    prop_oneof![
        Just(*b"isom"),
        Just(*b"mp41"),
        Just(*b"mp42"),
        Just(*b"iso2"),
        Just(*b"dash"),
        Just(*b"avc1"),
    ]
}

/// Invented tags for `skipped_boxes` coverage — none collide with a box
/// type the parser interprets.
fn skipped_tag() -> impl Strategy<Value = [u8; 4]> {
    prop_oneof![
        Just(*b"zzzz"),
        Just(*b"wxyz"),
        Just(*b"abcd"),
        Just(*b"efgh"),
        Just(*b"ijkl"),
        Just(*b"qrst"),
    ]
}

/// mdhd language codes that round-trip through the packed 15-bit field.
fn language() -> impl Strategy<Value = ([u8; 3], &'static str)> {
    prop_oneof![
        Just((*b"und", "und")),
        Just((*b"eng", "eng")),
        Just((*b"jpn", "jpn")),
        Just((*b"fra", "fra")),
    ]
}

/// The decode outcome the product contract promises for one generated
/// `soun` sample entry.
#[derive(Debug, Clone)]
enum ExpectAudio {
    /// PCM extracted into exactly these samples (native integer scale).
    Decoded {
        codec: &'static str,
        format: SampleFormat,
        bits_per_sample: u16,
        samples: Vec<f64>,
    },
    /// `audio` stays `None`; `integrity` gains a `decode_unavailable`
    /// observation whose `detail["kind"]` is this string
    /// (`"capability"` for undecodable/unsupported codecs, `"malformed"`
    /// for structurally bad declarations).
    Unavailable { kind: &'static str },
}

/// One generated `soun` entry: sample-description fields, the sample
/// table payload it carries, and the promised outcome.
#[derive(Debug, Clone)]
struct SounGen {
    codec: [u8; 4],
    version: u16,
    lpcm_flags: u32,
    pcmc: Option<(u8, u8)>,
    bits: u16,
    channels: u16,
    rate: u32,
    /// `stts` delta and sample count — for PCM entries one sample is
    /// one frame (`channels` x width bytes).
    delta: u32,
    count: u32,
    /// `stsz` per-sample bytes.
    sample_size: u32,
    /// Chunk payload; exactly `count * sample_size` bytes.
    payload: Vec<u8>,
    /// Expected `pcm_tracks` membership: the entry resolves to a
    /// supported PCM layout with nonzero channels/rate.
    pcm_track: bool,
    expect: ExpectAudio,
}

impl SounGen {
    /// The sample rate the decoder sees: v0/v1 entries store a 16.16
    /// fixed-point field whose high bits wrap, while v2 carries an f64.
    /// A rate that wraps to zero decodes as a zero-rate declaration —
    /// a malformed observation, not a decoded track.
    fn declared_rate(&self) -> u32 {
        if self.version == 2 {
            self.rate
        } else {
            self.rate & 0xFFFF
        }
    }
}

/// 16-bit signed PCM entries and the endianness they imply.
#[derive(Debug, Clone, Copy)]
enum Pcm16 {
    Sowt,
    Twos,
    LpcmLe,
    LpcmBe,
    IpcmLe,
    IpcmBe,
}

/// A decodable 16-bit `soun` entry; `samples` are interleaved frames
/// (`len == frames * channels`). `lpcm` v2 emits signed+packed flags
/// (the only layout the decoder accepts); `ipcm` carries a `pcmC` box.
fn soun_pcm16(kind: Pcm16, channels: u16, rate: u32, delta: u32, samples: &[i16]) -> SounGen {
    // CoreAudio lpcm flags: FLOAT=1, BIG_ENDIAN=2, SIGNED=4, PACKED=8.
    let (codec, version, lpcm_flags, pcmc, big_endian) = match kind {
        Pcm16::Sowt => (*b"sowt", 0, 0, None, false),
        Pcm16::Twos => (*b"twos", 0, 0, None, true),
        Pcm16::LpcmLe => (*b"lpcm", 2, 4 | 8, None, false),
        Pcm16::LpcmBe => (*b"lpcm", 2, 4 | 8 | 2, None, true),
        Pcm16::IpcmLe => (*b"ipcm", 0, 0, Some((1, 16)), false),
        Pcm16::IpcmBe => (*b"ipcm", 0, 0, Some((0, 16)), true),
    };
    let payload: Vec<u8> = samples
        .iter()
        .flat_map(|v| {
            if big_endian {
                v.to_be_bytes()
            } else {
                v.to_le_bytes()
            }
        })
        .collect();
    let frame = u32::from(channels) * 2;
    let declared = if version == 2 { rate } else { rate & 0xFFFF };
    let (pcm_track, expect) = if declared == 0 {
        // Wrapped-to-zero rate is a malformed declaration.
        (false, ExpectAudio::Unavailable { kind: "malformed" })
    } else {
        (
            true,
            ExpectAudio::Decoded {
                codec: if big_endian { "pcm_s16be" } else { "pcm_s16le" },
                format: SampleFormat::PcmS16,
                bits_per_sample: 16,
                samples: samples.iter().map(|&v| f64::from(v)).collect(),
            },
        )
    };
    SounGen {
        codec,
        version,
        lpcm_flags,
        pcmc,
        bits: 16,
        channels,
        rate,
        delta,
        count: samples.len() as u32 / u32::from(channels),
        sample_size: frame,
        payload,
        pcm_track,
        expect,
    }
}

/// `twos` 8-bit is big-endian signed; width-1 integer reads are
/// endian-neutral, so it exercises the signed-u8 decode arm.
fn soun_twos8(channels: u16, rate: u32, delta: u32, samples: &[i8]) -> SounGen {
    let frame = u32::from(channels);
    let declared = rate & 0xFFFF;
    let (pcm_track, expect) = if declared == 0 {
        (false, ExpectAudio::Unavailable { kind: "malformed" })
    } else {
        (
            true,
            ExpectAudio::Decoded {
                codec: "pcm_s8",
                format: SampleFormat::PcmS8,
                bits_per_sample: 8,
                samples: samples.iter().map(|&v| f64::from(v)).collect(),
            },
        )
    };
    SounGen {
        codec: *b"twos",
        version: 0,
        lpcm_flags: 0,
        pcmc: None,
        bits: 8,
        channels,
        rate,
        delta,
        count: samples.len() as u32 / frame,
        sample_size: frame,
        payload: samples.iter().map(|v| *v as u8).collect(),
        pcm_track,
        expect,
    }
}

/// `raw ` is unsigned 8-bit, offset onto the signed scale (-128..127).
fn soun_raw8(channels: u16, rate: u32, delta: u32, samples: &[u8]) -> SounGen {
    let frame = u32::from(channels);
    let declared = rate & 0xFFFF;
    let (pcm_track, expect) = if declared == 0 {
        (false, ExpectAudio::Unavailable { kind: "malformed" })
    } else {
        (
            true,
            ExpectAudio::Decoded {
                codec: "pcm_u8",
                format: SampleFormat::PcmU8,
                bits_per_sample: 8,
                samples: samples.iter().map(|&v| f64::from(v) - 128.0).collect(),
            },
        )
    };
    SounGen {
        codec: *b"raw ",
        version: 0,
        lpcm_flags: 0,
        pcmc: None,
        bits: 8,
        channels,
        rate,
        delta,
        count: samples.len() as u32 / frame,
        sample_size: frame,
        payload: samples.to_vec(),
        pcm_track,
        expect,
    }
}

/// A compressed fourcc — the entry resolves to no PCM layout, so it is
/// a capability observation and never a `pcm_tracks` member.
fn soun_compressed(
    codec: [u8; 4],
    channels: u16,
    rate: u32,
    delta: u32,
    count: u32,
    size: u32,
) -> SounGen {
    SounGen {
        codec,
        version: 0,
        lpcm_flags: 0,
        pcmc: None,
        bits: 16,
        channels,
        rate,
        delta,
        count,
        sample_size: size,
        payload: vec![0u8; (count * size) as usize],
        pcm_track: false,
        expect: ExpectAudio::Unavailable { kind: "capability" },
    }
}

/// `ipcm` without the `pcmC` child — the layout is unknown without the
/// ISO 23003-5 descriptor.
fn soun_ipcm_no_pcmc(channels: u16, rate: u32, delta: u32, frames: u32) -> SounGen {
    let frame = u32::from(channels) * 2;
    SounGen {
        codec: *b"ipcm",
        version: 0,
        lpcm_flags: 0,
        pcmc: None,
        bits: 16,
        channels,
        rate,
        delta,
        count: frames,
        sample_size: frame,
        payload: vec![0u8; (frames * frame) as usize],
        pcm_track: false,
        expect: ExpectAudio::Unavailable { kind: "capability" },
    }
}

/// `lpcm` v2 signed but *unpacked* — repacking non-interleaved PCM is
/// not implemented, so this is a capability failure.
fn soun_lpcm_unpacked(channels: u16, rate: u32, delta: u32, frames: u32) -> SounGen {
    let frame = u32::from(channels) * 2;
    SounGen {
        codec: *b"lpcm",
        version: 2,
        lpcm_flags: 4 | 2, // SIGNED | BIG_ENDIAN, no PACKED
        pcmc: None,
        bits: 16,
        channels,
        rate,
        delta,
        count: frames,
        sample_size: frame,
        payload: vec![0u8; (frames * frame) as usize],
        pcm_track: false,
        expect: ExpectAudio::Unavailable { kind: "capability" },
    }
}

/// `sowt` 12-bit — the layout resolves but matches no PCM width, so the
/// entry is skipped as a capability failure, not a `pcm_tracks` member.
fn soun_unsupported_width(channels: u16, rate: u32, delta: u32, samples: u32) -> SounGen {
    let size = samples * 2;
    SounGen {
        codec: *b"sowt",
        version: 0,
        lpcm_flags: 0,
        pcmc: None,
        bits: 12,
        channels,
        rate,
        delta,
        count: 1,
        sample_size: size,
        payload: vec![0u8; size as usize],
        pcm_track: false,
        expect: ExpectAudio::Unavailable { kind: "capability" },
    }
}

/// `sowt` whose payload is a whole number of samples but not of frames:
/// the layout is PCM-capable (a `pcm_tracks` member) and the data layer
/// fails mid-frame — a malformed observation.
fn soun_mid_frame(channels: u16, rate: u32, delta: u32, odd_samples: u32) -> SounGen {
    let size = odd_samples * 2;
    SounGen {
        codec: *b"sowt",
        version: 0,
        lpcm_flags: 0,
        pcmc: None,
        bits: 16,
        channels,
        rate,
        delta,
        count: 1,
        sample_size: size,
        payload: vec![0u8; size as usize],
        pcm_track: true,
        expect: ExpectAudio::Unavailable { kind: "malformed" },
    }
}

/// A `sowt` entry declaring zero channels or a zero sample rate — the
/// declaration itself is malformed.
fn soun_zero_param(channels: u16, rate: u32, delta: u32) -> SounGen {
    let frame = u32::from(channels.max(1)) * 2;
    SounGen {
        codec: *b"sowt",
        version: 0,
        lpcm_flags: 0,
        pcmc: None,
        bits: 16,
        channels,
        rate,
        delta,
        count: 1,
        sample_size: frame,
        payload: vec![0u8; frame as usize],
        pcm_track: false,
        expect: ExpectAudio::Unavailable { kind: "malformed" },
    }
}

/// Interleaved 16-bit PCM: channel count, rate, stts delta, and a
/// frame-aligned `i16` sample vector.
fn pcm_i16() -> impl Strategy<Value = (u16, u32, u32, Vec<i16>)> {
    (
        1u16..=4u16,
        1u32..=192_000u32,
        1u32..=8u32,
        0usize..=16usize,
    )
        .prop_flat_map(|(ch, rate, delta, frames)| {
            (
                Just(ch),
                Just(rate),
                Just(delta),
                prop::collection::vec(any::<i16>(), frames * usize::from(ch)),
            )
        })
}

/// Entries the contract says decode to PCM.
fn soun_decoded() -> impl Strategy<Value = SounGen> {
    prop_oneof![
        (
            pcm_i16(),
            prop_oneof![
                Just(Pcm16::Sowt),
                Just(Pcm16::Twos),
                Just(Pcm16::LpcmLe),
                Just(Pcm16::LpcmBe),
                Just(Pcm16::IpcmLe),
                Just(Pcm16::IpcmBe),
            ],
        )
            .prop_map(|((ch, rate, delta, samples), kind)| soun_pcm16(
                kind, ch, rate, delta, &samples
            )),
        (
            1u16..=4u16,
            1u32..=192_000u32,
            1u32..=8u32,
            0usize..=16usize
        )
            .prop_flat_map(|(ch, rate, delta, frames)| {
                (
                    Just(ch),
                    Just(rate),
                    Just(delta),
                    prop::collection::vec(any::<i8>(), frames * usize::from(ch)),
                )
            },)
            .prop_map(|(ch, rate, delta, samples)| soun_twos8(ch, rate, delta, &samples)),
        (
            1u16..=4u16,
            1u32..=192_000u32,
            1u32..=8u32,
            0usize..=16usize
        )
            .prop_flat_map(|(ch, rate, delta, frames)| {
                (
                    Just(ch),
                    Just(rate),
                    Just(delta),
                    prop::collection::vec(any::<u8>(), frames * usize::from(ch)),
                )
            },)
            .prop_map(|(ch, rate, delta, samples)| soun_raw8(ch, rate, delta, &samples)),
    ]
}

/// Entries the contract says stay undecoded — each maps to exactly one
/// `decode_unavailable` observation.
fn soun_unavailable() -> impl Strategy<Value = SounGen> {
    prop_oneof![
        (
            prop_oneof![
                Just(*b"mp4a"),
                Just(*b"ac-3"),
                Just(*b"alac"),
                Just(*b"ec-3")
            ],
            1u16..=4u16,
            1u32..=192_000u32,
            1u32..=8u32,
            1u32..=8u32,
            1u32..=32u32,
        )
            .prop_map(|(codec, ch, rate, delta, count, size)| {
                soun_compressed(codec, ch, rate, delta, count, size)
            }),
        (1u16..=4u16, 1u32..=192_000u32, 1u32..=8u32, 0u32..=16u32)
            .prop_map(|(ch, rate, delta, frames)| soun_ipcm_no_pcmc(ch, rate, delta, frames)),
        (1u16..=4u16, 1u32..=192_000u32, 1u32..=8u32, 0u32..=16u32)
            .prop_map(|(ch, rate, delta, frames)| soun_lpcm_unpacked(ch, rate, delta, frames)),
        (1u16..=4u16, 1u32..=192_000u32, 1u32..=8u32, 1u32..=8u32).prop_map(
            |(ch, rate, delta, samples)| soun_unsupported_width(ch, rate, delta, samples)
        ),
        // Odd sample counts can never complete a 2- or 4-channel frame.
        (
            prop_oneof![Just(2u16), Just(4u16)],
            1u32..=192_000u32,
            1u32..=8u32,
            0u32..=7u32
        )
            .prop_map(|(ch, rate, delta, half_frames)| {
                soun_mid_frame(ch, rate, delta, half_frames * 2 + 1)
            }),
        prop_oneof![
            (1u32..=192_000u32, 1u32..=8u32)
                .prop_map(|(rate, delta)| soun_zero_param(0, rate, delta)),
            (1u16..=4u16, 1u32..=8u32).prop_map(|(ch, delta)| soun_zero_param(ch, 0, delta)),
        ],
    ]
}

fn soun_gen() -> impl Strategy<Value = SounGen> {
    prop_oneof![3 => soun_decoded(), 2 => soun_unavailable()]
}

/// One generated `vide` entry — opaque payloads (the decoder never
/// reads video sample bytes); `ctts` exercises the PTS origin.
#[derive(Debug, Clone)]
struct VideGen {
    codec: [u8; 4],
    width: u16,
    height: u16,
    delta: u32,
    count: u32,
    ctts_offset: Option<u32>,
    stss: Option<Vec<u32>>,
}

fn vide_gen() -> impl Strategy<Value = VideGen> {
    (
        prop_oneof![
            Just(*b"avc1"),
            Just(*b"jpeg"),
            Just(*b"mp4v"),
            Just(*b"hvc1")
        ],
        1u16..=4096u16,
        1u16..=4096u16,
        1u32..=9000u32,
        0u32..=16u32,
    )
        .prop_flat_map(|(codec, width, height, delta, count)| {
            (
                Just(codec),
                Just(width),
                Just(height),
                Just(delta),
                Just(count),
                prop::option::of(0u32..=6000u32),
                prop::option::of(prop::collection::vec(1u32..=count.max(1), 0usize..=3usize)),
            )
        })
        .prop_map(|(codec, width, height, delta, count, ctts_offset, stss)| {
            // Sync indices outside the sample count are meaningless;
            // keep the list sorted, deduped, and in range.
            let stss = stss.and_then(|mut v| {
                v.retain(|&i| i <= count);
                v.sort_unstable();
                v.dedup();
                (!v.is_empty()).then_some(v)
            });
            VideGen {
                codec,
                width,
                height,
                delta,
                count,
                ctts_offset,
                stss,
            }
        })
}

/// One generated `trak`.
#[derive(Debug, Clone)]
struct TrackGen {
    track_id: u32,
    timescale: u32,
    duration: u64,
    language: [u8; 3],
    language_text: &'static str,
    kind: TrackKind,
    extra_boxes: Vec<([u8; 4], Vec<u8>)>,
}

#[derive(Debug, Clone)]
enum TrackKind {
    Soun(SounGen),
    Vide(VideGen),
}

fn track_gen() -> impl Strategy<Value = TrackGen> {
    (
        1u32..=1_000_000u32,
        1u32..=192_000u32,
        0u64..=1_000_000u64,
        language(),
        prop::collection::vec(
            (
                skipped_tag(),
                prop::collection::vec(any::<u8>(), 0usize..=8usize),
            ),
            0usize..=1usize,
        ),
        prop_oneof![
            soun_gen().prop_map(TrackKind::Soun),
            vide_gen().prop_map(TrackKind::Vide),
        ],
    )
        .prop_map(
            |(track_id, timescale, duration, (language, language_text), extra_boxes, kind)| {
                TrackGen {
                    track_id,
                    timescale,
                    duration,
                    language,
                    language_text,
                    kind,
                    extra_boxes,
                }
            },
        )
}

/// A whole generated MP4: brands, optional `mvhd`, unknown top-level
/// boxes, and 1–3 tracks.
#[derive(Debug, Clone)]
struct Mp4Case {
    major_brand: [u8; 4],
    compatible_brands: Vec<[u8; 4]>,
    movie_timescale: u32,
    extra_top_boxes: Vec<([u8; 4], Vec<u8>)>,
    tracks: Vec<TrackGen>,
}

fn mp4_case() -> impl Strategy<Value = Mp4Case> {
    (
        brand(),
        prop::collection::vec(brand(), 0usize..=3usize),
        0u32..=2000u32,
        prop::collection::vec(
            (
                skipped_tag(),
                prop::collection::vec(any::<u8>(), 0usize..=8usize),
            ),
            0usize..=2usize,
        ),
        prop::collection::vec(track_gen(), 1usize..=3usize),
    )
        .prop_map(
            |(major_brand, compatible_brands, movie_timescale, extra_top_boxes, tracks)| Mp4Case {
                major_brand,
                compatible_brands,
                movie_timescale,
                extra_top_boxes,
                tracks,
            },
        )
}

/// A track with fixed trivia for focused properties.
fn fixed_track(track_id: u32, kind: TrackKind) -> TrackGen {
    TrackGen {
        track_id,
        timescale: 48_000,
        duration: 0,
        language: *b"und",
        language_text: "und",
        kind,
        extra_boxes: vec![],
    }
}

/// Single-chunk sample table; `count == 0` emits the legal
/// empty-but-complete table (`stsz` sample count 0).
fn table(
    count: u32,
    size: u32,
    delta: u32,
    payload: Vec<u8>,
    ctts: Option<u32>,
    stss: Option<Vec<u32>>,
) -> Mp4TableSpec {
    if count == 0 {
        return Mp4TableSpec {
            stts: vec![],
            ctts: None,
            stss: None,
            stsc: vec![],
            chunks: vec![],
            sample_sizes: vec![],
        };
    }
    Mp4TableSpec {
        stts: vec![(count, delta)],
        ctts: ctts.map(|off| vec![(count, off)]),
        stss,
        stsc: vec![(1, count, 1)],
        chunks: vec![payload],
        sample_sizes: vec![size; count as usize],
    }
}

fn track_spec(t: &TrackGen) -> Mp4TrackSpec {
    let (handler, entry, table) = match &t.kind {
        TrackKind::Soun(s) => (
            *b"soun",
            Mp4EntrySpec::Sound {
                codec: s.codec,
                channels: s.channels,
                bits: s.bits,
                rate: s.rate,
                version: s.version,
                lpcm_flags: s.lpcm_flags,
                pcmc: s.pcmc,
            },
            table(
                s.count,
                s.sample_size,
                s.delta,
                s.payload.clone(),
                None,
                None,
            ),
        ),
        TrackKind::Vide(v) => (
            *b"vide",
            Mp4EntrySpec::Video {
                codec: v.codec,
                width: v.width,
                height: v.height,
            },
            table(
                v.count,
                4,
                v.delta,
                vec![0xAB; (v.count * 4) as usize],
                v.ctts_offset,
                v.stss.clone(),
            ),
        ),
    };
    Mp4TrackSpec {
        track_id: t.track_id,
        handler,
        timescale: t.timescale,
        duration: t.duration,
        language: mp4_language(&t.language),
        entry,
        table,
        extra_trak_boxes: t.extra_boxes.clone(),
    }
}

fn mp4_spec(case: &Mp4Case) -> Mp4Spec {
    Mp4Spec {
        major_brand: case.major_brand,
        compatible_brands: case.compatible_brands.clone(),
        movie_timescale: case.movie_timescale,
        tracks: case.tracks.iter().map(track_spec).collect(),
        extra_top_boxes: case.extra_top_boxes.clone(),
    }
}

/// The `decode_unavailable` observations a case must produce, in track
/// order. Undecodable or failed `soun` tracks before the first decoded
/// one keep their own `detail["kind"]`; every later `soun` track is a
/// "secondary" capability observation regardless of its entry.
fn expected_integrity(case: &Mp4Case) -> Vec<(u32, &'static str)> {
    let decoded_at = case.tracks.iter().position(|t| {
        matches!(&t.kind, TrackKind::Soun(s) if matches!(s.expect, ExpectAudio::Decoded { .. }))
    });
    case.tracks
        .iter()
        .enumerate()
        .filter_map(|(i, t)| {
            let TrackKind::Soun(s) = &t.kind else {
                return None;
            };
            if Some(i) == decoded_at {
                return None;
            }
            let kind = if decoded_at.is_some_and(|d| i > d) {
                "capability" // secondary `soun` track
            } else {
                match &s.expect {
                    ExpectAudio::Unavailable { kind } => *kind,
                    ExpectAudio::Decoded { .. } => unreachable!("first decoded wins"),
                }
            };
            Some((i as u32, kind))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// AVI spec generation
// ---------------------------------------------------------------------------

/// The `vids` stream of an AVI case: BGR24 `DIB ` frames.
#[derive(Debug, Clone)]
struct AviVidGen {
    width: u32,
    height: u32,
    fps: u32,
    start: u32,
    /// Whole BGR24 frames (`width * height * 3` bytes each).
    frames: Vec<Vec<u8>>,
}

/// The `auds` side of an AVI case.
#[derive(Debug, Clone)]
enum AviAudio {
    /// PCM s16 WAVEFORMATEX stream; `packets` are `wb` bodies in mux
    /// order and `samples` the promised decode (packet order).
    Pcm {
        channels: u16,
        rate: u32,
        start: u32,
        packets: Vec<Vec<u8>>,
        samples: Vec<f64>,
    },
    /// `auds` stream whose WAVEFORMATEX tag is non-PCM (0x55): packets
    /// exist but the codec is undecodable.
    NonPcm {
        channels: u16,
        rate: u32,
        start: u32,
        packets: Vec<Vec<u8>>,
    },
    /// No `auds` stream at all.
    Absent,
}

/// A whole generated AVI: optional `vids` + `auds` streams in either
/// `hdrl` order, mux-order packets optionally interleaved.
#[derive(Debug, Clone)]
struct AviCase {
    video: Option<AviVidGen>,
    audio: AviAudio,
    audio_first: bool,
    interleave: bool,
}

fn avi_video() -> impl Strategy<Value = Option<AviVidGen>> {
    prop::option::of(
        (
            // BGR24 rows must word-align: width * 3 % 4 == 0.
            prop_oneof![
                Just(4u32),
                Just(8u32),
                Just(16u32),
                Just(32u32),
                Just(64u32)
            ],
            1u32..=16u32,
            1u32..=60u32,
            0u32..=8u32,
            0usize..=4usize,
        )
            .prop_flat_map(|(w, h, fps, start, n_frames)| {
                (
                    Just(w),
                    Just(h),
                    Just(fps),
                    Just(start),
                    prop::collection::vec(
                        prop::collection::vec(any::<u8>(), (w * h * 3) as usize),
                        n_frames,
                    ),
                )
            })
            .prop_map(|(width, height, fps, start, frames)| AviVidGen {
                width,
                height,
                fps,
                start,
                frames,
            }),
    )
}

fn avi_audio() -> impl Strategy<Value = AviAudio> {
    prop_oneof![
        3 => (1u16..=4u16, 1u32..=192_000u32, 0u32..=8u32).prop_flat_map(|(ch, rate, start)| {
            (
                Just(ch),
                Just(rate),
                Just(start),
                prop::collection::vec(
                    (0usize..=4usize).prop_flat_map(move |frames| {
                        prop::collection::vec(any::<i16>(), frames * usize::from(ch))
                    }),
                    0usize..=4usize,
                ),
            )
        })
            .prop_map(|(ch, rate, start, packets)| {
                let samples: Vec<f64> = packets
                    .concat()
                    .iter()
                    .map(|&v| f64::from(v))
                    .collect();
                let bodies = packets
                    .iter()
                    .map(|p| {
                        pcm_s16le(&p.iter().map(|&v| f64::from(v)).collect::<Vec<_>>())
                    })
                    .collect();
                AviAudio::Pcm {
                    channels: ch,
                    rate,
                    start,
                    packets: bodies,
                    samples,
                }
            }),
        1 => (
            1u16..=4u16,
            1u32..=192_000u32,
            0u32..=8u32,
            prop::collection::vec(
                prop::collection::vec(any::<u8>(), 1usize..=64usize),
                0usize..=3usize,
            ),
        )
            .prop_map(|(channels, rate, start, packets)| AviAudio::NonPcm {
                channels,
                rate,
                start,
                packets,
            }),
        1 => Just(AviAudio::Absent),
    ]
}

fn avi_case() -> impl Strategy<Value = AviCase> {
    (avi_video(), avi_audio(), any::<bool>(), any::<bool>()).prop_map(
        |(video, audio, audio_first, interleave)| AviCase {
            video,
            audio,
            audio_first,
            interleave,
        },
    )
}

/// Round-robin merge of two packet lists preserving each list's order;
/// `b_first` picks which list leads.
fn interleave(a: Vec<MoviPacket>, b: Vec<MoviPacket>, b_first: bool) -> Vec<MoviPacket> {
    let (lead, lag) = if b_first { (b, a) } else { (a, b) };
    let mut out = Vec::with_capacity(lead.len() + lag.len());
    let mut lead = lead.into_iter();
    let mut lag = lag.into_iter();
    loop {
        match (lead.next(), lag.next()) {
            (None, None) => break,
            (x, y) => {
                out.extend(x);
                out.extend(y);
            }
        }
    }
    out
}

/// Expected `timing` entry for one `auds`/`vids` stream, computed from
/// the documented packet-tick rule (mux-order cumulative `strh` ticks;
/// `dwSampleSize` streams keep a remainder).
#[derive(Debug)]
struct StreamExpect {
    stream: u32,
    stream_type: &'static str,
    clock: ClockKind,
    start_tick: i64,
    timestamps: Vec<i64>,
    timebase_hz: u64,
}

/// Build the `AviSpec` plus the expected `m.timing` entries — the model
/// restates the documented rule, it does not reuse decoder internals.
fn avi_spec(case: &AviCase) -> (AviSpec, Vec<StreamExpect>) {
    let aud_stream = |case: &AviCase| -> Option<AviStreamSpec> {
        match &case.audio {
            AviAudio::Pcm {
                channels,
                rate,
                start,
                ..
            }
            | AviAudio::NonPcm {
                channels,
                rate,
                start,
                ..
            } => {
                let mut s = AviStreamSpec::pcm_audio(*channels, *rate).with_start(*start);
                if matches!(case.audio, AviAudio::NonPcm { .. }) {
                    // WAVEFORMATEX tag 0x55 (MP3-ish) — present but
                    // undecodable.
                    s.format[0..2].copy_from_slice(&0x55u16.to_le_bytes());
                }
                Some(s)
            }
            AviAudio::Absent => None,
        }
    };
    let vid_stream = case
        .video
        .as_ref()
        .map(|v| AviStreamSpec::video(v.width, v.height, v.fps).with_start(v.start));
    let (first, second) = if case.audio_first {
        (aud_stream(case), vid_stream)
    } else {
        (vid_stream, aud_stream(case))
    };
    let mut streams = Vec::new();
    let mut vidx = None;
    let mut aidx = None;
    for s in [first, second].into_iter().flatten() {
        let i = streams.len();
        if &s.fcc_type == b"vids" {
            vidx = Some(i);
        } else {
            aidx = Some(i);
        }
        streams.push(s);
    }

    let vid_packets: Vec<MoviPacket> = case
        .video
        .iter()
        .flat_map(|v| {
            v.frames
                .iter()
                .map(|f| MoviPacket::video_frame(vidx.unwrap_or(0), f.clone()))
        })
        .collect();
    let aud_packets: Vec<MoviPacket> = match &case.audio {
        AviAudio::Pcm { packets, .. } | AviAudio::NonPcm { packets, .. } => packets
            .iter()
            .map(|b| MoviPacket::audio(aidx.unwrap_or(0), b.clone()))
            .collect(),
        AviAudio::Absent => vec![],
    };
    let packets = if case.interleave {
        interleave(vid_packets, aud_packets, case.audio_first)
    } else if case.audio_first {
        [aud_packets, vid_packets].concat()
    } else {
        [vid_packets, aud_packets].concat()
    };

    let mut expects = Vec::new();
    for (i, s) in streams.iter().enumerate() {
        let is_vids = &s.fcc_type == b"vids";
        let start_tick = i64::from(s.start) * i64::from(s.scale);
        let mut timestamps = Vec::new();
        let mut clock = start_tick;
        let mut remainder = 0u128;
        for p in packets.iter().filter(|p| p.stream == i) {
            timestamps.push(clock);
            let advance = if s.sample_size > 0 {
                let num = p.body.len() as u128 * u128::from(s.scale) + remainder;
                remainder = num % u128::from(s.sample_size);
                num / u128::from(s.sample_size)
            } else {
                u128::from(s.scale)
            };
            clock += i64::try_from(advance).expect("tick advance fits i64");
        }
        expects.push(StreamExpect {
            stream: i as u32,
            stream_type: if is_vids { "vids" } else { "auds" },
            clock: if is_vids {
                ClockKind::Video
            } else {
                ClockKind::Audio
            },
            start_tick,
            timestamps,
            timebase_hz: u64::from(s.rate),
        });
    }

    let (width, height, usec) = match &case.video {
        Some(v) => (v.width, v.height, 1_000_000 / v.fps.max(1)),
        None => (0, 0, 0),
    };
    (
        AviSpec {
            width,
            height,
            usec_per_frame: usec,
            total_frames: None,
            streams,
            packets,
        },
        expects,
    )
}

// ---------------------------------------------------------------------------
// Deterministic base containers (mutant/kani fodder)
// ---------------------------------------------------------------------------

/// A small valid MP4: one stereo `sowt` PCM track, two frames.
fn base_mp4() -> Vec<u8> {
    write_mp4(&Mp4Spec {
        major_brand: *b"isom",
        compatible_brands: vec![*b"isom"],
        movie_timescale: 1000,
        tracks: vec![Mp4TrackSpec {
            track_id: 1,
            handler: *b"soun",
            timescale: 48_000,
            duration: 2,
            language: mp4_language(b"und"),
            entry: Mp4EntrySpec::Sound {
                codec: *b"sowt",
                channels: 2,
                bits: 16,
                rate: 48_000,
                version: 0,
                lpcm_flags: 0,
                pcmc: None,
            },
            table: Mp4TableSpec::uniform(pcm_s16le(&[1.0, -2.0, 3.0, -4.0]), 2, 4, 1),
            extra_trak_boxes: vec![],
        }],
        extra_top_boxes: vec![],
    })
}

/// A small valid AVI: 4x2 BGR24 video (2 frames) + mono PCM audio.
fn base_avi() -> Vec<u8> {
    write_avi(&AviSpec::cfr(
        4,
        2,
        30,
        vec![vec![7u8; 24], vec![9u8; 24]],
        pcm_s16le(&[1.0, -2.0]),
        1,
        48_000,
    ))
}

// ---------------------------------------------------------------------------
// Properties
// ---------------------------------------------------------------------------

proptest! {
    /// The demux contract: `Ok(_)`/`Err(MediaError)`/`None`, never a
    /// panic, on any input bytes.
    #[test]
    fn container_decoders_never_panic_on_arbitrary_bytes(
        data in prop::collection::vec(any::<u8>(), 0usize..=4096),
    ) {
        prop_assert!(catch_unwind(AssertUnwindSafe(|| decode_mp4(&data))).is_ok());
        let cfg = VideoFeatureConfig::default();
        prop_assert!(catch_unwind(AssertUnwindSafe(|| decode_avi(&data, &cfg))).is_ok());
        prop_assert!(catch_unwind(AssertUnwindSafe(|| detect_format(&data))).is_ok());
    }

    /// `write_mp4` produces a minimal ISO-BMFF that `decode_mp4` must
    /// accept: brands, movie header, per-track declared facts, sample-
    /// table DTS/PTS timing, PCM extraction, and integrity observations
    /// all agree with the spec — including unknown boxes (recorded in
    /// `skipped_boxes`) and secondary/undecodable `soun` tracks.
    #[test]
    fn mp4_writer_output_round_trips_through_decode(case in mp4_case()) {
        let bytes = write_mp4(&mp4_spec(&case));
        prop_assert_eq!(detect_format(&bytes), Some(MediaFormat::Mp4));
        let m = decode_mp4(&bytes).unwrap();

        // ftyp/mvhd facts.
        prop_assert_eq!(m.facts.brands.major_brand, tag_text(&case.major_brand));
        prop_assert_eq!(m.facts.brands.minor_version, 0);
        prop_assert_eq!(
            m.facts.brands.compatible_brands,
            case.compatible_brands.iter().map(tag_text).collect::<Vec<_>>()
        );
        if case.movie_timescale > 0 {
            prop_assert_eq!(m.facts.movie_timescale, Some(case.movie_timescale));
            prop_assert_eq!(m.facts.movie_duration_ticks, Some(0));
        } else {
            prop_assert_eq!(m.facts.movie_timescale, None);
            prop_assert_eq!(m.facts.movie_duration_ticks, None);
        }

        // Per-track declared facts, in trak order.
        prop_assert_eq!(m.facts.tracks.len(), case.tracks.len());
        for (i, (t, want)) in m.facts.tracks.iter().zip(&case.tracks).enumerate() {
            prop_assert_eq!(t.track_index, i as u32);
            prop_assert_eq!(t.track_id, Some(want.track_id));
            prop_assert_eq!(t.timescale, want.timescale);
            prop_assert_eq!(t.duration_ticks, want.duration);
            prop_assert_eq!(t.movie_duration_ticks, Some(want.duration));
            prop_assert_eq!(&t.language, want.language_text);
            prop_assert!(!t.has_edit_list);
            prop_assert_eq!(t.edit_media_time_ticks, None);
            match &want.kind {
                TrackKind::Soun(s) => {
                    let codec = tag_text(&s.codec);
                    prop_assert_eq!(&t.handler, "soun");
                    prop_assert_eq!(t.codec.as_deref(), Some(codec.as_str()));
                    prop_assert_eq!(t.sample_count, u64::from(s.count));
                    prop_assert!(t.sync_samples.is_none());
                    let a = t.audio.as_ref().expect("soun track audio info");
                    prop_assert_eq!(a.channels, s.channels);
                    prop_assert_eq!(a.bits_per_sample, s.bits);
                    prop_assert_eq!(a.sample_rate, s.declared_rate());
                    prop_assert!(t.video.is_none());
                }
                TrackKind::Vide(v) => {
                    let codec = tag_text(&v.codec);
                    prop_assert_eq!(&t.handler, "vide");
                    prop_assert_eq!(t.codec.as_deref(), Some(codec.as_str()));
                    prop_assert_eq!(t.sample_count, u64::from(v.count));
                    prop_assert_eq!(t.sync_samples.as_ref(), v.stss.as_ref());
                    let vi = t.video.as_ref().expect("vide track video info");
                    prop_assert_eq!(vi.width, v.width);
                    prop_assert_eq!(vi.height, v.height);
                    prop_assert!(t.audio.is_none());
                }
            }
        }

        // skipped_boxes in parse order: top-level extras (bare tag),
        // `mdat`, then per track the uninterpreted minf children
        // (media header + dref) and the trak extras.
        let mut skipped: Vec<String> = case
            .extra_top_boxes
            .iter()
            .map(|(tag, _)| tag_text(tag))
            .collect();
        skipped.push("mdat".into());
        for t in &case.tracks {
            let mhtag = match &t.kind {
                TrackKind::Soun(_) => "smhd",
                TrackKind::Vide(_) => "vmhd",
            };
            skipped.push(format!("moov/trak/mdia/minf/{mhtag}"));
            skipped.push("moov/trak/mdia/minf/dref".into());
            for (tag, _) in &t.extra_boxes {
                skipped.push(format!("moov/trak/{}", tag_text(tag)));
            }
        }
        prop_assert_eq!(m.facts.skipped_boxes, skipped);

        // Sample-table timing: one DTS entry per track, plus a sorted
        // PTS entry when the track carries `ctts`. Uniform stts runs
        // produce no discontinuities or duplicates.
        let mut timing_expect = Vec::new();
        for (i, want) in case.tracks.iter().enumerate() {
            let (handler, clock, count, delta, ctts) = match &want.kind {
                TrackKind::Soun(s) => ("soun", ClockKind::Audio, s.count, s.delta, None),
                TrackKind::Vide(v) => ("vide", ClockKind::Video, v.count, v.delta, v.ctts_offset),
            };
            let dts = expected_dts(count, delta);
            timing_expect.push((
                i as u32,
                handler,
                clock,
                TimestampOrigin::SampleTable,
                dts.clone(),
                u64::from(want.timescale),
            ));
            // `table()` emits no ctts for an empty sample table.
            if let Some(off) = ctts.filter(|_| count > 0) {
                let mut pts: Vec<i64> = dts.iter().map(|d| d + i64::from(off)).collect();
                pts.sort_unstable();
                timing_expect.push((
                    i as u32,
                    handler,
                    clock,
                    TimestampOrigin::SampleTablePts,
                    pts,
                    u64::from(want.timescale),
                ));
            }
        }
        prop_assert_eq!(m.timing.len(), timing_expect.len());
        for (t, (stream, stype, clock, origin, ts, tb)) in m.timing.iter().zip(&timing_expect) {
            prop_assert_eq!(t.stream, *stream);
            prop_assert_eq!(&t.stream_type, stype);
            prop_assert_eq!(t.timestamp_origin, *origin);
            prop_assert_eq!(t.start_tick, 0);
            prop_assert_eq!(&t.stream_timestamps, ts);
            prop_assert_eq!(t.observation.clock, *clock);
            prop_assert_eq!(t.observation.timebase_hz, *tb);
            prop_assert_eq!(t.observation.timestamp_count, ts.len());
            prop_assert!(t.observation.discontinuities.is_empty());
            prop_assert!(t.observation.duplicates.is_empty());
        }

        // pcm_tracks: every PCM-capable soun entry, in track order —
        // membership is declared capability, independent of the decode
        // outcome.
        let pcm_tracks: Vec<u32> = case
            .tracks
            .iter()
            .enumerate()
            .filter_map(|(i, t)| match &t.kind {
                TrackKind::Soun(s) if s.pcm_track => Some(i as u32),
                _ => None,
            })
            .collect();
        prop_assert_eq!(&m.pcm_tracks, &pcm_tracks);

        // packet_lengths are the stsz tables verbatim, in trak order.
        for (i, want) in case.tracks.iter().enumerate() {
            let (count, size) = match &want.kind {
                TrackKind::Soun(s) => (s.count, s.sample_size),
                TrackKind::Vide(v) => (v.count, 4),
            };
            prop_assert_eq!(&m.packet_lengths[i], &vec![size; count as usize]);
        }

        // Integrity observations, in track order.
        let integrity = expected_integrity(&case);
        prop_assert_eq!(m.integrity.len(), integrity.len());
        for (o, (ti, kind)) in m.integrity.iter().zip(&integrity) {
            prop_assert_eq!(&o.kind, "decode_unavailable");
            prop_assert_eq!(o.detail["track_index"].as_u64(), Some(u64::from(*ti)));
            prop_assert_eq!(o.detail["handler"].as_str(), Some("soun"));
            prop_assert_eq!(o.detail["kind"].as_str(), Some(*kind));
        }

        // PCM audio: the first decodable soun track wins; when none
        // exists `audio` is `None` — never an error.
        let decoded = case.tracks.iter().enumerate().find(|(_, t)| {
            matches!(&t.kind, TrackKind::Soun(s) if matches!(s.expect, ExpectAudio::Decoded { .. }))
        });
        match decoded {
            Some((i, t)) => {
                let TrackKind::Soun(s) = &t.kind else {
                    unreachable!()
                };
                let ExpectAudio::Decoded {
                    codec,
                    format,
                    bits_per_sample,
                    samples,
                } = &s.expect
                else {
                    unreachable!()
                };
                prop_assert_eq!(m.audio_stream, Some(i as u32));
                let a = m.audio.as_ref().expect("PCM audio decodes");
                prop_assert_eq!(&a.codec, codec);
                prop_assert_eq!(a.channels, s.channels);
                prop_assert_eq!(a.sample_rate, s.declared_rate());
                prop_assert_eq!(a.bits_per_sample, *bits_per_sample);
                prop_assert_eq!(a.sample_format, *format);
                prop_assert_eq!(&a.samples, samples);
            }
            None => {
                prop_assert_eq!(m.audio_stream, None);
                prop_assert!(m.audio.is_none());
            }
        }
    }

    /// `write_avi` produces a legacy AVI that `decode_avi` must accept:
    /// first `vids` declared metadata + features, first `auds` PCM or
    /// an `audio_unavailable` observation, and per-stream `movi_order`
    /// packet timing on separate audio/video clocks.
    #[test]
    fn avi_writer_output_round_trips_through_decode(case in avi_case()) {
        let (spec, expects) = avi_spec(&case);
        let bytes = write_avi(&spec);
        prop_assert_eq!(detect_format(&bytes), Some(MediaFormat::Avi));
        let m = decode_avi(&bytes, &VideoFeatureConfig::default()).unwrap();

        // First vids stream: declared metadata + decodable features.
        match &case.video {
            Some(v) => {
                let vi = m.video.as_ref().expect("vids metadata");
                prop_assert_eq!(&vi.codec, "rawvideo");
                prop_assert_eq!(vi.width, v.width);
                prop_assert_eq!(vi.height, v.height);
                prop_assert_eq!(vi.bits_per_pixel, 24);
                prop_assert_eq!(vi.frames, v.frames.len() as u64);
                prop_assert_eq!(vi.fps, Some(v.fps as f64));
                prop_assert_eq!(
                    vi.duration_secs,
                    Some(v.frames.len() as f64 / v.fps as f64)
                );
                prop_assert!(m.video_features.is_some());
            }
            None => {
                prop_assert!(m.video.is_none());
                prop_assert!(m.video_features.is_none());
            }
        }

        // First auds stream: PCM decodes exactly, anything else is an
        // audio_unavailable observation — never a demux failure.
        let pcm_payload: usize = match &case.audio {
            AviAudio::Pcm { packets, .. } => packets.iter().map(Vec::len).sum(),
            _ => 0,
        };
        match &case.audio {
            AviAudio::Pcm {
                channels,
                rate,
                samples,
                ..
            } if pcm_payload > 0 => {
                let a = m.audio.as_ref().expect("PCM auds decodes");
                prop_assert_eq!(&a.codec, "pcm_s16le");
                prop_assert_eq!(a.channels, *channels);
                prop_assert_eq!(a.sample_rate, *rate);
                prop_assert_eq!(a.bits_per_sample, 16);
                prop_assert_eq!(a.sample_format, SampleFormat::PcmS16);
                prop_assert_eq!(&a.samples, samples);
            }
            _ => {
                prop_assert!(m.audio.is_none());
                prop_assert!(m.audio_unavailable.is_some());
            }
        }
        prop_assert_eq!(m.audio_failure.is_some(), m.audio.is_none());
        prop_assert_eq!(m.audio_unavailable.is_some(), m.audio.is_none());

        // Per-stream movi_order timing on separate clocks.
        prop_assert_eq!(m.timing.len(), expects.len());
        for (t, e) in m.timing.iter().zip(&expects) {
            prop_assert_eq!(t.stream, e.stream);
            prop_assert_eq!(&t.stream_type, e.stream_type);
            prop_assert_eq!(t.timestamp_origin, TimestampOrigin::MoviOrder);
            prop_assert_eq!(t.start_tick, e.start_tick);
            prop_assert_eq!(&t.stream_timestamps, &e.timestamps);
            prop_assert_eq!(t.observation.clock, e.clock);
            prop_assert_eq!(t.observation.timebase_hz, e.timebase_hz);
            prop_assert_eq!(t.observation.timestamp_count, e.timestamps.len());
            if e.clock == ClockKind::Video {
                // Constant-interval video frames never discontinuous.
                prop_assert!(t.observation.discontinuities.is_empty());
                prop_assert!(t.observation.duplicates.is_empty());
            }
        }
    }

    /// Partial ingest, MP4 side: when every `soun` entry is compressed,
    /// `decode_mp4` still returns `Ok` with `audio: None` and one
    /// capability observation per `soun` track.
    #[test]
    fn compressed_audio_is_an_observation_not_an_error(
        codec in prop_oneof![Just(*b"mp4a"), Just(*b"ac-3"), Just(*b"alac"), Just(*b"ec-3")],
        n_soun in 1usize..=3usize,
        with_vide in any::<bool>(),
    ) {
        let mut tracks = Vec::new();
        if with_vide {
            tracks.push(fixed_track(
                1,
                TrackKind::Vide(VideGen {
                    codec: *b"avc1",
                    width: 16,
                    height: 16,
                    delta: 1000,
                    count: 2,
                    ctts_offset: None,
                    stss: None,
                }),
            ));
        }
        for i in 0..n_soun {
            tracks.push(fixed_track(
                2 + i as u32,
                TrackKind::Soun(soun_compressed(codec, 2, 48_000, 1, 4, 64)),
            ));
        }
        let case = Mp4Case {
            major_brand: *b"isom",
            compatible_brands: vec![],
            movie_timescale: 1000,
            extra_top_boxes: vec![],
            tracks,
        };
        let m = decode_mp4(&write_mp4(&mp4_spec(&case))).unwrap();
        prop_assert!(m.audio.is_none());
        prop_assert_eq!(m.audio_stream, None);
        prop_assert!(m.pcm_tracks.is_empty());
        prop_assert_eq!(m.integrity.len(), n_soun);
        let soun_tracks: Vec<&SounGen> = case
            .tracks
            .iter()
            .filter_map(|t| match &t.kind {
                TrackKind::Soun(s) => Some(s),
                _ => None,
            })
            .collect();
        for (o, s) in m.integrity.iter().zip(&soun_tracks) {
            let codec = tag_text(&s.codec);
            prop_assert_eq!(&o.kind, "decode_unavailable");
            prop_assert_eq!(o.detail["kind"].as_str(), Some("capability"));
            prop_assert_eq!(o.detail["codec"].as_str(), Some(codec.as_str()));
        }
    }

    /// Partial ingest, AVI side: a missing `auds` stream, a non-PCM
    /// `auds` codec, or an `auds` stream with no `wb` payload each keep
    /// `audio` `None` with an `audio_unavailable` capability record —
    /// video and timing stay intact.
    #[test]
    fn missing_or_undecodable_auds_is_audio_unavailable(
        kind in prop_oneof![Just(0u8), Just(1u8), Just(2u8)],
        n_frames in 0usize..=4usize,
    ) {
        let frames: Vec<Vec<u8>> = (0..n_frames).map(|i| vec![i as u8; 24]).collect();
        let audio = match kind {
            0 => AviAudio::Absent,
            1 => AviAudio::NonPcm {
                channels: 1,
                rate: 48_000,
                start: 0,
                packets: vec![vec![0xAA; 16]],
            },
            // auds stream present but carries no wb packets.
            _ => AviAudio::Pcm {
                channels: 1,
                rate: 48_000,
                start: 0,
                packets: vec![],
                samples: vec![],
            },
        };
        let case = AviCase {
            video: Some(AviVidGen {
                width: 4,
                height: 2,
                fps: 30,
                start: 0,
                frames,
            }),
            audio,
            audio_first: false,
            interleave: false,
        };
        let (spec, _) = avi_spec(&case);
        let m = decode_avi(&write_avi(&spec), &VideoFeatureConfig::default()).unwrap();
        prop_assert!(m.audio.is_none());
        prop_assert!(m.audio_unavailable.is_some());
        prop_assert_eq!(
            m.audio_failure.as_ref().map(|u| u.kind),
            Some(UnavailableKind::Capability)
        );
        if kind == 0 {
            prop_assert_eq!(
                m.audio_unavailable.as_deref(),
                Some("AVI has no audio stream")
            );
        }
        prop_assert!(m.video.is_some());
    }

    /// `detect_format` follows its documented signature rules exactly:
    /// `RIFF` + `WAVE`/`AVI ` form type, `ftyp` at offset 4, nothing
    /// else — exercised at full length and on truncated views.
    #[test]
    fn detect_format_follows_signature_rules(
        riff in any::<bool>(),
        size_or_tag in any::<[u8; 4]>(),
        form in prop_oneof![
            Just(*b"WAVE"),
            Just(*b"AVI "),
            Just(*b"XVID"),
            Just(*b"ftyp"),
            Just(*b"WEBP"),
            any::<[u8; 4]>(),
        ],
        ftyp_box in any::<bool>(),
        tail in prop::collection::vec(any::<u8>(), 0usize..=16usize),
        cut in 0usize..=32usize,
    ) {
        let mut data = Vec::new();
        data.extend_from_slice(if riff { b"RIFF" } else { &size_or_tag });
        data.extend_from_slice(if ftyp_box { b"ftyp" } else { &size_or_tag });
        data.extend_from_slice(&form);
        data.extend_from_slice(&tail);
        prop_assert_eq!(detect_format(&data), expected_detect(&data));
        // Truncated views exercise the len<12 / len<8 boundaries.
        let n = cut.min(data.len());
        prop_assert_eq!(detect_format(&data[..n]), expected_detect(&data[..n]));
    }

    /// Mutation-shaped input: random byte overwrites and truncation of
    /// writer-produced containers may be `Ok` or `Err`, never a panic.
    #[test]
    fn mutated_or_truncated_containers_never_panic(
        mp4_edits in prop::collection::vec((0usize..=512, any::<u8>()), 0usize..=16),
        mp4_truncate in 0usize..=512usize,
        avi_edits in prop::collection::vec((0usize..=512, any::<u8>()), 0usize..=16),
        avi_truncate in 0usize..=512usize,
    ) {
        let mut mp4 = base_mp4();
        for (at, v) in mp4_edits {
            let i = at % mp4.len();
            mp4[i] = v;
        }
        mp4.truncate(mp4_truncate.min(mp4.len()));
        prop_assert!(catch_unwind(AssertUnwindSafe(|| decode_mp4(&mp4))).is_ok());
        prop_assert!(catch_unwind(AssertUnwindSafe(|| detect_format(&mp4))).is_ok());

        let mut avi = base_avi();
        for (at, v) in avi_edits {
            let i = at % avi.len();
            avi[i] = v;
        }
        avi.truncate(avi_truncate.min(avi.len()));
        let cfg = VideoFeatureConfig::default();
        prop_assert!(catch_unwind(AssertUnwindSafe(|| decode_avi(&avi, &cfg))).is_ok());
        prop_assert!(catch_unwind(AssertUnwindSafe(|| detect_format(&avi))).is_ok());
    }
}

// ---------------------------------------------------------------------------
// Kani bounded proofs (cargo kani -p al-media --tests)
// ---------------------------------------------------------------------------

#[cfg(kani)]
mod kani_proofs {
    use super::*;

    /// Inputs that carry neither container signature (no `ftyp` at
    /// offset 4 and no `RIFF`+`AVI ` pair): `detect_format`,
    /// `decode_mp4`, and `decode_avi` all reject them early — the
    /// shallow, cheap half of the no-panic contract. Disjoint from
    /// the two `*_signed` proofs (al#274 input-domain split).
    #[kani::proof]
    #[kani::unwind(16)]
    fn containers_never_panics_unsigned() {
        let data: [u8; 64] = kani::any();
        kani::assume(&data[4..8] != b"ftyp");
        kani::assume(&data[0..4] != b"RIFF" || &data[8..12] != b"AVI ");
        let _ = detect_format(&data);
        let _ = decode_mp4(&data);
        let _ = decode_avi(&data, &VideoFeatureConfig::default());
    }

    /// `ftyp`-signed inputs may never panic `decode_mp4` (or
    /// `detect_format`, which reads only the first <=12 bytes and is
    /// near-free, so it rides along). 64 bytes covers `ftyp` plus a
    /// few box headers — ISO-BMFF needs ~60B of nesting to reach
    /// `stsd`, so this bound exercises top-level box walking plus
    /// early header validation (a bounded claim, not totality).
    /// Split per entry point for al#274: the combined 3-entry proof
    /// was solver-bound at 45min.
    #[kani::proof]
    #[kani::unwind(48)]
    fn mp4_never_panics_ftyp_signed() {
        let data: [u8; 64] = kani::any();
        kani::assume(&data[4..8] == b"ftyp");
        let _ = detect_format(&data);
        let _ = decode_mp4(&data);
    }

    /// `RIFF`+`AVI `-signed inputs may never panic `decode_avi`:
    /// RIFF chunk walking and `strl` parsing stay panic-free.
    #[kani::proof]
    #[kani::unwind(48)]
    fn avi_never_panics_riff_signed() {
        let data: [u8; 64] = kani::any();
        kani::assume(&data[0..4] == b"RIFF" && &data[8..12] == b"AVI ");
        let _ = detect_format(&data);
        let _ = decode_avi(&data, &VideoFeatureConfig::default());
    }

    /// The documented signature rules hold exhaustively at the three
    /// boundary lengths: len<8 is always `None`, len>=8 `ftyp` at
    /// offset 4 means `Mp4`, len>=12 `RIFF` resolves to `Wav`/`Avi`/
    /// `None` by form type — RIFF winning over `ftyp` is part of the
    /// contract (check order).
    #[kani::proof]
    #[kani::unwind(8)]
    fn detect_format_signature_rules() {
        let short: [u8; 7] = kani::any();
        assert_eq!(detect_format(&short), None);
        let eight: [u8; 8] = kani::any();
        assert_eq!(
            detect_format(&eight),
            if &eight[4..8] == b"ftyp" {
                Some(MediaFormat::Mp4)
            } else {
                None
            }
        );
        let full: [u8; 12] = kani::any();
        assert_eq!(detect_format(&full), expected_detect(&full));
    }

    /// Shared body of the MP4 writer round-trip probe. The canonical
    /// harness and temporary spike harnesses share exactly this body.
    fn mp4_writer_output_decodes_body() {
        let channels: u16 = kani::any();
        kani::assume((1..=2).contains(&channels));
        let rate: u32 = kani::any();
        kani::assume((1..=192_000).contains(&rate));
        let samples: [i16; 8] = kani::any();
        let keep = 4 * usize::from(channels);
        let s16: Vec<f64> = samples[..keep].iter().map(|&v| f64::from(v)).collect();
        let payload: Vec<u8> = samples[..keep]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let spec = Mp4Spec {
            major_brand: *b"isom",
            compatible_brands: vec![*b"isom"],
            movie_timescale: 1000,
            tracks: vec![Mp4TrackSpec {
                track_id: 1,
                handler: *b"soun",
                timescale: rate,
                duration: 4,
                language: mp4_language(b"und"),
                entry: Mp4EntrySpec::Sound {
                    codec: *b"sowt",
                    channels,
                    bits: 16,
                    rate,
                    version: 0,
                    lpcm_flags: 0,
                    pcmc: None,
                },
                table: Mp4TableSpec::uniform(payload, 4, u32::from(channels) * 2, 1),
                extra_trak_boxes: vec![],
            }],
            extra_top_boxes: vec![],
        };
        let bytes = write_mp4(&spec);
        assert_eq!(detect_format(&bytes), Some(MediaFormat::Mp4));
        let m = decode_mp4(&bytes).unwrap();
        assert_eq!(m.facts.tracks.len(), 1);
        let a = m.audio.expect("sowt PCM decodes");
        assert_eq!(a.codec, "pcm_s16le");
        assert_eq!(a.channels, channels);
        assert_eq!(a.sample_rate, rate);
        assert_eq!(a.samples, s16);
    }

    /// `write_mp4` output over symbolic `soun` params decodes and the
    /// PCM round-trip agrees. channels 1..=2, rate 1..=192000, and four
    /// frames of symbolic s16 keep the sample loop small; the bound
    /// exercises ftyp/moov/stbl/stsd parsing and the PCM path end to
    /// end. unwind(48) covers the fixed box nesting of this spec shape
    /// (al#274: 64 was over-provisioned for ~20 reachable iterations).
    #[kani::proof]
    #[kani::unwind(48)]
    fn mp4_writer_output_decodes() {
        mp4_writer_output_decodes_body();
    }

    // Temporary al#274 spike harnesses (not registered in the canonical
    // runner). The fmt variants are controls only and must not be read as
    // valid proofs: this body asserts the `format!`-produced PCM codec name,
    // so a fmt stub is not contract-equivalent here, can make the assertion
    // fail if reached, and must never be adopted for this harness.

    #[kani::proof]
    #[kani::unwind(48)]
    #[kani::stub(std::fmt::format, crate::kani_stub_models::fmt_format_empty)]
    fn spike_mp4_writer_fmt() {
        mp4_writer_output_decodes_body();
    }

    #[kani::proof]
    #[kani::unwind(48)]
    #[kani::stub(
        std::alloc::handle_alloc_error,
        crate::kani_stub_models::handle_alloc_error_infallible
    )]
    fn spike_mp4_writer_alloc() {
        mp4_writer_output_decodes_body();
    }

    #[kani::proof]
    #[kani::unwind(48)]
    #[kani::stub(std::fmt::format, crate::kani_stub_models::fmt_format_empty)]
    #[kani::stub(
        std::alloc::handle_alloc_error,
        crate::kani_stub_models::handle_alloc_error_infallible
    )]
    fn spike_mp4_writer_both() {
        mp4_writer_output_decodes_body();
    }

    /// `write_avi` output over symbolic PCM audio decodes: first `vids`
    /// metadata, first `auds` PCM, and per-stream `movi_order` timing
    /// agree. Concrete video (4x2, two frames) bounds the DIB feature
    /// path; symbolic audio is four frames of s16. unwind(64) covers
    /// RIFF chunk walking, both strl parsers, and the feature loops.
    #[kani::proof]
    #[kani::unwind(64)]
    fn avi_writer_output_decodes() {
        let channels: u16 = kani::any();
        kani::assume((1..=2).contains(&channels));
        let rate: u32 = kani::any();
        kani::assume((1..=192_000).contains(&rate));
        let samples: [i16; 8] = kani::any();
        let keep = 4 * usize::from(channels);
        let s16: Vec<f64> = samples[..keep].iter().map(|&v| f64::from(v)).collect();
        let spec = AviSpec::cfr(
            4,
            2,
            30,
            vec![vec![7u8; 24], vec![9u8; 24]],
            pcm_s16le(&s16),
            channels,
            rate,
        );
        let bytes = write_avi(&spec);
        assert_eq!(detect_format(&bytes), Some(MediaFormat::Avi));
        let m = decode_avi(&bytes, &VideoFeatureConfig::default()).unwrap();
        let a = m.audio.expect("PCM auds decodes");
        assert_eq!(a.codec, "pcm_s16le");
        assert_eq!(a.channels, channels);
        assert_eq!(a.sample_rate, rate);
        assert_eq!(a.samples, s16);
        let v = m.video.expect("vids metadata");
        assert_eq!((v.width, v.height), (4, 2));
        assert_eq!(v.frames, 2);
        assert_eq!(m.timing.len(), 2);
    }

    /// A single symbolic byte overwrite of a writer-produced MP4 may
    /// never panic decode or detection. Split from the combined
    /// mutate-and-truncate proof for al#274: each proof now covers its
    /// own corruption dimension fully — every mutation position and
    /// byte value over the full-length base. The base container is
    /// concrete (~500B), so the only symbolic state is which byte
    /// changes to what. unwind(64) covers box walking and the
    /// mutated-count table loops of the surviving file.
    #[kani::proof]
    #[kani::unwind(64)]
    fn mutated_mp4_never_panics() {
        let mut bytes = base_mp4();
        let at: u16 = kani::any();
        let v: u8 = kani::any();
        let i = usize::from(at) % bytes.len();
        bytes[i] = v;
        let _ = detect_format(&bytes);
        let _ = decode_mp4(&bytes);
    }

    /// The truncation dimension: a writer-produced MP4 cut to any
    /// length never panics decode or detection. Together with
    /// `mutated_mp4_never_panics` this covers the corruption classes
    /// separately; inputs that are both mutated and truncated stay
    /// under the proptest property
    /// `mutated_or_truncated_containers_never_panic`.
    #[kani::proof]
    #[kani::unwind(64)]
    fn truncated_mp4_never_panics() {
        let mut bytes = base_mp4();
        let cut: u16 = kani::any();
        bytes.truncate(usize::from(cut) % (bytes.len() + 1));
        let _ = detect_format(&bytes);
        let _ = decode_mp4(&bytes);
    }

    /// Same mutation budget over a writer-produced AVI: RIFF chunk
    /// walking, `hdrl`/`strl` parsing, and `movi` packet reads stay
    /// panic-free under a single corrupted byte.
    #[kani::proof]
    #[kani::unwind(80)]
    fn mutated_avi_never_panics() {
        let mut bytes = base_avi();
        let at: u16 = kani::any();
        let v: u8 = kani::any();
        let i = usize::from(at) % bytes.len();
        bytes[i] = v;
        let _ = detect_format(&bytes);
        let _ = decode_avi(&bytes, &VideoFeatureConfig::default());
    }

    /// The truncation dimension over the AVI base.
    #[kani::proof]
    #[kani::unwind(80)]
    fn truncated_avi_never_panics() {
        let mut bytes = base_avi();
        let cut: u16 = kani::any();
        bytes.truncate(usize::from(cut) % (bytes.len() + 1));
        let _ = detect_format(&bytes);
        let _ = decode_avi(&bytes, &VideoFeatureConfig::default());
    }
}
