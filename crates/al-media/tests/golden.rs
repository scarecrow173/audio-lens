//! ゴールデン fixture 行列: video/A-V + コンテナケース
//! (validation-strategy spec の Video/A-V 行列)。
//!
//! 全ケースは `al-testkit` のパラメタ化 AVI/MP4 writer で決定的に
//! 生成し、`decode_avi`/`decode_mp4`(必要に応じて実 `IngestStage`)
//! の観測が既知の生成パラメータを許容誤差内で回復することを検証
//! する。nominal FPS からの時間再構成が行われないことも対象に
//! 含める。
//!
//! ケース一覧:
//! - CFR timestamp recovery … case_cfr_timestamps_avi /
//!   case_cfr_timestamps_mp4
//! - VFR(nominal FPS 演算なし)… case_vfr_packets_avi /
//!   case_vfr_stts_mp4
//! - 既知 A/V オフセット(dwStart / ctts)…
//!   case_known_av_offset_avi / case_known_av_offset_mp4
//! - A/V ドリフト … case_av_drift_avi
//! - dup / drop フレームメタデータ … case_dup_frame_chunk_avi /
//!   case_drop_frame_avi
//! - ctts B フレーム reorder … case_ctts_reorder_mp4
//! - PCM 抽出往復(sowt / twos)… case_pcm_roundtrip_mp4
//! - decode/input unavailable(圧縮のみ MP4)…
//!   case_compressed_only_mp4_ingest

use std::path::{Path, PathBuf};

use al_core::{InputRole, StageKind, StageStatus};
use al_job::job::{JobDefinition, JobInput, input};
use al_job::manifest::Manifest;
use al_job::runner::StageRunner;
use al_job::stage::Stage;
use al_media::{
    AviMedia, IngestStage, MediaFormat, Mp4Media, SampleFormat, StreamTiming, TimestampOrigin,
    VideoFeatureArtifact, VideoFeatureConfig, decode_avi, decode_mp4, detect_format,
};
use al_testkit::expect::{Tolerance, expect_close, expect_true};
use al_testkit::media::{AviSpec, AviStreamSpec, MoviPacket, pcm_s16le, write_avi};
use al_testkit::mp4::{Mp4EntrySpec, Mp4Spec, Mp4TableSpec, Mp4TrackSpec, mp4_language, write_mp4};
use al_testkit::temp::TempDir;

/// 4x4 / 24bpp DIB フレームのバイト数(row は DWORD aligned)。
const W: u32 = 4;
const H: u32 = 4;
const FRAME_BYTES: usize = (W * H * 3) as usize;

fn frame(fill: u8) -> Vec<u8> {
    vec![fill; FRAME_BYTES]
}

/// `dwSampleSize` を 1 フレームのバイト数にした映像ストリーム。
/// パケットのバイト長がそのまま「保持フレーム数 × dwScale」の tick
/// advance になるため、複数フレームを抱えるパケットや 0 長 dup
/// チャンクが tick 間隔に反映される。
fn sized_video_stream(fps: u32, declared_frames: u32) -> AviStreamSpec {
    let mut s = AviStreamSpec::video(W, H, fps);
    s.sample_size = FRAME_BYTES as u32;
    s.length = Some(declared_frames);
    s
}

fn avi_bytes(fps: u32, streams: Vec<AviStreamSpec>, packets: Vec<MoviPacket>) -> Vec<u8> {
    write_avi(&AviSpec {
        width: W,
        height: H,
        usec_per_frame: 1_000_000 / fps,
        total_frames: None,
        streams,
        packets,
    })
}

fn demux_avi(bytes: &[u8]) -> AviMedia {
    decode_avi(bytes, &VideoFeatureConfig::default()).expect("decode_avi")
}

/// `stream_type` (`vids`/`auds`) の StreamTiming を取り出す。
fn stream_timing<'a>(m: &'a AviMedia, stream_type: &str) -> &'a StreamTiming {
    m.timing
        .iter()
        .find(|t| t.stream_type == stream_type)
        .expect("stream timing")
}

