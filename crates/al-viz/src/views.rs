//! Structured views: transfer / locate / stereo / residual / event
//! products assembled from stored dataset measurements. Views keep
//! the source references so a consumer can trace every series back to
//! the measurement it came from.

use std::io;

use al_dataset::reader::DatasetReader;
use al_dataset::table::Cell;

use crate::{JsonProduct, TensorProduct};

/// dB floor for magnitude transforms.
const DB_FLOOR: f64 = 1e-12;

fn read_full(reader: &DatasetReader, role: &str, name: &str) -> io::Result<(Vec<u64>, Vec<f64>)> {
    let t = reader.open_tensor(role, name)?;
    let shape = t.meta().shape.clone();
    let region: Vec<[u64; 2]> = shape.iter().map(|&d| [0, d]).collect();
    let (data, _) = t.read_range(&region)?;
    Ok((shape, data))
}

fn invalid(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.into())
}

fn cell_str(c: &Cell) -> Option<String> {
    match c {
        Cell::Str(s) => Some(s.clone()),
        _ => None,
    }
}

fn cell_f64(c: &Cell) -> Option<f64> {
    match c {
        Cell::F64(v) => Some(*v),
        Cell::I64(v) => Some(*v as f64),
        _ => None,
    }
}

/// Transfer views: one JSON per `intermediate/<pair>` group that has
/// a `transfer_detail` tensor (rows: |H|, unwrapped phase, group
/// delay over the lower half of the spectrum).
pub fn transfer_views(reader: &DatasetReader, sample_rate: f64) -> io::Result<Vec<JsonProduct>> {
    let span = tracing::info_span!("viz::views", method = "transfer_views");
    let _enter = span.enter();
    let mut out = Vec::new();
    for a in reader.artifacts("intermediate") {
        if a.store != al_dataset::manifest::StoreKind::Tensor
            || !a.path.ends_with("/transfer_detail")
        {
            continue;
        }
        let name = a.path.strip_prefix("intermediate/").unwrap_or(&a.path);
        let pair = name.strip_suffix("/transfer_detail").unwrap_or(name);
        let (shape, data) = read_full(reader, "intermediate", name)?;
        if shape.len() != 2 {
            continue;
        }
        let bins = shape[1] as usize;
        let row = |r: usize| -> Vec<f64> { data[r * bins..(r + 1) * bins].to_vec() };
        let magnitude_db: Vec<f64> = row(0)
            .iter()
            .map(|&v| 20.0 * v.max(DB_FLOOR).log10())
            .collect();
        let bin_hz = sample_rate / (bins * 2) as f64;
        let freq_hz: Vec<f64> = (0..bins).map(|k| k as f64 * bin_hz).collect();

        // Optional companions: coherence magnitude + band gain.
        let coherence_path = format!("intermediate/{pair}/coherence");
        let coherence = read_full(reader, "intermediate", &format!("{pair}/coherence"))
            .ok()
            .map(|(s, d)| {
                // coherence rows are (magnitude, phase) over full bins —
                // keep the magnitude row's lower half.
                let half = bins;
                let w = s[1] as usize;
                d[..half.min(w)].to_vec()
            });
        let band_gain_path = format!("intermediate/{pair}/band_gain_m2");
        let band_gain_db = read_full(reader, "intermediate", &format!("{pair}/band_gain_m2")).ok();
        let mut parents = vec![a.path.clone()];
        if coherence.is_some() {
            parents.push(coherence_path);
        }
        if band_gain_db.is_some() {
            parents.push(band_gain_path);
        }

        out.push(JsonProduct {
            path: format!("visualization/views/transfer_{pair}.json"),
            parents,
            body: serde_json::json!({
                "pair": pair,
                "sample_rate": sample_rate,
                "freq_hz": freq_hz,
                "magnitude_db": magnitude_db,
                "phase_rad": row(1),
                "group_delay_s": row(2),
                "coherence": coherence,
                "band_gain": band_gain_db.map(|(s, d)| serde_json::json!({
                    "shape": s, "values": d,
                })),
                "source": format!("intermediate/{pair}/transfer_detail"),
            }),
        });
    }
    Ok(out)
}

