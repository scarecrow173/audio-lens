//! Chunked N-dimensional f64 tensor store.
//!
//! Layout under one array root:
//!
//! ```text
//! array.json            dtype, shape, chunk_shape, order
//! chunks/<i0>_<i1>…<iN>.bin  raw little-endian f64, row-major
//! ```
//!
//! Edge chunks store only their real extent, so `shape` need not be a
//! multiple of `chunk_shape`. Reading a region touches only the chunk
//! files it overlaps — the full array is never materialized.

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use al_job::fsutil::{Durability, PublishMode, PublishOptions, publish_bytes};

/// Serialized array metadata (`array.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrayMeta {
    /// Element type (`f64` only today).
    pub dtype: String,
    /// Full array shape, one entry per dimension.
    pub shape: Vec<u64>,
    /// Chunk grid granularity.
    pub chunk_shape: Vec<u64>,
    /// Storage order; `row_major` (last dim fastest) only.
    pub order: String,
    /// Commit marker: `false` while a writer may still be writing
    /// chunks. Readers reject uncommitted arrays so a crashed writer
    /// can never pass a partially written tensor off as valid.
    #[serde(default)]
    pub committed: bool,
}

fn chunk_path(root: &Path, chunk_coords: &[u64]) -> PathBuf {
    let name = chunk_coords
        .iter()
        .map(u64::to_string)
        .collect::<Vec<_>>()
        .join("_");
    root.join("chunks").join(format!("{name}.bin"))
}

/// Extent of the chunk at `chunk_coords`.
fn chunk_extent(meta: &ArrayMeta, chunk_coords: &[u64]) -> Vec<u64> {
    chunk_coords
        .iter()
        .enumerate()
        .map(|(d, &c)| {
            meta.chunk_shape[d]
                .min(meta.shape[d].saturating_sub(c.saturating_mul(meta.chunk_shape[d])))
        })
        .collect()
}

/// Row-major strides for a shape.
fn strides(shape: &[u64]) -> Vec<u64> {
    let mut s = vec![1u64; shape.len()];
    for d in (0..shape.len().saturating_sub(1)).rev() {
        s[d] = s[d + 1] * shape[d + 1];
    }
    s
}

fn product(v: &[u64]) -> u64 {
    v.iter().product()
}

/// Advance `coords` through a `0..extent` box in row-major order.
/// Returns false when the box is exhausted.
fn increment(coords: &mut [u64], extent: &[u64]) -> bool {
    for d in (0..coords.len()).rev() {
        coords[d] += 1;
        if coords[d] < extent[d] {
            return true;
        }
        coords[d] = 0;
    }
    false
}

/// Advance `coords` through a `lo..=hi` box. Returns false on wrap.
fn increment_window(coords: &mut [u64], lo: &[u64], hi: &[u64]) -> bool {
    for d in (0..coords.len()).rev() {
        coords[d] += 1;
        if coords[d] <= hi[d] {
            return true;
        }
        coords[d] = lo[d];
    }
    false
}

fn bad_input(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, msg.into())
}

/// Writes one chunked tensor array.
pub struct TensorWriter {
    root: PathBuf,
    meta: ArrayMeta,
}

