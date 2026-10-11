//! The loft a stored shot borrows from the bag of the golfer who hit it, or which of four repairs
//! would give it one. The `loft_for_club` half of `storage/flight_inputs.py`. [M35 P8]
//!
//! `analysis::flight_infer` resolves the launch conditions an HD Golf screen does not print, and
//! the club's loft picks the branch of its spin solve. `ShotData` carries no club and never will (a
//! launch monitor reports a ball, not a bag), so the loft comes from the swing directory's club tag
//! looked up in the golfer's declared bag. A photo entry has a manifest, so it has the same route
//! (the M35 plan's finding 9), and `golf-core run-shot --session` (P11) is the first Rust caller.
//!
//! **Book loft off the bag entry, never a catalogue default and never inferred from the slot**: a
//! `3w` is a club name, not an angle. That is why the absence is [`LoftGap`] and not a bare `None`.
//! Three of the four gaps are a minute's typing on a page that already exists, and M15 P9 sent the
//! golfer to the bay for one of them because it could not tell them apart.
//!
//! # What is not ported
//!
//! The corpus-wide join, `read_flight_inputs` with `FlightInputs` and `_swings_by_shot_photo`:
//! every stored shot filed under the swing holding its photo, earliest arrival first. Its callers
//! are `scripts/simulate_flight.py`, the frozen server and `mcp/flight.py`, none of which has a Rust
//! twin yet, so it is M29's to port beside the first caller that needs it. Both Python routes run
//! the same `_resolve_loft`, which is ported here whole, so the second route will not need a second
//! definition of a declared loft.

use std::collections::BTreeMap;
use std::path::Path;

use contracts::club::ClubId;
use serde::{Deserialize, Serialize};

use crate::bag_store::BagStore;

/// Why a shot has no loft: four different repairs wearing one `None`. `LoftGap`, by wire name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoftGap {
    /// No swing on disk carries this shot's photo, so there is no club tag to read. **Never
    /// produced here**: [`loft_for_club`]'s caller is holding the swing. Kept so the enum names the
    /// set Python's does, for the corpus-wide join that will produce it (the module doc).
    NoSwing,
    /// The swing is there and nobody said what it was hit with: the repair the club cursor exists
    /// for, or `set_club` after the fact.
    NoClubTag,
    /// The swing names a club the golfer's bag has never declared, or there is no golfer to own a
    /// bag, or no bag on disk. All three are the same repair: add the club on the bag page.
    NoBagEntry,
    /// The bag entry exists and carries no loft. The entry's book loft is enough.
    NoDeclaredLoft,
}

impl LoftGap {
    /// Every gap, in declaration order.
    pub const ALL: [LoftGap; 4] = [
        LoftGap::NoSwing,
        LoftGap::NoClubTag,
        LoftGap::NoBagEntry,
        LoftGap::NoDeclaredLoft,
    ];

    /// The wire name, `LoftGap`'s `StrEnum` value.
    pub const fn as_str(self) -> &'static str {
        match self {
            LoftGap::NoSwing => "no_swing",
            LoftGap::NoClubTag => "no_club_tag",
            LoftGap::NoBagEntry => "no_bag_entry",
            LoftGap::NoDeclaredLoft => "no_declared_loft",
        }
    }
}

/// One golfer's declared loft for one club slot, or the gap it fell into. `loft_for_club`.
///
/// **A `Result` where Python returns `(loft, gap)`**, because Python's docstring promises the gap
/// is `None` exactly when the loft is not, and here that is the type rather than a promise. A caller
/// that wants Python's pair has `.ok()` and `.err()`.
///
/// `golfers_dir` is optional for Python's reason: a caller can be built with no golfer registry at
/// all, which is a bag nobody can read rather than a bag with nothing in it, and so the same
/// [`LoftGap::NoBagEntry`] a club tag with nobody to own a bag gets. **An empty `player_id` is no
/// golfer**, as Python's truthiness test reads it. A bag that is missing, unreadable or of an older
/// shape is no bag ([`BagStore::get`]'s tolerance), and so `NoBagEntry` too.
pub fn loft_for_club(
    player_id: Option<&str>,
    club: Option<ClubId>,
    golfers_dir: Option<&Path>,
) -> Result<f64, LoftGap> {
    let entries = match (player_id, golfers_dir) {
        (Some(player_id), Some(golfers_dir)) if !player_id.is_empty() => {
            Some(declared_lofts(&BagStore::new(golfers_dir), player_id))
        }
        _ => None,
    };
    resolve_loft(club, player_id, entries.as_ref())
}

/// Slot → declared loft for one golfer, empty when there is no bag on disk. `_declared_lofts`.
fn declared_lofts(bags: &BagStore, player_id: &str) -> BTreeMap<ClubId, Option<f64>> {
    bags.get(player_id)
        .map(|bag| {
            bag.entries
                .iter()
                .map(|(slot, entry)| (*slot, entry.spec.loft_deg))
                .collect()
        })
        .unwrap_or_default()
}

/// The four-way branch over data already in hand, with no I/O, so the corpus-wide join can share
/// it when it is ported. `_resolve_loft`, branch for branch and in Python's order: the club tag is
/// asked first, so an untagged swing is `NoClubTag` even with nobody to own a bag.
fn resolve_loft(
    club: Option<ClubId>,
    player_id: Option<&str>,
    entries: Option<&BTreeMap<ClubId, Option<f64>>>,
) -> Result<f64, LoftGap> {
    let Some(club) = club else {
        return Err(LoftGap::NoClubTag);
    };
    // A club tag with nobody to own a bag. The tag is real and there is no bag to look it up in,
    // which is the same repair as an undeclared club and is reported as one.
    let (Some(_), Some(entries)) = (player_id, entries) else {
        return Err(LoftGap::NoBagEntry);
    };
    match entries.get(&club) {
        None => Err(LoftGap::NoBagEntry),
        Some(None) => Err(LoftGap::NoDeclaredLoft),
        Some(Some(loft)) => Ok(*loft),
    }
}