/// MP4 の (handler, origin) に対応する StreamTiming を取り出す。
fn mp4_timing<'a>(m: &'a Mp4Media, handler: &str, origin: TimestampOrigin) -> &'a StreamTiming {
    m.timing
        .iter()
        .find(|t| t.stream_type == handler && t.timestamp_origin == origin)
        .expect("track timing")
}

/// `vide` トラック: stts run 列をそのまま渡し、1 チャンクに全
/// サンプルを置く。sample size は 4 バイト固定。
fn vide_track(track_id: u32, timescale: u32, stts: Vec<(u32, u32)>) -> Mp4TrackSpec {
    let n: u32 = stts.iter().map(|e| e.0).sum();
    let duration: u64 = stts.iter().map(|&(c, d)| u64::from(c) * u64::from(d)).sum();
    Mp4TrackSpec {
        track_id,
        handler: *b"vide",
        timescale,
        duration,
        language: mp4_language(b"und"),
        entry: Mp4EntrySpec::Video {
            codec: *b"avc1",
            width: 320,
            height: 240,
        },
        table: Mp4TableSpec {
            stts,
            ctts: None,
            stss: None,
            stsc: vec![(1, n, 1)],
            chunks: vec![vec![0xAB; n as usize * 4]],
            sample_sizes: vec![4; n as usize],
        },
        extra_trak_boxes: vec![],
    }
}

/// `soun` トラック: `pcm` を 1 チャンクに保持する。stts の 1
/// サンプル = 1 PCM フレーム(`channels * 2` バイト, delta=1 tick)。
fn soun_track(track_id: u32, codec: [u8; 4], channels: u16, rate: u32, pcm: &[u8]) -> Mp4TrackSpec {
    let frames = pcm.len() as u32 / u32::from(channels * 2).max(1);
    Mp4TrackSpec {
        track_id,
        handler: *b"soun",
        timescale: rate,
        duration: u64::from(frames),
        language: mp4_language(b"und"),
        entry: Mp4EntrySpec::Sound {
            codec,
            channels,
            bits: 16,
            rate,
            version: 0,
            lpcm_flags: 0,
            pcmc: None,
        },
        table: Mp4TableSpec::uniform(pcm.to_vec(), frames, u32::from(channels) * 2, 1),
        extra_trak_boxes: vec![],
    }
}

fn mp4_bytes(tracks: Vec<Mp4TrackSpec>) -> Vec<u8> {
    write_mp4(&Mp4Spec {
        major_brand: *b"isom",
        compatible_brands: vec![*b"isom", *b"mp41"],
        movie_timescale: 1000,
        tracks,
        extra_top_boxes: vec![],
    })
}

