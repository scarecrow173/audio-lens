use super::*;
use crate::events::CandidateEvent;
use al_dataset::types::Value;

#[test]
fn windowed_expensive_records_keep_the_analysis_rate_even_when_invalid() {
    let samples = noise(2048, 33);
    for rate in [8000, 32000] {
        for valid in [true, false] {
            let region = TriggeredRegion {
                start_s: 0.0,
                end_s: samples.len() as f64 / rate as f64,
                triggers: vec!["manual".into()],
                validity: if valid {
                    Validity::Valid
                } else {
                    Validity::LowCoherence
                },
                confidence: Confidence::High,
                has_valid_trigger: valid,
            };
            let out = analyze_region(
                "in",
                "r",
                &[&samples],
                rate,
                &region,
                None,
                None,
                &ExpensiveConfig {
                    stft_window: 64,
                    stft_hop: 32,
                    ..Default::default()
                },
            );
            let resolutions: Vec<_> = out.records.iter().filter_map(|r| r.resolution).collect();
            assert!(!resolutions.is_empty());
            assert!(resolutions.iter().all(|r| r.sample_rate == rate));
        }
    }
}

#[test]
fn ac29_short_secondary_channel_is_unsupported_without_panicking() {
    let primary = noise(4096, 29);
    let secondary = noise(512, 30);
    let region = TriggeredRegion {
        start_s: 1.0,
        end_s: 2.0,
        triggers: vec!["manual".into()],
        validity: Validity::Valid,
        confidence: Confidence::High,
        has_valid_trigger: true,
    };
    let cfg = ExpensiveConfig {
        stft_window: 64,
        stft_hop: 32,
        ..Default::default()
    };
    let out = analyze_region(
        "in",
        "r",
        &[&primary, &secondary],
        1024,
        &region,
        None,
        None,
        &cfg,
    );
    let coherence = record(&out, "exp.freq_lr_coherence");
    assert_eq!(coherence.validity, Validity::InsufficientSupport);
    assert_eq!(coherence.channels, Some(ChannelSelector::ChannelPair(0, 1)));
    assert!(val(coherence).is_nan());
}

fn ac29_run(
    sig: &[&[f64]],
    rate: u32,
    window: usize,
    pair: Option<&PairEvidence<'_>>,
) -> ExpensiveResult {
    let region = TriggeredRegion {
        start_s: 0.0,
        end_s: sig[0].len() as f64 / rate as f64,
        triggers: vec!["manual".into()],
        validity: Validity::Valid,
        confidence: Confidence::High,
        has_valid_trigger: true,
    };
    analyze_region(
        "in",
        "r",
        sig,
        rate,
        &region,
        None,
        pair,
        &ExpensiveConfig {
            stft_window: window,
            stft_hop: window / 2,
            ..Default::default()
        },
    )
}

#[test]
fn ac29_no_splice_has_valid_zero_count_but_no_first_time() {
    let sig = vec![0.01; 4096];
    let out = ac29_run(&[&sig], 48000, 256, None);
    assert_eq!(val(record(&out, "exp.splice_count")), 0.0);
    assert_eq!(record(&out, "exp.splice_count").validity, Validity::Valid);
    assert_eq!(
        record(&out, "exp.splice_first_s").validity,
        Validity::InsufficientSupport
    );
    assert!(val(record(&out, "exp.splice_first_s")).is_nan());
}

#[test]
fn ac29_no_codec_candidate_is_not_valid_zero() {
    let sig = noise(1024, 29);
    let out = ac29_run(&[&sig], 48000, 256, None);
    for id in ["exp.codec_block_size", "exp.codec_block_score"] {
        let r = record(&out, id);
        assert_eq!(r.validity, Validity::InsufficientSupport, "{id}");
        assert!(val(r).is_nan());
        assert_eq!(r.support_count, 0);
    }
}

#[test]
fn ac29_zero_fold_scores_have_no_source_rate_candidate() {
    let sig = vec![0.0; 8192];
    let out = ac29_run(&[&sig], 48000, 1024, None);
    let r = record(&out, "exp.src_candidate_rate");
    assert_eq!(r.validity, Validity::InsufficientSupport);
    assert!(val(r).is_nan());
}

#[test]
fn ac29_unsupported_gates_emit_records() {
    let sig = vec![0.0; 1024];
    let out = ac29_run(&[&sig], 48000, 256, None);
    let ids = [
        "exp.cutoff_hz",
        "exp.narrowband_peak_count",
        "exp.narrowband_notch_count",
        "exp.thd_ratio",
        "exp.spectral_hole_ratio",
        "exp.repeat_block_score",
    ];
    let missing: Vec<_> = ids
        .iter()
        .filter(|id| !out.records.iter().any(|r| &r.feature_id == *id))
        .collect();
    assert!(missing.is_empty(), "missing: {missing:?}");
    for id in [
        "exp.cutoff_hz",
        "exp.narrowband_peak_count",
        "exp.narrowband_notch_count",
        "exp.thd_ratio",
        "exp.spectral_hole_ratio",
        "exp.repeat_block_score",
    ] {
        let r = record(&out, id);
        assert_eq!(r.validity, Validity::InsufficientSupport, "{id}");
        assert!(val(r).is_nan(), "{id}");
        assert!(r.invalid_reason.is_some());
    }
}

