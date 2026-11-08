//! AL-G3: view model の写像検証 — synthetic JSON の単体試験と、
//! 実 dataset(ingest→dataset stage)を経る e2e fixture。

use std::sync::Arc;

use al_core::{Confidence, StageKind, StageStatus, Validity};
use al_gui::view::analysis::{AlignmentRow, EventRow, FeatureTable};
use al_gui::view::dataset::TimelineView;
use al_gui::view::job::{JobRow, RunOutcome};
use al_gui::view::run_screen::{STAGES_KEY, job_list, run_screen};
use al_gui::view::source::{SpectrumView, StreamTimeView, TimeMap, WaveformView};
use al_locate::{LocateResult, SampleRange};
use al_media::{ClockKind, StreamTiming, TimestampOrigin};
use al_service::dataset::DatasetSession;
use al_service::jobs::{JobService, RunControl, RunSpec};
use al_service::parse_input_spec;
use al_testkit::media::wav_s16_bytes;
use al_testkit::temp::TempDir;
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{AttachEpochAllocator, CellValue, ComponentKey, ComponentSpec, ViewHost};
use serde_json::json;

// ---------- unit: synthetic rows ----------

#[test]
fn event_row_maps_typed_enums() {
    let row = json!({
        "event_id": "ev-1",
        "kind": "level_anomaly",
        "input_id": "cap",
        "start_s": 0.25,
        "end_s": 0.75,
        "strength": 0.9,
        "confidence": "high",
        "validity": "valid",
        "source_feature": "cheap.rms",
        "freq_lo_hz": 100.0,
        "freq_hi_hz": 8000.0,
        "channels": "all",
        "algorithm_id": "cheap.v1",
        "algorithm_version": "1.0.0",
        "evidence": "{}",
        "params": "{}"
    });
    let e = EventRow::from_json(&row).unwrap();
    assert_eq!(e.confidence, Confidence::High);
    assert_eq!(e.validity, Validity::Valid);
    assert_eq!(e.start_s, 0.25);
    assert_eq!(e.end_s, Some(0.75));
    assert_eq!(e.frequency_range_hz, Some((100.0, 8000.0)));
}

#[test]
fn event_row_open_ended_end() {
    let row = json!({
        "event_id": "ev-2", "kind": "silence", "input_id": "cap",
        "start_s": 1.0, "end_s": null, "strength": 0.0,
        "confidence": "low", "validity": "insufficient_support",
        "source_feature": "cheap.rms", "freq_lo_hz": null, "freq_hi_hz": null,
        "channels": "all", "algorithm_id": "cheap.v1",
        "algorithm_version": "1", "evidence": "{}", "params": "{}"
    });
    let e = EventRow::from_json(&row).unwrap();
    assert_eq!(e.end_s, None);
    assert_eq!(e.validity, Validity::InsufficientSupport);
    assert_eq!(e.frequency_range_hz, None);
}

#[test]
fn alignment_row_preserves_typed_contracts() {
    let row = json!({
        "source_input_id": "src",
        "capture_input_id": "cap",
        "alignment_id": "al-1",
        "status": "accepted",
        "validity": "valid",
        "confidence": "high",
        "agreeing_methods": 2,
        "capture_start": 960,
        "capture_end": 4800
    });
    let a = AlignmentRow::from_json(&row).unwrap();
    assert!(a.accepted);
    assert_eq!(
        a.capture_range,
        SampleRange {
            start: 960,
            end: 4800
        }
    );
    assert_eq!(a.validity, Validity::Valid);
    assert_eq!(a.agreeing_methods, 2);
}

#[test]
fn feature_table_typed_records() {
    // FeatureRecord スキーマ(_artifact 拡張 field は serde が無視)。
    let record = json!({
        "feature_id": "cheap.rms",
        "value": {"scalar": 0.5},
        "unit": "ratio",
        "resolution_class": "segment",
        "confidence": "high",
        "validity": "valid",
        "support_count": 4,
        "algorithm_id": "cheap.v1",
        "algorithm_version": "1",
        "params": {},
        "input_id": "cap",
        "lineage": [],
        "_artifact": "features/cheap/cheap.rms"
    });
    let t = FeatureTable::from_json(&json!({"records": [record], "truncated": true})).unwrap();
    assert!(t.truncated);
    assert_eq!(t.records.len(), 1);
    assert_eq!(t.records[0].feature_id, "cheap.rms");
    assert_eq!(t.records[0].confidence, Confidence::High);
}

