//! The two verbs that write into a session: `golf-core run-shot --session` and
//! `golf-core set-intent`. [M35 P11]
//!
//! **No Python twin.** Frozen Python analyses a shot only inside a swing, through
//! `api/pipeline.py::analyze_swing_dir`, and it has no intent to set (ADR-035 clause 4). These are the
//! first Rust verbs that write into `data/` beside `flag-mishit`, while the frozen server still
//! writes there too (the M35 plan's decision 6). Every test runs them on a scratch copy.
//!
//! # `run-shot --session`: a photo entry's `shot_analysis.json`
//!
//! For each photo entry in one session, or the one `--swing` names (the M35 plan's call 10): the
//! shot the shot store holds for its photo, through [`crate::run_shot`], written atomically as
//! `shot_analysis.json` beside its manifest ([`save_shot_analysis`]), with one line printed per
//! manifest. A **photo entry** is the corpus's (`storage::corpus`'s module doc): no face-on clip
//! and a shot photo, a down-the-line clip beside it or not. What the shell resolves for it is
//! `api/pipeline.py`'s, read through the ported stores:
//!
//! - **the shot**: the shot store, keyed by the manifest's photo hash, as `_shot_for` looks it up
//!   before any OCR. There is no Rust OCR until §M29's `ort`, so this verb analyses only a photo
//!   frozen Python already parsed. One the store holds no parse for is named, with the frozen import
//!   that would parse it, and skipped (the M35 plan's finding 8);
//! - **the handedness**: the manifest's golfer in the golfer store, `_handedness_for`;
//! - **the loft**: [`loft_for_club`] over the manifest's golfer and club, `_loft_for`;
//! - **the intent**: the manifest's, with the manifest's club as [`crate::ShotInput::club`], so the
//!   goal's club category is the club's (the M35 plan's P4 finding 2).
//!
//! The result names the photo it was computed from (`photo_sha256`), is at this build's
//! `ANALYSIS_VERSION`, and reads as a `ShotResult`, which are the three things the corpus asks before
//! it pools a photo entry (P10 finding 8). Nothing in it depends on the clock, so a second run over
//! the same inputs writes the same bytes.
//!
//! ## What it does not do
//!
//! - **A swing is not analysed here.** A manifest with a face-on clip has its shot analysed with
//!   its swing, by the pipeline, into `analysis.json`, and the corpus reads only that (one artifact
//!   per kind). Walking a session lists it as skipped; naming it with `--swing` is refused.
//! - **A photo entry the corpus counts as a duplicate is skipped**, because a result written for it
//!   is one no reader ever reads (P10 finding 8): the photo-only copy of a swing's photo, or the
//!   later of two photo entries holding one photo. The answer is asked of
//!   [`read_corpus`] itself rather than of a second copy of its dedupe, so the verb and the reader
//!   cannot disagree about which copy counts, and the line prints the corpus's own sentence. An
//!   entry naming no golfer is in no corpus, so nothing makes it a duplicate yet, and it is analysed.
//! - **A drill is analysed and written like any shot** (the M35 plan's decision 1). The corpus is
//!   what leaves it out, as `untracked`.
//! - It runs no OCR, opens no image, and writes no `analysis.json`, no `analysis.state.json` and no
//!   manifest.
//!
//! ## The two gap notes, worded for a shot
//!
//! `analysis` learns only that no handedness or loft arrived, and the shell knows why. So a gap is
//! appended to `notes`, after the engine's own needs-review note (P2 finding 3), in sentences that
//! say "this shot" and name no checkpoint and no page: `_handedness_for`'s say `head_stays_back`,
//! and `loft_remedy`'s name the frozen server's pages. **Each is appended only once the engine has
//! reported the input missing** (`no_handedness`, `no_club_loft`), which is the pipeline's rule for
//! the loft (its ⚠️ comment: a shot whose screen printed a spin flies without one) and, for a shot,
//! the handedness's rule too. A swing always needs a handedness, for `head_stays_back`. A shot needs
//! one only when its screen printed a spin axis, since `+ = fade` is a right-handed reading (the M35
//! plan's P4 finding 5), so an unconditional note would tell a golfer to repair what nothing used.
//!
//! # `set-intent`
//!
//! What a shot was practised as (the M35 plan's decision 7, call 5): `--swing N` records a goal on
//! one manifest through `SwingBundleStore::set_intent`, overwriting what it said, and `--clear`
//! takes it off so it reads as `fundamentals` again. `--default` points the session's intent cursor
//! (`session.json`) at a goal through `storage::session_meta::set_current_intent`, and an upload
//! stamps it onto a swing that records none. `--default --clear` clears the cursor, which the plan's
//! call 10 did not list: without it a cursor once set could only be pointed at another mode.
//!
//! **The shape rules are this verb's**, because the stores record what they are given:
//! `--shape` only with `--mode shot_shaping`, and `shot_shaping` only with a `--shape`, since a shape
//! attempt with no shape declared grades nothing. The goal recorded holds the mode and the shape,
//! and its club stays the default: the club category is filled from the manifest's club when the
//! shot is analysed. `--default` refuses a session directory that does not exist rather than
//! creating one, since `save_session_meta` creates it, as Python's does, and a mistyped id would.
//!
//! # The frozen-Python hazard (the M35 plan's call 12)
//!
//! Frozen Python's `SwingManifest` and `SessionMeta` ignore an unknown key, so they read what these
//! verbs write, and they **drop `intent` on any rewrite**: another role uploaded into the swing,
//! `set_player`, `set_club`, `set_mishit`, `attribute_unlabeled`, or a cursor update. A drill
//! recorded here and then rewritten by the frozen server silently becomes a tracked shot. So
//! `set-intent` is safe on a session the frozen server no longer writes to, and §M29, which
//! replaces that server, is what retires the hazard.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use contracts::career::{ExcludedSwing, ExclusionReason};
use contracts::club::ClubId;
use contracts::golfer::Handedness;
use contracts::intent::{PracticeGoal, PracticeMode, TargetShape};
use contracts::shot_result::ShotResult;
use contracts::unscored::UnscoredReason;
use contracts::Timestamp;
use serde::Serialize;
use storage::bundle_store::SwingBundleStore;
use storage::corpus::{read_corpus, EngineVersions};
use storage::flight_inputs::{loft_for_club, LoftGap};
use storage::golfer_store::GolferStore;
use storage::manifest::{Role, SwingManifest};
use storage::session_meta::set_current_intent;
use storage::shot_store::ShotStore;
use storage::state::save_shot_analysis;

