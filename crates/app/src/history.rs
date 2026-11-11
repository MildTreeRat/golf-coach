//! How the stores become what History and a shot's detail screen show. [M38 P9]
//!
//! Outside `api` because the codegen binds what is `pub` there (rule 7). `api::history` holds the shapes,
//! and this adds no reading rule of its own: the entries are the bundle store's, the shot is the shot
//! store's, the analysis is `load_shot_analysis`'s, and what counts is `read_corpus`'s (C23).
//!
//! # A row walks the store, not the corpus
//!
//! The corpus holds what counts and names the rest, and a drill or a duplicate is not in its `swings`.
//! So a list built from it would lose exactly the shots a person comes here to ask about. Every entry
//! gets a row from its manifest and its shot, and the corpus only answers two questions about each:
//! why it is left out, and whether it is held out as a mishit.
//!
//! **An entry that has lost a file still lists** (P6 keeps an entry whose analysis failed, because the
//! read was good). A row needs only the manifest, so a missing shot is `None`s and a missing analysis
//! is the corpus's `not_analyzed`, and neither is an `Err`.
//!
//! # The shot is the shot store's, and not the analysis's copy
//!
//! The store is what `correct_shot` and `analyse_entry` work from, and is keyed by the photo. The copy
//! in `shot_analysis.json` is what was analysed, and may be missing. They agree after every write here
//! (P7), so one source is one less thing to disagree.
//!
//! # Whose sentence is whose
//!
//! - **`unscored`'s are the contract's**, `ReasonSpec::remedy`, verbatim. `contracts::unscored` holds one
//!   table so that "the prose cannot drift between the tip, the brief, the MCP view and the results page",
//!   and this is one more page.
//! - **`left_out`'s are this module's, by reason, with one exception.** The corpus's own sentences are
//!   the lab's: a drill's says `clear its intent with golf-core set-intent`, and a missing analysis says
//!   `run golf-core run-shot over its session`. Neither verb exists on a phone, the drill's is on every
//!   drill shot, and `spec/` holds those words (rule 4), so they stay as they are for the lab. The
//!   match is exhaustive, so a new reason stops the build until someone says what it means here. A
//!   duplicate's is the corpus's own, because only that says which entry was kept, and it names no verb.
//! - **`notes` are `analyse_entry`'s, with one clause changed.** A loft note ends `then re-run golf-core
//!   run-shot over its session`, and it is common: 2 of M33's six captures carry one under a club with
//!   no loft on record (provisional: pasted copies), since Full Swing's printed spin is `shown_only`
//!   and the flight solves its own. So the clause, and only it, is replaced as it is shown, through the
//!   constant the note is built from. The stored file keeps the lab's words for the lab.
//!
//! The repair these sentences offer is real: `correct_shot` runs `analyse_entry` even with no edits, and
//! P10's `set_shot_club` and `set_shot_intent` re-analyse too.
//!
//! # One moment
//!
//! Both reads hold `crate::record::one_writer()`. `ShotStore::put` writes in place, so a read landing
//! inside one finds a torn file, and an answer built from it would show a shot's values as not read.
//! History refreshes on entering its tab and a library batch runs photo by photo (C14), so nothing
//! stops the two overlapping, and the worst a wait costs is one photo's record. **So a writer must not
//! call either read while it holds that lock**: `std`'s `Mutex` does not re-enter, and it would hang.
//!
//! # The four writers [M38 P10]
//!
//! D10 and C15, each over the store's own repair, and each answering the detail screen as it now stands
//! but `delete_shot`, which leaves nothing to show. Each holds that lock for its read, change and write
//! and lets go before it reads the answer back.
//!
//! - **The club and the intent are read by the analysis, so each is analysed before it is stored.**
//!   `analyse_entry` takes the manifest it is handed, so it can be handed the changed one, and a refused
//!   analysis then leaves nothing moved, `updated_at` included. The other order needs a way back, and
//!   the store has none to a manifest naming no club. A manifest that then cannot be written runs the
//!   analysis again under the one still stored, so the two go on agreeing.
//! - **What a retag refuses is `golf_core::session::entry_to_change`'s**, shared with `correct_shot`. A
//!   shot the store does not hold cannot be analysed, so it is refused and not half done, and
//!   `delete_shot` is what is left for such an entry.
//! - **C15's refusal is `crate::record::goal_for`'s, flags and all**: `--mode shot_shaping needs --shape`
//!   behind a sentence for a person. No screen should offer either pair (P25), so this is what a bug
//!   would show, and a second copy of the rule in better words is what would drift.
//! - **A verdict is read by no analysis**, only by the corpus off the manifest, so nothing is re-run.
//! - **Saying what is already said moves no byte**, stamp included, as `correct_shot`'s empty save moves
//!   none and for its reason: a re-import at the lab would read the entry as differing. The analysis runs
//!   anyway, and a run over the same inputs writes the same bytes, so a retag is also what repairs an entry
//!   whose analysis was lost.
//! - **`delete_shot` takes the shot's record only when no entry still holds the photo.** The store is
//!   keyed by the photo and `Duplicates` is per golfer, so one photo under two golfers in two sessions
//!   is two entries over one record (P6 finding 7), and taking it would leave the other golfer's entry
//!   with no values. The walk is over every session and golfer, and after the entry has gone, so
//!   whichever goes last takes the record with it. Entry first, because a record left behind costs
//!   nothing, the next read of that photo writes over it, and an entry left with no record is a row
//!   of nothing.
//!   **The boxes and the read's line stay**: they are what happened at the bay (C7), a repeat of the
//!   photo takes the newer boxes (`crate::record::keep_boxes`), and P14 reads them.
//! - **`delete_shot` spends the entry's number before it removes anything**, so no later photo opens at
//!   a path the lab may already hold (`crate::record`'s module doc, the user's decision of 2026-11-05).
//!   If that cannot be written, nothing is removed.
//!
//! ## Limits, recorded and not fixed
//!
//! - **A change made after the lab imported the shot does not reach the lab**, nor does a delete, which
//!   is C16's limit on a correction: the lab reads the entry as a photo it holds, and keeps its own.
//! - **A surviving entry's shot can go on naming the entry deleted.** The record's ids are whichever entry
//!   wrote it last, as they already are for two golfers (P6 finding 7). Nothing opens a shot by them, and
//!   each surface asks by the manifest's hash.

