//! Spectrogram and band tiles: display-domain transforms of the
//! stored `intermediate`/`residual` tensors (dB mapping, validity
//! masks). Nothing is re-analyzed — products are reshaped copies of
//! stored measurements, emitted chunk-by-chunk so long captures stay
//! range-readable and never load whole tensors into memory.

use std::io;

use al_dataset::manifest::StoreKind;
use al_dataset::reader::DatasetReader;
use al_dataset::writer::TensorChunk;

use crate::{StreamedTensor, TensorProduct};

/// dB floor so log transforms stay finite on zero-energy bins.
const DB_FLOOR: f64 = 1e-12;

/// Elements per visualization tensor chunk (~1 MiB of f64): the
/// leading time dim splits so ranged display reads stay bounded on
/// long captures.
const TILE_CHUNK_ELEMS: u64 = 1 << 17;

/// `20 * log10(v)` for amplitude-like tensors.
fn mag_db(v: f64) -> f64 {
    20.0 * v.max(DB_FLOOR).log10()
}

/// `10 * log10(v)` for power-like tensors.
fn pow_db(v: f64) -> f64 {
    10.0 * v.max(DB_FLOOR).log10()
}

/// Identity transform for tensors already in display units.
fn passthrough(v: f64) -> f64 {
    v
}

/// Chunk grid for a `[time, freq, …]` product: inner dims stay whole
/// and the leading (time) dim splits so no chunk exceeds
/// `TILE_CHUNK_ELEMS` elements.
pub(crate) fn chunk_shape_for(shape: &[u64]) -> Vec<u64> {
    let inner: u64 = shape[1..].iter().product::<u64>().max(1);
    let mut chunk_shape = shape.to_vec();
    chunk_shape[0] = (TILE_CHUNK_ELEMS / inner).clamp(1, shape[0]);
    chunk_shape
}

/// Lazily transformed copy of a stored tensor: each output chunk is
/// produced from a ranged read of the source, so the full tensor is
/// never expanded in memory.
fn stream_tile(
    reader: &DatasetReader,
    role: &str,
    artifact_path: &str,
    out_path: String,
    map: fn(f64) -> f64,
) -> io::Result<StreamedTensor> {
    let name = artifact_path
        .strip_prefix(&format!("{role}/"))
        .unwrap_or(artifact_path);
    let tensor = reader.open_tensor(role, name)?;
    let shape = tensor.meta().shape.clone();
    let chunk_shape = chunk_shape_for(&shape);
    let rows_per_chunk = chunk_shape[0];
    let frames = shape[0];
    let n_chunks = frames.div_ceil(rows_per_chunk);
    let inner: Vec<u64> = shape[1..].to_vec();
    let ndim = shape.len();
    let mut idx = 0u64;
    let chunks = std::iter::from_fn(move || {
        if idx >= n_chunks {
            return None;
        }
        let start = idx * rows_per_chunk;
        let end = (start + rows_per_chunk).min(frames);
        let mut region = Vec::with_capacity(ndim);
        region.push([start, end]);
        region.extend(inner.iter().map(|&d| [0, d]));
        let mut coords = vec![0u64; ndim];
        coords[0] = idx;
        idx += 1;
        Some(tensor.read_range(&region).map(|(data, _)| TensorChunk {
            coords,
            data: data.iter().map(|&v| map(v)).collect(),
        }))
    });
    Ok(StreamedTensor {
        path: out_path,
        shape,
        chunk_shape,
        parents: vec![artifact_path.to_string()],
        chunks: Box::new(chunks),
    })
}

/// Names of `intermediate` tensor artifacts whose last path segment
/// equals `leaf`, grouped as `(input_or_pair, name)`.
fn intermediate_named(reader: &DatasetReader, leaf: &str) -> Vec<String> {
    reader
        .artifacts("intermediate")
        .into_iter()
        .filter(|a| a.store == StoreKind::Tensor && a.path.rsplit('/').next() == Some(leaf))
        .map(|a| a.path.clone())
        .collect()
}

