//! WAV codec property suite and Kani proof harness (Issue #256):
//! `decode_wav`/`wav_frame_count`/`read_wav_range` never panic on
//! arbitrary or mutated bytes, `WavWriter` output round-trips
//! losslessly through `decode_wav`, and range reads agree with the
//! full decode. The `#[cfg(kani)]` module holds bounded proofs that
//! only `cargo kani` compiles.

use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};

use al_media::wav::{WavEncoding, WavWriter, read_wav_range, wav_frame_count};
use al_media::{SampleFormat, decode_wav};
use proptest::prelude::*;

#[cfg(kani)]
mod kani_stub_models;

/// Largest sample rate that cannot overflow `byte_rate` for any
/// reachable `block_align` (8 channels x 8 bytes = 64).
const RATE_CAP: u32 = u32::MAX / 64;

fn encoding_strategy() -> impl Strategy<Value = WavEncoding> {
    prop_oneof![
        Just(WavEncoding::PcmS16),
        Just(WavEncoding::PcmS24),
        Just(WavEncoding::PcmS32),
        Just(WavEncoding::Float32),
        Just(WavEncoding::Float64),
    ]
}

/// In-representable-range samples per encoding: integral `f64` inside
/// the integer width, `f32`-backed values for Float32, and non-NaN
/// `f64` for Float64 (NaN cannot round-trip through `==`).
fn sample_strategy(enc: WavEncoding) -> BoxedStrategy<f64> {
    match enc {
        WavEncoding::PcmS16 => any::<i16>().prop_map(f64::from).boxed(),
        WavEncoding::PcmS24 => (-8_388_608i32..=8_388_607).prop_map(f64::from).boxed(),
        WavEncoding::PcmS32 => any::<i32>().prop_map(f64::from).boxed(),
        WavEncoding::Float32 => any::<f32>()
            .prop_filter("NaN cannot round-trip", |v| !v.is_nan())
            .prop_map(f64::from)
            .boxed(),
        WavEncoding::Float64 => any::<f64>()
            .prop_filter("NaN cannot round-trip", |v| !v.is_nan())
            .boxed(),
    }
}

/// `bits_per_sample` the writer declares for an encoding.
fn bits_of(enc: WavEncoding) -> u16 {
    match enc {
        WavEncoding::PcmS16 => 16,
        WavEncoding::PcmS24 => 24,
        WavEncoding::PcmS32 | WavEncoding::Float32 => 32,
        WavEncoding::Float64 => 64,
    }
}

/// `SampleFormat` the decoder reports for a writer encoding.
fn expected_format(enc: WavEncoding) -> SampleFormat {
    match enc {
        WavEncoding::PcmS16 => SampleFormat::PcmS16,
        WavEncoding::PcmS24 => SampleFormat::PcmS24,
        WavEncoding::PcmS32 => SampleFormat::PcmS32,
        WavEncoding::Float32 => SampleFormat::Float32,
        WavEncoding::Float64 => SampleFormat::Float64,
    }
}

/// Writer params, samples, and frame-aligned split points: encoding,
/// channels, a `byte_rate`-safe sample rate, total frames, interleaved
/// samples of `frames * channels` length, and up to two interior frame
/// boundaries that split `write_samples` into 1..=3 calls.
type WavCase = (WavEncoding, u16, u32, usize, Vec<f64>, Vec<usize>);

fn wav_case() -> impl Strategy<Value = WavCase> {
    (encoding_strategy(), 1u16..=8, 1u32..=RATE_CAP, 0usize..=72).prop_flat_map(
        |(enc, channels, sample_rate, frames)| {
            (
                Just(enc),
                Just(channels),
                Just(sample_rate),
                Just(frames),
                prop::collection::vec(sample_strategy(enc), frames * usize::from(channels)),
                prop::collection::vec(0usize..=frames, 0usize..=2),
            )
        },
    )
}

