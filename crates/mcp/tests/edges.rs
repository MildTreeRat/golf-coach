//! Nothing depends on `golf-mcp` (the M29 plan's rule on cargo edges, and D13). [M29 P9]
//!
//! It is a shell like `golf-core`: it may hold any library crate, so a crate that depended on it
//! would reach all of them through it, which is the cycle ADR-008's rule exists to prevent. Read
//! from the manifests by hand, as `crates/ocr/tests/ort_reach.rs` reads them, because a path
//! dependency is one line in this workspace's spelling.

use std::fs;
use std::path::PathBuf;

#[test]
fn no_crate_in_the_workspace_depends_on_golf_mcp() {
    let crates = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut read = 0;
    for entry in fs::read_dir(&crates).expect("crates/") {
        let dir = entry.expect("an entry").path();
        let manifest = dir.join("Cargo.toml");
        if dir.file_name().is_some_and(|n| n == "mcp") || !manifest.is_file() {
            continue;
        }
        let text = fs::read_to_string(&manifest).expect("a manifest");
        assert!(
            !text.contains("\"../mcp\"") && !text.contains("golf-mcp"),
            "{} depends on golf-mcp",
            manifest.display()
        );
        read += 1;
    }
    assert!(read >= 11, "read only {read} manifests");
}
