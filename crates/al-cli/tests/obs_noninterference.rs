//! AL-OBS-NONINTERFERENCE oracle: logging OFF/ON/degraded file sink states
//! must not alter f64 analysis values, JSON CLI output, or export artifacts,
//! and logging sinks must not disclose payload-shaped PCM or sentinel data.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Instant;

use al_testkit::media::wav_s16_bytes;
use al_testkit::signal;
use al_testkit::temp::TempDir;
use serde_json::Value;

const SAMPLE_RATE: u32 = 48_000;
const FRAMES: usize = 480_000;
const INPUT_ID: &str = "IN-0000000000000000000001";
const SENTINEL: &[u8; 16] = b"AL127_OBS_SECRET";
const SENTINEL_OFFSET: usize = 4096;
const FIXTURE_SHA256: &str = "dcad8817615ce1b56c67247e9dbc72fca36e8ba4f2afddaf17a1b5b3dab3eefb";
const SENTINEL_HEX: &str = "414c3132375f4f42535f534543524554";
const SENTINEL_BASE64: &str = "QUwxMjdfT0JTX1NFQ1JFVA==";
const FIRST_PCM_HEX: &str = "77bc3ac4ebb942d93ec0d83df74c14e0";

fn al_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_al"));
    command.env_remove("AL_LOG");
    command.env_remove("AL_LOG_FORMAT");
    command
}

fn sentinel_samples() -> Vec<i16> {
    SENTINEL
        .as_chunks::<2>()
        .0
        .iter()
        .map(|bytes| i16::from_le_bytes(*bytes))
        .collect()
}

fn fixture() -> (Vec<u8>, Vec<i16>) {
    let noise = signal::noise(FRAMES, 0xA127, 0.62 * 32767.0);
    let mut samples: Vec<i16> = noise
        .iter()
        .map(|v| v.round().clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16)
        .collect();
    for (offset, sample) in sentinel_samples().into_iter().enumerate() {
        samples[SENTINEL_OFFSET + offset] = sample;
    }
    (wav_s16_bytes(1, SAMPLE_RATE, &samples), samples)
}

fn run_output(args: &[String]) -> Output {
    al_command().args(args).output().unwrap()
}

fn job_id_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("job_id="))
        .expect("job_id line")
        .to_owned()
}