// ==== CFR タイムスタンプ回復: AVI ====
#[test]
fn case_cfr_timestamps_avi() {
    // 10 fps vids + 48 kHz mono auds(等サイズ wb × 4)。movi は
    // v×5,wb の繰り返しインターリーブ。mux order 由来の tick 列が
    // 既知値どおり回復する。
    let samples: Vec<f64> = (0..3_840)
        .map(|i| ((i % 500) as f64 - 250.0) * 10.0)
        .collect();
    let pcm = pcm_s16le(&samples); // 7680 bytes
    let audio_chunk = pcm.len() / 4; // 1920 bytes = 960 samples = 20 ms
    let mut packets = Vec::new();
    for i in 0..20 {
        packets.push(MoviPacket::video_frame(0, frame(7)));
        if i % 5 == 4 {
            let k = i / 5;
            packets.push(MoviPacket::audio(
                1,
                pcm[k * audio_chunk..(k + 1) * audio_chunk].to_vec(),
            ));
        }
    }
    let m = demux_avi(&avi_bytes(
        10,
        vec![
            AviStreamSpec::video(W, H, 10),
            AviStreamSpec::pcm_audio(1, 48_000),
        ],
        packets,
    ));

    let v = stream_timing(&m, "vids");
    assert_eq!(v.timestamp_origin, TimestampOrigin::MoviOrder);
    assert_eq!(v.start_tick, 0);
    assert_eq!(v.stream_timestamps, (0..20).collect::<Vec<i64>>());
    assert!(v.observation.intervals.iter().all(|&i| i == 1));
    assert_eq!(v.observation.timebase_hz, 10);
    let stats = v.observation.interval_stats.as_ref().unwrap();
    assert_eq!(
        (stats.min_ticks, stats.max_ticks, stats.median_ticks),
        (1, 1, 1)
    );
    expect_close(
        "vids mean interval",
        1.0,
        stats.mean_ticks,
        Tolerance::abs(1e-9),
    );
    // CFR は nominal(=dwScale)と一致 → ドリフト 0
    expect_close(
        "vids drift_ppm",
        0.0,
        v.observation.drift_ppm.expect("drift_ppm"),
        Tolerance::abs(1e-9),
    );
    expect_true(
        "vids observation is clean",
        v.observation.discontinuities.is_empty()
            && v.observation.duplicates.is_empty()
            && v.observation.drop_candidates.is_empty(),
        format!("{:?}", v.observation),
    );

    // auds: 1920 バイトの等間隔パケット。tick advance は
    // len*dwScale/dwSampleSize = バイト数そのもの(96 kHz byte-rate
    // クロックで 1920 tick = 20 ms)。
    let a = stream_timing(&m, "auds");
    assert_eq!(a.stream_timestamps, vec![0, 1920, 3840, 5760]);
    assert!(a.observation.intervals.iter().all(|&i| i == 1920));
    assert_eq!(a.observation.timebase_hz, 96_000);

    // PCM 往復: 生成サンプルがそのまま戻る
    let audio = m.audio.as_ref().expect("audio decoded");
    assert_eq!(audio.samples, samples);
    assert_eq!(audio.codec, "pcm_s16le");
    assert_eq!(audio.sample_rate, 48_000);

    let info = m.video.as_ref().expect("video info");
    assert_eq!(info.frames, 20);
    expect_close(
        "declared fps",
        10.0,
        info.fps.unwrap(),
        Tolerance::abs(1e-9),
    );
}

// ==== CFR タイムスタンプ回復: MP4 ====
#[test]
fn case_cfr_timestamps_mp4() {
    // 30 fps(3000 ticks @ 90 kHz)の vide + 48 kHz sowt の soun。
    let pcm: Vec<u8> = (0..960i16).flat_map(|v| v.to_le_bytes()).collect();
    let bytes = mp4_bytes(vec![
        vide_track(1, 90_000, vec![(24, 3_000)]),
        soun_track(2, *b"sowt", 1, 48_000, &pcm),
    ]);
    assert_eq!(detect_format(&bytes), Some(MediaFormat::Mp4));
    let m = decode_mp4(&bytes).unwrap();

    let v = mp4_timing(&m, "vide", TimestampOrigin::SampleTable);
    assert_eq!(
        v.stream_timestamps,
        (0..24).map(|i| i * 3_000).collect::<Vec<i64>>()
    );
    assert!(v.observation.intervals.iter().all(|&i| i == 3_000));
    assert_eq!(v.observation.timebase_hz, 90_000);
    // MP4 は nominal interval を捏造しない → drift_ppm は記録されない
    assert!(v.observation.drift_ppm.is_none());
    assert!(v.observation.discontinuities.is_empty());

    let a = mp4_timing(&m, "soun", TimestampOrigin::SampleTable);
    assert_eq!(a.stream_timestamps, (0..960).collect::<Vec<i64>>());
    assert_eq!(a.observation.timebase_hz, 48_000);
}

