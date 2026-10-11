//! The device capability model, against `devices.json` and the committed corpus. [M32 P10]
//!
//! Two kinds of test, and they guard different things. The first four hold the *file* to the
//! contract it claims: every name a real `ShotData` key, every `shown_only` with its reason, every
//! entry with its provenance. They are what fails when someone edits `devices.json` by hand. The rest
//! hold the *rule*, ADR-034 §2's declared ∩ `fields_present` with §M32's two special cases, on shots
//! built here, and then on the shots the corpus vectors actually carry.
//!
//! An integration test rather than unit tests in `capability.rs`, because the corpus test has to read
//! `spec/vectors/`, and `round_trip.rs` and `schemas.rs` beside this file are where this crate already
//! does that.

use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use contracts::capability::{
    analysed_on, analysed_view, capability_for, device_of, devices, printed_fields, printed_on,
    FieldUse, NOT_READINGS,
};
use contracts::shot::ShotData;
use flate2::read::GzDecoder;
use serde_json::{json, Value};

// The `ShotData` keys that say what the record *is* rather than what a screen printed are
// `contracts::capability::NOT_READINGS`. This file kept its own copy, `NOT_PRINTED_VALUES`, until
// M35 P2 gave `analysed_view` a reason to need the list in the crate; one list now, not two. A
// device declaring one of them would be declaring a tile that holds a shot's id.

/// A shot built from `fields`, over the four keys every `ShotData` needs.
fn shot(fields: Value) -> ShotData {
    let mut document = json!({
        "shot_id": "s",
        "session_id": "d",
        "timestamp": "2026-10-01T00:00:00Z",
        "source": "screen",
    });
    let object = document.as_object_mut().expect("an object");
    for (key, value) in fields.as_object().expect("fields are an object") {
        object.insert(key.clone(), value.clone());
    }
    serde_json::from_value(document).expect("a valid shot")
}

/// An HD Golf screen shot whose parse located `located`, or recorded nothing when it is `None`.
fn hd_golf_shot(located: Option<&[&str]>, values: Value) -> ShotData {
    let mut fields = values;
    fields["provenance"] = json!({
        "device": "hd_golf",
        "parse_confidence": 1.0,
        "fields_present": located,
    });
    shot(fields)
}

fn set<'a>(fields: impl IntoIterator<Item = &'a str>) -> BTreeSet<&'a str> {
    fields.into_iter().collect()
}

/// The keys a serialized `ShotData` writes, read off an instance so the struct is the only list.
/// Every `Option` is written as `null`, so a minimal shot writes them all.
fn shot_data_keys() -> BTreeSet<String> {
    serde_json::to_value(shot(json!({})))
        .expect("serialize")
        .as_object()
        .expect("an object")
        .keys()
        .cloned()
        .collect()
}