/// Locate/time-map views: reshape each `locate` role JSON into a
/// display document (segments, occurrences, ranges, evidence list).
pub fn locate_views(reader: &DatasetReader) -> io::Result<Vec<JsonProduct>> {
    let span = tracing::info_span!("viz::views", method = "locate_views");
    let _enter = span.enter();
    let mut out = Vec::new();
    for a in reader.artifacts("locate") {
        if a.store != al_dataset::manifest::StoreKind::Json {
            continue;
        }
        let name = a
            .path
            .strip_prefix("locate/")
            .unwrap_or(&a.path)
            .strip_suffix(".json")
            .unwrap_or(&a.path);
        let body = reader.read_json("locate", name)?;
        let result = &body["result"];
        out.push(JsonProduct {
            path: format!("visualization/views/locate_{}.json", name.replace('/', "_")),
            body: serde_json::json!({
                "alignment_id": result["alignment_id"],
                "source_input_id": result["source_input_id"],
                "capture_input_id": result["capture_input_id"],
                "source_range": result["source_range"],
                "capture_range": result["capture_range"],
                "accepted": result["accepted"],
                "confidence": result["confidence"],
                "validity": result["validity"],
                "reject_reason": result["reject_reason"],
                "agreeing_methods": result["agreeing_methods"],
                "position_spread": result["position_spread"],
                "time_map": body["time_map"],
                "occurrences": body["occurrences"],
                "evidence": result["evidence"],
                "source": a.path,
            }),
            parents: vec![a.path.clone()],
        });
    }
    Ok(out)
}

/// Stereo/channel views: per-input JSON collecting the `stereo.*`
/// scalar series and references to the channel-pair coherence tiles.
pub fn stereo_views(reader: &DatasetReader) -> io::Result<Vec<JsonProduct>> {
    let span = tracing::info_span!("viz::views", method = "stereo_views");
    let _enter = span.enter();
    let mut out = Vec::new();
    for a in reader.artifacts("features/cheap") {
        if a.store != al_dataset::manifest::StoreKind::Json {
            continue;
        }
        let input = a
            .path
            .strip_prefix("features/cheap/")
            .unwrap_or(&a.path)
            .strip_suffix(".json")
            .unwrap_or(&a.path);
        let body = reader.read_json("features/cheap", input)?;
        let Some(records) = body["records"].as_array() else {
            continue;
        };
        let mut stereo: Vec<serde_json::Value> = Vec::new();
        for r in records {
            let Some(id) = r["feature_id"].as_str() else {
                continue;
            };
            if !id.starts_with("stereo.") {
                continue;
            }
            stereo.push(serde_json::json!({
                "feature_id": id,
                "value": r["value"],
                "unit": r["unit"],
                "time_range": r["time_range"],
                "confidence": r["confidence"],
                "validity": r["validity"],
            }));
        }
        if stereo.is_empty() {
            continue;
        }
        // The matching tile paths are deterministic transforms of the
        // `intermediate` coherence artifacts (tiles are written in the
        // same run, so they are not visible through the reader yet).
        let coherence_refs: Vec<String> = reader
            .artifacts("intermediate")
            .into_iter()
            .filter(|v| {
                v.store == al_dataset::manifest::StoreKind::Tensor
                    && v.path.starts_with(&format!("intermediate/{input}/"))
                    && v.path.ends_with("/coherence")
            })
            .map(|v| {
                v.path
                    .replacen("intermediate/", "visualization/coherence/", 1)
            })
            .collect();
        out.push(JsonProduct {
            path: format!("visualization/views/stereo_{input}.json"),
            body: serde_json::json!({
                "input_id": input,
                "features": stereo,
                "coherence_tiles": coherence_refs,
            }),
            parents: vec![a.path.clone()],
        });
    }
    Ok(out)
}

/// Video views: one JSON per input with materialized `video_features`
/// observations — decoded frame series plus preview thumbnails, or an
/// explicit `unavailable` record. The viewer renders the persisted
/// payload verbatim; no re-decode, no new analysis.
pub fn video_views(reader: &DatasetReader) -> io::Result<Vec<JsonProduct>> {
    let span = tracing::info_span!("viz::views", method = "video_views");
    let _enter = span.enter();
    let mut out = Vec::new();
    for a in reader.artifacts("inputs") {
        if a.store != al_dataset::manifest::StoreKind::Json {
            continue;
        }
        let rel = a.path.strip_prefix("inputs/").unwrap_or(&a.path);
        let Some(input) = rel.strip_suffix(".video_features.json") else {
            continue;
        };
        let features = reader.read_json("inputs", &format!("{input}.video_features"))?;
        // Thumbnails are emitted only for successfully decoded
        // streams; an `unavailable` feature record has no sibling.
        let thumbs_name = format!("{input}.video_thumbs");
        let thumbs = reader.read_json("inputs", &thumbs_name).ok();
        let mut parents = vec![a.path.clone()];
        if thumbs.is_some() {
            parents.push(format!("inputs/{thumbs_name}.json"));
        }
        out.push(JsonProduct {
            path: format!("visualization/views/video_{input}.json"),
            body: serde_json::json!({
                "input_id": features["input_id"],
                "features": features["features"],
                "thumbs": thumbs.map(|t| t["thumbs"].clone()),
                "source": a.path,
            }),
            parents,
        });
    }
    Ok(out)
}

