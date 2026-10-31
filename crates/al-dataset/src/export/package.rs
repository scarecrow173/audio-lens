//! Relocatable export package: copy `analysis/` in full, shrink
//! absolute source paths down to file-name + hash references, and
//! emit a package manifest. The package must read on its own —
//! numeric/tensor data travels with it; original media only travels
//! when `config.export.copy_media` is set.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use al_job::manifest::Manifest;

/// Package manifest schema version (independent: regenerated, not
/// migrated).
pub const PACKAGE_SCHEMA_VERSION: u32 = 1;
/// Generator identity recorded in the package manifest.
pub const GENERATOR_ID: &str = "al-dataset";

/// Export configuration (`config.export.*`).
#[derive(Debug, Clone)]
pub struct ExportConfig {
    /// Copy original media files into `media/` (default: reference
    /// only).
    pub copy_media: bool,
    /// Package directory name under `export/`.
    pub name: String,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            copy_media: false,
            name: "analysis".into(),
        }
    }
}

impl ExportConfig {
    /// Overlay job config.
    pub fn from_job(config: &serde_json::Value) -> Self {
        let mut cfg = Self::default();
        let e = &config["export"];
        if let Some(v) = e["copy_media"].as_bool() {
            cfg.copy_media = v;
        }
        if let Some(v) = e["name"].as_str() {
            cfg.name = v.to_string();
        }
        cfg
    }
}

fn invalid(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.into())
}

/// `config.export.name` names the directory that gets deleted and
/// rebuilt, so it must be a single safe path component. Anything else
/// (`..`, separators, absolute or prefixed paths) would escape
/// `<job>/export/` and remove or write outside the job.
fn validate_package_name(name: &str) -> io::Result<()> {
    let mut components = Path::new(name).components();
    let single_normal =
        matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none();
    if single_normal {
        Ok(())
    } else {
        Err(invalid(format!(
            "export.name {name:?} must be a single directory name"
        )))
    }
}

/// True when `s` looks like an absolute path on any supported host.
/// `Path::is_absolute` only knows the host convention, so Windows
/// drive / UNC forms and POSIX roots are checked explicitly — a
/// package must not embed host-specific locations whichever OS
/// produced them.
fn looks_absolute(s: &str) -> bool {
    if s.starts_with('/') || s.starts_with('\\') || Path::new(s).is_absolute() {
        return true;
    }
    let b = s.as_bytes();
    b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && matches!(b[2], b'/' | b'\\')
}

/// Last path segment of `s` under either separator convention.
fn base_name(s: &str) -> &str {
    s.rsplit(['/', '\\']).find(|p| !p.is_empty()).unwrap_or(s)
}

/// Package-relative location of one input's copied media:
/// `media/<input_id>/<file_name>`. The input id scopes each file so
/// inputs that share a basename can never overwrite one another.
fn media_rel_path(input_id: &str, file_name: &str) -> String {
    format!("media/{input_id}/{file_name}")
}

/// Shrink absolute-path strings anywhere inside `v` down to
/// file-name references, so the copied job config embeds no
/// host-specific locations.
fn shrink_abs_paths(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::String(s) if looks_absolute(s) => {
            *s = base_name(s).to_string();
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(shrink_abs_paths),
        serde_json::Value::Object(map) => map.values_mut().for_each(shrink_abs_paths),
        _ => {}
    }
}

