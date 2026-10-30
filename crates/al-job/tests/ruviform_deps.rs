//! Ruviform Git 依存(AL#54 の OfflineDriver 適合性評価)の import
//! smoke-test。基礎型の構築とリンクのみを検証する
//! (docs/designs/ruviform-dependency-ci.md §1)。
//! ruviform-exec は採否未定のため dev-dep としてのみ参照する。
use ruviform_exec::{CancellationToken, OfflineConfig, Progress};

#[test]
fn exec_offline_types() {
    assert_ne!(ruviform_exec::version(), "");
    let config = OfflineConfig::default();
    assert_eq!(config.workers, 1);

    let cancel = CancellationToken::new();
    assert!(!cancel.is_cancelled());
    cancel.cancel();
    assert!(cancel.is_cancelled());

    let progress = Progress::new();
    assert_eq!(progress.fraction(), 0.0);
}