/// Small deterministic WAV from the product writer; mutation base for
/// the structured-header fuzz.
fn base_wav() -> Vec<u8> {
    let mut w =
        WavWriter::new(io::Cursor::new(Vec::new()), WavEncoding::PcmS16, 2, 48_000).unwrap();
    w.write_samples(&[1.0, -2.0, 3.0, -4.0]).unwrap();
    w.finish().unwrap().into_inner()
}

fn wav_with_terminal_chunk(
    declared_size: u32,
    body_len: usize,
    include_pad: bool,
    trailing: &[u8],
) -> Vec<u8> {
    let mut bytes = base_wav();
    bytes.extend_from_slice(b"junk");
    bytes.extend_from_slice(&declared_size.to_le_bytes());
    bytes.extend_from_slice(&vec![0x55; body_len]);
    if include_pad {
        bytes.push(0);
    }
    bytes.extend_from_slice(trailing);
    let riff = (bytes.len() - 8) as u32;
    bytes[4..8].copy_from_slice(&riff.to_le_bytes());
    bytes
}

#[test]
fn terminal_chunk_boundaries_follow_pad_and_exact_end_contract() {
    for (size, pad) in [(0u32, false), (2, false), (3, true)] {
        let bytes = wav_with_terminal_chunk(size, size as usize, pad, &[]);
        assert!(
            decode_wav(&bytes).is_ok(),
            "valid terminal chunk size={size} must decode"
        );
    }

    let missing_pad = wav_with_terminal_chunk(3, 3, false, &[]);
    assert!(decode_wav(&missing_pad).is_err());

    let extends = wav_with_terminal_chunk(8, 0, false, &[]);
    assert!(decode_wav(&extends).is_err());

    for trailing_len in 1..8usize {
        let trailing = vec![0xAA; trailing_len];
        let bytes = wav_with_terminal_chunk(0, 0, false, &trailing);
        assert!(
            decode_wav(&bytes).is_err(),
            "{trailing_len} trailing byte(s) must be rejected"
        );
    }
}