fn log_files(jobs: &Path) -> Vec<PathBuf> {
    match std::fs::read_dir(jobs.join("logs")) {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("al.") && name.ends_with(".log"))
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn looks_absolute(s: &str) -> bool {
    if s.starts_with('/') || s.starts_with('\\') || Path::new(s).is_absolute() {
        return true;
    }
    let bytes = s.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\')
}

#[derive(Default)]
struct IdCanonicalizer {
    replacements: HashMap<String, String>,
    counts: HashMap<&'static str, usize>,
}

impl IdCanonicalizer {
    fn replacement_for(&mut self, prefix: &'static str, token: &str) -> String {
        if let Some(existing) = self.replacements.get(token) {
            return existing.clone();
        }
        let next = self.counts.entry(prefix).or_insert(0);
        *next += 1;
        let replacement = format!("<{prefix}_{next}>");
        self.replacements
            .insert(token.to_owned(), replacement.clone());
        replacement
    }

    fn replace_ids(&mut self, text: &str) -> String {
        let bytes = text.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut pos = 0;
        while pos < bytes.len() {
            let mut matched = false;
            for prefix in ["JOB", "ART", "ALN"] {
                let marker = format!("{prefix}-");
                let marker_bytes = marker.as_bytes();
                if bytes[pos..].starts_with(marker_bytes) {
                    let end = pos + marker_bytes.len() + 22;
                    if end <= bytes.len()
                        && bytes[pos + marker_bytes.len()..end]
                            .iter()
                            .all(|b| b.is_ascii_alphanumeric())
                    {
                        let token = std::str::from_utf8(&bytes[pos..end]).unwrap();
                        let replacement = self.replacement_for(prefix, token);
                        out.extend_from_slice(replacement.as_bytes());
                        pos = end;
                        matched = true;
                        break;
                    }
                }
            }
            if !matched {
                out.push(bytes[pos]);
                pos += 1;
            }
        }
        String::from_utf8(out).unwrap()
    }
}

fn canonicalize_value(value: &Value, key: Option<&str>, ids: &mut IdCanonicalizer) -> Value {
    match value {
        Value::Number(_) if key == Some("duration_ms") => Value::String("<DURATION_MS>".to_owned()),
        Value::String(text) => {
            if looks_absolute(text) {
                Value::String("<ABS_PATH>".to_owned())
            } else {
                Value::String(ids.replace_ids(text))
            }
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| canonicalize_value(item, None, ids))
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, item)| (k.clone(), canonicalize_value(item, Some(k), ids)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn write_canonical(value: &Value, out: &mut Vec<u8>) {
    match value {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            out.extend_from_slice(&serde_json::to_vec(value).unwrap());
        }
        Value::Array(items) => {
            out.push(b'[');
            for (index, item) in items.iter().enumerate() {
                if index != 0 {
                    out.push(b',');
                }
                write_canonical(item, out);
            }
            out.push(b']');
        }
        Value::Object(map) => {
            out.push(b'{');
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            for (index, key) in keys.iter().enumerate() {
                if index != 0 {
                    out.push(b',');
                }
                out.extend_from_slice(&serde_json::to_vec(key).unwrap());
                out.push(b':');
                write_canonical(&map[*key], out);
            }
            out.push(b'}');
        }
    }
}

fn canonical_json_slice(bytes: &[u8]) -> Vec<u8> {
    let value: Value = serde_json::from_slice(bytes).expect("JSON input");
    let mut ids = IdCanonicalizer::default();
    let canonical = canonicalize_value(&value, None, &mut ids);
    let mut out = Vec::new();
    write_canonical(&canonical, &mut out);
    out
}

fn canonical_json_file(path: &Path, package_root: &Path) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    let mut value: Value = serde_json::from_slice(&bytes).expect("artifact JSON");
    let rel = path
        .strip_prefix(package_root)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    if rel == "golden/snapshot.json"
        && let Value::Object(map) = &mut value
    {
        let keys: Vec<String> = map.keys().cloned().collect();
        for key in keys {
            if key.ends_with(".json") {
                let target = package_root.join(&key);
                let digest = al_media::sha256_hex(&canonical_json_file(&target, package_root));
                if let Some(slot) = map.get_mut(&key) {
                    *slot = Value::String(digest);
                }
            }
        }
    }
    let mut ids = IdCanonicalizer::default();
    let canonical = canonicalize_value(&value, None, &mut ids);
    let mut out = Vec::new();
    write_canonical(&canonical, &mut out);
    out
}

fn collect_files(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_files(root, &path, files);
        } else {
            files.push(path.strip_prefix(root).unwrap().to_path_buf());
        }
    }
}

fn export_projection(job_dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let root = job_dir.join("export/analysis");
    let mut files = Vec::new();
    collect_files(&root, &root, &mut files);
    files.sort();
    let mut projection = BTreeMap::new();
    for rel in files {
        let path = root.join(&rel);
        let key = rel.to_string_lossy().replace('\\', "/");
        let bytes = if rel.extension().and_then(|ext| ext.to_str()) == Some("json") {
            canonical_json_file(&path, &root)
        } else {
            std::fs::read(&path).unwrap()
        };
        projection.insert(key, bytes);
    }
    projection
}

fn assert_projection_eq(
    label: &str,
    expected: &BTreeMap<String, Vec<u8>>,
    actual: &BTreeMap<String, Vec<u8>>,
) {
    let expected_keys: Vec<_> = expected.keys().collect();
    let actual_keys: Vec<_> = actual.keys().collect();
    assert_eq!(
        expected_keys, actual_keys,
        "{label}: export artifact inventory differs"
    );
    for (path, expected_bytes) in expected {
        let actual_bytes = &actual[path];
        if expected_bytes != actual_bytes {
            panic!(
                "{label}: export artifact {path} differs: expected len={} sha256={} actual len={} sha256={}",
                expected_bytes.len(),
                al_media::sha256_hex(expected_bytes),
                actual_bytes.len(),
                al_media::sha256_hex(actual_bytes)
            );
        }
    }
}

