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
    write_via_tmp(path, bytes, true)
}

/// Write `bytes` to `path` atomically but without fsyncs.
///
/// The rename keeps concurrent readers safe — they see either the old
/// file or the new one, never a torn write — but a power loss may lose
/// the file entirely or leave it truncated. Only use this for data that
/// is fully regenerable from the job's declared inputs; the resume-time
/// artifact verification detects such loss and re-runs the stage.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    write_via_tmp(path, bytes, false)
}

/// Shared tmp-write skeleton for `write_durable` and `write_atomic`.
fn write_via_tmp(path: &Path, bytes: &[u8], durable: bool) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = tmp_path_for(path);
    let result = (|| {
        {
            let mut file = fs::File::create(&tmp)?;
            file.write_all(bytes)?;
            if durable {
                file.sync_all()?;
            }
        }
        if durable {
            rename_durable(&tmp, path)
        } else {
            rename_atomic(&tmp, path)
        }
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

/// Rename a temp file over `path` without flushing the directory
/// entry. Same loss semantics as [`write_atomic`]: atomic for
/// concurrent readers, not durable across power loss.
///
/// For streamed writes: create the temp file with
/// [`tmp_path_for`], write it (no `sync_all`), then call this.
pub fn rename_atomic(tmp: &Path, path: &Path) -> io::Result<()> {
    fs::rename(tmp, path)
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

    // The create→finish rewrite paths (manifest.json, array.json) rely
    // on the rename replacing an existing destination atomically —
    // which `fs::rename` guarantees on both POSIX and Windows.
    #[test]
    fn write_durable_replaces_existing_destination() {
        let dir = std::env::temp_dir().join(format!("fsutil-{}", al_core::JobId::new()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("manifest.json");
        write_durable(&path, b"first").unwrap();
        write_durable(&path, b"second").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second");
        assert!(!tmp_path_for(&path).exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_atomic_replaces_existing_destination() {
        let dir = std::env::temp_dir().join(format!("fsutil-{}", al_core::JobId::new()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("chunk.bin");
        write_atomic(&path, b"first").unwrap();
        write_atomic(&path, b"second").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second");
        assert!(!tmp_path_for(&path).exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rename_atomic_renames_without_sync() {
        let dir = std::env::temp_dir().join(format!("fsutil-{}", al_core::JobId::new()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("out.wav");
        let tmp = tmp_path_for(&path);
        fs::write(&tmp, b"payload").unwrap();
        rename_atomic(&tmp, &path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"payload");
        assert!(!tmp.exists());
        fs::remove_dir_all(&dir).ok();
    }
}
