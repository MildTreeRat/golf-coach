//! `docs/CONFORMANCE.md` §3's rules, as library code. [M32 P1]
//!
//! Until M32 this lived in `tests/engine.rs` and returned sentences, which was all a gate needs: a
//! red run prints them and a human reads them. The re-record needs more than that. `golf-core
//! rerecord` (§M32 "The re-record") lets a change move only what it declares, and a declaration
//! names *added keys* apart from *moved values* — so the comparator has to say which of the two each
//! difference is, at a path the gate can match, and a sentence cannot be matched without parsing it
//! back. Hence [`Difference`]: a path and a [`DifferenceKind`], with the old sentences kept as its
//! `Display` so a red gate still reads as it did.
//!
//! # The rules
//!
//! Implemented over [`serde_json::Value`] because that is what `compare_results` compares — parsed
//! values, never bytes, for the reason M22 P2 measured: Python writes `-1.636758133827243e-05` where
//! `serde_json` writes `-0.00001636758133827243` for the identical f64. Bools before ints, because
//! `isinstance(True, int)` is true in Python and the recorded JSON therefore has to be read with the
//! same ordering; floats within [`RTOL`]; everything else exact, list order included. Every
//! difference is accumulated rather than the walk stopping at the first — the template
//! `crates/trigger/tests/conformance.rs` set.
//!
//! # Three kinds, and why a type difference is not a fourth
//!
//! A key the actual side has and the expected side lacks is [`DifferenceKind::AddedKey`]; the
//! reverse is [`DifferenceKind::RemovedKey`]. Everything else — a type difference, a refusal against
//! a number, a list of another length, a float out of tolerance — is [`DifferenceKind::Moved`] at
//! that path, because to a declaration each one says the same thing: *this value is not what was
//! recorded*. Splitting them further would make a declaration spell out *how* a value moved, which
//! the re-record has no use for and a reviewer reads off the two values anyway.
//!
//! # The path spelling
//!
//! Document-rooted, no leading dot, the root is `""`, and a list index is `[i]`:
//! `expected.swing.checkpoint_scores[3].message`. That is the spelling a re-record's ledger is
//! written in (M32's plan, call 3), so a difference's path and a declared path are the same string
//! for the same place. `Display` prints the root as `<root>`, which is for a reader and never part
//! of a path.

use std::fmt;

use serde_json::Value;

/// `docs/CONFORMANCE.md` §3's float rule: `|a - b| <= ATOL + RTOL * |expected|`.
///
/// Sized for a different **summation order**, not a different rule — the last bits of an f64 are the
/// same measurement added up differently; a hundredth of a degree is a different measurement.
pub const RTOL: f64 = 1e-9;
/// The absolute half of the float rule, so a recorded `0.0` is not compared with zero tolerance.
pub const ATOL: f64 = 1e-12;

/// One place where the actual document is not the expected one.
#[derive(Debug, Clone, PartialEq)]
pub struct Difference {
    /// Where, in the module doc's spelling. For a key difference it is the path *of the key*, so a
    /// declared added key and the difference it declares are the same string.
    pub path: String,
    pub kind: DifferenceKind,
}

/// What kind of difference, named from the *actual* side: "added" means the actual document has a
/// key the expected one lacks.
#[derive(Debug, Clone, PartialEq)]
pub enum DifferenceKind {
    /// The actual side has a key the expected side does not.
    AddedKey,
    /// The expected side has a key the actual side does not.
    RemovedKey,
    /// The value at this path is not the recorded one. Both sides are carried whole, so a report can
    /// show the move and a re-record can apply it without walking the documents a second time.
    Moved { expected: Value, actual: Value },
}

/// Every difference between `expected` and `actual` under §3's rules, in walk order.
///
/// An empty result is the pass. Keys are walked in the documents' own order, which is sorted — the
/// workspace does not enable `serde_json`'s `preserve_order` — so a report is stable across runs.
pub fn compare(expected: &Value, actual: &Value) -> Vec<Difference> {
    let mut out = Vec::new();
    walk(expected, actual, "", &mut out);
    out
}

