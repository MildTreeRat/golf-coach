//! This crate against every committed engine vector's `input` and `expected`. [M22 P2]
//!
//! **What this gate proves, and why it is the right one for a phase that ports only shapes.** A
//! vector's `input` is what `conformance.py::run_vector` feeds the engine and its `expected` is what
//! Python serialized back; between them they are every field of every payload shape, populated by a
//! real capture rather than by a fixture author's imagination. Reading each one into these types and
//! writing it out again asks the one question P2 can be wrong about: *does a field survive the
//! crossing*. A shape that silently drops a key compiles, passes every unit test, and then fails at
//! P6 as a missing sentence in a golfer's results page — four phases after the mistake.
//!
//! It is deliberately **stricter** than `docs/CONFORMANCE.md` §3. There is no tolerance here and
//! there should not be: no arithmetic happened, so a float that came in must go out with the same
//! bits, and a key that came in must go out. §3's `RTOL` is sized for a different *summation order*,
//! and nothing here sums anything. P4 onward is where that tolerance starts earning its keep.
//!
//! **The comparison is structural, over parsed values, because a byte comparison is not available.**
//! Python writes `-1.636758133827243e-05` and `serde_json` writes `-0.00001636758133827243` for the
//! identical f64 — both shortest-round-trip forms, differing only on when to reach for an exponent.
//! Seventy-seven distinct floats in the committed vectors take the exponent form. See the crate doc.
//!
//! The template is `crates/trigger/tests/conformance.rs`: read the directory, sort, deserialize,
//! accumulate **every** difference, one assertion at the end. A report naming four dropped fields is
//! worth four runs that each name one.

use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use contracts::golfer::Handedness;
use contracts::intent::PracticeGoal;
use contracts::keypoints::KeypointsFile;
use contracts::shot::ShotData;
use contracts::swing::{SwingBundleResult, ANALYSIS_VERSION};
use flate2::read::GzDecoder;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// `conformance.py::run_vector`'s parameters, as they sit in a vector file.
///
/// **The two-deep `Option` is not an accident.** A vector distinguishes a key that is absent from a
/// key spelled `null` — `windowed.json` simply has no `shot`, while every corpus vector carries
/// `"loft_deg": null` — and a round trip that flattened the two would pass while quietly rewriting
/// the file. `None` is absent, `Some(None)` is an explicit null. `run_vector` reads both with
/// `.get()` and cannot tell them apart; this gate can, so it does.
///
/// `deny_unknown_fields` because this is a *harness* shape rather than a contract: pydantic ignores
/// extras by design and so do the types under test, but a vector key nothing here models should stop
/// the run and name itself rather than be caught two hundred lines later as a missing output key.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct VectorInput {
    swing_id: String,
    session_id: String,
    face_on: KeypointsFile,

    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    down_the_line: Option<Option<KeypointsFile>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    shot: Option<Option<ShotData>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    intent: Option<Option<PracticeGoal>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    face_on_window: Option<Option<(i64, i64)>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    down_the_line_window: Option<Option<(i64, i64)>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    face_on_strikes: Option<Option<Vec<i64>>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    down_the_line_strikes: Option<Option<Vec<i64>>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    handedness: Option<Option<Handedness>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    loft_deg: Option<Option<f64>>,
}

/// Read a key that is *present*, whatever it holds, as `Some(..)`.
///
/// Without it `Option<Option<T>>` collapses: serde reads a JSON `null` straight through to the outer
/// `None`, so absent and null become one value again and the round trip rewrites `"loft_deg": null`
/// as no key at all. `deserialize_with` only runs when the key is there, which is exactly the
/// distinction wanted — `#[serde(default)]` covers the other case.
fn present<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// What `api/pipeline.py::analyze_swing_dir` drops before writing `analysis.json`, restated here for
/// the same reason `conformance.py` restates it: **the exclusion lives at a call site and in no
/// schema** (M19's own finding), so a port has to make the same drop and has nowhere to read it from.
///
/// Keypoints and detections are the *input*, echoed back on the result as the data it was computed
/// from. Round-tripping them through the comparison would make every vector thirty times larger and
/// check nothing — they are already compared, as `input.face_on`.
///
/// It lives in the harness rather than in the crate because it is not a property of the shape. P6's
/// `golf-core run` inherits it when it grows a stdout seam.
const EXCLUDED_FROM_RESULT: [(&str, &str); 2] = [("swing", "keypoints"), ("swing", "detections")];

fn spec_dir() -> PathBuf {
    // `CARGO_MANIFEST_DIR` is `crates/contracts`; the spec is two levels up, beside `pyproject.toml`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../spec/vectors")
        .canonicalize()
        .expect("spec/vectors is missing — run `python scripts/conformance.py regenerate`")
}

