//! One photo and the boxes Swift read from it, to a stored, analysed shot and the card showing it, and
//! the diagnostics line the bay report reads. [M38 P6]
//!
//! This adds no reading, storing or scoring: the entry is `golf_core::session`'s, the read is
//! `screen::read`'s, the shot store's key is the photo's hash, and the analysis is `analyse_entry`'s, as
//! the lab does it (C23). What is the phone's is the order they run in and the answer Dart gets.
//!
//! # What is stamped, and from where
//!
//! **The golfer, the club and the intent come from `state.json`** (C8) and not from Dart's arguments, so
//! a model out of step with what the change sheet saved cannot put a wrong club on a shot, and a
//! refusal here names what is missing. A challenge's call is the one exception (C15): the draw lives in
//! Dart, and passing it leaves the mode the challenge will restore where it is. Neither time is the
//! clock's: `captured_at` is Swift's (C9), and it stamps the shot, names the session and sets the
//! stored photo's modification time. `now` stamps the manifest, which is what the corpus means by
//! arrival.
//!
//! # A read of nothing keeps nothing but the diagnostics (C13)
//!
//! `screen::read` answering `None` takes the entry back out and moves the photo to `.app/failed/`, so
//! no shot, no entry and no empty session is left for History to list. A failed read is not a duplicate
//! afterwards, so the retake of the same bytes reads afresh.
//!
//! # Two kinds of duplicate
//!
//! **The session holding the photo** is asked first, by `session_holding`, the lab's rule, and opens
//! nothing. **Another session holding it** is the corpus's question, by `Duplicates`, and can only be
//! asked of an open entry. It is reachable: a photo with no EXIF date takes its file's date or the clock
//! (C9), so a pasted copy imported twice can name two sessions. So it is asked between the read and
//! `ShotStore::put`, because the store is keyed by the photo's hash and a `put` here would rewrite the
//! held shot's id to name an entry about to be dropped. Both answer the held shot.
//!
//! # One at a time
//!
//! `next_swing_id` is taken outside the store's lock, and that lock is per store instance, so two
//! overlapping calls could take one swing id and the second would replace the first's photo (P3
//! finding 4). Nothing serializes bridge calls, so [`record`] holds a lock of its own for all of
//! it. `RecordFlow` awaiting each photo keeps it uncontended, and a stray second tap costs a wait.
//!
//! # A number is never given out twice (the user's decision, 2026-11-05, on P10's question)
//!
//! `next_swing_id` is one past the highest entry there is, so deleting a session's newest entry would
//! free its number, and export is cumulative (C16). If the lab had imported the deleted entry, the next
//! photo would be other bytes at a path the lab holds, and that refuses the whole import, this one and
//! every later one, until someone removes the lab's copy by hand. So `crate::history::delete` writes
//! the number to `.app/deleted.json` **before** it removes anything, and each entry opens above
//! what that file holds for its session. It holds the highest for each, which is all a floor needs.
//!
//! - **Before, because a crash between the two costs one unused number**, and the other order would
//!   cost the conflict this exists to prevent.
//! - **`discard` writes nothing.** The entry it takes back was opened by this same call, under this
//!   same lock, so no export can have seen it, and a retake keeps the number the failed read had.
//! - **A session's directory going does not lose its floor**, which is why the numbers are not kept in
//!   it, and why they are in `.app/` at all: `sessions/` is all the lab's to read.
//! - **It stays home**, as `state.json` does (C7): `crate::export` walks C16's named paths, and the lab
//!   has no use for it. So a number the phone has spent is still not in any export.
//! - **An unreadable file reads as no floors**, the rule `crate::state`'s doc gives for a settings file:
//!   a torn file should not cost a shot at the bay. It is written by rename, so only a failing disk
//!   tears it, and what that risks is a named refusal at the lab, nothing lost. A delete is the
//!   opposite: if the number cannot be written, nothing is removed and the delete says so.
//!
//! # Two logs, one appender
//!
//! `.app/reads.jsonl` is one line per photo, and since P30b `.app/events.jsonl` is one line per thing
//! the person did or saw go wrong (D16). Both go through [`append_line`], and both travel in the export
//! (`crate::export`). Neither judges: the bay's bars read the reads, and the events are for a person.
//!
//! Outside `api` because rule 7 keeps that module to what crosses to Dart, as `crate::state` is for
//! setup: `api::record` holds the shapes and three one-line calls, and this holds what they do.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use contracts::club::ClubId;
use contracts::intent::PracticeGoal;
use contracts::shot::ShotData;
use contracts::Timestamp;
use golf_core::session::{
    analyse_entry, intent_action, open_photo_entry, session_holding, Duplicates, NewPhotoEntry,
    Outcome, ShotDirs,
};
use screen::{ScreenInput, TextBox};
use serde::{Deserialize, Serialize};
use storage::bundle_store::SwingBundleStore;
use storage::golfer_store::GolferStore;
use storage::manifest::{Role, SwingManifest};
use storage::shot_store::{hash_image, ShotStore};

