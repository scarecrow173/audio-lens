//! Standalone HTML report for an export package.
//!
//! Emits two files at the package root:
//!
//! ```text
//! report.html      static viewer template (embedded via include_str!)
//! report.data.js   window.REPORT = {...} — views JSON verbatim plus
//!                  downsampled f64 tensors as base64
//! ```
//!
//! `report.data.js` is loaded with a plain `<script src>`, which is
//! the only mechanism that works when `report.html` is opened from a
//! `file://` URL — `fetch()` on file origins is CORS-blocked in every
//! major browser, so JSON/chunk files cannot be read at runtime.

use std::fs;
use std::io;
use std::path::Path;

use al_dataset::manifest::StoreKind;
use al_dataset::reader::DatasetReader;
use serde_json::{Map, Value, json};

fn invalid(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.into())
}

/// Static viewer template, compiled into the binary.
pub const REPORT_HTML: &str = include_str!("report.html");

/// Downsample caps keep `report.data.js` small enough for browsers to
/// parse comfortably. Raw tensors stay available under `analysis/`.
const MAX_TENSOR_ROWS: u64 = 192;
const MAX_TENSOR_COLS: u64 = 1024;

/// Read a visualization tensor, striding rows/cols so the result fits
/// `[MAX_TENSOR_ROWS, MAX_TENSOR_COLS]`. Returns the downsampled shape
/// plus row-major values.
fn read_downsampled(
    reader: &DatasetReader,
    name: &str,
) -> io::Result<(Vec<u64>, Vec<u64>, Vec<f64>)> {
    let tr = reader.open_tensor("visualization", name)?;
    let shape = tr.meta().shape.clone();
    if shape.len() > 2 {
        return Err(invalid(format!(
            "report: tensor {name} has {} dims (expected <= 2)",
            shape.len()
        )));
    }
    let (rows, cols) = (shape[0], *shape.get(1).unwrap_or(&1));
    let row_step = rows.div_ceil(MAX_TENSOR_ROWS).max(1);
    let col_step = cols.div_ceil(MAX_TENSOR_COLS).max(1);
    let out_rows = rows.div_ceil(row_step);
    let out_cols = cols.div_ceil(col_step);
    let mut data = Vec::with_capacity((out_rows * out_cols) as usize);
    for r in (0..rows).step_by(row_step as usize) {
        let (line, _) = if shape.len() == 2 {
            tr.read_range(&[[r, r + 1], [0, cols]])?
        } else {
            tr.read_range(&[[r, r + 1]])?
        };
        data.extend(line.iter().step_by(col_step as usize).copied());
    }
    Ok((vec![out_rows, out_cols], shape, data))
}

/// Collect every visualization JSON product and tensor, then write
/// `report.html` + `report.data.js` at the package root `dest`.
///
/// `dest` must be the package root: data is read from the *copied*
/// `dest/analysis` so the report reflects exactly what ships and input
/// names are already shrunk to file names.
pub fn generate(dest: &Path) -> io::Result<()> {
    let reader = DatasetReader::open(&dest.join("analysis"))?;
    let manifest = reader.manifest();

    let inputs: Vec<Value> = manifest
        .inputs
        .iter()
        .map(|i| {
            json!({
                "input_id": i.input_id,
                "role": i.role,
                "file_name": i.source_path
                    .rsplit(['/', '\\'])
                    .next()
                    .unwrap_or(&i.source_path),
            })
        })
        .collect();

    let mut summary = Value::Null;
    let mut timeline = Value::Null;
    let mut skipped = Value::Null;
    let mut views = Map::new();
    let mut tensors = Map::new();

    for a in manifest
        .artifacts
        .iter()
        .filter(|a| a.role == "visualization")
    {
        let name = a.path.strip_prefix("visualization/").unwrap_or(&a.path);
        match a.store {
            StoreKind::Json => {
                let body = reader.read_json("visualization", name)?;
                match name {
                    "summary.json" => summary = body,
                    "timeline.json" => timeline = body,
                    "skipped.json" => skipped = body,
                    _ => {
                        let stem = name.strip_suffix(".json").unwrap_or(name);
                        views.insert(stem.to_string(), body);
                    }
                }
            }
            StoreKind::Tensor => {
                if let Ok((shape, tr_shape, data)) = read_downsampled(&reader, name) {
                    // Viewers do not need f64 precision; f32 halves the
                    // embedded payload.
                    let bytes: Vec<u8> = data
                        .iter()
                        .flat_map(|v| (*v as f32).to_le_bytes())
                        .collect();
                    tensors.insert(
                        name.to_string(),
                        json!({
                            "shape": shape,
                            // original row count: needed to recover the
                            // true duration after row-stride downsampling
                            "src_shape": tr_shape,
                            "dtype": "f32",
                            "b64": al_core::b64::encode(&bytes),
                        }),
                    );
                }
            }
            _ => {}
        }
    }

    let report = json!({
        "generator": {"id": "al-viz", "version": env!("CARGO_PKG_VERSION")},
        "inputs": inputs,
        "summary": summary,
        "timeline": timeline,
        "skipped": skipped,
        "views": views,
        "tensors": tensors,
    });

    let mut js = String::with_capacity(64);
    js.push_str("window.REPORT=");
    js.push_str(&serde_json::to_string(&report).map_err(io::Error::other)?);
    js.push_str(";\n");

    fs::write(dest.join("report.data.js"), js)?;
    fs::write(dest.join("report.html"), REPORT_HTML)?;
    Ok(())
}
