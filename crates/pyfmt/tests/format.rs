//! `pyfmt` against `spec/vectors/format/`. [M22 P3]
//!
//! The Rust half of ADR-032 §3. `scripts/conformance.py check` reports these vectors and defers
//! them here for the same reason it defers the audio family: the implementation under test is
//! Rust, and running CPython against CPython would gate nothing.
//!
//! **Five tables for the engine's four edges.** ADR-032 §3 names three; `repr.json` is M22 P5's
//! fourth, an f-string with no format spec at all. (P4 found one more — Python's `max` returns the
//! first maximum — which is not CPython *formatting* and lives in `phases.rs` instead.)
//!
//! **And five for the screen parser's**, from M34 P2: `str_repr`, `floor_div`, `text_case` and
//! `sum` are this crate's, and `difflib_ratio` is `crates/screen`'s. A table names its implementing
//! crate in `provenance.implemented_by`; the five engine tables predate the key, and its absence
//! means this crate. The parser's tables also break the rule below in one place: their inputs are
//! **strings**, which JSON carries exactly, so only the floats travel as `repr`.
//!
//! **There is no tolerance in this file, and that is deliberate.** `docs/CONFORMANCE.md` §3's
//! `RTOL = 1e-9` is sized for a different *summation order* — the last bits of a float that was
//! added up in another sequence. Nothing here adds anything up. A rounding rule either matches or
//! it is a different rule, so every comparison is `==` on bits or on bytes of text, and the
//! vectors carry every float as its Python `repr` so that is available: `repr` is
//! shortest-round-trip and Rust's `str::parse` is correctly rounded, so both sides land on the
//! identical f64. It also sidesteps `serde_json` rejecting the bare `NaN` that `json.dumps` would
//! have written, which matters because non-finite input is one of the two cases where these two
//! languages genuinely print different text.
//!
//! **Every ordering case is a tie.** A sort with distinct keys agrees under any algorithm; it is
//! only a tie that reads the insertion order back out, which is the answer ADR-032 §3's third edge
//! is about.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use pyfmt::{
    fixed, floor_div, g, is_space, registry_rank, repr, round_half_even, round_index, round_to,
    split, str_repr, strip, strip_space, sum, upper, OrderedMap,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;

/// One table: its cases, in whichever shape that table records.
#[derive(Deserialize)]
struct Table<C> {
    id: String,
    #[allow(dead_code)]
    python_version: String,
    cases: Vec<C>,
}

/// The engine tables' one flat shape — see [`Case`].
type Vector = Table<Case>;

/// Just enough of a vector to say which crate implements it.
#[derive(Deserialize)]
struct Header {
    provenance: Provenance,
}

#[derive(Deserialize)]
struct Provenance {
    implemented_by: Option<String>,
}

/// One flat shape for every family rather than one per family, because `serde` can tell them apart
/// by which keys are present and an enum would have to be told. `expected` is a
/// `serde_json::Value` for the same reason: it is an `i64` in the one-argument rounding cases, a
/// `repr` string in the two-argument ones, and text everywhere else.
#[derive(Deserialize)]
struct Case {
    // Edges 1 and 2.
    value: Option<String>,
    /// `null` on the one-argument `round(x)` cases and an integer on the two-argument ones.
    ///
    /// A plain `Option`, not the two-deep one P2's round trip needed: that distinction exists to
    /// keep a `null` from being rewritten as an absent key, and nothing here writes a vector back.
    /// The rounding family carries this key on every case, and the `expected` **type** is the
    /// discriminator either way — an integer for the one-argument form and a `repr` string for the
    /// other — so a case that lost the key fails on that rather than passing quietly.
    ndigits: Option<usize>,
    precision: Option<usize>,
    expected: Option<serde_json::Value>,
    // Edge 3.
    key: Option<String>,
    entries: Option<Vec<(String, String)>>,
    registry: Option<Vec<String>>,
    names: Option<Vec<String>>,
    expected_order: Option<Vec<String>>,
    expected_first: Option<String>,
}

impl Case {
    /// The input float, parsed from the `repr` the vector carries.
    ///
    /// Rust accepts `inf`, `-inf` and `nan` — the three `repr` writes for the non-finite values —
    /// which is the property that lets one encoding cover the whole table.
    fn value(&self) -> f64 {
        let text = self.value.as_deref().expect("case carries no `value`");
        text.parse()
            .unwrap_or_else(|e| panic!("cannot parse {text:?}: {e}"))
    }

    fn expected_text(&self) -> &str {
        self.expected
            .as_ref()
            .expect("case carries no `expected`")
            .as_str()
            .expect("`expected` is not a string")
    }

