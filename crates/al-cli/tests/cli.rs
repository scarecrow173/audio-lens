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
