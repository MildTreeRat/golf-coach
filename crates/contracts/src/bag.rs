//! The bag — which physical club is in each slot, what it is, and what used to be.
//! `contracts/bag.py`. [M36 P5]
//!
//! [`crate::club`] gave the vocabulary: `7i` names a **slot**, not an object. This is the object:
//! a per-golfer record of the club occupying the slot, whose specification is
//! [`crate::club_spec::ClubSpec`] (ADR-024 §2, ADR-026 §3). What is declared here is the three
//! facts about *this golfer's declaration* rather than about the manufactured club — when it was
//! declared, when it left the bag, and where its numbers came from.
//!
//! **Why the bag is declared when everything else is derived.** "Clubs used" stays derived from
//! shot history; the bag is a different question with its own answer — a club in the bag you have
//! not hit has no statistics and is not an error, and a club that has left still has real history.
//! `BagProfile` keeps the two apart for that reason.
//!
//! **Retention is not versioning.** A club that leaves the bag is kept on [`Bag::retired`] rather
//! than deleted, so re-declaring last year's 7 iron does not mean retyping a loft. It is *not*
//! ADR-024's deferred bag-entry versioning: nothing joins a shot to the stint that hit it, and the
//! bag-changed caveat reads the **current** entry's `recorded_at`.
//!
//! # Inheritance, as a flattened field
//!
//! Python's `BagEntry` *is* a `ClubSpec` plus three fields. Here it holds one, `#[serde(flatten)]`,
//! so the file is the same flat object and a field added to `ClubSpec` reaches the bag without a
//! second edit — Python's reason for inheriting rather than re-listing.
//!
//! # One order differs from Python's, and only in a refusal
//!
//! `Bag.entries` is a `dict` in insertion order in Python and a [`BTreeMap`] in bag order here.
//! Bag order is what every reader wants (`club_ids` walks it), and the written file is compared by
//! value, never by key order (the M36 plan's call 1). The one place insertion order is observable
//! is which slot a validator names when **two** slots are wrong: Python names the first inserted,
//! this the first in bag order. No vector holds such a bag, and either answer refuses it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::club::ClubId;
use crate::club_spec::{ClubSpec, SpecProvenance};
use crate::golfer::check_player_id;
use crate::{each, nested, ContractError, Timestamp, Validate};

/// The physical club occupying one slot, as the golfer declared it.
///
/// A [`ClubSpec`] — the whole published field list, `club` included — plus the three facts about
/// the declaration. Every spec field stays optional, `loft_deg` most importantly: a golfer who has
/// never put their irons on a loft machine still has a bag. Under ADR-026 §1 the published loft
/// fills the field, so a blank means nobody, the manufacturer included, has said; work that needs a
/// loft refuses **per club** when it is absent (ADR-010 §2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct BagEntry {
    /// The specification, flattened into the entry's own object as Python's subclass is.
    #[serde(flatten)]
    pub spec: ClubSpec,
    /// When this entry was declared. Load-bearing: an entry that changes makes the shots before and
    /// after it two different physical clubs pooled under one name, and the bag-changed caveat
    /// names this date.
    pub recorded_at: Timestamp,
    /// When this club left the bag, or `None` while it is in it. [`Bag`] pins the two states against
    /// the list the entry is in. It exists because a club can leave with no successor, and then
    /// nothing else on disk says when.
    #[serde(default)]
    pub retired_at: Option<Timestamp>,
    /// Where the spec fields came from, or `None` for an entry declared before anything recorded
    /// it. **Not part of club identity** — see [`BagEntry::same_club_as`].
    #[serde(default)]
    pub provenance: Option<SpecProvenance>,
}

crate::validated!(BagEntry);