/// Every committed engine vector, synthetic and corpus, as `(id, parsed file)`.
///
/// Discovered rather than listed, and sorted so a failure reports in a stable order — the same
/// choice `conformance.vector_paths` makes, for the same reason: a vector that exists on disk and in
/// no index is a vector nothing runs.
fn vectors() -> Vec<(String, Value)> {
    let mut paths: Vec<PathBuf> = ["synthetic", "corpus"]
        .iter()
        .flat_map(|family| {
            fs::read_dir(spec_dir().join(family))
                .unwrap_or_else(|e| panic!("cannot read spec/vectors/{family}: {e}"))
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        })
        .collect();
    paths.sort();
    assert!(
        !paths.is_empty(),
        "no engine vectors under {:?}",
        spec_dir()
    );

    paths
        .iter()
        .map(|path| {
            let mut text = String::new();
            if path.extension().is_some_and(|x| x == "gz") {
                GzDecoder::new(fs::File::open(path).expect("open vector"))
                    .read_to_string(&mut text)
                    .expect("decompress vector");
            } else {
                text = fs::read_to_string(path).expect("read vector");
            }
            let parsed: Value =
                serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path:?}: {e}"));
            let id = parsed["id"].as_str().expect("vector has an id").to_string();
            (id, parsed)
        })
        .collect()
}

/// Every place two parsed JSON values disagree, as `path: got != want` lines.
///
/// Key sets are compared in both directions, which is the half that matters: a dropped field and an
/// invented one are different mistakes and a port can make either.
fn differences(actual: &Value, expected: &Value, path: &str, into: &mut Vec<String>) {
    match (actual, expected) {
        (Value::Object(a), Value::Object(e)) => {
            let actual_keys: BTreeSet<&String> = a.keys().collect();
            let expected_keys: BTreeSet<&String> = e.keys().collect();
            for key in expected_keys.difference(&actual_keys) {
                into.push(format!("{path}.{key}: dropped (was {})", e[*key]));
            }
            for key in actual_keys.difference(&expected_keys) {
                into.push(format!("{path}.{key}: invented (is {})", a[*key]));
            }
            for key in actual_keys.intersection(&expected_keys) {
                differences(&a[*key], &e[*key], &format!("{path}.{key}"), into);
            }
        }
        (Value::Array(a), Value::Array(e)) if a.len() != e.len() => {
            into.push(format!("{path}: {} entries, expected {}", a.len(), e.len()));
        }
        (Value::Array(a), Value::Array(e)) => {
            for (i, (a, e)) in a.iter().zip(e).enumerate() {
                differences(a, e, &format!("{path}[{i}]"), into);
            }
        }
        (Value::Number(a), Value::Number(e)) => {
            // Bits rather than `==`, so a float that came back as `0.0` where Python wrote `-0.0`
            // is a finding rather than a pass. An integer that came back as a float is one too:
            // `docs/CONFORMANCE.md` §3 compares ints exactly and a widened one is a type difference.
            let same = match (a.as_f64(), e.as_f64()) {
                (Some(x), Some(y)) => a.is_f64() == e.is_f64() && x.to_bits() == y.to_bits(),
                _ => a == e,
            };
            if !same {
                into.push(format!("{path}: {a} != {e}"));
            }
        }
        (a, e) if a != e => into.push(format!("{path}: {a} != {e}")),
        _ => {}
    }
}

/// Drop what `analyze_swing_dir` drops, so the round trip is compared against what was committed.
fn strip_excluded(value: &mut Value) {
    for (owner, field) in EXCLUDED_FROM_RESULT {
        value
            .get_mut(owner)
            .and_then(Value::as_object_mut)
            .and_then(|owner| owner.remove(field))
            .expect("the excluded field is on the round trip, or the exclusion is stale");
    }
}

#[test]
fn every_vector_survives_the_crossing() {
    let mut report: Vec<String> = Vec::new();
    let mut inputs = 0usize;
    let mut expecteds = 0usize;

    for (id, vector) in vectors() {
        let stale = vector["analysis_version"].as_i64() != Some(ANALYSIS_VERSION);
        assert!(
            !stale,
            "{id} was recorded at analysis_version {} and this crate is {ANALYSIS_VERSION} — \
             regenerate the vectors in the change that bumped it",
            vector["analysis_version"]
        );

        let input: VectorInput = serde_json::from_value(vector["input"].clone())
            .unwrap_or_else(|e| panic!("{id}: reading `input`: {e}"));
        differences(
            &serde_json::to_value(&input).expect("serialize input"),
            &vector["input"],
            &format!("{id}.input"),
            &mut report,
        );
        inputs += 1;

        let expected: SwingBundleResult = serde_json::from_value(vector["expected"].clone())
            .unwrap_or_else(|e| panic!("{id}: reading `expected`: {e}"));
        let mut round_tripped = serde_json::to_value(&expected).expect("serialize expected");
        strip_excluded(&mut round_tripped);
        differences(
            &round_tripped,
            &vector["expected"],
            &format!("{id}.expected"),
            &mut report,
        );
        expecteds += 1;
    }

    assert!(
        report.is_empty(),
        "{} of {} halves differ:\n  {}",
        report.len(),
        inputs + expecteds,
        report.join("\n  ")
    );
    // The exit criterion is 21 engine vectors (ADR-032 §9), and a gate that silently ran three of
    // them would report the same green as one that ran all of them.
    assert_eq!(inputs, 21, "expected 21 engine vectors, ran {inputs}");
    assert_eq!(expecteds, 21);
}
