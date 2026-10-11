//! `screen::tile_refusals` over the screen family's Rust-read shots. [M35 P3]
//!
//! Every document's `expected.shot` is what [`screen::read`] stamps today (M34 P10 re-recorded the
//! Python-recorded ones through it, and `hand/` was worked for it), so these are the records a
//! golfer's photo becomes. Each test reads the rule off them rather than off shots made up for it,
//! so the cases are the layouts and the OCR damage the stored photos actually have: the bay layout's
//! empty Spin cells, the `Impact Position` pair the tie rule withholds, `---` on the synthetic
//! screens, and one direction word with no number. The shapes no photo has (a blank shown-only
//! tile, an unstamped record, a direct feed, an unknown device) are each one of those shots edited.

mod common;

use std::collections::BTreeMap;

use contracts::capability::{capability_for, FieldUse};
use contracts::shot::{ShotData, ShotSource};
use contracts::unscored::{UnscoredCheckpoint, UnscoredReason};
use serde_json::Value;

use screen::profile::load_profile;
use screen::tile_refusals;

const DEVICE: &str = "hd_golf";

/// Every document and hand vector, by vector `id` (`sub_family` keys by stem, so the `id` is read
/// off each).
fn documents() -> Vec<(String, Value)> {
    let hand = common::sub_family(common::HAND)
        .into_iter()
        .map(|(_, vector)| (vector["id"].as_str().expect("an id").to_string(), vector));
    common::every_document().into_iter().chain(hand).collect()
}

/// Every Rust-read shot the family holds, by vector `id`. `synthetic/unreadable` reads as no shot,
/// and is the one document left out.
fn read_shots() -> Vec<(String, ShotData)> {
    let shots: Vec<(String, ShotData)> = documents()
        .into_iter()
        .filter(|(_, vector)| !vector["expected"]["shot"].is_null())
        .map(|(id, vector)| {
            common::assert_recorded_for_this_parser(&vector);
            let shot = serde_json::from_value(vector["expected"]["shot"].clone())
                .unwrap_or_else(|e| panic!("{id}: expected.shot: {e}"));
            (id, shot)
        })
        .collect();
    assert!(shots.len() > 1, "the screen family holds no read shots");
    shots
}

fn shot(id: &str) -> ShotData {
    read_shots()
        .into_iter()
        .find(|(found, _)| found == id)
        .unwrap_or_else(|| panic!("no vector {id}"))
        .1
}

fn refusals(id: &str, shot: &ShotData) -> Vec<UnscoredCheckpoint> {
    tile_refusals(shot).unwrap_or_else(|e| panic!("{id}: {e}"))
}

/// `ShotData` field → the label of the tile that fills it.
fn labels() -> BTreeMap<String, String> {
    load_profile(DEVICE)
        .expect("the shipped profile")
        .stored_fields()
        .into_iter()
        .map(|tile| {
            let target = tile.target.clone().expect("a stored field has a target");
            (target, tile.label.clone())
        })
        .collect()
}

fn holds_a_value(shot: &ShotData, field: &str) -> bool {
    let written = serde_json::to_value(shot).expect("a ShotData serializes");
    !written[field].is_null()
}

fn located(shot: &ShotData) -> Vec<String> {
    shot.provenance
        .as_ref()
        .and_then(|provenance| provenance.fields_present.clone())
        .expect("a stamped read records what it located")
}

fn raw<'a>(shot: &'a ShotData, label: &str) -> Option<&'a str> {
    let provenance = shot
        .provenance
        .as_ref()
        .expect("a read shot has provenance");
    provenance.raw_fields.get(label).map(String::as_str)
}

/// One real photo, whole: two empty cells and the withheld `Impact Position` pair (`2026-08-23-1`,
/// where the V tile's label was read joined up as `ImpactPosition`). The four come in the device's
/// declared order, which interleaves the two reasons rather than grouping them, and each `detail`
/// is the one the shot vectors (P5) will hold end to end.
#[test]
fn one_bay_photo_refuses_exactly_its_two_blanks_and_its_tied_pair() {
    let id = "screen/corpus/2026-08-23-1";
    let want = [
        (
            "spin_rate",
            UnscoredReason::PrintedBlank,
            "Spin: nothing under the label",
        ),
        (
            "impact_position",
            UnscoredReason::Misread,
            "Impact Position: located, but its label tied with another tile's, so it was not read",
        ),
        (
            "spin_axis",
            UnscoredReason::PrintedBlank,
            "Spin Axis: nothing under the label",
        ),
        (
            "impact_position_v",
            UnscoredReason::Misread,
            "Impact Position V: located, but its label tied with another tile's, so it was not read",
        ),
    ];
    let want: Vec<UnscoredCheckpoint> = want
        .into_iter()
        .map(|(name, reason, detail)| UnscoredCheckpoint {
            name: name.to_string(),
            reason,
            detail: detail.to_string(),
        })
        .collect();
    assert_eq!(refusals(id, &shot(id)), want);
}

