//! 入力準備の再利用と、区間・版・レートによる分離。
use std::sync::Arc;

use al_core::Validity;
use al_locate::methods::{BandNcc, FullNcc, LocateMethod, propose_coarse_to_fine};
use al_locate::pool::CorrelationPool;
use al_locate::prepared::InputCache;

#[test]
fn cache_reuses_only_the_same_signal_contract() {
    let signal: Vec<_> = (0..1025).map(|i| (i as f64 * 0.31).sin()).collect();
    let mut cache = InputCache::default();
    let first = cache
        .prepare("capture", 0, 48000, "original", &signal)
        .unwrap();
    let again = cache
        .prepare("capture", 0, 48000, "original", &signal)
        .unwrap();
    assert!(Arc::ptr_eq(&first, &again));
    for (id, channel, rate, version, samples) in [
        ("other", 0, 48000, "original", signal.as_slice()),
        ("capture", 1, 48000, "original", signal.as_slice()),
        ("capture", 0, 44100, "original", signal.as_slice()),
        ("capture", 0, 48000, "residual:parent", signal.as_slice()),
        ("capture", 0, 48000, "original", &signal[..1024]),
    ] {
        let other = cache.prepare(id, channel, rate, version, samples).unwrap();
        assert!(!Arc::ptr_eq(&first, &other));
    }
    assert_eq!(first.coarse_samples().unwrap().len(), 257);
}

#[test]
fn cached_band_response_is_exactly_the_existing_full_interval_filter() {
    let mut cache = InputCache::default();
    for n in [1, 255, 256, 257, 1025] {
        let mut signal: Vec<_> = (0..n).map(|i| (i as f64 * 0.43).sin()).collect();
        signal[0] += 2.0;
        signal[n - 1] -= 3.0;
        let prepared = cache
            .prepare("capture", 0, 48000, "original", &signal)
            .unwrap();
        for ((lo, hi), band) in prepared.bands() {
            assert_eq!(
                band.as_ref(),
                al_locate::dsp::bandpass(&signal, 48000, *lo, *hi)
            );
        }
    }
}

#[test]
fn prepared_methods_keep_all_global_evidence_exactly() {
    let rate = 512;
    let mut state = 17u64;
    let template: Vec<_> = (0..2049)
        .map(|_| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            (state >> 33) as f64 / (1u64 << 31) as f64 - 0.5
        })
        .collect();
    // 近傍外の競合ピーク、端点、極性反転、奇数長を同時に含む。
    let mut signal = vec![0.0; 9001];
    signal[..template.len()].copy_from_slice(&template);
    for (out, value) in signal[6952..].iter_mut().zip(&template) {
        *out = -value;
    }
    let mut cache = InputCache::default();
    let t = cache
        .prepare("source", 0, rate, "original", &template)
        .unwrap();
    let s = cache
        .prepare("capture", 0, rate, "original", &signal)
        .unwrap();
    let trace = propose_coarse_to_fine(
        &template,
        &signal,
        rate,
        &t,
        &s,
        &mut CorrelationPool::new(),
    )
    .unwrap();
    let (evidence, actual_trace) = FullNcc.locate_coarse_to_fine(
        &template,
        &signal,
        rate,
        &t,
        &s,
        &mut CorrelationPool::new(),
    );
    assert_eq!(
        evidence,
        FullNcc.locate(&template, &signal, rate, &mut CorrelationPool::new())
    );
    // `template` / `-template` の2出現で |corr| が厳密同点となるため、
    // 証拠は AmbiguousAlignment + 候補保持となる(意味論シフト)。
    assert_eq!(evidence.validity, Validity::AmbiguousAlignment);
    assert!(evidence.candidates.len() >= 2);
    assert_eq!(
        actual_trace.unwrap().refined_positions,
        trace.refined_positions
    );
    assert_eq!(trace.search_range, [0, 6952]);
    assert_eq!(trace.coarse_rate, 128);
    assert!(!trace.coarse_positions.is_empty());
    assert!(!trace.refined_positions.is_empty());
    assert!(trace.full_search_required);
    for method in [&FullNcc as &dyn LocateMethod, &BandNcc] {
        assert_eq!(
            method.locate(&template, &signal, rate, &mut CorrelationPool::new()),
            method.locate_prepared(
                &template,
                &signal,
                rate,
                &t,
                &s,
                &mut CorrelationPool::new()
            )
        );
    }
    assert!(
        FullNcc
            .locate_prepared(
                &template,
                &signal,
                rate,
                &t,
                &s,
                &mut CorrelationPool::new()
            )
            .candidates
            .iter()
            .any(|c| c.position == 6952)
    );
}