use crate::reports::{data_dir, plural, Answer};
use crate::{run_shot, ShotInput};

/// The three directories `run-shot --session` reads: the sessions, the golfer registry (and the bags
/// beside it), and the shot store. Each by [`data_dir`]'s rule: the flag, else `GOLF_SESSIONS_DIR`,
/// `GOLF_GOLFERS_DIR` or `GOLF_SHOTS_DIR`, else `data/processed/{sessions,golfers,shots}` under the
/// repository (the M35 plan's call 10, as M36's call 11 resolves the first two).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShotDirs {
    pub sessions: PathBuf,
    pub golfers: PathBuf,
    pub shots: PathBuf,
}

impl ShotDirs {
    pub fn resolve(
        sessions: Option<PathBuf>,
        golfers: Option<PathBuf>,
        shots: Option<PathBuf>,
        env: impl Fn(&str) -> Option<String>,
    ) -> Self {
        ShotDirs {
            sessions: data_dir(sessions, "GOLF_SESSIONS_DIR", "sessions", &env),
            golfers: data_dir(golfers, "GOLF_GOLFERS_DIR", "golfers", &env),
            shots: data_dir(shots, "GOLF_SHOTS_DIR", "shots", &env),
        }
    }
}

/// What became of one manifest in a `run-shot --session` walk.
enum Outcome {
    /// A photo entry analysed, and its `shot_analysis.json` written.
    Written(Box<ShotResult>),
    /// A manifest with a face-on clip, whose shot is analysed with its swing.
    Swing,
    /// A manifest with neither a face-on clip nor a shot photo: nothing to analyse.
    NoPhoto,
    /// A photo entry the corpus counts as a duplicate, with the corpus's sentence.
    Duplicate(String),
    /// A photo entry whose photo the shot store holds no parse of, with the repair.
    NoParse(String),
    /// The analysis was refused, the stored parse could not be read, or the write failed.
    Failed(String),
}