#[test]
fn ac29_empty_quefrency_interval_emits_invalid_echo_records() {
    let sig = vec![0.0; 1024];
    let out = ac29_run(&[&sig], 1_000_000, 64, None);
    let ids = [
        "exp.echo_delay_s",
        "exp.cepstrum_prominence",
        "exp.echo2_delay_s",
        "exp.echo2_level_ratio",
    ];
    let missing: Vec<_> = ids
        .iter()
        .filter(|id| !out.records.iter().any(|r| &r.feature_id == *id))
        .collect();
    assert!(missing.is_empty(), "missing: {missing:?}");
    for id in [
        "exp.echo_delay_s",
        "exp.cepstrum_prominence",
        "exp.echo2_delay_s",
        "exp.echo2_level_ratio",
    ] {
        assert_eq!(
            record(&out, id).validity,
            Validity::InsufficientSupport,
            "{id}"
        );
    }
}

#[test]
fn ac29_ill_conditioned_coefficients_are_demoted() {
    let src = noise(8192, 1);
    let other: Vec<f64> = src
        .iter()
        .zip(noise(8192, 2))
        .map(|(&a, b)| a + 0.001 * b)
        .collect();
    let source_channels: [&[f64]; 2] = [&src, &other];
    let pair = aligned_pair(&source_channels, src.len());
    let out = ac29_run(&[&src], 48000, 1024, Some(&pair));
    let coeffs: Vec<_> = out
        .records
        .iter()
        .filter(|r| {
            r.feature_id == "exp.mix_band_coeff"
                && r.params["condition"].as_f64().is_some_and(|c| c > 1e3)
        })
        .collect();
    assert!(!coeffs.is_empty());
    for r in coeffs {
        assert_eq!(r.validity, Validity::LowCoherence);
        assert!(r.invalid_reason.is_some());
    }
}

#[test]
fn ac29_pair_summaries_do_not_claim_capture_channel_zero() {
    let sig = noise(8192, 29);
    let out = ac29_run(&[&sig], 48000, 256, None);
    for id in [
        "exp.level_transfer_slope",
        "exp.level_transfer_corr",
        "exp.mix_fit_quality",
        "exp.mix_polarity",
        "exp.mix_condition",
        "exp.ir_early_late_ratio",
        "exp.ir_decay_db_per_s",
        "exp.local_time_map",
    ] {
        assert_eq!(record(&out, id).channels, None, "{id}");
    }
}

#[test]
fn ac29_ir_windows_are_invariant_to_circular_peak_position() {
    let sig = noise(8192, 29);
    let measure = |shift: usize| {
        let n = 1024;
        let mut re = vec![0.0; n];
        re[shift] = 1.0;
        re[(shift + n - 1) % n] = 0.5;
        re[(shift + 300) % n] = 0.1;
        let mut im = vec![0.0; n];
        al_locate::dsp::fft(&mut re, &mut im, false);
        let mut pair = aligned_pair(&[], 0);
        pair.transfer_h = Some(re.into_iter().zip(im).collect());
        let out = ac29_run(&[&sig], 48000, 256, Some(&pair));
        val(record(&out, "exp.ir_early_late_ratio"))
    };
    let zero = measure(0);
    assert!((zero - 125.0).abs() < 1e-6, "{zero}");
    assert!((measure(900) - zero).abs() < 1e-6);
}

#[test]
fn ac29_mix_keeps_channel_identity_after_short_channel_is_excluded() {
    // channel family 境界を直接検証する。通常の orchestrator は ch0 timeline
    // で早期 return するため、この別ケースを family の入力で固定する。
    let short = noise(512, 29);
    let long = noise(4096, 30);
    let source = &long[1024..2048];
    let source_channels = [source];
    let mut pair = aligned_pair(&source_channels, 1024);
    pair.cap_start = 1024;
    let cfg = ExpensiveConfig {
        stft_window: 64,
        stft_hop: 32,
        ..Default::default()
    };
    let region = TriggeredRegion {
        start_s: 1.0,
        end_s: 2.0,
        triggers: vec!["manual".into()],
        validity: Validity::Valid,
        confidence: Confidence::High,
        has_valid_trigger: true,
    };
    let m = stft(source, 64, 32, 1024);
    let power = m.power();
    let mean_power: Vec<f64> = (0..m.fft_size)
        .map(|k| power.iter().map(|f| f[k]).sum::<f64>() / m.frames() as f64)
        .collect();
    let ctx = RegionCtx {
        region_name: "r",
        rate: 1024,
        s0: 1024,
        s1: 2048,
        chans: &[&short, &long],
        segs: vec![&[], source],
        bin_hz: 1024.0 / m.fft_size as f64,
        total_power: mean_power.iter().sum(),
        one_sided_power: mean_power[..m.fft_size / 2].iter().sum(),
        mean_power,
        stft: m,
        dictionary: None,
        pair: Some(&pair),
    };
    let mut out = ExpensiveResult::default();
    channel::run(&ctx, &Emit::new("in", &region, &cfg, ctx.rate), &mut out);
    let coeffs: Vec<_> = out
        .records
        .iter()
        .filter(|r| r.feature_id == "exp.mix_band_coeff" && r.validity == Validity::Valid)
        .collect();
    assert!(!coeffs.is_empty());
    assert!(
        coeffs
            .iter()
            .all(|r| r.channels == Some(ChannelSelector::Channel(1)))
    );
    assert!(
        out.records
            .iter()
            .any(|r| r.feature_id == "exp.mix_band_coeff"
                && r.channels == Some(ChannelSelector::Channel(0))
                && r.validity == Validity::InsufficientSupport)
    );
}

