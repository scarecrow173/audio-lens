//! 明示所有 workspace の数値・fresh-thread firstcall・drop 契約。
use al_math::{
    correlation::{CorrelationPlan, CorrelationWorkspace, gcc_phat, ncc},
    fft::{RealFftPlan, RealFftWorkspace},
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
#[derive(Clone, Copy, Default, Debug)]
struct Counts {
    calls: usize,
    reallocs: usize,
    bytes: usize,
    freed: usize,
}
thread_local! { static TRACK: Cell<(bool, Counts)> = const { Cell::new((false, Counts { calls:0,reallocs:0,bytes:0,freed:0 })) }; }
struct Counter;
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(l) };
        let _ = TRACK.try_with(|t| {
            let (on, mut c) = t.get();
            if on {
                c.calls += 1;
                c.bytes += l.size();
                t.set((on, c));
            }
        });
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        let _ = TRACK.try_with(|t| {
            let (on, mut c) = t.get();
            if on {
                c.freed += l.size();
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
                c.reallocs += 1;
                c.bytes += n;
                c.freed += l.size();
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
        let (_, c) = t.get();
        t.set((false, c));
        c
    })
}
fn noise(n: usize) -> Vec<f64> {
    let mut x = 0x52_u64;
    (0..n)
        .map(|_| {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((x >> 33) as f64 / (1_u64 << 31) as f64 - 0.5) * 0.4
        })
        .collect()
}
fn direct_ncc(t: &[f64], s: &[f64], max: usize) -> Vec<f64> {
    if t.is_empty() || s.len() < t.len() {
        return vec![];
    }
    let et = t.iter().map(|x| x * x).sum::<f64>();
    (0..=max.min(s.len() - t.len()))
        .map(|lag| {
            let w = &s[lag..lag + t.len()];
            let es = w.iter().map(|x| x * x).sum::<f64>();
            if et == 0.0 || es == 0.0 {
                0.0
            } else {
                t.iter().zip(w).map(|(a, b)| a * b).sum::<f64>() / (et * es).sqrt()
            }
        })
        .collect()
}
fn direct_phat(t: &[f64], s: &[f64], max: usize) -> Vec<f64> {
    let n = (s.len() + t.len() - 1).next_power_of_two();
    let spectrum: Vec<_> = (0..n)
        .map(|k| {
            let dft = |a: &[f64]| {
                a.iter().enumerate().fold((0.0, 0.0), |(r, i), (j, &v)| {
                    let angle = std::f64::consts::TAU * (j * k) as f64 / n as f64;
                    (r + v * angle.cos(), i - v * angle.sin())
                })
            };
            let (ar, ai) = dft(t);
            let (br, bi) = dft(s);
            let r = ar * br + ai * bi;
            let i = ar * bi - ai * br;
            let m = (r * r + i * i).sqrt();
            if m == 0.0 { (0.0, 0.0) } else { (r / m, i / m) }
        })
        .collect();
    (0..=max.min(s.len() - t.len()))
        .map(|j| {
            spectrum
                .iter()
                .enumerate()
                .map(|(k, &(r, i))| {
                    let angle = std::f64::consts::TAU * (j * k) as f64 / n as f64;
                    r * angle.cos() - i * angle.sin()
                })
                .sum::<f64>()
                / n as f64
        })
        .collect()
}
fn same(a: &[f64], b: &[f64]) {
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b) {
        assert!((x - y).abs() < 1e-10, "{x} vs {y}");
    }
}
#[test]
fn ownership_and_oracles() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<RealFftPlan>();
    send_sync::<CorrelationPlan>();
    let t = [1.0, -2.0, 0.5];
    let s = [0.3, 1.0, -2.0, 0.5, 0.7];
    let p = CorrelationPlan::new(t.len(), s.len());
    let mut ws = CorrelationWorkspace::new(&p);
    same(
        p.ncc(&mut ws, &t, &s, usize::MAX),
        &direct_ncc(&t, &s, usize::MAX),
    );
    same(p.gcc_phat(&mut ws, &t, &s, 2), &direct_phat(&t, &s, 2));
    same(p.ncc(&mut ws, &t, &s, 0), &ncc(&t, &s, 0));
    same(p.gcc_phat(&mut ws, &t, &s, 0), &gcc_phat(&t, &s, 0));
    same(p.ncc(&mut ws, &[0.0; 3], &s, 2), &[0.0; 3]);
    same(p.gcc_phat(&mut ws, &t, &[0.0; 5], 2), &[0.0; 3]);
    same(p.ncc(&mut ws, &t, &s, 2), &direct_ncc(&t, &s, 2));
    let inverted = s.map(|v| -v);
    same(
        p.ncc(&mut ws, &t, &inverted, 2),
        &direct_ncc(&t, &inverted, 2),
    );
}
#[test]
fn empty_short_and_shape_mismatch() {
    for (a, b) in [(0, 4), (4, 0), (4, 3)] {
        let p = CorrelationPlan::new(a, b);
        let mut w = CorrelationWorkspace::new(&p);
        assert!(p.ncc(&mut w, &vec![1.0; a], &vec![1.0; b], 10).is_empty());
    }
    let p = CorrelationPlan::new(2, 4);
    let mut w = CorrelationWorkspace::new(&p);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| p
            .ncc(&mut w, &[1.0], &[1.0; 4], 2)
            .len()))
        .is_err()
    );
    let other = CorrelationPlan::new(3, 4);
    let mut ow = CorrelationWorkspace::new(&other);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| p
            .ncc(&mut ow, &[1.0; 2], &[1.0; 4], 2)
            .len()))
        .is_err()
    );
}
#[test]
fn fresh_thread_first_fft_calls_allocate_nothing_and_workspace_drop_releases() {
    for n in [1, 2, 16384, 32768, 65536, 262144] {
        std::thread::spawn(move || {
            let p = RealFftPlan::new(n);
            let x = noise(n);
            let mut re = vec![0.0; n / 2 + 1];
            let mut im = re.clone();
            let mut back = vec![0.0; n];
            TRACK.with(|t| t.set((false, Counts::default())));
            start();
            let mut w = RealFftWorkspace::new(&p);
            let owned = stop();
            start();
            p.forward(&mut w, &x, &mut re, &mut im);
            let forward = stop();
            assert_eq!(
                (forward.calls, forward.reallocs),
                (0, 0),
                "N={n}: {forward:?}"
            );
            start();
            p.inverse(&mut w, &re, &im, &mut back);
            let inverse = stop();
            assert_eq!(
                (inverse.calls, inverse.reallocs),
                (0, 0),
                "N={n}: {inverse:?}"
            );
            same(&x, &back);
            start();
            drop(w);
            let released = stop();
            assert_eq!(
                released.freed,
                owned.bytes - owned.freed,
                "N={n}: workspace bytes"
            );
            eprintln!("fft-owned n={n}: constructor={owned:?} first_forward={forward:?} first_inverse={inverse:?} drop={released:?}");
            std::hint::black_box(&p);
        })
        .join()
        .unwrap();
    }
}
#[test]
fn fresh_thread_first_correlation_calls_allocate_nothing_and_drop_releases() {
    for (t, s) in [(3, 5), (4800, 5280), (96000, 100800)] {
        for phat in [false, true] {
            std::thread::spawn(move || {
                let p = CorrelationPlan::new(t, s);
                let x = noise(t);
                let y = noise(s);
                TRACK.with(|t| t.set((false, Counts::default())));
                start();
                let mut w = CorrelationWorkspace::new(&p);
                let owned = stop();
                start();
                let result = if phat {
                    p.gcc_phat(&mut w, &x, &y, s - t)
                } else {
                    p.ncc(&mut w, &x, &y, s - t)
                };
                std::hint::black_box(result);
                let first = stop();
                assert_eq!(
                    (first.calls, first.reallocs),
                    (0, 0),
                    "shape=({t},{s}), phat={phat}: {first:?}"
                );
                start();
                for _ in 0..100 {
                    std::hint::black_box(if phat {
                        p.gcc_phat(&mut w, &x, &y, s - t)
                    } else {
                        p.ncc(&mut w, &x, &y, s - t)
                    });
                }
                let repeated = stop();
                assert_eq!((repeated.calls, repeated.reallocs), (0, 0));
                start();
                drop(w);
                let released = stop();
                assert_eq!(
                    released.freed,
                    owned.bytes - owned.freed,
                    "workspace bytes; plan stays alive"
                );
                eprintln!("correlation-owned ({t},{s}) phat={phat}: constructor={owned:?} first={first:?} repeated={repeated:?} drop={released:?}");
                std::hint::black_box(&p);
            })
            .join()
            .unwrap();
        }
    }
}
#[test]
fn shared_plan_worker_workspaces_equal_sequential() {
    let p = std::sync::Arc::new(CorrelationPlan::new(63, 80));
    let x = noise(63);
    let y = noise(80);
    let mut w = CorrelationWorkspace::new(&p);
    let expected_ncc = p.ncc(&mut w, &x, &y, 17).to_vec();
    let expected_phat = p.gcc_phat(&mut w, &x, &y, 17).to_vec();
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let p = p.clone();
            let (x, y, a, b) = (&x, &y, &expected_ncc, &expected_phat);
            scope.spawn(move || {
                let mut w = CorrelationWorkspace::new(&p);
                same(p.ncc(&mut w, x, y, 17), a);
                same(p.gcc_phat(&mut w, x, y, 17), b);
            });
        }
    });
}