/// Copy `src` recursively into `dst` (which must not yet exist).
fn copy_tree(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Build the export package at `<job>/export/<name>/` and return the
/// list of package-relative files written. The package directory is
/// rebuilt from scratch (replacement semantics — partial leftovers
/// never survive a rerun).
pub fn build(
    job_dir: &Path,
    manifest: &Manifest,
    cfg: &ExportConfig,
) -> io::Result<(PathBuf, Vec<String>)> {
    let span = tracing::info_span!("export::package", len = tracing::field::Empty);
    let _enter = span.enter();
    validate_package_name(&cfg.name)?;
    let analysis = job_dir.join("analysis");
    if !analysis.join("manifest.json").exists() {
        return Err(invalid(
            "analysis dataset missing — run the dataset stage first",
        ));
    }
    let dest = job_dir.join("export").join(&cfg.name);
    if dest.exists() {
        fs::remove_dir_all(&dest)?;
    }
    fs::create_dir_all(&dest)?;

    // 1. Copy the dataset wholesale.
    copy_tree(&analysis, &dest.join("analysis"))?;

    // 2. Shrink absolute paths in the copied dataset manifest:
    //    declared input sources and every string inside the copied
    //    job config (dictionary / config file references travel as
    //    file names, never host paths).
    let manifest_path = dest.join("analysis/manifest.json");
    let mut ds_manifest: serde_json::Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    let mut input_records = Vec::new();
    if let Some(inputs) = ds_manifest["inputs"].as_array_mut() {
        for input in inputs.iter_mut() {
            let source_path = input["source_path"].as_str().unwrap_or("").to_string();
            let file_name = Path::new(&source_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| source_path.clone());
            input["source_path"] = serde_json::json!(file_name);
            let media_path = cfg.copy_media.then(|| {
                media_rel_path(input["input_id"].as_str().unwrap_or_default(), &file_name)
            });
            input_records.push(serde_json::json!({
                "input_id": input["input_id"],
                "role": input["role"],
                "file_name": file_name,
                "media_path": media_path,
                "sha256": input["source_sha256"],
            }));
        }
    }
    shrink_abs_paths(&mut ds_manifest["config"]);
    fs::write(&manifest_path, serde_json::to_string_pretty(&ds_manifest)?)?;

    // 3. Optional media copy: each input lands under its own
    //    `media/<input_id>/` directory — the same package-relative
    //    path recorded in the export manifest — so two inputs with
    //    the same file name keep distinct copies and each recorded
    //    sha256 matches the file it points at.
    if cfg.copy_media {
        for input in &manifest.inputs {
            let file_name = input
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .ok_or_else(|| invalid(format!("input {} has no file name", input.input_id)))?;
            let target = dest.join(media_rel_path(&input.input_id.to_string(), &file_name));
            fs::create_dir_all(target.parent().unwrap())?;
            fs::copy(&input.path, &target).map_err(|e| {
                invalid(format!(
                    "copy_media requested but {} is unreadable: {e}",
                    input.path.display()
                ))
            })?;
        }
    }

    // 4. Package manifest (deterministic: no timestamps, relative
    //    paths only).
    let roles: Vec<String> = ds_manifest["roles"]
        .as_object()
        .map(|m| {
            let mut v: Vec<String> = m
                .iter()
                .filter(|(_, e)| e["status"].as_str() == Some("produced"))
                .map(|(k, _)| k.clone())
                .collect();
            v.sort();
            v
        })
        .unwrap_or_default();
    let export_manifest = serde_json::json!({
        "schema_version": PACKAGE_SCHEMA_VERSION,
        "generator": {"id": GENERATOR_ID, "version": env!("CARGO_PKG_VERSION")},
        "dataset_schema_version": ds_manifest["schema_version"],
        "roles": roles,
        "inputs": input_records,
    });
    fs::write(
        dest.join("export_manifest.json"),
        serde_json::to_string_pretty(&export_manifest)?,
    )?;

    // 5. Portability gate: no declared absolute path may survive in
    //    any text file inside the package.
    let forbidden: Vec<String> = manifest
        .inputs
        .iter()
        .filter_map(|i| {
            std::path::absolute(&i.path)
                .ok()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
        })
        .chain(std::iter::once(
            job_dir.to_string_lossy().replace('\\', "/"),
        ))
        .collect();
    let mut files: Vec<String> = Vec::new();
    collect_files(&dest, &dest, &mut files)?;
    for rel in &files {
        let path = dest.join(rel);
        // Binary chunk files cannot hold stray text paths; scan text
        // outputs only.
        let text_ok = path
            .extension()
            .is_some_and(|e| matches!(e.to_str(), Some("json" | "txt" | "js" | "html")));
        if !text_ok {
            continue;
        }
        let body = fs::read_to_string(&path)?;
        for abs in &forbidden {
            if !abs.is_empty() && body.contains(abs.as_str()) {
                return Err(invalid(format!(
                    "export file {rel} still embeds absolute path {abs}"
                )));
            }
        }
    }
    span.record("len", files.len());
    Ok((dest, files))
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<String>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            collect_files(root, &entry.path(), out)?;
        } else {
            out.push(
                entry
                    .path()
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
    Ok(())
}