fn walk(expected: &Value, actual: &Value, path: &str, out: &mut Vec<Difference>) {
    let moved = |out: &mut Vec<Difference>| {
        out.push(Difference {
            path: path.to_string(),
            kind: DifferenceKind::Moved {
                expected: expected.clone(),
                actual: actual.clone(),
            },
        });
    };

    match (expected, actual) {
        // A refusal compares equal to nothing but a refusal (ADR-010 §2). Reported as a move and
        // never tested numerically: a port returning `0.0` where this returns `None` has turned
        // "could not measure" into "measured zero".
        (Value::Null, Value::Null) => {}
        (Value::Null, _) | (_, Value::Null) => moved(out),

        // Bools before numbers, because `isinstance(True, int)` is true in Python: the obvious
        // ordering compares a verdict numerically and lets `1` through for `true`.
        (Value::Bool(a), Value::Bool(b)) => {
            if a != b {
                moved(out);
            }
        }
        (Value::String(a), Value::String(b)) => {
            if a != b {
                moved(out);
            }
        }
        (Value::Number(a), Value::Number(b)) => {
            // An int is exact — frame indices, `population_n`, `analysis_version` — and a float is
            // within tolerance. "An int" means *both* sides are, as `compare_results`' own
            // `isinstance(expected, int) and isinstance(actual, int)`: a `3` recorded where a port
            // writes `3.0` falls to the float rule and passes. (The comment this replaced said the
            // kind came off the recorded value alone; neither comparator has ever done that.)
            let differs = match (a.as_i64(), b.as_i64()) {
                (Some(x), Some(y)) => x != y,
                _ => {
                    let (x, y) = (
                        a.as_f64().expect("a JSON number is an f64"),
                        b.as_f64().expect("a JSON number is an f64"),
                    );
                    (x - y).abs() > ATOL + RTOL * x.abs()
                }
            };
            if differs {
                moved(out);
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            // A length difference is one move of the whole list rather than a move per index past
            // the shorter end: an index is only a place in a list both sides agree the shape of.
            if a.len() != b.len() {
                moved(out);
                return;
            }
            for (index, (x, y)) in a.iter().zip(b).enumerate() {
                walk(x, y, &format!("{path}[{index}]"), out);
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            // Same keys, both ways. A key the port *added* is as much a finding as one it dropped:
            // `spec/schemas/` describes the shape and an extra field means the two implementations
            // disagree about what a `SwingBundleResult` is — unless a re-record declares it.
            for key in a.keys() {
                if !b.contains_key(key) {
                    out.push(Difference {
                        path: child(path, key),
                        kind: DifferenceKind::RemovedKey,
                    });
                }
            }
            for key in b.keys() {
                if !a.contains_key(key) {
                    out.push(Difference {
                        path: child(path, key),
                        kind: DifferenceKind::AddedKey,
                    });
                }
            }
            for (key, x) in a {
                if let Some(y) = b.get(key) {
                    walk(x, y, &child(path, key), out);
                }
            }
        }
        _ => moved(out),
    }
}

fn child(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{path}.{key}")
    }
}

/// The sentences `tests/engine.rs` printed before this was library code, kept so a red gate reads
/// the same: "expected a refusal", "type difference", "missing from the port", "the vector does not
/// have". Derived from the two carried values rather than stored, so a `Moved` has one source of
/// truth.
impl fmt::Display for Difference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let at = if self.path.is_empty() {
            "<root>"
        } else {
            &self.path
        };
        match &self.kind {
            DifferenceKind::RemovedKey => write!(f, "{at}: key missing from the port"),
            DifferenceKind::AddedKey => write!(f, "{at}: a key the vector does not have"),
            DifferenceKind::Moved { expected, actual } => match (expected, actual) {
                (Value::Null, other) => write!(f, "{at}: expected a refusal, got {other}"),
                (other, Value::Null) => write!(f, "{at}: expected {other}, got a refusal"),
                (Value::Bool(a), Value::Bool(b)) => write!(f, "{at}: {a} != {b}"),
                (Value::String(a), Value::String(b)) => write!(f, "{at}: {a:?} != {b:?}"),
                (Value::Number(a), Value::Number(b)) => write!(f, "{at}: {a} != {b}"),
                (Value::Array(a), Value::Array(b)) => {
                    write!(f, "{at}: length {} != {}", a.len(), b.len())
                }
                (a, b) => write!(f, "{at}: type difference, {a} against {b}"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn moved(expected: Value, actual: Value) -> DifferenceKind {
        DifferenceKind::Moved { expected, actual }
    }

    /// The comparator is worth nothing if it cannot see a difference, which is why
    /// `tests/test_conformance.py` unit-tests the Python one. The rules that are easy to get wrong,
    /// each with the kind it must be reported as — since M32 the kind is what a re-record gates on,
    /// so a case reported under the wrong kind is a declaration that matches the wrong thing.
    #[test]
    fn the_comparator_sees_the_differences_it_exists_to_see() {
        // (expected, actual, the first difference's path and kind, what its sentence says, and how
        // many differences in all — one everywhere but a swapped list, which moves at both ends.)
        let cases: [(Value, Value, &str, DifferenceKind, &str, usize); 10] = [
            // A refusal is not a zero. ADR-010 §2, the rule this suite is built around.
            (
                json!({"observed": null}),
                json!({"observed": 0.0}),
                "observed",
                moved(json!(null), json!(0.0)),
                "expected a refusal",
                1,
            ),
            (
                json!({"observed": 0.0}),
                json!({"observed": null}),
                "observed",
                moved(json!(0.0), json!(null)),
                "got a refusal",
                1,
            ),
            // A bool is not an int, in the direction Python gets wrong.
            (
                json!({"passed": true}),
                json!({"passed": 1}),
                "passed",
                moved(json!(true), json!(1)),
                "type difference",
                1,
            ),
            // An int is exact.
            (
                json!({"population_n": 40}),
                json!({"population_n": 41}),
                "population_n",
                moved(json!(40), json!(41)),
                "40 != 41",
                1,
            ),
            // A sentence is exact, to the byte.
            (
                json!({"message": "Good tempo - 2.7:1."}),
                json!({"message": "Good tempo - 2.70:1."}),
                "message",
                moved(json!("Good tempo - 2.7:1."), json!("Good tempo - 2.70:1.")),
                "!=",
                1,
            ),
            // A float past the tolerance, and list order.
            (
                json!({"score": 0.611_111_111_111_111}),
                json!({"score": 0.611_111_2}),
                "score",
                moved(json!(0.611_111_111_111_111), json!(0.611_111_2)),
                "!=",
                1,
            ),
            (
                json!({"tips": ["a", "b"]}),
                json!({"tips": ["b", "a"]}),
                "tips[0]",
                moved(json!("a"), json!("b")),
                "tips[0]",
                2,
            ),
            // A list of another length is one move of the list, not one per index.
            (
                json!({"tips": ["a", "b"]}),
                json!({"tips": ["a"]}),
                "tips",
                moved(json!(["a", "b"]), json!(["a"])),
                "length 2 != 1",
                1,
            ),
            // A key the port invented, and one it dropped.
            (
                json!({}),
                json!({"spine_angle": 1.0}),
                "spine_angle",
                DifferenceKind::AddedKey,
                "the vector does not have",
                1,
            ),
            (
                json!({"spine_angle": 1.0}),
                json!({}),
                "spine_angle",
                DifferenceKind::RemovedKey,
                "missing from the port",
                1,
            ),
        ];
        for (expected, actual, path, kind, wanted, count) in cases {
            let found = compare(&expected, &actual);
            assert_eq!(
                found.len(),
                count,
                "comparing {expected} against {actual}: {found:?}"
            );
            assert_eq!(
                found[0],
                Difference {
                    path: path.to_string(),
                    kind,
                },
                "comparing {expected} against {actual}"
            );
            let message = found[0].to_string();
            assert!(
                message.contains(wanted),
                "comparing {expected} against {actual} did not say {wanted:?}: {message}"
            );
        }

        // And it must not cry wolf: a float inside the tolerance is the same measurement summed in a
        // different order, which is exactly what `RTOL` is sized for.
        let quiet = compare(
            &json!({"score": 0.611_111_111_111_111}),
            &json!({"score": 0.611_111_111_111_111_2}),
        );
        assert!(quiet.is_empty(), "{quiet:?}");
    }

    /// The spelling a ledger is written in (call 3): keys dotted, indices bracketed, no leading dot —
    /// and every difference in the document reported, not just the first.
    #[test]
    fn a_path_is_spelled_the_way_a_declaration_spells_it() {
        let found = compare(
            &json!({"a": {"b": [0, {"c": 1}]}, "z": "same"}),
            &json!({"a": {"b": [0, {"c": 2, "d": true}]}, "z": "same"}),
        );
        assert_eq!(
            found,
            vec![
                Difference {
                    path: "a.b[1].d".to_string(),
                    kind: DifferenceKind::AddedKey,
                },
                Difference {
                    path: "a.b[1].c".to_string(),
                    kind: moved(json!(1), json!(2)),
                },
            ]
        );
    }

    /// The root is `""` as a path and `<root>` only when printed: the marker is for a reader, and a
    /// path that carried it would be a path no declaration could spell.
    #[test]
    fn the_root_is_the_empty_path_and_is_printed_as_root() {
        let found = compare(&json!(16), &json!(17));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, "");
        assert_eq!(found[0].to_string(), "<root>: 16 != 17");
        assert!(compare(&json!({"a": [1, 2]}), &json!({"a": [1, 2]})).is_empty());
    }
}
