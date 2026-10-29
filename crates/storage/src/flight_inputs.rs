//! The loft a stored shot borrows from the bag of the golfer who hit it, or which of four repairs
//! would give it one. `storage/flight_inputs.py`. [M35 P8, the join at M29 P28]
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
//! # Two routes to one rule
//!
//! [`loft_for_club`] is the half a caller holding a manifest needs (`golf-core run-shot --session`,
//! the lab's bundle pipeline). [`read_flight_inputs`] is the corpus-wide join, ported at M29 P28
//! for `golf-lab flight --shots`, `simulate_flight.py`'s twin: every stored shot filed under the
//! swing holding its photo, earliest arrival first, and through that swing to a club, a loft and a
//! handedness. **The join key is the shot photo's sha256**, which both sides already record, so
//! nothing is re-hashed. Both routes run the same [`resolve_loft`], so there is one definition of a
//! declared loft and one of what its absence means.
//!
//! `golf-mcp`'s `simulate_flight` reads one swing's loft through [`loft_for_club`] and never
//! needed the join (the M29 plan's P13 finding 7).

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use contracts::club::ClubId;
use contracts::golfer::Handedness;
use contracts::shot::ShotData;
use serde::{Deserialize, Serialize};

use crate::bag_store::BagStore;
use crate::bundle_store::SwingBundleStore;
use crate::golfer_store::GolferStore;
use crate::manifest::{Role, SwingManifest};

/// Why a shot has no loft: four different repairs wearing one `None`. `LoftGap`, by wire name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoftGap {
    /// No swing on disk carries this shot's photo, so there is no club tag to read: a screenshot
    /// imported on its own. Only [`read_flight_inputs`] produces it; [`loft_for_club`]'s caller is
    /// holding the swing.
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

/// The four-way branch over data already in hand, with no I/O, so both routes share it.
/// `_resolve_loft`, branch for branch and in Python's order: the club tag is asked first, so an
/// untagged swing is `NoClubTag` even with nobody to own a bag.
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

/// One stored shot with whatever the swing, the bag and the golfer register add to it.
/// `FlightInputs`. [M29 P28]
#[derive(Debug, Clone, PartialEq)]
pub struct FlightInputs {
    pub shot: ShotData,
    /// `"session/swing"`, or `None` when no swing on disk holds this photo.
    pub swing_ref: Option<String>,
    pub player_id: Option<String>,
    pub club: Option<ClubId>,
    /// Python's `(loft_deg, loft_gap)` pair as one value, as [`loft_for_club`] returns it: book loft
    /// off the bag entry, ready for `analysis::flight_infer::flight_for_shot`, or the gap.
    pub loft: Result<f64, LoftGap>,
    pub handedness: Option<Handedness>,
    /// Other swings holding the same photo, in the arrival order the survivor was chosen by. Three
    /// swing directories on disk share one 2026-08-10 screenshot, and a join that silently picked
    /// one would be choosing a club tag by luck.
    pub also_attached_to: Vec<String>,
}

impl FlightInputs {
    /// `loft_deg`: the declared loft, `None` exactly when [`Self::loft_gap`] is not.
    pub fn loft_deg(&self) -> Option<f64> {
        self.loft.ok()
    }

    /// `loft_gap`.
    pub fn loft_gap(&self) -> Option<LoftGap> {
        self.loft.err()
    }
}

