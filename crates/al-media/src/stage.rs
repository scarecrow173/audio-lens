//! `Ingest` stage implementation: detect, decode, hash, annotate, and
//! register artifacts through the manifest.

use std::fs;
use std::path::PathBuf;

use al_core::StageKind;
use al_job::manifest::ArtifactRecord;
use al_job::stage::{Stage, StageContext, StageError};

use al_core::sha256_hex;

use crate::decoder::{DecodeRequest, DecoderRegistry, IngestResult};
use crate::dictionary::Dictionary;
use crate::metadata::{AudioMetadata, IntegrityObservation};
use crate::video::{VideoFeatureArtifact, VideoFeatureConfig, VideoFeatureReport};

/// Ingest stage: decodes declared inputs into pre-normalization PCM plus
/// metadata/annotation artifacts registered in the manifest.
///
/// A `None` dictionary simply produces no annotations; raw measurements
/// are always recorded.
#[derive(Debug, Default)]
pub struct IngestStage {
    dictionary: Option<Dictionary>,
}

impl IngestStage {
    /// Stage with no artifact dictionary.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stage that annotates observations with `dictionary`.
    pub fn with_dictionary(dictionary: Dictionary) -> Self {
        Self {
            dictionary: Some(dictionary),
        }
    }

    /// Stage that loads a dictionary from an explicit file path.
    /// A missing or unreadable file is an explicit error — an explicit
    /// configuration must not silently degrade to "no dictionary".
    pub fn with_dictionary_file(path: impl Into<PathBuf>) -> Result<Self, StageError> {
        let path = path.into();
        let dictionary = Dictionary::load(&path).map_err(|e| {
            StageError::new(
                StageKind::Ingest,
                format!("cannot load dictionary {}: {e}", path.display()),
            )
        })?;
        Ok(Self::with_dictionary(dictionary))
    }

    fn ingest_input(
        &self,
        input: &al_job::job::JobInput,
        video_cfg: &VideoFeatureConfig,
        ctx: &StageContext<'_>,
    ) -> Result<Vec<ArtifactRecord>, StageError> {
        let span = tracing::info_span!(
            "media::ingest",
            input_id = %input.input_id,
            len = tracing::field::Empty,
            sha256 = tracing::field::Empty
        );
        let _enter = span.enter();
        let path = &input.path;
        let bytes = fs::read(path).map_err(|e| {
            StageError::new(
                StageKind::Ingest,
                format!(
                    "decode/input unavailable: cannot read {}: {e}",
                    path.display()
                ),
            )
        })?;
        span.record("len", bytes.len());
        let source_hash = sha256_hex(&bytes);
        span.record("sha256", source_hash.as_str());

        // The job pins input content at creation; a resume must not
        // silently ingest a replaced file under the same job id. An
        // unpinned input is an error, not a skip — the runner validates
        // this too, but the stage does not rely on that.
        let Some(expected) = &input.source_sha256 else {
            return Err(StageError::new(
                StageKind::Ingest,
                format!("input {} has no pinned source hash", path.display()),
            ));
        };
        if *expected != source_hash {
            return Err(StageError::new(
                StageKind::Ingest,
                format!(
                    "input content changed since job creation: {} \
                     (expected sha256 {expected}, got {source_hash})",
                    path.display()
                ),
            ));
        }

        let request = DecodeRequest {
            video_config: video_cfg.clone(),
            ..Default::default()
        };
        let media = DecoderRegistry::native()
            .decode(&bytes, &request)
            .map_err(|e| {
                StageError::new(StageKind::Ingest, format!("{}: {e}", input.path.display()))
            })?;
        self.ingest_decoded(input, media, source_hash, ctx)
    }

