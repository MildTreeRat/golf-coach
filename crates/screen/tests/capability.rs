//! `devices.json`'s `hd_golf` declares exactly the fields this crate's profile reads. [M34 P9]
//!
//! Two files describe one screen. `crates/contracts/devices.json` says which `ShotData` fields HD
//! Golf can print, and every grade and caveat downstream is cut from it (ADR-034 §2).
//! `crates/screen/profiles.json` says which tiles the parser looks for and the field each fills. A
//! field declared and never read is a stat the screen shows that no read ever delivers, and a field
//! read and never declared is one the read delivers and `printed_on` then drops as not printed.
//! Neither fails anything when it happens: the stat just goes missing, with every check passing, so
//! the two are held equal here.
//!
//! **A set, not an order.** The declaration lists its fields in the profile's tile order, which is
//! a convenience for a reader, and nothing downstream reads the order.
//!
//! **Here and not in `contracts`**, because `contracts` cannot see `screen` (ADR-008 as a cargo edge)
//! and `screen` can see `contracts` (§M34). It is the fork's profile that is pinned, since that is
//! the one that locates `Impact Position V`: the frozen Python copy lacks the tile, and
//! `tests/profile_fork.rs` holds that difference to the one it declares.

use std::collections::BTreeSet;

use contracts::capability::capability_for;
use screen::profile::load_profile;

const DEVICE: &str = "hd_golf";

fn declared() -> BTreeSet<&'static str> {
    capability_for(DEVICE)
        .unwrap_or_else(|e| panic!("{e}"))
        .declared()
        .collect()
}

fn read_by_the_profile() -> BTreeSet<&'static str> {
    load_profile(DEVICE)
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

#[test]
fn hd_golf_declares_exactly_what_its_profile_reads() {
    let declared = declared();
    let read = read_by_the_profile();
    assert_eq!(
        declared,
        read,
        "declared and never read: {:?}; read and never declared: {:?}",
        declared.difference(&read).collect::<Vec<_>>(),
        read.difference(&declared).collect::<Vec<_>>()
    );
}

/// The field M34 brought into the profile, which is what made the two sets equal. Until P8 it was
/// declared and not read (`devices.json`'s note on it said so); a pin that held without it would be
/// holding the old profile.
#[test]
fn the_v_tile_is_on_both_sides() {
    assert!(declared().contains("impact_position_v"));
    assert!(read_by_the_profile().contains("impact_position_v"));
}

/// The profile's layout-only tile (`Custom`, the settings gear) fills no field, so it is on neither
/// side, and the pin compares targets rather than tiles for that reason.
#[test]
fn a_tile_with_no_target_is_not_a_field() {
    let profile = load_profile(DEVICE).unwrap();
    let layout_only: Vec<&str> = profile
        .fields
        .iter()
        .filter(|field| field.target.is_none())
        .map(|field| field.label.as_str())
        .collect();
    assert_eq!(layout_only, ["Custom"]);
    assert_eq!(read_by_the_profile().len(), profile.fields.len() - 1);
}