    /// Compare two f64 by **bits**, not by `==`, so a NaN matches a NaN and `-0.0` does not match
    /// `0.0`. Both distinctions are real for the two-argument form: `round(nan, 4)` is a case, and
    /// `round(-0.04, 1)` is `-0.0` in Python, so a port that normalizes the sign passes a `==` test
    /// and then writes `0.0` where the engine serialized `-0.0`.
    ///
    /// Which is exactly why the **one-argument** form is not compared this way — see [`rounding`].
    fn assert_same_float(&self, actual: f64, expected: f64, what: &str) {
        assert!(
            actual.to_bits() == expected.to_bits(),
            "{what}: {:?} -> {actual:?}, expected {expected:?}",
            self.value.as_deref().unwrap_or("?")
        );
    }
}

fn spec_dir() -> PathBuf {
    // `CARGO_MANIFEST_DIR` is `crates/pyfmt`; the spec is two levels up, beside `pyproject.toml`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/pyfmt has a grandparent")
        .join("spec")
        .join("vectors")
        .join("format")
}

fn table<C: DeserializeOwned>(name: &str) -> Table<C> {
    load(name)
}

fn vector(name: &str) -> Vector {
    table(name)
}

fn load<T: DeserializeOwned>(name: &str) -> T {
    let path = spec_dir().join(format!("{name}.json"));
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path:?}: {e}"))
}

/// A float the vector carries as its Python `repr`.
fn float(text: &str) -> f64 {
    text.parse()
        .unwrap_or_else(|e| panic!("cannot parse {text:?}: {e}"))
}

/// Discovered rather than listed, the same choice `conformance.vector_paths` makes: a vector that
/// exists on disk and in no index is a vector nothing runs. This is the test that fails when a
/// family is added and nothing is taught to run it.
///
/// **Split by implementing crate since M34 P2**, because the family is no longer all this crate's.
/// The tables another crate implements are listed by name and crate, so moving one, or adding one
/// for a crate, fails here too. `difflib_ratio`'s runner is `crates/screen/tests/difflib.rs`, since
/// M34 P3; between P2 recording it and P3 it was committed and run by nothing, and this list is
/// where that was written down.
#[test]
fn every_committed_format_vector_is_run_by_a_test_in_this_file() {
    let mut ours: Vec<String> = Vec::new();
    let mut elsewhere: Vec<(String, String)> = Vec::new();
    for entry in fs::read_dir(spec_dir())
        .expect("cannot read spec/vectors/format")
        .filter_map(|entry| entry.ok())
    {
        let name = entry.file_name().to_string_lossy().replace(".json", "");
        let header: Header = load(&name);
        match header.provenance.implemented_by.as_deref() {
            None | Some("pyfmt") => ours.push(name),
            Some(other) => elsewhere.push((name, other.to_string())),
        }
    }
    ours.sort();
    elsewhere.sort();
    assert_eq!(
        ours,
        [
            "fixed",
            "floor_div",
            "general",
            "ordering",
            "repr",
            "rounding",
            "str_repr",
            "sum",
            "text_case"
        ],
        "a format vector was added or renamed and this file was not told about it"
    );
    assert_eq!(
        elsewhere,
        [("difflib_ratio".to_string(), "screen".to_string())],
        "a format vector names another crate and this file was not told about it"
    );
}

#[test]
fn rounding() {
    let vector = vector("rounding");
    let mut one_arg = 0;
    let mut two_arg = 0;
    for case in &vector.cases {
        match case.ndigits {
            None => {
                let expected = case
                    .expected
                    .as_ref()
                    .and_then(serde_json::Value::as_i64)
                    .expect("one-argument `expected` is not an integer");
                let value = case.value();
                // The index form exactly, because that is the shape Python's answer has: the
                // one-argument `round` returns an `int`, and the seventeen frame-index sites want
                // an integer out of it.
                assert_eq!(
                    round_index(value),
                    expected,
                    "round(x) as an index: {:?}",
                    case.value.as_deref().unwrap_or("?")
                );
                // And the float form by value rather than by bits, which is the one place this
                // file relaxes a comparison and it is not a tolerance. `round(-0.49999999999999994)`
                // is the `int` `0` in Python, which has no sign to carry, where `round_ties_even`
                // legitimately answers `-0.0`. Asserting bits here would be asserting a sign the
                // specification does not have — and the two-argument form, which *does* return a
                // float, is still compared by bits two arms down.
                assert!(
                    round_half_even(value) == expected as f64,
                    "round(x): {:?} -> {:?}, expected {expected}",
                    case.value.as_deref().unwrap_or("?"),
                    round_half_even(value)
                );
                one_arg += 1;
            }
            Some(ndigits) => {
                let expected: f64 = case.expected_text().parse().expect("`expected` repr");
                case.assert_same_float(
                    round_to(case.value(), ndigits),
                    expected,
                    &format!("round(x, {ndigits})"),
                );
                two_arg += 1;
            }
        }
    }
    assert!(
        one_arg > 100 && two_arg > 500,
        "{} cases",
        vector.cases.len()
    );
    println!(
        "{}: {one_arg} one-argument, {two_arg} two-argument",
        vector.id
    );
}