#[test]
fn time_map_offset_rate_mapping() {
    let result = LocateResult {
        alignment_id: al_core::AlignmentId::new(),
        source_input_id: "src".into(),
        capture_input_id: "cap".into(),
        source_range: SampleRange {
            start: 0,
            end: 4800,
        },
        capture_range: SampleRange {
            start: 960,
            end: 5760,
        },
        normalized_source_range: SampleRange {
            start: 0,
            end: 4800,
        },
        source_pcm: None,
        capture_pcm: None,
        evidence: vec![],
        agreeing_methods: 1,
        position_spread: 0,
        confidence: Confidence::High,
        validity: Validity::Valid,
        reject_reason: None,
        accepted: true,
        algorithm_id: "locate".into(),
        algorithm_version: "1".into(),
        params: json!({}),
    };
    let tm = TimeMap::from_result(&result, 48_000, 48_000);
    // capture 960 → source 0 → 0.0s; capture 5760 → source 4800 → 0.1s。
    assert_eq!(tm.capture_sample_to_source_seconds(960), 0.0);
    assert!((tm.capture_sample_to_source_seconds(5760) - 0.1).abs() < 1e-9);
}

#[test]
fn stream_time_view_bounded_ticks() {
    let timing: StreamTiming = serde_json::from_value(json!({
        "stream": 0,
        "stream_type": "auds",
        "timestamp_origin": "movi_order",
        "start_tick": 0,
        "stream_timestamps": [0, 480, 960, 1440, 1920],
        "observation": {
            "clock": "audio",
            "timebase_hz": 48000,
            "timestamp_count": 5,
            "intervals": [480, 480, 480, 480],
            "interval_stats": null,
            "drift_ppm": null,
            "discontinuities": [],
            "duplicates": [],
            "drop_candidates": []
        }
    }))
    .unwrap();
    let v = StreamTimeView::new(timing);
    assert_eq!(v.stream_type(), "auds");
    assert_eq!(v.timebase_hz(), 48_000);
    assert_eq!(v.len(), 5);
    assert_eq!(v.ticks(1, 3), &[480, 960]);
    assert_eq!(v.seconds_at(1), Some(0.01));
    // bounded: end は範囲に clamp される。
    assert_eq!(v.ticks(4, 100), &[1920]);
    // 範囲外/逆順/空 stream でも panic せず clamp・空 slice。
    assert_eq!(v.ticks(2, 1), &[] as &[i64]);
    assert_eq!(v.ticks(5, 6), &[] as &[i64]);
    assert_eq!(v.ticks(0, usize::MAX), v.ticks(0, v.len()));

    // 空 stream: start > len の `ticks(1, 2)` で slice panic
    // しない(start も len へ clamp)。
    let empty: StreamTiming = serde_json::from_value(json!({
        "stream": 0,
        "stream_type": "auds",
        "timestamp_origin": "movi_order",
        "start_tick": 0,
        "stream_timestamps": [],
        "observation": {
            "clock": "audio",
            "timebase_hz": 48000,
            "timestamp_count": 0,
            "intervals": [],
            "interval_stats": null,
            "drift_ppm": null,
            "discontinuities": [],
            "duplicates": [],
            "drop_candidates": []
        }
    }))
    .unwrap();
    let v = StreamTimeView::new(empty);
    assert!(v.is_empty());
    assert_eq!(v.ticks(1, 2), &[] as &[i64]);
    assert_eq!(v.seconds_at(0), None);
}

