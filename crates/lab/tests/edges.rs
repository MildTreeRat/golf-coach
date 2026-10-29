//! Nothing depends on `golf-lab` (the M29 plan's rule on cargo edges, and D13). [M29 P20]
//!
//! It is a shell: it may hold any library crate, `ocr`'s `ort` among them, so a crate that depended
//! on it would reach all of them, and `ort` with them, which `crates/ocr/tests/ort_reach.rs` would
//! catch only for the crates it guards. Read from the manifests by hand, as `crates/mcp/tests/edges.rs`
//! reads them for `golf-mcp`. Comments are skipped, because other manifests name this crate in
//! prose (`crates/ocr`'s says whose verbs are whose).

use std::fs;
use std::path::PathBuf;

/// Whether one manifest line declares `golf-lab` as a dependency: by its path, by its package
/// name, or as the key of an entry.
fn declares_golf_lab(line: &str) -> bool {
    let line = line.trim();
    if line.starts_with('#') {
        return false;
    }
    let key = line.split('=').next().unwrap_or("").trim();
    line.contains("\"../lab\"")
        || line.contains("package = \"golf-lab\"")
        || (line.contains('=') && (key == "golf-lab" || key == "golf_lab"))
        || line.trim_matches(['[', ']']).ends_with(".golf-lab")
}

#[test]
fn no_crate_in_the_workspace_depends_on_golf_lab() {
    let crates = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut read = 0;
    for entry in fs::read_dir(&crates).expect("crates/") {
        let dir = entry.expect("an entry").path();
        let manifest = dir.join("Cargo.toml");
        if dir.file_name().is_some_and(|n| n == "lab") || !manifest.is_file() {
            continue;
        }
        let text = fs::read_to_string(&manifest).expect("a manifest");
        if let Some(line) = text.lines().find(|line| declares_golf_lab(line)) {
            panic!("{} depends on golf-lab: {line}", manifest.display());
        }
        read += 1;
    }
    assert!(read >= 12, "read only {read} manifests");
}

#[test]
fn the_check_sees_each_spelling_of_the_edge() {
    // Positive controls, so the walk above cannot pass because the match saw nothing.
    for line in [
        "golf-lab = { path = \"../lab\" }",
        "lab = { package = \"golf-lab\", path = \"x\" }",
        "golf_lab = \"0.1\"",
        "[dependencies.golf-lab]",
    ] {
        assert!(declares_golf_lab(line), "{line}");
    }
    for line in [
        "# The lab's verbs are `golf-lab`'s (P20 on)",
        "description = \"the reader's own seam, not golf-lab's\"",
    ] {
        assert!(!declares_golf_lab(line), "{line}");
    }
}
