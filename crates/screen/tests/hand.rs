//! The shipping parser against the hand-worked screen vectors, `spec/vectors/screen/hand/`.
//! [M34 P8]
//!
//! M34 P8 changed what a parse produces: the tie rule, the `Impact Position V` tile, and the stamp.
//! Frozen Python never answered any of it, so these vectors' oracle is a person (ADR-035 clause 3):
//! each `expected` was worked out on paper, and each `provenance.note` shows the scores, the sums and
//! the counts it was worked from. They gate [`screen::read`], [`parse_screen`] and [`label_ratio`]
//! on the fork's profile, the three the shipping reader and `golf-core rerecord` call, as
//! `tests/parse.rs` and `tests/read.rs` gate them on the vectors frozen Python recorded and M34 P10
//! re-recorded.
//!
//! **No allowance.** These were written in M32's shape, the ten keys Rust's `ShotData` gained
//! included, so every key Rust writes is compared, under `docs/CONFORMANCE.md` §3's rules through
//! [`golf_core::compare`].
//!
//! Each case is one item of the M34 plan's P8 list, or of the M42 plan's P10 list, and
//! [`the_cases_are_the_plans`] holds the set: a case that went missing would take its rule's only
//! vector with it.
//!
//! **The M42 cases are Full Swing's** (`device: "full_swing"`), and the first hand vectors whose
//! boxes a recognizer printed rather than a person laid out: `golf-ocr read --device full_swing
//! --boxes` of two of the grid photos, one of them edited three ways. They pin the four rules a
//! profile opts into (a merged label split, a tile printed twice, a sign read from a lit letter, a
//! number printed in feet) where `cargo test` runs, without the OCR models the photo gate needs
//! (`crates/ocr/tests/full_swing.rs`). No Python twin: frozen Python has none of the four.

mod common;

use std::collections::BTreeSet;

use contracts::shot::SCREEN_PARSER_VERSION;
use golf_core::compare::compare;
use serde_json::{json, Value};

use screen::orient::label_ratio;
use screen::parser::parse_screen;
use screen::profile::load_profile;
use screen::{read, ScreenInput};

/// The M34 plan's P8 list, then the M42 plan's P10 list, each in its order, by file stem.
const CASES: [&str; 10] = [
    "impact-pair-exact",
    "impact-pair-v-read-as-y",
    "impact-pair-v-dropped",
    "impact-pair-v-joined",
    "impact-pair-real-joined",
    "carry-twice",
    "reference-layout",
    "full-swing-grid",
    "full-swing-merged-unlit",
    "full-swing-withheld",
];

/// Every hand vector, checked to be one: a person's answer, at this parser's version.
fn hand_vectors() -> Vec<(String, Value)> {
    let vectors = common::sub_family(common::HAND);
    for (stem, vector) in &vectors {
        assert_eq!(vector["provenance"]["oracle"], "hand", "{stem}");
        assert_eq!(
            vector["screen_parser_version"], SCREEN_PARSER_VERSION,
            "{stem}: a hand vector is worked for the parser that ships"
        );
    }
    vectors
}

fn input(stem: &str, vector: &Value) -> ScreenInput {
    serde_json::from_value(vector["input"].clone()).unwrap_or_else(|e| panic!("{stem}: input: {e}"))
}

/// The shipping reader's whole answer for one vector, in `expected`'s shape.
fn answer(stem: &str, vector: &Value) -> Value {
    let input = input(stem, vector);
    let profile = load_profile(&input.device).expect("every hand vector names a shipped profile");
    json!({
        "label_ratio": label_ratio(&input.boxes, profile),
        "parsed": common::parsed_json(&parse_screen(&input.boxes, profile)),
        "shot": read(&input).unwrap_or_else(|e| panic!("{stem}: {e}")),
    })
}

#[test]
fn the_cases_are_the_plans() {
    let mut want = CASES.to_vec();
    want.sort_unstable();
    let stems: Vec<String> = hand_vectors().into_iter().map(|(stem, _)| stem).collect();
    assert_eq!(stems, want);
}