#[test]
fn every_declared_field_is_a_shot_data_key_that_a_screen_could_print() {
    let keys = shot_data_keys();
    let mut wrong = Vec::new();
    for capability in devices() {
        for field in capability.declared() {
            if !keys.contains(field) || NOT_READINGS.contains(&field) {
                wrong.push(format!("{}.{field}", capability.device));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "devices.json declares fields that are not ShotData metrics (it names keys, never screen \
         labels): {wrong:?}"
    );
}

/// [`capability_for`] and `use_of` each take the first match, so a repeat would make the second
/// declaration silently unread.
#[test]
fn no_device_and_no_field_is_declared_twice() {
    let names: Vec<&str> = devices().iter().map(|c| c.device.as_str()).collect();
    assert_eq!(names.len(), set(names.iter().copied()).len(), "{names:?}");
    for capability in devices() {
        let fields: Vec<&str> = capability.declared().collect();
        assert_eq!(
            fields.len(),
            set(fields.iter().copied()).len(),
            "{} declares a field twice: {fields:?}",
            capability.device
        );
    }
}

#[test]
fn every_shown_only_field_says_why() {
    let mut silent = Vec::new();
    for capability in devices() {
        for declared in &capability.fields {
            let said = declared
                .note
                .as_deref()
                .is_some_and(|n| !n.trim().is_empty());
            if declared.field_use == FieldUse::ShownOnly && !said {
                silent.push(format!("{}.{}", capability.device, declared.field));
            }
        }
    }
    assert!(silent.is_empty(), "shown_only with no note: {silent:?}");
}

/// ADR-022: committed data says where it came from.
#[test]
fn every_declaration_says_where_it_came_from() {
    assert!(!devices().is_empty());
    for capability in devices() {
        assert!(
            !capability.source.trim().is_empty(),
            "{} has no source",
            capability.device
        );
    }
}

/// HD Golf's club speed and smash factor are the two fields ADR-034 §2 names `shown_only`, and
/// nothing else on it is.
#[test]
fn hd_golf_shows_club_speed_and_smash_and_analyses_the_rest() {
    let hd_golf = capability_for("hd_golf").expect("hd_golf is declared");
    let shown_only: BTreeSet<&str> = hd_golf
        .fields
        .iter()
        .filter(|declared| declared.field_use == FieldUse::ShownOnly)
        .map(|declared| declared.field.as_str())
        .collect();
    assert_eq!(shown_only, set(["club_head_speed", "smash_factor"]));
}

#[test]
fn printed_is_declared_and_located_per_shot_and_the_union_across_shots() {
    // `low_point` is located and held, and hd_golf does not declare it, so it is not printed.
    let first = hd_golf_shot(
        Some(&["ball_speed", "carry_distance", "low_point"]),
        json!({"ball_speed": 90.7, "carry_distance": 125.6, "low_point": 2.0}),
    );
    let second = hd_golf_shot(
        Some(&["ball_speed", "spin_rate"]),
        json!({"ball_speed": 91.2, "spin_rate": 5991.0}),
    );

    assert_eq!(
        printed_on(&first).unwrap(),
        set(["ball_speed", "carry_distance"])
    );
    assert_eq!(
        printed_on(&second).unwrap(),
        set(["ball_speed", "spin_rate"])
    );
    assert_eq!(
        printed_fields([&first, &second]).unwrap(),
        set(["ball_speed", "carry_distance", "spin_rate"])
    );
    assert_eq!(
        printed_fields([]).unwrap(),
        BTreeSet::new(),
        "no shots, nothing printed"
    );
}

/// The bay layout prints `Impact Position V` where the reference layout prints `Bounce & Roll`
/// (M31 P2). On a bay shot the V tile is printed even when it is blank, which is what lets a later
/// step name that blank; `bounce_and_roll` is simply not there, and must not read as blank.
#[test]
fn a_layout_without_a_tile_excludes_the_field_rather_than_blanking_it() {
    let bay = hd_golf_shot(
        Some(&["carry_distance", "impact_position", "impact_position_v"]),
        json!({"carry_distance": 125.6, "impact_position": "TOE", "impact_position_v": null}),
    );
    let printed = printed_on(&bay).unwrap();
    assert!(
        printed.contains("impact_position_v"),
        "located and blank is printed"
    );
    assert!(
        !printed.contains("bounce_and_roll"),
        "no tile is not printed"
    );
    assert_eq!(
        bay.bounce_and_roll, None,
        "and the metric is None either way"
    );
}

/// `fields_present: None` is every stored shot today. Declared ∩ held: a blank is left out rather
/// than named, so the rule can under-report a blank and never invents one.
#[test]
fn an_unstamped_screen_shot_prints_only_the_declared_fields_it_holds() {
    let unstamped = hd_golf_shot(
        None,
        json!({"ball_speed": 90.7, "spin_rate": null, "low_point": 2.0}),
    );
    assert_eq!(printed_on(&unstamped).unwrap(), set(["ball_speed"]));
}

/// A direct feed was never parsed, so there are no tiles to cut it to.
#[test]
fn a_direct_feed_prints_everything_its_device_declares() {
    let direct = shot(json!({"source": "mock"}));
    assert_eq!(
        device_of(&direct),
        "mock",
        "no provenance: the source's wire name"
    );
    let declared: BTreeSet<&str> = capability_for("mock").unwrap().declared().collect();
    assert_eq!(printed_on(&direct).unwrap(), declared);
    assert!(
        declared.contains("apex_height"),
        "and it is mock's list, not hd_golf's"
    );
}

/// The two M32 keys a reader would most expect a launch monitor to print, and HD Golf does not.
/// Neither a located tile nor a held value makes either one printed.
#[test]
fn hd_golf_never_prints_low_point_or_attack_angle() {
    let hd_golf = capability_for("hd_golf").unwrap();
    let values = json!({"low_point": 2.0, "attack_angle": -4.5});
    for shot in [
        hd_golf_shot(Some(&["low_point", "attack_angle"]), values.clone()),
        hd_golf_shot(None, values),
    ] {
        let printed = printed_on(&shot).unwrap();
        for field in ["low_point", "attack_angle"] {
            assert_eq!(hd_golf.use_of(field), None, "{field} is declared");
            assert!(!printed.contains(field), "{field} printed");
        }
    }
}

#[test]
fn an_unknown_device_is_refused() {
    let refused = capability_for("r10").unwrap_err();
    assert_eq!(refused.field, "DeviceCapability.device");
    assert!(refused.problem.contains("\"r10\""), "{refused}");

    // A direct feed from an undeclared device, and the same inside a set of declared ones.
    let r10 = shot(json!({"source": "r10"}));
    assert!(printed_on(&r10).is_err());
    let declared = hd_golf_shot(None, json!({"ball_speed": 90.7}));
    assert!(printed_fields([&declared, &r10]).is_err());
}

fn corpus_shots() -> Vec<(String, ShotData)> {
    // `CARGO_MANIFEST_DIR` is `crates/contracts`; the spec is two levels up, beside `pyproject.toml`.
    let corpus: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/vectors/corpus");
    let mut paths: Vec<PathBuf> = fs::read_dir(&corpus)
        .unwrap_or_else(|e| panic!("{}: {e}. It is committed, so restore it", corpus.display()))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no corpus vectors under {corpus:?}");

    paths
        .iter()
        .map(|path| {
            let mut text = String::new();
            GzDecoder::new(fs::File::open(path).expect("open vector"))
                .read_to_string(&mut text)
                .expect("decompress vector");
            let vector: Value = serde_json::from_str(&text).expect("parse vector");
            let id = vector["id"].as_str().expect("vector has an id").to_string();
            let shot = serde_json::from_value(vector["input"]["shot"].clone())
                .unwrap_or_else(|e| panic!("{id}: input.shot is not a ShotData: {e}"));
            (id, shot)
        })
        .collect()
}

/// Every stored shot is an HD Golf photo, so this is the device the capability model will be asked
/// about first. It lives here because `contracts` cannot see `analysis`; the `measurements` stage
/// gates the same function through `shot_measurements`' source string.
#[test]
fn device_of_is_hd_golf_on_every_corpus_shot() {
    let others: Vec<(String, String)> = corpus_shots()
        .into_iter()
        .filter(|(_, shot)| device_of(shot) != "hd_golf")
        .map(|(id, shot)| (id, device_of(&shot).to_string()))
        .collect();
    assert!(others.is_empty(), "{others:?}");
}

/// The stored shots are unstamped, so they are read conservatively, and none holds a V tile's value:
/// across the whole corpus the golfer's printed set is inside `hd_golf`'s declaration and lacks
/// `impact_position_v`. When M29's re-read stamps them, this is the test that moves.
#[test]
fn the_corpus_prints_only_what_hd_golf_declares_and_no_v_tile_yet() {
    let shots = corpus_shots();
    assert!(
        shots
            .iter()
            .all(|(_, shot)| shot.provenance.as_ref().unwrap().fields_present.is_none()),
        "a corpus shot records fields_present, so it is no longer read by the unstamped rule"
    );
    let printed = printed_fields(shots.iter().map(|(_, shot)| shot)).unwrap();
    let declared: BTreeSet<&str> = capability_for("hd_golf").unwrap().declared().collect();
    assert!(printed.is_subset(&declared), "{printed:?}");
    assert!(printed.contains("ball_speed"), "the check is not vacuous");
    assert!(!printed.contains("impact_position_v"), "{printed:?}");
}

/// `NOT_READINGS` is held to the struct from the side that matters. A bare shot writes every
/// reading as `null`, because each is an `Option`, so the keys it fills with a value are exactly
/// the identity keys, plus `provenance`, the one identity key that is optional. A key added with a
/// non-null default fails here until someone decides which side of the line it is on, rather than
/// being blanked out of every analysed view without a word. [M35 P2]
#[test]
fn the_keys_that_are_not_readings_are_the_ones_a_bare_shot_fills() {
    let written = serde_json::to_value(shot(json!({}))).expect("serialize");
    let mut filled: BTreeSet<&str> = written
        .as_object()
        .expect("an object")
        .iter()
        .filter(|(_, value)| !value.is_null())
        .map(|(key, _)| key.as_str())
        .collect();
    filled.insert("provenance");
    assert_eq!(filled, set(NOT_READINGS));
    let keys = shot_data_keys();
    assert!(NOT_READINGS.iter().all(|key| keys.contains(*key)));
}

/// Analysed is printed with the shown-only fields taken out, and nothing besides. [M35 P2]
#[test]
fn analysed_is_printed_without_the_shown_only_fields() {
    // An unstamped HD Golf shot holding both shown-only fields, and one HD Golf never prints.
    let stored = hd_golf_shot(
        None,
        json!({"ball_speed": 89.8, "carry_distance": 126.1, "club_head_speed": 103.6,
               "smash_factor": 0.87, "attack_angle": -4.5}),
    );
    assert_eq!(
        printed_on(&stored).unwrap(),
        set([
            "ball_speed",
            "carry_distance",
            "club_head_speed",
            "smash_factor"
        ])
    );
    assert_eq!(
        analysed_on(&stored).unwrap(),
        set(["ball_speed", "carry_distance"])
    );

    // Mock declares smash analysed, so the same field is kept on a mock shot: the rule is the
    // device's declaration, not the field's name.
    let direct = shot(json!({"source": "mock", "smash_factor": 1.38}));
    let analysed = analysed_on(&direct).unwrap();
    assert!(analysed.contains("smash_factor"), "{analysed:?}");
    assert_eq!(analysed, printed_on(&direct).unwrap());
}

/// The view blanks a reading three ways, and keeps everything that is not a reading. [M35 P2]
#[test]
fn the_analysed_view_blanks_every_reading_analysis_may_not_use() {
    // Stamped, with the carry tile not located though the shot holds a carry (the layout lacks the
    // tile), club speed located and shown-only, and a reading HD Golf does not declare at all.
    let stamped = hd_golf_shot(
        Some(&["ball_speed", "club_head_speed", "launch_angle"]),
        json!({"ball_speed": 89.8, "launch_angle": 22.4, "carry_distance": 126.1,
               "club_head_speed": 103.6, "attack_angle": -4.5}),
    );
    let view = analysed_view(&stamped).unwrap();
    assert_eq!(view.ball_speed, Some(89.8));
    assert_eq!(view.launch_angle, Some(22.4));
    assert_eq!(view.carry_distance, None, "not printed");
    assert_eq!(view.club_head_speed, None, "shown only");
    assert_eq!(view.attack_angle, None, "never declared");
    // Who the shot is and how it was read are untouched, so the engine still names its device.
    assert_eq!(view.shot_id, stamped.shot_id);
    assert_eq!(view.timestamp, stamped.timestamp);
    assert_eq!(view.source, stamped.source);
    assert_eq!(view.provenance, stamped.provenance);
    assert_eq!(device_of(&view), "hd_golf");

    // A shot with nothing to blank comes back as itself, which is also the check that the round
    // trip through `Value` loses nothing.
    let direct = shot(
        json!({"source": "mock", "ball_speed": 90.7, "smash_factor": 1.38,
                             "apex_height": 31.0}),
    );
    assert_eq!(analysed_view(&direct).unwrap(), direct);
}

/// On every stored shot the view blanks the two shown-only fields and nothing else: the M35 plan's
/// finding 7 measured here, on the shots the corpus vectors carry. `crates/analysis/tests/shot.rs`
/// pins the consequence, that no recorded shot or flight number moves. [M35 P2]
#[test]
fn on_every_corpus_shot_the_view_blanks_club_speed_and_smash_alone() {
    let shots = corpus_shots();
    for (id, shot) in &shots {
        let mut expected = shot.clone();
        expected.club_head_speed = None;
        expected.smash_factor = None;
        assert_eq!(&analysed_view(shot).unwrap(), &expected, "{id}");
    }
    assert!(
        shots.iter().any(|(_, shot)| shot.club_head_speed.is_some()),
        "no corpus shot holds a club speed, so this blanks nothing"
    );
}

/// An undeclared device is refused by the view as by the printed set it is built on. [M35 P2]
#[test]
fn the_view_refuses_an_unknown_device() {
    let r10 = shot(json!({"source": "r10", "ball_speed": 150.0}));
    assert_eq!(
        analysed_on(&r10).unwrap_err().field,
        "DeviceCapability.device"
    );
    assert_eq!(
        analysed_view(&r10).unwrap_err().field,
        "DeviceCapability.device"
    );
}
