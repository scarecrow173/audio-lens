//! Waveform LOD pyramid: per-channel min/max/RMS series at increasing
//! decimation, built from range reads — the full PCM is never held in
//! memory at once.

use std::io;

use al_dataset::reader::DatasetReader;

use crate::TensorProduct;
use crate::timeline::{LOD_BASE_BUCKET, LOD_LEVELS};

/// Frames decoded per range-read call.
const READ_BLOCK: u64 = 1 << 16;

/// One LOD bucket.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct Bucket {
    /// Minimum sample in the bucket.
    pub min: f64,
    /// Maximum sample in the bucket.
    pub max: f64,
    /// Root-mean-square of the bucket.
    pub rms: f64,
    /// Samples actually covered (last bucket may be partial).
    pub count: u64,
}

/// Running accumulator for one base bucket.
#[derive(Default, Clone, Copy)]
struct Acc {
    min: f64,
    max: f64,
    sum_sq: f64,
    count: u64,
}

impl Acc {
    fn push(&mut self, v: f64) {
        if self.count == 0 {
            self.min = v;
            self.max = v;
        } else {
            self.min = self.min.min(v);
            self.max = self.max.max(v);
        }
        self.sum_sq += v * v;
        self.count += 1;
    }

    fn bucket(&self) -> Bucket {
        Bucket {
            min: self.min,
            max: self.max,
            rms: if self.count == 0 {
                f64::NAN
            } else {
                (self.sum_sq / self.count as f64).sqrt()
            },
            count: self.count,
        }
    }

    /// Fold `other` into `self`.
    fn merge(&mut self, other: &Acc) {
        if other.count == 0 {
            return;
        }
        if self.count == 0 {
            *self = *other;
        } else {
            self.min = self.min.min(other.min);
            self.max = self.max.max(other.max);
            self.sum_sq += other.sum_sq;
            self.count += other.count;
        }
    }
}

/// The full pyramid for one input: `levels[i]` holds one channel-list
/// of bucket series at `LOD_BASE_BUCKET << i` samples per bucket.
pub struct Pyramid {
    /// Channel count.
    pub channels: usize,
    /// `levels[level][channel][bucket]`.
    pub levels: Vec<Vec<Vec<Bucket>>>,
}

/// Build the pyramid for `inputs/<input_id>` via chunked range reads.
pub fn build(reader: &DatasetReader, input_id: &str) -> io::Result<Pyramid> {
    let span = tracing::info_span!(
        "viz::lod",
        input_id = %input_id,
        len = tracing::field::Empty
    );
    let _enter = span.enter();
    let frames = reader.audio_frames("inputs", input_id)?;
    span.record("len", frames);
    if frames == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("inputs/{input_id} has no PCM frames"),
        ));
    }
    // Base buckets at LOD_BASE_BUCKET samples per channel.
    let n_base = frames.div_ceil(LOD_BASE_BUCKET) as usize;
    let head = reader.read_audio_range("inputs", input_id, 0, 1)?;
    let channels = head.channels as usize;

    // Accumulate base-level buckets per channel.
    let mut base: Vec<Vec<Acc>> = vec![vec![Acc::default(); n_base]; channels];
    let mut start = 0u64;
    while start < frames {
        let end = (start + READ_BLOCK).min(frames);
        let block = reader.read_audio_range("inputs", input_id, start, end)?;
        let ch = block.channels as usize;
        for (i, &v) in block.samples.iter().enumerate() {
            let frame = start + (i / ch) as u64;
            let c = i % ch;
            base[c][(frame / LOD_BASE_BUCKET) as usize].push(v);
        }
        start = end;
    }

    // Coarser levels merge adjacent base buckets in pairs.
    let mut levels: Vec<Vec<Vec<Bucket>>> = Vec::with_capacity(LOD_LEVELS);
    let mut current: Vec<Vec<Acc>> = base;
    for _ in 0..LOD_LEVELS {
        levels.push(
            current
                .iter()
                .map(|ch_acc| ch_acc.iter().map(Acc::bucket).collect())
                .collect(),
        );
        current = current
            .iter()
            .map(|ch_acc| {
                ch_acc
                    .chunks(2)
                    .map(|pair| {
                        let mut merged = pair[0];
                        if pair.len() == 2 {
                            merged.merge(&pair[1]);
                        }
                        merged
                    })
                    .collect()
            })
            .collect();
    }
    Ok(Pyramid { channels, levels })
}

/// Flatten the pyramid into tensor products: per level and channel a
/// `[buckets, 4]` matrix (min / max / rms / count). `source` is the
/// dataset-relative audio artifact the pyramid was built from.
pub fn products(input_id: &str, source: &str, p: &Pyramid) -> Vec<TensorProduct> {
    let mut out = Vec::new();
    for (level, chans) in p.levels.iter().enumerate() {
        for (c, buckets) in chans.iter().enumerate() {
            let mut data = Vec::with_capacity(buckets.len() * 4);
            for b in buckets {
                data.extend_from_slice(&[b.min, b.max, b.rms, b.count as f64]);
            }
            out.push(TensorProduct {
                path: format!("visualization/waveform/{input_id}/ch{c}/l{level}"),
                shape: vec![buckets.len() as u64, 4],
                data,
                parents: vec![source.to_string()],
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_combines_stats() {
        let mut a = Acc::default();
        for v in [1.0, -2.0, 3.0] {
            a.push(v);
        }
        let mut b = Acc::default();
        for v in [0.5, -0.5] {
            b.push(v);
        }
        a.merge(&b);
        let bucket = a.bucket();
        assert_eq!(bucket.min, -2.0);
        assert_eq!(bucket.max, 3.0);
        assert_eq!(bucket.count, 5);
        let expect = ((1.0 + 4.0 + 9.0 + 0.25 + 0.25) / 5.0_f64).sqrt();
        assert!((bucket.rms - expect).abs() < 1e-12);
    }
}