#[test]
fn stream_time_view_from_metadata_array() {
    // `av_timing` は stream 毎の `Vec<StreamTiming>` で永続化される
    // (al-media stage.rs:242)。単一 object 形は producer に存在しない。
    let timing = json!({
        "stream": 1, "stream_type": "vids",
        "timestamp_origin": "sample_table", "start_tick": 0,
        "stream_timestamps": [0],
        "observation": {
            "clock": "video", "timebase_hz": 30,
            "timestamp_count": 1, "intervals": [],
            "interval_stats": null, "drift_ppm": null,
            "discontinuities": [], "duplicates": [], "drop_candidates": []
        }
    });
    let meta = json!({"av_timing": [timing]});
    let views = StreamTimeView::from_metadata(&meta).unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].timestamp_origin(), TimestampOrigin::SampleTable);
    assert_eq!(views[0].stream(), 1);
    // origin/observation が typed で露出する。
    assert_eq!(views[0].observation().clock, ClockKind::Video);
    assert_eq!(views[0].observation().timebase_hz, 30);
    // WAV の空配列・key 欠落の両端。
    assert!(
        StreamTimeView::from_metadata(&json!({"av_timing": []}))
            .unwrap()
            .is_empty()
    );
    assert!(StreamTimeView::from_metadata(&json!({})).is_err());
    // 単一 object(旧の捏造 shape)は拒否される。
    assert!(StreamTimeView::from_metadata(&json!({"av_timing": {"stream": 0}})).is_err());
}

#[test]
fn waveform_window_clamps_saturating_seconds_to_sample() {
    // `seconds_to_sample` は巨大な秒入力を u64::MAX へ saturate する。
    // start を frames へ clamp しないと `start + 1` が overflow panic
    // する(ticks() と同じ両端 clamp の sibling)。
    let source = Arc::new(ruviform_viz::SliceWaveform::new(48_000, 1, vec![0.0; 4800]).unwrap());
    let v = WaveformView::try_from_source(source).unwrap();
    // 3.9e14 s × 48kHz ≫ u64::MAX — panic せず end 側も frames へ収まる。
    let bins = v.window(0, 1e15, 1e15, 64).unwrap();
    assert!(bins.len() <= 64);
    // 有限範囲は従来どおり読める。
    assert!(v.window(0, 0.0, 0.05, 64).unwrap()[0].end > 0);
}

#[test]
fn run_outcome_maps_stage_rows() {
    let v = json!({
        "job_id": "job-1",
        "stages": [
            {"stage": "ingest", "status": "completed"},
            {"stage": "locate", "status": "failed", "error": "x"}
        ]
    });
    let r = RunOutcome::from_json(&v).unwrap();
    assert_eq!(r.stages.len(), 2);
    assert_eq!(r.stages[0].kind, StageKind::Ingest);
    assert_eq!(r.stages[0].status, StageStatus::Completed);
    assert_eq!(r.stages[1].status, StageStatus::Failed);
}

// ---------- e2e: real dataset fixture ----------

/// f64 [-1,1] 信号を i16 PCM へ(testkit と同じ scaling)。
fn to_i16(samples: &[f64]) -> Vec<i16> {
    samples
        .iter()
        .map(|v| (v.clamp(-1.0, 1.0) * 32767.0).round() as i16)
        .collect()
}

