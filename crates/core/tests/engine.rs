//! The whole bundle against `spec/vectors/`, on the six synthetic vectors. [M22 P6]
//!
//! **The first end-to-end green, and the first gate in this workspace that is not a stage.** The
//! five before it each ran one of `analyze_swing_bundle`'s intermediate calls and compared a recorded
//! answer for it; this one runs `conformance.py::run_vector`'s whole seam — engine, then feedback,
//! then the serialization with its `exclude=` — and compares `expected` in full. Everything a stage
//! vector cannot hold is inside the criterion here: `mechanics_score` and `overall_score`, the
//! `unscored` list and its order, `notes`, `analysis_version`, and the ranked `feedback` payload
//! ADR-032 §2 explains cannot be a stage because `build_feedback` takes the *assembled* result.
//!
//! # Six and not twenty-one, and what that leaves ungated
//!
//! Every corpus vector carries a shot, an intent, a down-the-line clip and both strike lists, and was
//! recorded with them — so the corpus half needed P7's second view and still needs P8b's join
//! modules before it can be run at all. That is ADR-032 §9's exit criterion and not this phase's, and
//! [`the_corpus_half_is_deferred_and_the_only_reason_left_is_the_shot`] makes the deferral a checked
//! fact rather than a silence. **P7 narrowed it from three reasons to one**: the second view and the
//! strike ingestion have landed, so what still holds the fifteen out is the `shot` and `flight`
//! measurement groups alone.
//!
//! What the six *do* cover is worth stating positively, because "the first whole-bundle green" reads
//! as more than it is. Between them: a swing that passes four checkpoints and fails two, a tempo
//! failure, a head-sway-and-finish failure, a left-handed swing, a swing with **no golfer attributed**
//! (the only `unscored` entry anywhere in the six, and the only `_unmeasured_tip` this gate renders),
//! and a **windowed** clip, which is the only committed evidence for `_windowed` and `_shifted`
//! anywhere — the corpus vectors carry no `face_on_window`.
//!
//! And what none of them covers, each already pinned by a unit test rather than assumed: no vector
//! moves an impact onto a strike (`face_on_strikes` is absent from all six), none produces a
//! `tempo_notes` sentence, none carries a shot, and none has more than one camera. Six of the seven
//! sentences `engine.rs` can append are therefore unreachable here; the seventh is the
//! one-camera note every vector carries.
//!
//! # The rules
//!
//! `docs/CONFORMANCE.md` §3's, implemented over `serde_json::Value` because that is what
//! `compare_results` compares — parsed values, never bytes, for the reason M22 P2 measured: Python
//! writes `-1.636758133827243e-05` where `serde_json` writes `-0.00001636758133827243` for the
//! identical f64. Bools before ints, because `isinstance(True, int)` is true in Python and the
//! recorded JSON therefore has to be read with the same ordering; floats within `RTOL = 1e-9`;
//! everything else exact, list order included. Every difference across all six is accumulated and
//! reported in one assertion — the template `crates/trigger/tests/conformance.rs` set.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use contracts::swing::ANALYSIS_VERSION;
use flate2::read::GzDecoder;
use serde_json::Value;

/// `docs/CONFORMANCE.md` §3's float rule, and the same constants every other gate here uses.
const RTOL: f64 = 1e-9;
const ATOL: f64 = 1e-12;

fn spec_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/core has a grandparent")
        .join("spec")
        .join("vectors")
}

fn read_json(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    if path.extension().is_some_and(|e| e == "gz") {
        let mut text = String::new();
        GzDecoder::new(&bytes[..])
            .read_to_string(&mut text)
            .unwrap_or_else(|e| panic!("gunzip {path:?}: {e}"));
        text
    } else {
        String::from_utf8(bytes).unwrap_or_else(|e| panic!("utf-8 {path:?}: {e}"))
    }
}

/// Every committed vector in one half of `spec/vectors/`, sorted so a failure names a stable first
/// offender. Duplicated from `crates/analysis/tests/`' readers rather than shared, for the reason
/// those two duplicate it from each other: integration tests are separate binaries and a crate whose
/// only job is to be imported by three of them is more structure than thirty lines of reading earns.
fn vectors(half: &str) -> Vec<(String, Value)> {
    let root = spec_dir().join(half);
    let mut paths: Vec<PathBuf> = fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("list {root:?}: {e}"))
        .map(|entry| entry.expect("a readable directory entry").path())
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no vectors under {root:?}");

    paths
        .into_iter()
        .map(|path| {
            let text = read_json(&path);
            let value: Value =
                serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path:?}: {e}"));
            let id = value["id"]
                .as_str()
                .unwrap_or_else(|| panic!("{path:?} carries no id"))
                .to_string();
            (id, value)
        })
        .collect()
}

