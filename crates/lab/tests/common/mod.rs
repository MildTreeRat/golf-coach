//! What the ignored gates share: the values a re-read of a stored shot is declared to move.
//! [M29 P25, P29, P30]
//!
//! `tests/inputs.rs` (P25) holds the bundle pipeline's engine input to the committed corpus vector,
//! whose shot is frozen Python's stored shot, and excuses exactly these values there. P29 wrote the
//! re-reads into the store, so `tests/reread.rs` (P23, rewritten at P29) now finds each value's
//! re-read side in the store, where until P29 it excused the move in a dry run. Both fail if one is
//! never seen, so the list is written once, here, rather than once per gate. `tests/build_vectors.rs`
//! (P30) excuses the same shot paths between the vectors built from `data/` and the committed ones,
//! so the re-read's provenance text and the keys Rust's shot adds moved here from `tests/inputs.rs`
//! with it.

/// A value the re-read moves, against the stored shot, by the stored shot's id.
pub struct DeclaredValue {
    pub shot: &'static str,
    pub key: &'static str,
    /// Both sides as JSON literals.
    pub stored: &'static str,
    pub reread: &'static str,
}

/// Every value a re-read may move, and why.
///
/// - `2026-08-10-1`'s `shot_type` is M34's label fix: the frozen parser spilled the `Impact
///   Position` tile's `CENTER` into the tile beside it (`spec/declarations/screen-v1.json`).
/// - `2026-08-10-1`'s `impact_position` is the Rust reader reading the `V` of the `Impact Position
///   V` label, which PaddleOCR's five models read as `Impact Position`, so the frozen parse tied
///   and withheld both tiles. The photo shows `CENTER` and `---`; the user's (a) at M29 P7, and
///   `crates/ocr/tests/read.rs`' `DECLARED_DIFFERENCES` holds it against the screen vector.
/// - `2026-08-23-1`'s `impact_position` is M34's tie rule: two boxes the evidence cannot tell apart
///   withhold the tile, where the frozen parser gave it to the first (`HEEL`).
pub const DECLARED_VALUES: [DeclaredValue; 3] = [
    DeclaredValue {
        shot: "2026-08-10-1",
        key: "shot_type",
        stored: r#""CENTER SLIGHT FADE""#,
        reread: r#""SLIGHT FADE""#,
    },
    DeclaredValue {
        shot: "2026-08-10-1",
        key: "impact_position",
        stored: "null",
        reread: r#""CENTER""#,
    },
    DeclaredValue {
        shot: "2026-08-23-1",
        key: "impact_position",
        stored: r#""HEEL""#,
        reread: "null",
    },
];

/// Every directory under `sessions` holding a manifest, sorted, as both gates walk the store.
#[allow(dead_code)] // `tests/build_vectors.rs` walks the store through the builder.
pub fn swing_dirs(sessions: &std::path::Path) -> Vec<std::path::PathBuf> {
    use std::fs;
    assert!(
        sessions.is_dir(),
        "{} is not a directory",
        sessions.display()
    );
    let mut found = Vec::new();
    let mut session_dirs: Vec<_> = fs::read_dir(sessions)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect();
    session_dirs.sort();
    for session in session_dirs {
        let mut swings: Vec<_> = fs::read_dir(&session)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| storage::manifest::manifest_path(path).is_file())
            .collect();
        swings.sort();
        found.extend(swings);
    }
    assert!(!found.is_empty(), "no bundle under {}", sessions.display());
    found
}

/// The re-read's provenance text, each change of which `tests/reread.rs` held to a listed kind
/// until P29 (the M29 plan's P23 finding 3 is that measurement).
#[allow(dead_code)] // `tests/reread.rs` reads the store's shot, never a committed one.
pub const REREAD_TEXT: [&str; 3] = [
    "provenance.warnings",
    "provenance.raw_fields",
    "provenance.parse_confidence",
];

/// The keys `v17.json` added to `expected.swing.shot`, as paths inside a shot: what Rust's
/// `ShotData` writes and the frozen input lacks.
#[allow(dead_code)] // `tests/reread.rs` reads the store's shot, never a committed one.
pub fn rust_only_shot_keys() -> std::collections::BTreeSet<String> {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/declarations/v17.json");
    let declaration: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).expect("v17.json is committed")).unwrap();
    let keys: std::collections::BTreeSet<String> = declaration["added"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|path| path.as_str()?.strip_prefix("expected.swing.shot."))
        .map(str::to_string)
        .collect();
    assert!(
        keys.contains("provenance.parser_version"),
        "v17.json no longer adds the stamp: {keys:?}"
    );
    keys
}
