//! Nothing that ships depends on `golf-lab` (the M29 plan's rule on cargo edges, and D13). [M29 P20]
//!
//! It is a shell: it may hold any library crate, `ocr`'s `ort` among them, so a crate that depended
//! on it would reach all of them, and `ort` with them, which `crates/ocr/tests/ort_reach.rs` would
//! catch only for the crates it guards. Read from the manifests by hand, as `crates/mcp/tests/edges.rs`
//! reads them for `golf-mcp`. Comments are skipped, because other manifests name this crate in
//! prose (`crates/ocr`'s says whose verbs are whose).
//!
//! **A `[dev-dependencies]` table is not read, since M38 P13**, which is `ort_reach`'s own line: a
//! dev-dependency is never linked into what ships, so it cannot carry `ort` anywhere. The M38 plan's
//! exit needs it, because half of that exit is what the real `import-export` makes of what `app`
//! exports. So that this does not become a door, `the_one_dev_edge_is_the_facades_round_trip` names
//! the one crate holding such an edge, and a second has to come here and say why.

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

/// The manifest's lines outside any `dev-dependencies` table, whichever spelling it takes
/// (`[dev-dependencies]`, `[dev-dependencies.x]`, `[target.'cfg(unix)'.dev-dependencies]`).
fn shipped_lines(manifest: &str) -> Vec<&str> {
    let mut in_dev = false;
    manifest
        .lines()
        .filter(|line| {
            let line = line.trim();
            if line.starts_with('[') {
                in_dev = line
                    .trim_matches(['[', ']'])
                    .split('.')
                    .any(|part| part == "dev-dependencies");
            }
            !in_dev
        })
        .collect()
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
        let declared = shipped_lines(&text);
        if let Some(line) = declared.iter().find(|line| declares_golf_lab(line)) {
            panic!("{} depends on golf-lab: {line}", manifest.display());
        }
        read += 1;
    }
    assert!(read >= 12, "read only {read} manifests");
}

/// Every crate whose manifest names the lab in a dev table, against the module doc's one.
#[test]
fn the_one_dev_edge_is_the_facades_round_trip() {
    let crates = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut holding: Vec<String> = Vec::new();
    for entry in fs::read_dir(&crates).expect("crates/") {
        let dir = entry.expect("an entry").path();
        let manifest = dir.join("Cargo.toml");
        if dir.file_name().is_some_and(|n| n == "lab") || !manifest.is_file() {
            continue;
        }
        let text = fs::read_to_string(&manifest).expect("a manifest");
        if text.lines().any(declares_golf_lab) {
            holding.push(dir.file_name().unwrap().to_string_lossy().into_owned());
        }
    }
    assert_eq!(holding, ["app"]);
}

/// Each table's spelling, shipped and not, and a shipped table straight after a dev one, so the walk
/// cannot pass by never leaving the dev table it entered.
#[test]
fn the_check_reads_every_table_but_a_dev_one() {
    let declares = |manifest: &str| shipped_lines(manifest).into_iter().any(declares_golf_lab);
    let edge = "golf-lab = { path = \"../lab\" }";
    for table in [
        "[dependencies]",
        "[build-dependencies]",
        "[target.'cfg(unix)'.dependencies]",
    ] {
        assert!(declares(&format!("{table}\n{edge}\n")), "{table}");
        assert!(
            declares(&format!(
                "[dev-dependencies]\ntempfile = \"3\"\n{table}\n{edge}\n"
            )),
            "{table} after a dev table"
        );
    }
    assert!(declares("[dependencies.golf-lab]\npath = \"../lab\"\n"));
    for table in [
        "[dev-dependencies]",
        "[target.'cfg(unix)'.dev-dependencies]",
    ] {
        assert!(!declares(&format!("{table}\n{edge}\n")), "{table}");
    }
    assert!(!declares(
        "[dev-dependencies.golf-lab]\npath = \"../lab\"\n"
    ));
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
