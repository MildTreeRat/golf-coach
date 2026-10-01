//! The three Rust-owned schemas, against the Rust structs they describe. [M32 P9]
//!
//! **Why these three are pinned here and not regenerated.** The rest of `spec/schemas/` is frozen
//! Python's `export_schemas` output, and `tests/test_conformance.py` holds it to that. But
//! `shot_data`, `swing_result` and `swing_bundle_result` describe the shapes M32's ten keys move,
//! and frozen Python never gains those keys (ADR-035 clause 4), so its exporter would write them
//! back out of all three. From M32 those files are edited by hand, the Python pin skips them
//! (`conformance.py::RUST_OWNED_SCHEMAS`), and this file holds them instead (§M32 "Schemas: split
//! ownership"). Generating them from the structs with `schemars` was declined there: the files carry
//! pydantic's bound keywords, Rust keeps those bounds in its `Validate` impls where a derive cannot
//! see them, and restating each one as an attribute is a second copy that drifts.
//!
//! **What is pinned is the key sets, not the spelling.** A key added to `ShotData` or
//! `ShotProvenance` without a schema edit fails here, and so does a schema property with no struct
//! field behind it. How each property is spelled (pydantic's `anyOf [{type}, {type: null}]`,
//! `default`, a Title Case `title`) is the hand edit's job. The one part of the spelling a port
//! cannot do without, the `description` that carries the units, is pinned for the keys M32 added,
//! because no generator wrote those.
//!
//! The pin extends shape by shape as later milestones take more roots over: M36 takes the storage
//! roots, and is where §M32 says to weigh `schemars` again if hand edits prove the larger cost.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use contracts::shot::ShotData;
use contracts::unscored::{UnscoredReason, UNSCORED_REASONS};
use serde_json::{json, Value};

/// The roots Rust owns, spelled as `conformance.py::RUST_OWNED_SCHEMAS` spells them. That constant
/// is what makes the Python pin skip a file, and its own test refuses a name that is not a root; this
/// list is what makes the Rust pin read one. A root added to one and not the other is either
/// unpinned or pinned twice, and the second is the loud one.
const RUST_OWNED: [&str; 3] = ["shot_data", "swing_result", "swing_bundle_result"];

/// The photo-side reasons (M32 P7). They never reach a `SwingResult`, so the swing schemas leave
/// them out. Named as variants rather than strings so a rename is a compile error here rather than
/// a subtraction that silently removes nothing.
const PHOTO_SIDE: [UnscoredReason; 2] = [UnscoredReason::PrintedBlank, UnscoredReason::Misread];

fn spec_dir() -> PathBuf {
    // `CARGO_MANIFEST_DIR` is `crates/contracts`; the spec is two levels up, beside `pyproject.toml`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../spec")
        .canonicalize()
        .expect("spec/ is missing. It is committed, so restore it from git")
}

fn read_json(path: &Path) -> Value {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn schema(name: &str) -> Value {
    read_json(
        &spec_dir()
            .join("schemas")
            .join(format!("{name}.schema.json")),
    )
}

/// Where a shape sits in a schema file: at the root of the file that *is* that shape
/// (`shot_data`'s `ShotData`), and under `$defs` everywhere else. Pydantic puts every nested model
/// in the root's `$defs`, so one level is all there is.
fn shape<'a>(document: &'a Value, title: &str) -> &'a Value {
    if document["title"] == title {
        document
    } else {
        &document["$defs"][title]
    }
}

fn property_names(document: &Value, title: &str) -> BTreeSet<String> {
    shape(document, title)["properties"]
        .as_object()
        .unwrap_or_else(|| panic!("no `{title}` with properties"))
        .keys()
        .cloned()
        .collect()
}