use std::cmp::Reverse;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;

use contracts::career::{ExcludedSwing, ExclusionReason};
use contracts::mishit::MishitVerdict as ContractVerdict;
use contracts::shot::ShotData;
use contracts::Timestamp;
use golf_core::session::{
    analyse_entry, check_segments, entry_to_change, session_holding, EntryToChange, Outcome,
    ShotDirs, RERUN_SHOT,
};
use screen::profile::ProfileField;
use storage::bundle_store::SwingBundleStore;
use storage::corpus::read_corpus;
use storage::manifest::{Role, SwingManifest};
use storage::shot_store::ShotStore;
use storage::state::load_shot_analysis;

use crate::api::history::{
    MeasurementRow, MishitVerdict, SessionView, ShotDetail, ShotRow, UnscoredRow,
};
use crate::api::record::RowState;
use crate::api::setup::{PracticeMode, TargetShape};
use crate::card::{card_of, display_of, tile_for};
use crate::record::{goal_for, one_writer, remember_deleted, remove_entry};
use crate::{club_id, engine_versions, parse_now, Dirs};

/// Said when the shot store has nothing for an entry. `correct_shot` and a repeat of the photo both
/// refuse such an entry, so nothing on the phone restores it and this offers no repair.
const NO_SHOT: &str = "No shot is stored for this photo, so there are no values to show.";

