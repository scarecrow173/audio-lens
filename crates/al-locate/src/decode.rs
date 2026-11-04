//! Shared decoded-PCM loader.
//!
//! Restores one job input's `decoded_pcm` artifact into
//! channel-deinterleaved, float-normalized samples plus the lineage
//! ids of the artifacts it came from. Sample format comes from the
//! sibling `ingest_metadata` artifact — decoded PCM keeps the
//! source-native scale, so the format decides normalization.
//!
//! Any stage that consumes decoded input uses this loader rather
//! than re-implementing artifact lookup and scaling.

use std::fs;
use std::io;
use std::path::Path;

use al_core::ArtifactId;
use al_job::fsutil::checked_job_path;
use al_job::job::JobInput;
use al_job::stage::StageContext;
use al_media::metadata::SampleFormat;

use crate::normalize::{NormalizedParams, PreparedInput, bad};
use crate::result::{LocateResult, PcmReference, SampleRange};

/// Split interleaved `samples` into per-channel vectors, dropping the
/// tail remainder that does not complete a frame. Empty input yields
/// empty channels. This is AudioLens input-boundary semantics and is
/// not a Ruviform primitive.
pub fn deinterleave(samples: &[f64], channels: u16) -> Vec<Vec<f64>> {
    let ch = usize::from(channels.max(1));
    let frames = samples.len() / ch;
    let mut out: Vec<Vec<f64>> = (0..ch).map(|_| Vec::with_capacity(frames)).collect();
    for (i, &v) in samples[..frames * ch].iter().enumerate() {
        out[i % ch].push(v);
    }
    out
}

/// normalized PCM または残差 signal version を artifact ID で解決し、配置と区間を検証する。
pub fn read_normalized_pcm(
    reference: &PcmReference,
    range: SampleRange,
    ctx: &StageContext<'_>,
) -> io::Result<PreparedInput> {
    let record = ctx
        .manifest
        .artifacts
        .iter()
        .find(|a| a.artifact_id == reference.artifact_id)
        .ok_or_else(|| bad("normalized_pcm reference missing from manifest"))?;
    let pcm = read_normalized_record(record, ctx)?;
    if reference.sample_rate != pcm.params.sample_rate || reference.channels != pcm.params.channels
    {
        return Err(bad("normalized_pcm reference rate/channels inconsistent"));
    }
    pcm.params.validate_range(range)?;
    Ok(pcm)
}

pub(crate) fn read_normalized_record(
    record: &al_job::manifest::ArtifactRecord,
    ctx: &StageContext<'_>,
) -> io::Result<PreparedInput> {
    if record.role == "residual" {
        return read_residual_record(record, ctx);
    }
    if record.role != "normalized_pcm" || record.produced_by != Some(al_core::StageKind::Locate) {
        return Err(bad("artifact is not locate normalized_pcm"));
    }
    let params: NormalizedParams = serde_json::from_value(record.params.clone()).map_err(bad)?;
    params.validate()?;
    let input = ctx
        .manifest
        .inputs
        .iter()
        .find(|i| i.input_id.as_str() == params.input_id)
        .ok_or_else(|| bad("normalized_pcm input missing"))?;
    let declared = ctx
        .manifest
        .input_artifact(&input.input_id)
        .ok_or_else(|| bad("normalized_pcm input artifact missing"))?;
    let originals: Vec<_> = record
        .inputs
        .iter()
        .map(|id| {
            ctx.manifest
                .artifacts
                .iter()
                .find(|a| &a.artifact_id == id)
                .ok_or_else(|| bad("normalized_pcm original lineage missing"))
        })
        .collect::<io::Result<_>>()?;
    if originals.iter().filter(|a| a.role == "decoded_pcm").count() != 1
        || originals
            .iter()
            .filter(|a| a.role == "ingest_metadata")
            .count()
            > 1
        || originals.iter().any(|a| {
            !a.inputs.contains(&declared)
                || !matches!(a.role.as_str(), "decoded_pcm" | "ingest_metadata")
        })
        || originals
            .iter()
            .filter(|a| a.role == "decoded_pcm")
            .any(|a| {
                a.params["sample_rate"].as_u64() != Some(params.original_sample_rate as u64)
                    || a.params["channels"].as_u64() != Some(params.channels as u64)
            })
    {
        return Err(bad("normalized_pcm original lineage inconsistent"));
    }
    let expected_path =
        std::path::PathBuf::from(format!("locate/normalized/{}.f64le", record.artifact_id));
    if record.path != expected_path {
        return Err(bad("normalized_pcm path inconsistent with artifact id"));
    }
    let view = PcmView::open(&checked_job_path(ctx.registry.job_dir(), &record.path)?)?;
    let length = params.frames as u128 * params.channels as u128 * 8;
    if view.byte_len() as u128 != length {
        return Err(bad(
            "normalized_pcm byte length inconsistent with frame layout",
        ));
    }
    let channel_samples = view.deinterleave(params.channels, 1.0);
    if channel_samples.iter().flatten().any(|s| !s.is_finite()) {
        return Err(bad("normalized_pcm has non-finite samples"));
    }
    Ok(PreparedInput {
        decoded: DecodedInput {
            input_id: params.input_id.clone(),
            sample_rate: params.sample_rate,
            channels: params.channels,
            channel_samples,
            artifact_ids: vec![record.artifact_id.clone()],
        },
        record: record.clone(),
        params,
    })
}