#[test]
fn ac29_validity_categories_keep_first_non_valid_without_severity_order() {
    assert_eq!(
        first_non_valid(Validity::Valid, Validity::LowCoherence),
        Validity::LowCoherence
    );
    assert_eq!(
        first_non_valid(Validity::LowCoherence, Validity::DecodeInputUnavailable),
        Validity::LowCoherence
    );
    assert_eq!(
        first_non_valid(Validity::DecodeInputUnavailable, Validity::LowCoherence),
        Validity::DecodeInputUnavailable
    );
}

#[test]
fn ac29_codec_periodicity_measures_raw_boundary_energy() {
    let sig: Vec<f64> = (0..576 * 8)
        .map(|i| if (i / 64) % 9 == 0 { 0.1 } else { 0.01 })
        .collect();
    let out = ac29_run(&[&sig], 48000, 256, None);
    assert_eq!(val(record(&out, "exp.codec_block_size")), 576.0);
    assert!((val(record(&out, "exp.codec_block_score")) - 100.0).abs() < 1e-8);
}

#[test]
fn ac29_measured_pair_summaries_keep_pair_scope_and_alignment() {
    let sig: Vec<f64> = noise(16384, 29)
        .into_iter()
        .enumerate()
        .map(|(i, v)| v * (0.01 + i as f64 / 16384.0))
        .collect();
    let source_channels: [&[f64]; 1] = [&sig];
    let mut pair = aligned_pair(&source_channels, sig.len());
    pair.transfer_h = Some(vec![(1.0, 0.0); 1024]);
    let out = ac29_run(&[&sig], 48000, 256, Some(&pair));
    for id in [
        "exp.level_transfer_slope",
        "exp.level_transfer_corr",
        "exp.mix_fit_quality",
        "exp.mix_polarity",
        "exp.mix_condition",
        "exp.ir_early_late_ratio",
        "exp.local_time_map",
    ] {
        let r = record(&out, id);
        assert_eq!(r.channels, None, "{id}");
        assert_eq!(r.validity, Validity::Valid, "{id}");
        assert_eq!(r.alignment_id.as_deref(), Some("aln-test"), "{id}");
    }
    // mono-to-mono の実測は pair summary のパラメータに明示する。
    assert_eq!(
        record(&out, "exp.local_time_map").params["capture_channel"],
        0
    );
    assert_eq!(
        record(&out, "exp.level_transfer_slope").params["source_channel"],
        0
    );
}

fn ev(kind: EventKind, start: f64, end: Option<f64>) -> CandidateEvent {
    ev_with(kind, start, end, Validity::Valid, Confidence::High)
}

fn ev_with(
    kind: EventKind,
    start: f64,
    end: Option<f64>,
    validity: Validity,
    confidence: Confidence,
) -> CandidateEvent {
    CandidateEvent {
        event_id: format!("EVT-t-{kind:?}-{validity:?}"),
        kind,
        input_id: "in".into(),
        start_s: start,
        end_s: end,
        strength: 1.0,
        confidence,
        validity,
        source_feature: "f".into(),
        frequency_range: None,
        channels: Vec::new(),
        algorithm_id: "events.v1".into(),
        algorithm_version: "1".into(),
        evidence: Vec::new(),
        params: serde_json::json!({}),
    }
}

#[test]
fn no_triggers_produces_no_regions() {
    let cfg = ExpensiveConfig::default();
    assert!(plan_regions(10.0, &[], &cfg).is_empty());
    assert!(
        plan_regions(10.0, &[ev(EventKind::Clipping, 0.0, None)], &cfg)
            .iter()
            .all(|r| r.end_s <= 10.0)
    );
}

#[test]
fn always_run_and_manual_modes() {
    let cfg = ExpensiveConfig {
        always_run: true,
        ..ExpensiveConfig::default()
    };
    let r = plan_regions(10.0, &[], &cfg);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].triggers, vec!["always_run"]);

    let cfg = ExpensiveConfig {
        manual_ranges: vec![(2.0, 4.0), (20.0, 30.0)],
        ..ExpensiveConfig::default()
    };
    let r = plan_regions(10.0, &[], &cfg);
    assert_eq!(r.len(), 1);
    assert_eq!((r[0].start_s, r[0].end_s), (2.0, 4.0));
    assert_eq!(r[0].triggers, vec!["manual"]);
}