/// The corpus's `not_analyzed`, and what a detail screen adds for a drill, which the corpus decides
/// before it looks for an analysis.
const NOT_ANALYSED: &str =
    "No analysis is stored for this shot, so no club or player number counts \
                            it. Saving a review of it analyses it again.";

/// What a phone can do where a loft note names the lab's verb (the module doc).
const SAVE_AGAIN: &str = "then save a review of this shot";

/// `read_corpus`'s `excluded`, in words that are true on a phone (the module doc).
fn left_out_sentence(excluded: &ExcludedSwing) -> String {
    let sentence = match excluded.reason {
        ExclusionReason::Unattributed => "No golfer is named on this shot, so nobody's numbers count it.",
        ExclusionReason::NoFaceOn => "This entry holds no photo, so there is nothing to count.",
        ExclusionReason::Duplicate => return format!("Not counted here: {}.", excluded.detail),
        ExclusionReason::NotAnalyzed => NOT_ANALYSED,
        ExclusionReason::Stale => {
            "Its stored analysis names another photo, so no club or player number counts it. Saving \
             a review of it analyses it again."
        }
        ExclusionReason::Outdated => {
            "An older build analysed it, and those numbers do not pool with today's, so no club or \
             player number counts it. Saving a review of it analyses it again."
        }
        ExclusionReason::Untracked => {
            "Hit as a drill, so no club or player number counts it. Change what it was practised as \
             if it should count."
        }
    };
    sentence.to_string()
}

impl From<ContractVerdict> for MishitVerdict {
    fn from(verdict: ContractVerdict) -> Self {
        match verdict {
            ContractVerdict::Confirmed => MishitVerdict::Confirmed,
            ContractVerdict::Cleared => MishitVerdict::Cleared,
        }
    }
}

impl From<MishitVerdict> for ContractVerdict {
    fn from(verdict: MishitVerdict) -> Self {
        match verdict {
            MishitVerdict::Confirmed => ContractVerdict::Confirmed,
            MishitVerdict::Cleared => ContractVerdict::Cleared,
        }
    }
}

/// An entry's name, as the corpus's two lists name one.
type Entry = (String, String);

fn entry_of(manifest: &SwingManifest) -> Entry {
    (manifest.session_id.clone(), manifest.swing_id.clone())
}

/// What the corpus says about each of one golfer's entries.
struct CorpusAnswers {
    /// `CorpusSwing::is_mishit`, for the entries it has a swing for. The automatic half is stamped
    /// only there, because it needs every other carry the club has.
    mishits: HashMap<Entry, bool>,
    left_out: HashMap<Entry, Vec<String>>,
}

impl CorpusAnswers {
    fn read(dirs: &Dirs, player_id: &str) -> Self {
        let corpus = read_corpus(&dirs.sessions, player_id, engine_versions());
        let mishits = corpus
            .swings
            .iter()
            .map(|swing| {
                let entry = (swing.session_id.clone(), swing.swing_id.clone());
                (entry, swing.is_mishit())
            })
            .collect();
        let mut left_out: HashMap<Entry, Vec<String>> = HashMap::new();
        for excluded in &corpus.excluded {
            left_out
                .entry((excluded.session_id.clone(), excluded.swing_id.clone()))
                .or_default()
                .push(left_out_sentence(excluded));
        }
        CorpusAnswers { mishits, left_out }
    }

    /// An entry with no swing in the corpus has only a person's verdict to go on.
    fn is_mishit(&self, manifest: &SwingManifest) -> bool {
        self.mishits
            .get(&entry_of(manifest))
            .copied()
            .unwrap_or(manifest.mishit == Some(ContractVerdict::Confirmed))
    }

    fn left_out(&self, manifest: &SwingManifest) -> Vec<String> {
        self.left_out
            .get(&entry_of(manifest))
            .cloned()
            .unwrap_or_default()
    }
}