impl BagEntry {
    /// Is this the same physical club as `other`, ignoring when it was declared and looked up?
    ///
    /// The bag store's whole upsert turns on this: re-saving an unchanged row is an edit and must
    /// not move `recorded_at`, while a genuinely different club retires the old one.
    ///
    /// Python excludes three fields by name from a `model_dump`, so a field added to `ClubSpec` is
    /// compared from the day it is added. Here the specification is one field and the comparison is
    /// its equality, which gives the same guarantee — and the destructuring below makes a fourth
    /// field added *beside* `spec` a compile error until someone says whether it is identity.
    ///
    /// **`provenance` is the exclusion that does not look like a timestamp** (ADR-026 §4). It
    /// carries one, `retrieved_at`, and without excluding it a second lookup of the same club mints
    /// a fresh stamp, compares unequal, retires a good entry, and hands the bag-changed caveat a
    /// sentence about a club that never changed. Nothing raises; that invisibility is why it is
    /// written down here.
    ///
    /// No serial numbers exist on this instrument, so two identically specified 7 irons are one club
    /// as far as anything here can see — the honest answer, since nothing downstream could act on
    /// the distinction. Floats compare as Python's `==` does, so `-0.0` equals `0.0`.
    pub fn same_club_as(&self, other: &BagEntry) -> bool {
        let BagEntry {
            spec,
            recorded_at: _,
            retired_at: _,
            provenance: _,
        } = self;
        *spec == other.spec
    }
}

/// The inherited `_loft_range_is_ordered`, which is the only validator an entry carries.
impl Validate for BagEntry {
    fn validate(&self) -> Result<(), ContractError> {
        self.spec.validate()?;
        nested("BagEntry.provenance", self.provenance.as_ref())
    }
}

/// One golfer's declared bag, plus every club that has left it. The unit the bag store stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct Bag {
    /// The `Golfer.player_id` this bag belongs to, held to the same slug rule, because the store
    /// reads it straight into a path.
    pub player_id: String,
    /// Slot -> the club in it. Sparse: an absent club is undeclared rather than missing.
    #[serde(default)]
    pub entries: BTreeMap<ClubId, BagEntry>,
    /// Finished stints, oldest first. **Append-only**: a restore copies off this list and leaves
    /// the record in place, so a 7 iron swapped out, back in and out again is three stints. A flat
    /// list rather than a map keyed by club, because `BagEntry.club` already names the slot and a
    /// second key is a second thing that can disagree with it.
    #[serde(default)]
    pub retired: Vec<BagEntry>,
    pub updated_at: Timestamp,
}

crate::validated!(Bag);

impl Bag {
    /// The declared clubs, in canonical bag order.
    ///
    /// Walks [`ClubId::ALL`] rather than the map, so the order comes from the declaration and not
    /// from how the map happens to sort; sorting by wire name is the bug this prevents (`3w` lands
    /// between `2h` and `5h` alphabetically).
    pub fn club_ids(&self) -> Vec<ClubId> {
        ClubId::ALL
            .into_iter()
            .filter(|club| self.entries.contains_key(club))
            .collect()
    }

    /// This slot's finished stints, oldest first — so the last is the club held before this one.
    ///
    /// One home for the lookup: a hand-rolled filter at each caller is a place to get the order
    /// backwards, and backwards restores the oldest club a golfer ever owned.
    pub fn retired_for(&self, club: ClubId) -> Vec<&BagEntry> {
        self.retired
            .iter()
            .filter(|entry| entry.spec.club == club)
            .collect()
    }
}

/// The field checks in pydantic's order (`player_id`, then each entry, then each stint), then the
/// two model validators in declaration order, as pydantic runs them. Each refusal is Python's
/// message.
///
/// - `_keys_match_entries`: a slot may not hold a club that says it is a different club. Filing a
///   sand wedge's entry under `7i` surfaces later as a loft attached to the wrong club's history.
/// - `_live_and_retired_are_separated`: `retired_at` must agree with which list the entry is in,
///   both ways — a club the golfer no longer owns presented as the one in their hands, or a live
///   club parked on the shelf, which drops out of `club_ids` with nothing reporting a change.
impl Validate for Bag {
    fn validate(&self) -> Result<(), ContractError> {
        check_player_id("Bag", &self.player_id)?;
        for (slot, entry) in &self.entries {
            nested(&format!("Bag.entries[{slot}]"), Some(entry))?;
        }
        each("Bag.retired", &self.retired)?;

        for (slot, entry) in &self.entries {
            if entry.spec.club != *slot {
                return Err(refusal(format!(
                    "entry filed under {slot} declares itself {}",
                    entry.spec.club
                )));
            }
        }
        for (slot, entry) in &self.entries {
            if entry.retired_at.is_some() {
                return Err(refusal(format!(
                    "entry in slot {slot} is in the bag but carries retired_at"
                )));
            }
        }
        for entry in &self.retired {
            if entry.retired_at.is_none() {
                return Err(refusal(format!(
                    "retired entry for {} has no retired_at",
                    entry.spec.club
                )));
            }
        }
        Ok(())
    }
}

