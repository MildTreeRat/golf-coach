//! The Rust-owned storage schemas, against the Rust structs they describe. [M35 P7]
//!
//! `swing_manifest.schema.json` and `session_meta.schema.json` became **Rust's** when the manifest
//! and the session file gained `intent` (the M35 plan's call 9): frozen Python's models never
//! declare it (ADR-035 clause 4), so its exporter would write the key back out of both files. So
//! both are edited by hand, `conformance.py::RUST_OWNED_SCHEMAS` names them and the Python
//! freshness pin skips them, and this holds them to the structs instead. A failure here means the
//! *schema* is wrong, where `python_schemas.rs` beside it, which still holds the analysis state,
//! means the struct is.
//!
//! This is `crates/contracts/tests/schemas.rs`'s pattern, here rather than there because the
//! structs are this crate's and `contracts` cannot see them (M35 P1's finding 3). Per shape: the
//! key set Rust writes is the one the schema describes; removing a key is refused exactly when
//! `required` lists it; a `null` is read exactly when the schema admits one; and each `default` the
//! schema spells is what serde fills in. Both files keep their pydantic models, so both stay in
//! `SCHEMA_ROOTS`, and `tests/test_conformance.py` holds `RUST_OWNED_SCHEMAS` to the union of this
//! file's list and `crates/contracts`' own.
//!
//! `pin` is copied from `python_schemas.rs` with the default check added, rather than shared, for
//! the reason that file gives: integration tests are separate binaries.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use contracts::intent::PracticeGoal;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use storage::manifest::{RoleFile, SwingManifest, EXPECTED_ROLES};
use storage::session_meta::SessionMeta;

/// The storage roots Rust owns, as `conformance.py::RUST_OWNED_SCHEMAS` spells them. That set is
/// this list joined with `crates/contracts/tests/schemas.rs`' `RUST_OWNED`, and
/// `tests/test_conformance.py` reads both lists out of the source to hold it so.
const RUST_OWNED: [&str; 2] = ["swing_manifest", "session_meta"];

/// The shapes `intent` reaches. Every Rust-owned copy of them is `shot_result`'s, so a description
/// corrected in one file is corrected in all of them or fails here.
const INTENT_SHAPES: [&str; 4] = [
    "PracticeGoal",
    "PracticeMode",
    "TargetShape",
    "ClubCategory",
];

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

/// pydantic's spelling of an optional, which the hand edit keeps: `anyOf [..., {"type": "null"}]`.
fn admits_null(property: &Value) -> bool {
    property["type"] == "null"
        || property["anyOf"]
            .as_array()
            .is_some_and(|options| options.iter().any(|option| option["type"] == "null"))
}

/// `full` is a valid instance carrying every key, each at a value other than its default. Every
/// difference, in one report.
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
        let read = serde_json::from_value::<T>(without);
        if read.is_ok() == required.contains(key) {
            let schema = if read.is_ok() { "required" } else { "optional" };
            differences.push(format!("{key}: the schema has it {schema}, Rust does not"));
        }
        // A list or a map carries no `default`, as pydantic writes a `default_factory` field, so
        // only a spelled one is compared. A key Rust skips when unset (`intent`) reads back as
        // absent, which is what a spelled `null` means for it.
        if let (Ok(read), Some(declared)) = (read, property.get("default")) {
            let filled = serde_json::to_value(read).expect("serialize");
            let filled = filled.get(key.as_str()).unwrap_or(&Value::Null);
            if filled != declared {
                differences.push(format!(
                    "{key}: the schema's default is {declared}, and serde fills in {filled}"
                ));
            }
        }

        let mut nulled = full.clone();
        nulled[key] = Value::Null;
        let read = serde_json::from_value::<T>(nulled).is_ok();
        if read != admits_null(property) {
            let schema = if read { "refuses" } else { "reads" };
            differences.push(format!(
                "{key}: the schema {schema} null here, Rust does not"
            ));
        }
    }
    assert!(
        differences.is_empty(),
        "spec/schemas/{title} is out of step with crates/storage. The file is edited by hand from \
         M35, so edit it (docs/CONFORMANCE.md §1):\n  {}",
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

/// Every value differs from its default, and `warnings` is non-empty.
fn full_role_file(role: &str) -> Value {
    json!({
        "role": role,
        "filename": format!("{role}.0123456789ab.mov"),
        "content_sha256": "0123456789abcdef",
        "original_filename": "IMG_0001.MOV",
        "content_type": "video/quicktime",
        "size_bytes": 1024,
        "received_at": "2026-08-06T12:00:00.120000Z",
        "warnings": ["the clip is 30 fps"],
    })
}

/// A goal with every key away from its default: what no recorded goal holds (a recorded one keeps
/// `club` at `all`, the M35 plan's call 5), and what the shape admits.
fn full_goal() -> Value {
    json!({
        "mode": "shot_shaping",
        "target_shape": "draw",
        "club": "mid_iron",
        "focus_checkpoint": "tempo",
    })
}

#[test]
fn the_manifest_schema_is_the_struct_rust_writes() {
    let document = schema("swing_manifest");
    pin::<RoleFile>(&document, "RoleFile", full_role_file("face_on"));
    pin::<PracticeGoal>(&document, "PracticeGoal", full_goal());
    pin::<SwingManifest>(
        &document,
        "SwingManifest",
        json!({
            "swing_id": "1",
            "session_id": "2026-08-06",
            "created_at": "2026-08-06T12:00:00Z",
            "updated_at": "2026-08-06T12:05:00-05:00",
            "roles": {"face_on": full_role_file("face_on")},
            "player_id": "aaron",
            "club": "7i",
            "mishit": "confirmed",
            "intent": full_goal(),
        }),
    );

    let roles: Vec<String> = EXPECTED_ROLES.into_iter().map(wire).collect();
    assert_eq!(listed(&shape(&document, "Role")["enum"]), roles, "Role");
    for role in EXPECTED_ROLES {
        assert_eq!(
            wire(role),
            role.as_str(),
            "{role:?}: as_str is the wire name"
        );
    }
}

#[test]
fn the_session_meta_schema_is_the_struct_rust_writes() {
    let document = schema("session_meta");
    pin::<PracticeGoal>(&document, "PracticeGoal", full_goal());
    pin::<SessionMeta>(
        &document,
        "SessionMeta",
        json!({
            "player_id": "aaron",
            "club": "7i",
            "updated_at": "2026-08-12T12:00:00.250000Z",
            "intent": full_goal(),
        }),
    );
}

/// The intent's shapes in each storage root are `shot_result`'s copies, and the key both roots
/// gained says in its description what frozen Python does with it, because that hazard is the
/// first thing a reader of either file needs (the M35 plan's finding 3).
#[test]
fn each_storage_root_carries_shot_results_intent_shapes_and_says_who_writes_intent() {
    let reference = schema("shot_result");
    for name in RUST_OWNED {
        let document = schema(name);
        for title in INTENT_SHAPES {
            assert_eq!(
                shape(&document, title),
                shape(&reference, title),
                "{name}'s `{title}` differs from shot_result's"
            );
        }
        let description = document["properties"]["intent"]["description"]
            .as_str()
            .unwrap_or_else(|| panic!("{name}: `intent` carries no description"));
        assert!(
            description.contains("Rust alone") && description.contains("frozen Python"),
            "{name}: `intent`'s description must say Rust alone writes it and what frozen Python \
             does with it: {description:?}"
        );
    }
}