/// The two tiles a row shows.
struct RowTiles {
    carry: &'static ProfileField,
    ball_speed: &'static ProfileField,
}

impl RowTiles {
    fn of_device() -> Result<Self, String> {
        Ok(RowTiles {
            carry: tile_for("carry_distance")?,
            ball_speed: tile_for("ball_speed")?,
        })
    }
}

/// `list_sessions`' body: newest session first, by id, since C8's ids sort by date and two phones on
/// one date have no order but their names.
pub(crate) fn list(root: &str, player_id: &str) -> Result<Vec<SessionView>, String> {
    let _one_moment = one_writer();
    let dirs = Dirs::of(root);
    let tiles = RowTiles::of_device()?;
    let corpus = CorpusAnswers::read(&dirs, player_id);
    let store = SwingBundleStore::new(&dirs.sessions);

    let mut sessions = Vec::new();
    for session_id in store.list_session_ids().into_iter().rev() {
        // Walked backwards first and sorted stably, so photos taken at one instant, which a library
        // batch can hold, read newest entry first like everything else here.
        let mut rows: Vec<(Timestamp, ShotRow)> = store
            .get_session(&session_id)
            .iter()
            .rev()
            .filter(|manifest| manifest.player_id.as_deref() == Some(player_id))
            .map(|manifest| {
                let shot = shot_of(&dirs, manifest).ok().flatten();
                row_of(manifest, shot.as_ref(), &dirs, &tiles, &corpus)
            })
            .collect();
        if rows.is_empty() {
            continue;
        }
        rows.sort_by_key(|(taken, _)| Reverse(*taken));
        sessions.push(SessionView {
            date: date_of(&session_id),
            count: i64::try_from(rows.len()).unwrap_or(i64::MAX),
            rows: rows.into_iter().map(|(_, row)| row).collect(),
            session_id,
        });
    }
    Ok(sessions)
}

/// `shot_detail`'s body. The golfer is the manifest's, which is who the corpus counts it for.
pub(crate) fn detail(root: &str, session_id: &str, swing_id: &str) -> Result<ShotDetail, String> {
    let _one_moment = one_writer();
    let dirs = Dirs::of(root);
    let manifest = SwingBundleStore::new(&dirs.sessions)
        .get_swing(session_id, swing_id)
        .ok_or_else(|| format!("{session_id}/{swing_id} is not on this phone"))?;
    let tiles = RowTiles::of_device()?;
    // A manifest naming nobody is read under no golfer's name, which is how it is named `unattributed`.
    let corpus = CorpusAnswers::read(&dirs, manifest.player_id.as_deref().unwrap_or_default());

    let held = shot_of(&dirs, &manifest);
    let shot = held.as_ref().ok().and_then(Option::as_ref);
    let (_, mut row) = row_of(&manifest, shot, &dirs, &tiles, &corpus);

    let card = match &held {
        Ok(Some(shot)) => card_of(shot, session_id, swing_id),
        Ok(None) => Err(NO_SHOT.to_string()),
        Err(unreadable) => Err(unreadable.clone()),
    };
    let (card, no_card) = match card {
        Ok(card) => (Some(card), None),
        Err(why) => (None, Some(why)),
    };

    let Some(analysis) = load_shot_analysis(&swing_dir(&dirs, &manifest)) else {
        // The corpus says so itself unless it stopped before looking (the shape's doc).
        if !row.left_out.iter().any(|sentence| sentence == NOT_ANALYSED) {
            row.left_out.push(NOT_ANALYSED.to_string());
        }
        return Ok(ShotDetail {
            row,
            card,
            no_card,
            measurements: Vec::new(),
            unscored: Vec::new(),
            notes: Vec::new(),
        });
    };

    let corrected = |name: &str| {
        card.iter()
            .flat_map(|card| &card.rows)
            .any(|row| row.field == name && row.state == RowState::Corrected)
    };
    let unscored = analysis
        .unscored
        .iter()
        .filter(|entry| !corrected(&entry.name))
        .map(|entry| UnscoredRow {
            name: entry.name.clone(),
            label: tile_for(&entry.name).map_or_else(|_| entry.name.clone(), |t| t.label.clone()),
            reason: entry.spec().remedy.to_string(),
        })
        .collect();
    let measurements = analysis
        .measurements
        .into_iter()
        .map(|measurement| MeasurementRow {
            display: pyfmt::g(measurement.value),
            name: measurement.name,
            unit: measurement.unit,
            detail: measurement.detail,
        })
        .collect();
    Ok(ShotDetail {
        row,
        card,
        no_card,
        measurements,
        unscored,
        notes: analysis
            .notes
            .iter()
            .map(|note| note.replace(RERUN_SHOT, SAVE_AGAIN))
            .collect(),
    })
}

