use al_media::decoder::{DecodeRequest, DecoderRegistry};
use al_media::decoder::{MediaDecoder, NativeDecoder, UnavailableKind};
use al_testkit::media::{AviSpec, AviStreamSpec, MoviPacket, write_avi};

#[test]
fn native_registry_preserves_pcm_and_records_selection() {
    let bytes = al_testkit::media::wav_s16_bytes(2, 48000, &[123, -456, 32767, -32768]);
    let registry = DecoderRegistry::native();
    let result = registry.decode(&bytes, &DecodeRequest::default()).unwrap();
    assert_eq!(
        result.audio.unwrap().samples,
        vec![123.0, -456.0, 32767.0, -32768.0]
    );
    assert_eq!(result.provenance.backend, "native");
    assert!(!result.provenance.fallback);
}

#[test]
fn probe_is_pinned_and_does_not_decode_bad_payloads() {
    let spec = AviSpec::cfr(
        4,
        2,
        30,
        vec![vec![0; 1]],
        al_testkit::media::pcm_s16le(&[1.0, -1.0]),
        1,
        48000,
    );
    let bytes = write_avi(&spec);
    let probe = NativeDecoder.probe(&bytes).unwrap();
    assert!(probe.streams[0].decode.is_ok());
    assert_eq!(probe.streams[0].packet_lengths, vec![1]);
    let result = NativeDecoder
        .decode(&bytes, &probe, &DecodeRequest::default())
        .unwrap();
    assert!(result.audio.is_some());
    let failure = result
        .provenance
        .streams
        .iter()
        .find(|s| s.kind == "video")
        .unwrap();
    assert_eq!(
        failure.unavailable.as_ref().unwrap().kind,
        UnavailableKind::Malformed
    );

    let mut changed = bytes.clone();
    *changed.last_mut().unwrap() ^= 1;
    assert!(matches!(
        NativeDecoder.decode(&changed, &probe, &DecodeRequest::default()),
        Err(al_media::MediaError::Malformed { .. })
    ));
}

#[test]
fn avi_audio_errors_are_typed_and_independent_clocks_survive() {
    // 共有 writer の未パディング BGR24 契約に沿った幅・frame 長。
    for (format, kind) in [
        (vec![0; 2], UnavailableKind::Malformed),
        (
            {
                let mut fmt = al_testkit::media::waveformatex(1, 48000);
                fmt[..2].copy_from_slice(&0x55u16.to_le_bytes());
                fmt
            },
            UnavailableKind::Capability,
        ),
    ] {
        let mut spec = AviSpec::cfr(
            4,
            2,
            30,
            vec![vec![255; 4 * 2 * 3]],
            al_testkit::media::pcm_s16le(&[10.0]),
            1,
            48000,
        );
        spec.streams[1].format = format;
        let bytes = write_avi(&spec);
        let result = DecoderRegistry::native()
            .decode(&bytes, &DecodeRequest::default())
            .unwrap();
        assert!(result.audio.is_none());
        assert!(matches!(
            result.video_features,
            Some(al_media::VideoFeatureArtifact::Decoded(_))
        ));
        assert_eq!(result.av_timing.len(), 2);
        assert_eq!(result.av_timing[0].observation.timebase_hz, 30);
        assert_eq!(result.av_timing[1].observation.timebase_hz, 96000);
        let failure = result
            .provenance
            .streams
            .iter()
            .find(|s| s.kind == "audio")
            .unwrap();
        assert_eq!(failure.unavailable.as_ref().unwrap().kind, kind);
        assert!(!result.provenance.fallback);
    }
}

#[test]
fn timing_only_skips_pcm_and_pixel_decode_and_arbitrary_selection_is_rejected() {
    let spec = AviSpec::cfr(
        4,
        2,
        30,
        vec![vec![0; 1]],
        al_testkit::media::pcm_s16le(&[10.0]),
        1,
        48000,
    );
    let bytes = write_avi(&spec);
    let request = DecodeRequest {
        audio: false,
        video: false,
        ..Default::default()
    };
    let result = DecoderRegistry::native().decode(&bytes, &request).unwrap();
    assert!(result.audio.is_none());
    assert!(result.video_features.is_none());
    assert_eq!(result.av_timing.len(), 2);
    assert!(result.provenance.streams.is_empty());
    assert_eq!(result.provenance.attempts[0].outcome, "timing_only");
    let request = DecodeRequest {
        audio_stream: Some(99),
        ..Default::default()
    };
    assert!(matches!(
        DecoderRegistry::native().decode(&bytes, &request),
        Err(al_media::MediaError::InputUnavailable { .. })
    ));
}

