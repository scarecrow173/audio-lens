//! Relocatable export package: copy `analysis/` in full, shrink
//! absolute source paths down to file-name + hash references, and
//! emit a package manifest. The package must read on its own —
//! numeric/tensor data travels with it; original media only travels
//! when `config.export.copy_media` is set.
//!
//! Portability contract (issue #141): the published package must not
//! carry a host-private location — declared input source paths, the
//! job directory, or any other absolute path — in any separator or
//! escape spelling. The projection is schema-tolerant rather than
//! per-file: every `.json` artifact in the copied dataset is parsed
//! and every string value that looks like an absolute path on any
//! supported host (POSIX root, drive-letter, or UNC) is replaced by
//! its file name. Value semantics are preserved — input ids, roles,
//! digests, stream/timing metadata, and validity facts travel
//! unchanged; only the host-specific location is projected away. An
//! artifact that cannot be parsed or still contains a private path in
//! a non-JSON text file fails the build closed, leaves the source
//! dataset untouched, and reports the offending file.

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
/// file-name references, so a copied text artifact embeds no
/// host-specific locations. Returns whether any value changed; an
/// unchanged document keeps its original bytes on disk.
fn shrink_abs_paths(v: &mut serde_json::Value) -> bool {
    // Every branch is visited — a short-circuiting combinator would
    // stop projecting later siblings after the first hit.
    let mut changed = false;
    match v {
        serde_json::Value::String(s) if looks_absolute(s) => {
            *s = base_name(s).to_string();
            changed = true;
        }
        serde_json::Value::Array(items) => {
            for item in items {
                changed |= shrink_abs_paths(item);
            }
        }
        serde_json::Value::Object(map) => {
            for item in map.values_mut() {
                changed |= shrink_abs_paths(item);
            }
        }
        _ => {}
    }
    changed
}

/// Parse and project every `.json` artifact under `dir`: any
/// absolute-path string value becomes its file name. Unchanged
/// documents keep their original bytes; only files that embedded a
/// host path are re-serialized. A `.json` file that does not parse
/// cannot be inspected, so the build fails closed instead of
/// publishing unverifiable text.
fn sanitize_json_tree(pkg_root: &Path, dir: &Path) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            sanitize_json_tree(pkg_root, &path)?;
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let bytes = fs::read(&path)?;
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
            let rel = path
                .strip_prefix(pkg_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            invalid(format!("export artifact {rel} is not valid JSON: {e}"))
        })?;
        if shrink_abs_paths(&mut value) {
            fs::write(&path, serde_json::to_string_pretty(&value)?)?;
        }
    }
    Ok(())
}

/// Every textual spelling a private path can take inside package
/// text: the `/`-normalized form, the raw `\` form, and the
/// JSON-escaped doubled-`\` form a serialized string literal
/// produces. The gate checks all three so a leak is caught on the
/// producing host's convention and on any consumer's.
fn path_spellings(path: &Path) -> Vec<String> {
    let fwd = path.to_string_lossy().replace('\\', "/");
    let back = fwd.replace('/', "\\");
    let esc = back.replace('\\', "\\\\");
    let mut out = vec![fwd];
    for s in [back, esc] {
        if !out.contains(&s) {
            out.push(s);
        }
    }
    out
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
/// list of package-relative files written. The package is assembled
/// in a staging sibling (`export/.<name>.partial`) and published by
/// rename only after the portability gate passes, so a failed build
/// never leaves a half-written tree posing as a finished package —
/// and a previously published package survives a failed rerun
/// untouched (replacement semantics on success only).
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
    let export_root = job_dir.join("export");
    fs::create_dir_all(&export_root)?;
    let dest = export_root.join(&cfg.name);
    let staging = export_root.join(format!(".{}.partial", cfg.name));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    match build_package(job_dir, &staging, manifest, cfg) {
        Ok(files) => {
            if dest.exists() {
                fs::remove_dir_all(&dest)?;
            }
            fs::rename(&staging, &dest)?;
            span.record("len", files.len());
            Ok((dest, files))
        }
        Err(e) => {
            let _ = fs::remove_dir_all(&staging);
            Err(e)
        }
    }
}