// ==== VFR: AVI(可変パケットサイズ) ====
#[test]
fn case_vfr_packets_avi() {
    // dwSampleSize = フレームサイズの vids: パケットが 2〜3 フレーム分の
    // バイトを抱えると、その分だけ clock が進む → nominal FPS 演算なしで
    // 実デルタが VFR として観測される。
    let vids = sized_video_stream(10, 15); // 2+3+2+3+3+2 = 15 フレーム宣言
    let spans = [2usize, 3, 2, 3, 3, 2];
    let mut packets = Vec::new();
    for (i, &span) in spans.iter().enumerate() {
        packets.push(MoviPacket::video_frame(
            0,
            vec![i as u8; span * FRAME_BYTES],
        ));
        if i % 2 == 0 {
            packets.push(MoviPacket::audio(1, pcm_s16le(&vec![100.0; 960])));
        }
    }
    let m = demux_avi(&avi_bytes(
        10,
        vec![vids, AviStreamSpec::pcm_audio(1, 48_000)],
        packets,
    ));

    let v = stream_timing(&m, "vids");
    // 累積: 0,2,5,7,10,13 — 可変パケットサイズが間隔に直写しされる
    assert_eq!(v.stream_timestamps, vec![0, 2, 5, 7, 10, 13]);
    assert_eq!(v.observation.intervals, vec![2, 3, 2, 3, 3]);
    let stats = v.observation.interval_stats.as_ref().unwrap();
    assert_eq!(stats.min_ticks, 2);
    assert_eq!(stats.max_ticks, 3);
    assert_eq!(stats.median_ticks, 3);
    expect_close(
        "vfr mean interval",
        2.6,
        stats.mean_ticks,
        Tolerance::abs(1e-9),
    );
    // VFR の揺らぎは discontinuity / drop ではない
    expect_true(
        "vfr has no false discontinuity",
        v.observation.discontinuities.is_empty()
            && v.observation.duplicates.is_empty()
            && v.observation.drop_candidates.is_empty(),
        format!("{:?}", v.observation),
    );
    // nominal(1 パケット = dwScale=1 tick)に対する観測平均は 2.6 tick。
    // 複数フレーム抱合パケットを 1 フレームと誤認しない事実として残る。
    expect_close(
        "vids drift vs per-packet nominal",
        1_600_000.0,
        v.observation.drift_ppm.unwrap(),
        Tolerance::abs(1.0),
    );
    // 宣言フレーム数(15)は実パケット数(6)ではなくバイト量で語られる
    assert_eq!(m.video.as_ref().unwrap().frames, 15);

    // auds は等間隔 wb × 3: 別クロックが VFR に引きずられない
    let a = stream_timing(&m, "auds");
    assert_eq!(a.stream_timestamps, vec![0, 1920, 3840]);
    assert!(a.observation.intervals.iter().all(|&i| i == 1920));
}

// ==== VFR: MP4(非一様 stts run) ====
#[test]
fn case_vfr_stts_mp4() {
    // stts run が非一様: (3,1000),(2,1500),(4,2000),(2,1000)。
    // nominal FPS を仮定せず stts の実デルタがそのまま出る。
    let t = vide_track(
        1,
        90_000,
        vec![(3, 1_000), (2, 1_500), (4, 2_000), (2, 1_000)],
    );
    let m = decode_mp4(&mp4_bytes(vec![t])).unwrap();

    let v = mp4_timing(&m, "vide", TimestampOrigin::SampleTable);
    assert_eq!(
        v.stream_timestamps,
        vec![
            0, 1000, 2000, 3000, 4500, 6000, 8000, 10000, 12000, 14000, 15000
        ]
    );
    assert_eq!(
        v.observation.intervals,
        vec![1000, 1000, 1000, 1500, 1500, 2000, 2000, 2000, 2000, 1000]
    );
    let stats = v.observation.interval_stats.as_ref().unwrap();
    assert_eq!(stats.min_ticks, 1_000);
    assert_eq!(stats.max_ticks, 2_000);
    assert_eq!(stats.median_ticks, 1_500);
    expect_close(
        "vfr mean interval",
        1_500.0,
        stats.mean_ticks,
        Tolerance::abs(1e-9),
    );
    // VFR の変動は discontinuity ではなく、nominal 由来の drift も出ない
    assert!(v.observation.discontinuities.is_empty());
    assert!(v.observation.drop_candidates.is_empty());
    assert!(v.observation.drift_ppm.is_none());
}

