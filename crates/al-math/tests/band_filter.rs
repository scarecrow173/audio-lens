//! `BandFilter` 逐次生成のライブバイト計測。帯域ループ中に全帯域が
//! 同時保持されないことを counting allocator で検証する
//! (先例: `ncc_workspace.rs`)。
use al_math::filter::{BandFilter, bandpass_many};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

#[derive(Clone, Copy, Default, Debug)]
struct Counts {
    live: usize,
    max_live: usize,
}
thread_local! {
    static TRACK: Cell<(bool, Counts)> =
        const { Cell::new((false, Counts { live: 0, max_live: 0 })) };
}
struct Counter;
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(l) };
        let _ = TRACK.try_with(|t| {
            let (on, mut c) = t.get();
            if on {
                c.live += l.size();
                c.max_live = c.max_live.max(c.live);
                t.set((on, c));
            }
        });
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        let _ = TRACK.try_with(|t| {
            let (on, mut c) = t.get();
            if on {
                c.live -= l.size();
                t.set((on, c));
            }
        });
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let q = unsafe { System.realloc(p, l, n) };
        let _ = TRACK.try_with(|t| {
            let (on, mut c) = t.get();
            if on {
                c.live = c.live + n - l.size();
                c.max_live = c.max_live.max(c.live);
                t.set((on, c));
            }
        });
        q
    }
}
#[global_allocator]
static ALLOC: Counter = Counter;

fn start() {
    TRACK.with(|t| t.set((true, Counts::default())));
}
fn stop() -> Counts {
    TRACK.with(|t| {
        let c = t.get().1;
        t.set((false, c));
        c
    })
}

#[test]
fn band_streaming_holds_at_most_one_band_waveform_per_filter() {
    let len = 1 << 20;
    let rate = 48000;
    let input: Vec<f64> = (0..len).map(|i| (i as f64 * 0.713).sin()).collect();
    let bands: Vec<(f64, f64)> = (0..26)
        .map(|i| (40.0 * 1.26f64.powi(i), 40.0 * 1.26f64.powi(i + 1)))
        .collect();

    // 逐次経路: BandFilter 構築後、帯域ごとに生成→消費→破棄する。
    // 消費側が 2 入力分同時に持つ想定で上限は 2 帯域分。
    let mut src = BandFilter::new(&input, rate);
    let mut cap = BandFilter::new(&input, rate);
    start();
    let mut consumed = 0usize;
    for &(lo, hi) in &bands {
        let x = src.band(lo, hi);
        let y = cap.band(lo, hi);
        consumed += x.len() + y.len();
        drop(x);
        drop(y);
    }
    let streaming = stop();
    assert_eq!(consumed, bands.len() * len * 2);
    let slack = 64 * 1024;
    assert!(
        streaming.max_live <= 2 * len * 8 + slack,
        "streaming live={} > 2 bands",
        streaming.max_live
    );

    // 対照: 一括経路は全帯域を保持する。
    start();
    let all = bandpass_many(&input, rate, &bands);
    let batch = stop();
    assert_eq!(all.len(), bands.len());
    assert!(
        batch.max_live >= (bands.len() - 1) * len * 8,
        "batch live={} < ~{} bands",
        batch.max_live,
        bands.len() - 1
    );
    eprintln!(
        "streaming max_live={} vs batch max_live={} ({}x)",
        streaming.max_live,
        batch.max_live,
        batch.max_live / streaming.max_live.max(1)
    );
}
