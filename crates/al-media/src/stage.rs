//! `Ingest` stage implementation: detect, decode, hash, annotate, and
//! register artifacts through the manifest.

use std::fs;
use std::path::PathBuf;

use al_core::StageKind;
use al_job::manifest::ArtifactRecord;
use al_job::stage::{Stage, StageContext, StageError};

use crate::dictionary::Dictionary;
use crate::format::{MediaFormat, detect_format};
use crate::hash::sha256_hex;
use crate::metadata::AudioMetadata;
use crate::wav::decode_wav;

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
        ctx: &StageContext<'_>,
    ) -> Result<Vec<ArtifactRecord>, StageError> {
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
        let source_hash = sha256_hex(&bytes);

        let format = detect_format(&bytes).ok_or_else(|| {
            StageError::new(
                StageKind::Ingest,
                format!(
                    "decode/input unavailable: unknown format for {}",
                    path.display()
                ),
            )
        })?;

        match format {
            MediaFormat::Wav => self.ingest_wav(input, &bytes, source_hash, ctx),
        }
    }

    fn ingest_wav(
        &self,
        input: &al_job::job::JobInput,
        bytes: &[u8],
        source_hash: String,
        ctx: &StageContext<'_>,
    ) -> Result<Vec<ArtifactRecord>, StageError> {
        let audio = decode_wav(bytes).map_err(|e| {
            StageError::new(StageKind::Ingest, format!("{}: {e}", input.path.display()))
        })?;

        let annotations: Vec<crate::dictionary::Annotation> = self
            .dictionary
            .as_ref()
            .map(|d| d.annotate(Some(&audio.codec), Some("wav")))
            .unwrap_or_default();
        let dictionary_info = self.dictionary.as_ref().map(|d| {
            serde_json::json!({
                "version": d.version,
                "sha256": d.sha256,
                "entries": d.entries.len(),
            })
        });

        let metadata = AudioMetadata {
            container: "wav".into(),
            codec: audio.codec.clone(),
            sample_rate: audio.sample_rate,
            channels: audio.channels,
            sample_format: audio.sample_format,
            frames: Some(audio.frames()),
            duration_secs: audio.duration_secs(),
            source_hash,
            history: vec![],
        };

        let job_dir = ctx.registry.job_dir();
        let input_dir = job_dir.join("inputs").join(input.input_id.as_str());
        fs::create_dir_all(&input_dir).map_err(|e| {
            StageError::new(StageKind::Ingest, format!("cannot create inputs dir: {e}"))
        })?;

        // Decoded PCM: raw little-endian f64, interleaved.
        let pcm_rel = PathBuf::from("inputs")
            .join(input.input_id.as_str())
            .join("decoded.f64le");
        let pcm_bytes: Vec<u8> = audio.samples.iter().flat_map(|v| v.to_le_bytes()).collect();
        fs::write(job_dir.join(&pcm_rel), &pcm_bytes)
            .map_err(|e| StageError::new(StageKind::Ingest, format!("cannot write PCM: {e}")))?;

        let meta_rel = PathBuf::from("inputs")
            .join(input.input_id.as_str())
            .join("metadata.json");
        let meta_json = serde_json::json!({
            "input_id": input.input_id,
            "role": input.role,
            "source_path": input.path,
            "audio": metadata,
            "dictionary": dictionary_info,
            "annotations": annotations,
        });
        let meta_text = serde_json::to_string_pretty(&meta_json).map_err(|e| {
            StageError::new(StageKind::Ingest, format!("cannot serialize metadata: {e}"))
        })?;
        fs::write(job_dir.join(&meta_rel), meta_text).map_err(|e| {
            StageError::new(StageKind::Ingest, format!("cannot write metadata: {e}"))
        })?;

        // Link produced artifacts to the declared-input artifact so the
        // manifest lineage reaches back to the source file.
        let lineage: Vec<_> = ctx
            .manifest
            .input_artifact(&input.input_id)
            .into_iter()
            .collect();

        Ok(vec![
            ArtifactRecord {
                artifact_id: ctx.registry.issue(),
                role: "decoded_pcm".into(),
                path: pcm_rel,
                produced_by: Some(StageKind::Ingest),
                inputs: lineage.clone(),
                params: serde_json::json!({
                    "codec": audio.codec,
                    "sample_rate": audio.sample_rate,
                    "channels": audio.channels,
                }),
            },
            ArtifactRecord {
                artifact_id: ctx.registry.issue(),
                role: "ingest_metadata".into(),
                path: meta_rel,
                produced_by: Some(StageKind::Ingest),
                inputs: lineage,
                params: serde_json::json!({
                    "source_hash": metadata.source_hash,
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
            },
        ])
    }
}

impl Stage for IngestStage {
    fn kind(&self) -> StageKind {
        StageKind::Ingest
    }

    fn run(&mut self, ctx: &StageContext<'_>) -> Result<Vec<ArtifactRecord>, StageError> {
        let mut records = Vec::new();
        for input in &ctx.manifest.inputs {
            records.extend(self.ingest_input(input, ctx)?);
        }
        Ok(records)
    }
}
