//! The whole read against every screen document frozen Python recorded. [M34 P6, P10]
//!
//! `tests/parse.rs` gates the parser on `expected.parsed`. This gates [`screen::read`] on
//! `expected.shot`: the parse, the notes put first, `validate_parse`, and the `ShotData` the reader
//! stores, or `null` where it reads nothing (`import_screen`'s `failed`). Each document's whole
//! `input` is read as a [`ScreenInput`], so a vector shaped differently from the library's argument
//! fails here by name. The rules are `docs/CONFORMANCE.md` §3's, through [`golf_core::compare`], and
//! every difference across every document is reported in one assertion.
//!
//! **No allowance, since M34 P10.** Until then these documents held frozen Python's records, which
//! lack the ten keys M32 gave Rust's `ShotData`, and this file let exactly those keys through at
//! their defaults. P10's re-record (`spec/declarations/screen-v1.json`) added them to every recorded
//! shot, so every key the reader writes is compared, and the allowance went with the faithful port
//! it served, as `crates/contracts/tests/round_trip.rs`' allowance for `input.shot` is the same door
//! for the engine vectors.
//!
//! # `parse_confidence` is held to the bit as well
//!
//! For `tests/parse.rs`' reason, one step later. It is `round(conf, 3)` of a confidence the
//! validator has already moved, and `RTOL` cannot see an ulp. It is a rounded three-place value, so
//! the bits are the right test of it rather than a stricter taste.

mod common;

use golf_core::compare::compare;
use serde_json::{json, Value};

use screen::{read, ScreenInput};

/// One document's input through the whole read, under the document's own root.
fn run(vector: &Value) -> Value {
    common::assert_recorded_for_this_parser(vector);
    let input: ScreenInput = serde_json::from_value(vector["input"].clone())
        .unwrap_or_else(|e| panic!("{}: input: {e}", vector["id"]));
    let shot = read(&input).unwrap_or_else(|e| panic!("{}: {e}", vector["id"]));
    json!({"expected": {"shot": shot}})
}

fn recorded(vector: &Value) -> Value {
    json!({"expected": {"shot": vector["expected"]["shot"]}})
}

/// The value at a dotted path, which here never crosses a list.
fn at<'a>(document: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.')
        .try_fold(document, |node, step| node.get(step))
}

#[test]
fn every_recorded_shot_is_read() {
    let documents = common::every_document();
    let mut failures = Vec::new();
    for (id, vector) in &documents {
        failures.extend(
            compare(&recorded(vector), &run(vector))
                .into_iter()
                .map(|difference| format!("{id}: {difference}")),
        );
    }
    assert!(
        failures.is_empty(),
        "{} differences across {} documents:\n{}",
        failures.len(),
        documents.len(),
        failures.join("\n")
    );
}

/// See the module doc for why this one float is compared on bits.
#[test]
fn the_parse_confidence_is_held_to_the_bit() {
    let mut failures = Vec::new();
    for (id, vector) in common::every_document() {
        let (read, recorded) = (run(&vector), recorded(&vector));
        let path = "expected.shot.provenance.parse_confidence";
        let (Some(actual), Some(expected)) = (at(&read, path), at(&recorded, path)) else {
            continue;
        };
        let (actual, expected) = (
            actual.as_f64().expect("a float"),
            expected.as_f64().expect("a recorded float"),
        );
        if actual.to_bits() != expected.to_bits() {
            failures.push(format!("{id}: {actual:?}, recorded {expected:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
