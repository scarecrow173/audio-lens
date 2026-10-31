//! `VisualizationStage` / `ExportStage`: derive display products and
//! the relocatable package from the saved analysis dataset.

use std::io;
use std::path::PathBuf;

use al_core::{InputRole, StageKind};
use al_dataset::provenance::Provenance;
use al_dataset::reader::DatasetReader;
use al_dataset::writer::DatasetWriter;
use al_job::manifest::ArtifactRecord;
use al_job::stage::{Stage, StageContext, StageError};

use crate::package::ExportConfig;
use crate::{StreamedTensor, lod, summary, tiles, timeline, views};

fn invalid(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.into())
}

fn write_json(
    writer: &mut DatasetWriter,
    name: &str,
    body: &serde_json::Value,
    prov: Provenance,
) -> io::Result<()> {
    writer.write_json("visualization", name, body, prov)
}

/// Persist `data` as a chunked `[time, freq, …]` tensor: only the
/// leading (time) dim is split, so each chunk is a contiguous slice
/// and no chunk exceeds `tiles::TILE_CHUNK_ELEMS` elements.
fn write_tensor(
    writer: &mut DatasetWriter,
    name: &str,
    shape: &[u64],
    data: &[f64],
    prov: Provenance,
) -> io::Result<()> {
    if data.is_empty() {
        return Ok(());
    }
    let chunk_shape = tiles::chunk_shape_for(shape);
    let mut sink = writer.create_tensor("visualization", name, shape, &chunk_shape)?;
    let inner = shape[1..].iter().product::<u64>() as usize;
    for c0 in 0..shape[0].div_ceil(chunk_shape[0]) {
        let start = c0 * chunk_shape[0];
        let end = (start + chunk_shape[0]).min(shape[0]);
        let mut coords = vec![0u64; shape.len()];
        coords[0] = c0;
        sink.push(&coords, &data[start as usize * inner..end as usize * inner])?;
    }
    sink.finish(prov)
}

fn prov(input_id: &str, parents: Vec<String>) -> Provenance {
    Provenance {
        input_id: input_id.to_string(),
        algorithm_id: "viz.v1".into(),
        algorithm_version: "1".into(),
        params: serde_json::json!({}),
        alignment_id: None,
        parents,
    }
}

fn viz_name(path: &str) -> io::Result<&str> {
    path.strip_prefix("visualization/")
        .ok_or_else(|| invalid(format!("product path outside visualization role: {path}")))
}

/// Persist one derived JSON / in-memory-tensor product; returns the
/// registered dataset path. `parents` records the dataset-relative
/// artifact paths the product derives from so tensor tiles and views
/// stay traceable to their source measurements.
fn persist(
    writer: &mut DatasetWriter,
    path: &str,
    parents: Vec<String>,
    body: Option<&serde_json::Value>,
    tensor: Option<(&[u64], &[f64])>,
) -> io::Result<String> {
    let name = viz_name(path)?;
    if let Some(b) = body {
        write_json(writer, name, b, prov("job", parents))?;
    } else if let Some((shape, data)) = tensor {
        write_tensor(writer, name, shape, data, prov("job", parents))?;
    }
    Ok(path.to_string())
}

/// Persist a streamed tensor product chunk-by-chunk: the source
/// tensor is converted through ranged reads and each finished chunk
/// lands on disk before the next is read.
fn persist_stream(writer: &mut DatasetWriter, t: StreamedTensor) -> io::Result<String> {
    let name = viz_name(&t.path)?;
    let mut sink = writer.create_tensor("visualization", name, &t.shape, &t.chunk_shape)?;
    for chunk in t.chunks {
        let chunk = chunk?;
        sink.push(&chunk.coords, &chunk.data)?;
    }
    sink.finish(prov("job", t.parents))?;
    Ok(t.path)
}

/// The visualization stage.
#[derive(Debug, Default)]
pub struct VisualizationStage;

impl VisualizationStage {
    /// Create the stage.
    pub fn new() -> Self {
        Self
    }
}

impl Stage for VisualizationStage {
    fn kind(&self) -> StageKind {
        StageKind::Visualization
    }

    /// Everything this stage emits derives from the materialized
    /// dataset; whatever roles upstream stages produced is what gets
    /// visualized.
    fn depends_on(&self) -> Vec<StageKind> {
        vec![StageKind::Dataset]
    }

