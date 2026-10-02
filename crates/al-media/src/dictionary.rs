//! Versioned capture-artifact dictionary.
//!
//! The dictionary records known codec/container/SRC characteristics so that
//! downstream analysis can ask "does this observation match a known capture
//! artifact?". It returns annotations only — the contract is that dictionary
//! lookups never modify raw measurements.

use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// What an entry applies to. All present fields must match; absent fields
/// match anything.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EntryMatch {
    /// Codec name (e.g. `aac`, `pcm_s16le`), optional.
    #[serde(default)]
    pub codec: Option<String>,
    /// Container name (e.g. `wav`, `mkv`), optional.
    #[serde(default)]
    pub container: Option<String>,
}

/// One dictionary entry: a known artifact annotation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictEntry {
    /// Topic, e.g. `codec_hf_cutoff`, `container_block_size`, `src_route`.
    pub topic: String,
    /// Selector for which inputs this entry applies to.
    #[serde(default)]
    pub matches: EntryMatch,
    /// The annotation text (expected ranges, known constraints, ...).
    pub annotation: serde_json::Value,
    /// Optional reference describing where this knowledge came from.
    #[serde(default)]
    pub reference: Option<String>,
}

/// A versioned dictionary of known capture-artifact knowledge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dictionary {
    /// Dictionary version string; attached to every annotation result.
    pub version: String,
    /// Dictionary entries.
    #[serde(default)]
    pub entries: Vec<DictEntry>,
    /// SHA-256 of the JSON text this dictionary was parsed from.
    /// Recorded in job metadata so annotations stay attributable to the
    /// exact dictionary content, not just a version label.
    #[serde(skip)]
    pub sha256: String,
}

/// An annotation returned by a dictionary lookup. Carries the dictionary
/// version so results stay interpretable as the dictionary evolves.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Annotation {
    /// Entry topic.
    pub topic: String,
    /// Annotation payload from the entry.
    pub annotation: serde_json::Value,
    /// Optional provenance reference from the entry.
    pub reference: Option<String>,
    /// Version of the dictionary that produced this annotation.
    pub dictionary_version: String,
}

impl Dictionary {
    /// Load a dictionary from a JSON file.
    pub fn load(path: &Path) -> io::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Self::from_json(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Parse a dictionary from JSON text, recording its content hash.
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        let mut dict: Self = serde_json::from_str(text)?;
        dict.sha256 = crate::hash::sha256_hex(text.as_bytes());
        Ok(dict)
    }

    /// Raw JSON text of the dictionary bundled with the binary.
    ///
    /// Exposed so callers can snapshot the exact bytes (plus their hash)
    /// into a job config — a `resume` then stays pinned to this content
    /// even if the binary's embedded dictionary later changes.
    pub const EMBEDDED_JSON: &'static str =
        include_str!("../../../dictionaries/capture-artifacts.v1.json");

    /// The dictionary bundled with the binary.
    ///
    /// Using the embedded dictionary as the default keeps ingest results
    /// deterministic regardless of the working directory; the content hash
    /// recorded in metadata identifies exactly which dictionary produced
    /// the annotations.
    pub fn embedded() -> Self {
        Self::from_json(Self::EMBEDDED_JSON).expect("bundled dictionary must parse")
    }

    /// Look up annotations for a codec/container pair.
    ///
    /// Returns entries whose selector fields all match. This is an
    /// annotation query only — it returns metadata for interpretation and
    /// never mutates the caller's measurements.
    pub fn annotate(&self, codec: Option<&str>, container: Option<&str>) -> Vec<Annotation> {
        self.entries
            .iter()
            .filter(|e| {
                e.matches.codec.as_deref().is_none_or(|c| Some(c) == codec)
                    && e.matches
                        .container
                        .as_deref()
                        .is_none_or(|c| Some(c) == container)
            })
            .map(|e| Annotation {
                topic: e.topic.clone(),
                annotation: e.annotation.clone(),
                reference: e.reference.clone(),
                dictionary_version: self.version.clone(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_dict() -> Dictionary {
        Dictionary::from_json(
            r#"{
                "version": "1",
                "entries": [
                    {
                        "topic": "codec_hf_cutoff",
                        "matches": {"codec": "aac"},
                        "annotation": {"expected_khz": [16, 20]},
                        "reference": "encoder default"
                    },
                    {
                        "topic": "container_block_size",
                        "matches": {"codec": "mp3"},
                        "annotation": {"samples_per_frame": 1152}
                    }
                ]
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn annotate_matches_codec_only() {
        let dict = sample_dict();
        let hits = dict.annotate(Some("aac"), None);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].topic, "codec_hf_cutoff");
        assert_eq!(hits[0].dictionary_version, "1");
        assert_eq!(hits[0].reference.as_deref(), Some("encoder default"));
    }

    #[test]
    fn annotate_with_unmatched_fields_returns_empty() {
        let dict = sample_dict();
        assert!(dict.annotate(Some("pcm_s16le"), None).is_empty());
        // Absent selector fields act as wildcards: the aac entry still
        // matches when a container is also given.
        assert_eq!(dict.annotate(Some("aac"), Some("mkv")).len(), 1);
        assert!(dict.annotate(None, Some("mkv")).is_empty());
    }

    #[test]
    fn bundled_dictionary_loads() {
        let dict = Dictionary::embedded();
        assert_eq!(dict.version, "1");
        assert!(!dict.entries.is_empty());
        assert_eq!(dict.sha256.len(), 64);
    }

    #[test]
    fn loaded_dictionary_records_content_hash() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../dictionaries/capture-artifacts.v1.json");
        let text = std::fs::read_to_string(&path).unwrap();
        let dict = Dictionary::load(&path).unwrap();
        assert_eq!(dict.sha256, crate::hash::sha256_hex(text.as_bytes()));
    }

    #[test]
    fn annotations_are_data_only() {
        // The contract: lookups return copies and never touch measurements.
        let dict = sample_dict();
        let hits = dict.annotate(Some("aac"), None);
        assert_eq!(
            hits[0].annotation["expected_khz"],
            serde_json::json!([16, 20])
        );
    }
}
