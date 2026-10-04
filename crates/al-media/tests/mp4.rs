//! MP4 (ISO-BMFF) ingest tests: synthetic fixtures through `decode_mp4`
//! and the real ingest stage, covering sample-table timing recovery,
//! PCM extraction, the partial-ingest contract, and explicit failures.

use std::path::{Path, PathBuf};

use al_core::{InputRole, StageKind, StageStatus};
use al_job::job::{JobDefinition, input};
use al_job::runner::StageRunner;
use al_job::stage::Stage;
use al_media::{
    IngestStage, MediaError, MediaFormat, TimestampOrigin, decode_mp4, detect_format,
};
use al_testkit::mp4::{
    Mp4EntrySpec, Mp4Spec, Mp4TableSpec, Mp4TrackSpec, mp4_language, write_mp4,
};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("al-mp4-{tag}-{}", al_core::JobId::new()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_fixture(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

/// A `soun` PCM track spec with one chunk of `pcm` bytes.
fn soun_pcm_track(
    track_id: u32,
    codec: [u8; 4],
    channels: u16,
    rate: u32,
    pcm: &[u8],
) -> Mp4TrackSpec {
    let frames = pcm.len() as u32 / u32::from(channels * 2).max(1);
    Mp4TrackSpec {
        track_id,
        handler: *b"soun",
        timescale: rate,
        duration: u64::from(frames),
        language: mp4_language(b"und"),
        entry: Mp4EntrySpec::Sound {
            codec,
            channels,
            bits: 16,
            rate,
            version: 0,
            lpcm_flags: 0,
            pcmc: None,
        },
        table: Mp4TableSpec::uniform(pcm.to_vec(), frames, u32::from(channels) * 2, 1),
        extra_trak_boxes: vec![],
    }
}

/// A `vide` track spec with `n` constant-delta samples.
fn vide_track(track_id: u32, codec: [u8; 4], timescale: u32, n: u32, delta: u32) -> Mp4TrackSpec {
    Mp4TrackSpec {
        track_id,
        handler: *b"vide",
        timescale,
        duration: u64::from(n * delta),
        language: mp4_language(b"und"),
        entry: Mp4EntrySpec::Video {
            codec,
            width: 320,
            height: 240,
        },
        table: Mp4TableSpec {
            stts: vec![(n, delta)],
            ctts: None,
            stss: None,
            stsc: vec![(1, n, 1)],
            chunks: vec![vec![0xAB; n as usize * 4]],
            sample_sizes: vec![4; n as usize],
        },
        extra_trak_boxes: vec![],
    }
}

fn base_spec(tracks: Vec<Mp4TrackSpec>) -> Mp4Spec {
    Mp4Spec {
        major_brand: *b"isom",
        compatible_brands: vec![*b"isom", *b"mp41"],
        movie_timescale: 1000,
        tracks,
        extra_top_boxes: vec![],
    }
}

#[test]
fn detects_mp4_and_decodes_track_metadata() {
    let s16: Vec<u8> = (0..48i16).flat_map(|v| v.to_le_bytes()).collect();
    let spec = base_spec(vec![
        vide_track(1, *b"avc1", 90000, 10, 3000),
        soun_pcm_track(2, *b"sowt", 1, 48000, &s16),
    ]);
    let bytes = write_mp4(&spec);
    assert_eq!(detect_format(&bytes), Some(MediaFormat::Mp4));

    let m = decode_mp4(&bytes).unwrap();
    assert_eq!(m.facts.brands.major_brand, "isom");
    assert_eq!(m.facts.brands.compatible_brands, vec!["isom", "mp41"]);
    assert_eq!(m.facts.movie_timescale, Some(1000));
    assert_eq!(m.facts.tracks.len(), 2);

    let v = &m.facts.tracks[0];
    assert_eq!(v.handler, "vide");
    assert_eq!(v.codec.as_deref(), Some("avc1"));
    assert_eq!(v.timescale, 90000);
    assert_eq!(v.track_id, Some(1));
    assert_eq!(v.sample_count, 10);
    assert_eq!(v.video.as_ref().unwrap().width, 320);

    let a = &m.facts.tracks[1];
    assert_eq!(a.handler, "soun");
    assert_eq!(a.codec.as_deref(), Some("sowt"));
    assert_eq!(a.audio.as_ref().unwrap().sample_rate, 48000);
}

#[test]
fn stts_decode_order_timing_is_recovered() {
    let spec = base_spec(vec![vide_track(1, *b"avc1", 90000, 10, 3000)]);
    let m = decode_mp4(&write_mp4(&spec)).unwrap();

    let t = m
        .timing
        .iter()
        .find(|t| t.timestamp_origin == TimestampOrigin::SampleTable)
        .expect("sample_table timing");
    assert_eq!(t.stream_type, "vide");
    assert_eq!(t.observation.timebase_hz, 90000);
    assert_eq!(
        t.stream_timestamps,
        (0..10).map(|i| i * 3000).collect::<Vec<_>>()
    );
    assert!(t.observation.discontinuities.is_empty());
}

#[test]
fn vfr_stts_is_observed_without_nominal_fps() {
    // VFR: per-sample deltas alternating 2000/3000 ticks — variation,
    // not a regime shift, so no discontinuities.
    let mut spec = base_spec(vec![]);
    let mut t = vide_track(1, *b"avc1", 90000, 0, 0);
    t.table = Mp4TableSpec {
        stts: vec![
            (1, 2000),
            (1, 3000),
            (1, 2000),
            (1, 3000),
            (1, 2000),
            (1, 3000),
            (1, 2000),
            (1, 3000),
            (1, 2000),
            (1, 3000),
        ],
        ctts: None,
        stss: None,
        stsc: vec![(1, 10, 1)],
        chunks: vec![vec![0u8; 40]],
        sample_sizes: vec![4; 10],
    };
    spec.tracks.push(t);
    let m = decode_mp4(&write_mp4(&spec)).unwrap();

    let v = m
        .timing
        .iter()
        .find(|t| t.timestamp_origin == TimestampOrigin::SampleTable)
        .unwrap();
    let expect: Vec<i64> = [
        0, 2000, 5000, 7000, 10000, 12000, 15000, 17000, 20000, 22000,
    ]
    .into_iter()
    .collect();
    assert_eq!(v.stream_timestamps, expect);
    // Mixed intervals are VFR facts, not discontinuities.
    assert!(v.observation.discontinuities.is_empty());
    let stats = v.observation.interval_stats.as_ref().unwrap();
    assert_eq!(stats.min_ticks, 2000);
    assert_eq!(stats.max_ticks, 3000);
}

#[test]
fn ctts_reorder_keeps_dts_series_clean_and_records_pts() {
    // B-frame-like reorder: ctts offsets make decode order differ from
    // presentation order.
    let mut t = vide_track(1, *b"avc1", 90000, 4, 3000);
    t.table.ctts = Some(vec![(1, 3000), (1, 0), (1, 0), (1, 0)]);
    // dts = 0,3000,6000,9000; pts = 3000,3000,6000,9000 — a duplicate,
    // the classic one-frame-delay reorder.
    let spec = base_spec(vec![t]);
    let m = decode_mp4(&write_mp4(&spec)).unwrap();

    let dts = m
        .timing
        .iter()
        .find(|t| t.timestamp_origin == TimestampOrigin::SampleTable)
        .expect("dts series");
    // The observed series is decode-order DTS: constant deltas, no
    // false discontinuity from the reorder.
    assert_eq!(dts.stream_timestamps, vec![0, 3000, 6000, 9000]);
    assert!(dts.observation.discontinuities.is_empty());

    let pts = m
        .timing
        .iter()
        .find(|t| t.timestamp_origin == TimestampOrigin::SampleTablePts)
        .expect("pts series");
    let mut expected = vec![3000i64, 3000, 6000, 9000];
    expected.sort_unstable();
    assert_eq!(pts.stream_timestamps, expected);
}

#[test]
fn sowt_and_twos_decode_with_correct_endianness() {
    let le_samples: Vec<i16> = vec![0, 1000, -1000, 32767, -32768, 42];
    let le: Vec<u8> = le_samples.iter().flat_map(|v| v.to_le_bytes()).collect();
    let be: Vec<u8> = le_samples.iter().flat_map(|v| v.to_be_bytes()).collect();
    let expected: Vec<f64> = le_samples.iter().map(|&v| f64::from(v)).collect();

    let m = decode_mp4(&write_mp4(&base_spec(vec![soun_pcm_track(
        1, *b"sowt", 2, 48000, &le,
    )])))
    .unwrap();
    let a = m.audio.as_ref().expect("sowt decoded");
    assert_eq!(a.codec, "pcm_s16le");
    assert_eq!(a.channels, 2);
    assert_eq!(a.samples, expected);

    let m = decode_mp4(&write_mp4(&base_spec(vec![soun_pcm_track(
        1, *b"twos", 2, 48000, &be,
    )])))
    .unwrap();
    let a = m.audio.as_ref().expect("twos decoded");
    assert_eq!(a.codec, "pcm_s16be");
    assert_eq!(a.samples, expected);
}

#[test]
fn lpcm_v2_flags_select_endianness() {
    let samples: Vec<i16> = vec![1, -2, 300, -400];
    let le: Vec<u8> = samples.iter().flat_map(|v| v.to_le_bytes()).collect();
    // lpcm v2 entry, flags: isSignedInteger|isPacked (LE).
    let mut t = soun_pcm_track(1, *b"lpcm", 2, 48000, &le);
    t.entry = Mp4EntrySpec::Sound {
        codec: *b"lpcm",
        channels: 2,
        bits: 16,
        rate: 48000,
        version: 2,
        lpcm_flags: 4 | 8,
        pcmc: None,
    };
    let m = decode_mp4(&write_mp4(&base_spec(vec![t]))).unwrap();
    let a = m.audio.as_ref().expect("lpcm decoded");
    assert_eq!(a.codec, "pcm_s16le");
    assert_eq!(
        a.samples,
        samples.iter().map(|&v| f64::from(v)).collect::<Vec<_>>()
    );
}

#[test]
fn ipcm_decodes_via_pcmc_flags() {
    let samples: Vec<i16> = vec![5, -6, 700];
    let le: Vec<u8> = samples.iter().flat_map(|v| v.to_le_bytes()).collect();
    let mut t = soun_pcm_track(1, *b"ipcm", 1, 48000, &le);
    t.entry = Mp4EntrySpec::Sound {
        codec: *b"ipcm",
        channels: 1,
        bits: 16,
        rate: 48000,
        version: 0,
        lpcm_flags: 0,
        pcmc: Some((1, 16)), // little_endian flag, 16-bit
    };
    let m = decode_mp4(&write_mp4(&base_spec(vec![t]))).unwrap();
    let a = m.audio.as_ref().expect("ipcm decoded");
    assert_eq!(a.codec, "pcm_s16le");
    assert_eq!(
        a.samples,
        samples.iter().map(|&v| f64::from(v)).collect::<Vec<_>>()
    );
}

#[test]
fn compressed_soun_is_an_integrity_observation_not_a_failure() {
    // soun mp4a + vide avc1: no PCM anywhere, demux still succeeds.
    let aac = soun_pcm_track(2, *b"mp4a", 2, 48000, &[0u8; 24]);
    let spec = base_spec(vec![vide_track(1, *b"avc1", 90000, 5, 3000), aac]);
    let m = decode_mp4(&write_mp4(&spec)).unwrap();

    assert!(m.audio.is_none());
    assert_eq!(m.integrity.len(), 1);
    assert_eq!(m.integrity[0].kind, "decode/input unavailable");
    assert_eq!(m.integrity[0].detail["codec"], "mp4a");
    // Timing for both tracks is still observed.
    assert_eq!(
        m.timing
            .iter()
            .filter(|t| t.timestamp_origin == TimestampOrigin::SampleTable)
            .count(),
        2
    );
}

#[test]
fn first_pcm_decodable_soun_track_wins() {
    // Track 1: mp4a (compressed). Track 2: sowt PCM — decoded; track 1
    // is metadata + integrity observation only.
    let pcm: Vec<u8> = (0..24i16).flat_map(|v| v.to_le_bytes()).collect();
    let spec = base_spec(vec![
        soun_pcm_track(1, *b"mp4a", 2, 48000, &[0u8; 16]),
        soun_pcm_track(2, *b"sowt", 2, 48000, &pcm),
    ]);
    let m = decode_mp4(&write_mp4(&spec)).unwrap();
    let a = m.audio.as_ref().expect("second soun decoded");
    assert_eq!(a.codec, "pcm_s16le");
    assert_eq!(a.samples.len(), 24);
    assert_eq!(m.integrity.len(), 1);
    assert_eq!(m.integrity[0].detail["track_index"], 0);
}

#[test]
fn fragmented_mp4_fails_explicitly() {
    let mut spec = base_spec(vec![vide_track(1, *b"avc1", 90000, 2, 3000)]);
    spec.extra_top_boxes.push((*b"moof", vec![0u8; 8]));
    let err = decode_mp4(&write_mp4(&spec)).unwrap_err();
    assert!(matches!(err, MediaError::InputUnavailable { .. }));
    assert!(err.to_string().contains("fragmented"));
}

#[test]
fn unknown_boxes_are_skipped_and_recorded() {
    let mut spec = base_spec(vec![vide_track(1, *b"avc1", 90000, 2, 3000)]);
    spec.extra_top_boxes.push((*b"zzzz", vec![1, 2, 3]));
    spec.tracks[0]
        .extra_trak_boxes
        .push((*b"wxyz", vec![9u8; 4]));
    let m = decode_mp4(&write_mp4(&spec)).unwrap();
    assert!(m.facts.skipped_boxes.contains(&"zzzz".to_string()));
    assert!(
        m.facts
            .skipped_boxes
            .contains(&"moov/trak/wxyz".to_string())
    );
}

#[test]
fn missing_moov_is_malformed() {
    // ftyp + mdat only: no moov.
    let spec = Mp4Spec {
        major_brand: *b"isom",
        compatible_brands: vec![],
        movie_timescale: 0,
        tracks: vec![],
        extra_top_boxes: vec![],
    };
    let err = decode_mp4(&write_mp4(&spec)).unwrap_err();
    assert!(matches!(err, MediaError::Malformed { .. }));
}

#[test]
fn no_leading_ftyp_is_input_unavailable() {
    let err = decode_mp4(b"\x00\x00\x00\x08free").unwrap_err();
    assert!(matches!(err, MediaError::InputUnavailable { .. }));
}

#[test]
fn malformed_sample_table_fails() {
    // stts declares 5 samples but stsz only 4.
    let mut t = vide_track(1, *b"avc1", 90000, 5, 3000);
    t.table.sample_sizes = vec![4; 4];
    let err = decode_mp4(&write_mp4(&base_spec(vec![t]))).unwrap_err();
    assert!(matches!(err, MediaError::Malformed { .. }));
}

#[test]
fn stss_sync_samples_are_recorded() {
    let mut t = vide_track(1, *b"avc1", 90000, 6, 3000);
    t.table.stss = Some(vec![1, 4]);
    let m = decode_mp4(&write_mp4(&base_spec(vec![t]))).unwrap();
    assert_eq!(m.facts.tracks[0].sync_samples, Some(vec![1, 4]));
}

#[test]
fn multi_chunk_sample_map_locates_pcm() {
    // PCM split across two chunks via stsc.
    let pcm: Vec<u8> = (0..16i16).flat_map(|v| v.to_le_bytes()).collect();
    let (c0, c1) = (pcm[..16].to_vec(), pcm[16..].to_vec());
    let mut t = soun_pcm_track(1, *b"sowt", 2, 48000, &[]);
    t.table = Mp4TableSpec {
        stts: vec![(8, 1)],
        ctts: None,
        stss: None,
        stsc: vec![(1, 4, 1), (2, 4, 1)],
        chunks: vec![c0, c1],
        sample_sizes: vec![4; 8],
    };
    let m = decode_mp4(&write_mp4(&base_spec(vec![t]))).unwrap();
    let a = m.audio.unwrap();
    let expected: Vec<f64> = (0..16i16).map(f64::from).collect();
    assert_eq!(a.samples, expected);
}

fn run_ingest(job_dir: &Path, inputs: Vec<al_job::job::JobInput>) -> al_job::manifest::Manifest {
    let runner = StageRunner::new(job_dir);
    let def =
        JobDefinition::create(inputs, serde_json::json!({}), vec![StageKind::Ingest]).unwrap();
    let mut stages: Vec<Box<dyn Stage>> = vec![Box::new(IngestStage::new())];
    runner.run(&def, &mut stages).unwrap()
}

fn read_metadata(job_dir: &Path, manifest: &al_job::manifest::Manifest) -> serde_json::Value {
    let runner = StageRunner::new(job_dir);
    let rec = manifest
        .artifacts
        .iter()
        .find(|a| a.role == "ingest_metadata")
        .expect("ingest_metadata artifact");
    let path = runner.job_dir(&manifest.job_id).join(&rec.path);
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn ingest_mp4_produces_pcm_metadata_and_timing_artifacts() {
    let dir = temp_dir("ingest");
    let pcm: Vec<u8> = (0..960i16).flat_map(|v| v.to_le_bytes()).collect();
    let bytes = write_mp4(&base_spec(vec![
        vide_track(1, *b"avc1", 90000, 20, 3000),
        soun_pcm_track(2, *b"sowt", 2, 48000, &pcm),
    ]));
    let mp4 = write_fixture(&dir, "cap.mp4", &bytes);

    let job_dir = temp_dir("job");
    let manifest = run_ingest(&job_dir, vec![input(InputRole::CaptureBug, &mp4)]);
    assert_eq!(
        manifest.stage(StageKind::Ingest).unwrap().status,
        StageStatus::Completed
    );

    let meta = read_metadata(&job_dir, &manifest);
    assert_eq!(meta["container"], "mp4");
    assert_eq!(meta["audio"]["codec"], "pcm_s16le");
    assert_eq!(meta["audio"]["sample_rate"], 48000);
    assert_eq!(meta["mp4"]["brands"]["major_brand"], "isom");
    let tracks = meta["mp4"]["tracks"].as_array().unwrap();
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0]["handler"], "vide");
    assert_eq!(tracks[0]["codec"], "avc1");
    assert_eq!(tracks[1]["codec"], "sowt");

    let timing = meta["av_timing"].as_array().expect("av_timing array");
    let v = timing
        .iter()
        .find(|t| t["timestamp_origin"] == "sample_table" && t["stream_type"] == "vide")
        .expect("video timing");
    assert_eq!(v["observation"]["clock"], "video");
    assert_eq!(v["observation"]["timestamp_count"], 20);
    assert_eq!(v["stream_timestamps"].as_array().unwrap().len(), 20);

    // decoded_pcm artifact present for the PCM track.
    assert!(manifest.artifacts.iter().any(|a| a.role == "decoded_pcm"));

    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_dir_all(&job_dir).ok();
}

#[test]
fn compressed_only_mp4_completes_with_metadata_and_no_pcm() {
    let dir = temp_dir("compressed");
    let bytes = write_mp4(&base_spec(vec![
        vide_track(1, *b"avc1", 90000, 30, 3000),
        soun_pcm_track(2, *b"mp4a", 2, 48000, &[0u8; 48]),
    ]));
    let mp4 = write_fixture(&dir, "cap.mp4", &bytes);

    let job_dir = temp_dir("job-c");
    let manifest = run_ingest(&job_dir, vec![input(InputRole::CaptureBug, &mp4)]);
    // Partial ingest: the stage completes.
    assert_eq!(
        manifest.stage(StageKind::Ingest).unwrap().status,
        StageStatus::Completed
    );
    // No decoded_pcm artifact, but ingest_metadata persists.
    assert!(!manifest.artifacts.iter().any(|a| a.role == "decoded_pcm"));
    let meta = read_metadata(&job_dir, &manifest);
    assert!(meta["audio"].is_null());
    assert_eq!(meta["mp4"]["tracks"].as_array().unwrap().len(), 2);
    let integrity = meta["integrity"].as_array().expect("integrity array");
    assert_eq!(integrity.len(), 1);
    assert_eq!(integrity[0]["kind"], "decode/input unavailable");
    assert_eq!(integrity[0]["detail"]["codec"], "mp4a");
    // Timing artifacts persist independently of PCM.
    assert!(!meta["av_timing"].as_array().unwrap().is_empty());

    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_dir_all(&job_dir).ok();
}

#[test]
fn failed_input_does_not_block_other_inputs() {
    // A compressed-only MP4 still completes; a good WAV in the same
    // job keeps its PCM — per-input isolation by construction.
    let dir = temp_dir("mixed");
    let mp4_bytes = write_mp4(&base_spec(vec![soun_pcm_track(
        1, *b"mp4a", 2, 48000, &[0u8; 24],
    )]));
    let mp4 = write_fixture(&dir, "a.mp4", &mp4_bytes);
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&44u32.to_le_bytes());
    wav.extend_from_slice(
        b"WAVEfmt \x10\0\0\0\x01\0\x01\0\x80\xbb\0\0\0\x77\x01\0\x02\0\x10\0data\x08\0\0\0",
    );
    wav.extend_from_slice(&[1u8, 0, 2, 0, 3, 0, 4, 0]);
    let wavp = write_fixture(&dir, "b.wav", &wav);

    let job_dir = temp_dir("job-m");
    let manifest = run_ingest(
        &job_dir,
        vec![
            input(InputRole::CaptureBug, &mp4),
            input(InputRole::CaptureReference, &wavp),
        ],
    );
    assert_eq!(
        manifest.stage(StageKind::Ingest).unwrap().status,
        StageStatus::Completed
    );
    // One decoded_pcm (the WAV), two ingest_metadata artifacts.
    assert_eq!(
        manifest
            .artifacts
            .iter()
            .filter(|a| a.role == "decoded_pcm")
            .count(),
        1
    );
    assert_eq!(
        manifest
            .artifacts
            .iter()
            .filter(|a| a.role == "ingest_metadata")
            .count(),
        2
    );

    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_dir_all(&job_dir).ok();
}