/// Residual views: per residual stage name, index its artifacts
/// (waveform, PSD/band tensors) and the stored energies.
pub fn residual_views(reader: &DatasetReader) -> io::Result<Vec<JsonProduct>> {
    let span = tracing::info_span!("viz::views", method = "residual_views");
    let _enter = span.enter();
    let mut out = Vec::new();
    let mut stages: Vec<String> = reader
        .artifacts("residual")
        .into_iter()
        .filter_map(|a| {
            a.path
                .strip_prefix("residual/")
                .and_then(|p| p.split('/').next().map(str::to_string))
        })
        .collect();
    stages.sort();
    stages.dedup();
    for rname in stages {
        let artifacts: Vec<String> = reader
            .artifacts("residual")
            .into_iter()
            .filter(|a| a.path.starts_with(&format!("residual/{rname}/")))
            .map(|a| a.path.clone())
            .collect();
        out.push(JsonProduct {
            path: format!("visualization/views/residual_{rname}.json"),
            body: serde_json::json!({
                "residual": rname,
                "artifacts": artifacts,
            }),
            parents: artifacts.clone(),
        });
    }
    Ok(out)
}

/// Event markers: flatten the `events` table into a timeline-aligned
/// marker list keyed by the stable event id.
pub fn events_view(reader: &DatasetReader) -> io::Result<Option<JsonProduct>> {
    let span = tracing::info_span!("viz::views", method = "events_view");
    let _enter = span.enter();
    let table = match reader.open_table("events", "candidates") {
        Ok(t) => t,
        Err(_) => return Ok(None),
    };
    let rows = table.row_count();
    if rows == 0 {
        return Ok(Some(JsonProduct {
            path: "visualization/views/events.json".into(),
            body: serde_json::json!({"markers": []}),
            parents: vec!["events/candidates".into()],
        }));
    }
    // `read_range` is column-major: cols[col][row]. Resolve fields by
    // column name so a schema reorder never misaligns the markers.
    let cols = table.read_range(0, rows, None)?;
    let schema = table.schema();
    let idx = |name: &str| {
        schema
            .iter()
            .position(|c| c.name == name)
            .ok_or_else(|| invalid(format!("events table missing column {name}")))
    };
    let s = |i: usize, name: &str| -> io::Result<Option<String>> {
        Ok(cols
            .get(idx(name)?)
            .and_then(|c| c.get(i))
            .and_then(cell_str))
    };
    let f = |i: usize, name: &str| -> io::Result<Option<f64>> {
        Ok(cols
            .get(idx(name)?)
            .and_then(|c| c.get(i))
            .and_then(cell_f64))
    };
    let mut markers = Vec::with_capacity(rows as usize);
    for i in 0..rows as usize {
        markers.push(serde_json::json!({
            "event_id": s(i, "event_id")?,
            "kind": s(i, "kind")?,
            "input_id": s(i, "input_id")?,
            "start_s": f(i, "start_s")?,
            "end_s": f(i, "end_s")?,
            "strength": f(i, "strength")?,
            "confidence": s(i, "confidence")?,
            "validity": s(i, "validity")?,
            "source_feature": s(i, "source_feature")?,
            "freq_lo_hz": f(i, "freq_lo_hz")?,
            "freq_hi_hz": f(i, "freq_hi_hz")?,
            "channels": s(i, "channels")?,
            "algorithm_id": s(i, "algorithm_id")?,
            "algorithm_version": s(i, "algorithm_version")?,
            "evidence": s(i, "evidence")?,
            "params": s(i, "params")?,
        }));
    }
    Ok(Some(JsonProduct {
        path: "visualization/views/events.json".into(),
        body: serde_json::json!({"markers": markers}),
        parents: vec!["events/candidates".into()],
    }))
}

/// Tensor views: currently none — tiles carry the tensor products.
pub fn tensor_views(_reader: &DatasetReader) -> Vec<TensorProduct> {
    Vec::new()
}