#[test]
fn fixed_point() {
    let vector = vector("fixed");
    for case in &vector.cases {
        let precision = case.precision.expect("case carries no `precision`");
        let actual = fixed(case.value(), precision);
        assert_eq!(
            actual,
            case.expected_text(),
            "{:?} at .{precision}f",
            case.value.as_deref().unwrap_or("?")
        );
    }
    println!("{}: {} cases", vector.id, vector.cases.len());
}

#[test]
fn general() {
    let vector = vector("general");
    for case in &vector.cases {
        let actual = g(case.value());
        assert_eq!(
            actual,
            case.expected_text(),
            "{:?} at :g",
            case.value.as_deref().unwrap_or("?")
        );
    }
    println!("{}: {} cases", vector.id, vector.cases.len());
}

/// Edge 4: `str(x)` on a float, which three of `mechanics.py`'s sentences reach with no format
/// spec.
///
/// **A port that used Rust's `{}` passes every other vector in this repo**, because the two agree
/// on every band edge shipping today. What separates them is in this table and nowhere else: an
/// integral value, where CPython writes `4.0` and Rust writes `4`, and the exponent window at
/// `decpt <= -4 || decpt > 16`, which Rust's `{}` does not have.
#[test]
fn representation() {
    let vector = vector("repr");
    for case in &vector.cases {
        let actual = repr(case.value());
        assert_eq!(
            actual,
            case.expected_text(),
            "{:?} at str()",
            case.value.as_deref().unwrap_or("?")
        );
    }
    println!("{}: {} cases", vector.id, vector.cases.len());
}

#[test]
fn ordering() {
    let vector = vector("ordering");
    for case in &vector.cases {
        let expected = case
            .expected_order
            .as_ref()
            .expect("case carries no `expected_order`");
        let key = case.key.as_deref().expect("case carries no `key`");
        let actual: Vec<String> = match key {
            "registry" => {
                // `engine.py:797`'s shape. `sort_by_key` is stable, which is the whole point:
                // every unregistered name ranks `len(registry)` and must stay where it arrived.
                let registry = case.registry.as_ref().expect("registry case carries none");
                let mut names = case.names.clone().expect("registry case carries no names");
                names.sort_by_key(|name| registry_rank(name, registry));
                names
            }
            "neg_abs" | "neg" => {
                let entries = case
                    .entries
                    .as_ref()
                    .expect("share case carries no entries");
                let map: OrderedMap<f64> = entries
                    .iter()
                    .map(|(name, repr)| (name.clone(), repr.parse().expect("share repr")))
                    .collect();
                let sorted = if key == "neg_abs" {
                    map.sorted_by(|_, value| -value.abs())
                } else {
                    map.sorted_by(|_, value| -value)
                };
                // `next(iter(...))`, which is what actually reaches a sentence.
                assert_eq!(
                    sorted.first_key(),
                    case.expected_first.as_deref(),
                    "largest contributor under {key}"
                );
                sorted.keys().map(str::to_string).collect()
            }
            other => panic!("unknown ordering key {other:?}"),
        };
        assert_eq!(&actual, expected, "order under {key}");
    }
    println!("{}: {} cases", vector.id, vector.cases.len());
}

// ------------------------------------------------------------------- the screen parser's (M34 P2)

#[derive(Deserialize)]
struct TextCase {
    value: String,
    expected: String,
}

/// `repr(s)` on a `str`, compared byte for byte: the quote, the escapes and the printable test.
#[test]
fn string_representation() {
    let table: Table<TextCase> = table("str_repr");
    let mut differences = Vec::new();
    for case in &table.cases {
        let actual = str_repr(&case.value);
        if actual != case.expected {
            differences.push(format!(
                "{:?}: {actual}, CPython says {}",
                case.value, case.expected
            ));
        }
    }
    assert!(
        differences.is_empty(),
        "{} of {} differ:\n{}",
        differences.len(),
        table.cases.len(),
        differences.join("\n")
    );
    println!("{}: {} cases", table.id, table.cases.len());
}

