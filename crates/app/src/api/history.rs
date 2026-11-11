//! What the History tab and a shot's detail screen show, as plain data [M38 P9], and what a person can
//! do to a shot from there [M38 P10].
//!
//! The shapes Dart sees and six one-line calls (rule 7). `crate::history` holds what they read and in
//! what order, whose words each sentence is, and what each writer moves. **A row is every entry, drills
//! and refused ones included**, because History is where a person finds a shot the numbers do not count
//! and asks why.

use crate::api::record::ShotCard;
use crate::api::setup::{PracticeMode, TargetShape};

/// A person's own verdict on a shot, as `contracts::mishit::MishitVerdict` spells it. No verdict leaves
/// it to the automatic rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MishitVerdict {
    /// Yes, a mishit: hold it out of the distance averages.
    Confirmed,
    /// No, a real shot: count it, whatever the automatic rule thinks.
    Cleared,
}

/// One session's shots, for one golfer.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionView {
    pub session_id: String,
    /// The day the photos were taken, in their own offset (C8), `2026-11-04`.
    pub date: String,
    /// How many rows. Every entry of this golfer's in the session, whether or not it counts.
    pub count: i64,
    /// Newest first, by when each photo was taken.
    pub rows: Vec<ShotRow>,
}

/// One entry, as a line in History and at the head of its detail screen.
#[derive(Debug, Clone, PartialEq)]
pub struct ShotRow {
    pub session_id: String,
    pub swing_id: String,
    /// When the photo was taken, RFC 3339 in its own offset (C9), or when the entry arrived if the
    /// shot store holds no shot for it.
    pub time: String,
    /// `time`'s clock, `20:42`, as it read where the photo was taken. Here because a `DateTime` parsed
    /// in Dart drops the offset, so showing it there would need arithmetic to get this back (rule 8).
    pub time_display: String,
    /// `ClubId`'s own spelling, `7i`. Every entry this phone opens has one.
    pub club: Option<String>,
    /// What it was practised as, off the manifest, which is the intent of record.
    pub mode: PracticeMode,
    pub target_shape: Option<TargetShape>,
    /// The tile's own label, `Carry Yd`, which names the unit `carry_display` is in.
    pub carry_label: String,
    /// As a card shows it. `None` was not read, or was cleared by a person, and is never 0.
    pub carry_display: Option<String>,
    /// The tile's own label, `Ball Speed Mph`.
    pub ball_speed_label: String,
    /// As a card shows it. `None` was not read, or was cleared by a person, and is never 0.
    pub ball_speed_display: Option<String>,
    /// The stored `needs_review`: a read a person has not looked at yet.
    pub flagged: bool,
    /// A person has changed at least one value on this shot.
    pub corrected: bool,
    /// Held out of the distance averages, by a person's verdict or the automatic rule.
    pub mishit: bool,
    /// The person's own verdict, if any. `None` with `mishit` true is the automatic rule's doing, and
    /// this is what a detail screen's control starts on.
    pub mishit_verdict: Option<MishitVerdict>,
    /// Hit as a drill, so it counts in no club's or player's numbers. Derived, never stored.
    pub untracked: bool,
    /// The photo's absolute path, found through the manifest (C17). `None` if the entry holds none.
    pub photo_path: Option<String>,
    /// Why the corpus leaves this out, one sentence per reason and in the corpus's order, since a stale
    /// entry can be outdated too. Empty is an entry that counts.
    pub left_out: Vec<String>,
}

/// Everything a shot's detail screen shows.
#[derive(Debug, Clone, PartialEq)]
pub struct ShotDetail {
    /// The list's row, so the photo, the club, the intent and the mishit controls have one source.
    /// Its `left_out` also says so if no analysis is stored, which on a drill the corpus never asks.
    pub row: ShotRow,
    /// The stored shot's card, the one a read answered. `None` says why in `no_card`.
    pub card: Option<ShotCard>,
    /// Why there is no card: the shot store holds no shot for this photo, or none that can be shown.
    pub no_card: Option<String>,
    /// As stored: the shot's rows then the flight's. A number nothing could be read for has no row
    /// here, so it can never read as 0, and is in `unscored` or on the card as not read.
    pub measurements: Vec<MeasurementRow>,
    /// What could not be produced, in stored order. A tile a person cleared is not here: its card row
    /// says `Corrected`, and the stored sentence would blame the photo for what a person chose.
    pub unscored: Vec<UnscoredRow>,
    /// As `analyse_entry` wrote them, but for a loft note's repair, which says what a phone can do.
    pub notes: Vec<String>,
}

