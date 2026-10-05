//! Synthetic MP4/ISO-BMFF writer for fixtures and golden tests.
//!
//! Demux/parser knowledge lives in `al_media::mp4`; this module is the
//! test-side counterpart that produces byte-identical containers with
//! caller-controlled sample tables (stts/ctts/stss/stsz/stsc/stco),
//! PCM sample entries, and track metadata.

// ---------------------------------------------------------------------------
// Synthetic writer (fixtures and tests), mirroring `write_avi`.
// ---------------------------------------------------------------------------

/// Sample-table shape for one synthetic MP4 track.
pub struct Mp4TableSpec {
    /// `stts` runs: (sample_count, delta).
    pub stts: Vec<(u32, u32)>,
    /// Optional `ctts` runs: (sample_count, offset).
    pub ctts: Option<Vec<(u32, u32)>>,
    /// Optional `stss` sync-sample indices (1-based).
    pub stss: Option<Vec<u32>>,
    /// `stsc` runs: (first_chunk, samples_per_chunk, sd_index).
    pub stsc: Vec<(u32, u32, u32)>,
    /// Chunk payloads placed in `mdat` in order (stco computed).
    pub chunks: Vec<Vec<u8>>,
    /// Per-sample sizes (`stsz`); must cover every sample mapped by
    /// `stsc`.
    pub sample_sizes: Vec<u32>,
}

impl Mp4TableSpec {
    /// One chunk holding `sample_count` samples of `sample_size` bytes,
    /// constant `delta` ticks apart.
    pub fn uniform(chunk: Vec<u8>, sample_count: u32, sample_size: u32, delta: u32) -> Self {
        Self {
            stts: vec![(sample_count, delta)],
            ctts: None,
            stss: None,
            stsc: vec![(1, sample_count, 1)],
            chunks: vec![chunk],
            sample_sizes: vec![sample_size; sample_count as usize],
        }
    }
}

/// Sample description entry for a synthetic track.
pub enum Mp4EntrySpec {
    /// Sound sample entry (`soun` track).
    Sound {
        /// Entry fourcc (`sowt`, `twos`, `lpcm`, `mp4a`, ...).
        codec: [u8; 4],
        /// Channel count.
        channels: u16,
        /// Bits per sample.
        bits: u16,
        /// Sample rate in Hz.
        rate: u32,
        /// Sample entry version (0 or 2; 2 emits the QT structure with
        /// lpcm format flags).
        version: u16,
        /// `formatSpecificFlags` written for version-2 entries.
        lpcm_flags: u32,
        /// Emit a `pcmC` child box (ISO 23003-5): (format_flags,
        /// pcm_sample_size).
        pcmc: Option<(u8, u8)>,
    },
    /// Visual sample entry (`vide` track).
    Video {
        /// Entry fourcc (`avc1`, `rawvideo`-style tag, ...).
        codec: [u8; 4],
        /// Frame width in pixels.
        width: u16,
        /// Frame height in pixels.
        height: u16,
    },
}

/// One synthetic track.
pub struct Mp4TrackSpec {
    /// `tkhd` track id.
    pub track_id: u32,
    /// `hdlr` handler type (`soun`, `vide`, ...).
    pub handler: [u8; 4],
    /// `mdhd` timescale.
    pub timescale: u32,
    /// `mdhd` duration in timescale ticks.
    pub duration: u64,
    /// Packed 15-bit mdhd language; see [`mp4_language`].
    pub language: u16,
    /// Sample description entry.
    pub entry: Mp4EntrySpec,
    /// Sample table + chunk payloads.
    pub table: Mp4TableSpec,
    /// Extra boxes appended inside the `trak` (unknown-box coverage).
    pub extra_trak_boxes: Vec<([u8; 4], Vec<u8>)>,
}

/// Whole-file spec for [`write_mp4`].
pub struct Mp4Spec {
    /// `ftyp` major brand.
    pub major_brand: [u8; 4],
    /// `ftyp` compatible brands.
    pub compatible_brands: Vec<[u8; 4]>,
    /// `mvhd` movie timescale (0 omits `mvhd`).
    pub movie_timescale: u32,
    /// Tracks in `moov` order.
    pub tracks: Vec<Mp4TrackSpec>,
    /// Extra top-level boxes after `ftyp`/`mdat`/`moov` (e.g. `free`,
    /// unknown boxes — placed before `moov`).
    pub extra_top_boxes: Vec<([u8; 4], Vec<u8>)>,
}

