//! What the screen family's runners share: where it is on disk, and how a file is read. [M34 P5]

// Each test binary that includes this uses part of it.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use contracts::shot::SCREEN_PARSER_VERSION;
use serde_json::{json, Map, Value};

use screen::parser::{FieldValue, ParsedShot};

/// `spec/vectors/screen/`. `CARGO_MANIFEST_DIR` is `crates/screen`, two levels below the repo root.
pub fn screen_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/vectors/screen")
}

pub fn read(path: &Path) -> Value {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path:?}: {e}"))
}

/// The names in a directory, sorted: file stems for a sub-family's vectors, or its sub-directories.
pub fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {dir:?}: {e}"))
        .map(|entry| {
            let path = entry.expect("a directory entry").path();
            let stem = path.file_stem().expect("a named entry");
            stem.to_string_lossy().into_owned()
        })
        .collect();
    names.sort();
    names
}

/// Every vector in one sub-family, by file stem, sorted, each checked to be the file it says it is.
pub fn sub_family(name: &str) -> Vec<(String, Value)> {
    let dir = screen_dir().join(name);
    let vectors: Vec<(String, Value)> = names(&dir)
        .into_iter()
        .map(|stem| {
            let vector = read(&dir.join(format!("{stem}.json")));
            assert_eq!(
                vector["id"],
                format!("screen/{name}/{stem}"),
                "a vector's `id` is its path"
            );
            (stem, vector)
        })
        .collect();
    assert!(!vectors.is_empty(), "spec/vectors/screen/{name}/ is empty");
    vectors
}

/// The three sub-families shaped as documents: one screen's `input` and its `expected`, recorded by
/// frozen Python and re-recorded through this reader by `golf-core rerecord` (M34 P10), whose ledger
/// in `provenance.rerecords` names the values that are Rust's. `units/` is `tests/units.rs`'s.
pub const DOCUMENTS: [&str; 3] = ["corpus", "reference", "synthetic"];

/// Every document, by its `id`, in sub-family then file order.
pub fn every_document() -> Vec<(String, Value)> {
    DOCUMENTS
        .iter()
        .flat_map(|family| sub_family(family))
        .map(|(stem, vector)| (vector["id"].as_str().unwrap_or(&stem).to_string(), vector))
        .collect()
}

/// The hand-worked sub-family (M34 P8): documents shaped as the others are, whose `expected` a
/// person worked out, gating the shipping parser. `tests/hand.rs`'s.
pub const HAND: &str = "hand";

/// A document gates the parser that ships only if it was recorded for it: one at another
/// `screen_parser_version` holds answers a declaration has since moved, and passing it would certify
/// the reader against a retracted answer (ADR-030 §8's reason for re-recording with the bump).
pub fn assert_recorded_for_this_parser(vector: &Value) {
    assert_eq!(
        vector["screen_parser_version"], SCREEN_PARSER_VERSION,
        "{}: recorded for another parser version; re-record it with `golf-core rerecord`",
        vector["id"]
    );
}

/// `expected.parsed`'s shape: `ParsedShot` before validation, warnings without the notes, and
/// neither `device`, `needs_review` nor `fields_present`, which `_run_screen` does not record and
/// `golf_core::rerecord::run_screen` leaves out with it. `fields_present` reaches a document through
/// `expected.shot.provenance`.
pub fn parsed_json(parsed: &ParsedShot) -> Value {
    let values: Map<String, Value> = parsed
        .values
        .iter()
        .map(|(key, value)| {
            let value = match value {
                FieldValue::Number(number) => json!(number),
                FieldValue::Text(text) => json!(text),
            };
            (key.to_string(), value)
        })
        .collect();
    let raw_fields: Map<String, Value> = parsed
        .raw_fields
        .iter()
        .map(|(label, text)| (label.to_string(), json!(text)))
        .collect();
    json!({
        "values": values,
        "raw_fields": raw_fields,
        "confidence": parsed.confidence,
        "warnings": parsed.warnings,
    })
}
