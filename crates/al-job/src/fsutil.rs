//! Durable file writes for job-owned artifacts.
//!
//! `write_durable` fsyncs a temp file and renames it over the target so a
//! process kill or power loss can never leave a truncated file: readers
//! see either the previous complete content or the new one. Job
//! execution is serialized by the runner's advisory lock, so a fixed
//! `<name>.tmp` sibling is safe.

use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;

/// Write `bytes` to `path` atomically and durably.
pub fn write_durable(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_file_name(format!(
        "{}.tmp",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("tmp")
    ));
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    // Flush the rename's directory entry where the platform supports it
    // (directory fsync is not available on Windows).
    #[cfg(unix)]
    if let Some(parent) = path.parent()
        && let Ok(dir) = fs::File::open(parent)
    {
        let _ = dir.sync_all();
    }
    Ok(())
}
