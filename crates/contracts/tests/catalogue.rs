//! The committed club catalogue, read the way frozen Python writes it. [M36 P5]
//!
//! `src/golf_coach/clubs/club_catalogue.json` is written by Python's `remember` and read here through
//! `include_str!` (the M36 plan's call 10), so both pins are about that one direction:
//!
//! 1. **Every committed row parses**, and writes back as the row on disk. The reader is tolerant on
//!    purpose (a bad row is skipped, an unreadable file is no catalogue), which is exactly why
//!    something has to read the real file: a stray comma turns the cache off with no symptom beyond
//!    a model call that was supposed to be free. Python's own pin is
//!    `test_the_shipped_catalogue_parses_and_every_row_in_it_validates`.
//! 2. **Every value `contracts/club_spec.py` can write reads back**: each club, material and flex,
//!    a row with every optional key absent (`remember` writes `exclude_defaults=True`), and a row with
//!    every key present, `false` and `0.0` among them — a real zero is a measured fact about a blade,
//!    and must not read as a blank.
//!
//! "Writes back as the row" means `exclude_defaults`: Rust writes every key, `remember` drops each
//! one at its default, so the comparison drops them from Rust's side first.

use contracts::catalogue::{load_catalogue, read_rows, CatalogueRow, SCHEMA_VERSION};
use contracts::club::ClubId;
use contracts::club_spec::{ShaftFlex, ShaftMaterial};
use serde_json::{json, Value};

const CATALOGUE_JSON: &str = include_str!("../../../src/golf_coach/clubs/club_catalogue.json");

/// The keys with no default, which `exclude_defaults` therefore never drops: `ClubSpec.club`, and
/// `SpecProvenance.source` and `retrieved_at`. Every other key defaults to `None` or `""`.
/// `python_schemas.rs` holds the required sets this mirrors to pydantic's.
const SPEC_REQUIRED: [&str; 1] = ["club"];
const PROVENANCE_REQUIRED: [&str; 2] = ["source", "retrieved_at"];

/// `model_dump(mode="json", exclude_defaults=True)` of a row, from Rust's full serialization.
fn as_remembered(row: &CatalogueRow) -> Value {
    let mut written = serde_json::to_value(row).expect("serialize");
    for (block, required) in [
        ("spec", &SPEC_REQUIRED[..]),
        ("provenance", &PROVENANCE_REQUIRED[..]),
    ] {
        written[block]
            .as_object_mut()
            .expect("an object")
            .retain(|key, value| {
                required.contains(&key.as_str()) || !(value.is_null() || value.as_str() == Some(""))
            });
    }
    written
}

fn catalogue_of(rows: &[Value]) -> String {
    json!({"schema_version": SCHEMA_VERSION, "clubs": rows}).to_string()
}

#[test]
fn every_committed_row_parses_and_writes_back_as_the_row_on_disk() {
    let raw: Value = serde_json::from_str(CATALOGUE_JSON).expect("the committed catalogue is JSON");
    assert_eq!(raw["schema_version"], json!(SCHEMA_VERSION));
    let on_disk = raw["clubs"].as_array().expect("a list of rows");
    assert!(!on_disk.is_empty(), "the committed catalogue has no rows");

    let rows = load_catalogue();
    assert_eq!(
        rows.len(),
        on_disk.len(),
        "a committed row does not parse, and the tolerant reader skipped it"
    );
    for (row, disk) in rows.iter().zip(on_disk) {
        assert_eq!(&as_remembered(row), disk, "{}", row.spec.club);
    }
}

#[test]
fn every_value_club_spec_can_write_reads_back() {
    let provenance = json!({"source": "typed", "retrieved_at": "2026-09-01T13:51:39.545201Z"});
    let mut rows: Vec<Value> = Vec::new();
    for club in ClubId::ALL {
        rows.push(json!({"spec": {"club": club, "make": "Titleist", "model": "T150"}, "provenance": provenance}));
    }
    for material in ShaftMaterial::ALL {
        rows.push(json!({"spec": {"club": "7i", "make": "Titleist", "model": "T150", "shaft_material": material}, "provenance": provenance}));
    }
    for flex in ShaftFlex::ALL {
        rows.push(json!({"spec": {"club": "7i", "make": "Titleist", "model": "T150", "shaft_flex": flex}, "provenance": provenance}));
    }
    // Every optional key absent, a hand-typed row's shape (`remember` refuses no make or model, but
    // the reader does not).
    rows.push(json!({"spec": {"club": "7i"}, "provenance": provenance}));
    // Every key present, each away from its blank, and the provenance's notes too.
    rows.push(json!({
        "spec": {
            "club": "driver", "make": "Titleist", "model": "TSR3", "model_year": 2022,
            "head_type": "460cc", "set_composition": "driver", "loft_deg": 9.0, "lie_deg": 58.5,
            "bounce_deg": 0.0, "grind": "none", "offset_mm": 0.0, "face_angle_deg": -1.5,
            "head_weight_g": 200.5, "adjustable_hosel": false, "loft_range_deg": [7.25, 10.75],
            "shaft_model": "Tensei", "shaft_material": "graphite", "shaft_flex": "x_stiff",
            "shaft_weight_g": 65.0, "shaft_torque_deg": 3.4, "shaft_kick_point": "mid",
            "length_in": 45.75, "swing_weight": "D4", "total_weight_g": 310.0,
            "grip": "Golf Pride Tour Velvet 360", "cor": 0.83, "moi_g_cm2": 5100.0,
            "usga_conforming": true,
        },
        "provenance": {
            "source": "llm:claude-opus-5",
            "retrieved_at": "2026-09-01T13:51:39-05:00",
            "notes": "loft range from the maker's page",
        },
    }));

    let read = read_rows(&catalogue_of(&rows)).expect("a catalogue Python could write");
    assert_eq!(
        read.len(),
        rows.len(),
        "a row Python could write was skipped"
    );
    for (row, given) in read.iter().zip(&rows) {
        assert_eq!(&as_remembered(row), given);
    }
}