/// Assemble the package under `dest` (a staging directory that must
/// not yet exist) and return the package-relative file list. Any
/// error leaves `dest` partially written for the caller to discard.
fn build_package(
    job_dir: &Path,
    dest: &Path,
    manifest: &Manifest,
    cfg: &ExportConfig,
) -> io::Result<Vec<String>> {
    fs::create_dir_all(dest)?;

    // 1. Copy the dataset wholesale.
    copy_tree(&job_dir.join("analysis"), &dest.join("analysis"))?;

    // 2. Input identities for the package manifest, read before the
    //    projection shrinks their host paths to file names.
    let manifest_path = dest.join("analysis/manifest.json");
    let ds_manifest: serde_json::Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    let mut input_records = Vec::new();
    if let Some(inputs) = ds_manifest["inputs"].as_array() {
        for input in inputs {
            let source_path = input["source_path"].as_str().unwrap_or("");
            let file_name = base_name(source_path).to_string();
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

    // 3. Portable projection over every copied JSON artifact: input
    //    declarations, per-input ingest metadata, the dataset
    //    manifest (inputs + config + provenance), and any future
    //    text store. Absolute paths become file names; everything
    //    else keeps its bytes.
    sanitize_json_tree(dest, &dest.join("analysis"))?;

    // 4. Optional media copy: each input lands under its own
    //    `media/<input_id>/` directory — the same package-relative
    //    path recorded in the export manifest — so two inputs with
    //    the same file name keep distinct copies and each recorded
    //    sha256 matches the file it points at.
    if cfg.copy_media {
        for input in &manifest.inputs {
            let file_name = base_name(&input.path.to_string_lossy()).to_string();
            if file_name.is_empty() || looks_absolute(&file_name) {
                return Err(invalid(format!(
                    "input {} has no file name",
                    input.input_id
                )));
            }
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

    // 5. Package manifest (deterministic: no timestamps, relative
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

    // 6. Portability gate: no private path may survive in any text
    //    file inside the package, in any separator or escape
    //    spelling. Declared inputs and the job dir are checked in
    //    both their recorded and their absolutized spellings.
    let mut roots: Vec<PathBuf> = manifest.inputs.iter().map(|i| i.path.clone()).collect();
    roots.push(job_dir.to_path_buf());
    for p in roots.clone() {
        if let Ok(a) = std::path::absolute(&p)
            && !roots.contains(&a)
        {
            roots.push(a);
        }
    }
    let forbidden: Vec<String> = roots.iter().flat_map(|p| path_spellings(p)).collect();
    let mut files: Vec<String> = Vec::new();
    collect_files(dest, dest, &mut files)?;
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
    Ok(files)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_absolute_covers_every_supported_convention() {
        for s in [
            "/tmp/x/cap.wav",
            "/",
            "//server/share/cap.wav",
            "C:/x/cap.wav",
            "C:\\x\\cap.wav",
            "d:\\x\\cap.wav",
            "\\\\server\\share\\cap.wav",
            "\\\\?\\C:\\x\\cap.wav",
            "\\rooted\\cap.wav",
        ] {
            assert!(looks_absolute(s), "{s:?} must be absolute");
        }
        for s in [
            "cap.wav",
            "rel/dir/cap.wav",
            "./rel/cap.wav",
            "../rel/cap.wav",
            "C:rel/cap.wav",
            "a:b",
            "",
            "not a path but /has a slash",
        ] {
            assert!(!looks_absolute(s), "{s:?} must not be absolute");
        }
    }

    #[test]
    fn base_name_takes_the_last_segment_under_either_separator() {
        assert_eq!(base_name("/tmp/x/cap.wav"), "cap.wav");
        assert_eq!(base_name("C:\\x\\cap.wav"), "cap.wav");
        assert_eq!(base_name("C:/x/cap.wav"), "cap.wav");
        assert_eq!(base_name("\\\\server\\share\\cap.wav"), "cap.wav");
        assert_eq!(base_name("D:\\media\\カラオケ.wav"), "カラオケ.wav");
        assert_eq!(base_name("cap.wav"), "cap.wav");
        assert_eq!(base_name("/tmp/dir/"), "dir");
        // Same basename from different dirs keeps the shared name —
        // disambiguation is the input id's job, not the projection's.
        assert_eq!(base_name("C:\\a\\cap.wav"), base_name("/b/cap.wav"));
    }

    #[test]
    fn shrink_rewrites_only_absolute_string_values() {
        let mut v = serde_json::json!({
            "source_path": "/tmp/x/cap.wav",
            "nested": {"win": "C:\\x\\cap.wav", "list": ["\\\\s\\sh\\f.wav", "keep.txt"]},
            "count": 3,
            "flag": true,
            "nothing": null,
            "rel": "a/b/c.txt",
            "plain": "cap.wav"
        });
        assert!(shrink_abs_paths(&mut v));
        assert_eq!(v["source_path"], "cap.wav");
        assert_eq!(v["nested"]["win"], "cap.wav");
        assert_eq!(v["nested"]["list"][0], "f.wav");
        assert_eq!(v["nested"]["list"][1], "keep.txt");
        assert_eq!(v["count"], 3);
        assert_eq!(v["flag"], true);
        assert!(v["nothing"].is_null());
        assert_eq!(v["rel"], "a/b/c.txt");
        assert_eq!(v["plain"], "cap.wav");

        // A second pass is a no-op: nothing left to project.
        assert!(!shrink_abs_paths(&mut v));
    }

    #[test]
    fn path_spellings_covers_normalized_raw_and_escaped_forms() {
        let s = path_spellings(Path::new("C:/tmp/x"));
        assert!(s.contains(&"C:/tmp/x".to_string()));
        assert!(s.contains(&"C:\\tmp\\x".to_string()));
        assert!(s.contains(&"C:\\\\tmp\\\\x".to_string()));

        let s = path_spellings(Path::new("/tmp/x"));
        assert!(s.contains(&"/tmp/x".to_string()));
        assert!(s.contains(&"\\tmp\\x".to_string()));
        assert!(s.contains(&"\\\\tmp\\\\x".to_string()));
    }
}
