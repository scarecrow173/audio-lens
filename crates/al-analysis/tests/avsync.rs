//! End-to-end A/V sync: a synthetic uncompressed AVI runs the real
//! Ingest → CheapAnalysis stages, and the persisted `avsync.*` records
//! must recover the planted audio/video offset.
//!
//! The fixture carries `video_start != 0` so the test also proves the
//! container-level `dwStart` offset cancels out of `avsync.delta`.

use std::fs;
use std::path::PathBuf;

use al_core::{InputRole, JobId, StageKind};
use al_job::job::{JobDefinition, input};
use al_job::runner::StageRunner;
use al_job::stage::Stage;
use al_testkit::media::{AviSpec, AviStreamSpec, MoviPacket, write_avi};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("al-avsync-{tag}-{}", JobId::new()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Mono s16 PCM: silence, then a sustained 440 Hz tone starting at
/// `onset_s` — exactly one cheap onset, at a known position.
fn tone_after_silence(rate: u32, onset_s: f64, total_s: f64) -> Vec<u8> {
    let n = (total_s * rate as f64) as usize;
    let onset = (onset_s * rate as f64) as usize;
    let mut pcm = vec![0i16; n];
    for (i, v) in pcm.iter_mut().enumerate().skip(onset) {
        *v = (0.5 * (2.0 * std::f64::consts::PI * 440.0 * i as f64 / rate as f64).sin() * 32767.0)
            as i16;
    }
    pcm.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn find<'a>(records: &'a [serde_json::Value], id: &str) -> &'a serde_json::Value {
    records
        .iter()
        .find(|r| r["feature_id"] == id)
        .unwrap_or_else(|| panic!("missing record {id}"))
}

fn scalar(rec: &serde_json::Value) -> f64 {
    rec["value"]["scalar"]
        .as_f64()
        .unwrap_or_else(|| panic!("record {} is not a scalar", rec["feature_id"]))
}

#[test]
fn ingest_then_cheap_recovers_planted_av_offset() {
    // 4 s of 10 fps video: dark frames 0-14, bright 15-39 → a scene
    // change (and luminance step) at frame 15 → 1.5 s on the vids
    // clock. dwStart = 5 ticks declares a container-level offset that
    // must cancel out. Audio: tone onset at 1.3 s → delta +0.2 s.
    let fps = 10u32;
    let dark = vec![0u8; 64 * 48 * 3];
    let bright = vec![200u8; 64 * 48 * 3];
    let frames: Vec<Vec<u8>> = (0..40)
        .map(|f| if f < 15 { dark.clone() } else { bright.clone() })
        .collect();
    let rate = 48_000u32;
    let audio = tone_after_silence(rate, 1.3, 4.0);
    let mut packets: Vec<MoviPacket> = frames
        .into_iter()
        .map(|f| MoviPacket::video_frame(0, f))
        .collect();
    packets.push(MoviPacket::audio(1, audio));
    let avi_bytes = write_avi(&AviSpec {
        width: 64,
        height: 48,
        usec_per_frame: 1_000_000 / fps,
        total_frames: None,
        streams: vec![
            AviStreamSpec::video(64, 48, fps).with_start(5),
            AviStreamSpec::pcm_audio(1, rate),
        ],
        packets,
    });

    let dir = temp_dir("e2e");
    let avi = dir.join("cap.avi");
    fs::write(&avi, &avi_bytes).unwrap();

    let job_dir = temp_dir("job");
    let runner = StageRunner::new(&job_dir);
    let def = JobDefinition::create(
        vec![input(InputRole::CaptureBug, &avi)],
        serde_json::json!({}),
        vec![StageKind::Ingest, StageKind::CheapAnalysis],
    )
    .unwrap();
    let mut stages: Vec<Box<dyn Stage>> = vec![
        Box::new(al_media::IngestStage::new()),
        Box::new(al_analysis::stage::CheapAnalysisStage::new()),
    ];
    let manifest = runner.run(&def, &mut stages).expect("ingest + cheap");

    // The ingest stage persisted decoded video features.
    let vf_artifact = manifest
        .artifacts
        .iter()
        .find(|a| a.role == "video_features")
        .expect("video_features artifact");
    let vf: al_media::VideoFeatureReport = serde_json::from_str(
        &fs::read_to_string(runner.job_dir(&manifest.job_id).join(&vf_artifact.path))
            .expect("video_features readable"),
    )
    .expect("video_features parses");
    let al_media::VideoFeatureArtifact::Decoded(f) = &vf.features else {
        panic!("uncompressed AVI must decode video features")
    };
    assert_eq!(f.start_tick, 5);
    let scene = f
        .events
        .iter()
        .find(|e| e.kind == al_media::VideoEventKind::SceneChange)
        .expect("scene change detected");
    assert_eq!(scene.time_ticks, 5 + 15); // raw stream clock keeps dwStart

    // The cheap feature file carries the avsync records.
    let input_id = &def.inputs[0].input_id;
    let feature_path = runner
        .job_dir(&manifest.job_id)
        .join("analysis")
        .join("features")
        .join("cheap")
        .join(format!("{input_id}.json"));
    let body: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&feature_path).expect("cheap records"))
            .expect("cheap records parse");
    let records = body["records"].as_array().expect("records array");

    // The tone edge produces one or two onsets (10 ms hop
    // quantization across the rising edge); every one must land on
    // the planted A/V offset.
    let onset = find(records, "trans.onset_times");
    assert!(onset["support_count"].as_u64().unwrap() >= 1);

    // Matched pair(s), delta ≈ +0.2 s, container offset cancelled.
    let count = find(records, "avsync.pair_count");
    assert!(scalar(count) >= 1.0);
    let deltas: Vec<f64> = records
        .iter()
        .filter(|r| r["feature_id"] == "avsync.delta")
        .map(scalar)
        .collect();
    assert!(!deltas.is_empty());
    for d in &deltas {
        assert!((d - 0.2).abs() <= 0.02, "delta {d} should be ≈0.2 s");
    }
    let delta = find(records, "avsync.delta");
    assert_eq!(delta["validity"], "valid");
    let median = find(records, "avsync.median_delta");
    assert!((scalar(median) - 0.2).abs() <= 0.02);
    let audio_time = find(records, "avsync.audio_time");
    assert!((scalar(audio_time) - 1.3).abs() <= 0.02);
    let video_time = find(records, "avsync.video_time");
    assert!((scalar(video_time) - 1.5).abs() < 1e-9);

    fs::remove_dir_all(&dir).ok();
    fs::remove_dir_all(&job_dir).ok();
}

