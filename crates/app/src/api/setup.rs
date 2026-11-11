//! What first-run setup needs: who is swinging, what is in their bag, and the app's own settings.
//! [M38 P5]
//!
//! Thin calls to `GolferStore` and `BagStore`, the files the lab reads, so an export needs no
//! translating and each store's rules stand without a second copy here: a name's slug, a known
//! golfer's handedness winning, and a club leaving the bag being shelved and not lost. A refusal
//! reaches Dart as the store's or the contract's own words, the one exception being a loft that is
//! not a number at all, which neither of them asks about ([`set_bag_slot`] says why).
//!
//! **A bag here is slots and an optional loft** (D7): the picker lists these clubs and a flight borrows
//! the loft. No shaft and no catalogue, so every other field of the entry stays blank, which
//! `contracts::club_spec` says means nobody has said.
//!
//! **The enums are declared here rather than taken from `contracts`**, because the codegen binds only
//! what `app::api` holds (C1). `crate::state` holds the conversions both ways, each an exhaustive
//! `match`, so a variant added to either side stops the build until someone says what it means.

use std::path::Path;

use contracts::bag::{Bag, BagEntry};
use contracts::club::ClubId;
use contracts::club_spec::ClubSpec;
use contracts::golfer::Golfer;
use serde::{Deserialize, Serialize};
use storage::bag_store::BagStore;
use storage::golfer_store::GolferStore;

use crate::{club_id, parse_now, state, Dirs};

/// Which side a golfer swings from, as `contracts::golfer::Handedness` spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Handedness {
    Right,
    Left,
}

/// What a shot is practised as, as `contracts::intent::PracticeMode` spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PracticeMode {
    Fundamentals,
    ShotShaping,
    Performance,
    Drill,
}

/// A declared ball flight, as `contracts::intent::TargetShape` spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetShape {
    Straight,
    Draw,
    Fade,
}

/// The app's look (D13), in the order the setting offers them. The app's own, with no contract
/// behind it: it decides nothing about a shot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Appearance {
    Dark,
    System,
    Light,
}

/// The settings `.app/state.json` holds (C7). What the phone stamps on a shot lives here and not in
/// a `session.json`, which is what stops changing clubs conflicting at the lab (C8).
///
/// `club` is `ClubId`'s spelling and `last_export_at` RFC 3339 with an offset (C2), and [`set_state`]
/// refuses either otherwise. Whether `mode` and `target_shape` agree is not asked here, since a change
/// sheet passes through `shot_shaping` with no shape yet: C15's rule is for a shot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppState {
    /// `None` until [`init_state`], the one writer of it. Names every session this phone makes.
    pub install_id: Option<String>,
    pub player_id: Option<String>,
    pub club: Option<String>,
    pub mode: PracticeMode,
    pub target_shape: Option<TargetShape>,
    pub challenge: bool,
    pub challenge_shapes: Vec<TargetShape>,
    pub appearance: Appearance,
    pub last_export_at: Option<String>,
    pub last_export_shots: Option<i64>,
}

/// One golfer, as the setup and Settings screens list them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GolferView {
    pub player_id: String,
    pub display_name: String,
    pub handedness: Handedness,
}

/// One club in a bag.
#[derive(Debug, Clone, PartialEq)]
pub struct BagSlot {
    /// `ClubId`'s spelling.
    pub club: String,
    /// `None` is a loft nobody has said, never 0 (ADR-010 §2).
    pub loft_deg: Option<f64>,
    /// What the screen shows, since Dart formats no value (rule 8). `None` with `loft_deg`.
    pub loft_display: Option<String>,
}

fn golfers_at(root: &str) -> GolferStore {
    GolferStore::new(Dirs::of(root).golfers)
}

/// `GolferStore` and `BagStore` share a directory by design: the suffix keeps them apart.
fn bags_at(root: &str) -> BagStore {
    BagStore::new(Dirs::of(root).golfers)
}

fn golfer_view(golfer: Golfer) -> GolferView {
    GolferView {
        player_id: golfer.player_id,
        display_name: golfer.display_name,
        handedness: golfer.handedness.into(),
    }
}

fn slots_of(bag: &Bag) -> Vec<BagSlot> {
    bag.club_ids()
        .into_iter()
        .map(|club| {
            let loft_deg = bag.entries[&club].spec.loft_deg;
            BagSlot {
                club: club.as_str().to_string(),
                loft_deg,
                loft_display: loft_deg.map(|loft| format!("{loft}°")),
            }
        })
        .collect()
}