#[test]
fn real_dataset_produces_view_models() {
    // source を capture へ offset 9600・gain 1.0 で埋め込む
    // (al-locate golden の受理済み fixture 規約)。
    let dir = TempDir::new("al-gui-view-e2e");
    let source_f64 = al_testkit::signal::noise(4_096, 7, 1.0);
    let mut capture_f64 = al_testkit::signal::noise(24_576, 11, 0.02);
    al_testkit::signal::embed(&mut capture_f64, &source_f64, 9_600, 1.0);
    let src_wav = dir.join("src.wav");
    let cap_wav = dir.join("cap.wav");
    std::fs::write(&src_wav, wav_s16_bytes(1, 48_000, &to_i16(&source_f64))).unwrap();
    std::fs::write(&cap_wav, wav_s16_bytes(1, 48_000, &to_i16(&capture_f64))).unwrap();

    let service = JobService::new(dir.join("jobs"));
    let spec = RunSpec {
        inputs: vec![
            parse_input_spec(&format!("source_asset:{}", src_wav.display())).unwrap(),
            parse_input_spec(&format!("capture_bug:{}", cap_wav.display())).unwrap(),
        ],
        stages: Some(vec![
            StageKind::Ingest,
            StageKind::Locate,
            StageKind::CheapAnalysis,
            StageKind::Dataset,
        ]),
        config: json!({}),
    };
    let (job_id, manifest) = service.run(spec, &RunControl::default()).unwrap();
    assert!(
        manifest
            .stages
            .iter()
            .all(|s| s.status == StageStatus::Completed),
        "all stages completed: {:?}",
        manifest.stages
    );

    // JobRow / JobDetail。
    let summary = service.list().unwrap().remove(0);
    let row = JobRow::from_summary(&summary);
    assert_eq!(row.job_id, job_id.as_str());
    assert!(row.dataset_ready);
    assert!(
        row.stages
            .iter()
            .all(|s| s.status == StageStatus::Completed)
    );
    assert_eq!(row.inputs.len(), 2);
    assert_eq!(row.inputs[0].role, "source_asset");
    assert_eq!(row.inputs[1].role, "capture_bug");

    let detail = al_gui::view::job::JobDetail::from_inspection(&service.inspect(&job_id).unwrap());
    assert!(detail.dataset_ready);

    // Job 一覧と Run 画面の射影。Run 結果は `al.job.run` の stage 表示 JSON と
    // 同じ `{stage,status,duration_ms,error}` 形で写し、実 manifest と照合する。
    let jobs = job_list(std::slice::from_ref(&row)).unwrap();
    let ComponentSpec::Table(jobs) = &jobs else {
        panic!("job list must be a table");
    };
    assert_eq!(jobs.rows.len(), 1);
    assert_eq!(
        jobs.rows[0][0],
        Some(CellValue::Text(job_id.as_str().to_string()))
    );

    let stages: Vec<_> = manifest
        .stages
        .iter()
        .map(|r| json!({"stage": r.stage, "status": r.status, "duration_ms": r.duration_ms, "error": r.error}))
        .collect();
    let run = RunOutcome::from_json(&json!({"job_id": job_id.as_str(), "stages": stages})).unwrap();
    let view = run_screen(&run, Vec::new()).unwrap();
    let ComponentSpec::Panel(panel) = &view.root else {
        panic!("run screen must be a panel");
    };
    let ComponentSpec::Label(status) = &panel.children[0] else {
        panic!("run status must be a label");
    };
    let n = manifest.stages.len();
    assert_eq!(
        status.text,
        format!("job {}: completed {n}/{n}, failed 0", job_id.as_str())
    );
    let ComponentSpec::Table(table) = &panel.children[1] else {
        panic!("stage table");
    };
    assert_eq!(table.rows.len(), n);
    assert!(
        table
            .rows
            .iter()
            .all(|r| r[1] == Some(CellValue::Text("completed".to_string())))
    );
    let mut host = ViewHost::new(
        ViewInstanceId::new(1).unwrap(),
        AttachEpochAllocator::new().mint(),
    );
    host.mount(view).unwrap();
    assert!(
        host.observe(&ComponentKey::from(STAGES_KEY))
            .unwrap()
            .mounted
    );

    // Dataset session → bounded view models。
    let session = DatasetSession::open(service.dataset_dir(&job_id)).unwrap();
    let cap_id = row.inputs[1].input_id.clone();
    let view = WaveformView::open(&session, &cap_id).unwrap();
    assert_eq!(view.sample_rate, 48_000);
    let bins = view.window(0, 0.05, 0.15, 64).unwrap();
    assert!(bins.len() <= 64);
    assert!(!bins.is_empty());

    // CheapAnalysis が書く `intermediate/<input>/ch<N>/stft_*` →
    // bounded tile。
    let spectrum = SpectrumView::open(&session, &cap_id, 0).unwrap();
    assert!(spectrum.info.bins > 0);
    let tile = spectrum
        .tile(&ruviform_viz::TileRequest {
            frames: 0..16,
            rows: ruviform_viz::RowSpec::uniform(spectrum.info.bins, 8),
            cols: 16,
            aggregate: ruviform_viz::Aggregate::Max,
        })
        .unwrap();
    assert!(tile.cols > 0);
    assert!(tile.rows > 0);

    // timeline → typed rows。
    let timeline = TimelineView::from_json(&session.timeline().unwrap()).unwrap();
    assert_eq!(timeline.inputs.len(), 2);
    assert!(timeline.inputs.iter().all(|i| i.frames > 0));

    // locate → 実 AlignmentRow + detail → real TimeMap。
    let locate = session.locate().unwrap();
    let alignments = locate["alignments"].as_array().unwrap();
    assert_eq!(alignments.len(), 1, "one source×capture pair");
    let alignment = AlignmentRow::from_json(&alignments[0]).unwrap();
    assert_eq!(alignment.source_input_id, row.inputs[0].input_id);
    assert_eq!(alignment.capture_input_id, cap_id);
    assert!(
        alignment.accepted,
        "embedded source should be accepted: {alignments:?}"
    );
    let detail_json = session
        .locate_detail(&row.inputs[0].input_id, &cap_id)
        .unwrap();
    let result: LocateResult = serde_json::from_value(detail_json["result"].clone())
        .expect("locate detail carries a LocateResult");
    let tm = TimeMap::from_result(&result, 48_000, 48_000);
    // capture 9600 ≒ source 0 → 0.0s 近傍(alignment 精度の範囲)。
    let mapped = tm.capture_sample_to_source_seconds(result.capture_range.start);
    assert!(
        mapped.abs() < 0.05,
        "aligned position maps near 0: {mapped}"
    );

    // features → typed FeatureRecord。
    let feats = session
        .features(&al_service::dataset::FeatureQuery {
            roles: None,
            name_prefix: None,
            feature_id_prefix: None,
            limit: Some(10),
        })
        .unwrap();
    let table = FeatureTable::from_json(&feats).unwrap();
    assert!(
        !table.records.is_empty(),
        "dataset features should be non-empty"
    );
    assert!(table.records.iter().all(|r| !r.feature_id.is_empty()));

    // Source・Spectrogram・Analysis を Tabs 画面へ束ね、Fake 層へ mount する。
    let workspace = al_gui::view::workspace::workspace_screen(&session, &cap_id).unwrap();
    let ruviform_ui::view::ComponentSpec::Tabs(tabs) = &workspace.root else {
        panic!("workspace root must be tabs");
    };
    let titles: Vec<_> = tabs.tabs.iter().map(|t| t.title.as_str()).collect();
    assert_eq!(titles, ["Source", "Spectrogram", "Analysis"]);
    // Tabs は active の子だけを mount する。tab を切り替えて各画面を検証する。
    for (active, key) in [(0, "waveform"), (1, "spectrogram"), (2, "alignments")] {
        let mut view = workspace.clone();
        let ruviform_ui::view::ComponentSpec::Tabs(tabs) = &mut view.root else {
            unreachable!("checked above");
        };
        tabs.active = active;
        let mut host = ruviform_ui::view::ViewHost::new(
            ruviform_ui::dock::ViewInstanceId::new(1).expect("nonzero"),
            ruviform_ui::view::AttachEpoch::new(1),
        );
        host.mount(view).expect("workspace mounts");
        let key = ruviform_ui::view::ComponentKey::from(key);
        assert!(
            host.observe(&key).expect("mounted").mounted,
            "{key} mounted"
        );
    }
}