/// A located tile with no `raw_fields` key is one the tie rule withheld (the M34 plan's P8 finding
/// 3), and is `misread`, on every shot that has one: the hand-worked `Impact Position` pairs, the
/// two bay photos with that tie, and `carry-twice`'s Carry.
#[test]
fn a_tile_the_tie_rule_withheld_is_misread() {
    let labels = labels();
    let mut withheld = Vec::new();
    for (id, shot) in read_shots() {
        let refused = refusals(&id, &shot);
        for field in located(&shot) {
            if raw(&shot, &labels[&field]).is_some() {
                continue;
            }
            let entry = refused
                .iter()
                .find(|entry| entry.name == field)
                .unwrap_or_else(|| panic!("{id}: withheld {field} is not refused"));
            assert_eq!(entry.reason, UnscoredReason::Misread, "{id}: {field}");
            assert!(entry.detail.starts_with(&format!("{}: ", labels[&field])));
            withheld.push(format!("{id}:{field}"));
        }
    }
    for want in [
        "screen/hand/impact-pair-v-dropped:impact_position",
        "screen/hand/impact-pair-real-joined:impact_position_v",
        "screen/hand/carry-twice:carry_distance",
        "screen/corpus/2026-08-10-1:impact_position",
    ] {
        assert!(withheld.iter().any(|found| found == want), "{want}");
    }
}

/// An empty Spin cell, which is how the bay screen's `---` reaches the OCR on every bay photo the
/// family holds, is `printed_blank` (the M35 plan's decision 3), and so is a `---` the OCR did
/// see, which only the synthetic screens have.
#[test]
fn an_empty_spin_cell_and_a_printed_dash_are_printed_blank() {
    let (mut empty, mut dashes) = (0, 0);
    for (id, shot) in read_shots() {
        let spin = refusals(&id, &shot)
            .into_iter()
            .find(|entry| entry.name == "spin_rate");
        match raw(&shot, "Spin") {
            Some("") => {
                let spin = spin.unwrap_or_else(|| panic!("{id}: empty Spin not refused"));
                assert_eq!(spin.reason, UnscoredReason::PrintedBlank, "{id}");
                assert_eq!(spin.detail, "Spin: nothing under the label", "{id}");
                empty += 1;
            }
            Some("---") => {
                let spin = spin.unwrap_or_else(|| panic!("{id}: `---` Spin not refused"));
                assert_eq!(spin.reason, UnscoredReason::PrintedBlank, "{id}");
                assert_eq!(spin.detail, "Spin: '---', the screen's blank", "{id}");
                dashes += 1;
            }
            _ => {}
        }
    }
    assert!(empty > 0 && dashes > 0, "{empty} empty, {dashes} dashed");
}

/// Text under a tile that gave no value is `misread`: on `2026-08-23-3` the Club Face Angle cell
/// held the direction word `Open` and no number, the one such cell a committed photo has.
#[test]
fn a_word_with_no_number_in_it_is_misread() {
    let id = "screen/corpus/2026-08-23-3";
    let shot = shot(id);
    assert_eq!(raw(&shot, "Club Face Angle"), Some("Open"));
    let face = refusals(id, &shot)
        .into_iter()
        .find(|entry| entry.name == "club_face_angle")
        .expect("the face angle is refused");
    assert_eq!(face.reason, UnscoredReason::Misread);
    assert_eq!(
        face.detail,
        "Club Face Angle: 'Open' under the label, and no value could be read from it"
    );
}