/// `golf-core run-shot --session S [--swing N]`: analyse each photo entry in the session, or the
/// one named, and write its `shot_analysis.json` (the module doc).
///
/// Exit 0 when every manifest was analysed or skipped for a reason its line names, an empty session
/// included; 1 when any analysis was refused (a device `devices.json` does not declare), a stored
/// parse could not be read or a write failed, each named on stderr while the rest still run; 2 when
/// the session does not exist, or `--swing` names no manifest, a swing with a face-on clip, or a
/// manifest with no photo, with nothing written. `versions` is what the duplicate check reads the
/// corpus under; it decides nothing a duplicate depends on, and the verb passes this build's pair.
pub fn run_shot_session(
    session_id: &str,
    swing_id: Option<&str>,
    dirs: &ShotDirs,
    versions: EngineVersions,
) -> Answer {
    let refused = |text: String| Answer {
        stdout: String::new(),
        stderr: text,
        code: 2,
    };
    if !dirs.sessions.join(session_id).is_dir() {
        return refused(format!(
            "golf-core run-shot: no session {session_id} in {}\n",
            dirs.sessions.display()
        ));
    }
    let store = SwingBundleStore::new(&dirs.sessions);
    let manifests = match swing_id {
        Some(swing_id) => match store.get_swing(session_id, swing_id) {
            Some(manifest) if manifest.roles.contains_key(&Role::FaceOn) => {
                return refused(format!(
                    "golf-core run-shot: {session_id}/{swing_id} is {SWING_SENTENCE}\n"
                ))
            }
            Some(manifest) if !manifest.roles.contains_key(&Role::ShotScreen) => {
                return refused(format!(
                    "golf-core run-shot: {session_id}/{swing_id} has {NO_PHOTO_SENTENCE}\n"
                ))
            }
            Some(manifest) => vec![manifest],
            None => {
                return refused(format!(
                    "golf-core run-shot: no swing {session_id}/{swing_id}\n"
                ))
            }
        },
        None => store.get_session(session_id),
    };
    if manifests.is_empty() {
        return Answer {
            stdout: format!("{session_id}: no swings\n"),
            stderr: String::new(),
            code: 0,
        };
    }

    let mut duplicates = Duplicates::new(&dirs.sessions, versions);
    let (mut stdout, mut stderr, mut code) = (String::new(), String::new(), 0);
    for manifest in &manifests {
        let reference = format!("{}/{}", manifest.session_id, manifest.swing_id);
        match analyse_entry(manifest, dirs, &mut duplicates) {
            Outcome::Written(result) => {
                stdout += &format!("{reference}: {}\n", written(&result));
            }
            Outcome::Swing => stdout += &format!("{reference}: skipped: {SWING_SENTENCE}\n"),
            Outcome::NoPhoto => stdout += &format!("{reference}: skipped: {NO_PHOTO_SENTENCE}\n"),
            Outcome::Duplicate(detail) | Outcome::NoParse(detail) => {
                stdout += &format!("{reference}: skipped: {detail}\n");
            }
            Outcome::Failed(why) => {
                stderr += &format!("golf-core run-shot: {reference}: not analysed: {why}\n");
                code = 1;
            }
        }
    }
    Answer {
        stdout,
        stderr,
        code,
    }
}

const SWING_SENTENCE: &str = "a swing with a face-on clip, whose shot is analysed with it into \
                              analysis.json";
const NO_PHOTO_SENTENCE: &str = "no shot photo, so there is no shot to analyse";

/// One analysed entry's line: the file written, the mode, and what the result holds.
fn written(result: &ShotResult) -> String {
    let (measured, unscored, notes) = (
        result.measurements.len() as i64,
        result.unscored.len() as i64,
        result.notes.len() as i64,
    );
    let drill = if result.intent.mode.is_tracked() {
        ""
    } else {
        ", a drill that no club or player stat counts"
    };
    format!(
        "wrote shot_analysis.json ({}{drill}): {measured} measurement{}, {unscored} unscored, \
         {notes} note{}",
        wire(&result.intent.mode),
        plural(measured),
        plural(notes),
    )
}

