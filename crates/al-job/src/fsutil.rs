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
use std::path::{Component, Path, PathBuf};

/// Join `rel` to `job_dir`, rejecting paths that would escape it.
///
/// Persisted manifests are untrusted input on resume: an absolute path
/// or `..` component must never read or write outside the job.
pub fn checked_job_path(job_dir: &Path, rel: &Path) -> io::Result<PathBuf> {
    if rel.is_absolute()
        || rel
            .components()
            .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("path escapes job dir: {}", rel.display()),
        ));
    }
    Ok(job_dir.join(rel))
}

/// Write `bytes` to `path` atomically and durably.
pub fn write_durable(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = tmp_path_for(path);
    let result = (|| {
        {
            let mut file = fs::File::create(&tmp)?;
            file.write_all(bytes)?;
            file.sync_all()?;
        }
        rename_durable(&tmp, path)
    })();
    if result.is_err() {
        // A failed write must not leave the temp file behind.
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Rename an already-synced temp file over `path`, then flush the
/// rename's directory entry where the platform supports it (directory
/// fsync is not available on Windows).
///
/// For streamed writes: create the temp file with
/// [`tmp_path_for`], write and `sync_all` it, then call this.
pub fn rename_durable(tmp: &Path, path: &Path) -> io::Result<()> {
    fs::rename(tmp, path)?;
    #[cfg(unix)]
    if let Some(parent) = path.parent()
        && let Ok(dir) = fs::File::open(parent)
    {
        let _ = dir.sync_all();
    }
    Ok(())
}

/// The sibling temp path `write_durable` uses for `path`.
pub fn tmp_path_for(path: &Path) -> std::path::PathBuf {
    path.with_file_name(format!(
        "{}.tmp",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("tmp")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_relative_paths_pass() {
        let job = Path::new("job");
        assert_eq!(
            checked_job_path(job, Path::new("artifacts/decoded.f64le")).unwrap(),
            Path::new("job/artifacts/decoded.f64le")
        );
        assert_eq!(
            checked_job_path(job, Path::new("./artifacts/x")).unwrap(),
            Path::new("job/artifacts/x")
        );
    }

    #[test]
    fn traversal_is_rejected() {
        let job = Path::new("job");
        assert!(checked_job_path(job, Path::new("../escape")).is_err());
        assert!(checked_job_path(job, Path::new("a/../../escape")).is_err());
        assert!(checked_job_path(job, Path::new("..")).is_err());
    }

    #[test]
    fn absolute_paths_are_rejected() {
        let job = Path::new("job");
        #[cfg(windows)]
        let abs = Path::new(r"C:\tmp\evil.bin");
        #[cfg(not(windows))]
        let abs = Path::new("/tmp/evil.bin");
        assert!(checked_job_path(job, abs).is_err());
        #[cfg(windows)]
        assert!(checked_job_path(job, Path::new(r"\\server\share")).is_err());
    }
}