    fn run(&mut self, ctx: &StageContext<'_>) -> Result<Vec<ArtifactRecord>, StageError> {
        let err = |e: io::Error| StageError::new(StageKind::Visualization, e.to_string());
        let root = ctx.registry.job_dir().join("analysis");
        let reader = DatasetReader::open(&root).map_err(err)?;
        let mut writer = open_viz_dataset(ctx).map_err(err)?;
        let mut produced: Vec<String> = Vec::new();
        let mut skipped: Vec<summary::Skipped> = Vec::new();

        let mut skip = |product: &str, role: &str, reason: &str| {
            skipped.push(summary::Skipped {
                product: product.to_string(),
                required_role: role.to_string(),
                reason: reason.to_string(),
            });
        };

        // Contract: a produced role must contain its expected store.
        let expect = |role: &str, pred: fn(&al_dataset::manifest::DatasetArtifact) -> bool| {
            if reader.role_produced(role) && !reader.artifacts(role).into_iter().any(pred) {
                return Err(invalid(format!(
                    "role {role} is produced but has no expected artifacts"
                )));
            }
            Ok(())
        };
        let tensor = |a: &al_dataset::manifest::DatasetArtifact| {
            a.store == al_dataset::manifest::StoreKind::Tensor
        };
        let json = |a: &al_dataset::manifest::DatasetArtifact| {
            a.store == al_dataset::manifest::StoreKind::Json
        };
        let table = |a: &al_dataset::manifest::DatasetArtifact| {
            a.store == al_dataset::manifest::StoreKind::Table
        };
        // Partial-ingest contract: an input whose PCM was undecodable
        // contributes only its metadata JSON, so `inputs` may hold
        // Json stores with zero Audio stores.
        if reader.role_produced("inputs") {
            for input in &ctx.manifest.inputs {
                let audio = format!("inputs/{}.wav", input.input_id);
                let metadata = format!("inputs/{}.metadata.json", input.input_id);
                if !reader.artifacts("inputs").into_iter().any(|a| {
                    (a.store == al_dataset::manifest::StoreKind::Audio && a.path == audio)
                        || (a.store == al_dataset::manifest::StoreKind::Json && a.path == metadata)
                }) {
                    return Err(err(invalid(format!(
                        "materialized inputs lack audio or metadata for {}",
                        input.input_id
                    ))));
                }
            }
        }
        expect("intermediate", tensor).map_err(err)?;
        expect("features/cheap", json).map_err(err)?;
        expect("residual", tensor).map_err(err)?;
        expect("events", table).map_err(err)?;
        expect("locate", json).map_err(err)?;

        // Timeline: the first capture input defines the timebase.
        let capture = ctx
            .manifest
            .inputs
            .iter()
            .find(|i| matches!(i.role, InputRole::CaptureBug | InputRole::CaptureReference))
            .or_else(|| ctx.manifest.inputs.first());
        // The capture's own rate drives second-based conversions; it
        // only exists once `inputs` audio is materialized. When the
        // capture itself was undecodable there is no timebase — the
        // timeline product is skipped rather than failed.
        let mut sample_rate: Option<f64> = None;
        let capture_audio = |cap_id: &str| {
            reader.artifacts("inputs").into_iter().find(|a| {
                a.store == al_dataset::manifest::StoreKind::Audio
                    && a.path
                        .strip_prefix("inputs/")
                        .and_then(|p| p.strip_suffix(".wav"))
                        == Some(cap_id)
            })
        };
        if let Some(cap) = capture
            && reader.role_produced("inputs")
        {
            let cap_id = cap.input_id.to_string();
            if let Some(cap_audio) = capture_audio(&cap_id) {
                let tl = timeline::build(&reader, &cap_id, ctx.config).map_err(err)?;
                sample_rate = Some(f64::from(tl.sample_rate));
                let body = serde_json::to_value(&tl).map_err(|e| invalid(e.to_string()))?;
                produced.push(persist(
                    &mut writer,
                    "visualization/timeline.json",
                    vec![cap_audio.path.clone()],
                    Some(&body),
                    None,
                )?);
            } else {
                skip(
                    "timeline",
                    "inputs",
                    "capture input has no materialized audio",
                );
            }
        } else {
            skip(
                "timeline",
                "inputs",
                if capture.is_none() {
                    "no capture input"
                } else {
                    "role not produced"
                },
            );
        }

        // Waveform LOD per materialized input (audio stores only —
        // the role also carries declared-input JSON records).
        if reader.role_produced("inputs") {
            for a in reader
                .artifacts("inputs")
                .into_iter()
                .filter(|a| a.store == al_dataset::manifest::StoreKind::Audio)
            {
                let rel = a.path.strip_prefix("inputs/").unwrap_or(&a.path);
                let name = rel.strip_suffix(".wav").unwrap_or(rel).to_string();
                let pyramid = lod::build(&reader, &name).map_err(err)?;
                for t in lod::products(&name, &a.path, &pyramid) {
                    produced.push(persist(
                        &mut writer,
                        &t.path,
                        t.parents.clone(),
                        None,
                        Some((&t.shape, &t.data)),
                    )?);
                }
            }
            // Video frame series materialized under `inputs` become
            // per-input video views (features + preview thumbnails).
            for v in views::video_views(&reader).map_err(err)? {
                produced.push(persist(
                    &mut writer,
                    &v.path,
                    v.parents.clone(),
                    Some(&v.body),
                    None,
                )?);
            }
            // Video-series *tensor* products cannot exist: the dataset
            // carries no per-frame pixel tensor.
            skip(
                "video_series",
                "inputs",
                "no video frame tensor is materialized in the dataset",
            );
        } else {
            skip("waveform_lod", "inputs", "role not produced");
        }

        // Tiles + validity masks. Tile tensors stream chunk-by-chunk
        // so long spectrograms are converted and stored without a
        // full-tensor load on either side.
        if reader.role_produced("intermediate") {
            for t in tiles::build(&reader).map_err(err)? {
                produced.push(persist_stream(&mut writer, t)?);
            }
        } else {
            skip("spectrogram_tiles", "intermediate", "role not produced");
        }
        if reader.role_produced("features/cheap") {
            let hop_samples = ctx.config["analysis"]["cheap"]["stft_hop"]
                .as_f64()
                .unwrap_or(1024.0);
            let hop_s = hop_samples / sample_rate.unwrap_or(48_000.0);
            for t in tiles::validity_masks(&reader, hop_s).map_err(err)? {
                produced.push(persist(
                    &mut writer,
                    &t.path,
                    t.parents.clone(),
                    None,
                    Some((&t.shape, &t.data)),
                )?);
            }
        } else {
            skip("validity_masks", "features/cheap", "role not produced");
        }

        // Views.
        if reader.role_produced("intermediate") {
            for v in views::transfer_views(&reader, sample_rate.unwrap_or(48_000.0)).map_err(err)? {
                produced.push(persist(
                    &mut writer,
                    &v.path,
                    v.parents.clone(),
                    Some(&v.body),
                    None,
                )?);
            }
        } else {
            skip("transfer_view", "intermediate", "role not produced");
        }
        if reader.role_produced("locate") {
            for v in views::locate_views(&reader).map_err(err)? {
                produced.push(persist(
                    &mut writer,
                    &v.path,
                    v.parents.clone(),
                    Some(&v.body),
                    None,
                )?);
            }
        } else {
            skip("locate_view", "locate", "role not produced");
        }
        if reader.role_produced("features/cheap") {
            for v in views::stereo_views(&reader).map_err(err)? {
                produced.push(persist(
                    &mut writer,
                    &v.path,
                    v.parents.clone(),
                    Some(&v.body),
                    None,
                )?);
            }
        } else {
            skip("stereo_view", "features/cheap", "role not produced");
        }
        if reader.role_produced("residual") {
            for v in views::residual_views(&reader).map_err(err)? {
                produced.push(persist(
                    &mut writer,
                    &v.path,
                    v.parents.clone(),
                    Some(&v.body),
                    None,
                )?);
            }
        } else {
            skip("residual_view", "residual", "role not produced");
        }
        if reader.role_produced("events") {
            if let Some(v) = views::events_view(&reader).map_err(err)? {
                produced.push(persist(
                    &mut writer,
                    &v.path,
                    v.parents.clone(),
                    Some(&v.body),
                    None,
                )?);
            }
        } else {
            skip("event_markers", "events", "role not produced");
        }
        // Expensive features are summarized only; a dedicated view
        // needs no reshaping beyond the summary tally.
        if !reader.role_produced("features/expensive") {
            skip("expensive_view", "features/expensive", "role not produced");
        }

        // skipped.json + summary.json (paths are dataset-relative).
        // skipped.json is bookkeeping over absent roles — it has no
        // source artifacts; the summary aggregates every produced
        // product, which are its parents.
        produced.push(persist(
            &mut writer,
            "visualization/skipped.json",
            Vec::new(),
            Some(&serde_json::json!({"skipped_products": skipped})),
            None,
        )?);
        let body =
            summary::summary_body(&reader, writer.manifest(), &produced, &skipped).map_err(err)?;
        produced.push(persist(
            &mut writer,
            "visualization/summary.json",
            produced.clone(),
            Some(&body),
            None,
        )?);

        writer.finish().map_err(err)?;
        Ok(vec![ArtifactRecord {
            artifact_id: ctx.registry.issue(),
            role: "visualization".into(),
            path: PathBuf::from("analysis/visualization/summary.json"),
            produced_by: Some(StageKind::Visualization),
            inputs: Vec::new(),
            params: serde_json::json!({
                "products": produced.len(),
                "skipped": skipped.len(),
            }),
        }])
    }
}