/// One manifest of the walk, start to finish.
fn analyse_entry(
    manifest: &SwingManifest,
    dirs: &ShotDirs,
    duplicates: &mut Duplicates,
) -> Outcome {
    if manifest.roles.contains_key(&Role::FaceOn) {
        return Outcome::Swing;
    }
    let Some(photo) = manifest.roles.get(&Role::ShotScreen) else {
        return Outcome::NoPhoto;
    };
    if let Some(detail) = duplicates.of(manifest) {
        return Outcome::Duplicate(detail);
    }
    let shot = match ShotStore::new(&dirs.shots).get(&photo.content_sha256) {
        Ok(Some(shot)) => shot,
        Ok(None) => {
            let photo_path = dirs
                .sessions
                .join(&manifest.session_id)
                .join(&manifest.swing_id)
                .join(&photo.filename);
            return Outcome::NoParse(format!(
                "the shot store at {} holds no parse of shot photo {}; frozen Python reads it with \
                 python scripts/import_shot_screens.py {} --session {} --out {}, and then this \
                 verb analyses it",
                dirs.shots.display(),
                cut(&photo.content_sha256),
                photo_path.display(),
                manifest.session_id,
                dirs.shots.display(),
            ));
        }
        Err(unreadable) => return Outcome::Failed(unreadable.to_string()),
    };

    let handedness = handedness_for(manifest, &GolferStore::new(&dirs.golfers));
    let loft = loft_for_club(
        manifest.player_id.as_deref(),
        manifest.club,
        Some(&dirs.golfers),
    );
    let input = ShotInput {
        swing_id: manifest.swing_id.clone(),
        session_id: manifest.session_id.clone(),
        shot,
        intent: manifest.intent.clone(),
        club: manifest.club,
        handedness: handedness.as_ref().ok().copied(),
        loft_deg: loft.ok(),
        photo_sha256: Some(photo.content_sha256.clone()),
    };
    let mut result = match run_shot(&input) {
        Ok(result) => result,
        Err(refused) => return Outcome::Failed(refused.to_string()),
    };
    let reached_for = |reason: UnscoredReason| result.unscored.iter().any(|u| u.reason == reason);
    let handedness_note = handedness
        .err()
        .filter(|_| reached_for(UnscoredReason::NoHandedness));
    let loft_note = loft
        .err()
        .and_then(|gap| loft_note(gap, manifest.club))
        .filter(|_| reached_for(UnscoredReason::NoClubLoft));
    result.notes.extend(handedness_note);
    result.notes.extend(loft_note);

    // The corpus reads the result from `manifest.session_id / manifest.swing_id`, the manifest being
    // trusted over the directory it was found in, so it is written where it will be read.
    let swing_dir = dirs
        .sessions
        .join(&manifest.session_id)
        .join(&manifest.swing_id);
    match save_shot_analysis(&result, &swing_dir) {
        Ok(()) => Outcome::Written(Box::new(result)),
        Err(failed) => Outcome::Failed(format!("writing shot_analysis.json: {failed}")),
    }
}

/// Which photo entries the corpus counts as duplicates, read once per golfer.
struct Duplicates<'a> {
    sessions: &'a Path,
    versions: EngineVersions,
    by_golfer: BTreeMap<String, Vec<ExcludedSwing>>,
}

impl<'a> Duplicates<'a> {
    fn new(sessions: &'a Path, versions: EngineVersions) -> Self {
        Duplicates {
            sessions,
            versions,
            by_golfer: BTreeMap::new(),
        }
    }

    /// The corpus's sentence when it excludes this manifest as a duplicate, else `None`. Decided off
    /// the manifests alone, before any artifact is read, so this verb's own writes cannot move it
    /// mid-walk and one read per golfer serves the whole walk.
    fn of(&mut self, manifest: &SwingManifest) -> Option<String> {
        let player_id = manifest.player_id.as_deref()?;
        let (sessions, versions) = (self.sessions, self.versions);
        let excluded = self
            .by_golfer
            .entry(player_id.to_string())
            .or_insert_with(|| read_corpus(sessions, player_id, versions).excluded);
        excluded
            .iter()
            .find(|entry| {
                entry.reason == ExclusionReason::Duplicate
                    && entry.session_id == manifest.session_id
                    && entry.swing_id == manifest.swing_id
            })
            .map(|entry| entry.detail.clone())
    }
}

