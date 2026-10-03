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
use crate::{lod, summary, tiles, timeline, views};

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
    let chunk = al_dataset::writer::TensorChunk {
        coords: vec![0; shape.len()],
        data: data.to_vec(),
    };
    writer.write_tensor("visualization", name, shape, shape, &[chunk], prov)
}

fn prov(input_id: &str) -> Provenance {
    Provenance {
        input_id: input_id.to_string(),
        algorithm_id: "viz.v1".into(),
        algorithm_version: "1".into(),
        params: serde_json::json!({}),
        alignment_id: None,
        parents: Vec::new(),
    }
}

/// Persist one derived product; returns the registered dataset path.
fn persist(
    writer: &mut DatasetWriter,
    path: &str,
    body: Option<&serde_json::Value>,
    tensor: Option<(&[u64], &[f64])>,
) -> io::Result<String> {
    let name = path
        .strip_prefix("visualization/")
        .ok_or_else(|| invalid(format!("product path outside visualization role: {path}")))?;
    if let Some(b) = body {
        write_json(writer, name, b, prov("job"))?;
    } else if let Some((shape, data)) = tensor {
        write_tensor(writer, name, shape, data, prov("job"))?;
    }
    Ok(path.to_string())
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
        let audio = |a: &al_dataset::manifest::DatasetArtifact| {
            a.store == al_dataset::manifest::StoreKind::Audio
        };
        let table = |a: &al_dataset::manifest::DatasetArtifact| {
            a.store == al_dataset::manifest::StoreKind::Table
        };
        expect("inputs", audio).map_err(err)?;
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
        // only exists once `inputs` audio is materialized.
        let mut sample_rate: Option<f64> = None;
        if let Some(cap) = capture
            && reader.role_produced("inputs")
        {
            let cap_id = cap.input_id.to_string();
            let tl = timeline::build(&reader, &cap_id, ctx.config).map_err(err)?;
            sample_rate = Some(f64::from(tl.sample_rate));
            let body = serde_json::to_value(&tl).map_err(|e| invalid(e.to_string()))?;
            produced.push(persist(
                &mut writer,
                "visualization/timeline.json",
                Some(&body),
                None,
            )?);
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
                for t in lod::products(&name, &pyramid) {
                    produced.push(persist(
                        &mut writer,
                        &t.path,
                        None,
                        Some((&t.shape, &t.data)),
                    )?);
                }
            }
            // Video-series products cannot exist: the job stores no
            // video track in the dataset.
            skip(
                "video_series",
                "inputs",
                "no video stream is materialized in the dataset",
            );
        } else {
            skip("waveform_lod", "inputs", "role not produced");
        }

        // Tiles + validity masks.
        if reader.role_produced("intermediate") {
            for t in tiles::build(&reader).map_err(err)? {
                produced.push(persist(
                    &mut writer,
                    &t.path,
                    None,
                    Some((&t.shape, &t.data)),
                )?);
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
                produced.push(persist(&mut writer, &v.path, Some(&v.body), None)?);
            }
        } else {
            skip("transfer_view", "intermediate", "role not produced");
        }
        if reader.role_produced("locate") {
            for v in views::locate_views(&reader).map_err(err)? {
                produced.push(persist(&mut writer, &v.path, Some(&v.body), None)?);
            }
        } else {
            skip("locate_view", "locate", "role not produced");
        }
        if reader.role_produced("features/cheap") {
            for v in views::stereo_views(&reader).map_err(err)? {
                produced.push(persist(&mut writer, &v.path, Some(&v.body), None)?);
            }
        } else {
            skip("stereo_view", "features/cheap", "role not produced");
        }
        if reader.role_produced("residual") {
            for v in views::residual_views(&reader).map_err(err)? {
                produced.push(persist(&mut writer, &v.path, Some(&v.body), None)?);
            }
        } else {
            skip("residual_view", "residual", "role not produced");
        }
        if reader.role_produced("events") {
            if let Some(v) = views::events_view(&reader).map_err(err)? {
                produced.push(persist(&mut writer, &v.path, Some(&v.body), None)?);
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
        produced.push(persist(
            &mut writer,
            "visualization/skipped.json",
            Some(&serde_json::json!({"skipped_products": skipped})),
            None,
        )?);
        let body =
            summary::summary_body(&reader, writer.manifest(), &produced, &skipped).map_err(err)?;
        produced.push(persist(
            &mut writer,
            "visualization/summary.json",
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

    fn depends_on(&self) -> Vec<StageKind> {
        vec![StageKind::Dataset, StageKind::Visualization]
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
