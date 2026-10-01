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
//! **One allowance, since M32's re-record**: an `input.shot` key that went in absent may come out at
//! its default when this vector's own ledger added its twin under `expected.swing.shot` — the
//! committed inputs keep the shape frozen Python writes. [`DECLARED_BY_THE_ANSWER`] says exactly
//! where, and everything else absent-then-present is still a difference.
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
use serde_json::{json, Value};

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
        .expect("spec/vectors is missing — it is committed: restore it from git")
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

/// Where an `input` key may come back that went in absent: the owner it sits on, and the owner in
/// `expected` whose ledgered key licenses it. [M32 P8]
///
/// **This is the one allowance, and it is spelled out rather than derived.** M32's re-record added
/// ten keys to `expected.swing.shot` and rewrote no input (§M32: inputs keep the shape frozen Python
/// writes, because the frozen lab goes on writing exactly that until M40). So a corpus vector's
/// `input.shot` goes in without them and comes out of `ShotData` with each at its serde default — a
/// key this gate would otherwise call invented. The engine echoes the shot into its answer, so the
/// same key under `expected.swing.shot` is where the re-record declared it, and that declaration is
/// what licenses the input's copy. Two owners, because `ShotProvenance` is a struct of its own and a
/// key is matched against its *parent*: a key invented anywhere else — deeper, shallower, or under
/// an owner not in this table — is still a difference, ledger or not.
const DECLARED_BY_THE_ANSWER: [(&str, &str); 2] = [
    ("input.shot", "expected.swing.shot"),
    ("input.shot.provenance", "expected.swing.shot.provenance"),
];

/// Whether the key at `path`, absent from the committed `input` and present at `value` after the
/// round trip, is one the allowance above covers for this `vector`.
///
/// Two conditions, both required:
/// - **the answer's twin is in this vector's ledger**, as an `added` path of some
///   `provenance.rerecords` entry — so the allowance reaches exactly the keys a reviewed declaration
///   added to *this* file, and no vector that was never re-recorded;
/// - **the value is the one the answer carries there.** The re-record wrote that value from Rust's
///   reading of this same absent key, so it *is* the serde default as of the re-record. Comparing
///   against the committed file rather than against today's default is what makes this a check: a
///   default that changed later would come out here as a value the committed answer does not hold.
///   (`shot.rs`'s unit tests are what pin the defaults themselves.)
fn declared_by_the_answer(vector: &Value, path: &str, value: &Value) -> bool {
    let Some((owner, key)) = path.rsplit_once('.') else {
        return false;
    };
    let Some(twin) = DECLARED_BY_THE_ANSWER
        .iter()
        .find(|(input_owner, _)| *input_owner == owner)
        .map(|(_, answer_owner)| format!("{answer_owner}.{key}"))
    else {
        return false;
    };

    let ledgered = vector["provenance"]["rerecords"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry["added"].as_array())
        .flatten()
        .any(|added| added.as_str() == Some(twin.as_str()));
    let recorded = twin
        .split('.')
        .try_fold(vector, |node, step| node.get(step));
    ledgered && recorded == Some(value)
}

