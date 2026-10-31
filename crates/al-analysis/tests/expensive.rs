//! ExpensiveAnalysisStage integration tests.
//!
//! Runs the real `CheapAnalysisStage` first so the candidate
//! extraction reads actual persisted features, then exercises every
//! trigger mode and the non-destructive-output contract.

use std::fs;
use std::path::{Path, PathBuf};

use al_core::{InputRole, JobId, StageKind};
use al_job::job::input;
use al_job::manifest::{ArtifactRecord, Manifest};
use al_job::registry::ArtifactRegistry;
use al_job::stage::{Stage, StageContext};

/// Unbiased deterministic noise (LCG → uniform over [-1, 1)).
fn noise(n: usize, seed: u64) -> Vec<f64> {
    let mut s = seed;
    (0..n)
        .map(|_| {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((s >> 11) as f64 / ((1u64 << 53) as f64)) * 2.0 - 1.0
        })
        .collect()
}

fn write_pcm(job_dir: &Path, rel: &str, samples: &[f64]) {
    let path = job_dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let bytes: Vec<u8> = samples.iter().flat_map(|v| v.to_le_bytes()).collect();
    fs::write(&path, bytes).unwrap();
}

struct Fixture {
    dir: PathBuf,
    manifest: Manifest,
    input_id: String,
}

/// One `capture_bug` input with mono 48 kHz decoded PCM.
fn fixture(capture: &[f64]) -> Fixture {
    let dir = std::env::temp_dir().join(format!("al-exp-test-{}", JobId::new()));
    fs::create_dir_all(&dir).unwrap();

    let mut cap_input = input(InputRole::CaptureBug, dir.join("cap.bin"));
    cap_input.source_sha256 = Some("cc".repeat(32));
    let input_id = cap_input.input_id.to_string();
    let declared_id = cap_input.input_id.clone();
    let mut manifest = Manifest::new(
        JobId::new(),
        vec![cap_input],
        serde_json::json!({}),
        &[
            StageKind::Ingest,
            StageKind::CheapAnalysis,
            StageKind::ExpensiveAnalysis,
        ],
    );

    write_pcm(&dir, "inputs/cap.f64le", capture);
    let declared = manifest.input_artifact(&declared_id).unwrap();
    manifest.register_artifact(ArtifactRecord {
        artifact_id: al_core::ArtifactId::new(),
        role: "decoded_pcm".into(),
        path: PathBuf::from("inputs/cap.f64le"),
        produced_by: Some(StageKind::Ingest),
        inputs: vec![declared],
        params: serde_json::json!({"sample_rate": 48_000, "channels": 1}),
    });

    Fixture {
        dir,
        manifest,
        input_id,
    }
}

fn ctx<'a>(
    fx: &'a Fixture,
    registry: &'a ArtifactRegistry,
    config: &'a serde_json::Value,
) -> StageContext<'a> {
    StageContext {
        manifest: &fx.manifest,
        registry,
        config,
    }
}

/// Cheap feature JSON for the input (byte content, for
/// non-destructive comparison).
fn cheap_json(fx: &Fixture) -> String {
    fs::read_to_string(
        fx.dir
            .join("analysis")
            .join("features")
            .join("cheap")
            .join(format!("{}.json", fx.input_id)),
    )
    .expect("cheap features persisted")
}