/// The first run's call, and every launch's: stores `install_id` if the phone has none, and answers
/// the settings. An id already stored stands, so Dart may mint a fresh one on every launch.
pub fn init_state(root: String, install_id: String) -> Result<AppState, String> {
    state::init(Path::new(&root), &install_id)
}

/// The settings, or the defaults if there are none or they cannot be read.
pub fn get_state(root: String) -> AppState {
    state::read(Path::new(&root))
}

/// Store the settings, and answer what was stored. The stored `install_id` stands whatever `state`
/// says, since [`init_state`] is its one writer.
pub fn set_state(root: String, state: AppState) -> Result<AppState, String> {
    if let Some(club) = &state.club {
        club_id(club)?;
    }
    if let Some(at) = &state.last_export_at {
        parse_now(at)?;
    }
    state::replace(Path::new(&root), state)
}

/// Every known golfer, by display name.
pub fn list_golfers(root: String) -> Vec<GolferView> {
    golfers_at(&root)
        .list_all()
        .into_iter()
        .map(golfer_view)
        .collect()
}

/// The golfer this name folds to, created only if new. **A known golfer's handedness wins over the
/// one given**, and the answer says which it is, so a screen shows what was stored.
pub fn create_golfer(
    root: String,
    name: String,
    handedness: Handedness,
    now: String,
) -> Result<GolferView, String> {
    let now = parse_now(&now)?;
    golfers_at(&root)
        .get_or_create(&name, handedness.into(), now)
        .map(golfer_view)
        .map_err(|refused| refused.to_string())
}

/// Every club a bag can hold, in bag order and `ClubId`'s spelling.
pub fn all_clubs() -> Vec<String> {
    ClubId::ALL
        .into_iter()
        .map(|club| club.as_str().to_string())
        .collect()
}

/// A golfer's bag in bag order, empty if none is declared or it cannot be read.
pub fn bag_slots(root: String, player_id: String) -> Vec<BagSlot> {
    bags_at(&root)
        .get(&player_id)
        .map(|bag| slots_of(&bag))
        .unwrap_or_default()
}

/// Refuse a loft JSON cannot hold. Stored, it would say `NaN°` to the screen and `null` in the file,
/// and declaring it again would shelve a phantom, since the file reread holds no loft and the new entry
/// holds one.
fn refuse_a_loft_that_is_not_finite(loft_deg: Option<f64>) -> Result<(), String> {
    match loft_deg {
        Some(loft) if !loft.is_finite() => Err(format!(
            "a loft has to be a real number of degrees, and {loft} is not"
        )),
        _ => Ok(()),
    }
}

/// Declare a slot and its loft if known, and answer the bag. Changing a loft is a different club as
/// far as the store can see, so the old value goes to the shelf, and the same loft again writes
/// nothing.
///
/// **Finite is the only thing asked of a loft, which is the user's call on the M38 plan's P5**: `340`,
/// `-5` and `3.4` all stand. No contract bounds a loft, here or in frozen Python, and a bound there
/// would change what every reader accepts. A range here would be the only one anywhere, and it would
/// miss what it was for: the flight only asks if a loft is below 13°, so `340` flies as any iron does,
/// and `3.4`, a typo for 34, leaves the spin unscored and flies no wrong number.
pub fn set_bag_slot(
    root: String,
    player_id: String,
    club: String,
    loft_deg: Option<f64>,
    now: String,
) -> Result<Vec<BagSlot>, String> {
    let club = club_id(&club)?;
    let now = parse_now(&now)?;
    refuse_a_loft_that_is_not_finite(loft_deg)?;
    let entry = BagEntry {
        spec: ClubSpec {
            loft_deg,
            ..ClubSpec::blank(club)
        },
        // The store stamps it, discarding this, as its doc says.
        recorded_at: now,
        retired_at: None,
        provenance: None,
    };
    bags_at(&root)
        .set_entry(&player_id, &entry, now)
        .map(|bag| slots_of(&bag))
        .map_err(|refused| refused.to_string())
}

/// Take a club out of the bag, and answer the bag. The club is shelved rather than lost, and an empty
/// slot is no refusal: it is already what was asked for.
pub fn remove_bag_slot(
    root: String,
    player_id: String,
    club: String,
    now: String,
) -> Result<Vec<BagSlot>, String> {
    let club = club_id(&club)?;
    let now = parse_now(&now)?;
    let bags = bags_at(&root);
    match bags.remove_entry(&player_id, club, now) {
        Ok(Some(bag)) => Ok(slots_of(&bag)),
        Ok(None) => Ok(bag_slots(root, player_id)),
        Err(refused) => Err(refused.to_string()),
    }
}