/// The keys `ShotData` and `ShotProvenance` write, read off a serialized instance rather than
/// listed, so the struct is the only list.
///
/// A minimal shot is enough because neither struct skips a key at its default: every `Option` is
/// written as `null` and every map as `{}`, which is why each committed corpus answer carries all
/// ten of M32's keys. Were a field ever to gain a `skip_serializing_if`, this would see a schema
/// property Rust does not write, and that is the right question to be asked then.
fn rust_keys() -> (BTreeSet<String>, BTreeSet<String>) {
    let shot: ShotData = serde_json::from_value(json!({
        "shot_id": "s",
        "session_id": "d",
        "timestamp": "2026-10-01T00:00:00Z",
        "provenance": {"device": "hd_golf", "parse_confidence": 1.0},
    }))
    .expect("a minimal screen shot");
    let written = serde_json::to_value(&shot).expect("serialize");
    let keys = |value: &Value| -> BTreeSet<String> {
        value
            .as_object()
            .expect("an object")
            .keys()
            .cloned()
            .collect()
    };
    (keys(&written), keys(&written["provenance"]))
}

fn wire_name(reason: UnscoredReason) -> String {
    serde_json::to_value(reason)
        .expect("serialize")
        .as_str()
        .expect("a reason serializes as a string")
        .to_owned()
}

/// Every difference in every file, in one report, as `round_trip.rs` does: a hand edit that missed
/// a key in all three files is one fix, and should read as one.
#[test]
fn each_rust_owned_schema_describes_exactly_the_shot_keys_rust_writes() {
    let (shot, provenance) = rust_keys();
    let mut differences = Vec::new();
    for name in RUST_OWNED {
        let document = schema(name);
        for (title, written) in [("ShotData", &shot), ("ShotProvenance", &provenance)] {
            let described = property_names(&document, title);
            let undescribed: Vec<_> = written.difference(&described).collect();
            let unwritten: Vec<_> = described.difference(written).collect();
            if !undescribed.is_empty() {
                differences.push(format!(
                    "{name}: Rust's {title} writes {undescribed:?}, which the schema does not describe"
                ));
            }
            if !unwritten.is_empty() {
                differences.push(format!(
                    "{name}: the schema's {title} describes {unwritten:?}, which Rust does not write"
                ));
            }
        }
    }
    assert!(
        differences.is_empty(),
        "spec/schemas is out of step with crates/contracts. These files are edited by hand from \
         M32, so edit them (docs/CONFORMANCE.md §1):\n  {}",
        differences.join("\n  ")
    );
}

/// The hand edit is made once per file, where pydantic wrote one model into all three. So the
/// copies are held equal to each other: a description corrected in one file and not the other two
/// is exactly the drift a hand-maintained schema invites, and the key-set pin above cannot see it.
#[test]
fn the_three_copies_of_each_shot_shape_are_identical() {
    for title in ["ShotData", "ShotProvenance"] {
        let copies: Vec<(&str, Value)> = RUST_OWNED
            .iter()
            .map(|name| {
                let document = schema(name);
                let mut copy = shape(&document, title).clone();
                // `shot_data` is the one file whose root is `ShotData`, so its copy carries the
                // file's `$defs`. The others' copies sit inside a `$defs` and carry none.
                copy.as_object_mut()
                    .unwrap_or_else(|| panic!("{name} has no `{title}`"))
                    .remove("$defs");
                (*name, copy)
            })
            .collect();
        let (first, reference) = &copies[0];
        for (name, copy) in &copies[1..] {
            let differing = differing_parts(reference, copy);
            assert!(
                differing.is_empty(),
                "{title} in {name} differs from {first}'s at {differing:?}"
            );
        }
    }
}