/// Join each shot to its swing, and through it to a loft and a handedness. `read_flight_inputs`.
/// [M29 P28]
///
/// **Order is the caller's**: `ShotStore::all` is newest first and this keeps it, because the only
/// thing a shot sorts by here is its own timestamp. **The shots are an argument** for Python's
/// reason in a looser form: the shot store is `crate::shot_store`, which this module could reach,
/// but a caller that already holds the shots (or a test that builds them) should not be made to
/// write them to disk first.
///
/// Missing directories are an empty join rather than an error: a shot with no swing beside it is a
/// real state and [`LoftGap::NoSwing`] is its answer. **Earliest arrival wins**, which is the
/// corpus's rule for the same situation: a re-upload's timestamp dates the upload and not the
/// swing. One bag read per golfer rather than one per shot, as Python caches it.
pub fn read_flight_inputs(
    shots: impl IntoIterator<Item = ShotData>,
    sessions_dir: &Path,
    golfers_dir: &Path,
) -> Vec<FlightInputs> {
    let by_photo = swings_by_shot_photo(&SwingBundleStore::new(sessions_dir));
    let golfers = GolferStore::new(golfers_dir);
    let bags = BagStore::new(golfers_dir);
    let mut lofts: HashMap<String, BTreeMap<ClubId, Option<f64>>> = HashMap::new();

    let mut resolved = Vec::new();
    for shot in shots {
        // Python's `digest or ""`: a shot with no provenance, or an empty hash, looks up the empty
        // key, which no manifest written by the bundle store carries.
        let digest = shot
            .provenance
            .as_ref()
            .and_then(|provenance| provenance.image_sha256.as_deref())
            .unwrap_or("");
        let Some(attached) = by_photo.get(digest).filter(|swings| !swings.is_empty()) else {
            resolved.push(FlightInputs {
                shot,
                swing_ref: None,
                player_id: None,
                club: None,
                loft: Err(LoftGap::NoSwing),
                handedness: None,
                also_attached_to: Vec::new(),
            });
            continue;
        };

        let (survivor, others) = (&attached[0], &attached[1..]);
        let player_id = survivor.player_id.clone();
        let club = survivor.club;
        // `golfers.get(player_id) if player_id else None`: an empty id is no golfer.
        let golfer = player_id
            .as_deref()
            .filter(|id| !id.is_empty())
            .and_then(|id| golfers.get(id));
        let entries = player_id.as_ref().map(|id| {
            &*lofts
                .entry(id.clone())
                .or_insert_with(|| declared_lofts(&bags, id))
        });
        let loft = resolve_loft(club, player_id.as_deref(), entries);

        resolved.push(FlightInputs {
            shot,
            swing_ref: Some(swing_ref(survivor)),
            player_id,
            club,
            loft,
            handedness: golfer.map(|golfer| golfer.handedness),
            also_attached_to: others.iter().map(swing_ref).collect(),
        });
    }
    resolved
}

/// Every swing on disk, filed under the sha256 of the shot photo it arrived with, each list in
/// arrival order: `(created_at, session_id, swing_id)`, the instant first. `_swings_by_shot_photo`.
///
/// Built once rather than per shot, since it parses every manifest in every session. The sort is on
/// the instant, as Python's `datetime` comparison is, and stable, as `sorted` is.
fn swings_by_shot_photo(store: &SwingBundleStore) -> HashMap<String, Vec<SwingManifest>> {
    let mut by_photo: HashMap<String, Vec<SwingManifest>> = HashMap::new();
    for session_id in store.list_session_ids() {
        for manifest in store.get_session(&session_id) {
            if let Some(photo) = manifest.roles.get(&Role::ShotScreen) {
                by_photo
                    .entry(photo.content_sha256.clone())
                    .or_default()
                    .push(manifest);
            }
        }
    }
    for members in by_photo.values_mut() {
        members.sort_by(|a, b| {
            (a.created_at, &a.session_id, &a.swing_id).cmp(&(
                b.created_at,
                &b.session_id,
                &b.swing_id,
            ))
        });
    }
    by_photo
}

/// `"session/swing"`, the reference every CLI in this repo prints and takes back. `_ref`.
fn swing_ref(manifest: &SwingManifest) -> String {
    format!("{}/{}", manifest.session_id, manifest.swing_id)
}

#[cfg(test)]
mod tests {
    //! `tests/storage/test_flight_inputs.py`: the join's nine cases, the three `loft_for_club`
    //! cases, and the edges of that route they leave to the reader: no registry, an empty id, a bag
    //! that cannot be read.

    use super::*;
    use contracts::bag::BagEntry;
    use contracts::club_spec::ClubSpec;
    use contracts::Timestamp;
    use serde_json::json;
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

    // ----------------------------------------------------------------------- the join itself

