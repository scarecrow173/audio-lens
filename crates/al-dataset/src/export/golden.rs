//! Golden snapshot: a deterministic `{relative_path: sha256}` map of
//! the package contents. Comparing two snapshots proves byte-level
//! reproducibility without embedding the contents themselves.

use std::fs;
use std::io;
use std::path::Path;

/// Build `{path: sha256}` for every file under `root`, sorted by
/// path. `only` limits the snapshot to a subset of relative path
/// prefixes (empty = everything).
pub fn snapshot(root: &Path, only: &[String]) -> io::Result<serde_json::Value> {
    let span = tracing::info_span!("export::golden", len = only.len());
    let _enter = span.enter();
    let mut entries: Vec<(String, String)> = Vec::new();
    collect(root, root, only, &mut entries)?;
    entries.sort();
    let map: serde_json::Map<String, serde_json::Value> = entries
        .into_iter()
        .map(|(p, h)| (p, serde_json::Value::String(h)))
        .collect();
    Ok(serde_json::Value::Object(map))
}

fn collect(
    root: &Path,
    dir: &Path,
    only: &[String],
    out: &mut Vec<(String, String)>,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect(root, &path, only, out)?;
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if !only.is_empty() && !only.iter().any(|p| rel.starts_with(p)) {
            continue;
        }
        let bytes = fs::read(&path)?;
        out.push((rel, al_core::sha256_hex(&bytes)));
    }
    Ok(())
}