#[test]
fn every_hand_worked_answer_is_the_shipping_readers() {
    let vectors = hand_vectors();
    let mut failures = Vec::new();
    for (stem, vector) in &vectors {
        let expected = json!({"expected": vector["expected"]});
        let actual = json!({"expected": answer(stem, vector)});
        failures.extend(
            compare(&expected, &actual)
                .into_iter()
                .map(|difference| format!("{stem}: {difference}")),
        );
    }
    assert!(
        failures.is_empty(),
        "{} differences across {} hand vectors:\n{}",
        failures.len(),
        vectors.len(),
        failures.join("\n")
    );
}

/// The located-and-unread mark, as M35 and M37 will read it (the M34 plan's decision 3): a tile in
/// `fields_present` whose label has no `raw_fields` entry is one the tie rule withheld, its value
/// is `null`, and a warning says so; every label with a `raw_fields` entry is in `fields_present`.
/// Read off the vectors, so the rule is checked on the cases that need it and not only on the one
/// that wrote it down.
///
/// Each vector is read against its own device's profile (M42 P10). A Full Swing value withheld by
/// its own rule, a printed-twice disagreement or a sign no letter decides, keeps its `raw_fields`
/// entry, so it is *read* in this sense, and its warning is the rule's (`full-swing-withheld`).
#[test]
fn a_withheld_tile_is_located_and_unread_and_nothing_else_is() {
    let mut withheld_anywhere = 0;
    for (stem, vector) in hand_vectors() {
        let profile = load_profile(&input(&stem, &vector).device).expect("a shipped profile");
        let shot = &vector["expected"]["shot"];
        let provenance = &shot["provenance"];
        let present: BTreeSet<&str> = provenance["fields_present"]
            .as_array()
            .expect("a stamped read records what it located")
            .iter()
            .map(|key| key.as_str().expect("a key"))
            .collect();
        let raw = provenance["raw_fields"].as_object().expect("raw fields");
        let warnings: Vec<&str> = provenance["warnings"]
            .as_array()
            .expect("warnings")
            .iter()
            .map(|w| w.as_str().expect("a warning"))
            .collect();
        for field in profile.stored_fields() {
            let target = field
                .target
                .as_deref()
                .expect("a stored field has a target");
            let read = raw.contains_key(&field.label);
            assert!(
                !read || present.contains(target),
                "{stem}: {} read but not located",
                field.label
            );
            if present.contains(target) && !read {
                withheld_anywhere += 1;
                assert!(
                    shot[target].is_null(),
                    "{stem}: withheld {target} has a value"
                );
                let tie = format!("{}: label tie between ", field.label);
                assert!(
                    warnings.iter().any(|w| w.starts_with(&tie)),
                    "{stem}: withheld {} without a warning saying so",
                    field.label
                );
            }
        }
    }
    assert!(withheld_anywhere > 0, "no hand vector withholds a tile");
}

/// `fields_present` tells the two layouts apart (ADR-014's M31 addendum, §M34's exit): the bay
/// layout carries `impact_position_v` and not `bounce_and_roll`, the reference layout the reverse,
/// whether or not anything under those tiles was read.
///
/// Both are HD Golf's layouts. Full Swing's grid (M42 P10) prints neither tile, and its profile has
/// neither, so its vectors locate neither.
#[test]
fn what_was_located_tells_the_layouts_apart() {
    for (stem, vector) in hand_vectors() {
        let present = &vector["expected"]["shot"]["provenance"]["fields_present"];
        let has = |key: &str| {
            present
                .as_array()
                .is_some_and(|p| p.iter().any(|k| k == key))
        };
        if input(&stem, &vector).device != "hd_golf" {
            assert!(
                !has("bounce_and_roll") && !has("impact_position_v"),
                "{stem}"
            );
            continue;
        }
        let reference = stem == "reference-layout";
        assert_eq!(has("bounce_and_roll"), reference, "{stem}");
        assert_eq!(has("impact_position_v"), !reference, "{stem}");
    }
}