#[cfg(test)]
mod tests {
    //! `tests/storage/test_flight_inputs.py`'s three `loft_for_club` cases, and the edges of the
    //! route they leave to the reader: no registry, an empty id, a bag that cannot be read.

    use super::*;
    use contracts::bag::BagEntry;
    use contracts::club_spec::ClubSpec;
    use contracts::Timestamp;
    use std::fs;
    use std::path::PathBuf;

    fn when() -> Timestamp {
        "2026-08-23T03:37:00Z".parse().unwrap()
    }

    /// `golfers_dir`: a registry for `aaron` with no bag yet. `loft_for_club` reads only the bag,
    /// so the golfer record Python's fixture writes is not needed to reach any branch.
    fn golfers() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("golfers");
        (dir, root)
    }

    /// `_declare`.
    fn declare(golfers_dir: &Path, club: ClubId, loft_deg: Option<f64>) {
        let mut spec = ClubSpec::blank(club);
        spec.make = "Titleist".into();
        spec.loft_deg = loft_deg;
        let entry = BagEntry {
            spec,
            recorded_at: when(),
            retired_at: None,
            provenance: None,
        };
        BagStore::new(golfers_dir)
            .set_entry("aaron", &entry, when())
            .unwrap();
    }

    /// `test_the_bag_lookup_is_reachable_without_the_photo_join`.
    #[test]
    fn a_declared_loft_is_the_answer() {
        let (_dir, root) = golfers();
        declare(&root, ClubId::SevenIron, Some(30.5));
        assert_eq!(
            loft_for_club(Some("aaron"), Some(ClubId::SevenIron), Some(&root)),
            Ok(30.5)
        );
    }

    /// `test_the_bag_lookup_reports_the_same_four_gaps`: the distinction is the whole reason the
    /// enum exists, so it must survive this route. `NoSwing` is the one it cannot produce.
    #[test]
    fn the_bag_lookup_reports_each_gap() {
        let (_dir, root) = golfers();
        declare(&root, ClubId::ThreeWood, None);
        let gap = |player: Option<&str>, club: Option<ClubId>| {
            loft_for_club(player, club, Some(&root)).unwrap_err()
        };
        assert_eq!(gap(Some("aaron"), None), LoftGap::NoClubTag);
        assert_eq!(gap(None, Some(ClubId::SevenIron)), LoftGap::NoBagEntry);
        assert_eq!(
            gap(Some("aaron"), Some(ClubId::SevenIron)),
            LoftGap::NoBagEntry
        );
        assert_eq!(
            gap(Some("aaron"), Some(ClubId::ThreeWood)),
            LoftGap::NoDeclaredLoft
        );
    }

    /// `test_a_golfer_with_no_bag_on_disk_is_a_gap_and_not_an_error`.
    #[test]
    fn a_golfer_with_no_bag_on_disk_is_a_gap_and_not_an_error() {
        let (_dir, root) = golfers();
        assert_eq!(
            loft_for_club(Some("aaron"), Some(ClubId::SevenIron), Some(&root)),
            Err(LoftGap::NoBagEntry)
        );
    }

    /// The club tag is asked before anything else, so an untagged swing names the repair that is
    /// actually missing even when there is no registry and no golfer either.
    #[test]
    fn no_club_tag_wins_over_every_missing_bag() {
        assert_eq!(loft_for_club(None, None, None), Err(LoftGap::NoClubTag));
        assert_eq!(
            loft_for_club(Some("aaron"), None, None),
            Err(LoftGap::NoClubTag)
        );
    }

    /// No registry, an empty id and an unreadable bag are each no bag, and none is an error. The
    /// empty id is Python's `if player_id`, which reads `""` as no golfer before any path is made.
    #[test]
    fn no_registry_an_empty_id_and_an_unreadable_bag_are_each_no_bag() {
        let (_dir, root) = golfers();
        declare(&root, ClubId::SevenIron, Some(30.5));
        let club = Some(ClubId::SevenIron);
        assert_eq!(
            loft_for_club(Some("aaron"), club, None),
            Err(LoftGap::NoBagEntry)
        );
        assert_eq!(
            loft_for_club(Some(""), club, Some(&root)),
            Err(LoftGap::NoBagEntry)
        );
        fs::write(BagStore::new(&root).path_for("aaron"), "{not json").unwrap();
        assert_eq!(
            loft_for_club(Some("aaron"), club, Some(&root)),
            Err(LoftGap::NoBagEntry)
        );
    }

    #[test]
    fn the_gaps_are_python_s_wire_names_in_declaration_order() {
        let names: Vec<String> = LoftGap::ALL
            .iter()
            .map(|gap| serde_json::to_string(gap).unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "\"no_swing\"",
                "\"no_club_tag\"",
                "\"no_bag_entry\"",
                "\"no_declared_loft\""
            ]
        );
        for gap in LoftGap::ALL {
            assert_eq!(
                serde_json::to_string(&gap).unwrap(),
                format!("\"{}\"", gap.as_str())
            );
        }
    }
}