#[test]
fn native_is_first_and_duplicate_backend_is_rejected() {
    struct NeverRun;
    impl MediaDecoder for NeverRun {
        fn id(&self) -> &'static str {
            "test"
        }
        fn version(&self) -> &'static str {
            "1"
        }
        fn detect(&self, _: &[u8]) -> Option<al_media::MediaFormat> {
            Some(al_media::MediaFormat::Wav)
        }
        fn probe(&self, _: &[u8]) -> Result<al_media::decoder::MediaProbe, al_media::MediaError> {
            panic!("native corruption must never fall through")
        }
        fn decode(
            &self,
            _: &[u8],
            _: &al_media::decoder::MediaProbe,
            _: &DecodeRequest,
        ) -> Result<al_media::decoder::IngestResult, al_media::MediaError> {
            unreachable!()
        }
    }
    let mut registry = DecoderRegistry::native();
    registry.register(Box::new(NeverRun)).unwrap();
    assert!(registry.register(Box::new(NativeDecoder)).is_err());
    assert!(registry.register(Box::new(NeverRun)).is_err());
    assert_eq!(
        registry.resolve(b"RIFF\0\0\0\0WAVE").unwrap().id(),
        "native"
    );
    assert!(matches!(
        registry.decode(b"RIFF\0\0\0\0WAVE", &DecodeRequest::default()),
        Err(al_media::MediaError::Malformed { .. })
    ));
}

#[test]
fn first_avi_audio_stream_is_the_only_decoded_stream() {
    let spec = AviSpec {
        width: 0,
        height: 0,
        usec_per_frame: 0,
        total_frames: None,
        streams: vec![
            AviStreamSpec::pcm_audio(1, 48000),
            AviStreamSpec::pcm_audio(1, 48000),
        ],
        packets: vec![
            MoviPacket::audio(0, 123i16.to_le_bytes().to_vec()),
            MoviPacket::audio(1, 456i16.to_le_bytes().to_vec()),
        ],
    };
    let bytes = write_avi(&spec);
    let probe = NativeDecoder.probe(&bytes).unwrap();
    assert!(probe.streams[0].decode.is_ok());
    assert_eq!(
        probe.streams[1].decode.as_ref().unwrap_err().kind,
        UnavailableKind::Capability
    );
    let result = DecoderRegistry::native()
        .decode(&bytes, &DecodeRequest::default())
        .unwrap();
    assert_eq!(result.audio.unwrap().samples, vec![123.0]);
    assert_eq!(result.av_timing.len(), 2);
}

#[test]
fn zero_block_align_is_malformed_instead_of_panicking() {
    let mut bytes = al_testkit::media::wav_s16_bytes(1, 48000, &[1]);
    bytes[32..34].fill(0);
    assert!(matches!(
        DecoderRegistry::native().decode(&bytes, &DecodeRequest::default()),
        Err(al_media::MediaError::Malformed { .. })
    ));
}

