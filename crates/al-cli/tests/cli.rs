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
    let input_id_of = |name: &str| {
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
    };
    let loop_id = input_id_of("loop.wav");
    let quiet_id = input_id_of("quiet.wav");

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