// ==== 既知 A/V オフセット: AVI(dwStart) ====
#[test]
fn case_known_av_offset_avi() {
    // vids dwStart=3(scale=1, 30 fps → 100 ms)、auds dwStart=2400
    // (scale=2, byte-rate 96 kHz → 50 ms)。start_tick が既知
    // オフセットをエンコードし、パケット位置はそこから始まる。
    let mut packets: Vec<MoviPacket> = (0..10)
        .map(|i| MoviPacket::video_frame(0, frame(i as u8)))
        .collect();
    packets.push(MoviPacket::audio(1, pcm_s16le(&vec![50.0; 960])));
    let m = demux_avi(&avi_bytes(
        30,
        vec![
            AviStreamSpec::video(W, H, 30).with_start(3),
            AviStreamSpec::pcm_audio(1, 48_000).with_start(2_400),
        ],
        packets,
    ));

    let v = stream_timing(&m, "vids");
    let a = stream_timing(&m, "auds");
    // start_tick = dwStart * dwScale
    assert_eq!(v.start_tick, 3);
    assert_eq!(a.start_tick, 4_800);
    assert_eq!(v.stream_timestamps[0], 3);
    assert_eq!(a.stream_timestamps[0], 4_800);
    // 秒換算で既知オフセットが回復する: 0.1 s - 0.05 s = 50 ms
    let v_start = v.start_tick as f64 / v.observation.timebase_hz as f64;
    let a_start = a.start_tick as f64 / a.observation.timebase_hz as f64;
    expect_close(
        "av offset secs",
        0.05,
        v_start - a_start,
        Tolerance::abs(1e-9),
    );

    // video_features も同じストリームクロックを共有する
    let VideoFeatureArtifact::Decoded(f) = m.video_features.as_ref().expect("video features")
    else {
        panic!("expected decoded video features")
    };
    assert_eq!(f.start_tick, 3);
    assert_eq!(f.frame_timestamps[0], 3);
}

// ==== 既知 A/V オフセット: MP4(トラック間オフセット) ====
#[test]
fn case_known_av_offset_mp4() {
    // MP4 の DTS は常に 0 起点。トラック間のオフセットは ctts の
    // presentation shift で表現する: vide に一律 +6000 ticks
    // (90 kHz で 1/15 s)の遅延を与え、soun は 0 開始。
    let mut v = vide_track(1, 90_000, vec![(12, 3_000)]);
    v.table.ctts = Some(vec![(12, 6_000)]);
    let pcm: Vec<u8> = (0..480i16).flat_map(|s| s.to_le_bytes()).collect();
    let m = decode_mp4(&mp4_bytes(vec![
        v,
        soun_track(2, *b"sowt", 1, 48_000, &pcm),
    ]))
    .unwrap();

    // decode order DTS はオフセットを持たない
    let dts = mp4_timing(&m, "vide", TimestampOrigin::SampleTable);
    assert_eq!(dts.stream_timestamps[0], 0);
    // presentation PTS は既知の +6000 tick シフトを保持する
    let pts = mp4_timing(&m, "vide", TimestampOrigin::SampleTablePts);
    assert_eq!(
        pts.stream_timestamps,
        (0..12).map(|i| 6_000 + i * 3_000).collect::<Vec<i64>>()
    );
    let a = mp4_timing(&m, "soun", TimestampOrigin::SampleTable);
    assert_eq!(a.stream_timestamps[0], 0);

    // 秒換算オフセット: 6000/90000 - 0/48000 = 1/15 s
    let offset_secs =
        pts.stream_timestamps[0] as f64 / 90_000.0 - a.stream_timestamps[0] as f64 / 48_000.0;
    expect_close(
        "mp4 av offset secs",
        1.0 / 15.0,
        offset_secs,
        Tolerance::abs(1e-9),
    );
}