#[test]
fn event_regions_expand_and_merge_with_reasons() {
    let cfg = ExpensiveConfig::default(); // window 0.25 s
    let events = vec![
        ev(EventKind::LargeResidual, 1.0, None),
        ev(EventKind::Clipping, 1.1, None),
        ev(EventKind::LowCoherence, 8.0, Some(9.0)),
    ];
    let r = plan_regions(10.0, &events, &cfg);
    assert_eq!(r.len(), 2, "{r:?}");
    assert!((r[0].start_s - 0.75).abs() < 1e-9);
    assert!(r[0].triggers.contains(&"high_residual".to_string()));
    assert!(
        r[0].triggers
            .iter()
            .any(|t| t.starts_with("candidate:EVT-t-Clipping"))
    );
    assert!(r[1].triggers.contains(&"low_similarity".to_string()));
    assert!(
        r[1].triggers
            .iter()
            .any(|t| t.starts_with("candidate:EVT-t-LowCoherence"))
    );
}

#[test]
fn merged_regions_carry_the_weakest_trigger_trust() {
    let cfg = ExpensiveConfig::default();
    let events = vec![
        ev_with(
            EventKind::Clipping,
            1.0,
            None,
            Validity::Valid,
            Confidence::High,
        ),
        ev_with(
            EventKind::LargeResidual,
            1.1,
            None,
            Validity::AmbiguousAlignment,
            Confidence::Low,
        ),
    ];
    let r = plan_regions(10.0, &events, &cfg);
    assert_eq!(r.len(), 1, "{r:?}");
    assert_eq!(r[0].validity, Validity::AmbiguousAlignment);
    assert_eq!(r[0].confidence, Confidence::Low);
    assert!(r[0].has_valid_trigger);

    // Non-Valid evidence alone still selects a region, flagged,
    // but without a `Valid` trigger it must not be estimated.
    let r = plan_regions(
        10.0,
        &[ev_with(
            EventKind::Clipping,
            3.0,
            None,
            Validity::InsufficientSupport,
            Confidence::Medium,
        )],
        &cfg,
    );
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].validity, Validity::InsufficientSupport);
    assert_eq!(r[0].confidence, Confidence::Medium);
    assert!(!r[0].has_valid_trigger);

    // A `manual` range is an independent `Valid` trigger: merged
    // with a non-Valid event it still licenses estimation.
    let cfg = ExpensiveConfig {
        manual_ranges: vec![(2.5, 3.5)],
        ..ExpensiveConfig::default()
    };
    let r = plan_regions(
        10.0,
        &[ev_with(
            EventKind::Clipping,
            3.0,
            None,
            Validity::InsufficientSupport,
            Confidence::Medium,
        )],
        &cfg,
    );
    assert_eq!(r.len(), 1, "{r:?}");
    assert!(r[0].has_valid_trigger);
    assert_eq!(r[0].validity, Validity::InsufficientSupport);
}

#[test]
fn non_valid_trigger_records_invalid_observations() {
    let rate = 48_000u32;
    let sig = noise(1 << 15, 0xf00d);
    let chans: Vec<&[f64]> = vec![&sig];
    let region = TriggeredRegion {
        start_s: 0.0,
        end_s: sig.len() as f64 / rate as f64,
        triggers: vec!["candidate:EVT-t".into()],
        validity: Validity::AmbiguousAlignment,
        confidence: Confidence::Low,
        has_valid_trigger: false,
    };
    let out = analyze_region(
        "in",
        "in__r000",
        &chans,
        rate,
        &region,
        None,
        None,
        &ExpensiveConfig::default(),
    );
    // Every feature family is persisted as an invalid
    // observation: mapped validity, no finite numbers, no claim
    // beyond the trigger's confidence.
    assert!(!out.records.is_empty());
    for r in &out.records {
        assert_eq!(
            r.validity,
            Validity::AmbiguousAlignment,
            "record {} must carry the mapped validity",
            r.feature_id
        );
        assert_eq!(
            r.confidence,
            Confidence::Low,
            "record {} must not out-trust its trigger",
            r.feature_id
        );
        assert!(
            r.invalid_reason.is_some(),
            "record {} must explain why it is invalid",
            r.feature_id
        );
        match r.value {
            Value::Scalar(v) => assert!(
                v.is_nan(),
                "record {} must not emit a finite number",
                r.feature_id
            ),
            _ => panic!("scalar expected"),
        }
    }
    for id in [
        "exp.cutoff_hz",
        "exp.echo_delay_s",
        "exp.echo2_delay_s",
        "exp.src_image_score",
        "exp.codec_block_score",
        "exp.hum_harmonic_count",
        "exp.noise_class",
        "exp.transfer_knee_db",
        "exp.pumping_mod_hz",
        "exp.level_transfer_slope",
        "exp.mix_band_coeff",
        "exp.intermod_ratio",
        "exp.ir_early_late_ratio",
        "exp.ir_decay_db_per_s",
        "exp.splice_count",
        "exp.stuck_block_count",
        "exp.local_time_map",
    ] {
        assert!(
            out.records.iter().any(|r| r.feature_id == id),
            "missing invalid observation for {id}"
        );
    }
    // Stereo-only features are not emitted for mono input.
    assert!(
        out.records
            .iter()
            .all(|r| r.feature_id != "exp.freq_lr_coherence")
    );
}

