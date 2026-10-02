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