/// Kind strings (`Str` cells) of every event row.
fn event_kinds(fx: &Fixture) -> Vec<String> {
    let reader =
        al_dataset::reader::DatasetReader::open(&fx.dir.join("analysis")).expect("dataset opens");
    let table = reader
        .open_table("events", "candidates")
        .expect("events table");
    let n = table.row_count();
    if n == 0 {
        return Vec::new();
    }
    let cols = table.read_range(0, n, Some(&[1])).expect("kind column");
    cols[0]
        .iter()
        .filter_map(|c| match c {
            al_dataset::table::Cell::Str(s) => Some(s.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn clipping_candidate_triggers_expensive_without_overwriting_cheap() {
    // Hard-clipped sine: >99% of samples sit at |1.0|.
    let mut cap = noise(24_000, 7);
    for v in &mut cap {
        *v = v.signum();
    }
    let mut fx = fixture(&cap);
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({});

    let cheap_recs = al_analysis::stage::CheapAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("cheap stage");
    for rec in cheap_recs {
        fx.manifest.register_artifact(rec);
    }
    let cheap_before = cheap_json(&fx);

    let recs = al_analysis::stage::ExpensiveAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("expensive stage");

    // Candidate event persisted under the events role.
    let kinds = event_kinds(&fx);
    assert!(kinds.iter().any(|k| k == "clipping"), "events: {kinds:?}");
    assert_eq!(recs.first().unwrap().role, "candidate_events");
    let reader = al_dataset::reader::DatasetReader::open(&fx.dir.join("analysis")).unwrap();
    let candidate = reader.artifacts("events").into_iter().next().unwrap();
    assert!(
        candidate
            .provenance
            .parents
            .iter()
            .any(|p| p.contains("features/cheap/"))
    );
    // Expensive region was produced and records its trigger.
    assert!(
        recs.iter().any(|r| r.role == "expensive_features"),
        "artifacts: {recs:?}"
    );
    let exp_path = fx.dir.join("analysis").join("features").join("expensive");
    assert!(fs::read_dir(&exp_path).unwrap().next().is_some());
    // Cheap rows were not overwritten.
    assert_eq!(cheap_json(&fx), cheap_before);

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn suppressed_triggers_write_events_but_no_expensive() {
    // A quiet tonal sine: its only threshold hit is the noise-floor
    // heuristic (-23 dB > -40), which the config overlay raises out of
    // range — exercising the suppress path.
    let cap: Vec<f64> = (0..16_000)
        .map(|i| 0.1 * (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 48_000.0).sin())
        .collect();
    let mut fx = fixture(&cap);
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({
        "analysis": {"events": {"noise_floor_dbfs": -10.0}}
    });

    let cheap_recs = al_analysis::stage::CheapAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("cheap stage");
    for rec in cheap_recs {
        fx.manifest.register_artifact(rec);
    }

    let recs = al_analysis::stage::ExpensiveAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("expensive stage");

    // The events table exists (role produced) even with no hits;
    // expensive_features are suppressed.
    let _ = event_kinds(&fx);
    assert_eq!(recs.first().unwrap().role, "candidate_events");
    assert!(
        !recs.iter().any(|r| r.role == "expensive_features"),
        "unexpected expensive artifacts: {recs:?}"
    );

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn manual_ranges_analyze_without_any_candidate() {
    // No candidates expected from a quiet sine — the manual range
    // alone must select the region and stamp `manual` as trigger.
    let cap: Vec<f64> = (0..24_000)
        .map(|i| 0.1 * (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 48_000.0).sin())
        .collect();
    let mut fx = fixture(&cap);
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({
        "analysis": {
            "events": {"noise_floor_dbfs": -10.0},
            "expensive": {"manual_ranges": [[0.0, 0.25]]}
        }
    });

    let cheap_recs = al_analysis::stage::CheapAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("cheap stage");
    for rec in cheap_recs {
        fx.manifest.register_artifact(rec);
    }

    let recs = al_analysis::stage::ExpensiveAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("expensive stage");

    let exp: Vec<_> = recs
        .iter()
        .filter(|r| r.role == "expensive_features")
        .collect();
    assert_eq!(exp.len(), 1, "artifacts: {recs:?}");
    assert!(
        exp[0].params["triggers"]
            .as_array()
            .is_some_and(|t| t.iter().any(|v| v == "manual")),
        "triggers: {}",
        exp[0].params
    );

    fs::remove_dir_all(&fx.dir).ok();
}

/// Two-input fixture: `source_asset` + `capture_bug` decoded PCM.
struct PairFixture {
    dir: PathBuf,
    manifest: Manifest,
    capture_input_id: String,
}

fn pair_fixture(source: &[f64], capture: &[f64]) -> PairFixture {
    let dir = std::env::temp_dir().join(format!("al-exp-pair-{}", JobId::new()));
    fs::create_dir_all(&dir).unwrap();

    let mut src_input = input(InputRole::SourceAsset, dir.join("src.bin"));
    let mut cap_input = input(InputRole::CaptureBug, dir.join("cap.bin"));
    src_input.source_sha256 = Some("aa".repeat(32));
    cap_input.source_sha256 = Some("bb".repeat(32));
    let capture_input_id = cap_input.input_id.to_string();
    let input_ids = [src_input.input_id.clone(), cap_input.input_id.clone()];
    let manifest = Manifest::new(
        JobId::new(),
        vec![src_input, cap_input],
        serde_json::json!({}),
        &[
            StageKind::Ingest,
            StageKind::Locate,
            StageKind::CheapAnalysis,
            StageKind::ExpensiveAnalysis,
        ],
    );
    let mut manifest = manifest;
    for (idx, (rel, samples)) in [("inputs/src.f64le", source), ("inputs/cap.f64le", capture)]
        .into_iter()
        .enumerate()
    {
        write_pcm(&dir, rel, samples);
        let declared = manifest.input_artifact(&input_ids[idx]).unwrap();
        manifest.register_artifact(ArtifactRecord {
            artifact_id: al_core::ArtifactId::new(),
            role: "decoded_pcm".into(),
            path: PathBuf::from(rel),
            produced_by: Some(StageKind::Ingest),
            inputs: vec![declared],
            params: serde_json::json!({"sample_rate": 48_000, "channels": 1}),
        });
    }
    PairFixture {
        dir,
        manifest,
        capture_input_id,
    }
}

fn pctx<'a>(
    fx: &'a PairFixture,
    registry: &'a ArtifactRegistry,
    config: &'a serde_json::Value,
) -> StageContext<'a> {
    StageContext {
        manifest: &fx.manifest,
        registry,
        config,
    }
}

#[test]
fn pair_evidence_populates_pair_dependent_families() {
    // capture = 0.5× source, aligned at sample 0 (identity
    // alignment from the real locate stage). The locate artifact is present
    // but no reference stage ran, so `transfer_h` is absent: IR
    // stays `pair evidence unavailable` while the level-transfer,
    // mixing-matrix and time-map families measure.
    let n = 32_000usize;
    let source: Vec<f64> = noise(n, 0x51)
        .iter()
        .enumerate()
        .map(|(i, &v)| v * (0.1 + 0.9 * i as f64 / n as f64))
        .collect();
    let capture: Vec<f64> = source.iter().map(|&v| 0.5 * v).collect();
    let mut fx = pair_fixture(&source, &capture);
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({
        "locate": {"consensus": {"min_peak_to_background": 0.5}},
        "analysis": {
            "events": {"noise_floor_dbfs": 1e9, "zero_run_samples": 1e9, "stuck_run_samples": 1e9},
            "expensive": {"always_run": true}
        }
    });

    for rec in al_locate::stage::LocateStage::new()
        .run(&pctx(&fx, &registry, &config))
        .unwrap()
    {
        fx.manifest.register_artifact(rec);
    }

    let alignment_id = fx
        .manifest
        .artifacts
        .iter()
        .find(|r| r.role == "locate_result")
        .unwrap()
        .params["alignment_id"]
        .as_str()
        .unwrap()
        .to_string();
    for rec in al_analysis::stage::CheapAnalysisStage::new()
        .run(&pctx(&fx, &registry, &config))
        .expect("cheap stage")
    {
        fx.manifest.register_artifact(rec);
    }
    let recs = al_analysis::stage::ExpensiveAnalysisStage::new()
        .run(&pctx(&fx, &registry, &config))
        .expect("expensive stage");

    // The capture's region carries pair-derived records stamped
    // with the locate alignment id.
    let exp = recs
        .iter()
        .find(|r| {
            r.role == "expensive_features"
                && r.path.to_string_lossy().contains(&*fx.capture_input_id)
        })
        .expect("capture region produced");
    let body: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(fx.dir.join(&exp.path)).unwrap()).unwrap();
    let records = body["records"].as_array().unwrap();
    let find = |id: &str| {
        records
            .iter()
            .find(|r| r["feature_id"] == id)
            .unwrap_or_else(|| panic!("missing {id} in {records:?}"))
    };
    // Pair-dependent families measured against the accepted pair.
    for id in [
        "exp.level_transfer_slope",
        "exp.mix_polarity",
        "exp.local_time_map",
    ] {
        let r = find(id);
        assert_eq!(
            r["validity"].as_str(),
            Some("valid"),
            "{id} must be measured: {r}"
        );
        assert_eq!(
            r["alignment_id"].as_str(),
            Some(alignment_id.as_str()),
            "{id} must reference the locate alignment"
        );
        assert!(r["channels"].is_null(), "{id} is a pair summary: {r}");
    }
    // The mixing matrix fit is near-exact for a pure scalar mix.
    assert_eq!(find("exp.mix_polarity")["value"]["scalar"], 1.0);
    assert!(
        find("exp.mix_fit_quality")["value"]["scalar"]
            .as_f64()
            .unwrap()
            < 0.05
    );
    // The local time map is a tensor reference; the tensor landed
    // under features/expensive.
    assert!(
        find("exp.local_time_map")["value"]["tensor"]["array"]
            .as_str()
            .is_some_and(|p| p.contains("time_map"))
    );
    // IR is pair-dependent on the reference `transfer_h` artifact —
    // locate alone is not enough, so it stays invalid with the
    // pair-evidence reason.
    let ir = find("exp.ir_early_late_ratio");
    assert!(ir["channels"].is_null());
    assert_ne!(ir["validity"].as_str(), Some("valid"));
    assert_eq!(
        ir["invalid_reason"].as_str(),
        Some("pair evidence unavailable")
    );

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn non_valid_candidate_region_records_invalid_observations() {
    // Digital silence: `level.integrated_dbfs` (-240 dBFS) trips the
    // silence rule with `SilenceInsufficientEnergy` — the only
    // candidate is non-`Valid`, so the region must persist invalid
    // observations (null scalars) rather than computed numbers. The
    // zero/stuck-run and noise-floor rules are raised out of range
    // so no `Valid` trigger fires alongside.
    let cap = vec![0.0f64; 24_000];
    let mut fx = fixture(&cap);
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({
        "analysis": {"events": {
            "zero_run_samples": 1e9,
            "stuck_run_samples": 1e9,
            "noise_floor_dbfs": 1e9
        }}
    });

    let cheap_recs = al_analysis::stage::CheapAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("cheap stage");
    for rec in cheap_recs {
        fx.manifest.register_artifact(rec);
    }

    let recs = al_analysis::stage::ExpensiveAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("expensive stage");

    let exp = recs
        .iter()
        .find(|r| r.role == "expensive_features")
        .expect("non-valid candidate still produces a region");
    let body: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(fx.dir.join(&exp.path)).expect("expensive json"))
            .unwrap();
    let records = body["records"].as_array().unwrap();
    assert!(!records.is_empty());
    for r in records {
        assert_ne!(
            r["validity"].as_str().unwrap(),
            "valid",
            "{} claimed Valid",
            r["feature_id"]
        );
        assert!(
            r["value"]["scalar"].is_null(),
            "{} emitted a finite number: {}",
            r["feature_id"],
            r["value"]
        );
        assert!(r["invalid_reason"].is_string());
    }

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn always_run_analyzes_full_input_without_reference_stage() {
    // No locate/reference stages ran: cheap-only candidates must
    // still drive the stage.
    let cap = noise(20_000, 13);
    let mut fx = fixture(&cap);
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({
        "analysis": {"expensive": {"always_run": true}}
    });

    let cheap_recs = al_analysis::stage::CheapAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("cheap stage");
    for rec in cheap_recs {
        fx.manifest.register_artifact(rec);
    }

    let recs = al_analysis::stage::ExpensiveAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("expensive stage");

    let exp = recs
        .iter()
        .find(|r| r.role == "expensive_features")
        .expect("always_run produced expensive output");
    assert!(
        exp.params["triggers"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("always_run"))
    );
    // Full-input region: a broad set of family records exists.
    let name = exp.path.file_stem().unwrap().to_str().unwrap().to_string();
    let body: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(fx.dir.join(&exp.path)).expect("expensive json"))
            .unwrap();
    let _ = name;
    let ids: Vec<&str> = body["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r["feature_id"].as_str())
        .collect();
    for family in [
        "exp.cutoff_hz",
        "exp.echo_delay_s",
        "exp.src_image_score",
        "exp.polarity_asymmetry",
        "exp.repeat_block_score",
        "exp.spectral_hole_ratio",
        "exp.tonal_peak_count",
    ] {
        assert!(ids.contains(&family), "missing {family} in {ids:?}");
    }

    fs::remove_dir_all(&fx.dir).ok();
}

// ==== mechanical-caller-cleanup characterization ====
// TASK-034EO484s25JM8bsb6zUCh (plan T1). Pins the CURRENT outputs
// of the expensive-stage sites the mechanical caller cleanup bundle
// touches — A7 (expensive/channel.rs per-band `csd` accumulation)
// and A16a (expensive/mod.rs column-scan `mean_power`) — via the
// downstream records' `f64::to_bits`. Oracles re-implement the
// current formulas over public API (`stft` / `csd` /
// `complex_coherence` stay public); no tolerance predicate.
mod mechanical_cleanup {
    use super::*;
    use al_analysis::expensive::{
        ExpensiveConfig, ExpensiveResult, TriggeredRegion, analyze_region,
    };
    use al_analysis::stft::{csd, stft, third_octave_bands};
    use al_core::{Confidence, Validity};
    use al_dataset::types::{FeatureRecord, Value};

    fn scalar(r: &FeatureRecord) -> f64 {
        match &r.value {
            Value::Scalar(v) => *v,
            other => panic!("expected scalar, got {other:?}"),
        }
    }

    fn record<'a>(out: &'a ExpensiveResult, id: &str) -> &'a FeatureRecord {
        out.records
            .iter()
            .find(|r| r.feature_id == id)
            .unwrap_or_else(|| panic!("missing record {id}"))
    }

    const RATE: u32 = 48_000;
    const WINDOW: usize = 1024;
    const HOP: usize = 512;

    fn run_region(chans: &[&[f64]]) -> ExpensiveResult {
        let region = TriggeredRegion {
            start_s: 0.0,
            end_s: chans[0].len() as f64 / RATE as f64,
            triggers: vec!["always_run".into()],
            validity: Validity::Valid,
            confidence: Confidence::High,
            has_valid_trigger: true,
        };
        analyze_region(
            "in",
            "in__r000",
            chans,
            RATE,
            &region,
            None,
            None,
            &ExpensiveConfig {
                stft_window: WINDOW,
                stft_hop: HOP,
                ..Default::default()
            },
        )
    }

    /// Column-scan `mean_power` — the current
    /// `expensive/mod.rs` accumulation shape — plus its derived
    /// `total_power` / `one_sided_power` sums.
    fn mean_power_oracle(seg: &[f64]) -> (Vec<f64>, f64, f64, usize, f64) {
        let m = stft(seg, WINDOW, HOP, RATE);
        let bins = m.fft_size;
        let half = bins / 2;
        let frames = m.frames();
        let power = m.power();
        let mean: Vec<f64> = (0..bins)
            .map(|k| power.iter().map(|f| f[k]).sum::<f64>() / frames as f64)
            .collect();
        let bin_hz = RATE as f64 / bins as f64;
        (
            mean.clone(),
            mean.iter().sum(),
            mean[..half].iter().sum(),
            frames,
            bin_hz,
        )
    }

    // ---- A7: `exp.freq_lr_coherence` — the sxy accumulation over
    //      `band_bins` (expensive/channel.rs site) ----

    #[test]
    fn a7_freq_lr_coherence_is_bit_exact() {
        let primary = noise(16_384, 29);
        let secondary: Vec<f64> = primary
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                if i >= 7 {
                    0.8 * primary[i - 7]
                } else {
                    v * 0.2
                }
            })
            .collect();
        let chans: Vec<&[f64]> = vec![&primary, &secondary];
        let out = run_region(&chans);

        // Oracle: the channel.rs sxy path — `csd` over all bins,
        // accumulated only inside `band_bins`.
        let seg = &primary[..];
        let oseg = &secondary[..];
        let n_ch = seg.len().min(oseg.len());
        let mb = stft(&seg[..n_ch], WINDOW, HOP, RATE);
        let ob = stft(&oseg[..n_ch], WINDOW, HOP, RATE);
        let bins = mb.fft_size;
        let half = bins / 2;
        let bin_hz = RATE as f64 / bins as f64;
        let fr = mb.frames().min(ob.frames());
        assert!(fr >= 2, "fixture must yield >= 2 frames");
        let bands = third_octave_bands(100.0, RATE as f64 / 2.0 * 0.9);
        let band_bins: Vec<usize> = bands
            .iter()
            .map(|&(lo, hi)| ((lo / bin_hz) as usize, (hi / bin_hz) as usize))
            .flat_map(|(a, b)| a..b.min(half))
            .collect();
        let mut sxy = vec![(0.0f64, 0.0f64); half];
        let mut sxx = vec![0.0f64; half];
        let mut syy = vec![0.0f64; half];
        for f in 0..fr {
            let (cr, ci) = csd(&mb.re[f], &mb.im[f], &ob.re[f], &ob.im[f]);
            for &k in &band_bins {
                let aa = mb.re[f][k].powi(2) + mb.im[f][k].powi(2);
                let bb = ob.re[f][k].powi(2) + ob.im[f][k].powi(2);
                sxy[k].0 += cr[k];
                sxy[k].1 += ci[k];
                sxx[k] += aa;
                syy[k] += bb;
            }
        }
        let mut coh_sum = 0.0;
        let mut coh_n = 0u64;
        for &k in &band_bins {
            let den = sxx[k] * syy[k];
            if den > 0.0 {
                let num = sxy[k].0 * sxy[k].0 + sxy[k].1 * sxy[k].1;
                coh_sum += (num / den).sqrt();
                coh_n += 1;
            }
        }
        assert!(coh_n > 0, "fixture must leave supported band bins");
        let r = record(&out, "exp.freq_lr_coherence");
        assert_eq!(r.validity, Validity::Valid, "{r:?}");
        assert_eq!(
            scalar(r).to_bits(),
            (coh_sum / coh_n as f64).to_bits(),
            "exp.freq_lr_coherence"
        );
    }

    // ---- A16a: `mean_power` / `total_power` / `one_sided_power`
    //      consumers (exp.cutoff_hz / exp.thd_ratio /
    //      exp.spectral_hole_ratio) ----

    #[test]
    fn a16a_mean_power_consumers_are_bit_exact() {
        // Broadband-ish input with a strong tone so all three
        // records land on their Valid paths.
        let n = 16_384usize;
        let mut sig = noise(n, 0xabc);
        for (i, v) in sig.iter_mut().enumerate() {
            *v += 0.6 * (2.0 * std::f64::consts::PI * 3000.0 * i as f64 / RATE as f64).sin();
        }
        let chans: Vec<&[f64]> = vec![&sig];
        let out = run_region(&chans);

        let (mean, total_power, one_sided_power, _frames, bin_hz) = mean_power_oracle(&sig[..]);
        let m = stft(&sig[..], WINDOW, HOP, RATE);
        let half = m.fft_size / 2;
        assert!(total_power > 0.0 && one_sided_power > 0.0);

        // exp.cutoff_hz oracle (transfer.rs): first bin from the top
        // whose cumulative-above energy reaches 1% of one-sided.
        let mut above = 0.0;
        let mut cutoff = 0usize;
        for k in (0..half).rev() {
            above += mean[k];
            if above / one_sided_power >= 0.01 {
                cutoff = k;
                break;
            }
        }
        let r = record(&out, "exp.cutoff_hz");
        assert_eq!(
            scalar(r).to_bits(),
            (cutoff as f64 * bin_hz).to_bits(),
            "exp.cutoff_hz"
        );

        // exp.thd_ratio oracle: strongest bin's harmonic share.
        let (f0, _) = mean[4..half.min(2048)]
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, v)| (i + 4, *v))
            .unwrap_or((0, 0.0));
        let mut harm = 0.0;
        for h in 2..=8usize {
            let k = f0 * h;
            if k >= half {
                break;
            }
            harm += mean[k.saturating_sub(1)..=(k + 1).min(half - 1)]
                .iter()
                .sum::<f64>();
        }
        let r = record(&out, "exp.thd_ratio");
        assert_eq!(
            scalar(r).to_bits(),
            (harm / total_power).to_bits(),
            "exp.thd_ratio"
        );

        // exp.spectral_hole_ratio oracle (codec.rs): bins in the top
        // half below 1e-4 of the one-sided max.
        let maxp = mean[..half].iter().copied().fold(0.0, f64::max);
        let floor = maxp * 1e-4;
        let top = &mean[half / 2..half];
        let holes = top.iter().filter(|&&p| p < floor).count();
        let r = record(&out, "exp.spectral_hole_ratio");
        assert_eq!(
            scalar(r).to_bits(),
            (holes as f64 / top.len().max(1) as f64).to_bits(),
            "exp.spectral_hole_ratio"
        );
    }
}