proptest! {
    /// The decode contract: `Ok` or `Err(MediaError)`, never a panic.
    /// A panic here is the bug this suite guards.
    #[test]
    fn decode_wav_never_panics_on_arbitrary_bytes(
        data in prop::collection::vec(any::<u8>(), 0usize..=4096),
    ) {
        prop_assert!(catch_unwind(AssertUnwindSafe(|| decode_wav(&data))).is_ok());
    }

    /// Header-only reads share the contract: when `wav_frame_count`
    /// counts a stream, the full range decodes and valid sub-ranges
    /// slice it exactly; invalid ranges must fail.
    #[test]
    fn frame_count_and_range_reads_hold_the_contract(
        data in prop::collection::vec(any::<u8>(), 0usize..=4096),
        start in 0u64..=64,
        end in 0u64..=64,
    ) {
        let fc = catch_unwind(AssertUnwindSafe(|| {
            wav_frame_count(&mut io::Cursor::new(&data))
        }));
        prop_assert!(fc.is_ok());
        let rr = catch_unwind(AssertUnwindSafe(|| {
            read_wav_range(&mut io::Cursor::new(&data), start, end)
        }));
        prop_assert!(rr.is_ok());

        let rr = rr.unwrap();
        match (fc.unwrap(), decode_wav(&data)) {
            // The same layout parse runs first in range reads, so a
            // failure there means every range fails too.
            (Err(_), _) => prop_assert!(rr.is_err()),
            // `wav_frame_count` only runs the layout parse: a file can
            // be countable yet not decodable (sample_layout checks the
            // decode path additionally). Reads share the decode path,
            // so every range must then fail — countability alone never
            // promises decodability.
            (Ok(_), Err(_)) => prop_assert!(rr.is_err()),
            // Countable and decodable: total frames agree with the full
            // decode, valid sub-ranges slice it exactly, invalid ranges
            // must fail.
            (Ok(total), Ok(full)) => {
                prop_assert_eq!(full.frames(), total);
                if start < end && end <= total {
                    let part = rr.unwrap();
                    prop_assert_eq!(part.frames(), end - start);
                    let ch = usize::from(full.channels);
                    prop_assert_eq!(
                        part.samples.as_slice(),
                        &full.samples[start as usize * ch..end as usize * ch]
                    );
                } else {
                    prop_assert!(rr.is_err());
                }
            }
        }
    }

    /// Writer output is a minimal RIFF/WAVE `decode_wav` must accept,
    /// and the round-trip is exact for representable input.
    #[test]
    fn writer_output_round_trips_losslessly(
        (enc, channels, sample_rate, frames, samples, splits) in wav_case(),
        start in 0u64..=150,
        end in 0u64..=150,
    ) {
        let mut w =
            WavWriter::new(io::Cursor::new(Vec::new()), enc, channels, sample_rate).unwrap();
        let ch = usize::from(channels);
        let mut bounds = splits;
        bounds.sort_unstable();
        bounds.dedup();
        let mut prev = 0usize;
        for b in bounds {
            w.write_samples(&samples[prev * ch..b * ch]).unwrap();
            prev = b;
        }
        w.write_samples(&samples[prev * ch..]).unwrap();
        let bytes = w.finish().unwrap().into_inner();

        let decoded = decode_wav(&bytes).unwrap();
        prop_assert_eq!(decoded.channels, channels);
        prop_assert_eq!(decoded.sample_rate, sample_rate);
        prop_assert_eq!(decoded.bits_per_sample, bits_of(enc));
        prop_assert_eq!(decoded.sample_format, expected_format(enc));
        prop_assert_eq!(decoded.frames(), frames as u64);
        prop_assert_eq!(decoded.samples.as_slice(), samples.as_slice());

        let mut c = io::Cursor::new(&bytes);
        prop_assert_eq!(wav_frame_count(&mut c).unwrap(), frames as u64);

        let full_range = read_wav_range(&mut c, 0, frames as u64);
        if frames == 0 {
            prop_assert!(full_range.is_err());
        } else {
            let full = full_range.unwrap();
            prop_assert_eq!(full.frames(), frames as u64);
            prop_assert_eq!(full.samples.as_slice(), samples.as_slice());
        }

        let range = read_wav_range(&mut c, start, end);
        if start < end && end <= frames as u64 {
            let part = range.unwrap();
            prop_assert_eq!(part.frames(), end - start);
            prop_assert_eq!(
                part.samples.as_slice(),
                &samples[start as usize * ch..end as usize * ch]
            );
        } else {
            prop_assert!(range.is_err());
        }
    }

    /// `WavWriter::new` fails exactly on the documented parameter
    /// violations; every other parameter set must construct.
    #[test]
    fn writer_new_rejects_exactly_the_documented_invalid_params(
        enc in encoding_strategy(),
        channels in any::<u16>(),
        sample_rate in any::<u32>(),
    ) {
        let res = WavWriter::new(io::Cursor::new(Vec::new()), enc, channels, sample_rate);
        let block_align = u64::from(channels) * u64::from(bits_of(enc) / 8);
        let valid = channels > 0
            && sample_rate > 0
            && block_align <= u64::from(u16::MAX)
            && u64::from(sample_rate) * block_align <= u64::from(u32::MAX);
        prop_assert_eq!(res.is_ok(), valid);
    }

    /// `write_samples` fails exactly when the length is not a whole
    /// number of frames.
    #[test]
    fn write_samples_rejects_exactly_unaligned_lengths(
        channels in 1u16..=8,
        len in 0usize..=64,
    ) {
        let mut w =
            WavWriter::new(io::Cursor::new(Vec::new()), WavEncoding::PcmS16, channels, 48_000)
                .unwrap();
        let res = w.write_samples(&vec![0.5f64; len]);
        prop_assert_eq!(res.is_err(), len % usize::from(channels) != 0);
    }

    /// Overwritten header fields and truncation never panic any of the
    /// three read entry points.
    #[test]
    fn mutated_or_truncated_headers_never_panic(
        edits in prop::collection::vec((0usize..=96, any::<u8>()), 0usize..=16),
        truncate_at in 0usize..=64,
        start in 0u64..=8,
        end in 0u64..=8,
    ) {
        let mut bytes = base_wav();
        for (at, v) in edits {
            let i = at % bytes.len();
            bytes[i] = v;
        }
        bytes.truncate(truncate_at.min(bytes.len()));
        let dec = catch_unwind(AssertUnwindSafe(|| decode_wav(&bytes)));
        prop_assert!(dec.is_ok());
        let fc = catch_unwind(AssertUnwindSafe(|| {
            wav_frame_count(&mut io::Cursor::new(&bytes))
        }));
        prop_assert!(fc.is_ok());
        let rr = catch_unwind(AssertUnwindSafe(|| {
            read_wav_range(&mut io::Cursor::new(&bytes), start, end)
        }));
        prop_assert!(rr.is_ok());
    }
}