/// Pack an ISO-639-2/T language code into the mdhd 15-bit field.
pub fn mp4_language(lang: &[u8; 3]) -> u16 {
    (u16::from(lang[0] - 0x60) << 10) | (u16::from(lang[1] - 0x60) << 5) | u16::from(lang[2] - 0x60)
}

fn put_box(out: &mut Vec<u8>, tag: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(&((body.len() + 8) as u32).to_be_bytes());
    out.extend_from_slice(tag);
    out.extend_from_slice(body);
}

fn full_body(version_flags: u32, rest: &[u8]) -> Vec<u8> {
    let mut b = Vec::with_capacity(rest.len() + 4);
    b.extend_from_slice(&version_flags.to_be_bytes());
    b.extend_from_slice(rest);
    b
}

fn sound_entry_body(spec: &Mp4EntrySpec) -> Vec<u8> {
    let Mp4EntrySpec::Sound {
        channels,
        bits,
        rate,
        version,
        lpcm_flags,
        pcmc,
        ..
    } = spec
    else {
        unreachable!()
    };
    let mut b = Vec::new();
    b.extend_from_slice(&[0u8; 6]); // reserved
    b.extend_from_slice(&1u16.to_be_bytes()); // data_reference_index
    b.extend_from_slice(&version.to_be_bytes());
    b.extend_from_slice(&0u16.to_be_bytes()); // revision
    b.extend_from_slice(&0u32.to_be_bytes()); // vendor
    b.extend_from_slice(&channels.to_be_bytes());
    b.extend_from_slice(&bits.to_be_bytes());
    b.extend_from_slice(&0u16.to_be_bytes()); // compression id
    b.extend_from_slice(&0u16.to_be_bytes()); // packet size
    b.extend_from_slice(&(*rate << 16).to_be_bytes()); // 16.16 rate
    if *version == 2 {
        b.extend_from_slice(&36u32.to_be_bytes()); // sizeofStruct
        b.extend_from_slice(&(*rate as f64).to_be_bytes());
        b.extend_from_slice(&u32::from(*channels).to_be_bytes());
        b.extend_from_slice(&0x7F00_0000u32.to_be_bytes());
        b.extend_from_slice(&u32::from(*bits).to_be_bytes());
        b.extend_from_slice(&lpcm_flags.to_be_bytes());
        b.extend_from_slice(&0u32.to_be_bytes()); // bytesPerAudioPacket
        b.extend_from_slice(&0u32.to_be_bytes()); // lpcmFramesPerPacket
    }
    if let Some((flags, size)) = pcmc {
        // ISO/IEC 23003-5 pcmC: FullBox header (4) + format_flags (1)
        // + PCM_sample_size (1) — a 6-byte body, nothing else.
        let mut c = Vec::new();
        c.extend_from_slice(&0u32.to_be_bytes()); // version/flags
        c.push(*flags);
        c.push(*size);
        put_box(&mut b, b"pcmC", &c);
    }
    b
}

fn video_entry_body(spec: &Mp4EntrySpec) -> Vec<u8> {
    let Mp4EntrySpec::Video { width, height, .. } = spec else {
        unreachable!()
    };
    let mut b = vec![0u8; 78];
    b[6..8].copy_from_slice(&1u16.to_be_bytes()); // data_reference_index
    b[24..26].copy_from_slice(&width.to_be_bytes());
    b[26..28].copy_from_slice(&height.to_be_bytes());
    b[40..42].copy_from_slice(&1u16.to_be_bytes()); // frame_count
    b[74..76].copy_from_slice(&24u16.to_be_bytes()); // depth
    b[76..78].copy_from_slice(&(-1i16).to_be_bytes()); // pre_defined
    b
}