#[test]
fn non_valid_trigger_marks_stereo_features_invalid_too() {
    let rate = 48_000u32;
    let sig = noise(1 << 15, 0xbeef);
    let chans: Vec<&[f64]> = vec![&sig, &sig];
    let region = TriggeredRegion {
        start_s: 0.0,
        end_s: sig.len() as f64 / rate as f64,
        triggers: vec!["candidate:EVT-t".into()],
        validity: Validity::InsufficientSupport,
        confidence: Confidence::Medium,
        has_valid_trigger: false,
    };
    let out = analyze_region(
        "in",
        "in__r000",
        &chans,
        rate,
        &region,
        None,
        None,
        &ExpensiveConfig::default(),
    );
    let coh = record(&out, "exp.freq_lr_coherence");
    // InsufficientSupport maps to SilenceInsufficientEnergy.
    assert_eq!(coh.validity, Validity::SilenceInsufficientEnergy);
    assert_eq!(coh.channels, Some(ChannelSelector::ChannelPair(0, 1)));
    match coh.value {
        Value::Scalar(v) => assert!(v.is_nan()),
        _ => panic!("scalar expected"),
    }
}

#[test]
fn freq_lr_coherence_is_low_for_independent_channels() {
    // Regression: a single-frame CSD always satisfies
    // |CSD|^2 == Pxx * Pyy, so averaging per-frame magnitudes
    // degenerates to a constant 1.0. Coherence must be accumulated
    // across frames (Welch-style); independent noise reads low.
    let rate = 48_000u32;
    let a = noise(1 << 15, 0x1);
    let b = noise(1 << 15, 0x2);
    let chans: Vec<&[f64]> = vec![&a, &b];
    let region = TriggeredRegion {
        start_s: 0.0,
        end_s: a.len() as f64 / rate as f64,
        triggers: vec!["manual".into()],
        validity: Validity::Valid,
        confidence: Confidence::High,
        has_valid_trigger: true,
    };
    let out = analyze_region(
        "in",
        "in__r000",
        &chans,
        rate,
        &region,
        None,
        None,
        &ExpensiveConfig::default(),
    );
    let coh = record(&out, "exp.freq_lr_coherence");
    match coh.value {
        Value::Scalar(v) => {
            assert!(v.is_finite(), "coherence must be finite: {v}");
            assert!(
                v < 0.5,
                "independent channels must read low coherence, got {v}"
            );
        }
        _ => panic!("scalar expected"),
    }
}

#[test]
fn freq_lr_coherence_is_high_for_identical_channels() {
    let rate = 48_000u32;
    let a = noise(1 << 15, 0x3);
    let chans: Vec<&[f64]> = vec![&a, &a];
    let region = TriggeredRegion {
        start_s: 0.0,
        end_s: a.len() as f64 / rate as f64,
        triggers: vec!["manual".into()],
        validity: Validity::Valid,
        confidence: Confidence::High,
        has_valid_trigger: true,
    };
    let out = analyze_region(
        "in",
        "in__r000",
        &chans,
        rate,
        &region,
        None,
        None,
        &ExpensiveConfig::default(),
    );
    let coh = record(&out, "exp.freq_lr_coherence");
    match coh.value {
        Value::Scalar(v) => {
            assert!(v > 0.95, "identical channels must read ~1, got {v}");
        }
        _ => panic!("scalar expected"),
    }
}

#[test]
fn manual_trigger_still_estimates_when_merged_with_non_valid_event() {
    let rate = 48_000u32;
    let raw = noise(1 << 16, 0xabc);
    let sig = al_locate::dsp::bandpass(&raw, rate, 100.0, 8_000.0);
    let chans: Vec<&[f64]> = vec![&sig];
    let region = TriggeredRegion {
        start_s: 0.0,
        end_s: sig.len() as f64 / rate as f64,
        triggers: vec!["manual".into(), "candidate:EVT-t".into()],
        validity: Validity::AmbiguousAlignment,
        confidence: Confidence::Low,
        has_valid_trigger: true,
    };
    let out = analyze_region(
        "in",
        "in__r000",
        &chans,
        rate,
        &region,
        None,
        None,
        &ExpensiveConfig::default(),
    );
    // Estimation ran: finite numbers exist, but records still
    // carry the weakest trigger's validity and confidence.
    let cutoff = record(&out, "exp.cutoff_hz");
    match cutoff.value {
        Value::Scalar(v) => assert!(v.is_finite(), "manual trigger must estimate"),
        _ => panic!("scalar expected"),
    }
    for r in &out.records {
        assert_ne!(
            r.validity,
            Validity::Valid,
            "record {} must not claim Valid",
            r.feature_id
        );
        assert_eq!(r.confidence, Confidence::Low);
    }
}

fn noise(n: usize, seed: u64) -> Vec<f64> {
    let mut s = seed;
    (0..n)
        .map(|_| {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((s >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0
        })
        .map(|v| v - 0.5)
        .collect()
}

fn record<'a>(out: &'a ExpensiveResult, id: &str) -> &'a FeatureRecord {
    out.records
        .iter()
        .find(|r| r.feature_id == id)
        .unwrap_or_else(|| panic!("missing {id}"))
}

fn val(r: &FeatureRecord) -> f64 {
    match &r.value {
        al_dataset::types::Value::Scalar(v) => *v,
        _ => panic!("scalar expected"),
    }
}