/// The other side of every rule above, on every shot: a refusal names a field the screen printed,
/// that the device lets analysis use, and that holds no value; every such field is refused; and
/// they come in the device's declared order. So a read value is never refused, and a tile the
/// layout lacks (`bounce_and_roll` on a bay photo, `impact_position_v` on a reference one) is never
/// named.
#[test]
fn exactly_the_printed_analysed_tiles_with_no_value_are_refused_in_declared_order() {
    let capability = capability_for(DEVICE).expect("declared");
    let mut refused_anywhere = 0;
    for (id, shot) in read_shots() {
        let present = located(&shot);
        let want: Vec<&str> = capability
            .declared()
            .filter(|field| present.iter().any(|located| located == field))
            .filter(|field| capability.use_of(field) == Some(FieldUse::Analysed))
            .filter(|field| !holds_a_value(&shot, field))
            .collect();
        let refused = refusals(&id, &shot);
        let names: Vec<&str> = refused.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(names, want, "{id}");
        for entry in &refused {
            assert!(!holds_a_value(&shot, &entry.name), "{id}: {}", entry.name);
        }
        refused_anywhere += refused.len();
    }
    assert!(refused_anywhere > 0);
}

/// A shown-only tile left blank is shown blank and refuses nothing (ADR-034 §2): club speed and
/// smash factor blanked on a bay photo, one empty and one dashed, leave its refusals as they were.
#[test]
fn a_blank_shown_only_tile_refuses_nothing() {
    let id = "screen/corpus/2026-08-23-2";
    let shot = shot(id);
    let before = refusals(id, &shot);
    assert!(!before.is_empty());

    let mut blanked = shot.clone();
    blanked.club_head_speed = None;
    blanked.smash_factor = None;
    let provenance = blanked.provenance.as_mut().expect("provenance");
    provenance
        .raw_fields
        .insert("Club Speed".to_string(), String::new());
    provenance
        .raw_fields
        .insert("Smash Factor".to_string(), "---".to_string());
    for field in ["club_head_speed", "smash_factor"] {
        assert!(located(&blanked).iter().any(|located| located == field));
        assert_eq!(
            capability_for(DEVICE).unwrap().use_of(field),
            Some(FieldUse::ShownOnly)
        );
    }
    assert_eq!(refusals(id, &blanked), before);
}

/// Neither shot can say which tile was blank, so neither names one: an unstamped record (every
/// shot frozen Python stored) and a direct feed, which was never a photo. Both start from a shot
/// whose stamped record refuses four.
#[test]
fn an_unstamped_shot_and_a_direct_feed_name_no_tile() {
    let id = "screen/corpus/2026-08-23-1";
    let stamped = shot(id);
    assert_eq!(refusals(id, &stamped).len(), 4);

    let mut unstamped = stamped.clone();
    let provenance = unstamped.provenance.as_mut().expect("provenance");
    provenance.parser_version = 0;
    provenance.fields_present = None;
    assert_eq!(refusals(id, &unstamped), []);

    let mut direct = stamped;
    direct.provenance = None;
    direct.source = ShotSource::Mock;
    assert_eq!(refusals(id, &direct), []);
}

/// A device `devices.json` does not declare is refused, stamped or not, and so is one it declares
/// that no profile reads (`mock`, a direct feed, on a record that says a screen was parsed).
/// Refused and not answered with nothing, because an empty list says every tile was read.
#[test]
fn a_device_nobody_declared_or_no_profile_reads_is_refused() {
    let stamped = shot("screen/corpus/2026-08-23-1");
    let with_device = |device: &str, fields_present: bool| {
        let mut shot = stamped.clone();
        let provenance = shot.provenance.as_mut().expect("provenance");
        provenance.device = device.to_string();
        if !fields_present {
            provenance.fields_present = None;
        }
        shot
    };
    for fields_present in [true, false] {
        let refused = tile_refusals(&with_device("r10", fields_present)).expect_err("undeclared");
        assert_eq!(refused.field, "DeviceCapability.device");
    }
    let refused = tile_refusals(&with_device("mock", true)).expect_err("no mock profile");
    assert_eq!(refused.field, "ShotProvenance.device");
    assert!(
        refused
            .problem
            .contains("Unknown launch-monitor profile 'mock'"),
        "{}",
        refused.problem
    );
}

/// The vectors' JSON reads into the struct `tile_refusals` takes with nothing lost, so every rule
/// above was checked on the record and not on a lossy copy of it.
#[test]
fn every_read_shot_round_trips_through_shot_data() {
    for (id, vector) in documents() {
        let written = &vector["expected"]["shot"];
        if written.is_null() {
            continue;
        }
        let shot: ShotData = serde_json::from_value(written.clone()).expect("a shot");
        let again: Value = serde_json::to_value(&shot).expect("serializes");
        assert_eq!(&again, written, "{id}");
    }
}