/// Bounded proofs for `cargo kani` (`cargo kani -p al-media --tests`).
/// `cfg(kani)` is declared in the crate's check-cfg; plain `cargo
/// test`/`clippy` never compile this module.
///
/// Proof granularity (al#274): the original combined proofs were
/// solver-/envelope-bound at a 45min budget — one proof calling all
/// three read entry points, and one proof spanning all five
/// encodings. Each proof below covers exactly one entry point or one
/// encoding or one signature class, so the proof set as a whole
/// discharges the same contract while each individual formula stays
/// inside the lane budget.
/// Unwind bounds are sized to the worst-case loop trip count of the
/// code under proof (layout walk <= 5 chunks for a 48B input, sample
/// decode <= 28 iterations for a <=28B data body), not to a global
/// guess — CBMC unwinding checks are on, so a bound that is too small
/// fails loudly.
#[cfg(kani)]
mod kani_proofs {
    use super::*;

    /// Symbolic writer params inside the valid space: 1..=4 channels
    /// keeps `block_align` <= 32, so rates up to 192_000 can never
    /// overflow `byte_rate`.
    fn symbolic_params() -> (u16, u32) {
        let channels: u16 = kani::any();
        kani::assume((1..=4).contains(&channels));
        let sample_rate: u32 = kani::any();
        kani::assume((1..=192_000).contains(&sample_rate));
        (channels, sample_rate)
    }

    /// One symbolic in-range sample, mirroring `sample_strategy`.
    fn symbolic_sample(enc: WavEncoding) -> f64 {
        match enc {
            WavEncoding::PcmS16 => {
                let v: i16 = kani::any();
                f64::from(v)
            }
            WavEncoding::PcmS24 => {
                let v: i32 = kani::any();
                kani::assume((-8_388_608..=8_388_607).contains(&v));
                f64::from(v)
            }
            WavEncoding::PcmS32 => {
                let v: i32 = kani::any();
                f64::from(v)
            }
            WavEncoding::Float32 => {
                let v: f32 = kani::any();
                kani::assume(!v.is_nan());
                f64::from(v)
            }
            WavEncoding::Float64 => {
                let v: f64 = kani::any();
                kani::assume(!v.is_nan());
                v
            }
        }
    }

    /// A writer-produced WAV over symbolic params for one fixed
    /// encoding, with one frame of symbolic samples (symbolic `f64`
    /// conversion is the dominant verification cost; one frame still
    /// exercises the full write -> decode -> slice path).
    fn symbolic_wav(enc: WavEncoding) -> (Vec<u8>, u16, u32, Vec<f64>) {
        let (channels, sample_rate) = symbolic_params();
        let samples: Vec<f64> = (0..usize::from(channels))
            .map(|_| symbolic_sample(enc))
            .collect();
        let mut w =
            WavWriter::new(io::Cursor::new(Vec::new()), enc, channels, sample_rate).unwrap();
        w.write_samples(&samples).unwrap();
        (
            w.finish().unwrap().into_inner(),
            channels,
            sample_rate,
            samples,
        )
    }