#[test]
fn input_without_video_records_explicit_not_applicable() {
    // A WAV input reaches the same code path but carries no video —
    // the avsync records exist and say `not_applicable` rather than
    // being silently absent.
    let dir = temp_dir("wav");
    let wav_path = dir.join("cap.wav");
    let rate = 48_000u32;
    let audio = tone_after_silence(rate, 0.5, 1.0);
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + audio.len() as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&rate.to_le_bytes());
    wav.extend_from_slice(&(rate * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(audio.len() as u32).to_le_bytes());
    wav.extend_from_slice(&audio);
    fs::write(&wav_path, &wav).unwrap();

    let job_dir = temp_dir("job-wav");
    let runner = StageRunner::new(&job_dir);
    let def = JobDefinition::create(
        vec![input(InputRole::CaptureBug, &wav_path)],
        serde_json::json!({}),
        vec![StageKind::Ingest, StageKind::CheapAnalysis],
    )
    .unwrap();
    let mut stages: Vec<Box<dyn Stage>> = vec![
        Box::new(al_media::IngestStage::new()),
        Box::new(al_analysis::stage::CheapAnalysisStage::new()),
    ];
    let manifest = runner.run(&def, &mut stages).expect("ingest + cheap");
    assert!(
        !manifest
            .artifacts
            .iter()
            .any(|a| a.role == "video_features")
    );

    let input_id = &def.inputs[0].input_id;
    let feature_path = runner
        .job_dir(&manifest.job_id)
        .join("analysis")
        .join("features")
        .join("cheap")
        .join(format!("{input_id}.json"));
    let body: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&feature_path).expect("cheap records"))
            .expect("cheap records parse");
    let records = body["records"].as_array().expect("records array");
    assert_eq!(
        find(records, "avsync.median_delta")["validity"],
        "not_applicable"
    );
    assert_eq!(scalar(find(records, "avsync.pair_count")), 0.0);

    fs::remove_dir_all(&dir).ok();
    fs::remove_dir_all(&job_dir).ok();
}
