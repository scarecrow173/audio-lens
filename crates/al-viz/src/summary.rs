//! Deterministic human-readable summary + skipped-product record.
//! The summary aggregates what exists — it never estimates a root
//! cause (measurement-over-verdict).

use std::io;

use al_dataset::reader::DatasetReader;

/// A product that could not be generated, with the reason.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Skipped {
    /// Product name (e.g. `spectrogram/<input>`).
    pub product: String,
    /// Required dataset role.
    pub required_role: String,
    /// Why it was skipped.
    pub reason: String,
}

/// Build `summary.json` content: input list, produced roles,
/// candidate/alignment counts, confidence/validity tallies, and the
/// list of products written (dataset-relative paths only).
///
/// `roles_source` is the manifest being *written* by the producing
/// stage, not the read snapshot: the read snapshot predates this
/// run's `visualization` role, so role status would flip between
/// the first and second run of the stage.
pub fn summary_body(
    reader: &DatasetReader,
    roles_source: &al_dataset::manifest::DatasetManifest,
    produced_products: &[String],
    skipped: &[Skipped],
) -> io::Result<serde_json::Value> {
    let inputs: Vec<serde_json::Value> = reader
        .artifacts("inputs")
        .into_iter()
        .map(|a| {
            serde_json::json!({
                "path": a.path,
                "provenance_input_id": a.provenance.input_id,
            })
        })
        .collect();

    let produced_roles: Vec<String> = {
        let mut v: Vec<String> = roles_source
            .roles
            .iter()
            .filter(|(_, e)| e.status == al_dataset::manifest::RoleStatus::Produced)
            .map(|(k, _)| k.clone())
            .collect();
        v.sort();
        v
    };

    // Candidate/alignment counts.
    let events_count = reader
        .open_table("events", "candidates")
        .map(|t| t.row_count())
        .unwrap_or(0);
    let (mut accepted, mut rejected) = (0u64, 0u64);
    if let Ok(t) = reader.open_table("locate", "summary") {
        let n = t.row_count();
        // `read_range` is column-major: one column of `n` cells.
        let status_col = t.schema().iter().position(|c| c.name == "status");
        if n > 0
            && let Some(col) = status_col
        {
            for cell in t.read_range(0, n, Some(&[col]))?.into_iter().flatten() {
                if let al_dataset::table::Cell::Str(s) = cell {
                    match s.as_str() {
                        "accepted" => accepted += 1,
                        _ => rejected += 1,
                    }
                }
            }
        }
    }

    // Confidence/validity tallies over feature JSON records.
    let mut confidence: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    let mut validity: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    for role in ["features/cheap", "features/reference", "features/expensive"] {
        for a in reader.artifacts(role) {
            if a.store != al_dataset::manifest::StoreKind::Json {
                continue;
            }
            let name = a
                .path
                .strip_prefix(&format!("{role}/"))
                .unwrap_or(&a.path)
                .strip_suffix(".json")
                .unwrap_or(&a.path);
            if let Ok(body) = reader.read_json(role, name)
                && let Some(records) = body["records"].as_array()
            {
                for r in records {
                    if let Some(c) = r["confidence"].as_str() {
                        *confidence.entry(c.to_string()).or_default() += 1;
                    }
                    if let Some(v) = r["validity"].as_str() {
                        *validity.entry(v.to_string()).or_default() += 1;
                    }
                }
            }
        }
    }

    Ok(serde_json::json!({
        "inputs": inputs,
        "produced_roles": produced_roles,
        "candidate_events": events_count,
        "alignments": {"accepted": accepted, "rejected": rejected},
        "confidence_tally": confidence,
        "validity_tally": validity,
        "products": produced_products,
        "skipped_products": skipped,
    }))
}