    /// Register artifacts for one input: `metadata.json` always (with
    /// `video`, `av_timing`, `video_features`, `mp4`, and `integrity`
    /// sections for containers that carry them), plus `decoded.f64le`
    /// and `video_features.json` when produced. The partial-ingest
    /// contract: decode unavailability is an integrity observation, not
    /// a stage failure, so metadata and timing artifacts persist
    /// independently of PCM. Shared by every supported container.
    fn ingest_decoded(
        &self,
        input: &al_job::job::JobInput,
        mut mux: IngestResult,
        source_hash: String,
        ctx: &StageContext<'_>,
    ) -> Result<Vec<ArtifactRecord>, StageError> {
        // 部分 ingest のポリシーは backend ID ではなくコンテナ観測で決定。
        if mux.container == "wav" && mux.audio.is_none() {
            return Err(StageError::new(
                StageKind::Ingest,
                "WAV audio decode unavailable",
            ));
        }
        let audio = mux.audio.take();
        let container = mux.container;
        let codec_for_annotation = audio
            .as_ref()
            .map(|a| a.codec.as_str())
            .or(mux.fallback_codec.as_deref());
        let annotations: Vec<crate::dictionary::Annotation> = self
            .dictionary
            .as_ref()
            .map(|d| d.annotate(codec_for_annotation, Some(container)))
            .unwrap_or_default();
        let dictionary_info = self.dictionary.as_ref().map(|d| {
            serde_json::json!({
                "version": d.version,
                "sha256": d.sha256,
                "entries": d.entries.len(),
            })
        });

        // Audio stream facts — absent when the container could not
        // decode its audio; the container name stays a sibling field so
        // `metadata.json` is meaningful either way.
        let audio_metadata = audio.as_ref().map(|a| AudioMetadata {
            codec: a.codec.clone(),
            sample_rate: a.sample_rate,
            channels: a.channels,
            sample_format: a.sample_format,
            frames: Some(a.frames()),
            duration_secs: a.duration_secs(),
            source_hash: source_hash.clone(),
            history: vec![],
        });

        // Missing observations are recorded once, structurally —
        // downstream stages read facts, not implied absence.
        let integrity: Vec<IntegrityObservation> = mux.integrity.clone();

        let job_dir = ctx.registry.job_dir();
        let input_dir = job_dir.join("inputs").join(input.input_id.as_str());
        fs::create_dir_all(&input_dir).map_err(|e| {
            StageError::new(StageKind::Ingest, format!("cannot create inputs dir: {e}"))
        })?;

        // Artifact writes go through write_durable (tmp+fsync+rename):
        // the manifest can mark the stage Completed durably, so the
        // artifacts it points at must be durable too — otherwise a power
        // loss could leave Completed next to a truncated artifact, which
        // resume would wrongly skip as finished.

        // Link produced artifacts to the declared-input artifact so the
        // manifest lineage reaches back to the source file.
        let lineage: Vec<_> = ctx
            .manifest
            .input_artifact(&input.input_id)
            .into_iter()
            .collect();

        let mut records = Vec::new();
        if let Some(a) = audio.as_ref() {
            // `decoded_pcm` exists only when PCM was actually decoded;
            // a missing artifact is what `decode_input` reports as
            // `Ok(None)` → `DecodeInputUnavailable` downstream.
            let pcm_rel = PathBuf::from("inputs")
                .join(input.input_id.as_str())
                .join("decoded.f64le");
            let pcm_bytes: Vec<u8> = a.samples.iter().flat_map(|v| v.to_le_bytes()).collect();
            al_job::fsutil::write_durable(&job_dir.join(&pcm_rel), &pcm_bytes).map_err(|e| {
                StageError::new(StageKind::Ingest, format!("cannot write PCM: {e}"))
            })?;
            records.push(ArtifactRecord {
                artifact_id: ctx.registry.issue(),
                role: "decoded_pcm".into(),
                path: pcm_rel,
                produced_by: Some(StageKind::Ingest),
                inputs: lineage.clone(),
                params: serde_json::json!({
                    "codec": a.codec,
                    "sample_rate": a.sample_rate,
                    "channels": a.channels,
                }),
            });
        }

        let meta_rel = PathBuf::from("inputs")
            .join(input.input_id.as_str())
            .join("metadata.json");
        let meta_json = serde_json::json!({
            "input_id": input.input_id,
            "role": input.role,
            "source_path": input.path,
            "source_hash": source_hash,
            "container": container,
            "streams": mux.streams,
            "audio": audio_metadata,
            "video": mux.video,
            "av_timing": mux.av_timing,
            "video_features": mux.video_features.as_ref().map(video_feature_summary),
            "mp4": mux.mp4,
            "integrity": integrity,
            "dictionary": dictionary_info,
            "annotations": annotations,
            "decoder": mux.provenance,
        });
        let meta_text = serde_json::to_string_pretty(&meta_json).map_err(|e| {
            StageError::new(StageKind::Ingest, format!("cannot serialize metadata: {e}"))
        })?;
        al_job::fsutil::write_durable(&job_dir.join(&meta_rel), meta_text.as_bytes()).map_err(
            |e| StageError::new(StageKind::Ingest, format!("cannot write metadata: {e}")),
        )?;

        if let Some(features) = &mux.video_features {
            // Standalone `video_features.json` — cheap analysis reads
            // this artifact, not `metadata.json`, so the series does
            // not need to be duplicated into metadata.
            let vf_rel = PathBuf::from("inputs")
                .join(input.input_id.as_str())
                .join("video_features.json");
            let vf = VideoFeatureReport {
                input_id: input.input_id.to_string(),
                features: features.clone(),
            };
            let vf_text = serde_json::to_string_pretty(&vf).map_err(|e| {
                StageError::new(
                    StageKind::Ingest,
                    format!("cannot serialize video features: {e}"),
                )
            })?;
            al_job::fsutil::write_durable(&job_dir.join(&vf_rel), vf_text.as_bytes()).map_err(
                |e| {
                    StageError::new(
                        StageKind::Ingest,
                        format!("cannot write video features: {e}"),
                    )
                },
            )?;
            records.push(ArtifactRecord {
                artifact_id: ctx.registry.issue(),
                role: "video_features".into(),
                path: vf_rel,
                produced_by: Some(StageKind::Ingest),
                inputs: lineage.clone(),
                params: video_feature_summary(features),
            });
        }

        if let Some(thumbs) = &mux.video_thumbs {
            // Preview thumbnails ride alongside `video_features.json`
            // so the dataset/report can render the decoded stream
            // without re-decoding the container.
            let vt_rel = PathBuf::from("inputs")
                .join(input.input_id.as_str())
                .join("video_thumbs.json");
            let vt_text = serde_json::to_string_pretty(&serde_json::json!({
                "input_id": input.input_id,
                "thumbs": thumbs,
            }))
            .map_err(|e| {
                StageError::new(
                    StageKind::Ingest,
                    format!("cannot serialize video thumbs: {e}"),
                )
            })?;
            al_job::fsutil::write_durable(&job_dir.join(&vt_rel), vt_text.as_bytes()).map_err(
                |e| StageError::new(StageKind::Ingest, format!("cannot write video thumbs: {e}")),
            )?;
            records.push(ArtifactRecord {
                artifact_id: ctx.registry.issue(),
                role: "video_thumbs".into(),
                path: vt_rel,
                produced_by: Some(StageKind::Ingest),
                inputs: lineage.clone(),
                params: serde_json::json!({
                    "frames": thumbs.frame_indices.len(),
                    "width": thumbs.width,
                    "height": thumbs.height,
                }),
            });
        }

        records.push(ArtifactRecord {
            artifact_id: ctx.registry.issue(),
            role: "ingest_metadata".into(),
            path: meta_rel,
            produced_by: Some(StageKind::Ingest),
            inputs: lineage,
            params: serde_json::json!({
                "source_hash": source_hash,
                "annotation_count": annotations.len(),
                "dictionary_version": self
                    .dictionary
                    .as_ref()
                    .map(|d| d.version.clone()),
                "dictionary_sha256": self
                    .dictionary
                    .as_ref()
                    .map(|d| d.sha256.clone()),
            }),
        });
        for record in &mut records {
            if let Some(params) = record.params.as_object_mut() {
                params.insert(
                    "decoder".into(),
                    serde_json::to_value(&mux.provenance).map_err(|e| {
                        StageError::new(
                            StageKind::Ingest,
                            format!("cannot serialize decoder provenance: {e}"),
                        )
                    })?,
                );
            }
        }
        Ok(records)
    }
}