/// §3's comparison, accumulating every difference rather than stopping at the first.
///
/// `path` is a dotted trail into the payload (`swing.checkpoint_scores[3].message`), so a failure
/// says *where* without anyone having to re-run with a debugger. That is the one thing this owes a
/// reader that a bare `assert_eq!` on two `Value`s does not — and the reason it is written out rather
/// than delegated to `PartialEq`, which would also compare floats exactly.
fn compare(expected: &Value, actual: &Value, path: &str, out: &mut Vec<String>) {
    let at = |p: &str| {
        if p.is_empty() {
            "<root>".to_string()
        } else {
            p.to_string()
        }
    };

    match (expected, actual) {
        // A refusal compares equal to nothing but a refusal (ADR-010 §2). Reported as a type
        // difference and never tested numerically: a port returning `0.0` where this returns `None`
        // has turned "could not measure" into "measured zero".
        (Value::Null, Value::Null) => {}
        (Value::Null, other) => out.push(format!("{}: expected a refusal, got {other}", at(path))),
        (other, Value::Null) => out.push(format!("{}: expected {other}, got a refusal", at(path))),

        // Bools before numbers, because `isinstance(True, int)` is true in Python: the obvious
        // ordering compares a verdict numerically and lets `1` through for `true`.
        (Value::Bool(a), Value::Bool(b)) => {
            if a != b {
                out.push(format!("{}: {a} != {b}", at(path)));
            }
        }
        (Value::String(a), Value::String(b)) => {
            if a != b {
                out.push(format!("{}: {a:?} != {b:?}", at(path)));
            }
        }
        (Value::Number(a), Value::Number(b)) => {
            // An int is exact — frame indices, `population_n`, `analysis_version` — and a float is
            // within tolerance. Which one this is comes off the *recorded* value, because that is
            // what pydantic's declared type produced.
            match (a.as_i64(), b.as_i64()) {
                (Some(x), Some(y)) => {
                    if x != y {
                        out.push(format!("{}: {x} != {y}", at(path)));
                    }
                }
                _ => {
                    let (x, y) = (
                        a.as_f64().expect("a JSON number is an f64"),
                        b.as_f64().expect("a JSON number is an f64"),
                    );
                    if (x - y).abs() > ATOL + RTOL * x.abs() {
                        out.push(format!("{}: {x} != {y}", at(path)));
                    }
                }
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                out.push(format!("{}: length {} != {}", at(path), a.len(), b.len()));
                return;
            }
            for (index, (x, y)) in a.iter().zip(b).enumerate() {
                compare(x, y, &format!("{path}[{index}]"), out);
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            // Same keys, both ways. A key the port *added* is as much a finding as one it dropped:
            // `spec/schemas/` describes the shape and an extra field means the two implementations
            // disagree about what a `SwingBundleResult` is.
            for key in a.keys() {
                if !b.contains_key(key) {
                    out.push(format!("{}: key {key:?} missing from the port", at(path)));
                }
            }
            for key in b.keys() {
                if !a.contains_key(key) {
                    out.push(format!(
                        "{}: key {key:?} the vector does not have",
                        at(path)
                    ));
                }
            }
            for (key, x) in a {
                if let Some(y) = b.get(key) {
                    let child = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    compare(x, y, &child, out);
                }
            }
        }
        (a, b) => out.push(format!("{}: type difference, {a} against {b}", at(path))),
    }
}

/// **All twenty-one vectors, end to end, in one report — this milestone's exit criterion.**
/// [M22 P8b]
///
/// Six synthetic and fifteen corpus, run through `golf_core::run` and compared against `expected` in
/// full under §3's rules. ADR-032 §9 says the corpus half is not optional and cannot be deferred, and
/// this is where that stops being a plan.
#[test]
fn the_whole_bundle_conforms_on_all_twenty_one_vectors() {
    let mut differences: Vec<String> = Vec::new();
    let mut committed = vectors("synthetic");
    committed.extend(vectors("corpus"));
    assert_eq!(
        committed.len(),
        21,
        "the committed set changed size; ADR-032 §9's exit criterion is twenty-one"
    );

    for (id, vector) in &committed {
        assert_eq!(
            vector["analysis_version"].as_i64(),
            Some(ANALYSIS_VERSION),
            "{id}: recorded at v{} against a port claiming v{ANALYSIS_VERSION} — regenerate the \
             vectors in the change that bumped it",
            vector["analysis_version"]
        );

        let input = serde_json::from_value(vector["input"].clone())
            .unwrap_or_else(|e| panic!("{id}: input does not parse into the ported shapes: {e}"));
        let actual = golf_core::run(&input);
        compare(&vector["expected"], &actual, id, &mut differences);
    }

    assert!(
        differences.is_empty(),
        "{} difference(s) across {} vectors:\n{}",
        differences.len(),
        committed.len(),
        differences.join("\n")
    );
    println!("the whole bundle conforms on {} vectors", committed.len());
}

/// The comparator is worth nothing if it cannot see a difference, which is why
/// `tests/test_conformance.py` unit-tests the Python one. The four rules that are easy to get wrong.
#[test]
fn the_comparator_sees_the_differences_it_exists_to_see() {
    let cases: [(Value, Value, &str); 7] = [
        // A refusal is not a zero. ADR-010 §2, the rule this suite is built around.
        (
            serde_json::json!({"observed": null}),
            serde_json::json!({"observed": 0.0}),
            "expected a refusal",
        ),
        (
            serde_json::json!({"observed": 0.0}),
            serde_json::json!({"observed": null}),
            "got a refusal",
        ),
        // A bool is not an int, in the direction Python gets wrong.
        (
            serde_json::json!({"passed": true}),
            serde_json::json!({"passed": 1}),
            "type difference",
        ),
        // A sentence is exact, to the byte.
        (
            serde_json::json!({"message": "Good tempo - 2.7:1."}),
            serde_json::json!({"message": "Good tempo - 2.70:1."}),
            "!=",
        ),
        // A float past the tolerance, and list order.
        (
            serde_json::json!({"score": 0.611_111_111_111_111}),
            serde_json::json!({"score": 0.611_111_2}),
            "!=",
        ),
        (
            serde_json::json!({"tips": ["a", "b"]}),
            serde_json::json!({"tips": ["b", "a"]}),
            "tips[0]",
        ),
        // A key the port invented.
        (
            serde_json::json!({}),
            serde_json::json!({"spine_angle": 1.0}),
            "the vector does not have",
        ),
    ];
    for (expected, actual, wanted) in cases {
        let mut out = Vec::new();
        compare(&expected, &actual, "", &mut out);
        assert!(
            out.iter().any(|d| d.contains(wanted)),
            "comparing {expected} against {actual} did not report {wanted:?}: {out:?}"
        );
    }

    // And it must not cry wolf: a float inside the tolerance is the same measurement summed in a
    // different order, which is exactly what `RTOL` is sized for.
    let mut quiet = Vec::new();
    compare(
        &serde_json::json!({"score": 0.611_111_111_111_111}),
        &serde_json::json!({"score": 0.611_111_111_111_111_2}),
        "",
        &mut quiet,
    );
    assert!(quiet.is_empty(), "{quiet:?}");
}

/// **Which half of the committed set exercises what**, now that both halves are inside the gate.
/// [M22 P8b]
///
/// P6 and P7 carried a test here asserting the corpus half was *deferred*, and it earned its keep
/// while that was true: it narrowed from three reasons to one as P7 landed, and named the last one.
/// With the last one discharged there is nothing left to defer, and the replacement is the fact the
/// old test was standing in for — the two halves cover disjoint things, and a vector moving between
/// them would silently drop coverage that nothing else supplies.
///
/// The synthetic six are the only vectors with a **window**, the only **left-handed** swing, and the
/// only swing with **no golfer attributed** — and they carry no shot, no second camera and no strike
/// list. The corpus fifteen are the only vectors with a **shot** (so the only ones reaching the
/// outcome groups and the flight's refusals), a **second camera** and **strikes**.
#[test]
fn the_two_halves_of_the_committed_set_cover_disjoint_things() {
    let present = |vector: &Value, key: &str| {
        vector["input"]
            .get(key)
            .is_some_and(|value| !value.is_null())
    };

    let synthetic = vectors("synthetic");
    assert_eq!(synthetic.len(), 6, "the synthetic half changed size");
    for (id, vector) in &synthetic {
        for absent in ["shot", "down_the_line", "face_on_strikes", "loft_deg"] {
            assert!(
                !present(vector, absent),
                "{id} gained a {absent:?}; the corpus half was the only thing gating it"
            );
        }
    }
    assert_eq!(
        synthetic
            .iter()
            .filter(|(_, v)| present(v, "face_on_window"))
            .count(),
        1,
        "`windowed` is gated by exactly one vector anywhere in spec/"
    );

    let corpus = vectors("corpus");
    assert_eq!(corpus.len(), 15, "the corpus half changed size");
    for (id, vector) in &corpus {
        assert!(!present(vector, "face_on_window"), "{id} gained a window");
        for required in ["shot", "down_the_line", "face_on_strikes"] {
            assert!(
                present(vector, required),
                "{id} lost its {required:?}; the synthetic half does not supply one"
            );
        }
    }
    assert_eq!(
        corpus
            .iter()
            .filter(|(_, v)| present(v, "loft_deg"))
            .count(),
        5,
        "five corpus vectors carry a loft, which is what gates the spin solve's branch prior"
    );
}

/// **What the twenty-one still do not reach**, recorded so it is a measured hole rather than a
/// silence. [M22 P8b]
///
/// The whole bundle being green is the milestone's exit criterion and is not the same as the engine
/// being covered. Three things a reader would reasonably assume are gated and are not, each carried
/// by unit tests in the module that owns it:
///
/// - **Neither of `engine::tempo_notes`' two sentences is produced by any vector.** P6 measured why:
///   every collapsed motion-start boundary in the corpus is in the *down-the-line* view, and that
///   function reads the face-on anchors alone. The four committed notes that mention tempo are
///   `alignment.py`'s, from a different function with a different prefix — which is why this counts
///   the sentences rather than grepping for the word.
/// - **No shot anywhere is left-handed**, so `infer_spin_axis`'s mirror — the flip ADR-014's addendum
///   got wrong for a milestone — is reached by nothing in `spec/`. The one left-handed vector is
///   synthetic and carries no shot; all fifteen with a shot are right-handed. This is P5's
///   `head_stays_back` finding recurring in the outcome half.
/// - **No vector's flight is both solved and curved**, so the one combination in which all six
///   `FLIGHT_MEASUREMENTS` record at once has never been produced by this repo's own data.
///
/// What *is* reached, contrary to what P6 recorded here: nineteen of the twenty-one produce an
/// `_anchored_on_strike` note. P6's "no vector moves an impact onto a strike" was true of the six it
/// was gated by, and the corpus half landing turns it over.
#[test]
fn three_things_the_twenty_one_do_not_reach() {
    let mut tempo_notes = 0;
    let mut left_handed_shots = 0;
    let mut six_flight_rows = 0;
    let mut strike_notes = 0;

    let mut all = vectors("synthetic");
    all.extend(vectors("corpus"));
    for (id, vector) in &all {
        let expected = &vector["expected"];
        for note in expected["notes"].as_array().into_iter().flatten() {
            let note = note
                .as_str()
                .unwrap_or_else(|| panic!("{id}: a note is a string"));
            if note.starts_with("face-on: no measurable backswing")
                || note.starts_with("face-on: the backswing measures")
            {
                tempo_notes += 1;
            }
            if note.contains("onto the ball strike heard in this clip") {
                strike_notes += 1;
            }
        }
        let input = &vector["input"];
        if input.get("shot").is_some_and(|s| !s.is_null())
            && input.get("handedness").and_then(Value::as_str) == Some("left")
        {
            left_handed_shots += 1;
        }
        let flight_rows = expected["swing"]["measurements"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|row| row["source"] == "model:flight_v1")
            .count();
        if flight_rows == 6 {
            six_flight_rows += 1;
        }
    }
    assert_eq!((tempo_notes, left_handed_shots, six_flight_rows), (0, 0, 0));
    assert_eq!(strike_notes, 19, "the strike note is reached and counted");
}
