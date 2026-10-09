//! Declared bags — one JSON file per golfer, beside that golfer's own record.
//! `storage/bag_store.py`. [M36 P9]
//!
//! `<root>/<player_id>.bag.json`, sharing a directory with `<player_id>.golfer.json`. The suffix is
//! what keeps them apart: [`crate::golfer_store::GolferStore::list_all`] lists `*.golfer.json` and
//! this store lists nothing, so neither sees the other's files. A bag belongs to exactly one golfer
//! and is keyed by the same id, so a second top-level directory would name the same thing twice.
//!
//! Flat files, the golfer store's shape throughout: tolerant reads, atomic writes, no index.
//!
//! **The store stamps, and the caller's clock is the one it stamps with.** `BagEntry.recorded_at` is
//! required and has no default because the contract stays dumb and the writer stamps it, so
//! [`BagStore::set_entry`] discards whatever `recorded_at` the caller passed, as
//! `get_or_create` discards the handedness of a golfer it already knows. Python's one stamping site
//! reads `datetime.now`; here it is the `now` each mutator takes (the crate doc's "the clock is an
//! argument"), so one call stamps one instant, as Python's single read does.
//!
//! **Nothing here deletes a club.** Replacing or removing one moves the outgoing entry to
//! [`Bag::retired`], which is append-only; `contracts::bag` says why retention is not the bag-entry
//! versioning ADR-024 defers.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use contracts::bag::{Bag, BagEntry};
use contracts::club::ClubId;
use contracts::{Timestamp, Validate};

use crate::StoreError;

/// `_SUFFIX`.
pub const SUFFIX: &str = ".bag.json";

/// A directory of declared bags, one JSON file each.
#[derive(Debug, Clone)]
pub struct BagStore {
    root: PathBuf,
}

impl BagStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        BagStore { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `<root>/<player_id>.bag.json`, with the id taken as given. A writer never makes a path of an
    /// id that is not a slug, because [`Bag`]'s validator refuses it first (`set_entry` says how).
    pub fn path_for(&self, player_id: &str) -> PathBuf {
        self.root.join(format!("{player_id}{SUFFIX}"))
    }