#[test]
fn real_expensive_dataset_produces_event_rows() {
    // hard-clip → expensive stage の `clipping` candidate
    // (al-analysis tests/expensive.rs と同じ trigger)。
    let dir = TempDir::new("al-gui-view-e2e-events");
    let mut cap_f64 = al_testkit::signal::noise(24_000, 7, 1.0);
    for v in &mut cap_f64 {
        *v = v.signum();
    }
    let cap_wav = dir.join("clip.wav");
    std::fs::write(&cap_wav, wav_s16_bytes(1, 48_000, &to_i16(&cap_f64))).unwrap();

    let service = JobService::new(dir.join("jobs"));
    let spec = RunSpec {
        inputs: vec![parse_input_spec(&format!("capture_bug:{}", cap_wav.display())).unwrap()],
        stages: Some(vec![
            StageKind::Ingest,
            StageKind::CheapAnalysis,
            StageKind::ExpensiveAnalysis,
            StageKind::Dataset,
        ]),
        config: json!({}),
    };
    let (job_id, manifest) = service.run(spec, &RunControl::default()).unwrap();
    assert!(
        manifest
            .stages
            .iter()
            .all(|s| s.status == StageStatus::Completed),
        "all stages completed: {:?}",
        manifest.stages
    );

    let session = DatasetSession::open(service.dataset_dir(&job_id)).unwrap();
    let events = session.events(0, 100, None, None).unwrap();
    let rows = events["rows"].as_array().unwrap();
    assert!(!rows.is_empty(), "clipping capture should produce events");
    let parsed: Vec<EventRow> = rows
        .iter()
        .map(|r| EventRow::from_json(r).expect("real event row parses"))
        .collect();
    assert!(
        parsed.iter().any(|e| e.kind == "clipping"),
        "clipping candidate present: {:?}",
        parsed.iter().map(|e| &e.kind).collect::<Vec<_>>()
    );
    // typed enum が実 schema 由来で保持される。
    assert!(parsed.iter().all(|e| !e.algorithm_id.is_empty()));
}