/// Compact `video_features` summary for `metadata.json` and the
/// artifact record's `params` — status plus frame/event counts, never
/// the full series (that lives in `video_features.json`).
fn video_feature_summary(features: &VideoFeatureArtifact) -> serde_json::Value {
    match features {
        VideoFeatureArtifact::Decoded(f) => serde_json::json!({
            "status": "decoded",
            "stream": f.stream,
            "codec": f.codec,
            "frames": f.luminance.len(),
            "events": f.events.len(),
        }),
        VideoFeatureArtifact::Unavailable {
            stream,
            codec,
            reason,
        } => serde_json::json!({
            "status": "unavailable",
            "stream": stream,
            "codec": codec,
            "reason": reason,
        }),
    }
}

impl Stage for IngestStage {
    fn kind(&self) -> StageKind {
        StageKind::Ingest
    }

    fn run(&mut self, ctx: &StageContext<'_>) -> Result<Vec<ArtifactRecord>, StageError> {
        let video_cfg = VideoFeatureConfig::from_job(ctx.config)
            .map_err(|e| StageError::new(StageKind::Ingest, e))?;
        let mut records = Vec::new();
        for input in &ctx.manifest.inputs {
            records.extend(self.ingest_input(input, &video_cfg, ctx)?);
        }
        Ok(records)
    }
}