// ==== A/V ドリフト: AVI ====
#[test]
fn case_av_drift_avi() {
    // 90 kHz / 3000 tick nominal(30 fps)の vids で、実パケットが宣言
    // sample_size をわずかに超える → 観測平均間隔が nominal を上回る
    // 既知の +1000 ppm ドリフト。第 1 パケットのみ 3057 バイト
    // (advance 3057)、残り 19 パケットは 3000 バイト (advance 3000)。
    // mean = (3057 + 18*3000)/19 = 3003 → (3003-3000)/3000 = +1000 ppm。
    let mut vids = AviStreamSpec::video(W, H, 30);
    vids.scale = 3_000;
    vids.rate = 90_000;
    vids.sample_size = 3_000;
    vids.length = Some(20);
    let mut packets = vec![MoviPacket::video_frame(0, vec![7u8; 3_057])];
    packets.extend((0..19).map(|_| MoviPacket::video_frame(0, vec![7u8; 3_000])));
    packets.push(MoviPacket::audio(1, pcm_s16le(&vec![100.0; 960])));
    let m = demux_avi(&avi_bytes(
        30,
        vec![vids, AviStreamSpec::pcm_audio(1, 48_000)],
        packets,
    ));

    let v = stream_timing(&m, "vids");
    assert_eq!(v.observation.intervals[0], 3_057);
    assert!(v.observation.intervals[1..].iter().all(|&i| i == 3_000));
    expect_close(
        "av drift ppm",
        1_000.0,
        v.observation.drift_ppm.expect("drift_ppm"),
        Tolerance::abs(1e-3),
    );
    // 緩やかな乖離はギャップではない: discontinuity / drop は立たない
    assert!(v.observation.discontinuities.is_empty());
    assert!(v.observation.drop_candidates.is_empty());
    // フレーム timestamp 列もドリフトした tick をそのまま保持する
    let VideoFeatureArtifact::Decoded(f) = m.video_features.as_ref().expect("video features")
    else {
        panic!("expected decoded video features")
    };
    assert_eq!(f.frame_timestamps, v.stream_timestamps);
    assert_eq!(*v.stream_timestamps.last().unwrap(), 3_057 + 18 * 3_000);
}

// ==== dup フレーム: AVI(0 長 db チャンク) ====
#[test]
fn case_dup_frame_chunk_avi() {
    // 0 長 00db(AVI の dup フレーム表現)。dwSampleSize 宣言
    // ストリームでは 0 バイトパケットは clock を進めないので、
    // 次フレームと同じ tick に重複として記録される。
    let packets = vec![
        MoviPacket::video_frame(0, frame(1)),
        MoviPacket::video_frame(0, frame(2)),
        MoviPacket::video_frame(0, vec![]), // dup: 0 長
        MoviPacket::video_frame(0, frame(3)),
        MoviPacket::video_frame(0, frame(4)),
        MoviPacket::audio(1, pcm_s16le(&vec![50.0; 480])),
    ];
    let m = demux_avi(&avi_bytes(
        10,
        vec![
            sized_video_stream(10, 5),
            AviStreamSpec::pcm_audio(1, 48_000),
        ],
        packets,
    ));

    let v = stream_timing(&m, "vids");
    assert_eq!(v.stream_timestamps, vec![0, 1, 2, 2, 3]);
    assert_eq!(v.observation.duplicates, vec![3]);
    assert!(v.observation.drop_candidates.is_empty());
    // dup パケットも db/dc チャンクとしてフレーム数に数えられる
    assert_eq!(m.video.as_ref().unwrap().frames, 5);
    // 0 バイトフレームはデコード不能 → unavailable として記録される
    let Some(VideoFeatureArtifact::Unavailable { codec, reason, .. }) = &m.video_features else {
        panic!("zero-length frame must surface as unavailable")
    };
    assert_eq!(codec, "rawvideo");
    assert!(reason.contains("0 bytes"), "reason: {reason}");
}

