//! Every device with a profile declares, in `devices.json`, exactly the fields its profile reads.
//! [M34 P9; every device since M42 P8]
//!
//! Two files describe one screen. `crates/contracts/devices.json` says which `ShotData` fields a
//! launch monitor can print, and every grade and caveat downstream is cut from it (ADR-034 §2).
//! `crates/screen/profiles.json` says which tiles the parser looks for and the field each fills. A
//! field declared and never read is a stat the screen shows that no read ever delivers, and a field
//! read and never declared is one the read delivers and `printed_on` then drops as not printed.
//! Neither fails anything when it happens: the stat just goes missing, with every check passing, so
//! the two are held equal here.
//!
//! **Every device that has a profile, and only its screen fields.** M34 wrote this for `hd_golf`
//! alone, the one device with a profile. M42 added `full_swing`, so it walks every profile, and a
//! profile with no declaration fails here too (a shot read with it would be refused by
//! `capability_for`). A declared field whose `from` is not the screen (`FieldSource`, the M42 plan's
//! call 13: Full Swing's impact replay) is delivered by a reader other than this crate's, so it has
//! no tile and is left out of both comparisons. `mock` and `synthetic` have no profile and are not
//! walked: nothing reads them off a screen.
//!
//! **A set first, and then an order.** A declaration lists its fields in the profile's tile order.
//! That was only a convenience for a reader until M35 P3, when `screen::tile_refusals` began
//! listing a shot's blank and misread tiles in the declared order, so that a golfer reads them in
//! the order the screen shows them. The set pin says the two files describe one screen, and
//! `every_declaration_lists_its_fields_in_its_profiles_tile_order` says they list it the same way.
//!
//! **Here and not in `contracts`**, because `contracts` cannot see `screen` (ADR-008 as a cargo edge)
//! and `screen` can see `contracts` (§M34). It is the fork's profile that is pinned, since that is
//! the one that locates `Impact Position V` and the one that has `full_swing`: the frozen Python
//! copy lacks both, and `tests/profile_fork.rs` holds that difference to the ones it declares.

use std::collections::BTreeSet;

use contracts::capability::{capability_for, FieldSource};
use screen::profile::load_profile;

/// Every device a profile names, sorted.
///
/// The crate has no public list of its profiles, and its one public way to see every name is the
/// refusal `load_profile` gives a device no profile names, which lists them all. A name no profile
/// can carry asks for that list without adding an API for a test.
fn profiled_devices() -> Vec<String> {
    load_profile("\u{0}no profile has this name")
        .expect_err("no profile is named that")
        .known
}

/// The fields `device` declares as read off its screen, in the declaration's order.
fn declared(device: &str) -> Vec<&'static str> {
    capability_for(device)
        .unwrap_or_else(|e| panic!("{device} has a profile and no declaration: {e}"))
        .fields
        .iter()
        .filter(|declared| declared.from == FieldSource::Screen)
        .map(|declared| declared.field.as_str())
        .collect()
}

/// The targets of `device`'s profile, in tile order.
fn read_by_the_profile(device: &str) -> Vec<&'static str> {
    load_profile(device)
        .unwrap_or_else(|e| panic!("{e}"))
        .stored_fields()
        .into_iter()
        .map(|field| {
            field
                .target
                .as_deref()
                .expect("a stored field has a target")
        })
        .collect()
}

/// The devices this pin walks are the two with a profile today. A third profile is walked without
/// an edit; this says the walk is not empty, and which two it was written against.
#[test]
fn the_walk_covers_every_profile() {
    assert_eq!(profiled_devices(), ["full_swing", "hd_golf"]);
}

#[test]
fn every_profiled_device_declares_exactly_what_its_profile_reads() {
    for device in profiled_devices() {
        let declared: BTreeSet<&str> = declared(&device).into_iter().collect();
        let read: BTreeSet<&str> = read_by_the_profile(&device).into_iter().collect();
        assert_eq!(
            declared,
            read,
            "{device}: declared and never read: {:?}; read and never declared: {:?}",
            declared.difference(&read).collect::<Vec<_>>(),
            read.difference(&declared).collect::<Vec<_>>()
        );
    }
}

/// `tile_refusals` orders by the declaration, and the screen's order is the profile's, so the two
/// lists have to agree field for field, not only as sets. A layout-only tile fills nothing and is
/// on neither.
#[test]
fn every_declaration_lists_its_fields_in_its_profiles_tile_order() {
    for device in profiled_devices() {
        assert_eq!(declared(&device), read_by_the_profile(&device), "{device}");
    }
}

/// The field M34 brought into the profile, which is what made `hd_golf`'s two sets equal. Until M34
/// P8 it was declared and not read (`devices.json`'s note on it said so); a pin that held without
/// it would be holding the old profile.
#[test]
fn the_v_tile_is_on_both_sides() {
    assert!(declared("hd_golf").contains(&"impact_position_v"));
    assert!(read_by_the_profile("hd_golf").contains(&"impact_position_v"));
}

/// Each profile's layout-only tiles fill no field, so they are on neither side, and the pin
/// compares targets rather than tiles for that reason: `Custom` is HD Golf's settings gear, and
/// Full Swing's header row prints the shot's lie and its distance to the hole beside the total.
#[test]
fn a_tile_with_no_target_is_not_a_field() {
    let expected: [(&str, &[&str]); 2] = [
        ("full_swing", &["Shot Result", "To Hole Yd"]),
        ("hd_golf", &["Custom"]),
    ];
    for (device, layout_only_expected) in expected {
        let profile = load_profile(device).unwrap();
        let layout_only: Vec<&str> = profile
            .fields
            .iter()
            .filter(|field| field.target.is_none())
            .map(|field| field.label.as_str())
            .collect();
        assert_eq!(layout_only, layout_only_expected, "{device}");
        assert_eq!(
            read_by_the_profile(device).len(),
            profile.fields.len() - layout_only.len(),
            "{device}"
        );
    }
}