/// Every place two parsed JSON values disagree, as `path: got != want` lines.
///
/// Key sets are compared in both directions, which is the half that matters: a dropped field and an
/// invented one are different mistakes and a port can make either. `licensed` is asked about each
/// invented key, with its path and value, and a key it accepts is not a difference — the expected
/// half passes one that accepts nothing, and the input half [`declared_by_the_answer`].
fn differences(
    actual: &Value,
    expected: &Value,
    path: &str,
    licensed: &dyn Fn(&str, &Value) -> bool,
    into: &mut Vec<String>,
) {
    match (actual, expected) {
        (Value::Object(a), Value::Object(e)) => {
            let actual_keys: BTreeSet<&String> = a.keys().collect();
            let expected_keys: BTreeSet<&String> = e.keys().collect();
            for key in expected_keys.difference(&actual_keys) {
                into.push(format!("{path}.{key}: dropped (was {})", e[*key]));
            }
            for key in actual_keys.difference(&expected_keys) {
                let at = format!("{path}.{key}");
                if !licensed(&at, &a[*key]) {
                    into.push(format!("{at}: invented (is {})", a[*key]));
                }
            }
            for key in actual_keys.intersection(&expected_keys) {
                differences(&a[*key], &e[*key], &format!("{path}.{key}"), licensed, into);
            }
        }
        (Value::Array(a), Value::Array(e)) if a.len() != e.len() => {
            into.push(format!("{path}: {} entries, expected {}", a.len(), e.len()));
        }
        (Value::Array(a), Value::Array(e)) => {
            for (i, (a, e)) in a.iter().zip(e).enumerate() {
                differences(a, e, &format!("{path}[{i}]"), licensed, into);
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

/// `vector`'s `input`, read into the harness shape and written back, against what was committed —
/// under [`declared_by_the_answer`]'s allowance, and nothing else's.
///
/// Walked from `"input"` rather than from the id, so a path reaches the allowance spelled the way a
/// ledger spells it; the caller puts the id back on for the report.
fn input_differences(id: &str, vector: &Value) -> Vec<String> {
    let input: VectorInput = serde_json::from_value(vector["input"].clone())
        .unwrap_or_else(|e| panic!("{id}: reading `input`: {e}"));
    let mut found = Vec::new();
    differences(
        &serde_json::to_value(&input).expect("serialize input"),
        &vector["input"],
        "input",
        &|path, value| declared_by_the_answer(vector, path, value),
        &mut found,
    );
    found
}

/// The allowance is a door, so this is the test that it opens only where it should: a ledgered key
/// at the value the answer recorded, under one of the two owners — and shut for a key the answer
/// holds but no ledger names, a value the answer does not hold, an owner the table does not list, and
/// a vector with no ledger at all.
#[test]
fn the_allowance_reaches_only_ledgered_keys_at_their_recorded_value() {
    let vector = json!({
        "provenance": {"rerecords": [{"added": [
            "expected.swing.shot.attack_angle",
            "expected.swing.shot.provenance.corrections"
        ], "moved": []}]},
        "expected": {"swing": {"shot": {
            "attack_angle": null,
            "low_point": null,
            "provenance": {"corrections": {}}
        }}}
    });

    assert!(declared_by_the_answer(
        &vector,
        "input.shot.attack_angle",
        &Value::Null
    ));
    assert!(declared_by_the_answer(
        &vector,
        "input.shot.provenance.corrections",
        &json!({})
    ));

    // In the answer, but no ledger entry added it.
    assert!(!declared_by_the_answer(
        &vector,
        "input.shot.low_point",
        &Value::Null
    ));
    // Ledgered, but not the value the answer recorded.
    assert!(!declared_by_the_answer(
        &vector,
        "input.shot.attack_angle",
        &json!(0.0)
    ));
    // A ledgered key's name under an owner the table does not list.
    assert!(!declared_by_the_answer(
        &vector,
        "input.attack_angle",
        &Value::Null
    ));
    assert!(!declared_by_the_answer(
        &vector,
        "input.shot.provenance.raw_fields.corrections",
        &json!({})
    ));

    let mut unledgered = vector.clone();
    unledgered["provenance"]
        .as_object_mut()
        .unwrap()
        .remove("rerecords");
    assert!(!declared_by_the_answer(
        &unledgered,
        "input.shot.attack_angle",
        &Value::Null
    ));
}

/// On the committed set, the allowance is doing real work and exactly the declared work: take a
/// corpus vector's ledger away and its `input` comes back with one invented key per `input`-side
/// twin of an `added` path it listed — no more, no fewer.
///
/// Read off the vector's own ledger rather than spelled here, so a later declaration that adds keys
/// to the shot is checked by the same test without an edit.
#[test]
fn without_its_ledger_a_corpus_input_comes_back_with_exactly_the_declared_keys() {
    let (id, vector) = vectors()
        .into_iter()
        .find(|(id, _)| id.starts_with("corpus/"))
        .expect("at least one corpus vector");
    assert!(
        input_differences(&id, &vector).is_empty(),
        "{id}: the allowance does not hold with the ledger in place"
    );

    let mut declared: Vec<String> = vector["provenance"]["rerecords"]
        .as_array()
        .expect("a re-recorded corpus vector carries a ledger")
        .iter()
        .flat_map(|entry| entry["added"].as_array().expect("`added` is a list"))
        .filter_map(|added| {
            let added = added.as_str().expect("a ledger path is a string");
            DECLARED_BY_THE_ANSWER
                .iter()
                .find_map(|(input_owner, answer_owner)| {
                    let key = added.strip_prefix(answer_owner)?.strip_prefix('.')?;
                    (!key.contains('.')).then(|| format!("{input_owner}.{key}"))
                })
        })
        .collect();
    declared.sort();
    assert!(!declared.is_empty(), "{id}: its ledger adds no shot key");

    let mut unledgered = vector.clone();
    unledgered["provenance"]
        .as_object_mut()
        .unwrap()
        .remove("rerecords");
    let mut invented: Vec<String> = input_differences(&id, &unledgered)
        .iter()
        .map(|line| {
            line.split_once(": invented")
                .unwrap_or_else(|| panic!("{id}: not an invented key: {line}"))
                .0
                .to_string()
        })
        .collect();
    invented.sort();
    assert_eq!(invented, declared);
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
             re-record the vectors with `golf-core rerecord` in the change that bumped it",
            vector["analysis_version"]
        );

        report.extend(
            input_differences(&id, &vector)
                .iter()
                .map(|line| format!("{id}.{line}")),
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
            &|_, _| false,
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