// ==== drop フレーム: AVI(宣言長 > パケット数 + ギャップ) ====
#[test]
fn case_drop_frame_avi() {
    // 宣言 dwLength=8 に対し movi は 7 パケット、そのうち 1 個が
    // 2 フレーム分(96 B)を抱える → tick の孤立したギャップが
    // drop_candidates に記録され、frames は宣言値 8 を保持する。
    let packets = vec![
        MoviPacket::video_frame(0, frame(1)),
        MoviPacket::video_frame(0, frame(2)),
        MoviPacket::video_frame(0, frame(3)),
        MoviPacket::video_frame(0, frame(4)),
        MoviPacket::video_frame(0, frame(5)),
        MoviPacket::video_frame(0, vec![6u8; FRAME_BYTES * 2]), // 2 フレーム分
        MoviPacket::video_frame(0, frame(7)),
        MoviPacket::audio(1, pcm_s16le(&vec![50.0; 480])),
    ];
    let m = demux_avi(&avi_bytes(
        10,
        vec![
            sized_video_stream(10, 8),
            AviStreamSpec::pcm_audio(1, 48_000),
        ],
        packets,
    ));

    let v = stream_timing(&m, "vids");
    // tick 5 の次が 7: 欠落スロットがギャップとして残る
    assert_eq!(v.stream_timestamps, vec![0, 1, 2, 3, 4, 5, 7]);
    assert_eq!(v.observation.intervals, vec![1, 1, 1, 1, 1, 2]);
    assert_eq!(v.observation.discontinuities.len(), 1);
    assert_eq!(v.observation.discontinuities[0].interval_index, 5);
    assert_eq!(v.observation.discontinuities[0].observed_ticks, 2);
    assert_eq!(v.observation.drop_candidates, vec![5]);
    // 宣言 8 フレーム vs 実パケット 7 の不一致も記録される
    assert_eq!(m.video.as_ref().unwrap().frames, 8);
    // フレーム timestamp 列にもギャップが残る(フレーム index ではなく
    // ストリームクロック位置が記録されるため)
    let VideoFeatureArtifact::Decoded(f) = m.video_features.as_ref().expect("video features")
    else {
        panic!("expected decoded video features")
    };
    assert_eq!(f.frame_timestamps, vec![0, 1, 2, 3, 4, 5, 7]);
    assert_eq!(f.luminance.len(), 7);
}

// ==== ctts B フレーム reorder: MP4 ====
#[test]
fn case_ctts_reorder_mp4() {
    // decode order I,P,B,B: dts = [0,3000,6000,9000]、ctts =
    // [3000,9000,0,0] → decode order の pts = [3000,12000,6000,9000]。
    // presentation order にソートすると単調系列 [3000,...,12000] になり、
    // reorder が偽の discontinuity を生まない。
    let mut t = vide_track(1, 90_000, vec![(4, 3_000)]);
    t.table.ctts = Some(vec![(1, 3_000), (1, 9_000), (1, 0), (1, 0)]);
    t.table.stss = Some(vec![1]); // 先頭 I フレームのみ sync
    let m = decode_mp4(&mp4_bytes(vec![t])).unwrap();
    assert_eq!(m.facts.tracks[0].sync_samples, Some(vec![1]));

    let dts = mp4_timing(&m, "vide", TimestampOrigin::SampleTable);
    assert_eq!(dts.stream_timestamps, vec![0, 3_000, 6_000, 9_000]);
    // DTS 系列は単調で偽の discontinuity を持たない
    assert!(dts.stream_timestamps.windows(2).all(|w| w[1] > w[0]));
    assert!(dts.observation.discontinuities.is_empty());

    let pts = mp4_timing(&m, "vide", TimestampOrigin::SampleTablePts);
    assert_eq!(pts.stream_timestamps, vec![3_000, 6_000, 9_000, 12_000]);
    assert_eq!(pts.observation.intervals, vec![3_000; 3]);
    assert!(pts.observation.discontinuities.is_empty());
    assert!(pts.observation.duplicates.is_empty());
}

