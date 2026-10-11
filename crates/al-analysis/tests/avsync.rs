//! End-to-end A/V sync: a synthetic uncompressed AVI runs the real
//! Ingest → CheapAnalysis stages, and the persisted `avsync.*` records
//! must recover the planted audio/video offset.
//!
//! auds/vids の共通原点だけを相殺し、dwStart の実相対 offset を保持する。

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
    for (video_start, audio_start) in [(0, 0), (5, 0), (5, 24000), (5, 48000)] {
        run_offset_fixture(false, true, video_start, audio_start);
    }
}

#[test]
fn roi_series_tensor_and_event_reach_avsync() {
    for (video_start, audio_start) in [(0, 0), (5, 0), (5, 24000), (5, 48000)] {
        run_offset_fixture(true, true, video_start, audio_start);
    }
}

#[test]
fn roi_tensor_survives_missing_audio() {
    run_offset_fixture(true, false, 5, 0);
}

#[test]
fn supplied_marker_reaches_pairs_with_stream_clock_and_provenance() {
    for (video_start, audio_start) in [(0, 0), (5, 0), (5, 24000), (5, 48000)] {
        run_fixture(false, true, video_start, audio_start, true);
    }
}

fn run_offset_fixture(roi: bool, has_audio: bool, video_start: u32, audio_start: u32) {
    run_fixture(roi, has_audio, video_start, audio_start, false);
}

