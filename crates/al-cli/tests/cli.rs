//! CLI-level regression test: an external dictionary file is snapshotted
//! into the job config at `run` time, so `resume` stays reproducible even
//! if the file is modified or deleted afterwards.

use std::path::PathBuf;
use std::process::Command;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("al-cli-{tag}-{}", al_core::JobId::new()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn wav_bytes() -> Vec<u8> {
    let frames: Vec<i16> = (0..960).map(|i| i % 200 - 100).collect();
    let data_len = frames.len() as u32 * 2;
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&48000u32.to_le_bytes());
    out.extend_from_slice(&96000u32.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in frames {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

#[test]
fn resume_uses_snapshotted_dictionary_after_source_deleted() {
    let al = env!("CARGO_BIN_EXE_al");
    let dir = temp_dir("dict");
    let wav = dir.join("cap.wav");
    std::fs::write(&wav, wav_bytes()).unwrap();
    let dict = dir.join("dict.json");
    let dict_text = r#"{"version":"ext","entries":[]}"#;
    std::fs::write(&dict, dict_text).unwrap();
    let config = dir.join("config.json");
    std::fs::write(
        &config,
        serde_json::to_string(&serde_json::json!({
            "dictionary": dict.display().to_string(),
        }))
        .unwrap(),
    )
    .unwrap();
    let jobs = dir.join("jobs");

    let out = Command::new(al)
        .args([
            "run",
            "--base-dir",
            jobs.to_str().unwrap(),
            "--input",
            &format!("capture_bug:{}", wav.display()),
            "--stages",
            "ingest",
            "--config",
            config.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "run failed: {:?}", out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let job_id = stdout
        .lines()
        .find_map(|l| l.strip_prefix("job_id="))
        .expect("job_id line");

    // The job manifest pins the dictionary content and its hash.
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(jobs.join(job_id).join("manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        manifest["config"]["dictionary_text"].as_str(),
        Some(dict_text)
    );
    assert_eq!(
        manifest["config"]["dictionary_sha256"].as_str(),
        Some(al_media::sha256_hex(dict_text.as_bytes()).as_str())
    );

    // Deleting the dictionary file must not break resume.
    std::fs::remove_file(&dict).unwrap();
    let out = Command::new(al)
        .args([
            "resume",
            "--base-dir",
            jobs.to_str().unwrap(),
            "--job-id",
            job_id,
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "resume failed after dictionary deleted: {:?}",
        String::from_utf8_lossy(&out.stderr)
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn default_run_snapshots_bundled_dictionary_into_config() {
    let al = env!("CARGO_BIN_EXE_al");
    let dir = temp_dir("embedded");
    let wav = dir.join("cap.wav");
    std::fs::write(&wav, wav_bytes()).unwrap();
    let jobs = dir.join("jobs");

    // No --config: the bundled dictionary is still snapshotted into the
    // job config so resume stays pinned to those exact bytes.
    let out = Command::new(al)
        .args([
            "run",
            "--base-dir",
            jobs.to_str().unwrap(),
            "--input",
            &format!("capture_bug:{}", wav.display()),
            "--stages",
            "ingest",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "run failed: {:?}", out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let job_id = stdout
        .lines()
        .find_map(|l| l.strip_prefix("job_id="))
        .expect("job_id line");
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(jobs.join(job_id).join("manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        manifest["config"]["dictionary_sha256"].as_str(),
        Some(al_media::sha256_hex(al_media::Dictionary::EMBEDDED_JSON.as_bytes()).as_str())
    );

    std::fs::remove_dir_all(&dir).ok();
}

fn wav_from_samples(samples: &[i16]) -> Vec<u8> {
    let data_len = samples.len() as u32 * 2;
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&48000u32.to_le_bytes());
    out.extend_from_slice(&96000u32.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for &s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

fn noise_i16(seed: u64, n: usize, amp: f64) -> Vec<i16> {
    let mut state = seed;
    (0..n)
        .map(|_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let v = ((state >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0;
            (v * amp) as i16
        })
        .collect()
}

fn input_id_of(artifacts: &[serde_json::Value], name: &str) -> String {
    artifacts
        .iter()
        .find(|a| {
            a["role"].as_str() == Some("declared_input")
                && a["params"]["source_path"]
                    .as_str()
                    .is_some_and(|p| p.ends_with(name))
        })
        .and_then(|a| a["params"]["input_id"].as_str())
        .unwrap_or_else(|| panic!("declared input for {name}"))
        .to_owned()
}

/// Run `ingest,locate` over the given `role:path` inputs and return
/// the job directory. `config` is an optional job config JSON file.
fn run_ingest_locate(
    al: &str,
    jobs: &std::path::Path,
    inputs: &str,
    config: Option<&std::path::Path>,
) -> PathBuf {
    let mut args = vec![
        "run".to_string(),
        "--base-dir".to_string(),
        jobs.to_str().unwrap().to_string(),
        "--input".to_string(),
        inputs.to_string(),
        "--stages".to_string(),
        "ingest,locate".to_string(),
    ];
    if let Some(config) = config {
        args.push("--config".to_string());
        args.push(config.to_str().unwrap().to_string());
    }
    let out = Command::new(al).args(&args).output().unwrap();
    assert!(
        out.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let job_id = stdout
        .lines()
        .find_map(|l| l.strip_prefix("job_id="))
        .expect("job_id line");
    jobs.join(job_id)
}

fn manifest_artifacts(job_dir: &std::path::Path) -> Vec<serde_json::Value> {
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(job_dir.join("manifest.json")).unwrap())
            .unwrap();
    manifest["artifacts"].as_array().unwrap().clone()
}

/// Parsed body of the `time_map` artifact for one source input.
fn time_map_body(
    job_dir: &std::path::Path,
    artifacts: &[serde_json::Value],
    source_input_id: &str,
) -> serde_json::Value {
    let rec = artifacts
        .iter()
        .find(|a| {
            a["role"].as_str() == Some("time_map")
                && a["params"]["source_input_id"].as_str() == Some(source_input_id)
        })
        .unwrap_or_else(|| panic!("time_map artifact for {source_input_id}"));
    serde_json::from_str(
        &std::fs::read_to_string(job_dir.join(rec["path"].as_str().unwrap())).unwrap(),
    )
    .unwrap()
}

/// The TimeMap contract: segments are ordered and non-overlapping in
/// capture time (ranges are half-open, so touching endpoints are
/// fine), and source ranges never regress except across a Resume.
fn assert_segments_ordered(segments: &[serde_json::Value]) {
    let rng = |s: &serde_json::Value, key: &str| {
        (s[key][0].as_u64().unwrap(), s[key][1].as_u64().unwrap())
    };
    for w in segments.windows(2) {
        let (pc0, pc1) = rng(&w[0], "capture_range");
        let (nc0, _) = rng(&w[1], "capture_range");
        assert!(
            pc0 <= pc1 && nc0 >= pc1,
            "capture ranges unordered: {segments:?}"
        );
        let resumes =
            w[0]["kind"].as_str() == Some("resume") || w[1]["kind"].as_str() == Some("resume");
        if !resumes {
            let (_, ps1) = rng(&w[0], "source_range");
            let (ns0, _) = rng(&w[1], "source_range");
            assert!(ns0 >= ps1, "source ranges overlap: {segments:?}");
        }
    }
}

#[test]
fn locate_finds_source_embedded_in_capture() {
    let al = env!("CARGO_BIN_EXE_al");
    let dir = temp_dir("locate");
    let jobs = dir.join("jobs");

    // Source: 512 frames of noise. Capture: background noise with the
    // source embedded at frame 2000 at 0.5 gain.
    let source = noise_i16(7, 512, 20000.0);
    let mut capture = noise_i16(11, 8192, 3000.0);
    for (i, &s) in source.iter().enumerate() {
        capture[2000 + i] = capture[2000 + i].saturating_add((s as i32 / 2) as i16);
    }
    let src_wav = dir.join("src.wav");
    let cap_wav = dir.join("cap.wav");
    std::fs::write(&src_wav, wav_from_samples(&source)).unwrap();
    std::fs::write(&cap_wav, wav_from_samples(&capture)).unwrap();

    let out = Command::new(al)
        .args([
            "run",
            "--base-dir",
            jobs.to_str().unwrap(),
            "--input",
            &format!(
                "capture_bug:{},source_asset:{}",
                cap_wav.display(),
                src_wav.display()
            ),
            "--stages",
            "ingest,locate",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let job_id = stdout
        .lines()
        .find_map(|l| l.strip_prefix("job_id="))
        .expect("job_id line");

    // A locate_result artifact is registered, accepted near frame 2000.
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(jobs.join(job_id).join("manifest.json")).unwrap(),
    )
    .unwrap();
    let artifacts = manifest["artifacts"].as_array().unwrap();
    let locate = artifacts
        .iter()
        .find(|a| a["role"].as_str() == Some("locate_result"))
        .expect("locate_result artifact");
    assert_eq!(locate["params"]["accepted"].as_bool(), Some(true));
    let result_path = jobs.join(job_id).join(locate["path"].as_str().unwrap());
    let body: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(result_path).unwrap()).unwrap();
    let start = body["result"]["capture_range"]["start"].as_u64().unwrap();
    assert!(
        start.abs_diff(2000) <= 256,
        "located position {start} too far from 2000"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn locate_emits_time_map_and_residual_artifacts() {
    let al = env!("CARGO_BIN_EXE_al");
    let dir = temp_dir("locate-tracking");
    let jobs = dir.join("jobs");

    // Source A: a 300-frame loop body, embedded contiguously three
    // times starting at frame 1000 with descending gain so all methods
    // vote the first pass as the primary position. Source B: a
    // 200-frame clip embedded quietly *inside* the first pass, so the
    // dominant loop masks it until residual subtraction removes the fit.
    let body = noise_i16(17, 300, 20000.0);
    let quiet = noise_i16(23, 200, 24000.0);
    let mut capture = noise_i16(29, 4096, 3000.0);
    for (rep, gain) in [(4i32, 5i32), (3, 5), (2, 5)].iter().enumerate() {
        for (i, &s) in body.iter().enumerate() {
            let at = 1000 + rep * 300 + i;
            capture[at] = capture[at].saturating_add((s as i32 * gain.0 / gain.1) as i16);
        }
    }
    for (i, &s) in quiet.iter().enumerate() {
        capture[1050 + i] = capture[1050 + i].saturating_add((s as i32 / 4) as i16);
    }
    let loop_wav = dir.join("loop.wav");
    let quiet_wav = dir.join("quiet.wav");
    let cap_wav = dir.join("cap.wav");
    std::fs::write(&loop_wav, wav_from_samples(&body)).unwrap();
    std::fs::write(&quiet_wav, wav_from_samples(&quiet)).unwrap();
    std::fs::write(&cap_wav, wav_from_samples(&capture)).unwrap();

    let out = Command::new(al)
        .args([
            "run",
            "--base-dir",
            jobs.to_str().unwrap(),
            "--input",
            &format!(
                "capture_bug:{},source_asset:{},source_asset:{}",
                cap_wav.display(),
                loop_wav.display(),
                quiet_wav.display()
            ),
            "--stages",
            "ingest,locate",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let job_id = stdout
        .lines()
        .find_map(|l| l.strip_prefix("job_id="))
        .expect("job_id line");
    let job_dir = jobs.join(job_id);
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(job_dir.join("manifest.json")).unwrap())
            .unwrap();
    let artifacts = manifest["artifacts"].as_array().unwrap();
    let loop_id = input_id_of(artifacts, "loop.wav");
    let quiet_id = input_id_of(artifacts, "quiet.wav");

    // Every located pair registers full per-channel-pair evidence.
    assert!(
        artifacts
            .iter()
            .any(|a| a["role"].as_str() == Some("locate_evidence")),
        "locate_evidence artifact missing"
    );

    // The loop pair produces a real piecewise time map: one anchor per
    // detected pass, segments built by build_time_map.
    let time_map_rec = artifacts
        .iter()
        .find(|a| {
            a["role"].as_str() == Some("time_map")
                && a["params"]["source_input_id"].as_str() == Some(loop_id.as_str())
        })
        .expect("time_map artifact for loop source");
    let body_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(job_dir.join(time_map_rec["path"].as_str().unwrap())).unwrap(),
    )
    .unwrap();
    let occurrences = body_json["loop_occurrences"].as_array().unwrap();
    assert!(
        occurrences.len() >= 3,
        "expected >=3 loop occurrences, got {occurrences:?}"
    );
    // Detected positions are kept, not snapped to a capture-0 grid.
    let positions: Vec<u64> = occurrences
        .iter()
        .map(|o| o["occurrence_position"].as_u64().unwrap())
        .collect();
    assert!(
        positions.iter().any(|&p| p.abs_diff(1000) <= 256)
            && positions.iter().any(|&p| p.abs_diff(1600) <= 256),
        "occurrence positions {positions:?} should cover the three passes"
    );
    let anchors = body_json["anchors"].as_array().unwrap();
    assert!(anchors.len() >= 3, "time map needs one anchor per pass");
    let segments = body_json["time_map"]["segments"].as_array().unwrap();
    assert!(
        segments
            .iter()
            .any(|s| s["kind"].as_str() == Some("continuous")),
        "contiguous passes should produce a continuous segment: {segments:?}"
    );

    // Residual search: subtracting the accepted loop exposes the quiet
    // source; provenance links parent alignment and energies.
    let residual_recs: Vec<_> = artifacts
        .iter()
        .filter(|a| a["role"].as_str() == Some("residual"))
        .collect();
    assert!(!residual_recs.is_empty(), "residual artifacts missing");
    let residual = residual_recs
        .iter()
        .find(|a| {
            a["params"]["parent_source"].as_str() == Some(loop_id.as_str())
                && a["params"]["child_source"].as_str() == Some(quiet_id.as_str())
        })
        .expect("residual artifact for loop->quiet");
    let res_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(job_dir.join(residual["path"].as_str().unwrap())).unwrap(),
    )
    .unwrap();
    assert!(
        res_json["parent_alignment"].as_str().is_some()
            && !res_json["channel_transform"].as_array().unwrap().is_empty(),
        "subtraction provenance incomplete: {res_json}"
    );
    let before = res_json["energy"]["before"].as_f64().unwrap();
    let after = res_json["energy"]["after"].as_f64().unwrap();
    assert!(after < before, "subtraction must remove energy");
    // The child locate actually ran on the residual (all four methods
    // produced evidence there) and unmasked the quiet source.
    assert!(
        res_json["child"]["evidence"].as_array().unwrap().len() >= 4,
        "child locate evidence missing: {res_json}"
    );
    assert_eq!(res_json["child"]["accepted"].as_bool(), Some(true));
    let child_start = res_json["child"]["capture_range"]["start"]
        .as_u64()
        .unwrap();
    assert!(
        child_start.abs_diff(1050) <= 256,
        "quiet source located at {child_start}, expected ~1050"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn capture_only_runs_through_cheap_analysis_and_dataset() {
    let al = env!("CARGO_BIN_EXE_al");
    let dir = temp_dir("cheap");
    let wav = dir.join("cap.wav");
    std::fs::write(&wav, wav_from_samples(&noise_i16(5, 8192, 12000.0))).unwrap();
    let jobs = dir.join("jobs");

    let out = Command::new(al)
        .args([
            "run",
            "--base-dir",
            jobs.to_str().unwrap(),
            "--input",
            &format!("capture_bug:{}", wav.display()),
            "--stages",
            "ingest,cheap_analysis,dataset",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let job_id = stdout
        .lines()
        .find_map(|l| l.strip_prefix("job_id="))
        .expect("job_id line");

    // Cheap feature records land under analysis/features/cheap.
    let cheap_dir = jobs.join(job_id).join("analysis/features/cheap");
    let entries: Vec<_> = std::fs::read_dir(&cheap_dir)
        .expect("features/cheap dir")
        .collect();
    assert!(
        entries.iter().any(|e| e
            .as_ref()
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".json")),
        "no cheap feature JSON written"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn stage_plan_missing_dependency_is_rejected() {
    let al = env!("CARGO_BIN_EXE_al");
    let dir = temp_dir("dep");
    let wav = dir.join("cap.wav");
    std::fs::write(&wav, wav_bytes()).unwrap();
    let jobs = dir.join("jobs");

    // cheap_analysis requires ingest earlier in the plan.
    let out = Command::new(al)
        .args([
            "run",
            "--base-dir",
            jobs.to_str().unwrap(),
            "--input",
            &format!("capture_bug:{}", wav.display()),
            "--stages",
            "cheap_analysis",
        ])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "missing ingest dependency should fail at the entry"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn locate_tracks_long_source_with_windowed_anchors() {
    let al = env!("CARGO_BIN_EXE_al");
    let dir = temp_dir("locate-long");
    let jobs = dir.join("jobs");

    // A 4096-frame source embedded once at frame 1500: long enough to
    // split into per-window anchors, so the time map is piecewise
    // instead of a single offset.
    let source = noise_i16(31, 4096, 20000.0);
    let mut capture = noise_i16(37, 12288, 3000.0);
    for (i, &s) in source.iter().enumerate() {
        capture[1500 + i] = capture[1500 + i].saturating_add((s as i32 * 3 / 5) as i16);
    }
    let src_wav = dir.join("src.wav");
    let cap_wav = dir.join("cap.wav");
    std::fs::write(&src_wav, wav_from_samples(&source)).unwrap();
    std::fs::write(&cap_wav, wav_from_samples(&capture)).unwrap();

    let job_dir = run_ingest_locate(
        al,
        &jobs,
        &format!(
            "capture_bug:{},source_asset:{}",
            cap_wav.display(),
            src_wav.display()
        ),
        None,
    );
    let artifacts = manifest_artifacts(&job_dir);
    let src_id = input_id_of(&artifacts, "src.wav");
    let body = time_map_body(&job_dir, &artifacts, &src_id);

    let anchors = body["anchors"].as_array().unwrap();
    assert!(
        anchors.len() >= 4,
        "long source needs multiple window anchors: {anchors:?}"
    );
    let segments = body["time_map"]["segments"].as_array().unwrap();
    assert!(
        segments
            .iter()
            .all(|s| s["kind"].as_str() == Some("continuous")),
        "uniform insertion should stay continuous: {segments:?}"
    );
    assert_segments_ordered(segments);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn locate_tracks_linear_drift_as_drift_segment() {
    let al = env!("CARGO_BIN_EXE_al");
    let dir = temp_dir("locate-drift");
    let jobs = dir.join("jobs");

    // Linear drift modelled as a staircase: each 512-frame block is
    // embedded at 1.1x its source offset, so capture time advances
    // faster than source time. Within a block the offset is exact,
    // so per-window anchors stay sharp.
    let source = noise_i16(41, 4096, 20000.0);
    let mut capture = noise_i16(43, 8192, 3000.0);
    for (k, chunk) in source.chunks(512).enumerate() {
        let at = 1500 + (k as f64 * 512.0 * 1.1).round() as usize;
        for (i, &s) in chunk.iter().enumerate() {
            capture[at + i] = capture[at + i].saturating_add((s as i32 * 3 / 5) as i16);
        }
    }
    let src_wav = dir.join("src.wav");
    let cap_wav = dir.join("cap.wav");
    std::fs::write(&src_wav, wav_from_samples(&source)).unwrap();
    std::fs::write(&cap_wav, wav_from_samples(&capture)).unwrap();
    // Whole-source correlation smears over the drift, so the agreeing
    // methods land ~270 samples apart: widen the consensus position
    // tolerance for this job to let the insertion be accepted.
    let cfg = dir.join("config.json");
    std::fs::write(
        &cfg,
        r#"{"locate":{"consensus":{"position_tolerance":512}}}"#,
    )
    .unwrap();

    let job_dir = run_ingest_locate(
        al,
        &jobs,
        &format!(
            "capture_bug:{},source_asset:{}",
            cap_wav.display(),
            src_wav.display()
        ),
        Some(&cfg),
    );
    let artifacts = manifest_artifacts(&job_dir);
    let src_id = input_id_of(&artifacts, "src.wav");
    let body = time_map_body(&job_dir, &artifacts, &src_id);

    let segments = body["time_map"]["segments"].as_array().unwrap();
    let drift = segments
        .iter()
        .find(|s| s["kind"].as_str() == Some("drift"))
        .unwrap_or_else(|| panic!("drift segment missing: {segments:?}"));
    let rate = drift["rate"].as_f64().unwrap();
    assert!(
        (1.05..=1.15).contains(&rate),
        "drift rate {rate} should be ~1.1: {segments:?}"
    );
    assert_segments_ordered(segments);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn locate_tracks_skip_and_resume_piecewise() {
    let al = env!("CARGO_BIN_EXE_al");
    let dir = temp_dir("locate-skip");
    let jobs = dir.join("jobs");

    // The source is inserted in three pieces: [0,2500) contiguous at
    // 1000, [3000,4000) at 6000 (skipping source 2500..3000 behind an
    // unmapped capture gap), and the skipped span itself replayed at
    // 9500 — a rewind, i.e. a resume at an earlier source position.
    let source = noise_i16(47, 4096, 20000.0);
    let mut capture = noise_i16(53, 12288, 3000.0);
    for &(s_from, s_to, at) in &[
        (0usize, 2500usize, 1000usize),
        (3000, 4000, 6000),
        (2500, 3000, 9500),
    ] {
        for i in 0..s_to - s_from {
            capture[at + i] =
                capture[at + i].saturating_add((source[s_from + i] as i32 * 3 / 5) as i16);
        }
    }
    let src_wav = dir.join("src.wav");
    let cap_wav = dir.join("cap.wav");
    std::fs::write(&src_wav, wav_from_samples(&source)).unwrap();
    std::fs::write(&cap_wav, wav_from_samples(&capture)).unwrap();

    let job_dir = run_ingest_locate(
        al,
        &jobs,
        &format!(
            "capture_bug:{},source_asset:{}",
            cap_wav.display(),
            src_wav.display()
        ),
        None,
    );
    let artifacts = manifest_artifacts(&job_dir);
    let src_id = input_id_of(&artifacts, "src.wav");
    let body = time_map_body(&job_dir, &artifacts, &src_id);

    let segments = body["time_map"]["segments"].as_array().unwrap();
    let kinds: Vec<_> = segments.iter().map(|s| s["kind"].as_str()).collect();
    assert!(
        kinds.contains(&Some("gap")),
        "gap segment missing: {kinds:?}"
    );
    assert!(
        kinds.contains(&Some("resume")),
        "resume segment missing: {kinds:?}"
    );
    assert_segments_ordered(segments);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn missing_or_invalid_config_is_an_explicit_error() {
    let al = env!("CARGO_BIN_EXE_al");
    let dir = temp_dir("cfg");
    let wav = dir.join("cap.wav");
    std::fs::write(&wav, wav_bytes()).unwrap();
    let jobs = dir.join("jobs");

    for config in ["nonexistent.json", "broken.json"] {
        if config == "broken.json" {
            std::fs::write(dir.join(config), "{ not json").unwrap();
        }
        let out = Command::new(al)
            .args([
                "run",
                "--base-dir",
                jobs.to_str().unwrap(),
                "--input",
                &format!("capture_bug:{}", wav.display()),
                "--stages",
                "ingest",
                "--config",
                dir.join(config).to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            !out.status.success(),
            "config {config} should fail explicitly"
        );
    }

    std::fs::remove_dir_all(&dir).ok();
}
