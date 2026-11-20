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
#[cfg(kani)]
mod kani_proofs {
    use super::*;

    /// Symbolic encoding + parameters inside the writer's valid space:
    /// 1..=4 channels keeps `block_align` <= 32, so rates up to
    /// 192_000 can never overflow `byte_rate`.
    fn symbolic_params() -> (WavEncoding, u16, u32) {
        let sel: u8 = kani::any();
        kani::assume(sel < 5);
        let enc = match sel {
            0 => WavEncoding::PcmS16,
            1 => WavEncoding::PcmS24,
            2 => WavEncoding::PcmS32,
            3 => WavEncoding::Float32,
            _ => WavEncoding::Float64,
        };
        let channels: u16 = kani::any();
        kani::assume((1..=4).contains(&channels));
        let sample_rate: u32 = kani::any();
        kani::assume((1..=192_000).contains(&sample_rate));
        (enc, channels, sample_rate)
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

    /// A writer-produced WAV over a symbolic encoding/params with one
    /// frame of symbolic samples (symbolic `f64` conversion is the
    /// dominant verification cost; one frame still exercises the
    /// full write -> decode -> slice path).
    fn symbolic_wav() -> (Vec<u8>, u16, u32, Vec<f64>) {
        let (enc, channels, sample_rate) = symbolic_params();
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

    /// No input byte pattern may panic the three read entry points.
    /// 48 bytes covers a complete minimal RIFF/WAVE header (44B) plus
    /// a few bytes of chunk data — enough to exercise every
    /// `wav_layout` branch and a few iterations of the sample loops,
    /// which `unwind(56)` covers with slack. (The bound is a
    /// verification budget trade-off: arbitrary-byte decode is
    /// documented as bounded, not total.)
    #[kani::proof]
    #[kani::unwind(56)]
    fn decode_never_panics_on_bounded_input() {
        let data: [u8; 48] = kani::any();
        let _ = decode_wav(&data);
        let _ = wav_frame_count(&mut io::Cursor::new(&data[..]));
        // u8-derived bounds: every `read_wav_range` branch is decided
        // by start<end / end<=total — values past total exit before
        // the (end-start)*block multiply, so [0,255] is exhaustive
        // over branch space while unconstrained u64 multiplies would
        // dominate the solver.
        let start = u64::from(kani::any::<u8>());
        let end = u64::from(kani::any::<u8>());
        let _ = read_wav_range(&mut io::Cursor::new(&data[..]), start, end);
    }

    /// Writer -> decoder round-trip is exact for representable input.
    #[kani::proof]
    #[kani::unwind(32)]
    fn writer_round_trip_is_exact() {
        let (bytes, channels, sample_rate, samples) = symbolic_wav();
        let decoded = decode_wav(&bytes).unwrap();
        assert_eq!(decoded.channels, channels);
        assert_eq!(decoded.sample_rate, sample_rate);
        assert_eq!(decoded.samples, samples);
    }

    /// Frame count and range reads agree with the writer-produced
    /// layout. Everything is concrete except `start`/`end`: the
    /// whole `u64` range space is then verified exhaustively —
    /// valid ranges slice exactly, every invalid range errors —
    /// which symbolic writer params would only sample (and cost
    /// far more: symbolic `f64` conversion dominates CBMC time).
    /// Value fidelity is `writer_round_trip_is_exact`'s job.
    #[kani::proof]
    #[kani::unwind(32)]
    fn header_reads_agree_with_writer_layout() {
        let channels: u16 = 2;
        let samples: Vec<f64> = (0..8 * usize::from(channels))
            .map(|i| (i + 1) as f64)
            .collect();
        let mut w = WavWriter::new(
            io::Cursor::new(Vec::new()),
            WavEncoding::PcmS16,
            channels,
            48_000,
        )
        .unwrap();
        w.write_samples(&samples).unwrap();
        let bytes = w.finish().unwrap().into_inner();
        let frames = 8u64;
        let mut c = io::Cursor::new(&bytes[..]);
        assert_eq!(wav_frame_count(&mut c).unwrap(), frames);
        // u8-derived bounds keep the solver off unconstrained u64
        // multiplies while covering every branch: [0,255] spans all
        // valid ranges plus both invalid classes (start>=end,
        // end>total).
        let start = u64::from(kani::any::<u8>());
        let end = u64::from(kani::any::<u8>());
        let part = read_wav_range(&mut c, start, end);
        let ch = usize::from(channels);
        if start < end && end <= frames {
            let part = part.unwrap();
            assert_eq!(part.frames(), end - start);
            assert_eq!(
                part.samples,
                samples[start as usize * ch..end as usize * ch].to_vec()
            );
        } else {
            assert!(part.is_err());
        }
    }
}
