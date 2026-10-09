//! Regenerate `report.html` + `report.data.js` for an existing export
//! package without re-running the pipeline:
//!
//! ```text
//! cargo run -p al-viz --example regen_report -- <package-dir>
//! ```
//!
//! `<package-dir>` is the export package root that contains `analysis/`.

use std::path::PathBuf;

fn main() -> std::io::Result<()> {
    let dest = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .expect("usage: regen_report <package-dir>");
    al_viz::report::generate(&dest)
}
