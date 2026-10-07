//! The parser's private functions against `spec/vectors/screen/units/`, exactly. [M34 P5]
//!
//! Each table is one of `parser.py`'s or `profiles.py`'s functions over crafted inputs, recorded by
//! frozen Python with the screen family, for the edges no screen reaches often enough to be gated by
//! one: a Unicode digit, a comma at and off the thousands boundary, a one-letter sign token against
//! `CLOSED`, every blank marker spaced and unspaced, `//` where it parts from a floored division.
//!
//! **Every case carries what it ran against.** `field_for` names a profile in the table's own
//! `profiles`, and `sign_from` and `is_blank` carry their field and markers inline, so the
//! `Impact Position V` tile the fork gains in M34 P8 cannot move an answer here.
//!
//! No tolerance anywhere, `tests/format.rs`'s rule: nothing here sums anything. Floats travel as
//! their Python `repr` and are compared on bits, so a `-0.0` is not a `0.0`.

mod common;

use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::collections::BTreeMap;

use screen::parser::{cell_text, first_number, is_blank, sign_from};
use screen::profile::{normalize_label, DeviceProfile, ProfileField};
use screen::TextBox;

/// The tables this file runs, one test each.
const TABLES: [&str; 6] = [
    "cell_text",
    "field_for",
    "first_number",
    "is_blank",
    "normalize_label",
    "sign_from",
];

#[derive(Deserialize)]
struct Table<C> {
    cases: Vec<C>,
}

fn table<C: DeserializeOwned>(name: &str) -> Vec<C> {
    let path = common::screen_dir().join(format!("units/{name}.json"));
    let vector = common::read(&path);
    // Frozen Python's answers, and only ever its: `golf-core rerecord` never reads `units/`
    // (`SCREEN_UNREAD`), so a table here stays at the unversioned 0 it was recorded at.
    assert_eq!(vector["provenance"]["oracle"], "python", "{name}");
    assert_eq!(vector["screen_parser_version"], 0, "{name}");
    assert_eq!(vector["id"], format!("screen/units/{name}"));
    let table: Table<C> = serde_json::from_value(vector).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(!table.cases.is_empty(), "{name} has no cases");
    table.cases
}

fn bits(repr: &str) -> u64 {
    repr.parse::<f64>()
        .unwrap_or_else(|e| panic!("{repr:?} is not a float repr: {e}"))
        .to_bits()
}

/// Every failure in a table, reported together: a wrong rule shows as a pattern across cases.
fn assert_none_failed(name: &str, total: usize, failures: Vec<String>) {
    assert!(
        failures.is_empty(),
        "{name}: {} of {total} cases differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn every_units_table_is_run_by_a_test_in_this_file() {
    assert_eq!(
        common::names(&common::screen_dir().join("units")),
        TABLES,
        "a units table was added or renamed and this file was not told about it"
    );
}

#[derive(Deserialize)]
struct NormalizeCase {
    text: String,
    expected: String,
}

#[test]
fn normalize_label_table() {
    let cases: Vec<NormalizeCase> = table("normalize_label");
    let failures = cases
        .iter()
        .filter_map(|case| {
            let actual = normalize_label(&case.text);
            (actual != case.expected)
                .then(|| format!("{:?}: {actual:?}, Python {:?}", case.text, case.expected))
        })
        .collect();
    assert_none_failed("normalize_label", cases.len(), failures);
}

#[derive(Deserialize)]
struct FieldForCase {
    profile: String,
    text: String,
    label: Option<String>,
    score: Option<String>,
}

/// `field_for` against the profiles the table carries, never the shipped fork.
#[test]
fn field_for_table() {
    let path = common::screen_dir().join("units/field_for.json");
    let profiles: BTreeMap<String, DeviceProfile> =
        serde_json::from_value(common::read(&path)["profiles"].clone()).expect("`profiles`");
    let cases: Vec<FieldForCase> = table("field_for");
    let failures = cases
        .iter()
        .filter_map(|case| {
            let profile = &profiles[&case.profile];
            let matched = profile.field_for(&case.text);
            let actual = matched.map(|field| (field.label.clone(), field.matches(&case.text)));
            let agrees = match (&actual, &case.label, &case.score) {
                (None, None, None) => true,
                (Some((label, score)), Some(want_label), Some(want_score)) => {
                    label == want_label && score.to_bits() == bits(want_score)
                }
                _ => false,
            };
            (!agrees).then(|| {
                format!(
                    "{} {:?}: {actual:?}, Python {:?} at {:?}",
                    case.profile, case.text, case.label, case.score
                )
            })
        })
        .collect();
    assert_none_failed("field_for", cases.len(), failures);
}

#[derive(Deserialize)]
struct NumberCase {
    text: String,
    expected: Option<FoundNumber>,
}

#[derive(Deserialize)]
struct FoundNumber {
    value: String,
    matched: String,
}

#[test]
fn first_number_table() {
    let cases: Vec<NumberCase> = table("first_number");
    let failures = cases
        .iter()
        .filter_map(|case| {
            let actual = first_number(&case.text);
            let agrees = match (actual, &case.expected) {
                (None, None) => true,
                (Some((value, matched)), Some(want)) => {
                    value.to_bits() == bits(&want.value) && matched == want.matched
                }
                _ => false,
            };
            (!agrees).then(|| {
                let want = case
                    .expected
                    .as_ref()
                    .map(|want| (want.value.as_str(), want.matched.as_str()));
                format!("{:?}: {actual:?}, Python {want:?}", case.text)
            })
        })
        .collect();
    assert_none_failed("first_number", cases.len(), failures);
}

#[derive(Deserialize)]
struct SignCase {
    residual: String,
    field: ProfileField,
    expected: Option<i64>,
}

#[test]
fn sign_from_table() {
    let cases: Vec<SignCase> = table("sign_from");
    let failures = cases
        .iter()
        .filter_map(|case| {
            let actual = sign_from(&case.residual, &case.field);
            (actual != case.expected).then(|| {
                format!(
                    "{} {:?}: {actual:?}, Python {:?}",
                    case.field.label, case.residual, case.expected
                )
            })
        })
        .collect();
    assert_none_failed("sign_from", cases.len(), failures);
}

#[derive(Deserialize)]
struct BlankCase {
    text: String,
    blank_markers: Vec<String>,
    expected: bool,
}

#[test]
fn is_blank_table() {
    let cases: Vec<BlankCase> = table("is_blank");
    let failures = cases
        .iter()
        .filter_map(|case| {
            let actual = is_blank(&case.text, &case.blank_markers);
            (actual != case.expected).then(|| {
                format!(
                    "{:?} against {:?}: {actual}, Python {}",
                    case.text, case.blank_markers, case.expected
                )
            })
        })
        .collect();
    assert_none_failed("is_blank", cases.len(), failures);
}

#[derive(Deserialize)]
struct CellCase {
    case: String,
    label_box: TextBox,
    value_boxes: Vec<TextBox>,
    expected: String,
}

#[test]
fn cell_text_table() {
    let cases: Vec<CellCase> = table("cell_text");
    let failures = cases
        .iter()
        .filter_map(|case| {
            let value_boxes: Vec<&TextBox> = case.value_boxes.iter().collect();
            let actual = cell_text(&case.label_box, &value_boxes);
            (actual != case.expected)
                .then(|| format!("{}: {actual:?}, Python {:?}", case.case, case.expected))
        })
        .collect();
    assert_none_failed("cell_text", cases.len(), failures);
}
