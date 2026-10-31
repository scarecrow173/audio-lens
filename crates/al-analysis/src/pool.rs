//! stage 内並列化で共有する ordered-slot ワーカープール。
//!
//! al-locate::stage `run_pool` と同じパターン: `n_tasks` 個の独立
//! タスクを scoped worker で実行し、`Vec<Mutex<Option<T>>>` の slot
//! へ書き込んでタスク順に回収する。各 worker は `make_ctx` で自分
//! 専用のコンテキストを構築するため、plan・workspace 等の排他資源は
//! スレッド間で共有されない。

/// Run `n_tasks` independent units of work over a worker pool,
/// collecting one result per task in input order. Each worker builds
/// its own context via `make_ctx`, so per-worker resources (e.g.
/// correlation plans/workspaces) stay exclusively owned by that
/// thread. al-locate::stage `run_pool` と同じパターン。
///
/// `max_workers` は同時実行数の追加上限。実効 worker 数は
/// `min(n_tasks, available_parallelism, max_workers)` になる。
/// decode 済み PCM や band signal のような大きなワーキングセットを
/// 持つ外側タスクは小さい上限で常駐メモリを抑え、`usize::MAX` で
/// 従来どおりコア数まで占有する。
pub(crate) fn run_pool<T: Send, C>(
    n_tasks: usize,
    max_workers: usize,
    make_ctx: &(dyn Fn() -> C + Sync),
    work: &(dyn Fn(usize, &mut C) -> T + Sync),
) -> Vec<T> {
    let slots: Vec<std::sync::Mutex<Option<T>>> =
        (0..n_tasks).map(|_| std::sync::Mutex::new(None)).collect();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(n_tasks)
        .min(max_workers);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                let mut ctx = make_ctx();
                loop {
                    let idx = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if idx >= n_tasks {
                        return;
                    }
                    *slots[idx].lock().unwrap() = Some(work(idx, &mut ctx));
                }
            });
        }
    });
    slots
        .into_iter()
        .map(|s| s.into_inner().unwrap().unwrap())
        .collect()
}