impl TensorWriter {
    /// Create a new array root. Fails if `root` already exists.
    pub fn create(root: &Path, shape: &[u64], chunk_shape: &[u64]) -> io::Result<Self> {
        if shape.is_empty() || shape.len() != chunk_shape.len() {
            return Err(bad_input("shape and chunk_shape must share ndim > 0"));
        }
        if shape.contains(&0) || chunk_shape.contains(&0) {
            return Err(bad_input("shape and chunk_shape entries must be > 0"));
        }
        fs::create_dir(root).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("create array root {}: {e}", root.display()),
            )
        })?;
        let w = Self {
            root: root.to_path_buf(),
            meta: ArrayMeta {
                dtype: "f64".into(),
                shape: shape.to_vec(),
                chunk_shape: chunk_shape.to_vec(),
                order: "row_major".into(),
                committed: false,
            },
        };
        w.flush_meta()?;
        Ok(w)
    }

    /// Write one whole chunk at `chunk_coords`. `data` length must
    /// equal the chunk extent (smaller at array edges).
    pub fn write_chunk(&mut self, chunk_coords: &[u64], data: &[f64]) -> io::Result<()> {
        let meta = &self.meta;
        if chunk_coords.len() != meta.shape.len() {
            return Err(bad_input("chunk coord ndim mismatch"));
        }
        for (d, &c) in chunk_coords.iter().enumerate() {
            let offset = c
                .checked_mul(meta.chunk_shape[d])
                .ok_or_else(|| bad_input(format!("chunk coord {chunk_coords:?} overflows")))?;
            if offset >= meta.shape[d] {
                return Err(bad_input(format!(
                    "chunk coord {chunk_coords:?} out of array"
                )));
            }
        }
        let extent = chunk_extent(meta, chunk_coords);
        if data.len() as u64 != product(&extent) {
            return Err(bad_input(format!(
                "chunk {chunk_coords:?} needs {} values, got {}",
                product(&extent),
                data.len()
            )));
        }
        let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        let path = chunk_path(&self.root, chunk_coords);
        fs::create_dir_all(path.parent().unwrap())?;
        // Relaxed tier: chunk payloads are regenerable bulk data;
        // array.json (durable, committed flag) is the completion
        // record.
        publish_bytes(
            &path,
            &bytes,
            PublishOptions::new(PublishMode::Replace, Durability::BestEffort),
        )?;
        Ok(())
    }

    /// Mark the array committed and re-write `array.json`. Call after
    /// the last `write_chunk`.
    pub fn finish(mut self) -> io::Result<()> {
        self.meta.committed = true;
        self.flush_meta()
    }

    fn flush_meta(&self) -> io::Result<()> {
        let json = serde_json::to_vec_pretty(&self.meta)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        publish_bytes(
            &self.root.join("array.json"),
            &json,
            PublishOptions::new(PublishMode::Replace, Durability::FileAndDirectorySync),
        )?;
        Ok(())
    }
}

/// Reads regions of a chunked tensor without loading the array.
pub struct TensorReader {
    root: PathBuf,
    meta: ArrayMeta,
}

