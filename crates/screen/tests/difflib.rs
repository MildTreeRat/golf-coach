//! `difflib::ratio` against `spec/vectors/format/difflib_ratio.json`, on bits. [M34 P3]
//!
//! The format family's one table this crate implements (`provenance.implemented_by: "screen"`);
//! `crates/pyfmt/tests/format.rs` runs the rest and names this file as the runner of this one.
//! No tolerance, for `pyfmt`'s reason: `ratio` adds nothing up. It is a count of matched code
//! points doubled and divided once, so any difference is a different count, which is a different
//! block search.

use std::fs;
use std::path::Path;

use serde::Deserialize;

use screen::difflib::ratio;

#[derive(Deserialize)]
struct Table {
    id: String,
    provenance: Provenance,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Provenance {
    implemented_by: String,
}

/// The two strings as recorded, which JSON carries exactly, and the ratio as its Python `repr`.
#[derive(Deserialize)]
struct Case {
    a: String,
    b: String,
    expected: String,
}

fn table() -> Table {
    // `CARGO_MANIFEST_DIR` is `crates/screen`; the spec is two levels up, beside `pyproject.toml`.
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/vectors/format/difflib_ratio.json");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path:?}: {e}"))
}

/// Every case, with every failure reported rather than the first, because a wrong tie-break shows
/// as a pattern across many cases and one of them says little.
#[test]
fn every_ratio_is_cpythons_to_the_bit() {
    let table = table();
    assert_eq!(table.id, "format/difflib_ratio");
    assert_eq!(table.provenance.implemented_by, "screen");
    assert!(!table.cases.is_empty());

    let failures: Vec<String> = table
        .cases
        .iter()
        .filter_map(|case| {
            let expected: f64 = case.expected.parse().expect("`expected` is a float repr");
            let actual = ratio(&case.a, &case.b);
            (actual.to_bits() != expected.to_bits()).then(|| {
                format!(
                    "{:?} vs {:?}: {actual:?}, CPython {expected:?}",
                    case.a, case.b
                )
            })
        })
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} cases differ:\n{}",
        failures.len(),
        table.cases.len(),
        failures.join("\n")
    );
}

/// `autojunk` is ported rather than refused, and it fires only at `len(b) >= 200`, which no label
/// reaches. So the table's long crafted cases are the only thing gating it, and a re-recording
/// that dropped them would leave it gated by nothing while every test stayed green.
#[test]
fn the_table_still_reaches_autojunk() {
    let long = table()
        .cases
        .iter()
        .filter(|case| case.b.chars().count() >= 200)
        .count();
    assert!(long > 0, "no case has a `b` of 200 code points or more");
}