use crate::api::record::{AppEvent, PhotoIn, ReadLog, RecordOutcome, RecordStatus, ShotCard};
use crate::api::setup::{PracticeMode, TargetShape};
use crate::card::card_of;
use crate::{club_id, engine_versions, parse_now, state, Dirs, DEVICE, MIN_CONFIDENCE};

/// C13's two sentences, here because Dart holds layout and no words about a read (rule 8). The first
/// borrows the camera overlay's own instruction (C10), which is what a retake can change.
const FAILED: &str = "This photo could not be read. Retake it, filling the frame with the tiles.";
const ALREADY_RECORDED: &str = "This photo is already recorded, so this is the shot it holds.";

/// The highest number deleted from each session, under [`Dirs::app`] (the module doc).
const DELETED_NAME: &str = "deleted.json";

/// The reads' log under [`Dirs::app`], which the bay's bars read, and which the export carries.
pub(crate) const READS_NAME: &str = "reads.jsonl";

/// The events' log under [`Dirs::app`] (D16), which the export carries beside the reads. [M38 P30b]
pub(crate) const EVENTS_NAME: &str = "events.jsonl";

/// One [`record`] at a time (the module doc), and since P7 one `correct_shot`, whose read, change and
/// write of a shot's file another call must not fall between. A panic elsewhere holding it left nothing
/// half-made in memory, so the lock is still good.
static RECORDING: Mutex<()> = Mutex::new(());

/// Hold [`RECORDING`] for as long as the answer is kept.
pub(crate) fn one_writer() -> MutexGuard<'static, ()> {
    RECORDING.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `record_photo`'s body, top to bottom in the order the module doc gives.
