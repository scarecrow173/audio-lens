//! ExportStage / export package integration tests.
//!
//! A small dataset is written directly through `DatasetWriter`, then
//! the real export stage runs against it: dependency contract,
//! package portability, determinism, media copy, and path-safety
//! gates are asserted. The package never contains display products
//! or a static report — those were removed with `al-viz` (#106).

use std::fs;
use std::path::{Path, PathBuf};

use al_core::{InputRole, JobId, StageKind};
use al_dataset::export::{ExportStage, package};
use al_dataset::provenance::{self, Provenance};
use al_dataset::writer::{DatasetWriter, TensorChunk};
use al_job::job::input;
use al_job::manifest::Manifest;
use al_job::registry::ArtifactRegistry;
use al_job::stage::{Stage, StageContext};

fn prov(input_id: &str) -> Provenance {
    Provenance {
        input_id: input_id.into(),
        algorithm_id: "test.v1".into(),
        algorithm_version: "1".into(),
        params: serde_json::json!({}),
        alignment_id: None,
        parents: vec![],
    }
}

struct Fixture {
    dir: PathBuf,
    manifest: Manifest,
    input_id: String,
}

/// One `capture_bug` input; the job manifest only lists the stages
/// the fixture exercises.
fn fixture() -> Fixture {
    let dir = std::env::temp_dir().join(format!("al-export-test-{}", JobId::new()));
    fs::create_dir_all(&dir).unwrap();

    let mut cap = input(InputRole::CaptureBug, dir.join("cap.wav"));
    cap.source_sha256 = Some("cc".repeat(32));
    let input_id = cap.input_id.to_string();
    let manifest = Manifest::new(
        JobId::new(),
        vec![cap],
        serde_json::json!({}),
        &[StageKind::Ingest, StageKind::Dataset, StageKind::Export],
    );
    Fixture {
        dir,
        manifest,
        input_id,
    }
}

fn ctx<'a>(
    fx: &'a Fixture,
    registry: &'a ArtifactRegistry,
    config: &'a serde_json::Value,
) -> StageContext<'a> {
    StageContext {
        manifest: &fx.manifest,
        registry,
        config,
    }
}

fn analysis_root(fx: &Fixture) -> PathBuf {
    fx.dir.join("analysis")
}

/// The input-role JSON stores a real dataset carries: `declared.json`
/// (input declaration) and `<id>.metadata.json` (ingest metadata).
/// Both record the producing host's absolute source path — the
/// projection under test. `metadata` additionally embeds
/// foreign-convention absolute paths (POSIX drive / UNC) to prove the
/// projection does not depend on the exporting host.
fn seed_input_records(fx: &Fixture, config: &serde_json::Value) {
    let mut w = open_writer(fx, config);
    let refs = provenance::input_references(&fx.manifest).unwrap();
    w.write_json(
        "inputs",
        "declared",
        &serde_json::json!({ "inputs": refs }),
        prov("job"),
    )
    .unwrap();
    let source = fx.manifest.inputs[0].path.to_string_lossy().to_string();
    w.write_json(
        "inputs",
        &format!("{}.metadata", fx.input_id),
        &serde_json::json!({
            "input_id": fx.input_id,
            "role": "capture_bug",
            "source_path": source,
            "source_hash": "cc".repeat(32),
            "container": "wav",
            "streams": [{"stream": 0, "kind": "audio"}],
            "audio": {
                "codec": "pcm_s16le", "sample_rate": 48000, "channels": 1,
                "sample_format": "pcm_s16", "frames": 8192,
                "duration_secs": 0.1707,
                "source_hash": "cc".repeat(32), "history": []
            },
            "integrity": [],
            "decoder": {"backend": "test", "version": "0"},
            "foreign": {
                "posix": "/var/tmp/foreign dir/cap.wav",
                "windows": "C:\\foreign\\cap.wav",
                "unc": "\\\\server\\share\\cap.wav",
                "unicode": "D:\\media\\カラオケ.wav",
                "relative": "rel/dir/keep.wav",
                "plain": "cap.wav"
            }
        }),
        prov(&fx.input_id),
    )
    .unwrap();
    w.finish().unwrap();
}