// ==== PCM 抽出往復: MP4(sowt / twos) ====
#[test]
fn case_pcm_roundtrip_mp4() {
    // 既知 s16 サンプル列: sowt(LE)/twos(BE)で往復して一致する。
    let samples: Vec<i16> = vec![0, 1_000, -1_000, 32_767, -32_768, 42, -42, 12_345];
    let expected: Vec<f64> = samples.iter().map(|&v| f64::from(v)).collect();

    let le: Vec<u8> = samples.iter().flat_map(|v| v.to_le_bytes()).collect();
    let m = decode_mp4(&mp4_bytes(vec![soun_track(1, *b"sowt", 2, 48_000, &le)])).unwrap();
    let a = m.audio.as_ref().expect("sowt decoded");
    assert_eq!(a.codec, "pcm_s16le");
    assert_eq!(a.sample_format, SampleFormat::PcmS16);
    assert_eq!(a.channels, 2);
    assert_eq!(a.sample_rate, 48_000);
    assert_eq!(a.samples, expected);
    assert_eq!(a.frames(), 4);

    let be: Vec<u8> = samples.iter().flat_map(|v| v.to_be_bytes()).collect();
    let m = decode_mp4(&mp4_bytes(vec![soun_track(1, *b"twos", 2, 48_000, &be)])).unwrap();
    let a = m.audio.as_ref().expect("twos decoded");
    assert_eq!(a.codec, "pcm_s16be");
    assert_eq!(a.samples, expected);
}

// ==== decode/input unavailable: 圧縮のみ MP4 の ingest ====
#[test]
fn case_compressed_only_mp4_ingest() {
    // avc1 + mp4a のみ: PCM は存在しないが、metadata + timing
    // artifact は persist され、integrity observation が記録される
    // (decode/input unavailable はステージ失敗ではない)。
    let dir = temp_dir("compressed-mp4");
    let bytes = mp4_bytes(vec![
        vide_track(1, 90_000, vec![(30, 3_000)]),
        soun_track(2, *b"mp4a", 2, 48_000, &[0u8; 48]),
    ]);
    let mp4 = write_fixture(&dir, "cap.mp4", &bytes);

    let job_dir = temp_dir("job-compressed");
    let manifest = run_ingest(&job_dir, vec![input(InputRole::CaptureBug, &mp4)]);
    assert_eq!(
        manifest.stage(StageKind::Ingest).unwrap().status,
        StageStatus::Completed
    );
    // decoded_pcm は生成されないが metadata artifact は残る
    assert!(!manifest.artifacts.iter().any(|a| a.role == "decoded_pcm"));
    assert!(
        manifest
            .artifacts
            .iter()
            .any(|a| a.role == "ingest_metadata")
    );

    let meta = read_metadata(&job_dir, &manifest);
    assert_eq!(meta["container"], "mp4");
    assert!(meta["audio"].is_null());
    assert_eq!(meta["mp4"]["tracks"].as_array().unwrap().len(), 2);
    // vide/soun 両トラックの DTS observation が persist される
    let timing = meta["av_timing"].as_array().expect("av_timing array");
    assert_eq!(
        timing
            .iter()
            .filter(|t| t["timestamp_origin"] == "sample_table")
            .count(),
        2
    );
    let integrity = meta["integrity"].as_array().expect("integrity array");
    assert!(
        integrity
            .iter()
            .any(|o| o["kind"] == "decode/input unavailable" && o["detail"]["codec"] == "mp4a"),
        "integrity: {integrity:?}"
    );
}

// ---- ingest ステージ経由のケース用ヘルパ ----

fn temp_dir(tag: &str) -> TempDir {
    TempDir::new(&format!("al-media-golden-{tag}"))
}

fn write_fixture(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

fn run_ingest(job_dir: &Path, inputs: Vec<JobInput>) -> Manifest {
    let runner = StageRunner::new(job_dir);
    let def =
        JobDefinition::create(inputs, serde_json::json!({}), vec![StageKind::Ingest]).unwrap();
    let mut stages: Vec<Box<dyn Stage>> = vec![Box::new(IngestStage::new())];
    runner.run(&def, &mut stages).unwrap()
}

fn read_metadata(job_dir: &Path, manifest: &Manifest) -> serde_json::Value {
    let runner = StageRunner::new(job_dir);
    let rec = manifest
        .artifacts
        .iter()
        .find(|a| a.role == "ingest_metadata")
        .expect("ingest_metadata artifact");
    let path = runner.job_dir(&manifest.job_id).join(&rec.path);
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