/// 減算を再実行せず、保存された残差 signal version を検証して復元する。
fn read_residual_record(
    record: &al_job::manifest::ArtifactRecord,
    ctx: &StageContext<'_>,
) -> io::Result<PreparedInput> {
    use crate::residual::ResidualPcmParams;
    let params: ResidualPcmParams = serde_json::from_value(record.params.clone()).map_err(bad)?;
    params.pcm.validate()?;
    let expected_path = std::path::PathBuf::from(format!(
        "locate/residual/{}_{}.wav",
        params.parent_source, record.artifact_id
    ));
    if record.produced_by != Some(al_core::StageKind::Locate)
        || params.encoding != "wav-f64le"
        || record.path != expected_path
        || record.inputs
            != vec![
                params.source_pcm.artifact_id.clone(),
                params.original_capture_pcm.artifact_id.clone(),
                params.parent_artifact.clone(),
            ]
    {
        return Err(bad("residual PCM layout or lineage inconsistent"));
    }
    // 親は必ず通常の normalized PCM。循環・残差の暗黙な再減算を拒否する。
    let resolve_original = |reference: &PcmReference| -> io::Result<PreparedInput> {
        let original = ctx
            .manifest
            .artifacts
            .iter()
            .find(|a| a.artifact_id == reference.artifact_id)
            .ok_or_else(|| bad("residual original PCM missing"))?;
        if original.role != "normalized_pcm" {
            return Err(bad("residual original PCM role inconsistent"));
        }
        read_normalized_pcm(reference, SampleRange { start: 0, end: 0 }, ctx)
    };
    let source = resolve_original(&params.source_pcm)?;
    let capture = resolve_original(&params.original_capture_pcm)?;
    let parent = ctx
        .manifest
        .artifacts
        .iter()
        .find(|a| a.artifact_id == params.parent_artifact)
        .ok_or_else(|| bad("residual parent alignment missing"))?;
    if parent.role != "locate_result" || parent.produced_by != Some(al_core::StageKind::Locate) {
        return Err(bad("residual parent alignment role inconsistent"));
    }
    let body: serde_json::Value = serde_json::from_slice(&fs::read(checked_job_path(
        ctx.registry.job_dir(),
        &parent.path,
    )?)?)
    .map_err(bad)?;
    let alignment: LocateResult = serde_json::from_value(body["result"].clone()).map_err(bad)?;
    if !alignment.accepted
        || alignment.alignment_id != params.parent_alignment
        || alignment.source_input_id != params.parent_source
        || alignment.source_pcm.as_ref() != Some(&params.source_pcm)
        || alignment.capture_pcm.as_ref() != Some(&params.original_capture_pcm)
        || !parent.inputs.contains(&params.source_pcm.artifact_id)
        || !parent
            .inputs
            .contains(&params.original_capture_pcm.artifact_id)
        || source.params.input_id != params.parent_source
        || alignment.capture_input_id != params.pcm.input_id
        || capture.params != params.pcm
        || params.channel_transform.len() != params.pcm.channels as usize
        || params.channel_transform.iter().any(|row| {
            row.len() != source.params.channels as usize || row.iter().any(|v| !v.is_finite())
        })
        || !params.energy.before.is_finite()
        || !params.energy.after.is_finite()
        || params.energy.before < 0.0
        || params.energy.after < 0.0
    {
        return Err(bad("residual subtraction lineage inconsistent"));
    }
    let audio = al_dataset::read_wav(&checked_job_path(ctx.registry.job_dir(), &record.path)?)?;
    if audio.sample_format != SampleFormat::Float64
        || audio.sample_rate != params.pcm.sample_rate
        || audio.channels != params.pcm.channels
        || audio.samples.len() as u128 != params.pcm.frames as u128 * params.pcm.channels as u128
        || audio.samples.iter().any(|v| !v.is_finite())
    {
        return Err(bad("residual WAV inconsistent with PCM layout"));
    }
    Ok(PreparedInput {
        decoded: DecodedInput {
            input_id: params.pcm.input_id.clone(),
            sample_rate: audio.sample_rate,
            channels: audio.channels,
            channel_samples: deinterleave(&audio.samples, audio.channels),
            artifact_ids: vec![record.artifact_id.clone()],
        },
        record: record.clone(),
        params: params.pcm,
    })
}

