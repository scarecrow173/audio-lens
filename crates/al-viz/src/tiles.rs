//! Spectrogram and band tiles: display-domain transforms of the
//! stored `intermediate`/`residual` tensors (dB mapping, validity
//! masks). Nothing is re-analyzed — products are reshaped copies of
//! stored measurements.

use std::io;

use al_dataset::reader::DatasetReader;

use crate::TensorProduct;

/// dB floor so log transforms stay finite on zero-energy bins.
const DB_FLOOR: f64 = 1e-12;

/// `20 * log10(v)` for amplitude-like tensors.
fn mag_db(v: f64) -> f64 {
    20.0 * v.max(DB_FLOOR).log10()
}

/// `10 * log10(v)` for power-like tensors.
fn pow_db(v: f64) -> f64 {
    10.0 * v.max(DB_FLOOR).log10()
}

/// Read a whole tensor artifact.
fn read_full(reader: &DatasetReader, role: &str, name: &str) -> io::Result<(Vec<u64>, Vec<f64>)> {
    let t = reader.open_tensor(role, name)?;
    let shape = t.meta().shape.clone();
    let region: Vec<[u64; 2]> = shape.iter().map(|&d| [0, d]).collect();
    let (data, _) = t.read_range(&region)?;
    Ok((shape, data))
}

/// Names of `intermediate` tensor artifacts whose last path segment
/// equals `leaf`, grouped as `(input_or_pair, name)`.
fn intermediate_named(reader: &DatasetReader, leaf: &str) -> Vec<String> {
    reader
        .artifacts("intermediate")
        .into_iter()
        .filter(|a| {
            a.store == al_dataset::manifest::StoreKind::Tensor
                && a.path.rsplit('/').next() == Some(leaf)
        })
        .map(|a| a.path.clone())
        .collect()
}

/// Build all tile products from whatever the dataset produced.
/// Callers decide skip/error policy per family via
/// `reader.role_produced`.
pub fn build(reader: &DatasetReader) -> io::Result<Vec<TensorProduct>> {
    let mut out = Vec::new();

    // Spectrogram tiles: dB magnitude per input channel.
    for path in intermediate_named(reader, "magnitude") {
        let name = path.strip_prefix("intermediate/").unwrap_or(&path);
        let (shape, data) = read_full(reader, "intermediate", name)?;
        out.push(TensorProduct {
            path: format!("visualization/spectrogram/{name}"),
            shape,
            data: data.iter().map(|&v| mag_db(v)).collect(),
        });
    }

    // Band heatmap: dB 1/3-octave power per input channel.
    for path in intermediate_named(reader, "third_octave") {
        let name = path.strip_prefix("intermediate/").unwrap_or(&path);
        let (shape, data) = read_full(reader, "intermediate", name)?;
        out.push(TensorProduct {
            path: format!("visualization/band/{name}"),
            shape,
            data: data.iter().map(|&v| pow_db(v)).collect(),
        });
    }

    // Coherence tiles pass through unchanged (already display-shaped).
    for path in intermediate_named(reader, "coherence") {
        let name = path.strip_prefix("intermediate/").unwrap_or(&path);
        let (shape, data) = read_full(reader, "intermediate", name)?;
        out.push(TensorProduct {
            path: format!("visualization/coherence/{name}"),
            shape,
            data,
        });
    }

    // dB difference spectrogram proxy: mean-PSD ratio per pair.
    for path in intermediate_named(reader, "psd_ratio") {
        let name = path.strip_prefix("intermediate/").unwrap_or(&path);
        let (shape, data) = read_full(reader, "intermediate", name)?;
        out.push(TensorProduct {
            path: format!("visualization/diff/{name}"),
            shape,
            data: data.iter().map(|&v| pow_db(v)).collect(),
        });
    }

    // Residual tiles: dB PSD and dB band energy per residual stage.
    for a in reader.artifacts("residual") {
        if a.store != al_dataset::manifest::StoreKind::Tensor {
            continue;
        }
        let name = a.path.strip_prefix("residual/").unwrap_or(&a.path);
        let leaf = name.rsplit('/').next().unwrap_or("");
        let map = match leaf {
            "psd" | "band_energy" => Some(pow_db as fn(f64) -> f64),
            _ => None,
        };
        if let Some(f) = map {
            let (shape, data) = read_full(reader, "residual", name)?;
            out.push(TensorProduct {
                path: format!("visualization/residual/{name}_db"),
                shape,
                data: data.iter().map(|&v| f(v)).collect(),
            });
        }
    }
    Ok(out)
}

/// Validity masks for spectrogram tiles: one `[frames]` series per
/// input channel, where a frame is `0` (invalid) when an invalid
/// cheap record's `time_range` covers it. Records without a time
/// range or with `Valid` validity do not mask.
pub fn validity_masks(reader: &DatasetReader, hop_s: f64) -> io::Result<Vec<TensorProduct>> {
    let mut out = Vec::new();
    for a in reader.artifacts("features/cheap") {
        if a.store != al_dataset::manifest::StoreKind::Json {
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
        // Mark ranges covered by any invalid record.
        let mut ranges: Vec<(u64, u64)> = Vec::new();
        for r in records {
            let valid = r["validity"].as_str() == Some("valid");
            let tr = r["time_range"].as_array();
            if valid || tr.is_none() {
                continue;
            }
            let tr = tr.unwrap();
            let (Ok(s), Ok(e)) = (tr[0].as_f64().ok_or(()), tr[1].as_f64().ok_or(())) else {
                continue;
            };
            let start = (s / hop_s).floor() as u64;
            let end = (e / hop_s).ceil() as u64;
            ranges.push((start, end.max(start + 1)));
        }
        if ranges.is_empty() {
            continue;
        }
        let frames = ranges.iter().map(|r| r.1).max().unwrap_or(0);
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
        });
    }
    Ok(out)
}