/// One number measured from the shot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasurementRow {
    /// The contract's name, `flight_carry_yds`. No label table exists for these, and none is added here.
    pub name: String,
    /// `pyfmt::g` of the stored value, as a card's numbers are.
    pub display: String,
    /// The contract's spelling: `yards`, `mph`, `degrees`, `seconds`, `rpm`.
    pub unit: String,
    /// How it was taken, and any sign convention: `SIMULATED …`, or `… not where it finished`. A number
    /// with a name like these needs this beside it, which is why the contract keeps it on the row.
    pub detail: String,
}

/// One thing that could not be produced, and what to tell the golfer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnscoredRow {
    /// The contract's name: a `ShotData` key for a tile, `sidespin`, or a flight row's.
    pub name: String,
    /// The tile's own label, `Sidespin Rpm`, or `name` where it has no tile.
    pub label: String,
    /// `contracts::unscored`'s golfer-facing sentence for the reason, verbatim.
    pub reason: String,
}

/// One golfer's sessions, newest first, with a golfer who has none answering an empty list. `Err` is
/// only a build whose device table has lost a tile.
pub fn list_sessions(root: String, player_id: String) -> Result<Vec<SessionView>, String> {
    crate::history::list(&root, &player_id)
}

/// One shot's detail. `Err` is an entry that is not on this phone, and everything an entry can have
/// lost still answers, saying what is missing.
pub fn shot_detail(
    root: String,
    session_id: String,
    swing_id: String,
) -> Result<ShotDetail, String> {
    crate::history::detail(&root, &session_id, &swing_id)
}

/// Say which club hit this shot, and fly it again with that club's loft. The repair every session needs
/// when someone forgets to change clubs. `club` is `ClubId`'s own spelling, `7i`.
///
/// Answers the screen as it now stands. `Err` is a sentence to show, with nothing written: a club or a
/// time that does not read, an entry that is not here, one whose shot the store does not hold, or one
/// that is not what its photo counts under.
pub fn set_shot_club(
    root: String,
    session_id: String,
    swing_id: String,
    club: String,
    now: String,
) -> Result<ShotDetail, String> {
    crate::history::set_club(&root, &session_id, &swing_id, &club, &now)
}

/// Say what this shot was practised as, and analyse it again under that (C15). A shape goes with
/// `ShotShaping` alone, and `ShotShaping` needs one. A drill is still listed, and counts in no number.
///
/// Answers the screen as it now stands. `Err` is as [`set_shot_club`]'s, and those two rules too.
pub fn set_shot_intent(
    root: String,
    session_id: String,
    swing_id: String,
    mode: PracticeMode,
    target_shape: Option<TargetShape>,
    now: String,
) -> Result<ShotDetail, String> {
    crate::history::set_intent(&root, &session_id, &swing_id, mode, target_shape, &now)
}

/// Record a person's verdict on this shot (D10), or with `None` leave it to the automatic rule again.
///
/// Answers the screen as it now stands. `Err` is an entry that is not here or a time that does not read,
/// with nothing written.
pub fn set_shot_mishit(
    root: String,
    session_id: String,
    swing_id: String,
    verdict: Option<MishitVerdict>,
    now: String,
) -> Result<ShotDetail, String> {
    crate::history::set_mishit(&root, &session_id, &swing_id, verdict, &now)
}

/// Remove a stray photo's entry and the shot read from it (D10), so the photo can be recorded afresh.
/// **Not undone by anything**: the photo in the entry goes too, and only the phone's Photos library, if
/// it came from there, still has one. `Err` is an entry that is not here, with nothing removed.
pub fn delete_shot(root: String, session_id: String, swing_id: String) -> Result<(), String> {
    crate::history::delete(&root, &session_id, &swing_id)
}