/// alignment 比較の共有読込境界。元 PCM への fallback は行わない。
pub fn read_alignment_pcm(
    result: &LocateResult,
    ctx: &StageContext<'_>,
) -> io::Result<Option<(PreparedInput, PreparedInput)>> {
    let (Some(src), Some(cap)) = (&result.source_pcm, &result.capture_pcm) else {
        let empty = SampleRange { start: 0, end: 0 };
        if !result.accepted
            && result.source_pcm.is_none()
            && result.capture_pcm.is_none()
            && result.source_range == empty
            && result.capture_range == empty
            && result.normalized_source_range == empty
            && matches!(
                result.validity,
                al_core::Validity::DecodeInputUnavailable | al_core::Validity::InsufficientSupport
            )
        {
            return Ok(None); // 未デコード・空入力の明示的な mark
        }
        return Err(bad("alignment missing normalized PCM references"));
    };
    let source = read_normalized_pcm(src, result.normalized_source_range, ctx)?;
    let capture = read_normalized_pcm(cap, result.capture_range, ctx)?;
    let residual_parent = &result.params["residual_parent_alignment"];
    if source.record.role != "normalized_pcm"
        || (capture.record.role == "residual"
            && (residual_parent.is_null()
                || residual_parent != &capture.record.params["parent_alignment"]))
        || (capture.record.role != "residual" && !residual_parent.is_null())
        || source.params.input_id != result.source_input_id
        || capture.params.input_id != result.capture_input_id
        || source.params.sample_rate != capture.params.sample_rate
        || capture.params.original_sample_rate != capture.params.sample_rate
        || result.params["normalization"] != source.record.params
        || source
            .params
            .original_range(result.normalized_source_range)?
            != result.source_range
        || (result.accepted
            && result.normalized_source_range.end - result.normalized_source_range.start
                < result.capture_range.end - result.capture_range.start)
    {
        return Err(bad("alignment PCM coordinates inconsistent"));
    }
    Ok(Some((source, capture)))
}

/// Read-only backing for an f64le PCM artifact: a memory map when the
/// platform provides one, otherwise an owned buffer from `fs::read`.
enum PcmBacking {
    Mapped(memmap2::Mmap),
    Owned(Vec<u8>),
}

impl PcmBacking {
    fn bytes(&self) -> &[u8] {
        match self {
            Self::Mapped(m) => m,
            Self::Owned(v) => v,
        }
    }
}

/// Read-only view over one f64le PCM artifact. The view is a derived
/// representation, never the source of truth — the manifest artifact
/// remains authoritative. Each call opens its own view; nothing here
/// shares decoded PCM across stages.
pub(crate) struct PcmView {
    backing: PcmBacking,
}

impl PcmView {
    /// Open `path` read-only. Memory-maps when the file is a non-empty
    /// regular file and mapping succeeds; falls back to an owned read
    /// otherwise. I/O errors from opening or reading propagate.
    fn open(path: &Path) -> io::Result<Self> {
        let file = fs::File::open(path)?;
        let meta = file.metadata()?;
        if meta.is_file() && meta.len() > 0 {
            // SAFETY: artifact files are immutable once the staged
            // publish lands them via atomic rename, and the manifest
            // only records an artifact after that point — no writer
            // mutates or truncates the file while this mapping is alive.
            if let Ok(mmap) = unsafe { memmap2::Mmap::map(&file) } {
                return Ok(Self {
                    backing: PcmBacking::Mapped(mmap),
                });
            }
        }
        Self::open_owned(path)
    }

    /// Open `path` as an owned buffer, bypassing mmap. Used as the
    /// fallback and by tests to force the non-mmap path.
    pub(crate) fn open_owned(path: &Path) -> io::Result<Self> {
        Ok(Self {
            backing: PcmBacking::Owned(fs::read(path)?),
        })
    }