/// Build all tile products from whatever the dataset produced.
/// Callers decide skip/error policy per family via
/// `reader.role_produced`.
pub fn build(reader: &DatasetReader) -> io::Result<Vec<StreamedTensor>> {
    let span = tracing::info_span!("viz::tiles", method = "spectrogram");
    let _enter = span.enter();
    let mut out = Vec::new();

    // Spectrogram tiles: dB magnitude per input channel.
    for path in intermediate_named(reader, "magnitude") {
        let name = path.strip_prefix("intermediate/").unwrap_or(&path);
        out.push(stream_tile(
            reader,
            "intermediate",
            &path,
            format!("visualization/spectrogram/{name}"),
            mag_db,
        )?);
    }

    // Band heatmap: dB 1/3-octave power per input channel.
    for path in intermediate_named(reader, "third_octave") {
        let name = path.strip_prefix("intermediate/").unwrap_or(&path);
        out.push(stream_tile(
            reader,
            "intermediate",
            &path,
            format!("visualization/band/{name}"),
            pow_db,
        )?);
    }

    // Coherence tiles pass through unchanged (already display-shaped).
    for path in intermediate_named(reader, "coherence") {
        let name = path.strip_prefix("intermediate/").unwrap_or(&path);
        out.push(stream_tile(
            reader,
            "intermediate",
            &path,
            format!("visualization/coherence/{name}"),
            passthrough,
        )?);
    }

    // dB difference spectrogram proxy: mean-PSD ratio per pair.
    for path in intermediate_named(reader, "psd_ratio") {
        let name = path.strip_prefix("intermediate/").unwrap_or(&path);
        out.push(stream_tile(
            reader,
            "intermediate",
            &path,
            format!("visualization/diff/{name}"),
            pow_db,
        )?);
    }

    // Residual tiles: dB PSD and dB band energy per residual stage.
    for a in reader.artifacts("residual") {
        if a.store != StoreKind::Tensor {
            continue;
        }
        let name = a.path.strip_prefix("residual/").unwrap_or(&a.path);
        let leaf = name.rsplit('/').next().unwrap_or("");
        let map = match leaf {
            "psd" | "band_energy" => Some(pow_db as fn(f64) -> f64),
            _ => None,
        };
        if let Some(f) = map {
            out.push(stream_tile(
                reader,
                "residual",
                &a.path,
                format!("visualization/residual/{name}_db"),
                f,
            )?);
        }
    }
    Ok(out)
}

/// Validity masks for spectrogram tiles: one `[frames]` series per
/// input channel, where a frame is `0` (invalid) when an invalid
/// cheap record's `time_range` covers it. Records without a time
/// range or with `Valid` validity do not mask. The mask spans the
/// companion spectrogram's whole frame count — trailing frames after
/// the last invalid range stay `1`.
pub fn validity_masks(reader: &DatasetReader, hop_s: f64) -> io::Result<Vec<TensorProduct>> {
    let span = tracing::info_span!("viz::tiles", method = "validity_masks");
    let _enter = span.enter();
    let mut out = Vec::new();
    for a in reader.artifacts("features/cheap") {
        if a.store != StoreKind::Json {
            continue;
        }
        let name = a
            .path
            .strip_prefix("features/cheap/")
            .unwrap_or(&a.path)
            .strip_suffix(".json")
            .unwrap_or(&a.path);
        let body = reader.read_json("features/cheap", name)?;
        let Some(records) = body["records"].as_array() else {
            continue;
        };
        // Mark ranges covered by any invalid record. `time_range` is
        // a Range object `{start, end}` in seconds.
        let mut ranges: Vec<(u64, u64)> = Vec::new();
        for r in records {
            if r["validity"].as_str() == Some("valid") {
                continue;
            }
            let tr = &r["time_range"];
            let (Some(s), Some(e)) = (tr["start"].as_f64(), tr["end"].as_f64()) else {
                continue;
            };
            let start = (s / hop_s).floor() as u64;
            let end = (e / hop_s).ceil() as u64;
            ranges.push((start, end.max(start + 1)));
        }
        if ranges.is_empty() {
            continue;
        }
        let mut parents = vec![a.path.clone()];
        // The mask must cover every spectrogram frame: frames after
        // the last invalid range are valid and need an explicit `1`.
        let mut frames = ranges.iter().map(|r| r.1).max().unwrap_or(0);
        if let Some(mag) = reader.artifacts("intermediate").into_iter().find(|t| {
            t.store == StoreKind::Tensor
                && t.path.starts_with(&format!("intermediate/{name}/"))
                && t.path.ends_with("/magnitude")
        }) {
            let tname = mag.path.strip_prefix("intermediate/").unwrap_or(&mag.path);
            frames = frames.max(reader.open_tensor("intermediate", tname)?.meta().shape[0]);
            parents.push(mag.path.clone());
        }
        let mut mask = vec![1.0; frames as usize];
        for (s, e) in ranges {
            for m in mask
                .iter_mut()
                .take(e.min(frames) as usize)
                .skip(s as usize)
            {
                *m = 0.0;
            }
        }
        out.push(TensorProduct {
            path: format!("visualization/spectrogram/{name}/mask"),
            shape: vec![frames],
            data: mask,
            parents,
        });
    }
    Ok(out)
}
