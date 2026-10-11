//! The analysis state against the schema frozen Python exports. [M36 P8, M35 P7]
//!
//! `analysis_state.schema.json` stays **Python-owned** (the M36 plan's call 8): no shape moves, so
//! `regenerate --schemas-only` keeps writing it from pydantic, and this holds the Rust struct to it
//! from the other side. A failure means the *struct* is wrong.
//!
//! The manifest was pinned here too until M35 P7 gave it `intent`, which frozen Python's model does
//! not declare. That made `swing_manifest` Rust's, with `session_meta` beside it, and their pin is
//! `schemas.rs` in this directory, where a failure means the *schema* is wrong.
//!
//! `crates/contracts/tests/python_schemas.rs` pins the bag and the golfer the same way, and its
//! `pin` is copied here rather than shared: integration tests are separate binaries, and a crate
//! whose only job is to be imported by two of them is more structure than forty lines earn. Per
//! shape: the key set Rust writes is the one pydantic describes; removing a key is refused exactly
//! when `required` lists it; a `null` is read exactly when the schema admits one. Every enum's
//! members are pydantic's, in order.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use storage::state::{AnalysisState, Status};

fn schema(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../spec/schemas")
        .join(format!("{name}.schema.json"));
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn shape<'a>(document: &'a Value, title: &str) -> &'a Value {
    if document["title"] == title {
        document
    } else {
        &document["$defs"][title]
    }
}

fn names(values: impl IntoIterator<Item = String>) -> BTreeSet<String> {
    values.into_iter().collect()
}

/// pydantic spells an optional as `anyOf [..., {"type": "null"}]`.
fn admits_null(property: &Value) -> bool {
    property["type"] == "null"
        || property["anyOf"]
            .as_array()
            .is_some_and(|options| options.iter().any(|option| option["type"] == "null"))
}

/// `full` is a valid instance carrying every key. Every difference, in one report.
fn pin<T: DeserializeOwned + Serialize>(document: &Value, title: &str, full: Value) {
    let described = shape(document, title)["properties"]
        .as_object()
        .unwrap_or_else(|| panic!("no `{title}` with properties"));
    let required = names(
        shape(document, title)["required"]
            .as_array()
            .map(|keys| {
                keys.iter()
                    .map(|k| k.as_str().unwrap().to_owned())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
    );

    let parsed: T = serde_json::from_value(full.clone())
        .unwrap_or_else(|e| panic!("{title}: the full instance is refused: {e}"));
    let written = serde_json::to_value(&parsed).expect("serialize");
    let written = names(written.as_object().expect("an object").keys().cloned());
    let schema_keys = names(described.keys().cloned());
    let given = names(full.as_object().expect("an object").keys().cloned());
    assert_eq!(
        given, schema_keys,
        "{title}: the test's full instance is not full"
    );

    let mut differences = Vec::new();
    for key in written.symmetric_difference(&schema_keys) {
        let side = if written.contains(key) {
            "Rust writes it and the schema does not describe it"
        } else {
            "the schema describes it and Rust does not write it"
        };
        differences.push(format!("{key}: {side}"));
    }
    for (key, property) in described {
        let mut without = full.clone();
        without.as_object_mut().unwrap().remove(key);
        let read = serde_json::from_value::<T>(without).is_ok();
        if read == required.contains(key) {
            let python = if read { "required" } else { "optional" };
            differences.push(format!("{key}: pydantic has it {python}, Rust does not"));
        }

        let mut nulled = full.clone();
        nulled[key] = Value::Null;
        let read = serde_json::from_value::<T>(nulled).is_ok();
        if read != admits_null(property) {
            let python = if read { "refuses" } else { "reads" };
            differences.push(format!("{key}: pydantic {python} null here, Rust does not"));
        }
    }
    assert!(
        differences.is_empty(),
        "crates/storage's {title} is out of step with what pydantic exported:\n  {}",
        differences.join("\n  ")
    );
}

fn wire<T: Serialize>(member: T) -> String {
    serde_json::to_value(member)
        .expect("serialize")
        .as_str()
        .expect("a member serializes as a string")
        .to_owned()
}

fn listed(values: &Value) -> Vec<String> {
    values
        .as_array()
        .expect("an enum")
        .iter()
        .map(|value| value.as_str().expect("a wire name").to_owned())
        .collect()
}

#[test]
fn the_analysis_state_is_the_shape_pydantic_exported() {
    let document = schema("analysis_state");
    pin::<AnalysisState>(
        &document,
        "AnalysisState",
        json!({
            "status": "failed",
            "inputs": {"face_on": "0123456789abcdef"},
            "queued_at": "2026-08-06T12:00:00Z",
            "started_at": "2026-08-06T12:00:01.500000Z",
            "completed_at": "2026-08-06T12:00:09+02:00",
            "duration_seconds": 8.5,
            "error": "pose estimation found no person",
            "partial": true,
            "missing_roles": ["shot_screen"],
            "video": "rendered.mp4",
            "video_codec": "avc1",
            "score": 0.75,
            "headline": "Tempo is the one to work on",
        }),
    );

    let statuses: Vec<String> = Status::ALL.into_iter().map(wire).collect();
    assert_eq!(
        listed(&document["properties"]["status"]["enum"]),
        statuses,
        "Status"
    );
}