/// Serialize an [`Mp4Spec`] into an ISO-BMFF file:
/// `ftyp`, extra top boxes, `mdat`, then `moov` (so `stco` offsets are
/// known before `moov` is emitted).
pub fn write_mp4(spec: &Mp4Spec) -> Vec<u8> {
    let mut ftyp = Vec::new();
    ftyp.extend_from_slice(&spec.major_brand);
    ftyp.extend_from_slice(&0u32.to_be_bytes()); // minor version
    for b in &spec.compatible_brands {
        ftyp.extend_from_slice(b);
    }

    let mut extra = Vec::new();
    for (tag, body) in &spec.extra_top_boxes {
        put_box(&mut extra, tag, body);
    }

    let mdat_len: usize = spec
        .tracks
        .iter()
        .flat_map(|t| t.table.chunks.iter())
        .map(|c| c.len())
        .sum();
    // mdat payload offset = ftyp box + extra boxes + mdat box header.
    let mdat_start = 8 + ftyp.len() + extra.len() + 8;
    let mut chunk_offsets: Vec<Vec<u64>> = Vec::new();
    {
        let mut off = mdat_start as u64;
        for t in &spec.tracks {
            let mut v = Vec::new();
            for c in &t.table.chunks {
                v.push(off);
                off += c.len() as u64;
            }
            chunk_offsets.push(v);
        }
    }
    let mut mdat = Vec::with_capacity(mdat_len);
    for t in &spec.tracks {
        for c in &t.table.chunks {
            mdat.extend_from_slice(c);
        }
    }

    // moov
    let mut moov = Vec::new();
    if spec.movie_timescale > 0 {
        let mut mvhd = full_body(0, &[]);
        mvhd.extend_from_slice(&[0u8; 8]); // creation/modification
        mvhd.extend_from_slice(&spec.movie_timescale.to_be_bytes());
        mvhd.extend_from_slice(&0u32.to_be_bytes()); // duration
        mvhd.extend_from_slice(&0x0001_0000u32.to_be_bytes()); // rate
        mvhd.extend_from_slice(&0x0100u16.to_be_bytes()); // volume
        mvhd.extend_from_slice(&[0u8; 10]); // reserved
        mvhd.extend_from_slice(&[0u8; 36]); // matrix
        mvhd.extend_from_slice(&[0u8; 24]); // predefined
        mvhd.extend_from_slice(&1u32.to_be_bytes()); // next track id
        put_box(&mut moov, b"mvhd", &mvhd);
    }
    for (ti, t) in spec.tracks.iter().enumerate() {
        // tkhd (v0, flags=7)
        let mut tkhd = full_body(7, &[]);
        tkhd.extend_from_slice(&[0u8; 8]); // creation/modification
        tkhd.extend_from_slice(&t.track_id.to_be_bytes());
        tkhd.extend_from_slice(&0u32.to_be_bytes()); // reserved
        tkhd.extend_from_slice(&(t.duration as u32).to_be_bytes());
        tkhd.extend_from_slice(&[0u8; 8]); // reserved
        tkhd.extend_from_slice(&[0u8; 2]); // layer
        tkhd.extend_from_slice(&[0u8; 2]); // alternate group
        tkhd.extend_from_slice(&if &t.handler == b"soun" { 0x0100u16 } else { 0 }.to_be_bytes()); // volume
        tkhd.extend_from_slice(&[0u8; 2]); // reserved
        tkhd.extend_from_slice(&[0u8; 36]); // matrix
        let (w, h) = match &t.entry {
            Mp4EntrySpec::Video { width, height, .. } => (*width, *height),
            _ => (0, 0),
        };
        tkhd.extend_from_slice(&(u32::from(w) << 16).to_be_bytes());
        tkhd.extend_from_slice(&(u32::from(h) << 16).to_be_bytes());

        // mdhd (v0)
        let mut mdhd = full_body(0, &[]);
        mdhd.extend_from_slice(&[0u8; 8]); // creation/modification
        mdhd.extend_from_slice(&t.timescale.to_be_bytes());
        mdhd.extend_from_slice(&(t.duration as u32).to_be_bytes());
        mdhd.extend_from_slice(&t.language.to_be_bytes());
        mdhd.extend_from_slice(&0u16.to_be_bytes()); // quality

        // hdlr
        let mut hdlr = full_body(0, &[]);
        hdlr.extend_from_slice(&0u32.to_be_bytes()); // pre_defined
        hdlr.extend_from_slice(&t.handler);
        hdlr.extend_from_slice(&[0u8; 12]); // reserved
        hdlr.push(0); // name (empty cstring)

        // media header
        let (mhtag, mhbody) = if &t.handler == b"soun" {
            let mut b = full_body(0, &[]);
            b.extend_from_slice(&[0u8; 4]); // balance + reserved
            (b"smhd", b)
        } else {
            let mut b = full_body(1, &[]); // vmhd flags=1
            b.extend_from_slice(&[0u8; 8]); // graphicsmode + opcolor
            (b"vmhd", b)
        };
        let mut minf = Vec::new();
        put_box(&mut minf, mhtag, &mhbody);
        let mut dinf = Vec::new();
        {
            let mut dref = full_body(0, &[]);
            dref.extend_from_slice(&1u32.to_be_bytes());
            // `url ` full box with flags=1: media is in the same file.
            put_box(&mut dref, b"url ", &full_body(1, &[]));
            put_box(&mut dinf, b"dref", &dref);
        }
        minf.extend_from_slice(&dinf);

        // stbl
        let mut stbl = Vec::new();
        {
            let mut stsd = full_body(0, &[]);
            stsd.extend_from_slice(&1u32.to_be_bytes());
            let (etag, ebody) = match &t.entry {
                spec_entry @ Mp4EntrySpec::Sound { codec, .. } => {
                    (codec, sound_entry_body(spec_entry))
                }
                spec_entry @ Mp4EntrySpec::Video { codec, .. } => {
                    (codec, video_entry_body(spec_entry))
                }
            };
            put_box(&mut stsd, etag, &ebody);
            put_box(&mut stbl, b"stsd", &stsd);
        }
        {
            let mut stts = full_body(0, &[]);
            stts.extend_from_slice(&(t.table.stts.len() as u32).to_be_bytes());
            for &(count, delta) in &t.table.stts {
                stts.extend_from_slice(&count.to_be_bytes());
                stts.extend_from_slice(&delta.to_be_bytes());
            }
            put_box(&mut stbl, b"stts", &stts);
        }
        if let Some(ctts) = &t.table.ctts {
            let mut body = full_body(0, &[]);
            body.extend_from_slice(&(ctts.len() as u32).to_be_bytes());
            for &(count, off) in ctts {
                body.extend_from_slice(&count.to_be_bytes());
                body.extend_from_slice(&off.to_be_bytes());
            }
            put_box(&mut stbl, b"ctts", &body);
        }
        if let Some(stss) = &t.table.stss {
            let mut body = full_body(0, &[]);
            body.extend_from_slice(&(stss.len() as u32).to_be_bytes());
            for &i in stss {
                body.extend_from_slice(&i.to_be_bytes());
            }
            put_box(&mut stbl, b"stss", &body);
        }
        {
            let mut stsc = full_body(0, &[]);
            stsc.extend_from_slice(&(t.table.stsc.len() as u32).to_be_bytes());
            for &(first, spc, sd) in &t.table.stsc {
                stsc.extend_from_slice(&first.to_be_bytes());
                stsc.extend_from_slice(&spc.to_be_bytes());
                stsc.extend_from_slice(&sd.to_be_bytes());
            }
            put_box(&mut stbl, b"stsc", &stsc);
        }
        {
            let uniform = t
                .table
                .sample_sizes
                .first()
                .filter(|&&s| t.table.sample_sizes.iter().all(|&x| x == s))
                .copied()
                .unwrap_or(0);
            let mut stsz = full_body(0, &[]);
            stsz.extend_from_slice(&uniform.to_be_bytes());
            stsz.extend_from_slice(&(t.table.sample_sizes.len() as u32).to_be_bytes());
            if uniform == 0 {
                for &s in &t.table.sample_sizes {
                    stsz.extend_from_slice(&s.to_be_bytes());
                }
            }
            put_box(&mut stbl, b"stsz", &stsz);
        }
        {
            let mut stco = full_body(0, &[]);
            stco.extend_from_slice(&(chunk_offsets[ti].len() as u32).to_be_bytes());
            for &o in &chunk_offsets[ti] {
                stco.extend_from_slice(&(o as u32).to_be_bytes());
            }
            put_box(&mut stbl, b"stco", &stco);
        }
        put_box(&mut minf, b"stbl", &stbl);

        let mut mdia = Vec::new();
        put_box(&mut mdia, b"mdhd", &mdhd);
        put_box(&mut mdia, b"hdlr", &hdlr);
        put_box(&mut mdia, b"minf", &minf);

        let mut trak = Vec::new();
        put_box(&mut trak, b"tkhd", &tkhd);
        put_box(&mut trak, b"mdia", &mdia);
        for (tag, body) in &t.extra_trak_boxes {
            put_box(&mut trak, tag, body);
        }
        put_box(&mut moov, b"trak", &trak);
    }

    let mut out = Vec::new();
    put_box(&mut out, b"ftyp", &ftyp);
    out.extend_from_slice(&extra);
    put_box(&mut out, b"mdat", &mdat);
    put_box(&mut out, b"moov", &moov);
    out
}