pub(crate) fn record(photo: PhotoIn) -> Result<RecordOutcome, String> {
    let started = Instant::now();
    let _one_at_a_time = one_writer();

    let captured_at = parse_now(&photo.captured_at)?;
    let now = parse_now(&photo.now)?;
    let dirs = Dirs::of(&photo.root);
    let stamp = stamp_for(&photo, &dirs, captured_at)?;
    let path = PathBuf::from(&photo.path);
    let bytes = fs::read(&path)
        .map_err(|failed| format!("could not read photo {}: {failed}", photo.path))?;
    let digest = hash_image(&bytes);

    keep_boxes(&dirs, &digest, &photo.boxes_json)
        .map_err(|failed| format!("could not store the boxes of {}: {failed}", photo.path))?;
    let boxes = parse_boxes(&photo.boxes_json)?;
    let box_count = i64::try_from(boxes.len()).unwrap_or(i64::MAX);
    let answer = |status, card, message: Option<&str>| RecordOutcome {
        status,
        card,
        message: message.map(str::to_string),
        photo_sha256: digest.clone(),
        boxes: box_count,
        ms_rust: i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX),
    };

    let store = SwingBundleStore::new(&dirs.sessions);
    if let Some(held) = session_holding(&store, &stamp.session_id, &digest) {
        let card = held_card(&dirs, &held, &digest)?;
        return Ok(answer(
            RecordStatus::Duplicate,
            Some(card),
            Some(ALREADY_RECORDED),
        ));
    }

    let opened = open_photo_entry(
        &store,
        &NewPhotoEntry {
            session_id: &stamp.session_id,
            digest: &digest,
            bytes: &bytes,
            original_filename: path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("photo"),
            modified: system_time(captured_at),
            player_id: &stamp.player_id,
            club: stamp.club,
            intent: Some(&stamp.goal),
            floor: deleted_numbers(&dirs)
                .get(&stamp.session_id)
                .copied()
                .unwrap_or(0),
        },
        now,
    )
    .map_err(|failed| format!("could not open an entry for {}: {failed}", photo.path))?;

    let shot = match read_screen(boxes, &opened, &digest, captured_at) {
        Ok(Some(shot)) => shot,
        Ok(None) => {
            discard(&store, &opened)?;
            move_to_failed(&dirs, &path, &digest, &bytes)
                .map_err(|failed| format!("could not keep {}: {failed}", photo.path))?;
            return Ok(answer(RecordStatus::Failed, None, Some(FAILED)));
        }
        Err(refused) => {
            discard(&store, &opened)?;
            return Err(refused);
        }
    };

    let mut duplicates = Duplicates::new(&dirs.sessions, engine_versions());
    if duplicates.of(&opened).is_some() {
        discard(&store, &opened)?;
        let held = held_elsewhere(&store, &stamp.player_id, &digest).ok_or_else(|| {
            format!("the corpus says another entry holds {digest}, and none could be found")
        })?;
        let card = held_card(&dirs, &held, &digest)?;
        return Ok(answer(
            RecordStatus::Duplicate,
            Some(card),
            Some(ALREADY_RECORDED),
        ));
    }

    if let Err(failed) = ShotStore::new(&dirs.shots).put(&shot) {
        discard(&store, &opened)?;
        return Err(format!(
            "could not store the shot read from {}: {failed}",
            photo.path
        ));
    }

    // An entry whose analysis failed stays, with its shot stored: the read was good, History lists it
    // with its reason (P9), and the next feed of the photo answers the held shot.
    let shot_dirs = ShotDirs {
        sessions: dirs.sessions.clone(),
        golfers: dirs.golfers.clone(),
        shots: dirs.shots.clone(),
    };
    match analyse_entry(&opened, &shot_dirs, &mut duplicates) {
        Outcome::Written(_) => {}
        Outcome::Failed(why) => return Err(why),
        Outcome::Swing | Outcome::NoPhoto | Outcome::NoParse(_) | Outcome::Duplicate(_) => {
            return Err(format!(
                "{}/{} was opened with a photo and its shot stored, and could not be analysed",
                opened.session_id, opened.swing_id
            ));
        }
    }

    let card = card_of(&shot, &opened.session_id, &opened.swing_id)?;
    let status = if card.flagged {
        RecordStatus::Flagged
    } else {
        RecordStatus::Read
    };
    Ok(answer(status, Some(card), None))
}

/// `log_read`'s body.
pub(crate) fn append_read(root: &str, read: &ReadLog) -> Result<(), String> {
    append_line(root, READS_NAME, &read.at, read)
}

/// `log_event`'s body, through the same appender as the reads. [M38 P30b]
pub(crate) fn append_event(root: &str, event: &AppEvent) -> Result<(), String> {
    append_line(root, EVENTS_NAME, &event.at, event)
}

/// One JSON line appended to `name` under [`Dirs::app`], refused if `at` has no offset. One `write_all`
/// of the whole line on an append handle, so overlapping calls cannot interleave inside a line, and
/// `serde_json` escapes a newline inside a string, so a stack trace in an event's detail is still one
/// line.
fn append_line(root: &str, name: &str, at: &str, value: &impl Serialize) -> Result<(), String> {
    parse_now(at)?;
    let mut line = serde_json::to_string(value).map_err(|failed| failed.to_string())?;
    line.push('\n');
    let app = Dirs::of(root).app;
    fs::create_dir_all(&app)
        .and_then(|()| {
            fs::File::options()
                .create(true)
                .append(true)
                .open(app.join(name))
        })
        .and_then(|mut file| file.write_all(line.as_bytes()))
        .map_err(|failed| format!("could not append to {name}: {failed}"))
}

/// Everything a shot is stamped with that is not the photo's own.
struct Stamp {
    session_id: String,
    player_id: String,
    club: ClubId,
    goal: PracticeGoal,
}