fn run_fixture(roi: bool, has_audio: bool, video_start: u32, audio_start: u32, marker: bool) {
    // 4 s of 10 fps video: dark frames 0-14, bright 15-39 → a scene
    // change (and luminance step) at frame 15 → 1.5 s on the vids
    // clock. 音声 onset 1.3 s に対し内容差 0.2 s と stream 原点差を検証する。
    let fps = 10u32;
    let dark = vec![0u8; 64 * 48 * 3];
    let mut bright = vec![200u8; 64 * 48 * 3];
    if roi {
        bright.fill(0);
        // DIB は bottom-up。表示上左下の 8 × 2 ピクセルだけ変化。
        for y in 0..2 {
            bright[y * 64 * 3..y * 64 * 3 + 8 * 3].fill(200);
        }
    }
    let frames: Vec<Vec<u8>> = (0..40)
        .map(|f| {
            if marker || f < 15 {
                dark.clone()
            } else {
                bright.clone()
            }
        })
        .collect();
    let rate = 48_000u32;
    let relative_start = video_start as f64 / fps as f64 - audio_start as f64 / rate as f64;
    let expected_delta = 0.2 + relative_start;
    let audio = tone_after_silence(rate, 1.3, 4.0);
    let mut packets: Vec<MoviPacket> = frames
        .into_iter()
        .map(|f| MoviPacket::video_frame(0, f))
        .collect();
    if has_audio {
        packets.push(MoviPacket::audio(1, audio));
    }
    let avi_bytes = write_avi(&AviSpec {
        width: 64,
        height: 48,
        usec_per_frame: 1_000_000 / fps,
        total_frames: None,
        streams: vec![
            AviStreamSpec::video(64, 48, fps).with_start(video_start),
            AviStreamSpec::pcm_audio(1, rate).with_start(audio_start),
        ],
        packets,
    });

    let dir = temp_dir("e2e");
    let avi = dir.join("cap.avi");
    fs::write(&avi, &avi_bytes).unwrap();

    let job_dir = temp_dir("job");
    let runner = StageRunner::new(&job_dir);
    let mut def = JobDefinition::create(
        vec![input(InputRole::CaptureBug, &avi)],
        if roi {
            serde_json::json!({"analysis": {"avsync": {"match_window_s": 1.0}}, "ingest": {"video": {"rois": [
                {"name": "字幕/表示", "rect": {"x": 0, "y": 46, "width": 8, "height": 2}}
            ]}}})
        } else {
            serde_json::json!({"analysis": {"avsync": {"match_window_s": 1.0}}})
        },
        vec![StageKind::Ingest, StageKind::CheapAnalysis],
    )
    .unwrap();
    if marker {
        def.config["analysis"]["avsync"]["markers"] = serde_json::json!([{
            "input_id": def.inputs[0].input_id.to_string(),
            "clock": "video_stream", "time_s": 1.3 + video_start as f64 / fps as f64,
            "frame": 13
        }]);
    }
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
    assert_eq!(f.start_tick, i64::from(video_start));
    if !marker {
        let scene = f
            .events
            .iter()
            .find(|e| {
                e.kind
                    == if roi {
                        al_media::VideoEventKind::LuminanceStep
                    } else {
                        al_media::VideoEventKind::SceneChange
                    }
            })
            .expect("scene change detected");
        assert_eq!(scene.time_ticks, i64::from(video_start) + 15); // raw stream clock keeps dwStart
    }

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

    if marker {
        let delta = records
            .iter()
            .find(|r| {
                r["feature_id"] == "avsync.delta" && r["params"]["event_kind"] == "user_marker"
            })
            .expect("user marker pair");
        assert!((scalar(delta) - relative_start).abs() <= 0.02);
        assert_eq!(delta["params"]["marker"]["input_id"], input_id.to_string());
        assert_eq!(delta["params"]["marker"]["clock"], "video_stream");
        assert_eq!(
            delta["params"]["marker"]["config_path"],
            "analysis.avsync.markers[0]"
        );
        assert_eq!(delta["params"]["event_frame"], 13);
        fs::remove_dir_all(&dir).ok();
        fs::remove_dir_all(&job_dir).ok();
        return;
    }
    if roi {
        let record = find(records, "video.roi_luminance");
        assert_eq!(record["params"]["roi"]["name"], "字幕/表示");
        assert_eq!(record["support_count"], 40);
        let array = record["value"]["tensor"]["array"].as_str().unwrap();
        let reader = al_dataset::reader::DatasetReader::open(
            &runner.job_dir(&manifest.job_id).join("analysis"),
        )
        .unwrap();
        let name = array.strip_prefix("features/cheap/").unwrap();
        let (data, _) = reader
            .read_tensor("features/cheap", name, &[[0, 1], [0, 40]])
            .unwrap();
        assert!(data[..15].iter().all(|v| v.abs() < 1e-9));
        assert!(data[15..].iter().all(|v| (v - 200.0).abs() < 1e-9));
        let times = find(records, "video.frame_times");
        let time_array = times["value"]["tensor"]["array"].as_str().unwrap();
        let (times, _) = reader
            .read_tensor(
                "features/cheap",
                time_array.strip_prefix("features/cheap/").unwrap(),
                &[[0, 1], [0, 40]],
            )
            .unwrap();
        assert_eq!(times[15], 1.5);
        if has_audio {
            assert_eq!(
                find(records, "avsync.delta")["params"]["roi_name"],
                "字幕/表示"
            );
        }
        assert!(f.luminance[15] < 32.0);
    } else {
        assert!(f.roi_luminance.is_empty());
        assert!(
            !records
                .iter()
                .any(|r| r["feature_id"] == "video.roi_luminance")
        );
    }

    if !has_audio {
        assert_eq!(
            find(records, "avsync.pair_count")["validity"],
            "decode_input_unavailable"
        );
        fs::remove_dir_all(&dir).ok();
        fs::remove_dir_all(&job_dir).ok();
        return;
    }
    // The tone edge produces one or two onsets (10 ms hop
    // quantization across the rising edge); every one must land on
    // the planted A/V offset.
    let onset = find(records, "trans.onset_times");
    assert!(onset["support_count"].as_u64().unwrap() >= 1);

    // ペア delta は内容差 0.2 s と stream 原点差を含む。
    let count = find(records, "avsync.pair_count");
    assert!(scalar(count) >= 1.0);
    let deltas: Vec<f64> = records
        .iter()
        .filter(|r| r["feature_id"] == "avsync.delta")
        .map(scalar)
        .collect();
    assert!(!deltas.is_empty());
    for d in &deltas {
        assert!(
            (d - expected_delta).abs() <= 0.02,
            "delta {d} should be {expected_delta} s"
        );
    }
    let delta = find(records, "avsync.delta");
    assert_eq!(delta["validity"], "valid");
    assert_eq!(delta["params"]["clock_origin"], "audio_pcm_start");
    assert_eq!(
        delta["params"]["audio_start_s"],
        audio_start as f64 / rate as f64
    );
    let median = find(records, "avsync.median_delta");
    assert!((scalar(median) - expected_delta).abs() <= 0.02);
    let audio_time = find(records, "avsync.audio_time");
    assert!((scalar(audio_time) - 1.3).abs() <= 0.02);
    let video_time = find(records, "avsync.video_time");
    assert!((scalar(video_time) - (1.5 + relative_start)).abs() < 1e-9);

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
    let wav = wav_bytes(rate, &audio);
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

#[test]
fn marker_config_rejects_invalid_schema_and_clock() {
    use al_analysis::avsync::AvSyncConfig;
    for markers in [
        serde_json::json!(null),
        serde_json::json!({}),
        serde_json::json!([{"input_id":"IN-0000000000000000000000","clock":"wall_clock","time_s":1.0,"frame":0}]),
        serde_json::json!([{"input_id":"IN-0000000000000000000000","time_s":1.0,"frame":0}]),
        serde_json::json!([{"input_id":"","clock":"video_stream","time_s":1.0,"frame":0}]),
        serde_json::json!([{"input_id":"IN-0000000000000000000000","clock":"video_stream","time_s":null,"frame":0}]),
        serde_json::json!([{"input_id":"IN-0000000000000000000000","clock":"video_stream","time_s":"NaN","frame":0}]),
        serde_json::json!([{"input_id":"IN-0000000000000000000000","clock":"video_stream","time_s":1.0,"frame":-1}]),
        serde_json::json!([{"input_id":"IN-0000000000000000000000","clock":"video_stream","time_s":1.0,"frame":0,"game":"test"}]),
    ] {
        assert!(
            AvSyncConfig::from_job(&serde_json::json!({"analysis":{"avsync":{"markers":markers}}}))
                .is_err()
        );
    }
    assert_eq!(
        AvSyncConfig::from_job(&serde_json::json!({})).unwrap(),
        AvSyncConfig::from_job(&serde_json::json!({"analysis":{"avsync":{"markers":[]}}})).unwrap()
    );
}

#[test]
fn marker_boundary_checks_input_frame_and_time_and_isolates_inputs() {
    for case in [
        "valid",
        "unknown_input",
        "wrong_time",
        "outside_frame",
        "missing_video",
    ] {
        let dir = temp_dir(case);
        let avi = dir.join("cap.avi");
        fs::write(
            &avi,
            write_avi(&AviSpec {
                width: 64,
                height: 48,
                usec_per_frame: 100_000,
                total_frames: None,
                streams: vec![
                    AviStreamSpec::video(64, 48, 10).with_start(5),
                    AviStreamSpec::pcm_audio(1, 48_000).with_start(24_000),
                ],
                packets: (0..40)
                    .map(|_| MoviPacket::video_frame(0, vec![0; 64 * 48 * 3]))
                    .chain([MoviPacket::audio(1, tone_after_silence(48_000, 1.3, 4.0))])
                    .collect(),
            }),
        )
        .unwrap();
        let source = if case == "missing_video" {
            let wav = dir.join("cap.wav");
            fs::write(&wav, wav_bytes(48_000, &vec![0; 9600])).unwrap();
            wav
        } else {
            avi
        };
        let inputs = vec![
            input(InputRole::CaptureBug, &source),
            input(InputRole::CaptureReference, &source),
        ];
        let marker_id = if case == "unknown_input" {
            "IN-unknown".into()
        } else {
            inputs[0].input_id.to_string()
        };
        let def = JobDefinition::create(inputs, serde_json::json!({"analysis":{"avsync":{"markers":[{
            "input_id":marker_id,"clock":"video_stream", "time_s":if case == "wrong_time" {1.3} else {1.8},
            "frame":if case == "outside_frame" {400} else {13}
        }]}}}), vec![StageKind::Ingest, StageKind::CheapAnalysis]).unwrap();
        let runner = StageRunner::new(dir.join("jobs"));
        let mut stages: Vec<Box<dyn Stage>> = vec![
            Box::new(al_media::IngestStage::new()),
            Box::new(al_analysis::stage::CheapAnalysisStage::new()),
        ];
        let result = runner.run(&def, &mut stages);
        if case == "valid" {
            let manifest = result.unwrap();
            for (i, input) in def.inputs.iter().enumerate() {
                let path = runner
                    .job_dir(&manifest.job_id)
                    .join("analysis/features/cheap")
                    .join(format!("{}.json", input.input_id));
                let body: serde_json::Value =
                    serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
                let records = body["records"].as_array().unwrap();
                assert_eq!(
                    records
                        .iter()
                        .filter(|r| r["feature_id"] == "avsync.delta")
                        .count(),
                    usize::from(i == 0)
                );
                if i == 0 {
                    let delta = find(records, "avsync.delta");
                    assert!((scalar(delta)).abs() < 0.02);
                    assert_eq!(delta["params"]["marker"]["time_s"], 1.8);
                    assert_eq!(delta["lineage"].as_array().unwrap().len(), 3);
                }
            }
        } else {
            assert!(result.is_err(), "{case} must fail closed");
        }
        fs::remove_dir_all(dir).unwrap();
    }
}

fn wav_bytes(rate: u32, audio: &[u8]) -> Vec<u8> {
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
    wav.extend_from_slice(audio);
    wav
}