    /// RIFF/WAVE signature test shared by the signature-gated splits:
    /// `wav_layout` rejects inputs failing it after a 12-byte read,
    /// so the signature is the natural input-domain boundary.
    fn wave_signed(data: &[u8; 48]) -> bool {
        &data[0..4] == b"RIFF" && &data[8..12] == b"WAVE"
    }

    /// Shared body of the unsigned shallow WAV probe. The representative
    /// harness and the temporary spike harnesses differ only in their
    /// per-harness stub attributes.
    fn wav_reads_never_panics_unsigned_body() {
        let data: [u8; 48] = kani::any();
        kani::assume(!wave_signed(&data));
        let _ = decode_wav(&data);
        let _ = wav_frame_count(&mut io::Cursor::new(&data[..]));
        let start = u64::from(kani::any::<u8>());
        let end = u64::from(kani::any::<u8>());
        let _ = read_wav_range(&mut io::Cursor::new(&data[..]), start, end);
    }

    /// Inputs WITHOUT the RIFF/WAVE signature: every read entry point
    /// must reject them without a panic. This is the shallow class —
    /// cheap to verify and disjoint from the three `*_signed` proofs,
    /// so together they cover the same input domain as the original
    /// single proof (al#274 input-domain split).
    // The proof discards every Result and never inspects MediaError::reason;
    // diagnostic text is outside the no-panic contract being proved.
    #[kani::proof]
    #[kani::unwind(16)]
    #[kani::stub(std::fmt::format, crate::kani_stub_models::fmt_format_empty)]
    fn wav_reads_never_panics_unsigned() {
        wav_reads_never_panics_unsigned_body();
    }

    /// RIFF/WAVE-signed inputs may never panic `decode_wav`. 48 bytes
    /// covers a complete minimal RIFF/WAVE header (44B) plus a few
    /// bytes of chunk data — enough to exercise every `wav_layout`
    /// branch and a few iterations of the sample loops, which
    /// `unwind(40)` covers with slack (the bound is a verification
    /// budget trade-off: arbitrary-byte decode is documented as
    /// bounded, not total).
    #[kani::proof]
    #[kani::unwind(40)]
    fn decode_wav_never_panics_signed() {
        let data: [u8; 48] = kani::any();
        kani::assume(wave_signed(&data));
        let _ = decode_wav(&data);
    }

    /// `wav_frame_count` on signed inputs shares the no-panic
    /// contract; it only runs the layout walk, so a smaller unwind
    /// suffices.
    #[kani::proof]
    #[kani::unwind(16)]
    fn wav_frame_count_never_panics_signed() {
        let data: [u8; 48] = kani::any();
        kani::assume(wave_signed(&data));
        let _ = wav_frame_count(&mut io::Cursor::new(&data[..]));
    }

    /// Shared body of the signed deep range-read probe, factored so the
    /// spike matrix compares the same formula apart from stub attributes.
    fn read_wav_range_never_panics_signed_body() {
        let data: [u8; 48] = kani::any();
        kani::assume(wave_signed(&data));
        let start = u64::from(kani::any::<u8>());
        let end = u64::from(kani::any::<u8>());
        let _ = read_wav_range(&mut io::Cursor::new(&data[..]), start, end);
    }

    /// `read_wav_range` on signed inputs. u8-derived bounds:
    /// every branch is decided by start<end / end<=total — values
    /// past total exit before the (end-start)*block multiply, so
    /// [0,255] is exhaustive over branch space while unconstrained
    /// u64 multiplies would dominate the solver.
    // As above, error text is diagnostic payload only; range success/failure
    // and memory safety do not depend on the formatted string.
    #[kani::proof]
    #[kani::unwind(40)]
    #[kani::stub(std::fmt::format, crate::kani_stub_models::fmt_format_empty)]
    fn read_wav_range_never_panics_signed() {
        read_wav_range_never_panics_signed_body();
    }