/// Open the analysis dataset for append: identity mismatches fail
/// explicitly (pre-release — regenerate, never migrate).
fn open_viz_dataset(ctx: &StageContext<'_>) -> io::Result<DatasetWriter> {
    let root = ctx.registry.job_dir().join("analysis");
    DatasetWriter::open_or_create(
        &root,
        al_dataset::provenance::input_references(ctx.manifest)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?,
        ctx.config.clone(),
    )
}

/// The export stage.
#[derive(Debug, Default)]
pub struct ExportStage;

impl ExportStage {
    /// Create the stage.
    pub fn new() -> Self {
        Self
    }
}

impl Stage for ExportStage {
    fn kind(&self) -> StageKind {
        StageKind::Export
    }

    /// The package reads only the materialized analysis dataset:
    /// visualization products are optional contents, not a
    /// precondition (#103 — export must not require the old
    /// Visualization stage).
    fn depends_on(&self) -> Vec<StageKind> {
        vec![StageKind::Dataset]
    }

    fn run(&mut self, ctx: &StageContext<'_>) -> Result<Vec<ArtifactRecord>, StageError> {
        let err = |e: io::Error| StageError::new(StageKind::Export, e.to_string());
        let cfg = ExportConfig::from_job(ctx.config);
        let job_dir = ctx.registry.job_dir();
        let (dest, files) = crate::package::build(job_dir, ctx.manifest, &cfg).map_err(err)?;

        // Golden snapshot over the package (excluding itself).
        let only: Vec<String> = ctx.config["export"]["golden_paths"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let snap = crate::golden::snapshot(&dest, &only).map_err(err)?;
        let golden_rel = "golden/snapshot.json";
        std::fs::create_dir_all(dest.join("golden")).map_err(err)?;
        std::fs::write(
            dest.join(golden_rel),
            serde_json::to_string_pretty(&snap).map_err(|e| invalid(e.to_string()))?,
        )
        .map_err(err)?;

        let rel = |p: &str| PathBuf::from(format!("export/{}/{p}", cfg.name));
        let mut records = Vec::new();
        for (path, role) in [
            ("export_manifest.json", "export_manifest"),
            (golden_rel, "export_golden"),
        ] {
            records.push(ArtifactRecord {
                artifact_id: ctx.registry.issue(),
                role: role.into(),
                path: rel(path),
                produced_by: Some(StageKind::Export),
                inputs: Vec::new(),
                params: serde_json::json!({}),
            });
        }
        records.push(ArtifactRecord {
            artifact_id: ctx.registry.issue(),
            role: "export_package".into(),
            path: PathBuf::from(format!("export/{}", cfg.name)),
            produced_by: Some(StageKind::Export),
            inputs: Vec::new(),
            params: serde_json::json!({
                "package": cfg.name,
                "files": files.len(),
                "copy_media": cfg.copy_media,
            }),
        });
        Ok(records)
    }
}
