//! The manifest and the analysis state against the schemas frozen Python exports. [M36 P8]
//!
//! `swing_manifest.schema.json` and `analysis_state.schema.json` stay **Python-owned** through M36
//! (the plan's call 8): no shape moves here, so `regenerate --schemas-only` keeps writing them from
//! pydantic, and this holds the Rust structs to them from the other side. A failure means the
//! *struct* is wrong. M35's `face_on_sha256` is the first change that moves a manifest's shape, and
//! that is when `swing_manifest` becomes Rust-owned.
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
use storage::manifest::{RoleFile, SwingManifest, EXPECTED_ROLES};
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

#[test]
fn the_manifest_is_the_shape_pydantic_exported() {
    let document = schema("swing_manifest");
    pin::<RoleFile>(&document, "RoleFile", full_role_file("face_on"));
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