    // Temporary al#274 spike harnesses. They are intentionally not
    // registered in tools/run-formal-kani.mjs: they exist for the ablation
    // matrix and are not additional canonical proof responsibilities.
    // Explicit no-stub baselines preserve reproducibility now that the
    // canonical WAV harnesses carry the adopted fmt stub.

    #[kani::proof]
    #[kani::unwind(16)]
    fn spike_wav_unsigned_baseline() {
        wav_reads_never_panics_unsigned_body();
    }

    #[kani::proof]
    #[kani::unwind(40)]
    fn spike_wav_range_baseline() {
        read_wav_range_never_panics_signed_body();
    }

    #[kani::proof]
    #[kani::unwind(16)]
    #[kani::stub(std::fmt::format, crate::kani_stub_models::fmt_format_empty)]
    fn spike_wav_unsigned_fmt() {
        wav_reads_never_panics_unsigned_body();
    }

    #[kani::proof]
    #[kani::unwind(16)]
    #[kani::stub(
        std::alloc::handle_alloc_error,
        crate::kani_stub_models::handle_alloc_error_infallible
    )]
    fn spike_wav_unsigned_alloc() {
        wav_reads_never_panics_unsigned_body();
    }

    #[kani::proof]
    #[kani::unwind(16)]
    #[kani::stub(std::fmt::format, crate::kani_stub_models::fmt_format_empty)]
    #[kani::stub(
        std::alloc::handle_alloc_error,
        crate::kani_stub_models::handle_alloc_error_infallible
    )]
    fn spike_wav_unsigned_both() {
        wav_reads_never_panics_unsigned_body();
    }

    #[kani::proof]
    #[kani::unwind(40)]
    #[kani::stub(std::fmt::format, crate::kani_stub_models::fmt_format_empty)]
    fn spike_wav_range_fmt() {
        read_wav_range_never_panics_signed_body();
    }

    #[kani::proof]
    #[kani::unwind(40)]
    #[kani::stub(
        std::alloc::handle_alloc_error,
        crate::kani_stub_models::handle_alloc_error_infallible
    )]
    fn spike_wav_range_alloc() {
        read_wav_range_never_panics_signed_body();
    }

    #[kani::proof]
    #[kani::unwind(40)]
    #[kani::stub(std::fmt::format, crate::kani_stub_models::fmt_format_empty)]
    #[kani::stub(
        std::alloc::handle_alloc_error,
        crate::kani_stub_models::handle_alloc_error_infallible
    )]
    fn spike_wav_range_both() {
        read_wav_range_never_panics_signed_body();
    }

    /// Writer -> decoder round-trip is exact for representable input,
    /// proved per encoding. The integer encodings convert symbolic
    /// ints to `f64`; the float encodings carry the pricier symbolic
    /// float conversion in isolation.
    fn round_trip(enc: WavEncoding) {
        let (bytes, channels, sample_rate, samples) = symbolic_wav(enc);
        let decoded = decode_wav(&bytes).unwrap();
        assert_eq!(decoded.channels, channels);
        assert_eq!(decoded.sample_rate, sample_rate);
        assert_eq!(decoded.samples, samples);
    }

    #[kani::proof]
    #[kani::unwind(32)]
    fn writer_round_trip_pcm_s16() {
        round_trip(WavEncoding::PcmS16);
    }

    #[kani::proof]
    #[kani::unwind(32)]
    fn writer_round_trip_pcm_s24() {
        round_trip(WavEncoding::PcmS24);
    }

    #[kani::proof]
    #[kani::unwind(32)]
    fn writer_round_trip_pcm_s32() {
        round_trip(WavEncoding::PcmS32);
    }

    #[kani::proof]
    #[kani::unwind(32)]
    fn writer_round_trip_float32() {
        round_trip(WavEncoding::Float32);
    }

    #[kani::proof]
    #[kani::unwind(32)]
    fn writer_round_trip_float64() {
        round_trip(WavEncoding::Float64);
    }
}