/// The golfer's handedness, or the note that says how it went missing. `_handedness_for`, worded
/// for a shot: what a missing handedness costs a shot is its landing offline, not `head_stays_back`.
fn handedness_for(manifest: &SwingManifest, golfers: &GolferStore) -> Result<Handedness, String> {
    const COST: &str = "so this shot's landing offline could not be computed: the screen's spin \
                        axis reads '+ = fade', and which way a fade curves depends on which side \
                        the golfer swings from";
    let Some(player_id) = manifest.player_id.as_deref() else {
        return Err(format!("no golfer is attributed to this shot, {COST}"));
    };
    match golfers.get(player_id) {
        Some(golfer) => Ok(golfer.handedness),
        None => Err(format!(
            "golfer {} is not in the registry, {COST}",
            pyfmt::str_repr(player_id)
        )),
    }
}

/// The repair a loft gap wants, worded for a shot. `loft_remedy`, whose sentences name the frozen
/// server's pages and say "this swing". [`LoftGap::NoSwing`] has none, as `remedy.get` has none: this
/// verb is holding the swing, so it cannot arrive.
fn loft_note(gap: LoftGap, club: Option<ClubId>) -> Option<String> {
    const RERUN: &str = "then re-run golf-core run-shot over its session";
    let club = club.map_or("None", ClubId::as_str);
    match gap {
        LoftGap::NoClubTag => Some(format!(
            "no club is tagged on this shot, so its ball flight could not be simulated: tag the \
             club it was hit with, {RERUN}"
        )),
        LoftGap::NoBagEntry => Some(format!(
            "club {club} is not in this golfer's bag, so its loft is unknown and this shot's ball \
             flight could not be simulated: add the club to the bag, {RERUN}"
        )),
        LoftGap::NoDeclaredLoft => Some(format!(
            "the bag entry for {club} carries no loft, so this shot's ball flight could not be \
             simulated: the club's book loft is enough, {RERUN}"
        )),
        LoftGap::NoSwing => None,
    }
}

/// A hash cut to twelve characters, as the corpus's sentences and the reports cut one.
fn cut(sha256: &str) -> String {
    sha256.chars().take(12).collect()
}

/// A unit enum's wire name, through serde, so the spelling has one definition.
fn wire(value: &impl Serialize) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(name)) => name,
        other => panic!("a unit enum serializes as a string, not {other:?}"),
    }
}

// --------------------------------------------------------------------------------- set-intent

/// What `set-intent` writes to: one swing's manifest, or the session's cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentTarget {
    Swing(String),
    SessionDefault,
}

/// One `set-intent` run, once its flags are read: where, and the goal (`None` clears).
#[derive(Debug, Clone, PartialEq)]
pub struct IntentAction {
    pub session_id: String,
    pub target: IntentTarget,
    pub goal: Option<PracticeGoal>,
}

/// The flags' one decision, and the shape rules (the module doc). `Err` is the refusal, which the
/// verb prints under its usage and exits 2 on, as argparse's `parser.error` would.
pub fn intent_action(
    session: Option<String>,
    swing: Option<String>,
    default: bool,
    mode: Option<String>,
    shape: Option<String>,
    clear: bool,
) -> Result<IntentAction, String> {
    let session_id = session
        .filter(|session| !session.is_empty())
        .ok_or("give the session with --session S")?;
    let target = match (swing.filter(|swing| !swing.is_empty()), default) {
        (Some(swing), false) => IntentTarget::Swing(swing),
        (None, true) => IntentTarget::SessionDefault,
        _ => return Err("give exactly one of --swing N and --default".to_string()),
    };
    let goal = match (mode, clear) {
        (Some(_), true) => {
            return Err("--clear records no goal, so it takes no --mode".to_string());
        }
        (None, true) if shape.is_some() => {
            return Err("--clear records no goal, so it takes no --shape".to_string());
        }
        (None, true) => None,
        (None, false) => return Err("give --mode M, or --clear".to_string()),
        (Some(mode), false) => Some(goal_of(&mode, shape.as_deref())?),
    };
    Ok(IntentAction {
        session_id,
        target,
        goal,
    })
}