#[derive(Deserialize)]
struct FloorDivCase {
    a: String,
    b: String,
    expected: String,
    as_int: i64,
}

/// Float `//` by bits, and `int(a // b)` — what `_Cell.text` keys on — as an integer.
#[test]
fn floor_division() {
    let table: Table<FloorDivCase> = table("floor_div");
    for case in &table.cases {
        let (a, b) = (float(&case.a), float(&case.b));
        let actual = floor_div(a, b);
        let expected = float(&case.expected);
        assert!(
            actual.to_bits() == expected.to_bits(),
            "{} // {}: {actual:?}, CPython says {expected:?}",
            case.a,
            case.b
        );
        // The parser's own use. Python's `int` has no signed zero, so this one is blind to it.
        assert_eq!(actual as i64, case.as_int, "int({} // {})", case.a, case.b);
    }
    println!("{}: {} cases", table.id, table.cases.len());
}

/// One shape for four string operations and the two whole-set cases, told apart by `op`.
#[derive(Deserialize)]
struct StringOpCase {
    op: String,
    value: Option<String>,
    source: Option<String>,
    expected: serde_json::Value,
}

impl StringOpCase {
    fn value(&self) -> &str {
        self.value.as_deref().expect("case carries no `value`")
    }

    fn expected_text(&self) -> &str {
        self.expected.as_str().expect("`expected` is not a string")
    }
}

/// `upper`, `split`, `strip`, the `\s+` deletion, and the whitespace set **whole**.
///
/// The whole-set cases are why this test walks every code point: the table records each one
/// CPython calls whitespace, by `str.isspace` and by `re`'s `\s`, and [`is_space`] has to agree on
/// all 1,114,112 — so a character nobody thought to put in a string is still checked.
#[test]
fn text_case() {
    let table: Table<StringOpCase> = table("text_case");
    let mut sets = 0;
    for case in &table.cases {
        match case.op.as_str() {
            "upper" => assert_eq!(
                upper(case.value()),
                case.expected_text(),
                "{:?}.upper()",
                case.value()
            ),
            "split" => {
                let expected: Vec<String> = serde_json::from_value(case.expected.clone())
                    .expect("`split` expects a list of strings");
                let actual: Vec<&str> = split(case.value()).collect();
                assert_eq!(actual, expected, "{:?}.split()", case.value());
            }
            "strip" => assert_eq!(
                strip(case.value()),
                case.expected_text(),
                "{:?}.strip()",
                case.value()
            ),
            "strip_space" => assert_eq!(
                strip_space(case.value()),
                case.expected_text(),
                "re.sub(r'\\s+', '', {:?})",
                case.value()
            ),
            "space_set" => {
                let expected: HashSet<u32> = serde_json::from_value(case.expected.clone())
                    .expect("`space_set` expects a list of code points");
                let source = case.source.as_deref().unwrap_or("?");
                let wrong: Vec<String> = (0..=char::MAX as u32)
                    .filter_map(char::from_u32)
                    .filter(|&c| is_space(c) != expected.contains(&(c as u32)))
                    .map(|c| format!("U+{:04X}", c as u32))
                    .collect();
                assert!(
                    wrong.is_empty(),
                    "is_space disagrees with {source} on {wrong:?}"
                );
                sets += 1;
            }
            other => panic!("unknown text_case op {other:?}"),
        }
    }
    assert_eq!(sets, 2, "the table carries both whole-set cases");
    println!("{}: {} cases", table.id, table.cases.len());
}

#[derive(Deserialize)]
struct SumCase {
    values: Vec<String>,
    expected: String,
}

/// `sum()` by bits. A left fold passes about half of these and is wrong on the rest.
///
/// A NaN is matched as a NaN rather than by bits, and that is the vector's limit, not a tolerance:
/// `repr` writes `nan` for every NaN, so the table cannot say which one CPython produced. `inf +
/// -inf` is the sign-bit-set default NaN on x86 in both languages; parsing `"nan"` is not.
#[test]
fn compensated_sum() {
    let table: Table<SumCase> = table("sum");
    for case in &table.cases {
        let values: Vec<f64> = case.values.iter().map(|v| float(v)).collect();
        let actual = sum(&values);
        let expected = float(&case.expected);
        assert!(
            actual.to_bits() == expected.to_bits() || (actual.is_nan() && expected.is_nan()),
            "sum({:?}) = {actual:?}, CPython says {expected:?}",
            case.values
        );
    }
    println!("{}: {} cases", table.id, table.cases.len());
}