/// The parts of two copies of one shape that differ: a property by name, anything else by its key.
/// A whole-object `assert_eq!` prints both shapes in full to say that one description moved.
fn differing_parts(a: &Value, b: &Value) -> Vec<String> {
    let keys = |value: &Value| -> BTreeSet<String> {
        value
            .as_object()
            .map(|object| object.keys().cloned().collect())
            .unwrap_or_default()
    };
    let mut parts = Vec::new();
    for key in keys(a).union(&keys(b)) {
        if key == "properties" {
            let (a, b) = (&a[key], &b[key]);
            parts.extend(
                keys(a)
                    .union(&keys(b))
                    .filter(|name| a[*name] != b[*name])
                    .map(|name| format!("properties.{name}")),
            );
        } else if a[key] != b[key] {
            parts.push(key.clone());
        }
    }
    parts
}

/// The swing schemas' `UnscoredReason` is frozen Python's set, which is Rust's wire names without
/// the two photo-side ones (M32 call 13).
///
/// `printed_blank` and `misread` must never be written into a `SwingResult`: frozen Python's enum is
/// closed and would refuse the artifact (ADR-035 clause 4). So a schema admitting them would
/// describe an artifact no reader accepts, and leaving them out is the contract. This makes it a
/// checked choice rather than an omission nobody noticed. Rust's names come from
/// `UNSCORED_REASONS`, which `unscored.rs`'s `every_reason_has_a_row` holds to the whole enum, and
/// the comparison is by name: the two do sit last in that table (M32 P7), but nothing here should
/// depend on it.
#[test]
fn the_swing_schemas_admit_every_reason_but_the_photo_side_two() {
    let rust: BTreeSet<String> = UNSCORED_REASONS
        .iter()
        .map(|(reason, _)| wire_name(*reason))
        .collect();
    let photo_side: BTreeSet<String> = PHOTO_SIDE.into_iter().map(wire_name).collect();
    assert!(
        photo_side.is_subset(&rust),
        "UNSCORED_REASONS lacks a photo-side reason, so subtracting it proves nothing"
    );
    let expected: BTreeSet<String> = rust.difference(&photo_side).cloned().collect();

    for name in ["swing_result", "swing_bundle_result"] {
        let document = schema(name);
        let described: BTreeSet<String> = document["$defs"]["UnscoredReason"]["enum"]
            .as_array()
            .unwrap_or_else(|| panic!("{name} has no `UnscoredReason` enum"))
            .iter()
            .map(|value| value.as_str().expect("a wire name").to_owned())
            .collect();
        assert_eq!(
            described, expected,
            "{name}: `UnscoredReason` must be Rust's reasons minus the photo-side two"
        );
    }
}

/// Every key M32 added carries a `description`, in all three files.
///
/// The keys are read from M32's declaration rather than listed again: `spec/declarations/v17.json`
/// already names each one, as a path under `expected.swing.shot`. The description is where a
/// property's units are, and a port that reads millimetres as inches still type-checks.
#[test]
fn every_key_m32_added_is_described_in_every_rust_owned_schema() {
    let declaration = read_json(&spec_dir().join("declarations").join("v17.json"));
    let added: Vec<(&str, &str)> = declaration["added"]
        .as_array()
        .expect("the declaration lists what it added")
        .iter()
        .map(|path| {
            let path = path.as_str().expect("a ledger path");
            // Provenance first, because its prefix extends the shot's.
            if let Some(key) = path.strip_prefix("expected.swing.shot.provenance.") {
                ("ShotProvenance", key)
            } else if let Some(key) = path.strip_prefix("expected.swing.shot.") {
                ("ShotData", key)
            } else {
                panic!("{path}: outside the shot, and this pin knows only the shot's two shapes")
            }
        })
        .collect();
    assert!(!added.is_empty(), "M32's declaration added nothing");

    let mut undescribed = Vec::new();
    for name in RUST_OWNED {
        let document = schema(name);
        for (title, key) in &added {
            let description = shape(&document, title)["properties"][*key]["description"].as_str();
            if description.is_none_or(str::is_empty) {
                undescribed.push(format!("{name}: {title}.{key}"));
            }
        }
    }
    assert!(
        undescribed.is_empty(),
        "these keys are missing from the schema or carry no description:\n  {}",
        undescribed.join("\n  ")
    );
}