/// A goal from `--mode` and `--shape`, each in the spelling the manifest writes, and the shape
/// rules. The club stays the default (the module doc).
fn goal_of(mode: &str, shape: Option<&str>) -> Result<PracticeGoal, String> {
    let mode: PracticeMode = serde_json::from_value(serde_json::Value::from(mode))
        .map_err(|_| format!("'{mode}' is not a practice mode, e.g. drill or shot_shaping"))?;
    let target_shape = match shape {
        None => None,
        Some(shape) => Some(
            serde_json::from_value::<TargetShape>(serde_json::Value::from(shape))
                .map_err(|_| format!("'{shape}' is not a target shape, e.g. draw or fade"))?,
        ),
    };
    match (mode, target_shape) {
        (PracticeMode::ShotShaping, None) => Err(
            "--mode shot_shaping needs --shape: a shape attempt with no shape declared grades \
             nothing"
                .to_string(),
        ),
        (PracticeMode::ShotShaping, Some(_)) | (_, None) => Ok(PracticeGoal {
            mode,
            target_shape,
            ..PracticeGoal::default()
        }),
        (_, Some(_)) => Err(format!(
            "--shape goes with --mode shot_shaping alone, and {} declares no shape",
            wire(&mode)
        )),
    }
}

/// `golf-core set-intent`, after its flags: write the goal, or clear it, with `updated_at` stamped
/// `now`. Exit 0 on a write; 2 on a swing or a session that does not exist, with nothing written; 1
/// if the file cannot be written.
pub fn set_intent(action: &IntentAction, sessions_dir: &Path, now: Timestamp) -> Answer {
    let session_id = &action.session_id;
    let refused = |text: String| Answer {
        stdout: String::new(),
        stderr: format!("golf-core set-intent: {text}\n"),
        code: 2,
    };
    let failed = |text: String| Answer {
        stdout: String::new(),
        stderr: format!("golf-core set-intent: writing {text}\n"),
        code: 1,
    };
    let said = match &action.goal {
        Some(goal) => describe(goal),
        None => "cleared, so it reads as fundamentals".to_string(),
    };
    let stdout = match &action.target {
        IntentTarget::Swing(swing_id) => {
            let store = SwingBundleStore::new(sessions_dir);
            match store.set_intent(session_id, swing_id, action.goal.clone(), now) {
                Ok(Some(_)) => format!("{session_id}/{swing_id}: intent {said}\n"),
                Ok(None) => return refused(format!("no swing {session_id}/{swing_id}")),
                Err(e) => return failed(format!("{session_id}/{swing_id}: {e}")),
            }
        }
        IntentTarget::SessionDefault => {
            let session_dir = sessions_dir.join(session_id);
            if !session_dir.is_dir() {
                return refused(format!(
                    "no session {session_id} in {}",
                    sessions_dir.display()
                ));
            }
            if let Err(e) = set_current_intent(&session_dir, action.goal.clone(), now) {
                return failed(format!("{session_id}/session.json: {e}"));
            }
            match action.goal {
                Some(_) => format!(
                    "{session_id}: session intent {said}, stamped onto each swing in it that \
                     records none when its next file arrives\n"
                ),
                None => format!("{session_id}: session intent cleared\n"),
            }
        }
    };
    Answer {
        stdout,
        stderr: String::new(),
        code: 0,
    }
}