#[test]
fn region_cutoff_and_echo_are_measured() {
    let rate = 48_000u32;
    // Lowpass-ish source: bandpassed noise below ~8 kHz.
    let raw = noise(1 << 16, 0xabc);
    let sig = al_locate::dsp::bandpass(&raw, rate, 100.0, 8_000.0);
    // Add an echo at 5 ms.
    let mut echoed = sig.clone();
    for i in 240..echoed.len() {
        echoed[i] += 0.4 * sig[i - 240];
    }
    let chans: Vec<&[f64]> = vec![&echoed];
    let region = TriggeredRegion {
        start_s: 0.0,
        end_s: echoed.len() as f64 / rate as f64,
        triggers: vec!["always_run".into()],
        validity: Validity::Valid,
        confidence: Confidence::High,
        has_valid_trigger: true,
    };
    let out = analyze_region(
        "in",
        "in__r000",
        &chans,
        rate,
        &region,
        None,
        None,
        &ExpensiveConfig::default(),
    );
    let cutoff = val(record(&out, "exp.cutoff_hz"));
    assert!(cutoff < 12_000.0 && cutoff > 4_000.0, "cutoff {cutoff}");
    let echo = val(record(&out, "exp.echo_delay_s"));
    assert!((echo - 0.005).abs() < 0.001, "echo delay {echo}");
    // Every record carries its trigger.
    assert!(
        out.records
            .iter()
            .all(|r| { r.params["triggers"].as_array().unwrap()[0] == "always_run" })
    );
}

/// Valid trigger + `always_run` region over `sig`, with optional
/// pair evidence.
fn run_region<'a>(
    sig: &'a [&'a [f64]],
    rate: u32,
    pair: Option<&PairEvidence<'a>>,
) -> ExpensiveResult {
    let region = TriggeredRegion {
        start_s: 0.0,
        end_s: sig[0].len() as f64 / rate as f64,
        triggers: vec!["always_run".into()],
        validity: Validity::Valid,
        confidence: Confidence::High,
        has_valid_trigger: true,
    };
    analyze_region(
        "in",
        "in__r000",
        sig,
        rate,
        &region,
        None,
        pair,
        &ExpensiveConfig::default(),
    )
}

