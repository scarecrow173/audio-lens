//! AAC-LC in MP4 decode tests: real ffmpeg-generated fixtures exercise the
//! Ruviform media-I/O decode path (Issue #100) while PCM and broken
//! containers pin the boundary contracts (attempts, stream results).

use std::path::{Path, PathBuf};

use al_core::{InputRole, StageKind, StageStatus};
use al_job::job::{JobDefinition, input};
use al_job::runner::StageRunner;
use al_job::stage::Stage;
use al_media::decoder::{DecodeRequest, DecoderRegistry};
use al_media::{IngestStage, SampleFormat};
use al_testkit::mp4::{Mp4EntrySpec, Mp4Spec, Mp4TableSpec, Mp4TrackSpec, mp4_language, write_mp4};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("al-aac-{tag}-{}", al_core::JobId::new()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn registry() -> DecoderRegistry {
    DecoderRegistry::ruviform()
}

fn fixture(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(path).unwrap()
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
fn aac_lc_mp4_decodes_audio_via_ruviform() {
    let bytes = fixture("aac_lc_48k_stereo.mp4");
    let result = registry()
        .decode(&bytes, &DecodeRequest::default())
        .expect("decode completes");
    let audio = result.audio.expect("audio decoded");
    assert_eq!(audio.codec, "aac");
    assert_eq!(audio.sample_format, SampleFormat::Float32);
    assert_eq!(audio.sample_rate, 48000);
    assert_eq!(audio.channels, 2);
    // 0.25s at 48kHz = 12000 frames after priming/padding trim.
    assert_eq!(audio.frames(), 12000);
    assert_eq!(audio.samples.len(), 24000);
    assert!(audio.samples.iter().all(|s| s.is_finite()));
    // Issue #100: AAC decode happens inside the Ruviform backend, so
    // there is no AudioLens-side symphonia fallback anymore.
    assert!(!result.provenance.fallback);
    assert_eq!(result.provenance.backend, "ruviform");
    assert_eq!(result.provenance.attempts.len(), 1);
    assert_eq!(result.provenance.attempts[0].backend, "ruviform");
    assert_eq!(result.provenance.attempts[0].outcome, "decoded");
    // The decoded audio stream result maps to a native track_index.
    let entry = result
        .provenance
        .streams
        .iter()
        .find(|s| s.kind == "audio" && s.decoded)
        .expect("decoded audio stream result");
    assert!(entry.unavailable.is_none());
    // Native container facts are preserved: mp4 track observations exist.
    let mp4 = result.mp4.expect("native mp4 facts retained");
    assert!(
        mp4.tracks
            .iter()
            .any(|t| t.handler == "soun" && t.codec.as_deref() == Some("mp4a"))
    );
}

#[test]
fn aac_lc_mono_mp4_decodes_audio_via_ruviform() {
    let bytes = fixture("aac_lc_48k_mono.mp4");
    let result = registry()
        .decode(&bytes, &DecodeRequest::default())
        .expect("decode completes");
    let audio = result.audio.expect("audio decoded");
    assert_eq!(audio.codec, "aac");
    assert_eq!(audio.sample_rate, 48000);
    assert_eq!(audio.channels, 1);
    assert_eq!(audio.frames(), 12000);
    assert!(!result.provenance.fallback);
}

#[test]
fn multi_track_mp4_selects_first_decodable_audio() {
    let bytes = fixture("aac_lc_two_tracks.mp4");
    let result = registry()
        .decode(&bytes, &DecodeRequest::default())
        .expect("decode completes");
    let audio = result.audio.expect("audio decoded");
    assert_eq!(audio.codec, "aac");
    // First track: 48kHz mono, 0.2s = 9600 frames.
    assert_eq!(audio.sample_rate, 48000);
    assert_eq!(audio.channels, 1);
    assert_eq!(audio.frames(), 9600);
    // The second audio track is recorded as unavailable.
    let other = result
        .provenance
        .streams
        .iter()
        .filter(|s| s.kind == "audio" && !s.decoded)
        .count();
    assert!(other >= 1);
    assert!(!result.provenance.fallback);
}

#[test]
fn pcm_mp4_decodes_via_ruviform_native_pcm() {
    let pcm: Vec<u8> = (0..480i16).flat_map(|v| v.to_le_bytes()).collect();
    let bytes = write_mp4(&Mp4Spec {
        major_brand: *b"isom",
        compatible_brands: vec![*b"isom", *b"mp41"],
        movie_timescale: 1000,
        tracks: vec![Mp4TrackSpec {
            track_id: 1,
            handler: *b"soun",
            timescale: 48000,
            duration: 480,
            language: mp4_language(b"und"),
            entry: Mp4EntrySpec::Sound {
                codec: *b"sowt",
                channels: 1,
                bits: 16,
                rate: 48000,
                version: 0,
                lpcm_flags: 0,
                pcmc: None,
            },
            table: Mp4TableSpec {
                stts: vec![(480, 1)],
                ctts: None,
                stss: None,
                stsc: vec![(1, 480, 1)],
                chunks: vec![pcm],
                sample_sizes: vec![2; 480],
            },
            extra_trak_boxes: vec![],
        }],
        extra_top_boxes: vec![],
    });
    let result = registry()
        .decode(&bytes, &DecodeRequest::default())
        .expect("decode completes");
    let audio = result.audio.expect("native PCM decoded");
    assert_eq!(audio.codec, "pcm_s16le");
    assert!(!result.provenance.fallback);
    assert_eq!(result.provenance.attempts.len(), 1);
    assert_eq!(result.provenance.attempts[0].backend, "ruviform");
}

#[test]
fn undecodable_mp4_audio_records_attempts_without_failing() {
    // Valid container with an `mp4a` entry but no usable AAC config/data:
    // symphonia cannot decode it, so the result keeps the native partial
    // ingest contract — no audio, fallback=false, both attempts recorded.
    let bytes = write_mp4(&Mp4Spec {
        major_brand: *b"isom",
        compatible_brands: vec![*b"isom", *b"mp41"],
        movie_timescale: 1000,
        tracks: vec![Mp4TrackSpec {
            track_id: 1,
            handler: *b"soun",
            timescale: 48000,
            duration: 24,
            language: mp4_language(b"und"),
            entry: Mp4EntrySpec::Sound {
                codec: *b"mp4a",
                channels: 2,
                bits: 16,
                rate: 48000,
                version: 0,
                lpcm_flags: 0,
                pcmc: None,
            },
            table: Mp4TableSpec {
                stts: vec![(24, 1)],
                ctts: None,
                stss: None,
                stsc: vec![(1, 24, 1)],
                chunks: vec![vec![0u8; 48]],
                sample_sizes: vec![2; 24],
            },
            extra_trak_boxes: vec![],
        }],
        extra_top_boxes: vec![],
    });
    let result = registry()
        .decode(&bytes, &DecodeRequest::default())
        .expect("decode completes without audio");
    assert!(result.audio.is_none());
    assert!(!result.provenance.fallback);
    // One backend attempt: the Ruviform decoder reports the track
    // undecodable and the partial-ingest contract keeps the result.
    assert_eq!(result.provenance.attempts.len(), 1);
    assert_eq!(result.provenance.attempts[0].backend, "ruviform");
    assert_ne!(result.provenance.attempts[0].outcome, "decoded");
}

#[test]
fn ingest_stage_writes_decoded_pcm_for_aac_mp4() {
    let dir = temp_dir("ingest");
    let mp4 = dir.join("capture.mp4");
    std::fs::write(&mp4, fixture("aac_lc_48k_stereo.mp4")).unwrap();

    let job_dir = temp_dir("job");
    let manifest = run_ingest(&job_dir, vec![input(InputRole::CaptureBug, &mp4)]);
    assert_eq!(
        manifest.stage(StageKind::Ingest).unwrap().status,
        StageStatus::Completed
    );

    let meta = read_metadata(&job_dir, &manifest);
    assert_eq!(meta["container"], "mp4");
    assert_eq!(meta["audio"]["codec"], "aac");
    assert_eq!(meta["audio"]["sample_rate"], 48000);
    assert_eq!(meta["audio"]["channels"], 2);
    assert_eq!(meta["audio"]["sample_format"], "float32");
    assert_eq!(meta["audio"]["frames"], 12000);
    assert_eq!(meta["decoder"]["fallback"], false);
    assert_eq!(meta["decoder"]["backend"], "ruviform");
    let attempts = meta["decoder"]["attempts"].as_array().unwrap();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0]["backend"], "ruviform");
    assert!(manifest.artifacts.iter().any(|a| a.role == "decoded_pcm"));

    // mp4 facts and timing observations remain native-owned.
    assert_eq!(meta["mp4"]["brands"]["major_brand"], "isom");

    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_dir_all(&job_dir).ok();
}