    /// One golfer's bag, or `None` if they have not declared one.
    ///
    /// Tolerant, as [`crate::golfer_store::GolferStore::get`] is: an unreadable or older-schema
    /// record — bad JSON, a slot holding another club, a missing `updated_at`, an id that is not a
    /// slug, a live entry carrying `retired_at` — reads as absent. A corrupt bag should cost
    /// re-entering a loft, not an error on the page someone is holding between swings. The writers
    /// below deliberately do not get that leniency; see [`Self::load_for_write`].
    pub fn get(&self, player_id: &str) -> Option<Bag> {
        let bytes = fs::read(self.path_for(player_id)).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    /// Write the bag as given, atomically (a `.tmp` beside it, then a rename), stamping nothing:
    /// `updated_at` belongs to whoever assembled it, which for every caller here is the method that
    /// also decided what changed.
    ///
    /// **It validates first, where Python's does not need to.** A pydantic `Bag` cannot be built
    /// invalid, so Python's `save` writes whatever it is handed; a Rust [`Bag`] can be, and writing
    /// one would leave a file every reader here answers `None` from. So a bag the contract refuses
    /// is [`StoreError::Invalid`] and nothing is written — pydantic's constructor's refusal, at the
    /// one place a Rust caller can still reach it.
    pub fn save(&self, bag: &Bag) -> Result<(), StoreError> {
        bag.validate().map_err(StoreError::Invalid)?;
        let path = self.path_for(&bag.player_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        crate::write_atomically(bag, &path)?;
        Ok(())
    }

    /// Declare the club in one slot, retiring whatever was there.
    ///
    /// Three cases, and the middle one is the reason this is not a plain map insert:
    ///
    /// 1. **Empty slot** — install, stamped `now`.
    /// 2. **The same physical club** ([`BagEntry::same_club_as`]) — answer the bag as loaded,
    ///    *without writing*. Re-saving an unchanged row is an edit and not a bag change, and the bag
    ///    page saves every row it shows; moving `recorded_at` there would hand the bag-changed caveat
    ///    a sentence about shots all hit with the same club.
    /// 3. **A different club** — stamp the outgoing entry's `retired_at`, append it to the shelf,
    ///    and install the new one.
    ///
    /// The caller's `recorded_at` is discarded in 1 and 3, and its `retired_at` cleared. With no
    /// bag on disk the slot is filled into a new, empty bag for `player_id`, and that bag's
    /// validator is what refuses an id that is not a slug — `../aaron`, `Dave Smith` — before a path
    /// is made of it, as pydantic's constructor refuses it in Python.
    pub fn set_entry(
        &self,
        player_id: &str,
        entry: &BagEntry,
        now: Timestamp,
    ) -> Result<Bag, StoreError> {
        let bag = match self.load_for_write(player_id)? {
            Some(bag) => bag,
            None => {
                let empty = Bag {
                    player_id: player_id.to_string(),
                    entries: BTreeMap::new(),
                    retired: Vec::new(),
                    updated_at: now,
                };
                empty.validate().map_err(StoreError::Invalid)?;
                empty
            }
        };

        let current = bag.entries.get(&entry.spec.club);
        if current.is_some_and(|current| current.same_club_as(entry)) {
            return Ok(bag);
        }

        let mut retired = bag.retired.clone();
        if let Some(current) = current {
            retired.push(BagEntry {
                retired_at: Some(now),
                ..current.clone()
            });
        }
        let live = BagEntry {
            recorded_at: now,
            retired_at: None,
            ..entry.clone()
        };
        let mut entries = bag.entries.clone();
        entries.insert(entry.spec.club, live);
        self.write(&bag, entries, retired, now)
    }

    /// Take a club out of the bag, keeping its record. `None` if the slot was already empty, or
    /// there is no bag, and then nothing is written.
    ///
    /// The removed entry lands on the shelf rather than being dropped, which is what
    /// [`Self::restore_entry`] reads and what makes an accidental removal a one-call repair instead
    /// of a retyped loft. It is also the only record of *when* a club left when nothing replaces it.
    pub fn remove_entry(
        &self,
        player_id: &str,
        club: ClubId,
        now: Timestamp,
    ) -> Result<Option<Bag>, StoreError> {
        let Some(bag) = self.load_for_write(player_id)? else {
            return Ok(None);
        };
        let Some(leaving) = bag.entries.get(&club) else {
            return Ok(None);
        };
        let mut retired = bag.retired.clone();
        retired.push(BagEntry {
            retired_at: Some(now),
            ..leaving.clone()
        });
        let mut entries = bag.entries.clone();
        entries.remove(&club);
        self.write(&bag, entries, retired, now).map(Some)
    }

    /// Put back the club this slot held before the current one. `None` if it never held another,
    /// or there is no bag.
    ///
    /// A **copy** comes off the shelf and the shelved stint stays where it is, so a club that goes
    /// out and comes back is two stints rather than one overwritten record. That is also why this
    /// routes through [`Self::set_entry`] rather than reimplementing the upsert: the restored club
    /// is just another declaration, and it takes a fresh `recorded_at` because it re-entered the bag
    /// today. A restore whose stint is the club already in the slot is `set_entry`'s same-club case,
    /// answered and not written (the storage family's `bag-remove-restore`).
    ///
    /// [`Bag::retired_for`] is oldest first, so its last is the immediately previous club. Reaching
    /// further back is `set_entry` with an earlier stint, which needs nothing here.
    pub fn restore_entry(
        &self,
        player_id: &str,
        club: ClubId,
        now: Timestamp,
    ) -> Result<Option<Bag>, StoreError> {
        let Some(bag) = self.load_for_write(player_id)? else {
            return Ok(None);
        };
        let Some(previous) = bag.retired_for(club).last().map(|stint| (*stint).clone()) else {
            return Ok(None);
        };
        self.set_entry(player_id, &previous, now).map(Some)
    }

    /// Read before a write, telling "no bag yet" from "a bag I cannot read".
    ///
    /// [`Self::get`] collapses those two into `None`, which is right for a reader and dangerous
    /// here: a writer that treats an unreadable file as an empty bag replaces the whole bag *and its
    /// entire shelf* with whatever single club it was asked to set. The shelf is the part that was
    /// meant to survive replacement, so the failure got worse the moment it existed. So anything at
    /// the path that does not read as a valid [`Bag`] — a directory included — is refused, with
    /// frozen Python's message and the path as the platform spells it.
    fn load_for_write(&self, player_id: &str) -> Result<Option<Bag>, StoreError> {
        let path = self.path_for(player_id);
        if !path.exists() {
            return Ok(None);
        }
        fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .map(Some)
            .ok_or_else(|| {
                StoreError::Refused(format!(
                    "bag at {} exists but cannot be read; refusing to overwrite it",
                    path.display()
                ))
            })
    }

    /// Rebuild, validate and save. **Constructed rather than edited in place on purpose**: both of
    /// [`Bag`]'s model validators guard the invariants these three methods are the only thing
    /// maintaining — a slot's key matches its entry, and a retired entry never sits in `entries` —
    /// and building the bag afresh and validating it (in [`Self::save`]) is what makes a mistake in
    /// this file fail here instead of on the next read. Python's `model_copy` skips validators,
    /// which is the reason it builds a new `Bag`; the same reason holds here.
    fn write(
        &self,
        bag: &Bag,
        entries: BTreeMap<ClubId, BagEntry>,
        retired: Vec<BagEntry>,
        at: Timestamp,
    ) -> Result<Bag, StoreError> {
        let rebuilt = Bag {
            player_id: bag.player_id.clone(),
            entries,
            retired,
            updated_at: at,
        };
        self.save(&rebuilt)?;
        Ok(rebuilt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::club_spec::{ClubSpec, SpecProvenance};

    const STALE: &str = "2020-01-01T00:00:00Z";

    fn at(text: &str) -> Timestamp {
        text.parse().expect("a test timestamp")
    }

    /// Each call its own instant, a minute apart, so an ordering assertion means something.
    fn tick(minute: u32) -> Timestamp {
        at(&format!("2026-08-06T12:{minute:02}:00Z"))
    }

    /// `test_bag_store.py::_entry`: a declaration as a caller writes one, `recorded_at` supplied and
    /// about to be ignored.
    fn entry(club: ClubId, edit: impl FnOnce(&mut ClubSpec)) -> BagEntry {
        let mut spec = ClubSpec::blank(club);
        edit(&mut spec);
        BagEntry {
            spec,
            recorded_at: at(STALE),
            retired_at: None,
            provenance: None,
        }
    }

    fn made(club: ClubId, make: &str) -> BagEntry {
        entry(club, |s| s.make = make.into())
    }

    fn store() -> (tempfile::TempDir, BagStore) {
        let dir = tempfile::tempdir().unwrap();
        let bags = BagStore::new(dir.path().join("golfers"));
        (dir, bags)
    }

    fn makes(entries: &[&BagEntry]) -> Vec<String> {
        entries.iter().map(|e| e.spec.make.clone()).collect()
    }

    #[test]
    fn no_bag_and_a_corrupt_bag_both_read_as_none() {
        let (_dir, bags) = store();
        assert_eq!(bags.get("aaron"), None);
        bags.set_entry("aaron", &made(ClubId::SevenIron, ""), tick(0))
            .unwrap();
        fs::write(bags.path_for("aaron"), "{not json").unwrap();
        assert_eq!(bags.get("aaron"), None);
    }

    #[test]
    fn a_bag_round_trips_through_the_store_and_save_stamps_nothing() {
        let (_dir, bags) = store();
        bags.set_entry(
            "aaron",
            &entry(ClubId::SevenIron, |s| {
                s.loft_deg = Some(34.0);
                s.make = "Titleist".into();
            }),
            tick(0),
        )
        .unwrap();
        bags.set_entry(
            "aaron",
            &entry(ClubId::PitchingWedge, |s| s.loft_deg = Some(46.0)),
            tick(1),
        )
        .unwrap();
        let stored = bags.get("aaron").unwrap();
        assert_eq!(
            stored.club_ids(),
            [ClubId::SevenIron, ClubId::PitchingWedge]
        );
        assert_eq!(stored.entries[&ClubId::SevenIron].spec.make, "Titleist");

        let given = Bag {
            player_id: "gina".into(),
            entries: BTreeMap::new(),
            retired: Vec::new(),
            updated_at: at(STALE),
        };
        bags.save(&given).unwrap();
        assert_eq!(bags.get("gina"), Some(given));
        assert!(!bags.path_for("gina").with_extension("tmp").exists());
    }

    #[test]
    fn the_store_stamps_recorded_at_and_ignores_the_callers() {
        let (_dir, bags) = store();
        let mut given = made(ClubId::SevenIron, "Ping");
        given.retired_at = Some(at(STALE));
        let stored = bags.set_entry("aaron", &given, tick(5)).unwrap();
        let live = &stored.entries[&ClubId::SevenIron];
        assert_eq!(live.recorded_at, tick(5));
        assert_eq!(live.retired_at, None);
        assert_eq!(stored.updated_at, tick(5));
    }

    /// The same club again, or the same club with only a fresh `provenance` (the M12 P2 pin driven
    /// through the store), is answered untouched and not written.
    #[test]
    fn re_saving_or_re_looking_up_the_same_club_moves_nothing() {
        let (_dir, bags) = store();
        let spec = |s: &mut ClubSpec| {
            s.make = "Titleist".into();
            s.loft_deg = Some(30.5);
        };
        let mut first = entry(ClubId::SevenIron, spec);
        first.provenance = Some(SpecProvenance {
            source: "llm:claude-opus-5".into(),
            retrieved_at: at(STALE),
            notes: String::new(),
        });
        bags.set_entry("aaron", &first, tick(0)).unwrap();
        let written = fs::read_to_string(bags.path_for("aaron")).unwrap();

        let mut again = entry(ClubId::SevenIron, spec);
        again.provenance = Some(SpecProvenance {
            source: "catalogue".into(),
            retrieved_at: at("2026-08-31T00:00:00Z"),
            notes: String::new(),
        });
        let answered = bags.set_entry("aaron", &again, tick(1)).unwrap();

        assert_eq!(answered.entries[&ClubId::SevenIron].recorded_at, tick(0));
        assert!(answered.retired.is_empty());
        assert_eq!(answered.updated_at, tick(0), "the loaded bag, unwritten");
        assert_eq!(fs::read_to_string(bags.path_for("aaron")).unwrap(), written);
    }

    #[test]
    fn a_different_club_retires_the_old_one_and_leaves_the_others_alone() {
        let (_dir, bags) = store();
        bags.set_entry("aaron", &made(ClubId::Driver, "Callaway"), tick(0))
            .unwrap();
        bags.set_entry(
            "aaron",
            &entry(ClubId::SevenIron, |s| s.shaft_model = "Modus 105".into()),
            tick(1),
        )
        .unwrap();
        let updated = bags
            .set_entry(
                "aaron",
                &entry(ClubId::SevenIron, |s| s.shaft_model = "Project X LZ".into()),
                tick(2),
            )
            .unwrap();

        assert_eq!(
            updated.entries[&ClubId::SevenIron].spec.shaft_model,
            "Project X LZ"
        );
        let shelved = updated.retired_for(ClubId::SevenIron);
        assert_eq!(shelved.len(), 1);
        assert_eq!(shelved[0].spec.shaft_model, "Modus 105");
        assert_eq!(shelved[0].retired_at, Some(tick(2)));
        assert_eq!(shelved[0].recorded_at, tick(1));
        assert_eq!(updated.entries[&ClubId::Driver].spec.make, "Callaway");
        assert_eq!(updated.club_ids(), [ClubId::Driver, ClubId::SevenIron]);
    }

    #[test]
    fn removing_shelves_the_club_and_an_empty_slot_writes_nothing() {
        let (_dir, bags) = store();
        bags.set_entry("aaron", &made(ClubId::ThreeWood, "TaylorMade"), tick(0))
            .unwrap();
        let updated = bags
            .remove_entry("aaron", ClubId::ThreeWood, tick(1))
            .unwrap()
            .unwrap();
        assert!(updated.club_ids().is_empty());
        assert_eq!(
            makes(&updated.retired_for(ClubId::ThreeWood)),
            ["TaylorMade"]
        );
        assert_eq!(updated.retired[0].retired_at, Some(tick(1)));

        let written = fs::read_to_string(bags.path_for("aaron")).unwrap();
        assert!(bags
            .remove_entry("aaron", ClubId::ThreeWood, tick(2))
            .unwrap()
            .is_none());
        assert!(bags
            .remove_entry("nobody", ClubId::Driver, tick(2))
            .unwrap()
            .is_none());
        assert_eq!(fs::read_to_string(bags.path_for("aaron")).unwrap(), written);
    }

    /// The append-only pin: out and back is two stints, a copy comes off the shelf, and the
    /// restored club is stamped afresh.
    #[test]
    fn restoring_copies_the_previous_club_off_the_shelf() {
        let (_dir, bags) = store();
        bags.set_entry(
            "aaron",
            &entry(ClubId::SevenIron, |s| {
                s.make = "Titleist".into();
                s.loft_deg = Some(34.0);
            }),
            tick(0),
        )
        .unwrap();
        bags.set_entry("aaron", &made(ClubId::SevenIron, "Ping"), tick(1))
            .unwrap();

        let restored = bags
            .restore_entry("aaron", ClubId::SevenIron, tick(2))
            .unwrap()
            .unwrap();
        let current = &restored.entries[&ClubId::SevenIron];
        assert_eq!(
            (current.spec.make.as_str(), current.spec.loft_deg),
            ("Titleist", Some(34.0))
        );
        assert_eq!((current.recorded_at, current.retired_at), (tick(2), None));
        assert_eq!(
            makes(&restored.retired_for(ClubId::SevenIron)),
            ["Titleist", "Ping"]
        );

        assert!(bags
            .restore_entry("aaron", ClubId::Driver, tick(3))
            .unwrap()
            .is_none());
        assert!(bags
            .restore_entry("nobody", ClubId::SevenIron, tick(3))
            .unwrap()
            .is_none());
    }

    #[test]
    fn a_removed_club_can_be_restored() {
        let (_dir, bags) = store();
        bags.set_entry(
            "aaron",
            &entry(ClubId::ThreeWood, |s| s.loft_deg = Some(15.0)),
            tick(0),
        )
        .unwrap();
        bags.remove_entry("aaron", ClubId::ThreeWood, tick(1))
            .unwrap();
        let restored = bags
            .restore_entry("aaron", ClubId::ThreeWood, tick(2))
            .unwrap()
            .unwrap();
        assert_eq!(
            restored.entries[&ClubId::ThreeWood].spec.loft_deg,
            Some(15.0)
        );
        assert_eq!(restored.club_ids(), [ClubId::ThreeWood]);
    }

    /// All three mutators share one reader, and a fourth that reached for `get` instead would
    /// reintroduce exactly the loss this prevents.
    #[test]
    fn a_corrupt_bag_is_never_written_over() {
        let (_dir, bags) = store();
        bags.set_entry("aaron", &made(ClubId::SevenIron, "Titleist"), tick(0))
            .unwrap();
        fs::write(bags.path_for("aaron"), "{not json").unwrap();
        let refused = |result: Result<_, StoreError>| match result {
            Err(StoreError::Refused(message)) => {
                assert!(message.ends_with("exists but cannot be read; refusing to overwrite it"));
                assert!(message.contains(&bags.path_for("aaron").display().to_string()));
            }
            other => panic!("{other:?}"),
        };
        refused(
            bags.set_entry("aaron", &made(ClubId::Driver, ""), tick(1))
                .map(|_| ()),
        );
        refused(
            bags.remove_entry("aaron", ClubId::SevenIron, tick(1))
                .map(|_| ()),
        );
        refused(
            bags.restore_entry("aaron", ClubId::SevenIron, tick(1))
                .map(|_| ()),
        );
        assert_eq!(
            fs::read_to_string(bags.path_for("aaron")).unwrap(),
            "{not json"
        );
    }

    #[test]
    fn a_player_id_that_is_not_a_slug_never_becomes_a_filename() {
        let (_dir, bags) = store();
        for id in ["../aaron", "Dave Smith"] {
            match bags.set_entry(id, &made(ClubId::Driver, ""), tick(0)) {
                Err(StoreError::Invalid(error)) => assert!(
                    error.problem.contains("is not a slug"),
                    "{id}: {}",
                    error.problem
                ),
                other => panic!("{id}: {other:?}"),
            }
        }
        assert!(!bags.root().exists());
        assert!(!bags
            .root()
            .parent()
            .unwrap()
            .join("aaron.bag.json")
            .exists());
    }

    /// `save` refuses a bag its contract refuses, where Python's could not have been handed one.
    #[test]
    fn save_refuses_a_bag_the_contract_refuses_and_writes_nothing() {
        let (_dir, bags) = store();
        let mut entries = BTreeMap::new();
        entries.insert(ClubId::SevenIron, made(ClubId::PitchingWedge, ""));
        let filed_wrong = Bag {
            player_id: "aaron".into(),
            entries,
            retired: Vec::new(),
            updated_at: at(STALE),
        };
        assert!(matches!(
            bags.save(&filed_wrong),
            Err(StoreError::Invalid(_))
        ));
        assert!(!bags.path_for("aaron").exists());
    }
}
