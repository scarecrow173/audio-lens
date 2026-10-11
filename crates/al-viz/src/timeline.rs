//! Shared capture timebase: every visualization product registers its
//! window/hop/offset here so consumers can align series without
//! re-deriving analysis parameters.

use std::collections::BTreeMap;
use std::io;

use al_dataset::reader::DatasetReader;

/// Waveform LOD decimation factors (samples per bucket per level).
/// Level 0 is the coarsest declared level; finer levels multiply.
pub const LOD_BASE_BUCKET: u64 = 256;
/// Number of pyramid levels generated for each waveform.
pub const LOD_LEVELS: usize = 5;

/// Axis declaration for one product family.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct SeriesDecl {
    /// What the series is indexed by: `sample`, `frame`, `bucket`,
    /// or `seconds`.
    pub unit: String,
    /// Analysis window in samples, when the series is frame-based.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<u64>,
    /// Hop in samples, when the series is frame-based.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hop: Option<u64>,
    /// Samples per bucket per level for LOD series.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub samples_per_bucket: Option<Vec<u64>>,
}

/// The shared timebase written to `visualization/timeline.json`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Timeline {
    /// Sample rate the capture timebase is expressed in.
    pub sample_rate: u32,
    /// Total capture frames.
    pub capture_samples: u64,
    /// Per-product axis declarations.
    pub series: BTreeMap<String, SeriesDecl>,
}

/// Build the timeline from the dataset's `inputs` audio role and the
/// job config's declared analysis resolution.
///
/// `capture` names the inputs artifact (`<input_id>`) that defines the
/// timebase; the first capture-role input is the canonical choice.
pub fn build(
    reader: &DatasetReader,
    capture: &str,
    config: &serde_json::Value,
) -> io::Result<Timeline> {
    let span = tracing::info_span!(
        "viz::timeline",
        input_id = %capture,
        len = tracing::field::Empty
    );
    let _enter = span.enter();
    let capture_samples = reader.audio_frames("inputs", capture)?;
    span.record("len", capture_samples);
    // A one-frame read returns the format metadata without touching
    // the bulk of the file.
    let head = reader.read_audio_range("inputs", capture, 0, 1)?;
    let sample_rate = head.sample_rate;

    let cheap = &config["analysis"]["cheap"];
    let stft_window = cheap["stft_window"].as_u64().unwrap_or(2048);
    let stft_hop = cheap["stft_hop"].as_u64().unwrap_or(1024);

    let mut series = BTreeMap::new();
    series.insert(
        "waveform_lod".into(),
        SeriesDecl {
            unit: "bucket".into(),
            window: None,
            hop: None,
            samples_per_bucket: Some((0..LOD_LEVELS).map(|i| LOD_BASE_BUCKET << i).collect()),
        },
    );
    for (name, window, hop) in [
        ("spectrogram", stft_window, stft_hop),
        ("band_heatmap", stft_window, stft_hop),
    ] {
        series.insert(
            name.into(),
            SeriesDecl {
                unit: "frame".into(),
                window: Some(window),
                hop: Some(hop),
                samples_per_bucket: None,
            },
        );
    }
    series.insert(
        "events".into(),
        SeriesDecl {
            // Event markers carry `start_s`/`end_s` in seconds, not
            // sample positions.
            unit: "seconds".into(),
            window: None,
            hop: None,
            samples_per_bucket: None,
        },
    );

    Ok(Timeline {
        sample_rate,
        capture_samples,
        series,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn series_decl_omits_absent_fields() {
        let decl = SeriesDecl {
            unit: "sample".into(),
            window: None,
            hop: None,
            samples_per_bucket: None,
        };
        let v = serde_json::to_value(&decl).unwrap();
        assert_eq!(v, serde_json::json!({"unit": "sample"}));
    }

    #[test]
    fn lod_levels_are_powers_of_two() {
        let buckets: Vec<u64> = (0..LOD_LEVELS).map(|i| LOD_BASE_BUCKET << i).collect();
        assert_eq!(buckets, vec![256, 512, 1024, 2048, 4096]);
    }
}