/// `= drill`, `= shot_shaping (draw)`, and for a drill what it costs.
fn describe(goal: &PracticeGoal) -> String {
    let shape = goal
        .target_shape
        .map(|shape| format!(" ({})", wire(&shape)))
        .unwrap_or_default();
    let drill = if goal.mode.is_tracked() {
        ""
    } else {
        ", which no club or player stat counts"
    };
    format!("= {}{shape}{drill}", wire(&goal.mode))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(player_id: Option<&str>) -> SwingManifest {
        serde_json::from_value(serde_json::json!({
            "swing_id": "2",
            "session_id": "2026-08-20",
            "created_at": "2026-08-20T12:02:00Z",
            "updated_at": "2026-08-20T12:02:00Z",
            "roles": {},
            "player_id": player_id,
        }))
        .expect("a manifest")
    }

    /// Each gap's sentence says "this shot", names no page and no checkpoint, and ends on the repair
    /// that the next run of this verb would pick up; `NoSwing` has none.
    #[test]
    fn every_gap_note_is_worded_for_a_shot() {
        let notes: Vec<String> = LoftGap::ALL
            .into_iter()
            .filter_map(|gap| loft_note(gap, Some(ClubId::SevenIron)))
            .collect();
        assert_eq!(notes.len(), LoftGap::ALL.len() - 1, "NoSwing has no note");
        assert!(loft_note(LoftGap::NoSwing, Some(ClubId::SevenIron)).is_none());
        assert!(notes[1].starts_with("club 7i is not in this golfer's bag"));
        assert!(notes[2].starts_with("the bag entry for 7i carries no loft"));

        let golfers = GolferStore::new(std::env::temp_dir().join("golf-core-no-registry"));
        let nobody = handedness_for(&manifest(None), &golfers).expect_err("no golfer");
        let unknown = handedness_for(&manifest(Some("brian")), &golfers).expect_err("unknown");
        assert!(nobody.starts_with("no golfer is attributed to this shot"));
        assert!(unknown.starts_with("golfer 'brian' is not in the registry"));

        for note in notes.iter().chain([&nobody, &unknown]) {
            assert!(note.contains("this shot"), "{note}");
            for word in ["page", "this swing", "head_stays_back"] {
                assert!(!note.contains(word), "{note} names {word}");
            }
        }
        for note in &notes {
            assert!(
                note.ends_with("re-run golf-core run-shot over its session"),
                "{note}"
            );
        }
    }

    /// Call 5's refusals, and what each accepted spelling records.
    #[test]
    fn the_flags_decide_and_the_shape_rules_hold() {
        let ok = |swing: Option<&str>, default, mode: Option<&str>, shape: Option<&str>, clear| {
            intent_action(
                Some("s".into()),
                swing.map(str::to_string),
                default,
                mode.map(str::to_string),
                shape.map(str::to_string),
                clear,
            )
        };
        let drill = ok(Some("3"), false, Some("drill"), None, false).expect("a drill");
        assert_eq!(drill.target, IntentTarget::Swing("3".into()));
        assert_eq!(
            drill.goal,
            Some(PracticeGoal {
                mode: PracticeMode::Drill,
                ..PracticeGoal::default()
            })
        );
        let draw = ok(None, true, Some("shot_shaping"), Some("draw"), false).expect("a draw");
        assert_eq!(draw.target, IntentTarget::SessionDefault);
        let goal = draw.goal.expect("a goal");
        assert_eq!(
            (goal.mode, goal.target_shape),
            (PracticeMode::ShotShaping, Some(TargetShape::Draw))
        );
        assert_eq!(
            goal.club,
            PracticeGoal::default().club,
            "the club stays default"
        );
        assert_eq!(
            ok(Some("3"), false, None, None, true)
                .expect("a clear")
                .goal,
            None
        );
        assert_eq!(
            ok(None, true, None, None, true).expect("a clear").goal,
            None
        );

        for (swing, default, mode, shape, clear, says) in [
            (None, false, Some("drill"), None, false, "exactly one of"),
            (
                Some("3"),
                true,
                Some("drill"),
                None,
                false,
                "exactly one of",
            ),
            (
                Some("3"),
                false,
                None,
                None,
                false,
                "give --mode M, or --clear",
            ),
            (
                Some("3"),
                false,
                Some("drill"),
                None,
                true,
                "takes no --mode",
            ),
            (
                Some("3"),
                false,
                None,
                Some("draw"),
                true,
                "takes no --shape",
            ),
            (
                Some("3"),
                false,
                Some("drill"),
                Some("draw"),
                false,
                "drill declares no shape",
            ),
            (
                Some("3"),
                false,
                Some("shot_shaping"),
                None,
                false,
                "needs --shape",
            ),
            (
                Some("3"),
                false,
                Some("Drill"),
                None,
                false,
                "'Drill' is not a practice mode",
            ),
            (
                Some("3"),
                false,
                Some("shot_shaping"),
                Some("hook"),
                false,
                "'hook' is not",
            ),
        ] {
            let refused = ok(swing, default, mode, shape, clear).expect_err(says);
            assert!(refused.contains(says), "{refused}");
        }
        let no_session = intent_action(None, Some("3".into()), false, None, None, true);
        assert!(no_session.is_err_and(|e| e.contains("--session")));
    }

    #[test]
    fn a_goal_is_described_by_its_wire_names() {
        let goal = goal_of("shot_shaping", Some("fade")).expect("a fade");
        assert_eq!(describe(&goal), "= shot_shaping (fade)");
        let drill = goal_of("drill", None).expect("a drill");
        assert_eq!(
            describe(&drill),
            "= drill, which no club or player stat counts"
        );
    }
}