/// What `state.json` says to stamp, or why it cannot, before anything is written.
///
/// **A session is never named without the install id** (C8): a default state file has none, and a
/// session named with a guess cannot be renamed. Nor is a golfer the registry does not hold taken on
/// trust, since `set_state` asks nothing of the id and it goes on every manifest.
fn stamp_for(photo: &PhotoIn, dirs: &Dirs, captured_at: Timestamp) -> Result<Stamp, String> {
    let state = state::read(Path::new(&photo.root));
    let install = state
        .install_id
        .ok_or("this phone has no install id yet, so no session can be named")?;
    let player_id = state
        .player_id
        .ok_or("no golfer is chosen, so this shot would be nobody's")?;
    if GolferStore::new(&dirs.golfers).get(&player_id).is_none() {
        return Err(format!("golfer '{player_id}' is not on this phone"));
    }
    let club = state
        .club
        .ok_or("no club is chosen, so this shot could not be flown or counted")?;
    let club = club_id(&club)?;
    let session_id = format!("{}-phone-{install}", captured_at.date_ymd());
    let (mode, shape) = match photo.challenge_call {
        Some(call) => (PracticeMode::ShotShaping, Some(call)),
        None => (state.mode, state.target_shape),
    };
    let goal = goal_for(&session_id, mode, shape)?;
    Ok(Stamp {
        session_id,
        player_id,
        club,
        goal,
    })
}

/// C15's rule, which is `intent_action`'s and has no second copy here: a shape only with
/// `shot_shaping`, and `shot_shaping` only with a shape. Its words name its flags, so they are said to
/// be about this shot. `state.json` can hold either, since a change sheet passes through both.
pub(crate) fn goal_for(
    session_id: &str,
    mode: PracticeMode,
    shape: Option<TargetShape>,
) -> Result<PracticeGoal, String> {
    let action = intent_action(
        Some(session_id.to_string()),
        None,
        true,
        Some(spelled(mode)),
        shape.map(spelled),
        false,
    )
    .map_err(|refused| {
        format!("the mode and shape chosen do not go together, so change them first: {refused}")
    })?;
    Ok(action.goal.unwrap_or_default())
}

/// A plain enum's serde spelling, which is the manifest's.
pub(crate) fn spelled(value: impl Serialize) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(name)) => name,
        other => panic!("a plain enum serializes as a string, not {other:?}"),
    }
}

/// The boxes as Swift returned them, one file per photo (C7), failed reads included.
///
/// **A photo the store already holds a shot for keeps the boxes it was read from**, since those are
/// what a later re-read needs (the plan's "Not in M38"), and a repeat of the same photo decides
/// nothing. A photo that failed, or that was deleted with its shot, takes the newer read.
fn keep_boxes(dirs: &Dirs, digest: &str, text: &str) -> io::Result<()> {
    let boxes = dirs.app.join("boxes");
    let path = boxes.join(format!("{digest}.json"));
    if ShotStore::new(&dirs.shots).has(digest) && path.exists() {
        return Ok(());
    }
    fs::create_dir_all(&boxes)?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, &path)
}

/// The reader's text as boxes, in either shape [`PhotoIn::boxes_json`] allows. Its first character picks
/// which, so a malformed list is refused for what is wrong with it rather than for not being an object.
pub(crate) fn parse_boxes(text: &str) -> Result<Vec<TextBox>, String> {
    #[derive(Deserialize)]
    struct Read {
        boxes: Vec<TextBox>,
    }
    let parsed = if text.trim_start().starts_with('[') {
        serde_json::from_str(text)
    } else {
        serde_json::from_str::<Read>(text).map(|read| read.boxes)
    };
    parsed.map_err(|refused| format!("the reader's answer is not a list of boxes: {refused}"))
}

/// `screen::read` for the entry just opened, with C8's shot id and C17's root-relative path.
fn read_screen(
    boxes: Vec<TextBox>,
    opened: &SwingManifest,
    digest: &str,
    captured_at: Timestamp,
) -> Result<Option<ShotData>, String> {
    let (session_id, swing_id) = (&opened.session_id, &opened.swing_id);
    let filename = opened
        .roles
        .get(&Role::ShotScreen)
        .map(|photo| photo.filename.as_str())
        .ok_or_else(|| format!("{session_id}/{swing_id} was opened with no photo"))?;
    screen::read(&ScreenInput {
        device: DEVICE.to_string(),
        boxes,
        notes: Vec::new(),
        shot_id: format!("{session_id}-{swing_id}"),
        session_id: session_id.clone(),
        timestamp: captured_at,
        image_sha256: Some(digest.to_string()),
        image_path: Some(format!("sessions/{session_id}/{swing_id}/{filename}")),
        min_confidence: MIN_CONFIDENCE,
    })
    .map_err(|refused| refused.to_string())
}

