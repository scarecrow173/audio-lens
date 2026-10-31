//! Derives the resolved Ruviform revision from the workspace
//! `Cargo.lock` so recorded provenance cannot drift from the pinned
//! git dependency (AL#115). All `ruviform` git sources must resolve to
//! a single commit — a mixed lockfile fails the build loudly.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let lock = find_lockfile(&manifest_dir).expect("Cargo.lock not found above CARGO_MANIFEST_DIR");
    println!("cargo:rerun-if-changed={}", lock.display());
    let text = fs::read_to_string(&lock).expect("cannot read Cargo.lock");
    let revs = ruviform_revs(&text);
    match revs.len() {
        1 => println!("cargo:rustc-env=RUVIFORM_REV={}", revs[0]),
        0 => panic!("no ruviform git dependency found in {}", lock.display()),
        _ => panic!(
            "multiple ruviform revisions in {}: {revs:?}",
            lock.display()
        ),
    }
}

/// Walk ancestors of `dir` until a `Cargo.lock` is found (the
/// workspace root lockfile for workspace builds).
fn find_lockfile(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .map(|d| d.join("Cargo.lock"))
        .find(|p| p.is_file())
}

/// Resolved commit SHAs (the `#<sha>` fragment) of every `ruviform`
/// git source in the lockfile, deduplicated.
fn ruviform_revs(lock: &str) -> Vec<String> {
    let mut revs = BTreeSet::new();
    for line in lock.lines() {
        let Some(source) = line
            .trim()
            .strip_prefix("source = \"")
            .and_then(|s| s.strip_suffix('"'))
        else {
            continue;
        };
        if !source.starts_with("git+") || !source.contains("ruviform") {
            continue;
        }
        let Some(sha) = source.rsplit('#').next() else {
            continue;
        };
        assert!(
            sha.len() == 40 && sha.bytes().all(|b| b.is_ascii_hexdigit()),
            "ruviform source lacks a resolved commit fragment: {source}"
        );
        revs.insert(sha.to_string());
    }
    revs.into_iter().collect()
}