impl TensorReader {
    /// Open an array root.
    pub fn open(root: &Path) -> io::Result<Self> {
        let meta: ArrayMeta = serde_json::from_slice(&fs::read(root.join("array.json"))?)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        if meta.dtype != "f64" || meta.order != "row_major" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported array meta {meta:?}"),
            ));
        }
        if !meta.committed {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "array.json is not committed (writer did not finish)",
            ));
        }
        // On-disk meta is untrusted: a shorter/zero chunk_shape would
        // otherwise panic inside read_range.
        if meta.shape.is_empty()
            || meta.shape.len() != meta.chunk_shape.len()
            || meta.shape.contains(&0)
            || meta.chunk_shape.contains(&0)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid array shape/chunk_shape {meta:?}"),
            ));
        }
        Ok(Self {
            root: root.to_path_buf(),
            meta,
        })
    }

    /// Array metadata.
    pub fn meta(&self) -> &ArrayMeta {
        &self.meta
    }

    /// Read the region given as per-dimension `[start, end)` bounds.
    /// Returns values in row-major order plus the region shape.
    pub fn read_range(&self, region: &[[u64; 2]]) -> io::Result<(Vec<f64>, Vec<u64>)> {
        let meta = &self.meta;
        if region.len() != meta.shape.len() {
            return Err(bad_input(format!(
                "region has {} dims, array has {}",
                region.len(),
                meta.shape.len()
            )));
        }
        for (d, &[start, end]) in region.iter().enumerate() {
            if start >= end || end > meta.shape[d] {
                return Err(bad_input(format!(
                    "region dim {d} [{start}, {end}) out of {}",
                    meta.shape[d]
                )));
            }
        }
        let region_shape: Vec<u64> = region.iter().map(|r| r[1] - r[0]).collect();
        let region_strides = strides(&region_shape);
        let mut out = vec![0.0; product(&region_shape) as usize];

        // Enumerate only the chunk coords the region overlaps.
        let ndim = meta.shape.len();
        let lo: Vec<u64> = region
            .iter()
            .enumerate()
            .map(|(d, r)| r[0] / meta.chunk_shape[d])
            .collect();
        let hi: Vec<u64> = region
            .iter()
            .enumerate()
            .map(|(d, r)| (r[1] - 1) / meta.chunk_shape[d])
            .collect();
        let mut coords = lo.clone();
        loop {
            let chunk_data = self.read_chunk_file(&coords)?;
            let extent = chunk_extent(meta, &coords);
            let ext_strides = strides(&extent);
            // Intersect the chunk volume with the requested region,
            // walking the innermost dim as whole "rows": when the
            // outer coords are inside the region and the row's
            // innermost extent is fully contained in the region's
            // innermost bound, the row is one contiguous span in
            // both layouts (innermost stride == 1) and copies as a
            // block. Partial containment falls back to the
            // per-element gather.
            let last = ndim - 1;
            let extent_last = extent[last];
            let g_lo = coords[last] * meta.chunk_shape[last];
            let g_hi = g_lo + extent_last;
            let row_fully_contained = g_lo >= region[last][0] && g_hi <= region[last][1];
            let mut local = vec![0u64; ndim];
            loop {
                // Row head: `local[last]` stays 0 while the outer
                // dims walk. `inside`/`src`/`dst` for the outer dims
                // are hoisted — the per-element check adds the
                // innermost dim on top.
                let mut inside = true;
                let mut dst = 0u64;
                let mut src = 0u64;
                for d in 0..last {
                    let global = coords[d] * meta.chunk_shape[d] + local[d];
                    if global < region[d][0] || global >= region[d][1] {
                        inside = false;
                        break;
                    }
                    src += local[d] * ext_strides[d];
                    dst += (global - region[d][0]) * region_strides[d];
                }
                if inside {
                    if row_fully_contained {
                        let src = src as usize;
                        let dst = (dst + (g_lo - region[last][0])) as usize;
                        let len = extent_last as usize;
                        out[dst..dst + len].copy_from_slice(&chunk_data[src..src + len]);
                    } else {
                        for l in 0..extent_last {
                            let global = g_lo + l;
                            if global >= region[last][0] && global < region[last][1] {
                                // Innermost stride is 1 in both
                                // layouts.
                                out[(dst + (global - region[last][0])) as usize] =
                                    chunk_data[(src + l) as usize];
                            }
                        }
                    }
                }
                if !increment(&mut local[..last], &extent[..last]) {
                    break;
                }
            }
            if !increment_window(&mut coords, &lo, &hi) {
                break;
            }
        }
        Ok((out, region_shape))
    }

    fn read_chunk_file(&self, coords: &[u64]) -> io::Result<Vec<f64>> {
        let path = chunk_path(&self.root, coords);
        let mut file = fs::File::open(&path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let expected = product(&chunk_extent(&self.meta, coords)) as usize * 8;
        if bytes.len() != expected {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "chunk {} has {} bytes, expected {expected}",
                    path.display(),
                    bytes.len()
                ),
            ));
        }
        Ok(bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|b| f64::from_le_bytes(*b))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_region_across_chunks() {
        let dir = std::env::temp_dir().join(format!("tns-{}", al_core::JobId::new()));
        fs::create_dir_all(&dir).unwrap();
        let root = dir.join("arr");
        // 6x5 array, 4x3 chunks → chunk grid 2x2, edge chunks included.
        let shape = [6u64, 5];
        let chunk = [4u64, 3];
        let full: Vec<f64> = (0..30).map(|i| i as f64).collect();
        let mut w = TensorWriter::create(&root, &shape, &chunk).unwrap();
        for c0 in 0..2 {
            for c1 in 0..2 {
                let coords = [c0, c1];
                let extent = [chunk[0].min(6 - c0 * 4), chunk[1].min(5 - c1 * 3)];
                let mut data = Vec::new();
                for r in 0..extent[0] {
                    for c in 0..extent[1] {
                        let g0 = c0 * 4 + r;
                        let g1 = c1 * 3 + c;
                        data.push(full[(g0 * 5 + g1) as usize]);
                    }
                }
                w.write_chunk(&coords, &data).unwrap();
            }
        }
        w.finish().unwrap();

        let r = TensorReader::open(&root).unwrap();
        // Region rows 2..5, cols 1..4 crosses all 4 chunks.
        let (vals, rshape) = r.read_range(&[[2, 5], [1, 4]]).unwrap();
        assert_eq!(rshape, vec![3, 3]);
        let expect: Vec<f64> = vec![11.0, 12.0, 13.0, 16.0, 17.0, 18.0, 21.0, 22.0, 23.0];
        assert_eq!(vals, expect);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_bad_inputs() {
        let dir = std::env::temp_dir().join(format!("tns-b-{}", al_core::JobId::new()));
        fs::create_dir_all(&dir).unwrap();
        assert!(TensorWriter::create(&dir.join("z"), &[0, 3], &[2, 2]).is_err());
        let mut w = TensorWriter::create(&dir.join("a"), &[4, 4], &[2, 2]).unwrap();
        assert!(w.write_chunk(&[0, 0], &[1.0]).is_err());
        assert!(w.write_chunk(&[5, 0], &[1.0; 4]).is_err());
        w.finish().unwrap();
        let r = TensorReader::open(&dir.join("a")).unwrap();
        assert!(r.read_range(&[[0, 9], [0, 4]]).is_err());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn corrupt_meta_and_chunks_fail_not_panic() {
        let dir = std::env::temp_dir().join(format!("tns-c-{}", al_core::JobId::new()));
        fs::create_dir_all(&dir).unwrap();
        let root = dir.join("arr");
        let mut w = TensorWriter::create(&root, &[4], &[2]).unwrap();
        w.write_chunk(&[0], &[1.0, 2.0]).unwrap();
        w.finish().unwrap();

        // Uncommitted: rewrite array.json without the commit flag.
        fs::write(
            root.join("array.json"),
            r#"{"dtype":"f64","shape":[4],"chunk_shape":[2],"order":"row_major","committed":false}"#,
        )
        .unwrap();
        assert!(TensorReader::open(&root).is_err());

        // Zero-length chunk_shape division would panic unguarded.
        fs::write(
            root.join("array.json"),
            r#"{"dtype":"f64","shape":[4],"chunk_shape":[0],"order":"row_major","committed":true}"#,
        )
        .unwrap();
        assert!(TensorReader::open(&root).is_err());

        // Valid meta, truncated chunk file → explicit error on read.
        fs::write(
            root.join("array.json"),
            r#"{"dtype":"f64","shape":[4],"chunk_shape":[2],"order":"row_major","committed":true}"#,
        )
        .unwrap();
        fs::write(root.join("chunks").join("0.bin"), [0u8; 8]).unwrap();
        let r = TensorReader::open(&root).unwrap();
        assert!(r.read_range(&[[0, 4]]).is_err());
        fs::remove_dir_all(&dir).ok();
    }

    // ==== mechanical-caller-cleanup characterization (A16d) ====
    // TASK-034EO484s25JM8bsb6zUCh (plan T1). Pins the CURRENT
    // `read_range` contract — the per-element inside-test that the
    // contiguous-row fast path must reproduce exactly — for a
    // fully-covered innermost dimension and for partial inner-dim
    // edges. `(values, region_shape)` equality is element/bit
    // exact (test-spec Rules C/D).

    /// Writes a fully committed 2-D array whose cell at global
    /// `(g0, g1)` is a deterministic non-linear function — no two
    /// cells share a value, so order is pinned too.
    fn write_grid(tag: &str, shape: [u64; 2], chunk: [u64; 2]) -> (PathBuf, Vec<f64>) {
        let dir = std::env::temp_dir().join(format!("tns-a16d-{tag}-{}", al_core::JobId::new()));
        fs::create_dir_all(&dir).unwrap();
        let root = dir.join("arr");
        let n = (shape[0] * shape[1]) as usize;
        let full: Vec<f64> = (0..n)
            .map(|i| {
                // Reversible deterministic bit pattern per index.
                let x = (i as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ 0xA5A5A5A5A5A5A5A5;
                f64::from_bits((x & !(0x7FFu64 << 52)) | (0x3FFu64 << 52))
            })
            .collect();
        let mut w = TensorWriter::create(&root, &shape, &chunk).unwrap();
        let nc0 = shape[0].div_ceil(chunk[0]);
        let nc1 = shape[1].div_ceil(chunk[1]);
        for c0 in 0..nc0 {
            for c1 in 0..nc1 {
                let extent = [
                    chunk[0].min(shape[0] - c0 * chunk[0]),
                    chunk[1].min(shape[1] - c1 * chunk[1]),
                ];
                let mut data = Vec::new();
                for r in 0..extent[0] {
                    for c in 0..extent[1] {
                        let g = ((c0 * chunk[0] + r) * shape[1] + (c1 * chunk[1] + c)) as usize;
                        data.push(full[g]);
                    }
                }
                w.write_chunk(&[c0, c1], &data).unwrap();
            }
        }
        w.finish().unwrap();
        (root, full)
    }

    /// Row-major oracle: enumerate the region volume last-dim-
    /// fastest and gather `full[global_linear]` for each point.
    /// Mirrors the documented output contract, not the code path.
    fn region_oracle(full: &[f64], shape: &[u64], region: &[[u64; 2]]) -> Vec<f64> {
        let ndim = shape.len();
        let shape_strides: Vec<u64> = {
            let mut s = vec![1u64; ndim];
            for d in (0..ndim - 1).rev() {
                s[d] = s[d + 1] * shape[d + 1];
            }
            s
        };
        let region_shape: Vec<u64> = region.iter().map(|r| r[1] - r[0]).collect();
        let count: u64 = region_shape.iter().product();
        let mut out = Vec::with_capacity(count as usize);
        for lin in 0..count {
            let mut rem = lin;
            let mut g = 0u64;
            for d in (0..ndim).rev() {
                let local = rem % region_shape[d];
                rem /= region_shape[d];
                g += (region[d][0] + local) * shape_strides[d];
            }
            out.push(full[g as usize]);
        }
        out
    }

    fn assert_region(
        reader: &TensorReader,
        full: &[f64],
        shape: &[u64],
        region: &[[u64; 2]],
        what: &str,
    ) {
        let (vals, rshape) = reader.read_range(region).unwrap();
        let expected_shape: Vec<u64> = region.iter().map(|r| r[1] - r[0]).collect();
        assert_eq!(rshape, expected_shape, "{what} region_shape");
        let expected = region_oracle(full, shape, region);
        assert_eq!(vals.len(), expected.len(), "{what} len");
        for (i, (&a, &e)) in vals.iter().zip(&expected).enumerate() {
            assert_eq!(a.to_bits(), e.to_bits(), "{what}[{i}]");
        }
    }

    /// Full innermost dimension: every row of the region is one
    /// contiguous span — the case the A16d fast path targets.
    #[test]
    fn a16d_read_range_full_inner_dim_is_bit_exact() {
        // 6x5 array, 4x3 chunks → 2x2 grid with edge chunks.
        let shape = [6u64, 5];
        let (root, full) = write_grid("full-inner", shape, [4, 3]);
        let r = TensorReader::open(&root).unwrap();
        for region in [
            // Whole array.
            [[0, 6], [0, 5]],
            // Full-width row band crossing the chunk boundary.
            [[2, 5], [0, 5]],
            // Single full-width row.
            [[3, 4], [0, 5]],
        ] {
            assert_region(&r, &full, &shape, &region, "full_inner");
        }
        fs::remove_dir_all(root.parent().unwrap()).ok();
    }

    /// Partial innermost-dimension edges: the region starts and/or
    /// ends mid-chunk in the last dim, so the inside-test rejects
    /// part of each touched chunk.
    #[test]
    fn a16d_read_range_partial_inner_dim_edge_is_bit_exact() {
        let shape = [6u64, 5];
        let (root, full) = write_grid("partial-inner", shape, [4, 3]);
        let r = TensorReader::open(&root).unwrap();
        for region in [
            // Suffix of chunk0 inner extent + prefix of chunk1.
            [[0, 6], [2, 4]],
            // Single column at a chunk inner edge.
            [[0, 6], [3, 4]],
            // Mid range spanning the boundary both dims.
            [[3, 6], [1, 4]],
            // One cell at the array's inner edge.
            [[5, 6], [4, 5]],
            // Prefix of the inner dim (left partial edge only).
            [[1, 3], [0, 2]],
        ] {
            assert_region(&r, &full, &shape, &region, "partial_inner");
        }
        fs::remove_dir_all(root.parent().unwrap()).ok();
    }
}