    fn byte_len(&self) -> usize {
        self.backing.bytes().len()
    }

    fn sample_count(&self) -> usize {
        self.byte_len() / 8
    }

    fn sample(&self, index: usize) -> f64 {
        let offset = index * 8;
        f64::from_le_bytes(self.backing.bytes()[offset..offset + 8].try_into().unwrap())
    }

    /// Per-sample `scale` division fused with deinterleave, equivalent
    /// to collecting scaled `Vec<f64>` and then `dsp::deinterleave`:
    /// only complete frames are emitted (a trailing partial frame is
    /// dropped), `channels == 0` yields an empty vector, and channel
    /// order is preserved.
    fn deinterleave(&self, channels: u16, scale: f64) -> Vec<Vec<f64>> {
        let channels = usize::from(channels);
        if channels == 0 {
            return Vec::new();
        }
        let frames = self.sample_count() / channels;
        (0..channels)
            .map(|ch| {
                let mut out = Vec::with_capacity(frames);
                for f in 0..frames {
                    out.push(self.sample(f * channels + ch) / scale);
                }
                out
            })
            .collect()
    }
}

/// Decoded PCM of one input, with channel structure restored.
#[derive(Debug)]
pub struct DecodedInput {
    /// Manifest input id.
    pub input_id: String,
    /// Sample rate from the `decoded_pcm` artifact params.
    pub sample_rate: u32,
    /// Channel count from the `decoded_pcm` artifact params.
    pub channels: u16,
    /// Deinterleaved channels, normalized to float range.
    /// Always `channels` entries; each may be empty.
    pub channel_samples: Vec<Vec<f64>>,
    /// Artifact ids this input's PCM and metadata came from.
    pub artifact_ids: Vec<ArtifactId>,
}

/// Scale factor mapping one `SampleFormat` into `[-1, 1]` float.
/// Integer PCM is stored integer-scaled; spectral-envelope
/// comparisons and residual energies are absolute-scale-sensitive.
pub fn format_scale(format: SampleFormat) -> f64 {
    match format {
        SampleFormat::PcmU8 | SampleFormat::PcmS8 => 128.0,
        SampleFormat::PcmS16 => 32768.0,
        SampleFormat::PcmS24 => 8_388_608.0,
        SampleFormat::PcmS32 => 2_147_483_648.0,
        SampleFormat::Float32 | SampleFormat::Float64 | SampleFormat::Other => 1.0,
    }
}