fn refusal(problem: String) -> ContractError {
    ContractError {
        field: "Bag".to_string(),
        problem,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::club_spec::{ShaftFlex, ShaftMaterial};
    use serde_json::json;

    const WHEN: &str = "2026-08-21T12:00:00Z";
    const LATER: &str = "2026-08-21T13:00:00Z";

    fn at(text: &str) -> Timestamp {
        text.parse().expect("a test timestamp")
    }

    fn entry(club: ClubId) -> BagEntry {
        BagEntry {
            spec: ClubSpec::blank(club),
            recorded_at: at(WHEN),
            retired_at: None,
            provenance: None,
        }
    }

    fn with(club: ClubId, edit: impl FnOnce(&mut ClubSpec)) -> BagEntry {
        let mut out = entry(club);
        edit(&mut out.spec);
        out
    }

    fn retired(club: ClubId, when: &str, make: &str) -> BagEntry {
        let mut out = with(club, |s| s.make = make.into());
        out.retired_at = Some(at(when));
        out
    }

    fn bag(entries: Vec<BagEntry>, retired: Vec<BagEntry>) -> Bag {
        Bag {
            player_id: "aaron".into(),
            entries: entries.into_iter().map(|e| (e.spec.club, e)).collect(),
            retired,
            updated_at: at(LATER),
        }
    }

    fn round_trip(bag: &Bag) -> Bag {
        serde_json::from_str(&serde_json::to_string(bag).unwrap()).unwrap()
    }

    #[test]
    fn a_bag_round_trips_through_json_and_unmeasured_stays_unmeasured() {
        let original = bag(
            vec![
                with(ClubId::SevenIron, |s| {
                    s.loft_deg = Some(34.0);
                    s.make = "Titleist".into();
                }),
                with(ClubId::SandWedge, |s| {
                    s.shaft_model = "Dynamic Gold".into();
                    s.length_in = Some(35.25);
                }),
            ],
            vec![],
        );
        let restored = round_trip(&original);
        assert_eq!(restored, original);
        assert_eq!(
            restored.entries[&ClubId::SevenIron].spec.loft_deg,
            Some(34.0)
        );
        assert_eq!(restored.entries[&ClubId::SandWedge].spec.loft_deg, None);
    }

    #[test]
    fn the_entry_is_one_flat_object_on_disk() {
        let written = serde_json::to_value(entry(ClubId::SevenIron)).unwrap();
        let object = written.as_object().unwrap();
        assert_eq!(object["club"], "7i");
        assert_eq!(object["recorded_at"], WHEN);
        assert!(object.contains_key("loft_range_deg"));
        assert!(!object.contains_key("spec"));
    }

    #[test]
    fn club_ids_is_bag_order_and_not_insertion_or_alphabetical() {
        let inserted = [
            ClubId::PitchingWedge,
            ClubId::ThreeWood,
            ClubId::Driver,
            ClubId::SevenIron,
        ];
        let b = bag(inserted.iter().map(|c| entry(*c)).collect(), vec![]);
        assert_eq!(
            b.club_ids(),
            vec![
                ClubId::Driver,
                ClubId::ThreeWood,
                ClubId::SevenIron,
                ClubId::PitchingWedge
            ]
        );
        let mut alphabetical: Vec<&str> = inserted.iter().map(|c| c.as_str()).collect();
        alphabetical.sort();
        let ordered: Vec<&str> = b.club_ids().iter().map(|c| c.as_str()).collect();
        assert_ne!(ordered, alphabetical, "sorted, not bag order");
    }

    #[test]
    fn club_ids_covers_exactly_the_declared_clubs() {
        let every = bag(ClubId::ALL.iter().map(|c| entry(*c)).collect(), vec![]);
        assert_eq!(every.club_ids(), ClubId::ALL.to_vec());
        let partial = bag(vec![entry(ClubId::FiveHybrid)], vec![]);
        assert_eq!(partial.club_ids(), vec![ClubId::FiveHybrid]);
    }

    #[test]
    fn an_empty_bag_is_empty_rather_than_an_error() {
        let parsed: Bag =
            serde_json::from_value(json!({"player_id": "aaron", "updated_at": WHEN})).unwrap();
        assert!(parsed.club_ids().is_empty());
        assert!(parsed.retired.is_empty());
    }

    #[test]
    fn an_entry_filed_under_the_wrong_slot_is_rejected() {
        let mut b = bag(vec![], vec![]);
        b.entries
            .insert(ClubId::SevenIron, entry(ClubId::SandWedge));
        let err = b.validate().unwrap_err();
        assert_eq!(err.problem, "entry filed under 7i declares itself sw");
    }

    #[test]
    fn player_id_must_be_a_slug() {
        for bad in ["Aaron Sierra", "", "../aaron", "aaron_sierra"] {
            let mut b = bag(vec![], vec![]);
            b.player_id = bad.into();
            let err = b.validate().unwrap_err();
            assert!(err.problem.contains("is not a slug"), "{bad:?}");
        }
        let mut b = bag(vec![], vec![]);
        b.player_id = "aaron-sierra".into();
        assert!(b.validate().is_ok());
    }

    #[test]
    fn a_bag_with_a_shelf_round_trips_and_keeps_its_order() {
        let original = bag(
            vec![with(ClubId::SevenIron, |s| {
                s.make = "Ping".into();
                s.loft_deg = Some(32.0);
            })],
            vec![
                {
                    let mut e = retired(ClubId::SevenIron, LATER, "Titleist");
                    e.spec.loft_deg = Some(34.0);
                    e
                },
                retired(ClubId::ThreeWood, LATER, "TaylorMade"),
            ],
        );
        let restored = round_trip(&original);
        assert_eq!(restored, original);
        let makes: Vec<&str> = restored
            .retired
            .iter()
            .map(|e| e.spec.make.as_str())
            .collect();
        assert_eq!(makes, ["Titleist", "TaylorMade"]);
        assert_eq!(restored.retired[0].spec.loft_deg, Some(34.0));
    }

    #[test]
    fn same_club_ignores_the_timestamps_and_provenance_and_nothing_else() {
        let titleist = with(ClubId::SevenIron, |s| {
            s.make = "Titleist".into();
            s.loft_deg = Some(34.0);
            s.shaft_model = "Dynamic Gold".into();
        });
        let mut later = titleist.clone();
        later.recorded_at = at(LATER);
        later.retired_at = Some(at(LATER));
        later.provenance = Some(SpecProvenance {
            source: "typed".into(),
            retrieved_at: at(LATER),
            notes: String::new(),
        });
        assert!(titleist.same_club_as(&later), "timestamps are not identity");
        assert!(titleist.same_club_as(&{
            let mut none = later.clone();
            none.provenance = None;
            none
        }));
    }

    /// `_ODD_VALUES`: one value per comparable field that differs from the blank, each of which
    /// must read as a different club. Applied through JSON by field name, so the list is the
    /// schema's — a key added to `ClubSpec` without a row here fails `every_field_is_listed`.
    fn odd_values() -> serde_json::Map<String, serde_json::Value> {
        let odd = json!({
            "club": "8i", "make": "Ping", "model": "i230", "model_year": 2021,
            "head_type": "game improvement", "set_composition": "4-PW",
            "loft_deg": 41.5, "lie_deg": 62.5, "bounce_deg": 7.0, "grind": "S",
            "offset_mm": 3.2, "face_angle_deg": 1.0, "head_weight_g": 271.0,
            "adjustable_hosel": true, "loft_range_deg": [7.25, 10.75],
            "shaft_model": "Project X", "shaft_material": "graphite", "shaft_flex": "stiff",
            "shaft_weight_g": 120.0, "shaft_torque_deg": 2.1, "shaft_kick_point": "low",
            "length_in": 37.0, "swing_weight": "D2", "total_weight_g": 415.0,
            "grip": "Golf Pride MCC",
            "cor": 0.83, "moi_g_cm2": 5100.0, "usga_conforming": true,
        });
        odd.as_object().unwrap().clone()
    }

    #[test]
    fn every_field_is_listed() {
        let blank = serde_json::to_value(ClubSpec::blank(ClubId::SevenIron)).unwrap();
        let mut spec_keys: Vec<&String> = blank.as_object().unwrap().keys().collect();
        spec_keys.sort();
        let odd = odd_values();
        let mut odd_keys: Vec<&String> = odd.keys().collect();
        odd_keys.sort();
        assert_eq!(spec_keys, odd_keys);
    }

    #[test]
    fn same_club_compares_every_descriptive_field_that_exists() {
        let base = entry(ClubId::SevenIron);
        let base_json = serde_json::to_value(&base).unwrap();
        for (field, value) in odd_values() {
            let mut changed = base_json.clone();
            changed[&field] = value;
            let other: BagEntry = serde_json::from_value(changed).unwrap();
            assert!(
                !base.same_club_as(&other),
                "{field} changed and went unnoticed"
            );
        }
    }

    #[test]
    fn a_second_lookup_of_the_same_club_is_the_same_club() {
        let mut looked_up = with(ClubId::SevenIron, |s| {
            s.make = "Titleist".into();
            s.model = "T150".into();
            s.loft_deg = Some(30.5);
        });
        looked_up.provenance = Some(SpecProvenance {
            source: "llm:claude-opus-5".into(),
            retrieved_at: at(WHEN),
            notes: String::new(),
        });
        let mut again = looked_up.clone();
        again.provenance = Some(SpecProvenance {
            source: "catalogue".into(),
            retrieved_at: at(LATER),
            notes: "served from club_catalogue.json".into(),
        });
        assert!(looked_up.same_club_as(&again));
    }

    #[test]
    fn a_retired_club_may_not_sit_in_the_bag() {
        let mut b = bag(vec![], vec![]);
        b.entries
            .insert(ClubId::SevenIron, retired(ClubId::SevenIron, LATER, ""));
        let err = b.validate().unwrap_err();
        assert_eq!(
            err.problem,
            "entry in slot 7i is in the bag but carries retired_at"
        );
    }

    #[test]
    fn a_live_club_may_not_sit_on_the_shelf() {
        let b = bag(vec![], vec![entry(ClubId::SevenIron)]);
        let err = b.validate().unwrap_err();
        assert_eq!(err.problem, "retired entry for 7i has no retired_at");
    }

    /// pydantic runs the model validators in declaration order, so a slot that is both misfiled and
    /// carrying `retired_at` is refused as misfiled (measured at M36 P5).
    #[test]
    fn the_misfiled_check_runs_before_the_shelf_check() {
        let mut b = bag(vec![], vec![]);
        b.entries
            .insert(ClubId::SevenIron, retired(ClubId::PitchingWedge, LATER, ""));
        let err = b.validate().unwrap_err();
        assert_eq!(err.problem, "entry filed under 7i declares itself pw");
    }

    #[test]
    fn retired_for_is_one_slots_stints_oldest_first() {
        let b = bag(
            vec![],
            vec![
                retired(ClubId::SevenIron, WHEN, "first"),
                retired(ClubId::ThreeWood, WHEN, "a wood"),
                retired(ClubId::SevenIron, LATER, "second"),
            ],
        );
        let makes: Vec<&str> = b
            .retired_for(ClubId::SevenIron)
            .iter()
            .map(|e| e.spec.make.as_str())
            .collect();
        assert_eq!(makes, ["first", "second"]);
        assert!(b.retired_for(ClubId::Driver).is_empty());
    }

    #[test]
    fn the_shelf_is_not_a_view_of_the_bag() {
        let b = bag(
            vec![with(ClubId::SevenIron, |s| s.make = "Ping".into())],
            vec![retired(ClubId::ThreeWood, LATER, "TaylorMade")],
        );
        assert_eq!(b.club_ids(), vec![ClubId::SevenIron]);
        assert_eq!(b.retired_for(ClubId::ThreeWood)[0].spec.make, "TaylorMade");
    }

    #[test]
    fn enum_values_survive_an_entry() {
        let e = with(ClubId::Driver, |s| {
            s.shaft_material = Some(ShaftMaterial::Graphite);
            s.shaft_flex = Some(ShaftFlex::XStiff);
        });
        let written = serde_json::to_value(&e).unwrap();
        assert_eq!(written["shaft_material"], "graphite");
        assert_eq!(written["shaft_flex"], "x_stiff");
        let back: BagEntry = serde_json::from_value(written).unwrap();
        assert_eq!(back, e);
    }
}