/// `set_shot_club`'s body.
pub(crate) fn set_club(
    root: &str,
    session_id: &str,
    swing_id: &str,
    club: &str,
    now: &str,
) -> Result<ShotDetail, String> {
    let now = parse_now(now)?;
    let club = club_id(club)?;
    retag(
        root,
        session_id,
        swing_id,
        |manifest| manifest.club = Some(club),
        |store| store.set_club(session_id, swing_id, club, now),
    )
}

/// `set_shot_intent`'s body. An entry with no goal reads as the default one, so saying that moves
/// nothing. The ids are asked first, since `goal_for` would say an empty session is a missing flag.
pub(crate) fn set_intent(
    root: &str,
    session_id: &str,
    swing_id: &str,
    mode: PracticeMode,
    target_shape: Option<TargetShape>,
    now: &str,
) -> Result<ShotDetail, String> {
    let now = parse_now(now)?;
    check_segments(session_id, swing_id)?;
    let goal = goal_for(session_id, mode, target_shape)?;
    retag(
        root,
        session_id,
        swing_id,
        |manifest| {
            if manifest.intent.clone().unwrap_or_default() != goal {
                manifest.intent = Some(goal.clone());
            }
        },
        |store| store.set_intent(session_id, swing_id, Some(goal.clone()), now),
    )
}

/// The club's and the intent's one shape (the module doc): the analysis under the changed manifest, then
/// the store's own repair, which is `write`. `change` leaving the manifest as it was skips the repair.
fn retag(
    root: &str,
    session_id: &str,
    swing_id: &str,
    change: impl FnOnce(&mut SwingManifest),
    write: impl FnOnce(&SwingBundleStore) -> io::Result<Option<SwingManifest>>,
) -> Result<ShotDetail, String> {
    {
        let _one_at_a_time = one_writer();
        let dirs = Dirs::of(root);
        let reference = format!("{session_id}/{swing_id}");
        let shot_dirs = ShotDirs {
            sessions: dirs.sessions.clone(),
            golfers: dirs.golfers.clone(),
            shots: dirs.shots.clone(),
        };
        let EntryToChange {
            manifest: stored,
            mut duplicates,
            ..
        } = entry_to_change(&shot_dirs, session_id, swing_id, engine_versions())?;
        let mut changed = stored.clone();
        change(&mut changed);

        let failure = match analyse_entry(&changed, &shot_dirs, &mut duplicates) {
            Outcome::Written(_) => None,
            Outcome::Failed(why) => Some(why),
            Outcome::Swing | Outcome::NoPhoto | Outcome::NoParse(_) | Outcome::Duplicate(_) => {
                Some("it is not an entry with a shot to analyse".to_string())
            }
        };
        if let Some(why) = failure {
            return Err(format!("{reference} could not be analysed: {why}"));
        }

        if changed != stored {
            let written = write(&SwingBundleStore::new(&dirs.sessions)).and_then(|written| {
                written.ok_or_else(|| io::Error::other("its manifest could not be read"))
            });
            if let Err(failed) = written {
                let _ = analyse_entry(&stored, &shot_dirs, &mut duplicates);
                return Err(format!("could not write {reference}: {failed}"));
            }
        }
    }
    detail(root, session_id, swing_id)
}

