//! The many-shot layer's shapes against the schemas frozen Python exports. [M36 P5]
//!
//! `bag.schema.json`, `golfer.schema.json` and `swing_manifest.schema.json` stay **Python-owned**
//! through M36 (the plan's call 8): no shape moves here, so `regenerate --schemas-only` keeps writing
//! them from pydantic and `tests/test_conformance.py` holds them to it. This holds the Rust structs
//! to them from the other side. It is `schemas.rs`'s pin pointed the other way: there Rust owns the
//! file and the file follows the struct; here pydantic owns the file and the struct follows it, so a
//! failure here means the *struct* is wrong.
//!
//! What is pinned, per shape, and each is a question the round trip in `stores.rs` cannot ask,
//! because no vector holds every key in every state:
//!
//! - **The key set.** A property Rust does not write, or a key it writes that pydantic does not
//!   describe.
//! - **Which keys are required.** Removing a key from a full instance is refused exactly when the
//!   schema's `required` lists it, which is where a missing `#[serde(default)]` (or a stray one) shows.
//! - **Which keys admit `null`.** Setting a key to `null` is read exactly when the schema admits it.
//!   pydantic refuses `"make": null`, because `make: str = ""` is not optional, and so must this.
//! - **Every enum's members, in order.** `ClubId`'s order is bag order (the plan's call 12) and
//!   `ShaftFlex`'s is softest to stiffest; both are read, never sorted, so the order is the contract.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use contracts::bag::{Bag, BagEntry};
use contracts::club::ClubId;
use contracts::club_spec::{ShaftFlex, ShaftMaterial, SpecProvenance};
use contracts::golfer::{Golfer, Handedness};
use contracts::mishit::MishitVerdict;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};

fn schema(name: &str) -> Value {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../spec/schemas")
        .join(format!("{name}.schema.json"));
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// A shape is the root of the file that *is* it, or one of its `$defs`.
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
    let described = &shape(document, title)["properties"];
    let described = described
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
        "crates/contracts' {title} is out of step with what pydantic exported:\n  {}",
        differences.join("\n  ")
    );
}

/// An enum's members as the schema lists them, which is pydantic's declaration order.
fn members(document: &Value, title: &str) -> Vec<String> {
    shape(document, title)["enum"]
        .as_array()
        .unwrap_or_else(|| panic!("no `{title}` enum"))
        .iter()
        .map(|value| value.as_str().expect("a wire name").to_owned())
        .collect()
}

fn wire<T: Serialize>(member: T) -> String {
    serde_json::to_value(member)
        .expect("serialize")
        .as_str()
        .expect("a member serializes as a string")
        .to_owned()
}

/// A provenance block, a spec with every key, and an entry around it. Every value differs from its
/// blank, and `retired_at` is set, because an entry in isolation may carry one.
fn full_entry(club: &str, retired: bool) -> Value {
    json!({
        "club": club, "make": "Ping", "model": "i230", "model_year": 2021,
        "head_type": "game improvement", "set_composition": "4-PW",
        "loft_deg": 41.5, "lie_deg": 62.5, "bounce_deg": 7.0, "grind": "S",
        "offset_mm": 0.0, "face_angle_deg": 1.0, "head_weight_g": 271.0,
        "adjustable_hosel": false, "loft_range_deg": [7.25, 10.75],
        "shaft_model": "Project X", "shaft_material": "graphite", "shaft_flex": "stiff",
        "shaft_weight_g": 120.0, "shaft_torque_deg": 2.1, "shaft_kick_point": "low",
        "length_in": 37.0, "swing_weight": "D2", "total_weight_g": 415.0,
        "grip": "Golf Pride MCC", "cor": 0.83, "moi_g_cm2": 5100.0, "usga_conforming": true,
        "recorded_at": "2026-08-21T12:00:00Z",
        "retired_at": if retired { json!("2026-08-22T12:00:00-05:00") } else { Value::Null },
        "provenance": full_provenance(),
    })
}

fn full_provenance() -> Value {
    json!({
        "source": "llm:claude-opus-5",
        "retrieved_at": "2026-09-01T13:51:39.545201Z",
        "notes": "the published loft",
    })
}

#[test]
fn the_bag_shapes_are_the_ones_pydantic_exported() {
    let document = schema("bag");
    pin::<SpecProvenance>(&document, "SpecProvenance", full_provenance());
    pin::<BagEntry>(&document, "BagEntry", full_entry("7i", true));
    pin::<Bag>(
        &document,
        "Bag",
        json!({
            "player_id": "aaron",
            "entries": {"7i": full_entry("7i", false)},
            "retired": [full_entry("7i", true)],
            "updated_at": "2026-08-22T12:00:00Z",
        }),
    );
}

#[test]
fn the_golfer_is_the_shape_pydantic_exported() {
    pin::<Golfer>(
        &schema("golfer"),
        "Golfer",
        json!({
            "player_id": "aaron",
            "display_name": "Aaron",
            "handedness": "left",
            "created_at": "2026-08-06T12:00:00.120000Z",
        }),
    );
}

/// Each enum against the schema that carries it, member for member and in order. `ClubId` is in
/// three schemas, and all three must agree, because the manifest, the session and the bag each name
/// a club and none of them is the one place it is defined.
#[test]
fn every_enum_is_pythons_members_in_pythons_order() {
    let clubs: Vec<String> = ClubId::ALL.into_iter().map(wire).collect();
    for name in ["bag", "swing_manifest", "session_meta"] {
        assert_eq!(members(&schema(name), "ClubId"), clubs, "{name}: ClubId");
    }

    let bag = schema("bag");
    let flexes: Vec<String> = ShaftFlex::ALL.into_iter().map(wire).collect();
    assert_eq!(members(&bag, "ShaftFlex"), flexes, "ShaftFlex");
    let materials: Vec<String> = ShaftMaterial::ALL.into_iter().map(wire).collect();
    assert_eq!(members(&bag, "ShaftMaterial"), materials, "ShaftMaterial");

    let verdicts: Vec<String> = MishitVerdict::ALL.into_iter().map(wire).collect();
    assert_eq!(
        members(&schema("swing_manifest"), "MishitVerdict"),
        verdicts,
        "MishitVerdict"
    );
    let hands: Vec<String> = [Handedness::Right, Handedness::Left]
        .into_iter()
        .map(wire)
        .collect();
    assert_eq!(
        members(&schema("golfer"), "Handedness"),
        hands,
        "Handedness"
    );
}