#[test]
fn real_avi_metadata_produces_stream_time_views() {
    // AVI(movi_order timing)→ ingest → metadata.av_timing の実配列。
    // CFR 映像 + PCM 音声の 2 stream(al-media tests/ingest.rs と同じ
    // fixture 規約)。
    let dir = TempDir::new("al-gui-view-e2e-avi");
    let audio: Vec<u8> = (0..960i16)
        .flat_map(|v| (v % 400 - 200).to_le_bytes())
        .collect();
    let avi = dir.join("cap.avi");
    std::fs::write(
        &avi,
        al_testkit::media::write_avi(&al_testkit::media::AviSpec::cfr(
            64,
            48,
            10,
            vec![vec![7u8; 64 * 48 * 3]; 20],
            audio,
            1,
            48_000,
        )),
    )
    .unwrap();

    let service = JobService::new(dir.join("jobs"));
    let spec = RunSpec {
        inputs: vec![parse_input_spec(&format!("capture_bug:{}", avi.display())).unwrap()],
        stages: Some(vec![StageKind::Ingest, StageKind::Dataset]),
        config: json!({}),
    };
    let (job_id, manifest) = service.run(spec, &RunControl::default()).unwrap();
    assert!(
        manifest
            .stages
            .iter()
            .all(|s| s.status == StageStatus::Completed),
        "all stages completed: {:?}",
        manifest.stages
    );

    let session = DatasetSession::open(service.dataset_dir(&job_id)).unwrap();
    let input_id = session.overview().unwrap()["inputs"].as_array().unwrap()[0]["input_id"]
        .as_str()
        .unwrap()
        .to_string();
    // metadata artifact は `inputs/<input_id>.metadata`。
    let meta = session
        .json_artifact("inputs", &format!("{input_id}.metadata"))
        .unwrap();
    let views = StreamTimeView::from_metadata(&meta).expect("real av_timing array parses");
    assert_eq!(
        views.len(),
        2,
        "vids + auds timing records: {:?}",
        views.iter().map(|v| v.stream_type()).collect::<Vec<_>>()
    );
    // movi_order origin は typed で保持される。
    assert!(
        views
            .iter()
            .all(|v| v.timestamp_origin() == TimestampOrigin::MoviOrder),
        "AVI timing is movi_order-derived: {:?}",
        views
            .iter()
            .map(|v| v.timestamp_origin())
            .collect::<Vec<_>>()
    );
    let audio_stream = views
        .iter()
        .find(|v| v.stream_type() == "auds")
        .expect("audio stream timing present");
    assert_eq!(audio_stream.observation().clock, ClockKind::Audio);
    assert!(!audio_stream.is_empty());
    assert_eq!(audio_stream.seconds_at(0), Some(0.0));
    let video_stream = views
        .iter()
        .find(|v| v.stream_type() == "vids")
        .expect("video stream timing present");
    assert_eq!(video_stream.observation().clock, ClockKind::Video);
    assert_eq!(video_stream.observation().timebase_hz, 10);
}