/// `set_shot_mishit`'s body.
pub(crate) fn set_mishit(
    root: &str,
    session_id: &str,
    swing_id: &str,
    verdict: Option<MishitVerdict>,
    now: &str,
) -> Result<ShotDetail, String> {
    let now = parse_now(now)?;
    check_segments(session_id, swing_id)?;
    let reference = format!("{session_id}/{swing_id}");
    let verdict = verdict.map(ContractVerdict::from);
    {
        let _one_at_a_time = one_writer();
        let store = SwingBundleStore::new(Dirs::of(root).sessions);
        let stored = store
            .get_swing(session_id, swing_id)
            .ok_or_else(|| format!("{reference} is not on this phone"))?;
        if stored.mishit != verdict {
            store
                .set_mishit(session_id, swing_id, verdict, now)
                .map_err(|failed| format!("could not write {reference}: {failed}"))?;
        }
    }
    detail(root, session_id, swing_id)
}

/// `delete_shot`'s body (the module doc). The ids are asked first because `delete_swing` removes what it
/// is pointed at, and the manifest because a directory no store can see is no entry a screen offered.
pub(crate) fn delete(root: &str, session_id: &str, swing_id: &str) -> Result<(), String> {
    check_segments(session_id, swing_id)?;
    let reference = format!("{session_id}/{swing_id}");
    let _one_at_a_time = one_writer();
    let dirs = Dirs::of(root);
    let store = SwingBundleStore::new(&dirs.sessions);
    let stored = store
        .get_swing(session_id, swing_id)
        .ok_or_else(|| format!("{reference} is not on this phone"))?;
    remember_deleted(&dirs, session_id, swing_id)
        .map_err(|failed| format!("could not delete {reference}: {failed}"))?;
    remove_entry(&store, session_id, swing_id)
        .map_err(|failed| format!("could not delete {reference}: {failed}"))?;

    let Some(photo) = stored.roles.get(&Role::ShotScreen) else {
        return Ok(());
    };
    let digest = &photo.content_sha256;
    let still_held = store
        .list_session_ids()
        .iter()
        .any(|session| session_holding(&store, session, digest).is_some());
    if still_held {
        return Ok(());
    }
    match fs::remove_file(ShotStore::new(&dirs.shots).path_for(digest)) {
        Ok(()) => Ok(()),
        Err(missing) if missing.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(failed) => Err(format!(
            "{reference} is deleted, and the shot read from it could not be: {failed}"
        )),
    }
}

fn swing_dir(dirs: &Dirs, manifest: &SwingManifest) -> PathBuf {
    dirs.sessions
        .join(&manifest.session_id)
        .join(&manifest.swing_id)
}

/// The shot the store holds for this entry's photo. `Ok(None)` is nothing held, an entry with no photo
/// included, and `Err` is a file that cannot be read as a shot, which a row shows as no shot.
fn shot_of(dirs: &Dirs, manifest: &SwingManifest) -> Result<Option<ShotData>, String> {
    let Some(photo) = manifest.roles.get(&Role::ShotScreen) else {
        return Ok(None);
    };
    ShotStore::new(&dirs.shots)
        .get(&photo.content_sha256)
        .map_err(|unreadable| unreadable.to_string())
}