/// Pair evidence that treats `src` as already aligned with the
/// capture's sample `cap_start..cap_start+len`.
fn aligned_pair<'a>(src: &'a [&'a [f64]], len: usize) -> PairEvidence<'a> {
    PairEvidence {
        pair: "src__in".into(),
        alignment_id: "aln-test".into(),
        accepted: true,
        validity: Validity::Valid,
        confidence: Confidence::High,
        aligned_src: src.to_vec(),
        cap_start: 0,
        len,
        transfer_h: None,
    }
}

#[test]
fn pair_dependent_families_degrade_without_pair() {
    let rate = 48_000u32;
    let sig = noise(1 << 16, 0xd0e5);
    let chans: Vec<&[f64]> = vec![&sig];
    let out = run_region(&chans, rate, None);
    for id in [
        "exp.ir_early_late_ratio",
        "exp.ir_decay_db_per_s",
        "exp.level_transfer_slope",
        "exp.level_transfer_corr",
        "exp.mix_band_coeff",
        "exp.mix_fit_quality",
        "exp.mix_polarity",
        "exp.mix_condition",
        "exp.local_time_map",
    ] {
        let r = record(&out, id);
        assert_eq!(
            r.validity,
            Validity::InsufficientSupport,
            "{id} must be InsufficientSupport without pair"
        );
        assert_eq!(
            r.invalid_reason.as_deref(),
            Some("pair evidence unavailable"),
            "{id} must name the missing pair evidence"
        );
        match r.value {
            Value::Scalar(v) => assert!(v.is_nan(), "{id} must not emit a number"),
            _ => panic!("scalar expected for {id}"),
        }
    }
}

#[test]
fn ir_family_recovers_synthetic_decay() {
    let rate = 48_000u32;
    // IR h[n] = exp(-0.004 n): fast enough to decay fully inside the
    // 2048-bin transform (truncation must not curve the EDC), slow
    // enough that the -5..-35 dB fit window spans hundreds of
    // samples. Fitted slope: -2·a·10·log10(e)·rate ≈ -1668 dB/s.
    let n = 2048usize;
    let mut re: Vec<f64> = (0..n).map(|i| (-0.004 * i as f64).exp()).collect();
    let mut im = vec![0.0; n];
    al_locate::dsp::fft(&mut re, &mut im, false);
    let h: Vec<(f64, f64)> = re.iter().copied().zip(im).collect();
    let sig = noise(1 << 16, 0x1e5e);
    let chans: Vec<&[f64]> = vec![&sig];
    let mut pair = aligned_pair(&[], 0);
    pair.transfer_h = Some(h);
    let out = run_region(&chans, rate, Some(&pair));

    let el = record(&out, "exp.ir_early_late_ratio");
    assert_eq!(el.validity, Validity::Valid);
    assert_eq!(el.alignment_id.as_deref(), Some("aln-test"));
    let decay = val(record(&out, "exp.ir_decay_db_per_s"));
    let expect = -0.004 * 20.0 * std::f64::consts::LOG10_E * rate as f64;
    assert!(
        (decay - expect).abs() < 0.1 * expect.abs(),
        "decay {decay} vs expected {expect}"
    );
    assert_eq!(decay.signum(), -1.0);
}

#[test]
fn codec_secondary_echo_is_detected() {
    let rate = 48_000u32;
    // A strong 0.6 reflection at 20 ms plus a weaker 0.25 early
    // echo at 5 ms. The cepstral primary is the 20 ms comb; the
    // |d2−d1| = 15 ms cross-term (≈a1·a2, second order) stays well
    // below the true 5 ms echo's first-order peak, so the
    // non-harmonic secondary resolves to 5 ms.
    let raw = noise(1 << 16, 0xec10);
    let sig = al_locate::dsp::bandpass(&raw, rate, 100.0, 8_000.0);
    let mut echoed = sig.clone();
    for i in 960..echoed.len() {
        echoed[i] += 0.25 * sig[i - 240] + 0.6 * sig[i - 960];
    }
    for i in 240..960 {
        echoed[i] += 0.25 * sig[i - 240];
    }
    let chans: Vec<&[f64]> = vec![&echoed];
    let out = run_region(&chans, rate, None);
    let e1 = val(record(&out, "exp.echo_delay_s"));
    assert!((e1 - 0.020).abs() < 0.002, "primary echo delay {e1}");
    let e2 = record(&out, "exp.echo2_delay_s");
    assert_eq!(
        e2.validity,
        Validity::Valid,
        "secondary echo must be supported"
    );
    assert!(
        (val(e2) - 0.005).abs() < 0.002,
        "secondary echo delay {}",
        val(e2)
    );
    assert!(val(record(&out, "exp.echo2_level_ratio")).is_finite());
}

#[test]
fn level_transfer_detects_compression() {
    let rate = 48_000u32;
    // Ramp envelope gives source-level spread; cube-root playback
    // compresses (gain_db ≈ -(2/3)·src_db).
    let n = 1 << 16;
    let raw = noise(n, 0x5a1d);
    let env: Vec<f64> = (0..n).map(|i| 0.1 + 0.9 * i as f64 / n as f64).collect();
    let src: Vec<f64> = raw.iter().zip(&env).map(|(a, b)| a * b).collect();
    let cap: Vec<f64> = src
        .iter()
        .map(|&v| v.signum() * v.abs().powf(1.0 / 3.0) * 0.5)
        .collect();
    let srcs: Vec<&[f64]> = vec![&src];
    let chans: Vec<&[f64]> = vec![&cap];
    let pair = aligned_pair(&srcs, n);
    let out = run_region(&chans, rate, Some(&pair));
    let slope = val(record(&out, "exp.level_transfer_slope"));
    assert!((-0.9..-0.3).contains(&slope), "compressor slope {slope}");
    let corr = val(record(&out, "exp.level_transfer_corr"));
    assert!(corr < -0.8, "corr {corr}");
    assert_eq!(
        record(&out, "exp.level_transfer_slope")
            .alignment_id
            .as_deref(),
        Some("aln-test")
    );
}

#[test]
fn channel_mix_recovers_signed_band_coeffs() {
    let rate = 48_000u32;
    let n = 1 << 16;
    let s0 = al_locate::dsp::bandpass(&noise(n, 0x11), rate, 100.0, 12_000.0);
    let s1 = al_locate::dsp::bandpass(&noise(n, 0x22), rate, 100.0, 12_000.0);
    // Capture ch0: +0.3·s0 − 0.8·s1 — dominant coeff −0.8 →
    // polarity-routing evidence must read negative.
    let c0: Vec<f64> = (0..n).map(|i| 0.3 * s0[i] - 0.8 * s1[i]).collect();
    let c1: Vec<f64> = (0..n).map(|i| 0.5 * s0[i] + 0.2 * s1[i]).collect();
    let srcs: Vec<&[f64]> = vec![&s0, &s1];
    let chans: Vec<&[f64]> = vec![&c0, &c1];
    let pair = aligned_pair(&srcs, n);
    let out = run_region(&chans, rate, Some(&pair));
    assert_eq!(val(record(&out, "exp.mix_polarity")), -1.0);
    let fit = val(record(&out, "exp.mix_fit_quality"));
    assert!(fit < 0.05, "exact mix must fit almost perfectly: {fit}");
    let coeffs: Vec<f64> = out
        .records
        .iter()
        .filter(|r| {
            r.feature_id == "exp.mix_band_coeff"
                && r.channels == Some(ChannelSelector::Channel(0))
                && r.validity == Validity::Valid
        })
        .map(val)
        .collect();
    assert!(!coeffs.is_empty(), "expected per-band coeff records");
    let max_c = coeffs.iter().copied().fold(0.0, f64::max);
    assert!(
        (max_c - 0.8).abs() < 0.15,
        "dominant band coeff {max_c} vs 0.8"
    );
}

#[test]
fn splice_and_stuck_buffer_are_detected() {
    let rate = 48_000u32;
    let n = 1 << 16;
    let mut sig = noise(n, 0x5c10);
    // A hard splice: replace one sample with a step far above the
    // noise's natural |Δ| (values live in [-1.5, 0.5)).
    sig[20_000] = 3.0;
    // A stuck buffer: repeat block 39 bit-exactly into blocks
    // 40-41 (chunks are aligned at 1024 boundaries).
    let (a, b) = sig.split_at_mut(40_960);
    b[..1024].copy_from_slice(&a[39_936..40_960]);
    b[1024..2048].copy_from_slice(&a[39_936..40_960]);
    let chans: Vec<&[f64]> = vec![&sig];
    let out = run_region(&chans, rate, None);
    assert_eq!(val(record(&out, "exp.splice_count")), 1.0);
    let first = val(record(&out, "exp.splice_first_s"));
    assert!(
        (first - 20_000.0 / rate as f64).abs() < 0.001,
        "splice at {first}s"
    );
    assert!(
        val(record(&out, "exp.stuck_block_count")) >= 1.0,
        "stuck runs must be counted"
    );
    // 3 identical blocks → longest run ≈ 64 ms.
    let max_ms = val(record(&out, "exp.stuck_max_ms"));
    assert!(
        (max_ms - 3.0 * 1024.0 / rate as f64 * 1e3).abs() < 1.0,
        "stuck run {max_ms}ms"
    );
}

#[test]
fn local_time_map_tracks_constant_residual_delay() {
    let rate = 48_000u32;
    let n = 1 << 16;
    let src = al_locate::dsp::bandpass(&noise(n, 0x711e), rate, 200.0, 10_000.0);
    // Capture is locally delayed by +5 samples vs the alignment —
    // the time map should read lag ≈ +5.
    let mut cap = vec![0.0f64; n];
    cap[5..].copy_from_slice(&src[..n - 5]);
    let srcs: Vec<&[f64]> = vec![&src];
    let chans: Vec<&[f64]> = vec![&cap];
    let pair = aligned_pair(&srcs, n - 64);
    let out = run_region(&chans, rate, Some(&pair));
    let rec = record(&out, "exp.local_time_map");
    assert_eq!(rec.validity, Validity::Valid);
    match &rec.value {
        Value::Tensor { array, .. } => {
            assert_eq!(array, "features/expensive/in__r000/time_map");
        }
        _ => panic!("tensor reference expected"),
    }
    let t = out
        .tensors
        .iter()
        .find(|t| t.path == "features/expensive/in__r000/time_map")
        .expect("time-map tensor emitted");
    let rows = t.shape[0] as usize;
    assert!(rows > 0);
    let lags: Vec<f64> = (0..rows).map(|r| t.data[r * 3 + 1]).collect();
    let mean_lag = lags.iter().sum::<f64>() / rows as f64;
    assert!((mean_lag - 5.0).abs() < 2.0, "mean residual lag {mean_lag}");
}

#[test]
fn noise_classification_scores_known_signals() {
    let rate = 48_000u32;
    let n = 1 << 17;
    // Stationary broadband noise.
    let stationary = noise(n, 0x570a);
    let chans: Vec<&[f64]> = vec![&stationary];
    let out = run_region(&chans, rate, None);
    assert_eq!(val(record(&out, "exp.noise_class")), 0.0);
    assert!(val(record(&out, "exp.noise_stationary_score")) > 0.5);

    // Stable tones → tonal class.
    let tonal: Vec<f64> = (0..n)
        .map(|i| {
            (2.0 * std::f64::consts::PI * 440.0 * i as f64 / rate as f64).sin() * 0.3
                + (2.0 * std::f64::consts::PI * 880.0 * i as f64 / rate as f64).sin() * 0.2
        })
        .collect();
    let chans: Vec<&[f64]> = vec![&tonal];
    let out = run_region(&chans, rate, None);
    assert_eq!(val(record(&out, "exp.noise_class")), 1.0);
    assert!(val(record(&out, "exp.noise_tonal_score")) > 0.8);

    // Deep 2 Hz amplitude modulation → modulated class. The long
    // signal keeps the envelope spectrum's fundamental bin sharp.
    let n2 = 1 << 18;
    let base = noise(n2, 0x570a);
    let modulated: Vec<f64> = (0..n2)
        .map(|i| {
            let m = 0.1
                + 0.9
                    * (0.5
                        + 0.5 * (2.0 * std::f64::consts::PI * 2.0 * i as f64 / rate as f64).sin());
            base[i] * m
        })
        .collect();
    let chans: Vec<&[f64]> = vec![&modulated];
    let out = run_region(&chans, rate, None);
    assert_eq!(val(record(&out, "exp.noise_class")), 2.0);
    assert!(val(record(&out, "exp.noise_modulated_score")) > 0.6);
}

#[test]
fn compressor_transfer_curve_marks_flattened_top() {
    let rate = 48_000u32;
    let n = 1 << 16;
    // Hard limiter at ±0.4 on a ramping-envelope noise: the level
    // histogram's top collapses → slope_above_knee ≪ 1.
    let raw = noise(n, 0x1a1e);
    let sig: Vec<f64> = raw
        .iter()
        .enumerate()
        .map(|(i, &v)| (v * (0.2 + 0.8 * i as f64 / n as f64)).clamp(-0.4, 0.4))
        .collect();
    let chans: Vec<&[f64]> = vec![&sig];
    let out = run_region(&chans, rate, None);
    let slope = val(record(&out, "exp.transfer_slope_above_knee"));
    assert!(slope < 0.5, "limiter must flatten the top: {slope}");
    assert!(val(record(&out, "exp.transfer_knee_db")).is_finite());
}
