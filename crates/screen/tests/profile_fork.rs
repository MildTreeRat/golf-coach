//! The fork of `profiles.json` equals the frozen Python copy, except a delta declared here. [M34 P3]
//!
//! ADR-032 §5's one copy on disk is given up for this file until M40 (the M34 plan's decision 1),
//! and this pin is what the copy is held by instead. Without it, an edit to either file would
//! quietly make the phone and the lab read one screen two ways, and nothing would say which way was
//! meant. With it, every difference is either in [`DECLARED_DELTA`], or a whole device in
//! [`DECLARED_DEVICES`], or a failure.
//!
//! **The frozen copy is read at test time, from `src/`.** It is the frozen parser's package data
//! (`launch_monitor/screen/profiles.json`), so `include_str!` would tie this crate's build to a file
//! the crate does not own. A test reading it is the same edge `crates/contracts/tests/` already has
//! to `spec/`. (M34 P8's `screen::frozen` did `include_str!` it, because the faithful gate had to
//! parse with it; P10 deleted the module, and the edge with it.)
//!
//! Compared as parsed JSON rather than as bytes, so the fork may be reformatted, but every key,
//! `_comment`s included, is compared.

use std::fs;
use std::path::Path;

use serde_json::Value;

/// `(device, label)` of every field the fork has and the frozen copy does not, on a device both
/// have.
///
/// Empty while the fork was byte-identical (M34 P3). M34 P8 added the bay layout's `Impact Position
/// V`, and that is meant to be the whole of it for the life of the fork: a second entry is a second
/// way the phone and the lab read one screen differently, and wants a decision of its own.
const DECLARED_DELTA: &[(&str, &str)] = &[("hd_golf", "Impact Position V")];

/// Every device the fork has a profile for and the frozen copy does not, compared nowhere else.
/// [M42 P8]
///
/// A different thing from [`DECLARED_DELTA`]: there the phone and the lab read one screen two ways,
/// and here only one of them can read the screen at all. Frozen Python gains nothing from M32
/// (ADR-035 clause 4), so a device added after the freeze is the fork's alone, and there is no
/// frozen profile to drift from. The decision a new entry here wants is the one that made the
/// device a target: for `full_swing`, the M42 plan's decision 1 (the Full Swing grid layout), taken
/// by the user in that plan's interview. Its rules are ones the frozen parser does not have either
/// (`split_merged_labels`, `printed_twice`, `lit_sign`, `printed_unit`), so the frozen copy could
/// not hold it even if it were allowed to change.
const DECLARED_DEVICES: &[&str] = &["full_swing"];

const FORK: &str = include_str!("../profiles.json");

fn frozen() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../src/golf_coach/launch_monitor/screen/profiles.json");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path:?}: {e}"))
}

/// `profiles` with each declared field taken out of its device's `fields`.
///
/// A declared field that is missing, or present twice, panics: a delta that no longer describes
/// the fork is as wrong as an undeclared difference.
fn without(mut profiles: Value, delta: &[(&str, &str)]) -> Value {
    for &(device, label) in delta {
        let fields = profiles["profiles"]
            .as_array_mut()
            .expect("`profiles` is a list")
            .iter_mut()
            .find(|profile| profile["device"] == device)
            .unwrap_or_else(|| panic!("the delta names device {device:?}, which has no profile"))
            ["fields"]
            .as_array_mut()
            .expect("`fields` is a list");
        let before = fields.len();
        fields.retain(|field| field["label"] != label);
        assert_eq!(
            before - fields.len(),
            1,
            "the delta names {device:?}'s {label:?}, which the fork carries {} times",
            before - fields.len()
        );
    }
    profiles
}

/// `profiles` with each declared device's profile taken out of `profiles`.
///
/// A declared device the fork lacks, or carries twice, panics, as a stale [`DECLARED_DELTA`] entry
/// does in [`without`].
fn without_devices(mut profiles: Value, devices: &[&str]) -> Value {
    let list = profiles["profiles"]
        .as_array_mut()
        .expect("`profiles` is a list");
    for &device in devices {
        let before = list.len();
        list.retain(|profile| profile["device"] != device);
        assert_eq!(
            before - list.len(),
            1,
            "DECLARED_DEVICES names {device:?}, which the fork carries {} times",
            before - list.len()
        );
    }
    profiles
}

#[test]
fn the_fork_is_the_frozen_profile_plus_the_declared_delta() {
    let fork: Value = serde_json::from_str(FORK).expect("the fork parses");
    let frozen = frozen();
    // Each declared device must be absent from the frozen copy, or it is not the fork's alone.
    for &device in DECLARED_DEVICES {
        let mut devices = frozen["profiles"].as_array().into_iter().flatten();
        assert!(
            !devices.any(|profile| profile["device"] == device),
            "the frozen copy already has a {device:?} profile, so it is not the fork's alone"
        );
    }
    let fork = without_devices(fork, DECLARED_DEVICES);
    // Each declared field must be absent from the frozen copy, or the delta is not a delta.
    for &(device, label) in DECLARED_DELTA {
        let mut fields = frozen["profiles"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|profile| profile["device"] == device)
            .flat_map(|profile| profile["fields"].as_array().into_iter().flatten());
        assert!(
            !fields.any(|field| field["label"] == label),
            "the frozen profile already has {device:?}'s {label:?}, so it is not a delta"
        );
    }
    assert_eq!(
        without(fork, DECLARED_DELTA),
        frozen,
        "the fork differs from the frozen profile by more than DECLARED_DELTA"
    );
}

/// [`without`] is what makes the pin above mean anything once the delta is not empty, so it is
/// checked now, on a fork made by hand, rather than first exercised by the phase that needs it.
#[test]
fn a_declared_field_is_the_only_thing_taken_out() {
    let frozen: Value = serde_json::from_str(
        r#"{"profiles": [{"device": "d", "fields": [{"label": "A"}, {"label": "B"}]}]}"#,
    )
    .unwrap();
    let fork: Value = serde_json::from_str(
        r#"{"profiles": [{"device": "d", "fields": [{"label": "A"}, {"label": "V"}, {"label": "B"}]}]}"#,
    )
    .unwrap();
    assert_eq!(without(fork.clone(), &[("d", "V")]), frozen);
    assert_ne!(without(fork, &[]), frozen);
}

#[test]
#[should_panic(expected = "which the fork carries 0 times")]
fn a_stale_delta_fails() {
    let profiles: Value =
        serde_json::from_str(r#"{"profiles": [{"device": "d", "fields": [{"label": "A"}]}]}"#)
            .unwrap();
    without(profiles, &[("d", "V")]);
}

/// [`without_devices`] takes out the declared device's whole profile and nothing else, and a
/// declared device the fork no longer has fails rather than passing in silence. [M42 P8]
#[test]
fn a_declared_device_is_the_only_profile_taken_out() {
    let frozen: Value =
        serde_json::from_str(r#"{"profiles": [{"device": "d", "fields": [{"label": "A"}]}]}"#)
            .unwrap();
    let fork: Value = serde_json::from_str(
        r#"{"profiles": [{"device": "d", "fields": [{"label": "A"}]}, {"device": "e", "fields": []}]}"#,
    )
    .unwrap();
    assert_eq!(without_devices(fork.clone(), &["e"]), frozen);
    assert_ne!(without_devices(fork, &[]), frozen);
}

#[test]
#[should_panic(expected = "which the fork carries 0 times")]
fn a_stale_declared_device_fails() {
    let profiles: Value =
        serde_json::from_str(r#"{"profiles": [{"device": "d", "fields": []}]}"#).unwrap();
    without_devices(profiles, &["e"]);
}