fn open_writer(fx: &Fixture, config: &serde_json::Value) -> DatasetWriter {
    DatasetWriter::open_or_create(
        &analysis_root(fx),
        provenance::input_references(&fx.manifest).unwrap(),
        config.clone(),
    )
    .expect("dataset writer")
}

/// A dataset with inputs audio, cheap features, and an intermediate
/// magnitude tensor — every store kind the package copy must carry.
fn seed_dataset(fx: &Fixture, config: &serde_json::Value) {
    let mut w = open_writer(fx, config);
    // 48 kHz mono, 8192 samples.
    let samples: Vec<f64> = (0..8192)
        .map(|i| 0.5 * (2.0 * std::f64::consts::PI * 1000.0 * i as f64 / 48_000.0).sin())
        .collect();
    w.write_audio(
        "inputs",
        &fx.input_id,
        al_media::wav::WavEncoding::Float32,
        1,
        48_000,
        &samples,
        prov(&fx.input_id),
    )
    .unwrap();
    w.write_json(
        "features/cheap",
        &fx.input_id,
        &serde_json::json!({"records": [
            {
                "feature_id": "level.rms_dbfs",
                "value": {"scalar": -8.0},
                "unit": "dbfs",
                "time_range": null,
                "freq_range": null,
                "channels": null,
                "resolution": null,
                "resolution_class": "segment",
                "target": -8.0,
                "reference": null,
                "delta": null,
                "confidence": "high",
                "validity": "valid",
                "invalid_reason": null,
                "support_count": 8192,
                "algorithm_id": "cheap.v1",
                "algorithm_version": "1",
                "params": {},
                "input_id": fx.input_id,
                "alignment_id": null,
                "lineage": []
            }
        ]}),
        prov(&fx.input_id),
    )
    .unwrap();
    // [4 frames, 8 bins] magnitude spectrogram.
    let mag: Vec<f64> = (0..32).map(|i| 0.01 * (i + 1) as f64).collect();
    w.write_tensor(
        "intermediate",
        &format!("{}/ch0/magnitude", fx.input_id),
        &[4, 8],
        &[4, 8],
        &[TensorChunk {
            coords: vec![0, 0],
            data: mag,
        }],
        prov(&fx.input_id),
    )
    .unwrap();
    w.finish().unwrap();
}

