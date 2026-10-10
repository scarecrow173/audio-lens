//! テスト用ディレクトリの所有と、panic を含むスコープ終了時の回収。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// 作成直後からパスを所有する。fixture 構築途中の失敗も回収対象。
#[derive(Debug)]
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "{tag}-{}-{epoch}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        // 既存パスを引き継いで削除しないよう、create_dir_all は使わない。
        std::fs::create_dir(&path).expect("create fixture directory");
        Self(path)
    }
}

impl std::ops::Deref for TempDir {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for TempDir {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // unwind 中の二重 panic は避けるが、回収失敗は診断に残す。
        if let Err(error) = std::fs::remove_dir_all(&self.0) {
            eprintln!("fixture cleanup failed ({}): {error}", self.0.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directories_are_removed_on_success_and_assertion_failure() {
        let path;
        {
            let dir = TempDir::new("al-testkit-success");
            path = dir.to_path_buf();
            std::fs::write(dir.join("fixture"), b"fixture").unwrap();
        }
        assert!(!path.exists());

        let mut failed_path = None;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let dir = TempDir::new("al-testkit-failure");
            failed_path = Some(dir.to_path_buf());
            std::fs::create_dir(dir.join("nested")).unwrap();
            std::fs::write(dir.join("nested/fixture"), b"fixture").unwrap();
            assert_eq!(1, 2, "simulate failed golden assertion");
        }));
        assert!(result.is_err());
        assert!(!failed_path.unwrap().exists());
    }
}