fn collect_f64_values(value: &Value, path: &str, out: &mut Vec<(String, u64)>) {
    match value {
        Value::Number(number) if number.is_f64() => {
            out.push((path.to_owned(), number.as_f64().unwrap().to_bits()));
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_f64_values(item, &format!("{path}/{index}"), out);
            }
        }
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            for key in keys {
                collect_f64_values(&map[key], &format!("{path}/{key}"), out);
            }
        }
        _ => {}
    }
}

fn cheap_feature_f64_values(job_dir: &Path) -> Vec<(String, u64)> {
    let path = job_dir.join(format!(
        "export/analysis/analysis/features/cheap/{INPUT_ID}.json"
    ));
    let value: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut out = Vec::new();
    collect_f64_values(&value, "", &mut out);
    out.sort();
    out
}

fn assert_no_privacy_leak(label: &str, text: &str, first_samples: &[i16]) {
    let sentinel_debug = format!("{:?}", sentinel_samples());
    let first_debug = format!("{:?}", &first_samples[..8]);
    let sentinel_joined = sentinel_samples()
        .iter()
        .map(i16::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let first_joined = first_samples[..8]
        .iter()
        .map(i16::to_string)
        .collect::<Vec<_>>()
        .join(",");
    for pattern in [
        "AL127_OBS_SECRET",
        SENTINEL_HEX,
        SENTINEL_BASE64,
        FIRST_PCM_HEX,
        sentinel_debug.as_str(),
        first_debug.as_str(),
        sentinel_joined.as_str(),
        first_joined.as_str(),
    ] {
        assert!(
            !text.contains(pattern),
            "{label}: privacy payload pattern leaked: {pattern:?}"
        );
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LoggingState {
    Off,
    On,
    FileSinkFault,
}

struct StateEvidence {
    _temp: TempDir,
    inspect_canonical: Vec<u8>,
    export: BTreeMap<String, Vec<u8>>,
    feature_values: Vec<(String, u64)>,
    wall_ms: u128,
}

fn run_state(state: LoggingState, wav: &[u8], first_samples: &[i16]) -> StateEvidence {
    let tag = match state {
        LoggingState::Off => "off",
        LoggingState::On => "on",
        LoggingState::FileSinkFault => "fault",
    };
    let temp = TempDir::new(&format!("al-cli-obs-noninterference-{tag}"));
    let wav_path = temp.join("cap.wav");
    std::fs::write(&wav_path, wav).unwrap();
    let jobs = temp.join("jobs");
    if state == LoggingState::FileSinkFault {
        std::fs::create_dir_all(&jobs).unwrap();
        std::fs::write(jobs.join("logs"), b"not a directory").unwrap();
    }

    let mut args = vec![
        "run".to_owned(),
        "--base-dir".to_owned(),
        jobs.to_str().unwrap().to_owned(),
        "--input".to_owned(),
        format!("capture_bug:{}", wav_path.display()),
        "--input-ids".to_owned(),
        INPUT_ID.to_owned(),
        "--stages".to_owned(),
        "ingest,cheap_analysis,dataset,export".to_owned(),
    ];
    if state != LoggingState::Off {
        args.extend(["--log".to_owned(), "info".to_owned()]);
    }
    let started = Instant::now();
    let run = run_output(&args);
    let wall_ms = started.elapsed().as_millis();
    assert!(
        run.status.success(),
        "{tag}: run failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let job_id = job_id_of(&run);
    let job_dir = jobs.join(&job_id);
    let run_stderr = String::from_utf8_lossy(&run.stderr).into_owned();

    match state {
        LoggingState::Off => {
            assert!(
                !jobs.join("logs").exists(),
                "default OFF must not create <base_dir>/logs"
            );
        }
        LoggingState::On => {
            assert!(
                run_stderr.contains("stage completed"),
                "ON stderr must carry stage events: {run_stderr}"
            );
            let files = log_files(&jobs);
            assert_eq!(files.len(), 1, "ON must create one rotated log file");
            let body = std::fs::read_to_string(&files[0]).unwrap();
            assert!(
                body.contains("\"stage\":\"ingest\""),
                "ON file sink must carry stage JSON events"
            );
            assert_no_privacy_leak("on file log", &body, first_samples);
        }
        LoggingState::FileSinkFault => {
            assert!(
                run_stderr.contains("warn: cannot initialize file log sink"),
                "fault state must degrade explicitly: {run_stderr}"
            );
            assert!(
                run_stderr.contains("stage completed"),
                "fault state must keep the stderr sink active: {run_stderr}"
            );
            assert!(jobs.join("logs").is_file(), "fault sentinel file remains");
        }
    }
    assert_no_privacy_leak("run stderr", &run_stderr, first_samples);

    let mut inspect_args = vec![
        "inspect".to_owned(),
        "--base-dir".to_owned(),
        jobs.to_str().unwrap().to_owned(),
        "--job-id".to_owned(),
        job_id,
    ];
    if state != LoggingState::Off {
        inspect_args.extend(["--log".to_owned(), "info".to_owned()]);
    }
    let inspect = run_output(&inspect_args);
    assert!(
        inspect.status.success(),
        "{tag}: inspect failed: {}",
        String::from_utf8_lossy(&inspect.stderr)
    );
    serde_json::from_slice::<Value>(&inspect.stdout)
        .expect("inspect stdout must remain valid JSON");
    let inspect_stderr = String::from_utf8_lossy(&inspect.stderr);
    assert_no_privacy_leak("inspect stderr", &inspect_stderr, first_samples);

    let inspect_canonical = canonical_json_slice(&inspect.stdout);
    let export = export_projection(&job_dir);
    let feature_values = cheap_feature_f64_values(&job_dir);
    assert!(
        feature_values.len() >= 20,
        "{tag}: expected a non-trivial f64 feature set, got {}",
        feature_values.len()
    );

    StateEvidence {
        _temp: temp,
        inspect_canonical,
        export,
        feature_values,
        wall_ms,
    }
}

#[test]
fn logging_state_does_not_interfere_with_analysis_cli_or_export() {
    let (wav, samples) = fixture();
    assert_eq!(al_media::sha256_hex(&wav), FIXTURE_SHA256);

    let off = run_state(LoggingState::Off, &wav, &samples);
    let on = run_state(LoggingState::On, &wav, &samples);
    let fault = run_state(LoggingState::FileSinkFault, &wav, &samples);

    assert_eq!(
        off.inspect_canonical, on.inspect_canonical,
        "inspect canonical JSON differs: OFF vs ON"
    );
    assert_eq!(
        off.inspect_canonical, fault.inspect_canonical,
        "inspect canonical JSON differs: OFF vs file-sink fault"
    );
    assert_eq!(
        off.feature_values, on.feature_values,
        "f64 feature values differ: OFF vs ON"
    );
    assert_eq!(
        off.feature_values, fault.feature_values,
        "f64 feature values differ: OFF vs file-sink fault"
    );
    assert_projection_eq("OFF vs ON", &off.export, &on.export);
    assert_projection_eq("OFF vs file-sink fault", &off.export, &fault.export);

    eprintln!(
        "obs_noninterference wall_ms: off={} on={} fault={} (observation only)",
        off.wall_ms, on.wall_ms, fault.wall_ms
    );
}

#[test]
fn canonical_projection_preserves_semantic_difference() {
    let mut ids = IdCanonicalizer::default();
    let a = canonicalize_value(
        &serde_json::json!({"duration_ms": 10, "value": 1.5, "artifact": "ART-0000000000000000000000"}),
        None,
        &mut ids,
    );
    assert_eq!(a["duration_ms"], "<DURATION_MS>");
    assert_eq!(a["value"], 1.5);
    assert_ne!(a["artifact"], "ART-0000000000000000000000");
}