/// Sorted (path, sha256) pairs for every file under `root`.
fn snapshot(root: &Path) -> Vec<(String, String)> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let p = entry.path();
            if p.is_dir() {
                walk(&p, root, out);
            } else {
                let rel = p
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push((rel, al_core::sha256_hex(&fs::read(&p).unwrap())));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

#[test]
fn export_stage_depends_only_on_dataset() {
    // #103/#106: the export package reads the materialized analysis
    // dataset only — the pipeline-order default would demand the
    // analysis stages, which an ingest+dataset job legitimately
    // skips.
    assert_eq!(ExportStage::new().depends_on(), vec![StageKind::Dataset]);
}

#[test]
fn export_package_is_portable_deterministic_and_manifested() {
    let fx = fixture();
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({});
    seed_dataset(&fx, &config);

    let recs = ExportStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("export stage");
    assert!(
        recs.iter().any(|r| r.role == "export_manifest")
            && recs.iter().any(|r| r.role == "export_golden"),
        "artifacts: {recs:?}"
    );

    let pkg = fx.dir.join("export").join("analysis");
    // #106: no display products, no static report.
    assert!(!pkg.join("analysis/visualization").exists());
    assert!(!pkg.join("report.html").exists());
    assert!(!pkg.join("report.data.js").exists());

    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(pkg.join("export_manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["schema_version"], 1);
    // Inputs are file-name + hash references, never absolute paths.
    let src = manifest["inputs"][0]["file_name"].as_str().unwrap();
    assert_eq!(src, "cap.wav");
    assert!(manifest["inputs"][0]["sha256"].is_string());
    // No absolute path survives in any text file (the stage also
    // enforces this internally; assert the observable result).
    let abs = fx.dir.to_string_lossy().replace('\\', "/");
    for (rel, _) in snapshot(&pkg) {
        if rel.ends_with(".json") {
            let body = fs::read_to_string(pkg.join(&rel)).unwrap();
            assert!(!body.contains(&abs), "{rel} embeds an absolute path");
        }
    }

    // Golden snapshot exists and a rebuild reproduces it byte-for-byte.
    let golden1 = fs::read_to_string(pkg.join("golden/snapshot.json")).unwrap();
    ExportStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("export rerun");
    let golden2 = fs::read_to_string(pkg.join("golden/snapshot.json")).unwrap();
    assert_eq!(golden1, golden2, "export is not deterministic");

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn export_copy_media_separates_same_named_inputs() {
    // Two inputs that share a basename but differ in content must
    // not collapse into a single media/ file.
    let dir = std::env::temp_dir().join(format!("al-export-test-{}", JobId::new()));
    fs::create_dir_all(dir.join("a")).unwrap();
    fs::create_dir_all(dir.join("b")).unwrap();
    fs::write(dir.join("a/cap.wav"), b"media-A").unwrap();
    fs::write(dir.join("b/cap.wav"), b"media-B-differs").unwrap();

    let mut cap_a = input(InputRole::CaptureBug, dir.join("a/cap.wav"));
    cap_a.source_sha256 = Some(al_core::sha256_hex(b"media-A"));
    let mut cap_b = input(InputRole::CaptureReference, dir.join("b/cap.wav"));
    cap_b.source_sha256 = Some(al_core::sha256_hex(b"media-B-differs"));
    let id_a = cap_a.input_id.to_string();
    let id_b = cap_b.input_id.to_string();
    let manifest = Manifest::new(
        JobId::new(),
        vec![cap_a, cap_b],
        serde_json::json!({}),
        &[StageKind::Dataset, StageKind::Export],
    );

    DatasetWriter::open_or_create(
        &dir.join("analysis"),
        provenance::input_references(&manifest).unwrap(),
        serde_json::json!({}),
    )
    .unwrap()
    .finish()
    .unwrap();

    let cfg = package::ExportConfig {
        copy_media: true,
        name: "analysis".into(),
    };
    let (pkg, files) = package::build(&dir, &manifest, &cfg).expect("export build");

    // Each input is copied under its own media/<input_id>/ dir.
    let rel_a = format!("media/{id_a}/cap.wav");
    let rel_b = format!("media/{id_b}/cap.wav");
    assert_eq!(fs::read(pkg.join(&rel_a)).unwrap(), b"media-A");
    assert_eq!(fs::read(pkg.join(&rel_b)).unwrap(), b"media-B-differs");
    assert!(files.contains(&rel_a) && files.contains(&rel_b));

    // The export manifest records each input's real package path,
    // so every recorded sha256 matches the file it points at.
    let export_manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(pkg.join("export_manifest.json")).unwrap()).unwrap();
    let inputs = export_manifest["inputs"].as_array().unwrap();
    for (id, rel) in [(id_a, rel_a), (id_b, rel_b)] {
        let rec = inputs
            .iter()
            .find(|i| i["input_id"].as_str() == Some(id.as_str()))
            .unwrap_or_else(|| panic!("missing export record for {id}"));
        assert_eq!(rec["media_path"].as_str(), Some(rel.as_str()));
        let copied = al_core::sha256_hex(&fs::read(pkg.join(&rel)).unwrap());
        assert_eq!(rec["sha256"].as_str().unwrap(), copied.as_str());
    }

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn export_name_rejects_unsafe_directory_names() {
    let fx = fixture();
    let config = serde_json::json!({});
    seed_dataset(&fx, &config);

    // Sentinel under the job dir: `../keep` would reach it through
    // the unvalidated join + remove_dir_all.
    let sentinel = fx.dir.join("keep").join("marker.txt");
    fs::create_dir_all(sentinel.parent().unwrap()).unwrap();
    fs::write(&sentinel, b"keep").unwrap();

    let mut names = vec!["../keep", "..", ".", "a/b", ""];
    #[cfg(windows)]
    names.extend(["a\\b", "C:/keep", "C:\\keep", "\\\\server\\share"]);
    #[cfg(not(windows))]
    names.push("/keep");
    for name in names {
        let cfg = package::ExportConfig {
            copy_media: false,
            name: name.into(),
        };
        let err = package::build(&fx.dir, &fx.manifest, &cfg)
            .expect_err(&format!("export name {name:?} must be rejected"));
        assert_eq!(
            err.kind(),
            std::io::ErrorKind::InvalidData,
            "export name {name:?} produced wrong error kind"
        );
    }
    assert!(sentinel.exists(), "traversal deleted the sentinel file");

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn export_shrinks_absolute_paths_inside_config() {
    let fx = fixture();
    let registry = ArtifactRegistry::new(&fx.dir);
    let dict = fx.dir.join("cfg").join("dict.bin");
    let config = serde_json::json!({
        "analysis": {"dict": dict.to_string_lossy()},
        "extra": {"refs": ["plain", dict.to_string_lossy()]},
    });
    seed_dataset(&fx, &config);
    ExportStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("export stage");

    let pkg_manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(fx.dir.join("export/analysis/analysis/manifest.json")).unwrap(),
    )
    .unwrap();
    // Absolute config paths travel as file-name references; the
    // original host path must not survive anywhere in config.
    assert_eq!(pkg_manifest["config"]["analysis"]["dict"], "dict.bin");
    assert_eq!(pkg_manifest["config"]["extra"]["refs"][1], "dict.bin");
    assert_eq!(pkg_manifest["config"]["extra"]["refs"][0], "plain");
    let body = fs::read_to_string(fx.dir.join("export/analysis/analysis/manifest.json")).unwrap();
    let dir_fwd = fx.dir.to_string_lossy().replace('\\', "/");
    let dir_back = fx.dir.to_string_lossy().replace('/', "\\");
    assert!(!body.contains(&dir_fwd) && !body.contains(&dir_back));

    fs::remove_dir_all(&fx.dir).ok();
}

/// Every separator / escape spelling a private path can take inside
/// package text: `/`-normalized, raw `\`, and the JSON-escaped
/// doubled-`\` form a serialized string literal produces.
fn path_spellings(path: &Path) -> [String; 3] {
    let fwd = path.to_string_lossy().replace('\\', "/");
    let back = fwd.replace('/', "\\");
    let esc = back.replace('\\', "\\\\");
    [fwd, back, esc]
}

fn assert_no_private_path(pkg: &Path, private: &[String]) {
    for (rel, _) in snapshot(pkg) {
        if rel.ends_with(".json") || rel.ends_with(".txt") {
            let body = fs::read_to_string(pkg.join(&rel)).unwrap();
            for pat in private {
                assert!(
                    !body.contains(pat.as_str()),
                    "{rel} still embeds private path spelling {pat:?}"
                );
            }
        }
    }
}

#[test]
fn export_sanitizes_dataset_json_artifacts() {
    let fx = fixture();
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({});
    seed_dataset(&fx, &config);
    seed_input_records(&fx, &config);

    ExportStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("export stage");

    let pkg = fx.dir.join("export/analysis");

    // metadata projection: host path becomes the media file name;
    // identity, digests, and stream/timing facts are preserved.
    let meta: serde_json::Value = serde_json::from_slice(
        &fs::read(pkg.join(format!("analysis/inputs/{}.metadata.json", fx.input_id))).unwrap(),
    )
    .unwrap();
    assert_eq!(meta["source_path"], "cap.wav");
    assert_eq!(meta["input_id"].as_str().unwrap(), fx.input_id);
    assert_eq!(meta["role"], "capture_bug");
    assert_eq!(meta["source_hash"], "cc".repeat(32));
    assert_eq!(meta["container"], "wav");
    assert_eq!(meta["audio"]["codec"], "pcm_s16le");
    assert_eq!(meta["audio"]["sample_rate"], 48000);
    assert_eq!(meta["audio"]["source_hash"], "cc".repeat(32));
    // Foreign-convention absolute paths shrink on every host; a
    // relative or plain-name string is not a path and stays intact.
    assert_eq!(meta["foreign"]["posix"], "cap.wav");
    assert_eq!(meta["foreign"]["windows"], "cap.wav");
    assert_eq!(meta["foreign"]["unc"], "cap.wav");
    assert_eq!(meta["foreign"]["unicode"], "カラオケ.wav");
    assert_eq!(meta["foreign"]["relative"], "rel/dir/keep.wav");
    assert_eq!(meta["foreign"]["plain"], "cap.wav");

    // The input declaration keeps id/role/digest and loses the host
    // path the same way.
    let declared: serde_json::Value =
        serde_json::from_slice(&fs::read(pkg.join("analysis/inputs/declared.json")).unwrap())
            .unwrap();
    assert_eq!(declared["inputs"][0]["source_path"], "cap.wav");
    assert_eq!(
        declared["inputs"][0]["input_id"].as_str().unwrap(),
        fx.input_id
    );
    assert_eq!(declared["inputs"][0]["source_sha256"], "cc".repeat(32));
    assert_eq!(declared["inputs"][0]["role"], "capture_bug");

    // The dataset manifest projection is unchanged: inputs and config
    // still travel as file-name references.
    let ds_manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(pkg.join("analysis/manifest.json")).unwrap()).unwrap();
    assert_eq!(ds_manifest["inputs"][0]["source_path"], "cap.wav");

    // No separator or escape spelling of the private paths survives
    // in any package text file.
    let mut private: Vec<String> = path_spellings(&fx.manifest.inputs[0].path)
        .into_iter()
        .collect();
    private.extend(path_spellings(&fx.dir));
    assert_no_private_path(&pkg, &private);

    // The source dataset keeps its original provenance — the
    // projection applies to the export copy only.
    let src_meta = fs::read_to_string(
        analysis_root(&fx).join(format!("inputs/{}.metadata.json", fx.input_id)),
    )
    .unwrap();
    let [fwd, _back, esc] = path_spellings(&fx.manifest.inputs[0].path);
    assert!(
        src_meta.contains(&fwd) || src_meta.contains(&esc),
        "source metadata lost its original path"
    );

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn export_fails_closed_when_a_text_artifact_cannot_be_sanitized() {
    let fx = fixture();
    let config = serde_json::json!({});
    seed_dataset(&fx, &config);
    // Non-JSON text cannot be projected structurally. One that still
    // names a private path must stop the publish — in whichever
    // separator spelling it was written.
    let leak = fx.manifest.inputs[0].path.to_string_lossy().to_string();
    fs::write(
        analysis_root(&fx).join("notes.txt"),
        format!("source: {leak}"),
    )
    .unwrap();

    let cfg = package::ExportConfig {
        copy_media: false,
        name: "analysis".into(),
    };
    let err = package::build(&fx.dir, &fx.manifest, &cfg)
        .expect_err("a leaking text artifact must fail the export");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);

    // No finished package, no partial staging leftovers, and the
    // source dataset keeps its original bytes.
    assert!(!fx.dir.join("export/analysis").exists());
    assert!(!fx.dir.join("export/.analysis.partial").exists());
    assert!(analysis_root(&fx).join("notes.txt").exists());

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn export_rerun_failure_keeps_the_last_verified_package() {
    let fx = fixture();
    let config = serde_json::json!({});
    seed_dataset(&fx, &config);
    let cfg = package::ExportConfig {
        copy_media: false,
        name: "analysis".into(),
    };
    package::build(&fx.dir, &fx.manifest, &cfg).expect("first export");
    let manifest_path = fx.dir.join("export/analysis/export_manifest.json");
    assert!(manifest_path.exists());
    let published = fs::read(&manifest_path).unwrap();

    // A rerun that fails mid-build must not replace or corrupt the
    // previously published package.
    fs::write(
        analysis_root(&fx).join("notes.txt"),
        format!("src {}", fx.manifest.inputs[0].path.display()),
    )
    .unwrap();
    package::build(&fx.dir, &fx.manifest, &cfg).expect_err("rerun must fail");
    assert_eq!(fs::read(&manifest_path).unwrap(), published);
    assert!(!fx.dir.join("export/.analysis.partial").exists());

    fs::remove_dir_all(&fx.dir).ok();
}