#[test]
fn large_fft_ncc_matches_direct_sliding_oracle() {
    let template = noise(173);
    let signal = noise(32980);
    let plan = CorrelationPlan::new(template.len(), signal.len());
    let mut ws = CorrelationWorkspace::new(&plan);
    same(
        plan.ncc(&mut ws, &template, &signal, 3),
        &direct_ncc(&template, &signal, 3),
    );
}

#[test]
fn plan_drop_and_workspace_drop_release_distinct_owned_buffers() {
    std::thread::spawn(|| {
        TRACK.with(|t| t.set((false, Counts::default())));
        start();
        let plan = CorrelationPlan::new(96000, 100800);
        let plan_owned = stop();
        start();
        let workspace = CorrelationWorkspace::new(&plan);
        let workspace_owned = stop();
        start();
        drop(plan);
        let plan_drop = stop();
        assert_eq!(plan_drop.freed, plan_owned.bytes - plan_owned.freed);
        std::hint::black_box(&workspace);
        start();
        drop(workspace);
        let workspace_drop = stop();
        assert_eq!(workspace_drop.freed, workspace_owned.bytes - workspace_owned.freed);
        eprintln!("separate-owned plan={plan_owned:?} plan_drop={plan_drop:?} workspace={workspace_owned:?} workspace_drop={workspace_drop:?}");
    }).join().unwrap();
}