/// Restore `input`'s decoded PCM. Returns `Ok(None)` when the input
/// has no `decoded_pcm` artifact (never ingested).
pub fn decode_input(
    input: &JobInput,
    ctx: &StageContext<'_>,
) -> Result<Option<DecodedInput>, io::Error> {
    let input_artifact = ctx.manifest.input_artifact(&input.input_id);
    let decoded = ctx.manifest.artifacts.iter().find(|a| {
        a.role == "decoded_pcm"
            && input_artifact
                .as_ref()
                .is_some_and(|ia| a.inputs.contains(ia))
    });
    let Some(decoded) = decoded else {
        return Ok(None);
    };
    let job_dir = ctx.registry.job_dir();
    fn bad(msg: impl Into<String>) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, msg.into())
    }
    let sample_rate = decoded.params["sample_rate"]
        .as_u64()
        .ok_or_else(|| bad("decoded_pcm artifact missing sample_rate"))?
        as u32;
    if sample_rate == 0 {
        return Err(bad("decoded_pcm artifact has sample_rate 0"));
    }
    let channels = decoded.params["channels"]
        .as_u64()
        .ok_or_else(|| bad("decoded_pcm artifact missing channels"))? as u16;
    if channels == 0 {
        return Err(bad("decoded_pcm artifact has 0 channels"));
    }

    let metadata = ctx.manifest.artifacts.iter().find(|a| {
        a.role == "ingest_metadata"
            && input_artifact
                .as_ref()
                .is_some_and(|ia| a.inputs.contains(ia))
    });
    let scale = match metadata {
        Some(meta) => {
            let text = fs::read_to_string(checked_job_path(job_dir, &meta.path)?)?;
            let json: serde_json::Value =
                serde_json::from_str(&text).map_err(|e| bad(e.to_string()))?;
            let format: SampleFormat = json
                .pointer("/audio/sample_format")
                .cloned()
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or(SampleFormat::Other);
            format_scale(format)
        }
        None => 1.0,
    };

    let view = PcmView::open(&checked_job_path(job_dir, &decoded.path)?)?;
    if view.byte_len() % 8 != 0 {
        return Err(bad(format!(
            "decoded_pcm has {} bytes, not a multiple of 8",
            view.byte_len()
        )));
    }
    let channel_samples = view.deinterleave(channels, scale);
    let mut artifact_ids = vec![decoded.artifact_id.clone()];
    if let Some(m) = metadata {
        artifact_ids.push(m.artifact_id.clone());
    }
    Ok(Some(DecodedInput {
        input_id: input.input_id.to_string(),
        sample_rate,
        channels,
        channel_samples,
        artifact_ids,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Instant;

    static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

    /// Write `samples` as little-endian f64 to a fresh temp file and
    /// return (dir, file) so callers can clean up.
    fn temp_f64le(name: &str, samples: &[f64]) -> (std::path::PathBuf, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "al_pcmview_{}_{}_{}",
            std::process::id(),
            name,
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{name}.f64le"));
        let mut file = fs::File::create(&path).unwrap();
        for s in samples {
            file.write_all(&s.to_le_bytes()).unwrap();
        }
        (dir, path)
    }

    /// Baseline oracle: `fs::read` → per-sample scale division →
    /// `deinterleave` — the pipeline the view replaces.
    fn oracle(path: &Path, channels: u16, scale: f64) -> Vec<Vec<f64>> {
        let bytes = fs::read(path).unwrap();
        let samples: Vec<f64> = bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|b| f64::from_le_bytes(*b) / scale)
            .collect();
        deinterleave(&samples, channels)
    }

    #[test]
    fn pcm_view_matches_oracle_across_scales_and_channels() {
        let cases: Vec<(u16, f64, Vec<f64>)> = vec![
            (1, 1.0, vec![0.5, -0.25, 1.0, -1.0, 0.0]),
            (2, 1.0, vec![0.1, -0.2, 0.3, -0.4, 0.5, -0.6, 0.7]),
            (
                2,
                32768.0,
                vec![16384.0, -8192.0, 100.5, -32768.0, 0.0, 1.0],
            ),
            (2, 128.0, vec![64.0, -128.0, 32.0, -16.0]),
            (3, 1.0, (0..10).map(|i| i as f64 * 0.25).collect()),
            (6, 1.0, (0..12).map(|i| i as f64).collect()),
        ];
        for (channels, scale, samples) in cases {
            let (dir, path) = temp_f64le("equiv", &samples);
            let expected = oracle(&path, channels, scale);
            for view in [
                PcmView::open(&path).unwrap(),
                PcmView::open_owned(&path).unwrap(),
            ] {
                assert_eq!(view.deinterleave(channels, scale), expected);
            }
            fs::remove_dir_all(&dir).unwrap();
        }
    }

    #[test]
    fn pcm_view_partial_frame_and_channel_edges() {
        // 3ch 10 samples → 3 complete frames; the trailing sample of the
        // partial fourth frame is dropped, matching `chunks_exact`.
        let samples: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let (dir, path) = temp_f64le("partial", &samples);
        let view = PcmView::open(&path).unwrap();
        assert_eq!(
            view.deinterleave(3, 1.0),
            vec![
                vec![0.0, 3.0, 6.0],
                vec![1.0, 4.0, 7.0],
                vec![2.0, 5.0, 8.0]
            ]
        );
        // channels == 0 yields an empty vector, like `dsp::deinterleave`.
        assert!(view.deinterleave(0, 1.0).is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn pcm_view_empty_file_falls_back_to_owned() {
        let (dir, path) = temp_f64le("empty", &[]);
        let view = PcmView::open(&path).unwrap();
        assert!(matches!(view.backing, PcmBacking::Owned(_)));
        assert_eq!(view.byte_len(), 0);
        let out = view.deinterleave(2, 1.0);
        assert_eq!(out, vec![Vec::<f64>::new(), Vec::<f64>::new()]);
        assert_eq!(oracle(&path, 2, 1.0), out);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn pcm_view_open_prefers_mmap_and_matches_owned() {
        let samples: Vec<f64> = (0..1024).map(|i| (i as f64 * 0.5).sin()).collect();
        let (dir, path) = temp_f64le("mapped", &samples);
        let mapped = PcmView::open(&path).unwrap();
        assert!(matches!(mapped.backing, PcmBacking::Mapped(_)));
        let owned = PcmView::open_owned(&path).unwrap();
        for ch in [1u16, 2, 3] {
            assert_eq!(mapped.deinterleave(ch, 1.0), owned.deinterleave(ch, 1.0));
        }
        assert_eq!(mapped.byte_len(), owned.byte_len());
        assert_eq!(mapped.sample(100), owned.sample(100));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn pcm_view_sample_floor_and_byte_len() {
        // 3 whole samples plus 4 trailing bytes → sample_count floors.
        let (dir, path) = temp_f64le("floor", &[1.0, 2.0, 3.0]);
        fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(&[0xAA; 4])
            .unwrap();
        let view = PcmView::open(&path).unwrap();
        assert_eq!(view.byte_len(), 28);
        assert_eq!(view.sample_count(), 3);
        assert_eq!(view.sample(0), 1.0);
        assert_eq!(view.sample(2), 3.0);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn pcm_view_missing_file_errors() {
        let missing = std::env::temp_dir().join("al_pcmview_missing_zzz.f64le");
        assert!(PcmView::open(&missing).is_err());
        assert!(PcmView::open_owned(&missing).is_err());
    }

    // --- Measurement harness (baseline vs mmap view vs Owned view) ---

    #[derive(Clone, Copy, Default, Debug)]
    struct Counts {
        calls: usize,
        reallocs: usize,
        bytes: usize,
        freed: usize,
    }

    thread_local! {
        static TRACK: Cell<(bool, Counts)> = const {
            Cell::new((
                false,
                Counts {
                    calls: 0,
                    reallocs: 0,
                    bytes: 0,
                    freed: 0,
                },
            ))
        };
    }

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

    fn track_start() {
        TRACK.with(|t| t.set((true, Counts::default())));
    }

    fn track_stop() -> Counts {
        TRACK.with(|t| {
            let (_, c) = t.get();
            t.set((false, c));
            c
        })
    }

    /// Pre-view pipeline: `fs::read` → per-sample scale division →
    /// `deinterleave`.
    fn baseline_materialize(path: &Path, channels: u16, scale: f64) -> Vec<Vec<f64>> {
        let bytes = fs::read(path).unwrap();
        let samples: Vec<f64> = bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|b| f64::from_le_bytes(*b) / scale)
            .collect();
        deinterleave(&samples, channels)
    }

    /// Wall-clock percentile over sorted microsecond samples.
    fn percentile(sorted: &[u128], pct: usize) -> u128 {
        let idx = (sorted.len() * pct).div_ceil(100) - 1;
        sorted[idx]
    }

    /// Best-effort RSS of the current process, in bytes.
    #[cfg(target_os = "linux")]
    fn rss_bytes() -> Option<u64> {
        let status = fs::read_to_string("/proc/self/status").ok()?;
        let kb: u64 = status
            .lines()
            .find(|l| l.starts_with("VmRSS"))?
            .split_whitespace()
            .nth(1)?
            .parse()
            .ok()?;
        Some(kb * 1024)
    }

    #[cfg(target_os = "windows")]
    fn rss_bytes() -> Option<u64> {
        // LIST format: the working-set line is the last line and is
        // locale-dependent, so parse digits from the final line.
        let out = std::process::Command::new("tasklist")
            .args([
                "/FI",
                &format!("PID eq {}", std::process::id()),
                "/FO",
                "LIST",
            ])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        let line = text.lines().rfind(|l| !l.trim().is_empty())?;
        let digits: String = line.chars().filter(|c| c.is_ascii_digit()).collect();
        digits.parse::<u64>().ok().map(|kb| kb * 1024)
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    fn rss_bytes() -> Option<u64> {
        None
    }

    fn assert_bit_identical(a: &[Vec<f64>], b: &[Vec<f64>]) {
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b) {
            assert_eq!(x.len(), y.len());
            for (u, v) in x.iter().zip(y) {
                assert_eq!(u.to_bits(), v.to_bits());
            }
        }
    }

    struct PathReport {
        name: &'static str,
        open_p50_us: u128,
        total_p50_us: u128,
        total_p95_us: u128,
        allocs: Counts,
        rss_before: Option<u64>,
        rss_after: Option<u64>,
    }

    /// Run the 3-path comparison on one fixture: bit-identical outputs,
    /// N timing iterations after warmup, allocator counts for one
    /// materialization, RSS before/after. Returns per-path reports.
    fn measure_three_paths(
        path: &Path,
        channels: u16,
        scale: f64,
        iters: usize,
    ) -> Vec<PathReport> {
        type Materialize = fn(&Path, u16, f64) -> Vec<Vec<f64>>;
        let mut reports = Vec::new();
        let mut expected: Option<Vec<Vec<f64>>> = None;
        let paths: [(&'static str, Materialize); 3] = [
            (
                "baseline(fs::read+collect+dsp::deinterleave)",
                baseline_materialize,
            ),
            ("mmap_view(PcmView::open+deinterleave)", |p, ch, s| {
                PcmView::open(p).unwrap().deinterleave(ch, s)
            }),
            (
                "owned_view(PcmView::open_owned+deinterleave)",
                |p, ch, s| PcmView::open_owned(p).unwrap().deinterleave(ch, s),
            ),
        ];
        for (name, run) in paths {
            let rss_before = rss_bytes();
            for _ in 0..3 {
                std::hint::black_box(run(path, channels, scale));
            }
            track_start();
            let first = run(path, channels, scale);
            let allocs = track_stop();
            if let Some(e) = &expected {
                assert_bit_identical(e, &first);
            } else {
                expected = Some(first);
            }
            let mut opens = Vec::with_capacity(iters);
            let mut totals = Vec::with_capacity(iters);
            for _ in 0..iters {
                if name.starts_with("baseline") {
                    let t = Instant::now();
                    std::hint::black_box(run(path, channels, scale));
                    totals.push(t.elapsed().as_micros());
                    opens.push(0u128);
                } else {
                    let t = Instant::now();
                    let view = if name.starts_with("mmap") {
                        PcmView::open(path).unwrap()
                    } else {
                        PcmView::open_owned(path).unwrap()
                    };
                    let open_elapsed = t.elapsed().as_micros();
                    let out = view.deinterleave(channels, scale);
                    totals.push(t.elapsed().as_micros());
                    opens.push(open_elapsed);
                    std::hint::black_box(&out);
                }
            }
            opens.sort_unstable();
            totals.sort_unstable();
            reports.push(PathReport {
                name,
                open_p50_us: percentile(&opens, 50),
                total_p50_us: percentile(&totals, 50),
                total_p95_us: percentile(&totals, 95),
                allocs,
                rss_before,
                rss_after: rss_bytes(),
            });
        }
        reports
    }

    fn write_evidence(dir: &Path, fixture: &Path, reports: &[PathReport]) -> std::path::PathBuf {
        let fixture_len = fs::metadata(fixture).map(|m| m.len()).unwrap_or(0);
        let mmap = &reports[1];
        let baseline = &reports[0];
        let open_share = if mmap.total_p50_us > 0 {
            mmap.open_p50_us as f64 / mmap.total_p50_us as f64
        } else {
            0.0
        };
        let mut report = String::new();
        report.push_str("# PCM view measurement evidence\n\n");
        report.push_str(&format!(
            "- env: {}-{} (test profile, debug build)\n",
            std::env::consts::OS,
            std::env::consts::ARCH
        ));
        report.push_str(&format!(
            "- fixture: {} ({} bytes, f64le interleaved)\n",
            fixture.display(),
            fixture_len
        ));
        report.push_str("- iterations: 30 timed after 3 warmup\n\n");
        report.push_str(
            "| path | open p50 (us) | total p50 (us) | total p95 (us) | alloc calls | alloc bytes | freed bytes | RSS before | RSS after |\n",
        );
        report.push_str("| --- | --- | --- | --- | --- | --- | --- | --- | --- |\n");
        for r in reports {
            report.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
                r.name,
                if r.name.starts_with("baseline") {
                    "-".to_string()
                } else {
                    r.open_p50_us.to_string()
                },
                r.total_p50_us,
                r.total_p95_us,
                r.allocs.calls,
                r.allocs.bytes,
                r.allocs.freed,
                r.rss_before
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "n/a".into()),
                r.rss_after
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "n/a".into()),
            ));
        }
        report.push_str(&format!(
            "\n- mmap open share of materialization p50: {:.2}%\n",
            open_share * 100.0
        ));
        report.push_str(&format!(
            "- mmap vs baseline p50 speedup: {:.2}x\n",
            baseline.total_p50_us as f64 / mmap.total_p50_us.max(1) as f64
        ));
        report.push_str("- 3-path channel_samples: bit-identical (asserted)\n");
        let out_path = dir.join(format!(
            "pcm-view-measurement-{}.md",
            fixture.file_stem().unwrap_or_default().to_string_lossy()
        ));
        fs::write(&out_path, report).unwrap();
        out_path
    }

    /// Generate a deterministic f64le stereo fixture of `frames` frames.
    fn write_stereo_fixture(path: &Path, frames: usize) {
        let mut x = 0x52_u64;
        let mut bytes = Vec::with_capacity(frames * 2 * 8);
        for _ in 0..frames * 2 {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
            let v = ((x >> 33) as f64 / (1_u64 << 31) as f64 - 0.5) * 0.4;
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        fs::File::create(path).unwrap().write_all(&bytes).unwrap();
    }

    fn evidence_dir() -> std::path::PathBuf {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/pcm-view-evidence");
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn pcm_view_measure_three_paths_ci_fixture() {
        // ~36 MB stereo fixture: CI-scale comparison run.
        let dir = evidence_dir();
        let fixture = dir.join("fixture-stereo-36mb.f64le");
        if !fixture.exists() {
            write_stereo_fixture(&fixture, 2_250_000);
        }
        let reports = measure_three_paths(&fixture, 2, 1.0, 30);
        let out = write_evidence(&dir, &fixture, &reports);
        eprintln!("pcm-view evidence written to {}", out.display());
    }

    #[test]
    #[ignore = "large real-size fixture; run explicitly for evidence"]
    fn pcm_view_measure_three_paths_large_fixture() {
        // ~368 MB stereo fixture: real-size comparison run.
        let dir = evidence_dir();
        let fixture = dir.join("fixture-stereo-368mb.f64le");
        if !fixture.exists() {
            write_stereo_fixture(&fixture, 23_000_000);
        }
        let reports = measure_three_paths(&fixture, 2, 1.0, 30);
        let out = write_evidence(&dir, &fixture, &reports);
        eprintln!("pcm-view evidence written to {}", out.display());
    }

    fn noise(seed: u64, n: usize) -> Vec<f64> {
        let mut s = seed;
        (0..n)
            .map(|_| {
                s = s
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ((s >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0
            })
            .collect()
    }

    #[test]
    fn deinterleave_splits_channels() {
        let interleaved = [1.0, 10.0, 2.0, 20.0, 3.0, 30.0];
        let ch = deinterleave(&interleaved, 2);
        assert_eq!(ch[0], vec![1.0, 2.0, 3.0]);
        assert_eq!(ch[1], vec![10.0, 20.0, 30.0]);
    }

    /// A15: `deinterleave` oracle — round-robin element order,
    /// remainder drop, `channels=0` clamp, and empty input.
    fn deinterleave_oracle(interleaved: &[f64], channels: u16) -> Vec<Vec<f64>> {
        let ch = usize::from(channels.max(1));
        let mut out = vec![Vec::new(); ch];
        for (i, &v) in interleaved.iter().enumerate() {
            if i % ch < ch {
                out[i % ch].push(v);
            }
        }
        // Drop tail remainder that does not complete a frame.
        let frames = interleaved.len() / ch;
        for c in &mut out {
            c.truncate(frames);
        }
        out
    }

    fn assert_deinterleave(interleaved: &[f64], channels: u16) {
        let expected = deinterleave_oracle(interleaved, channels);
        assert_eq!(deinterleave(interleaved, channels), expected);
    }

    #[test]
    fn a15_deinterleave_element_order_and_edges() {
        // Round-robin order across many channel counts.
        for ch in [1u16, 2, 3, 6] {
            let n = ch as usize * 64;
            let x = noise(50 + u64::from(ch), n);
            assert_deinterleave(&x, ch);
        }
        // Tail remainder is dropped.
        for ch in [2u16, 3] {
            let n = ch as usize * 10 + (ch as usize - 1);
            let x = noise(60 + u64::from(ch), n);
            assert_deinterleave(&x, ch);
        }
        // channels=0 clamps to 1; empty input yields one empty channel.
        assert_eq!(deinterleave(&[], 0), vec![Vec::<f64>::new()]);
        assert_eq!(deinterleave(&[1.0, 2.0, 3.0], 0)[0], vec![1.0, 2.0, 3.0]);
        // Non-power-of-two channel counts.
        let x = noise(61, 3 * 128);
        assert_deinterleave(&x, 3);
        let x = noise(62, 6 * 100);
        assert_deinterleave(&x, 6);
    }

    #[test]
    fn a15_deinterleave_byte_layout_roundtrip() {
        // interleaved[c * frames + f] must land in out[c][f] — the
        // layout decode.rs' readers materialize.
        let channels = 2u16;
        let frames = 512usize;
        let x = noise(70, channels as usize * frames);
        let out = deinterleave(&x, channels);
        for c in 0..channels as usize {
            for f in 0..frames {
                assert_eq!(out[c][f], x[c + f * channels as usize]);
            }
        }
    }
}