    /// The join's fixtures: an empty sessions root, and a registry holding one right-handed
    /// golfer and no bag yet, as `corpus_dir` and `golfers_dir` are.
    fn corpus() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let sessions = dir.path().join("sessions");
        let golfers = dir.path().join("golfers");
        GolferStore::new(&golfers)
            .get_or_create("Aaron", Handedness::Right, when())
            .unwrap();
        (dir, sessions, golfers)
    }

    /// `write_swing` with the keys the join reads: a manifest holding a shot photo by sha.
    fn swing(
        sessions: &Path,
        at: (&str, &str),
        player_id: Option<&str>,
        club: Option<ClubId>,
        photo: &str,
        created_at: &str,
    ) {
        let (session, swing) = at;
        let dir = sessions.join(session).join(swing);
        fs::create_dir_all(&dir).unwrap();
        let manifest = json!({
            "swing_id": swing,
            "session_id": session,
            "created_at": created_at,
            "updated_at": created_at,
            "roles": {"shot_screen": {
                "role": "shot_screen",
                "filename": "shot_screen.jpg",
                "content_sha256": photo,
                "original_filename": "IMG_0001.jpg",
                "content_type": "image/jpeg",
                "size_bytes": 7,
                "received_at": created_at,
                "warnings": [],
            }},
            "player_id": player_id,
            "club": club,
        });
        fs::write(
            dir.join(crate::manifest::MANIFEST_NAME),
            manifest.to_string(),
        )
        .unwrap();
    }

    const NOON_23: &str = "2026-08-23T12:00:00Z";

    /// `_shot`: a parsed shot keyed to a screen photo the way the shot store files one.
    fn shot(shot_id: &str, photo: Option<&str>) -> ShotData {
        let provenance = photo.map(
            |photo| json!({"device": "hd_golf", "parse_confidence": 0.9, "image_sha256": photo}),
        );
        serde_json::from_value(json!({
            "shot_id": shot_id,
            "session_id": "2026-08-23",
            "timestamp": "2026-08-23T03:37:00Z",
            "source": "screen",
            "ball_speed": 89.8,
            "launch_angle": 22.4,
            "carry_distance": 126.1,
            "provenance": provenance,
        }))
        .unwrap()
    }

    fn join_one(shot: ShotData, sessions: &Path, golfers: &Path) -> FlightInputs {
        read_flight_inputs([shot], sessions, golfers).remove(0)
    }

    /// `test_a_shot_reaches_its_club_and_loft_through_the_swing_it_arrived_with`.
    #[test]
    fn a_shot_reaches_its_club_and_loft_through_the_swing_it_arrived_with() {
        let (_dir, sessions, golfers) = corpus();
        let at = ("2026-08-23", "4");
        swing(
            &sessions,
            at,
            Some("aaron"),
            Some(ClubId::SevenIron),
            "photo-a",
            NOON_23,
        );
        declare(&golfers, ClubId::SevenIron, Some(30.5));
        let row = join_one(shot("2026-08-23-4", Some("photo-a")), &sessions, &golfers);
        assert_eq!(row.swing_ref.as_deref(), Some("2026-08-23/4"));
        assert_eq!(row.club, Some(ClubId::SevenIron));
        assert_eq!(row.loft_deg(), Some(30.5));
        assert_eq!(row.loft_gap(), None);
        assert_eq!(row.handedness, Some(Handedness::Right));
    }

    /// `test_a_tagged_club_with_no_bag_entry_names_the_bag_page_and_not_the_bay`, and
    /// `test_an_entry_with_no_declared_loft_is_a_third_state_again`, and
    /// `test_an_untagged_swing_is_the_gap_the_club_cursor_closes`: three gaps, three repairs.
    #[test]
    fn each_gap_the_join_can_name_is_named() {
        let (_dir, sessions, golfers) = corpus();
        let player = Some("aaron");
        swing(
            &sessions,
            ("2026-08-23", "6"),
            player,
            Some(ClubId::ThreeWood),
            "b",
            NOON_23,
        );
        swing(
            &sessions,
            ("2026-08-23", "1"),
            player,
            Some(ClubId::FiveIron),
            "c",
            NOON_23,
        );
        swing(&sessions, ("2026-08-10", "1"), player, None, "d", NOON_23);
        declare(&golfers, ClubId::FiveIron, None);
        let rows = read_flight_inputs(
            [
                shot("2026-08-23-6", Some("b")),
                shot("2026-08-23-1", Some("c")),
                shot("2026-08-10-1", Some("d")),
            ],
            &sessions,
            &golfers,
        );
        assert_eq!(rows[0].club, Some(ClubId::ThreeWood));
        assert_eq!(rows[0].loft, Err(LoftGap::NoBagEntry));
        assert_eq!(rows[1].club, Some(ClubId::FiveIron));
        assert_eq!(rows[1].loft, Err(LoftGap::NoDeclaredLoft));
        assert_eq!(rows[2].swing_ref.as_deref(), Some("2026-08-10/1"));
        assert_eq!(rows[2].club, None);
        assert_eq!(rows[2].loft, Err(LoftGap::NoClubTag));
    }

    /// `test_a_shot_no_swing_carries_is_a_state_and_not_an_error`: a photo nothing holds, and a
    /// shot with no provenance at all.
    #[test]
    fn a_shot_no_swing_carries_is_a_state_and_not_an_error() {
        let (_dir, sessions, golfers) = corpus();
        let rows = read_flight_inputs(
            [shot("loose-1", Some("photo-none")), shot("loose-2", None)],
            &sessions,
            &golfers,
        );
        for row in &rows {
            assert_eq!(row.loft, Err(LoftGap::NoSwing));
            assert_eq!(row.swing_ref, None);
            assert_eq!(row.handedness, None);
            assert!(row.also_attached_to.is_empty());
        }
    }

    /// `test_the_earliest_arrival_is_read_and_the_re_uploads_are_named`: three directories share
    /// one photo, and the one that arrived first is read, whatever its directory sorts as.
    #[test]
    fn the_earliest_arrival_is_read_and_the_re_uploads_are_named() {
        let (_dir, sessions, golfers) = corpus();
        let player = Some("aaron");
        let (nine, seven) = (Some(ClubId::NineIron), Some(ClubId::SevenIron));
        swing(
            &sessions,
            ("2026-08-10", "1"),
            player,
            nine,
            "e",
            "2026-08-10T12:00:00Z",
        );
        swing(
            &sessions,
            ("2026-08-07", "1"),
            player,
            seven,
            "e",
            "2026-08-07T12:00:00Z",
        );
        swing(
            &sessions,
            ("2026-08-09", "2"),
            player,
            None,
            "e",
            "2026-08-09T12:00:00Z",
        );
        declare(&golfers, ClubId::SevenIron, Some(30.5));
        declare(&golfers, ClubId::NineIron, Some(41.0));
        let row = join_one(shot("2026-08-10-1", Some("e")), &sessions, &golfers);
        assert_eq!(row.swing_ref.as_deref(), Some("2026-08-07/1"));
        assert_eq!(row.loft_deg(), Some(30.5));
        assert_eq!(row.also_attached_to, ["2026-08-09/2", "2026-08-10/1"]);
    }

    /// Arrival is an instant, not a spelling: 08:00-05:00 is 13:00Z and arrives after 12:00Z, and
    /// at one instant the session id and then the swing id decide, as Python's tuple does.
    #[test]
    fn arrival_is_the_instant_and_then_the_ids() {
        let (_dir, sessions, golfers) = corpus();
        let player = Some("aaron");
        swing(
            &sessions,
            ("b", "1"),
            player,
            None,
            "f",
            "2026-08-10T08:00:00-05:00",
        );
        swing(
            &sessions,
            ("a", "1"),
            player,
            None,
            "f",
            "2026-08-10T13:00:00Z",
        );
        swing(
            &sessions,
            ("c", "1"),
            player,
            None,
            "f",
            "2026-08-10T12:00:00Z",
        );
        let row = join_one(shot("x", Some("f")), &sessions, &golfers);
        assert_eq!(row.swing_ref.as_deref(), Some("c/1"));
        assert_eq!(row.also_attached_to, ["a/1", "b/1"]);
    }

    /// `test_a_club_with_no_golfer_to_own_a_bag_reports_the_repair_it_actually_needs`.
    #[test]
    fn a_club_with_no_golfer_to_own_a_bag_reports_the_repair_it_actually_needs() {
        let (_dir, sessions, golfers) = corpus();
        let at = ("2026-08-23", "9");
        swing(
            &sessions,
            at,
            None,
            Some(ClubId::ThreeWood),
            "photo-f",
            NOON_23,
        );
        let row = join_one(shot("2026-08-23-9", Some("photo-f")), &sessions, &golfers);
        assert_eq!(row.player_id, None);
        assert_eq!(row.handedness, None);
        assert_eq!(row.loft, Err(LoftGap::NoBagEntry));
    }

    /// `test_missing_directories_are_an_empty_join_rather_than_a_traceback`.
    #[test]
    fn missing_directories_are_an_empty_join_rather_than_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let row = join_one(
            shot("2026-08-23-4", Some("shot-hash")),
            &dir.path().join("nothing-here"),
            &dir.path().join("nor-here"),
        );
        assert_eq!(row.loft, Err(LoftGap::NoSwing));
    }

    /// `test_the_callers_order_survives_because_it_is_the_shots_own`.
    #[test]
    fn the_callers_order_survives_because_it_is_the_shots_own() {
        let (_dir, sessions, golfers) = corpus();
        let at = ("2026-08-23", "4");
        swing(
            &sessions,
            at,
            Some("aaron"),
            Some(ClubId::SevenIron),
            "photo-a",
            NOON_23,
        );
        let ids = ["c", "a", "b"];
        let rows = read_flight_inputs(ids.map(|id| shot(id, Some("photo-a"))), &sessions, &golfers);
        let got: Vec<&str> = rows.iter().map(|row| row.shot.shot_id.as_str()).collect();
        assert_eq!(got, ids);
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