/// One entry's row, and the time it sorts by.
fn row_of(
    manifest: &SwingManifest,
    shot: Option<&ShotData>,
    dirs: &Dirs,
    tiles: &RowTiles,
    corpus: &CorpusAnswers,
) -> (Timestamp, ShotRow) {
    let taken = shot.map_or(manifest.created_at, |shot| shot.timestamp);
    let provenance = shot.and_then(|shot| shot.provenance.as_ref());
    let goal = manifest.intent.as_ref();
    let display = |tile, value: Option<f64>| value.map(|value| display_of(value, tile));
    let row = ShotRow {
        session_id: manifest.session_id.clone(),
        swing_id: manifest.swing_id.clone(),
        time: taken.to_string(),
        time_display: clock_of(taken),
        club: manifest.club.map(|club| club.as_str().to_string()),
        mode: goal.map_or(PracticeMode::Fundamentals, |goal| goal.mode.into()),
        target_shape: goal.and_then(|goal| goal.target_shape).map(Into::into),
        carry_label: tiles.carry.label.clone(),
        carry_display: display(tiles.carry, shot.and_then(|shot| shot.carry_distance)),
        ball_speed_label: tiles.ball_speed.label.clone(),
        ball_speed_display: display(tiles.ball_speed, shot.and_then(|shot| shot.ball_speed)),
        flagged: provenance.is_some_and(|provenance| provenance.needs_review),
        corrected: provenance.is_some_and(|provenance| !provenance.corrections.is_empty()),
        mishit: corpus.is_mishit(manifest),
        mishit_verdict: manifest.mishit.map(Into::into),
        untracked: goal.is_some_and(|goal| !goal.mode.is_tracked()),
        photo_path: manifest.roles.get(&Role::ShotScreen).map(|photo| {
            swing_dir(dirs, manifest)
                .join(&photo.filename)
                .display()
                .to_string()
        }),
        left_out: corpus.left_out(manifest),
    };
    (taken, row)
}

/// `HH:MM` as written in the time's own offset, which `isoformat` keeps and an instant does not.
pub(crate) fn clock_of(at: Timestamp) -> String {
    at.isoformat().get(11..16).unwrap_or_default().to_string()
}

/// The date C8 starts every session's id with. An id too short to hold one stands for itself, so a
/// session the phone did not name still has something to show.
fn date_of(session_id: &str) -> String {
    session_id.get(..10).unwrap_or(session_id).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every reason walked, the unreachable ones too: what a person reads here has to be something
    /// they can do on the phone in their hand. The corpus's own words are what fail this (the module
    /// doc), which `tests/history.rs` shows on a real drill.
    #[test]
    fn no_reason_names_a_verb_or_a_file_a_phone_does_not_have() {
        for reason in ExclusionReason::ALL {
            let sentence = left_out_sentence(&ExcludedSwing {
                session_id: "2026-11-04-phone-a1b2".to_string(),
                swing_id: "2".to_string(),
                reason,
                detail: "shot photo abc is already in 2026-11-03-phone-a1b2/1 — the same shot \
                         uploaded again, not a second shot"
                    .to_string(),
            });
            for word in ["golf-core", "golf-lab", "scripts/", ".json", "pipeline"] {
                assert!(!sentence.contains(word), "{reason:?}: {sentence}");
            }
            assert!(sentence.ends_with('.'), "{reason:?}: {sentence}");
        }
    }

    /// An instant has no clock of its own: 01:42 UTC read 20:42 where the photo was taken.
    #[test]
    fn a_clock_is_the_one_the_photo_was_taken_under() {
        let at = |text| Timestamp::parse(text).expect("a time");
        assert_eq!(clock_of(at("2026-11-04T20:42:00-05:00")), "20:42");
        assert_eq!(clock_of(at("2026-11-05T01:42:00Z")), "01:42");
        assert_eq!(clock_of(at("2026-11-04T20:42:59.123456+05:30")), "20:42");
    }

    #[test]
    fn a_sessions_date_is_what_c8_starts_its_id_with() {
        assert_eq!(date_of("2026-11-04-phone-a1b2"), "2026-11-04");
        assert_eq!(date_of("2026-11-04"), "2026-11-04");
        assert_eq!(date_of("mine"), "mine");
    }
}