/// What `.app/deleted.json` holds, or nothing (the module doc).
fn deleted_numbers(dirs: &Dirs) -> BTreeMap<String, u64> {
    fs::read(dirs.app.join(DELETED_NAME))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// This entry's number is spent, for good (the module doc). Called with [`one_writer`] held, since it
/// reads the file to change it, as [`record`] reads it to open an entry. An id that is no number needs
/// no floor, since `next_swing_id` gives out nothing else.
pub(crate) fn remember_deleted(dirs: &Dirs, session_id: &str, swing_id: &str) -> io::Result<()> {
    let Ok(number) = swing_id.parse::<u64>() else {
        return Ok(());
    };
    let mut numbers = deleted_numbers(dirs);
    if numbers.get(session_id).is_some_and(|held| *held >= number) {
        return Ok(());
    }
    numbers.insert(session_id.to_string(), number);
    let text = serde_json::to_string_pretty(&numbers).map_err(io::Error::other)?;
    fs::create_dir_all(&dirs.app)?;
    let path = dirs.app.join(DELETED_NAME);
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, &path)
}

/// The entry this call opened goes again.
fn discard(store: &SwingBundleStore, opened: &SwingManifest) -> Result<(), String> {
    let (session_id, swing_id) = (&opened.session_id, &opened.swing_id);
    remove_entry(store, session_id, swing_id).map_err(|failed| {
        format!("{session_id}/{swing_id} holds a photo no shot came from, and stays: {failed}")
    })
}

/// An entry goes, and the session with it if nothing else is there, so an empty date is not left for
/// History to list.
pub(crate) fn remove_entry(
    store: &SwingBundleStore,
    session_id: &str,
    swing_id: &str,
) -> io::Result<()> {
    store.delete_swing(session_id, swing_id)?;
    // `remove_dir` refuses a directory that holds anything, which is the check, as the lab's `tidy`.
    let _ = fs::remove_dir(store.root().join(session_id));
    Ok(())
}

/// `<digest>.<ext>` under `.app/failed/`. A rename, since it keeps the photo's time and costs nothing,
/// and the bytes already in hand if that cannot cross a volume.
fn move_to_failed(dirs: &Dirs, photo: &Path, digest: &str, bytes: &[u8]) -> io::Result<()> {
    let failed = dirs.app.join("failed");
    fs::create_dir_all(&failed)?;
    let name = match photo.extension().and_then(|suffix| suffix.to_str()) {
        Some(suffix) => format!("{digest}.{suffix}"),
        None => digest.to_string(),
    };
    let to = failed.join(name);
    if fs::rename(photo, &to).is_ok() {
        return Ok(());
    }
    fs::write(&to, bytes)?;
    fs::remove_file(photo)
}

/// The entry in another session holding this golfer's photo: the corpus has said there is one, and says
/// only in a sentence which it is.
fn held_elsewhere(
    store: &SwingBundleStore,
    player_id: &str,
    digest: &str,
) -> Option<SwingManifest> {
    store.list_session_ids().iter().find_map(|session_id| {
        session_holding(store, session_id, digest)
            .filter(|held| held.player_id.as_deref() == Some(player_id))
    })
}

/// The card of a shot already held. Its shot not being in the store is refused rather than answered
/// with no card, since a `Duplicate` with nothing to show would read as a photo that vanished.
fn held_card(dirs: &Dirs, held: &SwingManifest, digest: &str) -> Result<ShotCard, String> {
    let shot = ShotStore::new(&dirs.shots)
        .get(digest)
        .map_err(|unreadable| unreadable.to_string())?
        .ok_or_else(|| {
            format!(
                "{}/{} holds this photo, and the shot store holds no shot from it",
                held.session_id, held.swing_id
            )
        })?;
    card_of(&shot, &held.session_id, &held.swing_id)
}

/// A `Timestamp` as the stored photo's modification time.
fn system_time(at: Timestamp) -> SystemTime {
    let micros = at.utc_micros();
    let since = Duration::from_micros(micros.unsigned_abs());
    if micros < 0 {
        UNIX_EPOCH - since
    } else {
        UNIX_EPOCH + since
    }
}