#[test]
fn all_wav_native_widths_preserve_scale_and_nonfinite_float_values() {
    use al_media::wav::{WavEncoding, WavWriter};
    for encoding in [
        WavEncoding::PcmS16,
        WavEncoding::PcmS24,
        WavEncoding::PcmS32,
        WavEncoding::Float32,
        WavEncoding::Float64,
    ] {
        let samples = [123.0, -456.0, 32767.0, -32768.0];
        let mut writer =
            WavWriter::new(std::io::Cursor::new(Vec::new()), encoding, 2, 48000).unwrap();
        writer.write_samples(&samples).unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        let result = DecoderRegistry::native()
            .decode(&bytes, &DecodeRequest::default())
            .unwrap();
        let audio = result.audio.unwrap();
        assert_eq!(audio.samples, samples);
        assert_eq!(audio.channels, 2);
        assert_eq!(audio.sample_rate, 48000);
    }
    let mut writer = WavWriter::new(
        std::io::Cursor::new(Vec::new()),
        WavEncoding::Float64,
        1,
        48000,
    )
    .unwrap();
    writer
        .write_samples(&[f64::NAN, f64::INFINITY, f64::NEG_INFINITY])
        .unwrap();
    let result = DecoderRegistry::native()
        .decode(
            &writer.finish().unwrap().into_inner(),
            &DecodeRequest::default(),
        )
        .unwrap();
    let samples = result.audio.unwrap().samples;
    assert!(samples[0].is_nan());
    assert_eq!(samples[1], f64::INFINITY);
    assert_eq!(samples[2], f64::NEG_INFINITY);

    let mut bytes = al_testkit::media::wav_s16_bytes(1, 48000, &[0, 0]);
    bytes.truncate(44);
    bytes.extend_from_slice(&[0, 128, 255, 0]);
    bytes[4..8].copy_from_slice(&40u32.to_le_bytes());
    bytes[28..32].copy_from_slice(&48000u32.to_le_bytes());
    bytes[32..34].copy_from_slice(&1u16.to_le_bytes());
    bytes[34..36].copy_from_slice(&8u16.to_le_bytes());
    bytes[40..44].copy_from_slice(&3u32.to_le_bytes());
    let result = DecoderRegistry::native()
        .decode(&bytes, &DecodeRequest::default())
        .unwrap();
    assert_eq!(result.audio.unwrap().samples, vec![-128.0, 0.0, 127.0]);
}

#[test]
fn mp4_keeps_existing_pcm_and_compressed_tracks_are_timing_only() {
    use al_testkit::mp4::{
        Mp4EntrySpec, Mp4Spec, Mp4TableSpec, Mp4TrackSpec, mp4_language, write_mp4,
    };
    let track = |id, codec| Mp4TrackSpec {
        track_id: id,
        handler: *b"soun",
        timescale: 48000,
        duration: 2,
        language: mp4_language(b"und"),
        entry: Mp4EntrySpec::Sound {
            codec,
            channels: 1,
            bits: 16,
            rate: 48000,
            version: 0,
            lpcm_flags: 0,
            pcmc: None,
        },
        table: Mp4TableSpec::uniform(al_testkit::media::pcm_s16le(&[123.0, -456.0]), 2, 2, 1),
        extra_trak_boxes: vec![],
    };
    let spec = Mp4Spec {
        major_brand: *b"isom",
        compatible_brands: vec![*b"isom"],
        movie_timescale: 1000,
        tracks: vec![track(1, *b"mp4a"), track(2, *b"sowt")],
        extra_top_boxes: vec![],
    };
    let bytes = write_mp4(&spec);
    let probe = NativeDecoder.probe(&bytes).unwrap();
    assert_eq!(
        probe.streams[0].decode.as_ref().unwrap_err().kind,
        UnavailableKind::Capability
    );
    assert!(probe.streams[1].decode.is_ok());
    assert_eq!(probe.streams[1].packet_lengths, vec![2, 2]);
    let result = DecoderRegistry::native()
        .decode(&bytes, &DecodeRequest::default())
        .unwrap();
    assert_eq!(result.audio.unwrap().samples, vec![123.0, -456.0]);
    assert_eq!(
        result
            .provenance
            .streams
            .iter()
            .find(|s| s.decoded)
            .unwrap()
            .stream,
        Some(1)
    );
    assert_eq!(
        result.av_timing,
        al_media::decode_mp4(&bytes).unwrap().timing
    );

    let mut compressed = spec;
    compressed.tracks.truncate(1);
    let result = DecoderRegistry::native()
        .decode(&write_mp4(&compressed), &DecodeRequest::default())
        .unwrap();
    assert!(result.audio.is_none());
    assert!(!result.av_timing.is_empty());
    assert_eq!(result.provenance.attempts[0].outcome, "timing_only");
    assert_eq!(
        result.provenance.streams[0]
            .unavailable
            .as_ref()
            .unwrap()
            .kind,
        UnavailableKind::Capability
    );
}

#[test]
fn unknown_format_is_capability_failure_and_corruption_is_fatal() {
    let registry = DecoderRegistry::native();
    assert!(matches!(
        registry.decode(b"unknown", &DecodeRequest::default()),
        Err(al_media::MediaError::InputUnavailable { .. })
    ));
    assert!(matches!(
        registry.decode(b"RIFF\0\0\0\0WAVE", &DecodeRequest::default()),
        Err(al_media::MediaError::Malformed { .. })
    ));
}