#[test]
fn coarse_fallback_preserves_periodic_broad_and_threshold_adjacent_evidence() {
    for rate in [512, 513] {
        for frequency in [0.002, 0.12] {
            let template: Vec<_> = (0..2051).map(|i| (i as f64 * frequency).sin()).collect();
            let signal: Vec<_> = (0..7001)
                .map(|i| (i as f64 * frequency + 0.7).sin() + 0.3 * (i as f64 * 0.713).sin())
                .collect();
            let mut cache = InputCache::default();
            let t = cache
                .prepare("source", 0, rate, "original", &template)
                .unwrap();
            let s = cache
                .prepare("capture", 0, rate, "original", &signal)
                .unwrap();
            let (actual, trace) = FullNcc.locate_coarse_to_fine(
                &template,
                &signal,
                rate,
                &t,
                &s,
                &mut CorrelationPool::new(),
            );
            let expected = FullNcc.locate(&template, &signal, rate, &mut CorrelationPool::new());
            assert_eq!(actual, expected);
            let trace = trace.unwrap();
            assert!(trace.full_search_required);
            assert!(trace.coarse_positions.len() > 1);
            let config = al_locate::ConsensusConfig {
                min_peak_to_background: expected.candidates[0].peak_to_background + 1e-10,
                min_agreeing_methods: 1,
                ..Default::default()
            };
            let evaluate = |evidence| {
                al_locate::evaluate(
                    &[evidence],
                    "s",
                    "c",
                    template.len() as u64,
                    signal.len() as u64,
                    &config,
                    "test",
                    "2",
                )
            };
            assert_eq!(evaluate(actual).accepted, evaluate(expected).accepted);
        }
    }
}

#[test]
fn missing_coarse_information_requires_full_search() {
    let template = vec![0.0; 1024];
    let signal = vec![0.0; 2048];
    let mut cache = InputCache::default();
    let t = cache
        .prepare("source", 0, 512, "original", &template)
        .unwrap();
    let s = cache
        .prepare("capture", 0, 512, "original", &signal)
        .unwrap();
    let trace =
        propose_coarse_to_fine(&template, &signal, 512, &t, &s, &mut CorrelationPool::new())
            .unwrap();
    assert!(trace.coarse_positions.is_empty());
    assert_eq!(trace.fallback_reason, "no-coarse-candidates");
    assert!(trace.full_search_required);
}

fn noise(seed: u64, n: usize) -> Vec<f64> {
    let mut state = seed;
    (0..n)
        .map(|_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0
        })
        .collect()
}

fn trace_at_offset(offset: usize, rate: u32) -> al_locate::methods::CoarseSearch {
    let template = noise(11, 2049);
    let mut signal = noise(23, 9001);
    // 埋め込み位置にだけ template を載せ、他区間は独立雑音にする。
    signal[offset..offset + template.len()].copy_from_slice(&template);
    let mut cache = InputCache::default();
    let t = cache
        .prepare("source", 0, rate, "original", &template)
        .unwrap();
    let s = cache
        .prepare("capture", 0, rate, "original", &signal)
        .unwrap();
    propose_coarse_to_fine(
        &template,
        &signal,
        rate,
        &t,
        &s,
        &mut CorrelationPool::new(),
    )
    .unwrap()
}

#[test]
fn coarse_positions_land_within_radius_of_the_true_offset() {
    let rate = 512;
    let offset = 3000usize;
    let trace = trace_at_offset(offset, rate);
    // 間引き位置→原レート換算は丸め込みで真値から coarse_rate 以内、
    // かつ検証半径内に収まること。
    assert!(
        trace
            .refined_positions
            .iter()
            .any(|&p| (p as i64 - offset as i64).unsigned_abs() <= trace.neighborhood_radius as u64),
        "refined={:?} radius={} offset={offset}",
        trace.refined_positions,
        trace.neighborhood_radius
    );
    assert!(
        trace
            .coarse_positions
            .iter()
            .any(|&p| (p as i64 - offset as i64).unsigned_abs() <= trace.coarse_rate as u64),
        "coarse={:?} coarse_rate={} offset={offset}",
        trace.coarse_positions,
        trace.coarse_rate
    );
}

#[test]
fn candidates_at_the_search_range_edges_are_marked_boundary() {
    let rate = 512;
    // 真のオフセットが search_range の両端にある fixture では
    // "boundary-candidate" が記録される。
    for offset in [0usize, 9001 - 2049] {
        let trace = trace_at_offset(offset, rate);
        assert_eq!(
            trace.fallback_reason, "boundary-candidate",
            "offset={offset} coarse={:?} radius={}",
            trace.coarse_positions, trace.neighborhood_radius
        );
        assert!(trace.full_search_required);
    }
}
