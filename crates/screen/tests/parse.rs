//! The parser against every screen document frozen Python recorded. [M34 P5, P10]
//!
//! A document (`corpus/`, `reference/`, `synthetic/`) is one screen's boxes and the answer for
//! them. This runs the boxes through [`parse_screen`] and [`label_ratio`] over the shipped profile
//! and compares `expected.parsed` and `expected.label_ratio`. `expected.shot` is the validated
//! record, and `tests/read.rs` gates it (M34 P6).
//!
//! **Whose answers these are.** Frozen Python recorded them, the faithful port passed every one
//! (P5), and M34 P10 re-recorded them through this parser under `spec/declarations/screen-v1.json`,
//! which names every value the tie rule and the `Impact Position V` tile moved. From P8 to P10 this
//! file ran a `frozen` copy of the faithful rules instead, which P10 deleted. So every value here is
//! frozen Python's except the ones a document's `provenance.rerecords` ledger names, and
//! `tests/hand.rs` gates the change on vectors a person worked.
//!
//! # The rules
//!
//! `docs/CONFORMANCE.md` §3's, through [`golf_core::compare`], the comparator `golf-core rerecord`
//! uses: strings and lists exact, floats within `RTOL`. Every difference across every document is
//! accumulated and reported in one assertion, `tests/engine.rs`'s template.
//!
//! **Two floats are held to the bit as well, and that is not a stricter taste.** `RTOL` is sized for
//! a sum taken in a different order. `confidence` is the one place the parser sums anything, and
//! *which order CPython sums in* is the edge under test: compensated since 3.12, so a left fold
//! lands one ulp away on the synthetic screens' constant `0.95` (M34 P4's finding 6), which `RTOL`
//! would pass and `round(conf, 3)` and `< min_confidence` downstream would not. Since P10 the
//! recorded confidences are the re-record's, summed by [`pyfmt::sum`], which the format family holds
//! to CPython's own; a reader that went back to a left fold lands an ulp off them all the same.
//! [`the_confidence_and_the_label_ratio_are_held_to_the_bit`] is that gate. `label_ratio` is held
//! with it because it is one correctly rounded division of two counts, so a last bit away is a
//! different count.

mod common;

use golf_core::compare::compare;
use serde::Deserialize;
use serde_json::{json, Value};

use screen::orient::label_ratio;
use screen::parser::{parse_screen, ParsedShot};
use screen::profile::load_profile;
use screen::TextBox;

/// What the parser reads of a document's `input`. The rest (identity, notes, `min_confidence`) is
/// the validator's and the record's, which `tests/read.rs` reads whole as a `ScreenInput`.
#[derive(Deserialize)]
struct Input {
    device: String,
    boxes: Vec<TextBox>,
}

/// One document's boxes through the port, as `_run_screen` records them.
struct Run {
    parsed: ParsedShot,
    label_ratio: f64,
}

fn run(vector: &Value) -> Run {
    common::assert_recorded_for_this_parser(vector);
    let input: Input = serde_json::from_value(vector["input"].clone())
        .unwrap_or_else(|e| panic!("{}: input: {e}", vector["id"]));
    let profile = load_profile(&input.device).expect("every document names a shipped profile");
    Run {
        parsed: parse_screen(&input.boxes, profile),
        label_ratio: label_ratio(&input.boxes, profile),
    }
}

#[test]
fn every_recorded_parse_is_reproduced() {
    let documents = common::every_document();
    let mut failures = Vec::new();
    for (id, vector) in &documents {
        let run = run(vector);
        let actual = json!({
            "label_ratio": run.label_ratio,
            "parsed": common::parsed_json(&run.parsed),
        });
        let expected = json!({
            "label_ratio": vector["expected"]["label_ratio"],
            "parsed": vector["expected"]["parsed"],
        });
        failures.extend(
            compare(&expected, &actual)
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

/// See the module doc for why these two, and only these, are compared on bits.
#[test]
fn the_confidence_and_the_label_ratio_are_held_to_the_bit() {
    let mut failures = Vec::new();
    for (id, vector) in common::every_document() {
        let run = run(&vector);
        for (name, actual, expected) in [
            (
                "confidence",
                run.parsed.confidence,
                &vector["expected"]["parsed"]["confidence"],
            ),
            (
                "label_ratio",
                run.label_ratio,
                &vector["expected"]["label_ratio"],
            ),
        ] {
            let expected = expected.as_f64().expect("a recorded float");
            if actual.to_bits() != expected.to_bits() {
                failures.push(format!("{id}: {name} {actual:?}, recorded {expected:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A sub-family nothing runs fails here rather than hiding, `tests/format.rs`'s discovery pin for
/// the screen family. The documents are this file's and `units/` is `tests/units.rs`'s, which pins
/// its own tables by name; `tests/read.rs` runs the same documents through the whole read, and
/// `tests/hand.rs` runs `hand/` (M34 P8).
#[test]
fn every_screen_sub_family_is_run_by_a_test() {
    let mut ours: Vec<&str> = common::DOCUMENTS.to_vec();
    ours.extend([common::HAND, "units"]);
    ours.sort_unstable();
    assert_eq!(
        common::names(&common::screen_dir()),
        ours,
        "a screen sub-family was added or renamed and no runner was told about it"
    );
}
