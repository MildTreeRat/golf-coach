//! The verbs that write into a session: `golf-core run-shot --session` and `golf-core set-intent`
//! [M35 P11], and the three repairs the frozen server's routes were, `golf-core set-golfer`,
//! `set-club` and `delete-swing` [M29 P32]. Beside them, and not a verb, [`open_photo_entry`]: how
//! a photo becomes an entry, moved down from `golf-lab` so the phone opens one with the same code
//! [M38 P3]. And [`correct_shot`], what a person's edit of a read stores [M38 P7].
//!
//! **No Python twin** for the first two. Frozen Python analyses a shot only inside a swing, through
//! `api/pipeline.py::analyze_swing_dir`, and it has no intent to set (ADR-035 clause 4). These were
//! the first Rust verbs that write into `data/` beside `flag-mishit`, while the frozen server still
//! wrote there too (the M35 plan's decision 6). The three repairs are routes' twins, in `api/app.py`
//! (their section below). Every test runs them on a scratch copy.
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
//!   before any OCR. `golf-core` holds no OCR (the reader is `crates/ocr`'s, which only `golf-ocr`
//!   and `golf-lab` reach, so `ort` stays off M38's app), so this verb analyses only a photo something already
//!   parsed. One the store holds no parse for is named, with the `golf-lab import-photos` line that
//!   would parse it, and skipped (the M35 plan's finding 8; the import named frozen Python's script
//!   until M29 P34 deleted it);
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
//! replaces that server, is what retires the hazard. **M29 P32 retired it**: the routes that made
//! those rewrites answer 410 naming the verb that replaces each (the M29 plan's D11), and the
//! repairs below are Rust's, whose manifest keeps `intent` through every one of them.
//!
//! # `set-golfer`, `set-club`, `delete-swing`: the frozen server's repairs [M29 P32]
//!
//! `POST /api/sessions/{s}/swings/{w}/golfer`, `…/club` and `DELETE /api/sessions/{s}/swings/{w}`
//! as verbs, over the store functions that already held their halves:
//! `SwingBundleStore::{set_player, set_club, delete_swing}` and `GolferStore::get_or_create`, each
//! gated by the storage family (`bundle.set_player`, `bundle.set_club`, `bundle.delete_swing`,
//! `golfer.get_or_create`). They exist so that every route D11 closes refuses naming a verb, and
//! so that ADR-024 §5's one repair path for a club outlives the route it was. Each keeps its
//! route's rule, with three differences, each because a CLI is not a page:
//!
//! - **The swing is checked before anything is written.** The golfer route registered a new
//!   golfer and only then learned the swing did not exist; here a refused run writes nothing.
//! - **The ids are path segments the routes' `_safe` admitted**, and the delete also wants a
//!   manifest that reads. `--swing ..` would otherwise name the session's parent, which
//!   `delete_swing` would remove whole; and a directory no store can see is not a swing any page
//!   could have offered to delete.
//! - **A retag says when the stored analysis predates it.** The club picks the loft a flight is
//!   solved with, and the golfer the handedness `head_stays_back` is read in and the bag a loft
//!   comes from. Neither is among the inputs `analysis.state.json` hashes, so nothing else would
//!   mark the stored result as computed under the old one.
//!
//! `set-golfer` registers through `get_or_create`, as the route's `_resolve_golfer` did: a known
//! golfer's stored handedness wins over `--handedness`, and a new golfer needs one, since a guessed
//! handedness reads a normal impact position as a fault forever after. The two cursor routes get
//! no verb: once uploads close nothing reads a cursor, and `golf-lab import-photos --into-session`
//! takes `--player` and `--club` itself.
//!
//! # `open_photo_entry`: one photo opened as an entry [M29 P24, here since M38 P3]
//!
//! What `golf-lab import-photos --into-session` and `import-video --into-session` open each entry
//! with, and what M38's phone opens one with too. It lives here because the phone cannot link
//! `golf-lab`, which reaches `ort` (`crates/ocr/tests/ort_reach.rs`), so a copy in the app would be
//! a second definition of an entry to drift from the first. No Python twin, as `golf-lab`'s had
//! none.
//!
//! - **By name**, `S/N` with `N` the store's next swing id, or above [`NewPhotoEntry::floor`] where
//!   the caller names one, through [`SwingBundleStore::assign_from_path`] as the upload route places
//!   a file: the photo is copied to `.incoming/` with its modification time (the shot's timestamp is
//!   the photo's), then moved in. By name because the store's automatic rule would slot the photo
//!   into the newest swing lacking one, a filmed swing among them (the store's module doc).
//! - **So opening does not dedupe**, and a caller asks [`session_holding`] first, the check that
//!   rule would have made. An entry is opened once per photo, and the corpus would count a second
//!   one as a duplicate anyway.
//! - **It finds out nothing about the photo for itself.** The modification time and the original
//!   file name are arguments and it does not stat, because only the lab's come off the file: the
//!   lab reads them with `fs::metadata` and the path it was given, and the phone's is the time the
//!   shot was taken (the M38 plan's C9), which the scratch copy it holds does not carry.
//! - **Only the entry.** Reading the photo, storing the shot and analysing it stay with the caller,
//!   and so does deleting the entry again when nothing was read from it (`golf_lab::import`'s
//!   module doc), since `golf-core` holds no reader.
//!
//! # `correct_shot`: what a person's edit stores [M38 P7]
//!
//! ADR-014 left a read auditable and M32 left `corrections` empty for M38 to settle, which this does
//! (the M38 plan's C11). No Python twin. It is not a verb yet: the phone's review screen is its one
//! caller, and it lives here and not in `crates/app` so the lab could share it, as the opener above is
//! shared.
//!
//! - **The typed field holds the truth and the map is the audit.** `corrections` maps a field to the
//!   value as read before a person changed it: `pyfmt::repr` of the number, or `""` if nothing was
//!   read. Written on the first edit and never again, and removed when a later edit brings the field
//!   back to it, so an empty map means every value is the reader's, which is what the bay report asks.
//! - **Saving is the review.** `needs_review` goes false with or without an edit, because a person has
//!   now looked, and that is what lets the corpus count a flagged shot. Warnings stay, as what the
//!   read said. The physics cross-checks do not re-run: they judge a read, and a person has overruled
//!   it.
//! - **A person types what the screen prints**, and Rust converts it (the M38 plan's P6 finding 5). A
//!   card holds `85.2` under `Apex Ft` and `2.2 R` under `Ball Direction Deg`, where `ShotData` holds
//!   `28.4` yards and a signed number, and Dart may do neither the arithmetic nor the sign (rule 8). So
//!   an edit is the printed number and, on a tile signed by a lit letter, that letter, read by the
//!   parser's rules: the letter gives the sign, zero needs none, and a unit converts after it. Typing
//!   back what was read therefore lands on the read's own bits and removes the entry. A tile signed
//!   by a word or a polarity (`hd_golf`'s) is refused, since no surface offers one (D3), and a guess
//!   at its sign would be a wrong number half the time.
//! - **The number crosses as text**, because a read of it in Dart is the trap. By its documentation,
//!   not run here, `double.tryParse` answers `null` for `8S.2`, which an `Option` would carry as "clear
//!   this", and `NaN` for `NaN`, which `serde_json` writes as `null`. So what is not a finite number is
//!   refused here, by the user's answer to the same question about a loft in P5: `ShotData` carries no
//!   bound on a reading, on purpose (its doc), so any real number stands. It is still read back through
//!   `ShotData`'s `Validate`, so a bound added there refuses an edit with its own message.
//! - **Refused means nothing written**, for every edit in the call. A clear is allowed, and `letter`
//!   then says nothing. A field named twice ends at its last.
//! - **A value typed for a tile the parse did not locate joins `fields_present`.** That list is the
//!   shot's printed set, and `analysed_view` blanks a reading outside it, so the typed number would be
//!   stored and silently ignored. A person typing it is the evidence the layout has that tile. It is
//!   not taken off again if they later clear it, since the list cannot say whether the parse or a
//!   person put it there, and a tile named `misread` is a smaller error than a printed one named
//!   nowhere.
//! - **Saving with nothing to change moves no byte.** An unflagged card can be opened and saved, and a
//!   stamp would make a re-import at the lab read the entry as differing. The analysis is run
//!   regardless, so it cannot stay behind the store. A change stamps the manifest's `updated_at`, the
//!   one date the entry has for when a person last changed it, and a failed analysis puts the shot
//!   back, so the store and `shot_analysis.json` never disagree.
//! - **Only what the corpus counts.** A swing's shot is analysed with its swing, by the lab, into
//!   `analysis.json`, which this cannot run. An entry the corpus calls a duplicate shares its shot's
//!   record with the entry it duplicates, so that is the one to correct. [`entry_to_change`] holds these
//!   refusals, because the phone's club and intent repairs meet the same ones (M38 P10).

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use contracts::capability::{capability_for, DeviceCapability, FieldSource};
use contracts::career::{ExcludedSwing, ExclusionReason};
use contracts::club::{parse_club, ClubId};
use contracts::golfer::Handedness;
use contracts::intent::{PracticeGoal, PracticeMode, TargetShape};
use contracts::shot::ShotData;
use contracts::shot_result::ShotResult;
use contracts::unscored::UnscoredReason;
use contracts::Timestamp;
use screen::profile::{load_profile, DeviceProfile, FieldKind, ProfileField};
use serde::Serialize;
use serde_json::Value;
use storage::bundle_store::{SwingBundleStore, Upload};
use storage::corpus::{read_corpus, EngineVersions};
use storage::flight_inputs::{loft_for_club, LoftGap};
use storage::golfer_store::{slugify, GolferStore};
use storage::manifest::{Role, SwingManifest};
use storage::session_meta::set_current_intent;
use storage::shot_store::ShotStore;
use storage::state::{load_state, save_shot_analysis, Status, ANALYSIS_NAME, SHOT_ANALYSIS_NAME};
use storage::StoreError;

use crate::reports::{data_dir, plural, Answer, DataDirs};
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
///
/// Public, with [`analyse_entry`] and [`Duplicates`], since M29 P27: `golf-lab reanalyze` re-runs a
/// photo entry through this verb's own code (the M29 plan's C7) and reports each answer in its own
/// words, so it takes the typed answer rather than parsing this verb's line.
pub enum Outcome {
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
pub fn analyse_entry(
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
                "the shot store at {} holds no parse of shot photo {}; golf-lab reads it with \
                 golf-lab import-photos {} --session {} --out {}, and then this verb analyses it",
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
///
/// It owns the sessions directory it reads, so a caller walking entries under more than one (`golf-lab
/// reanalyze`, given swing directories by path) can hold one per directory.
pub struct Duplicates {
    sessions: PathBuf,
    versions: EngineVersions,
    by_golfer: BTreeMap<String, Vec<ExcludedSwing>>,
}

impl Duplicates {
    pub fn new(sessions: &Path, versions: EngineVersions) -> Self {
        Duplicates {
            sessions: sessions.to_path_buf(),
            versions,
            by_golfer: BTreeMap::new(),
        }
    }

    /// The corpus's sentence when it excludes this manifest as a duplicate, else `None`. Decided off
    /// the manifests alone, before any artifact is read, so this verb's own writes cannot move it
    /// mid-walk and one read per golfer serves the whole walk.
    pub fn of(&mut self, manifest: &SwingManifest) -> Option<String> {
        let player_id = manifest.player_id.as_deref()?;
        let (sessions, versions) = (&self.sessions, self.versions);
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

/// The clause ending every loft note, which is true at the lab, where the verb is. `pub` because the
/// stored note is also shown on a phone, which has no such verb and says its own repair in this
/// clause's place (`crates/app`'s `history`). The file keeps these words, so an export reads as the
/// lab's own would. [M38 P9]
pub const RERUN_SHOT: &str = "then re-run golf-core run-shot over its session";

/// The repair a loft gap wants, worded for a shot. `loft_remedy`, whose sentences name the frozen
/// server's pages and say "this swing". [`LoftGap::NoSwing`] has none, as `remedy.get` has none: this
/// verb is holding the swing, so it cannot arrive.
fn loft_note(gap: LoftGap, club: Option<ClubId>) -> Option<String> {
    let club = club.map_or("None", ClubId::as_str);
    match gap {
        LoftGap::NoClubTag => Some(format!(
            "no club is tagged on this shot, so its ball flight could not be simulated: tag the \
             club it was hit with, {RERUN_SHOT}"
        )),
        LoftGap::NoBagEntry => Some(format!(
            "club {club} is not in this golfer's bag, so its loft is unknown and this shot's ball \
             flight could not be simulated: add the club to the bag, {RERUN_SHOT}"
        )),
        LoftGap::NoDeclaredLoft => Some(format!(
            "the bag entry for {club} carries no loft, so this shot's ball flight could not be \
             simulated: the club's book loft is enough, {RERUN_SHOT}"
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

// ------------------------------------------------------ set-golfer, set-club, delete-swing

/// The three repair verbs, by the name each is run under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairVerb {
    SetGolfer,
    SetClub,
    DeleteSwing,
}

impl RepairVerb {
    pub const ALL: [RepairVerb; 3] = [
        RepairVerb::SetGolfer,
        RepairVerb::SetClub,
        RepairVerb::DeleteSwing,
    ];

    pub fn name(self) -> &'static str {
        match self {
            RepairVerb::SetGolfer => "set-golfer",
            RepairVerb::SetClub => "set-club",
            RepairVerb::DeleteSwing => "delete-swing",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|verb| verb.name() == name)
    }
}

/// What a repair does to its swing, once its flags are read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Repair {
    /// The golfer `name` slugs to, registered with `handedness` if the registry has no such golfer.
    Golfer {
        name: String,
        handedness: Option<Handedness>,
    },
    Club(ClubId),
    Delete,
}

/// One repair run: the swing, and what is done to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairAction {
    pub session_id: String,
    pub swing_id: String,
    pub repair: Repair,
}

impl RepairAction {
    pub fn verb(&self) -> RepairVerb {
        match self.repair {
            Repair::Golfer { .. } => RepairVerb::SetGolfer,
            Repair::Club(_) => RepairVerb::SetClub,
            Repair::Delete => RepairVerb::DeleteSwing,
        }
    }
}

/// `api/app.py::_SAFE_SEGMENT`, `^[A-Za-z0-9][A-Za-z0-9._-]*$`: what the routes' `_safe` let into a
/// path, and so what these verbs join to one. By hand, as `contracts::golfer::is_player_id` is,
/// since Rust carries no regex engine. `..` and `.hidden` fail on their first character and `a/b`
/// on the slash. Python's `$` also matched before one trailing newline, which only ever let a
/// newline into a path, so that is not ported.
fn is_safe_segment(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphanumeric())
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
}

/// A swing's two ids, each one the store could have named, or the refusal saying which is not. `pub`
/// because the phone joins them to a path too, and `delete_swing` would remove whatever `..` names
/// (M38 P10).
pub fn check_segments(session_id: &str, swing_id: &str) -> Result<(), String> {
    for (kind, id) in [("session", session_id), ("swing", swing_id)] {
        if !is_safe_segment(id) {
            return Err(format!(
                "{kind} id {} is not one the store names: letters, digits, '.', '_' and '-', \
                 starting with a letter or a digit",
                pyfmt::str_repr(id)
            ));
        }
    }
    Ok(())
}

/// The flags' one decision for `verb`: the swing, as two safe segments, and the value its verb
/// needs, parsed. `Err` is the refusal, which the binary prints under its usage and exits 2 on, as
/// [`intent_action`]'s is. The binary has already refused a flag `verb` does not take, so a value
/// given here for another verb is not looked at.
pub fn repair_action(
    verb: RepairVerb,
    session: Option<String>,
    swing: Option<String>,
    name: Option<String>,
    handedness: Option<String>,
    club: Option<String>,
) -> Result<RepairAction, String> {
    let session_id = session
        .filter(|session| !session.is_empty())
        .ok_or("give the session with --session S")?;
    let swing_id = swing
        .filter(|swing| !swing.is_empty())
        .ok_or("give the swing with --swing W")?;
    check_segments(&session_id, &swing_id)?;
    let repair = match verb {
        RepairVerb::SetGolfer => {
            let name = name
                .filter(|name| !name.is_empty())
                .ok_or("give the golfer with --name N")?;
            // `get_or_create`'s own refusal, worded as the store words it, since a slug of nothing
            // is refused before the registry is read.
            if slugify(&name).is_empty() {
                return Err(format!(
                    "name {} has no usable characters for an id",
                    pyfmt::str_repr(&name)
                ));
            }
            let handedness = match handedness {
                None => None,
                Some(text) => Some(
                    serde_json::from_value::<Handedness>(serde_json::Value::from(text.as_str()))
                        .map_err(|_| format!("'{text}' is not a handedness: give right or left"))?,
                ),
            };
            Repair::Golfer { name, handedness }
        }
        RepairVerb::SetClub => {
            let text = club.ok_or("give the club with --club C")?;
            // `_resolve_club`'s refusal, word for word: never nudged toward a nearest match.
            Repair::Club(parse_club(&text).ok_or_else(|| {
                format!(
                    "{} is not a club in the bag — try '7i', '7 iron' or 'pw'",
                    pyfmt::str_repr(&text)
                )
            })?)
        }
        RepairVerb::DeleteSwing => Repair::Delete,
    };
    Ok(RepairAction {
        session_id,
        swing_id,
        repair,
    })
}

/// `golf-core set-golfer`, `set-club` or `delete-swing`, after its flags: one line on stdout naming
/// what was written, with `updated_at` stamped `now` on a retag and on a golfer registered. Exit 0
/// on a write; 2 on a swing that does not exist (or is being analysed, for a delete) and on a new
/// golfer given no handedness, each with nothing written; 1 if a file cannot be written.
pub fn repair(action: &RepairAction, dirs: &DataDirs, now: Timestamp) -> Answer {
    let verb = action.verb().name();
    let (session_id, swing_id) = (&action.session_id, &action.swing_id);
    let reference = format!("{session_id}/{swing_id}");
    let refused = |text: String| Answer {
        stdout: String::new(),
        stderr: format!("golf-core {verb}: {text}\n"),
        code: 2,
    };
    let failed = |what: &str, e: &dyn std::fmt::Display| Answer {
        stdout: String::new(),
        stderr: format!("golf-core {verb}: writing {what}: {e}\n"),
        code: 1,
    };
    let store = SwingBundleStore::new(&dirs.sessions);
    let Some(before) = store.get_swing(session_id, swing_id) else {
        return refused(format!("no swing {reference}"));
    };
    let swing_dir = dirs.sessions.join(session_id).join(swing_id);
    // The sentence a changed value earns when an analysis is on disk (the module doc's third
    // difference): `golf-lab reanalyze` re-runs a swing and a photo entry alike (the M29 plan's C7).
    let predates = |used: &str| {
        if swing_dir.join(ANALYSIS_NAME).is_file() || swing_dir.join(SHOT_ANALYSIS_NAME).is_file() {
            format!(
                "; its stored analysis {used}, so re-run golf-lab reanalyze {reference} to bring \
                 it up to date"
            )
        } else {
            String::new()
        }
    };

    let line = match &action.repair {
        Repair::Golfer { name, handedness } => {
            let golfers = GolferStore::new(&dirs.golfers);
            let player_id = slugify(name);
            let (golfer, registered) = match golfers.get(&player_id) {
                Some(known) => (known, false),
                None => {
                    let Some(handedness) = *handedness else {
                        return refused(format!(
                            "{} is new here — say whether they swing right- or left-handed, \
                             with --handedness right or --handedness left",
                            pyfmt::str_repr(pyfmt::strip(name))
                        ));
                    };
                    match golfers.get_or_create(name, handedness, now) {
                        Ok(golfer) => (golfer, true),
                        Err(StoreError::Refused(text)) => return refused(text),
                        Err(e) => return failed(&format!("golfer {player_id}"), &e),
                    }
                }
            };
            if let Err(e) = store.set_player(session_id, swing_id, &golfer.player_id, now) {
                return failed(&reference, &e);
            }
            let was = before.player_id.as_deref().unwrap_or("none");
            let mut line = format!("{reference}: golfer = {}, was {was}", golfer.player_id);
            let hand = golfer.handedness.as_str();
            if registered {
                line += &format!(" (registered now: {}, {hand}-handed)", golfer.display_name);
            } else if let Some(given) = handedness.filter(|given| *given != golfer.handedness) {
                line += &format!(
                    " ({} is on file as {hand}-handed, so --handedness {} changed nothing)",
                    golfer.player_id,
                    given.as_str()
                );
            }
            if before.player_id.as_deref() != Some(golfer.player_id.as_str()) {
                line += &predates("was read with the old golfer's handedness and bag");
            }
            line
        }
        Repair::Club(club) => {
            if let Err(e) = store.set_club(session_id, swing_id, *club, now) {
                return failed(&reference, &e);
            }
            let was = before.club.map_or("none", ClubId::as_str);
            let mut line = format!("{reference}: club = {}, was {was}", club.as_str());
            if before.club != Some(*club) {
                line += &predates("flew it with the old club's loft");
            }
            line
        }
        Repair::Delete => {
            // The route's 409: deleting under a running job loses the race, since the worker's
            // `save_state` re-creates the directory for its terminal state and leaves a swing with
            // a state file and no files. The frozen worker still runs while the frozen server does.
            if let Some(state) = load_state(&swing_dir)
                .filter(|state| matches!(state.status, Status::Queued | Status::Running))
            {
                return refused(format!(
                    "{reference} is being analyzed ({}) — wait for it to finish, then delete it",
                    wire(&state.status)
                ));
            }
            match store.delete_swing(session_id, swing_id) {
                Ok(true) => {}
                Ok(false) => return refused(format!("no swing {reference}")),
                Err(e) => return failed(&reference, &e),
            }
            let roles: Vec<String> = before.roles.keys().map(wire).collect();
            let held = if roles.is_empty() {
                "no file".to_string()
            } else {
                roles.join(", ")
            };
            format!("{reference}: deleted, with every file in its directory (it held {held})")
        }
    };
    Answer {
        stdout: format!("{line}\n"),
        stderr: String::new(),
        code: 0,
    }
}

// -------------------------------------------------------------------------------- correct_shot

/// One tile a person changed, in the screen's terms (the module doc).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShotEdit {
    /// The `ShotData` key, which is what a card's row names.
    pub field: String,
    /// The number as the screen prints it, as typed: `85.2` on a tile printed in feet, `2.2` on one
    /// signed by a lit letter. `None` clears the value.
    pub printed: Option<String>,
    /// The lit letter, on a tile signed by one and only there. A printed zero needs none.
    pub letter: Option<String>,
}

/// A photo entry as it stands when a person goes to change something [`analyse_entry`] reads, with
/// what that asks already asked. [M38 P10]
pub struct EntryToChange {
    pub manifest: SwingManifest,
    /// The shot its photo holds. A change to the manifest is analysed from this too, so an entry with
    /// none is refused before anything moves, not after the manifest has.
    pub shot: ShotData,
    /// Already read for this golfer, so the analysis that follows reads the corpus no second time.
    pub duplicates: Duplicates,
}

/// The entry [`correct_shot`] and the phone's club and intent repairs may change, or the refusal saying
/// why not, with nothing written: the module doc's "only what the corpus counts", in one place so the
/// callers cannot differ on it. It is asked before a write because [`analyse_entry`]'s own
/// [`Outcome::Duplicate`] arrives after one (the M38 plan's P7 finding 8).
pub fn entry_to_change(
    dirs: &ShotDirs,
    session_id: &str,
    swing_id: &str,
    versions: EngineVersions,
) -> Result<EntryToChange, String> {
    check_segments(session_id, swing_id)?;
    let reference = format!("{session_id}/{swing_id}");
    let manifest = SwingBundleStore::new(&dirs.sessions)
        .get_swing(session_id, swing_id)
        .ok_or_else(|| format!("no swing {reference}"))?;
    if manifest.roles.contains_key(&Role::FaceOn) {
        return Err(format!("{reference} is {SWING_SENTENCE}"));
    }
    let photo = manifest
        .roles
        .get(&Role::ShotScreen)
        .ok_or_else(|| format!("{reference} has {NO_PHOTO_SENTENCE}"))?;
    let mut duplicates = Duplicates::new(&dirs.sessions, versions);
    if let Some(detail) = duplicates.of(&manifest) {
        return Err(format!(
            "{reference} is not the entry its photo counts under: {detail}"
        ));
    }
    let shot = ShotStore::new(&dirs.shots)
        .get(&photo.content_sha256)
        .map_err(|unreadable| unreadable.to_string())?
        .ok_or_else(|| format!("{reference} holds a photo the shot store holds no shot from"))?;
    Ok(EntryToChange {
        manifest,
        shot,
        duplicates,
    })
}

/// Store a person's review of one photo entry's shot, and re-analyse it: the module doc's rules.
/// `edits` empty is "reviewed, nothing to change". Answers the shot as stored.
///
/// `Err` is a refusal with nothing written, or a write that failed, which says what it could not do.
/// `versions` is what the duplicate check reads the corpus under, as [`run_shot_session`]'s is, and
/// `now` stamps the manifest when the shot changes.
pub fn correct_shot(
    dirs: &ShotDirs,
    session_id: &str,
    swing_id: &str,
    edits: &[ShotEdit],
    versions: EngineVersions,
    now: Timestamp,
) -> Result<ShotData, String> {
    let EntryToChange {
        manifest,
        shot,
        mut duplicates,
    } = entry_to_change(dirs, session_id, swing_id, versions)?;
    let reference = format!("{session_id}/{swing_id}");
    let store = SwingBundleStore::new(&dirs.sessions);
    let shots = ShotStore::new(&dirs.shots);

    let corrected = with_edits(&shot, edits)?;
    let changed = corrected != shot;
    if changed {
        // The store's one stamp is a repair of a manifest field, and this writes back the one it
        // holds, which `golf-core set-intent` would do too, so nothing but `updated_at` moves.
        store
            .set_intent(session_id, swing_id, manifest.intent.clone(), now)
            .map_err(|failed| format!("could not stamp {reference}: {failed}"))?;
        shots
            .put(&corrected)
            .map_err(|failed| format!("could not store {reference}'s shot: {failed}"))?;
    }
    let failure = match analyse_entry(&manifest, dirs, &mut duplicates) {
        Outcome::Written(_) => return Ok(corrected),
        Outcome::Failed(why) | Outcome::NoParse(why) | Outcome::Duplicate(why) => why,
        Outcome::Swing => SWING_SENTENCE.to_string(),
        Outcome::NoPhoto => NO_PHOTO_SENTENCE.to_string(),
    };
    if changed {
        shots.put(&shot).map_err(|failed| {
            format!("{reference} could not be analysed ({failure}), and its shot could not be put back: {failed}")
        })?;
    }
    Err(format!("{reference} could not be analysed: {failure}"))
}

/// The shot with `edits` applied and its review recorded, written nowhere yet, or the refusal.
///
/// Through `serde_json::Value`, as `capability::analysed_view` is, for that function's reason: the
/// struct stays the only list of its fields. Reading the result back is what asks `Validate`.
fn with_edits(shot: &ShotData, edits: &[ShotEdit]) -> Result<ShotData, String> {
    let read_by = shot.provenance.as_ref().ok_or_else(|| {
        format!(
            "shot {} holds no provenance, so no screen read it and there is no read to correct",
            shot.shot_id
        )
    })?;
    let capability = capability_for(&read_by.device).map_err(|refused| refused.to_string())?;
    let profile = load_profile(&read_by.device).map_err(|refused| refused.to_string())?;

    let mut provenance = read_by.clone();
    let mut written = serde_json::to_value(shot).map_err(|failed| failed.to_string())?;
    for edit in edits {
        let field = edit.field.as_str();
        let tile = tile_to_correct(capability, profile, field)?;
        let after = edit
            .printed
            .as_deref()
            .map(|typed| stored_number(tile, typed, edit.letter.as_deref()))
            .transpose()?;

        let spelled = |value: Option<f64>| value.map_or_else(String::new, pyfmt::repr);
        let as_read = provenance
            .corrections
            .remove(field)
            .unwrap_or_else(|| spelled(written[field].as_f64()));
        if spelled(after) != as_read {
            provenance.corrections.insert(field.to_string(), as_read);
        }
        if let (Some(_), Some(present)) = (after, provenance.fields_present.as_mut()) {
            if !present.iter().any(|located| located == field) {
                present.push(field.to_string());
            }
        }
        written[field] = after.map_or(Value::Null, Value::from);
    }
    provenance.needs_review = false;
    written["provenance"] =
        serde_json::to_value(&provenance).map_err(|failed| failed.to_string())?;
    serde_json::from_value(written).map_err(|refused| refused.to_string())
}

/// The tile a person may correct `field` on, or the refusal naming the field: a number the device's
/// screen prints, signed in a way a person can say (the module doc).
fn tile_to_correct<'a>(
    capability: &DeviceCapability,
    profile: &'a DeviceProfile,
    field: &str,
) -> Result<&'a ProfileField, String> {
    let device = &capability.device;
    let named = pyfmt::str_repr(field);
    let declared = capability
        .fields
        .iter()
        .find(|declared| declared.field == field)
        .ok_or_else(|| format!("{named} is not a reading {device} prints, so it has no tile"))?;
    if declared.from != FieldSource::Screen {
        return Err(format!(
            "{named} is not on {device}'s screen, so there is no tile to correct it from"
        ));
    }
    let tile = profile
        .fields
        .iter()
        .find(|tile| tile.target.as_deref() == Some(field))
        .ok_or_else(|| format!("{device}'s profile has no tile for {named}"))?;
    if tile.kind != FieldKind::Number {
        return Err(format!(
            "{named} holds words, and only a number can be typed"
        ));
    }
    if !tile.sign_tokens.is_empty() || tile.printed_sign.is_some() {
        return Err(format!(
            "{named} is signed by a word or a polarity on {device}'s screen, and no surface says \
             which"
        ));
    }
    Ok(tile)
}

/// What the screen printed, as `ShotData` holds it: `parser::read_text`'s order for a number, the sign
/// and then the unit, so a number typed back lands on the read's own bits.
fn stored_number(tile: &ProfileField, typed: &str, letter: Option<&str>) -> Result<f64, String> {
    let label = &tile.label;
    let number = pyfmt::strip(typed)
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
        .ok_or_else(|| format!("{label}: {} is not a number", pyfmt::str_repr(typed)))?;
    let signed = if tile.lit_sign.is_empty() {
        if let Some(letter) = letter {
            return Err(format!(
                "{label} lights no letter, so {} signs nothing",
                pyfmt::str_repr(letter)
            ));
        }
        number
    } else {
        lit_number(tile, number, letter)?
    };
    Ok(tile
        .printed_unit
        .map_or(signed, |unit| unit.to_yards(signed)))
}

/// A number on a tile signed by a lit letter. The tile prints it unsigned, so a typed `-` says nothing
/// the letter could agree with and is refused rather than discarded as a read's stray mark is: a person
/// who typed one meant something by it. **Zero needs no sign**, and is `0.0` whatever is lit, as a read
/// of it is. A number with no letter is refused and not stored unsigned (the parser's own rule, ADR-010
/// §2).
fn lit_number(tile: &ProfileField, number: f64, letter: Option<&str>) -> Result<f64, String> {
    let label = &tile.label;
    let letters = tile
        .lit_sign
        .iter()
        .map(|lit| lit.token.as_str())
        .collect::<Vec<_>>()
        .join(" or ");
    let lit = letter
        .map(|letter| {
            tile.lit_sign
                .iter()
                .find(|lit| lit.token == letter)
                .ok_or_else(|| {
                    format!(
                        "{label} lights {letters}, and not {}",
                        pyfmt::str_repr(letter)
                    )
                })
        })
        .transpose()?;
    if number < 0.0 {
        return Err(format!(
            "{label} prints no sign of its own: type the number, and choose {letters}"
        ));
    }
    if number == 0.0 {
        return Ok(0.0);
    }
    let lit = lit.ok_or_else(|| {
        format!("{label} needs its lit letter, {letters}, or the number has no direction")
    })?;
    Ok(number * lit.sign as f64)
}

// ---------------------------------------------------------------------------- open_photo_entry

/// One photo for [`open_photo_entry`], with everything about it that the opener does not go and
/// find out for itself (the module doc).
pub struct NewPhotoEntry<'a> {
    pub session_id: &'a str,
    /// The bytes' SHA-256, `shot_store::hash_image`'s, which the caller has already computed to ask
    /// [`session_holding`].
    pub digest: &'a str,
    pub bytes: &'a [u8],
    /// The name the photo arrived under: the manifest records it, and its suffix picks the content
    /// type and the stored file's extension.
    pub original_filename: &'a str,
    /// Set on the stored copy, because the shot's timestamp is the photo's.
    pub modified: SystemTime,
    pub player_id: &'a str,
    pub club: ClubId,
    pub intent: Option<&'a PracticeGoal>,
    /// The highest number this session must not open again, or `0`, which is no floor, since a
    /// session's numbers start at 1.
    ///
    /// The store numbers from what it holds and remembers nothing, so a number deleted is free again,
    /// and only a caller whose entries leave for another store can be hurt by that. The phone's are
    /// exported, and a reused number is other bytes at a path the lab already holds, which refuses
    /// that import and every later one (`app`'s `.app/deleted.json`, the M38 plan's P13). The lab has
    /// nowhere to export to, so it passes `0`.
    pub floor: u64,
}

/// The session's next swing id, raised above the floor. A next number too long for a `u64` is above
/// any floor there can be, since the store carries its numbers as digits at any length.
fn number_above(next: String, floor: u64) -> String {
    match next.parse::<u64>() {
        Ok(number) if number <= floor => floor.saturating_add(1).to_string(),
        _ => next,
    }
}

/// Open a photo entry under the session's next swing id, and read its manifest back (the module
/// doc). A photo that cannot be placed leaves nothing in `.incoming/`.
pub fn open_photo_entry(
    store: &SwingBundleStore,
    photo: &NewPhotoEntry<'_>,
    now: Timestamp,
) -> io::Result<SwingManifest> {
    let incoming = store.root().join(".incoming");
    fs::create_dir_all(&incoming)?;
    let tmp = incoming.join(format!("{}.{}.part", photo.digest, std::process::id()));
    fs::write(&tmp, photo.bytes)?;
    let placed = fs::File::options()
        .write(true)
        .open(&tmp)
        .and_then(|file| file.set_modified(photo.modified))
        .and_then(|()| {
            let swing_id = number_above(store.next_swing_id(photo.session_id), photo.floor);
            store.assign_from_path(
                &Upload {
                    session_id: photo.session_id,
                    role: Role::ShotScreen,
                    tmp_path: &tmp,
                    digest: photo.digest,
                    original_filename: photo.original_filename,
                    content_type: content_type_of(Path::new(photo.original_filename)),
                    size_bytes: i64::try_from(photo.bytes.len()).unwrap_or(i64::MAX),
                    swing_id: Some(&swing_id),
                    player_id: Some(photo.player_id),
                    club: Some(photo.club),
                    intent: photo.intent,
                },
                now,
            )
        });
    let placed = match placed {
        Ok(placed) => placed,
        Err(error) => {
            let _ = fs::remove_file(&tmp);
            return Err(error);
        }
    };
    store
        .get_swing(&placed.session_id, &placed.swing_id)
        .ok_or_else(|| io::Error::other("the entry's manifest could not be read back"))
}

/// The swing in `session_id` already holding these bytes as its photo, if one does: the dedupe the
/// store's automatic rule makes, which opening an entry by name skips. `import-photos
/// --into-session` opens an entry once per photo by it, and `import-export` skips an exported photo
/// entry by it, so the two cannot disagree about which photo a session holds.
pub fn session_holding(
    store: &SwingBundleStore,
    session_id: &str,
    digest: &str,
) -> Option<SwingManifest> {
    store.get_session(session_id).into_iter().find(|manifest| {
        manifest
            .roles
            .get(&Role::ShotScreen)
            .is_some_and(|photo| photo.content_sha256 == digest)
    })
}

/// What the upload route records as the content type, from the suffix: the phone sent `image/jpeg`
/// and `image/png` for the stored photos, and `application/octet-stream` for the one HEIC.
pub fn content_type_of(image: &Path) -> &'static str {
    let suffix = image
        .extension()
        .and_then(|suffix| suffix.to_str())
        .map(str::to_lowercase);
    match suffix.as_deref() {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("heic") => "image/heic",
        Some("heif") => "image/heif",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use contracts::swing::{ANALYSIS_VERSION, COMPARABLE_FROM};
    use storage::manifest::MANIFEST_NAME;
    use storage::state::load_shot_analysis;

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

    /// A stores root for the opener's tests, gone with the test. Hand-rolled because this crate has
    /// no `tempfile`, as `storage_family`'s `Scratch` says, whose constructor is that module's own.
    struct Root(PathBuf);

    impl Root {
        fn new(label: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("golf-core-open-{label}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("a scratch root");
            Root(path)
        }
    }

    impl Drop for Root {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const PHOTO: &[u8] = b"the photo's bytes";

    fn opened_at() -> Timestamp {
        Timestamp::parse("2026-11-04T20:00:00+00:00").expect("a timestamp")
    }

    fn new_entry<'a>(digest: &'a str, modified: SystemTime) -> NewPhotoEntry<'a> {
        NewPhotoEntry {
            session_id: "2026-11-04-phone-a1b2",
            digest,
            bytes: PHOTO,
            original_filename: "IMG_2893.HEIC",
            modified,
            player_id: "aaron",
            club: ClubId::SevenIron,
            intent: None,
            floor: 0,
        }
    }

    /// The opener stats nothing, and here there is no source file it could have read a time or a
    /// name from, so what lands is what it was handed. This is the phone's whole use of it: a
    /// library photo's stored copy carries when the shot was taken and not when it was copied.
    #[test]
    fn an_entry_holds_the_time_and_the_name_it_was_handed() {
        let root = Root::new("handed");
        let store = SwingBundleStore::new(&root.0);
        let digest = storage::shot_store::hash_image(PHOTO);
        let taken = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_790_000_000);
        let drill = PracticeGoal {
            mode: PracticeMode::Drill,
            ..PracticeGoal::default()
        };
        let photo = NewPhotoEntry {
            intent: Some(&drill),
            ..new_entry(&digest, taken)
        };

        let opened = open_photo_entry(&store, &photo, opened_at()).expect("opened");

        assert_eq!(opened.swing_id, "1");
        assert_eq!(opened.player_id.as_deref(), Some("aaron"));
        assert_eq!(opened.club, Some(ClubId::SevenIron));
        assert_eq!(opened.intent.as_ref(), Some(&drill));
        let role = &opened.roles[&Role::ShotScreen];
        assert_eq!(role.original_filename, "IMG_2893.HEIC");
        assert_eq!(role.content_type, "image/heic");
        assert_eq!(role.content_sha256, digest);
        assert_eq!(role.size_bytes, PHOTO.len() as i64);
        let stored = root.0.join(photo.session_id).join("1").join(&role.filename);
        assert_eq!(fs::read(&stored).expect("the stored photo"), PHOTO);
        let modified = fs::metadata(&stored).and_then(|meta| meta.modified());
        assert_eq!(modified.expect("its time"), taken);
        let incoming = fs::read_dir(root.0.join(".incoming")).expect("the scratch directory");
        assert_eq!(incoming.count(), 0, "the part moved into the swing");
    }

    /// The number the phone's fix rests on (the M38 plan's P13): the floor lifts an entry above a number
    /// nobody holds any more, and never below what the store would open anyway. `0` is the lab's answer,
    /// and the store's own rule.
    #[test]
    fn an_entry_opens_above_its_floor_and_above_what_the_session_holds() {
        let root = Root::new("floor");
        let store = SwingBundleStore::new(&root.0);
        let digest = storage::shot_store::hash_image(PHOTO);
        let open = |floor: u64| {
            let photo = NewPhotoEntry {
                floor,
                ..new_entry(&digest, SystemTime::UNIX_EPOCH)
            };
            open_photo_entry(&store, &photo, opened_at())
                .expect("opened")
                .swing_id
        };

        assert_eq!(open(0), "1", "no floor is the store's own number");
        assert_eq!(
            open(5),
            "6",
            "above the floor, with the session holding less"
        );
        assert_eq!(open(5), "7", "above what it holds, with the floor below it");
        assert_eq!(open(8), "9", "the number the floor names is not opened");
    }

    /// The store's numbers have no ceiling, so the floor's cannot be allowed to bring one back down.
    #[test]
    fn a_number_too_long_for_the_floor_to_read_stays_the_stores() {
        let long = "18446744073709551616".to_string();
        assert_eq!(number_above(long.clone(), u64::MAX), long);
        assert_eq!(number_above(long.clone(), 3), long);
        assert_eq!(number_above("4".to_string(), 3), "4");
        assert_eq!(number_above("3".to_string(), 3), "4");
    }

    /// Opening is by name, so the same bytes open a second entry if asked. That is why a caller
    /// asks `session_holding` first, and the phone's `Duplicate` answer rests on it.
    #[test]
    fn opening_does_not_dedupe_and_session_holding_is_what_does() {
        let root = Root::new("holding");
        let store = SwingBundleStore::new(&root.0);
        let digest = storage::shot_store::hash_image(PHOTO);
        let photo = new_entry(&digest, SystemTime::UNIX_EPOCH);
        assert!(session_holding(&store, photo.session_id, &digest).is_none());

        let first = open_photo_entry(&store, &photo, opened_at()).expect("the first");
        let second = open_photo_entry(&store, &photo, opened_at()).expect("the second");
        assert_eq!(
            (first.swing_id.as_str(), second.swing_id.as_str()),
            ("1", "2")
        );

        let held = session_holding(&store, photo.session_id, &digest).expect("held");
        assert_eq!(held.swing_id, "1");
        assert!(session_holding(&store, photo.session_id, "another digest").is_none());
        assert!(session_holding(&store, "another-session", &digest).is_none());
    }

    /// A session path that is a file refuses the swing directory, after the part was written.
    #[test]
    fn a_photo_that_cannot_be_placed_leaves_nothing_in_incoming() {
        let root = Root::new("refused");
        let store = SwingBundleStore::new(&root.0);
        let digest = storage::shot_store::hash_image(PHOTO);
        let photo = new_entry(&digest, SystemTime::UNIX_EPOCH);
        fs::write(root.0.join(photo.session_id), b"not a directory").expect("in the way");

        assert!(open_photo_entry(&store, &photo, opened_at()).is_err());

        let incoming = fs::read_dir(root.0.join(".incoming")).expect("the scratch directory");
        assert_eq!(incoming.count(), 0, "the part went with the failure");
    }

    /// The suffix alone, in any case, and anything else is the upload route's fallback.
    #[test]
    fn a_content_type_comes_from_the_suffix_in_any_case() {
        for (name, content_type) in [
            ("a.jpg", "image/jpeg"),
            ("a.JPEG", "image/jpeg"),
            ("a.png", "image/png"),
            ("IMG_2893.HEIC", "image/heic"),
            ("a.heif", "image/heif"),
            ("a.webp", "application/octet-stream"),
            ("a", "application/octet-stream"),
            ("", "application/octet-stream"),
        ] {
            assert_eq!(content_type_of(Path::new(name)), content_type, "{name:?}");
        }
    }

    // ---------------------------------------------------------------------------- correct_shot

    const SESSION: &str = "2026-11-04-phone-a1b2";

    /// One photo entry with its shot stored and analysed, as a phone's read leaves it. Full Swing's
    /// numbers as `ShotData` holds them: `-0.8` is printed `0.8 L`, the apex is printed `85.2` feet,
    /// and `sidespin` was located and withheld.
    struct Entry {
        root: Root,
        dirs: ShotDirs,
        digest: String,
    }

    impl Entry {
        fn new(label: &str) -> Self {
            Self::with(label, |_| {})
        }

        fn with(label: &str, change: impl FnOnce(&mut Value)) -> Self {
            let root = Root::new(&format!("correct-{label}"));
            let dirs = ShotDirs {
                sessions: root.0.join("sessions"),
                golfers: root.0.join("golfers"),
                shots: root.0.join("shots"),
            };
            GolferStore::new(&dirs.golfers)
                .get_or_create("Aaron", Handedness::Right, opened_at())
                .expect("a golfer");
            let store = SwingBundleStore::new(&dirs.sessions);
            let digest = storage::shot_store::hash_image(PHOTO);
            let manifest = open_photo_entry(
                &store,
                &new_entry(&digest, SystemTime::UNIX_EPOCH),
                opened_at(),
            )
            .expect("an entry");
            let mut shot = serde_json::json!({
                "shot_id": format!("{SESSION}-1"), "session_id": SESSION,
                "timestamp": "2026-11-04T20:00:00-05:00", "source": "screen",
                "total_distance": 140.0, "carry_distance": 128.1, "club_head_speed": 86.9,
                "ball_speed": 95.0, "club_path": -2.0, "club_face_angle": 1.5,
                "launch_direction": -0.8, "launch_angle": 19.0, "apex_height": 85.2 / 3.0,
                "smash_factor": 2.08, "backspin": 5000.0,
                "provenance": {
                    "device": "full_swing", "parse_confidence": 0.7, "needs_review": true,
                    "warnings": ["Smash Factor: 2.08 is not ball speed over club speed"],
                    "image_sha256": digest, "image_path": format!("sessions/{SESSION}/1/photo.jpg"),
                    "raw_fields": {"Carry Yd": "128.1"}, "parser_version": 1,
                    "fields_present": [
                        "total_distance", "carry_distance", "club_head_speed", "ball_speed",
                        "club_path", "club_face_angle", "launch_direction", "launch_angle",
                        "apex_height", "smash_factor", "backspin", "sidespin",
                    ],
                },
            });
            change(&mut shot);
            let shot: ShotData = serde_json::from_value(shot).expect("a shot");
            ShotStore::new(&dirs.shots).put(&shot).expect("stored");
            let mut duplicates = Duplicates::new(&dirs.sessions, versions());
            assert!(matches!(
                analyse_entry(&manifest, &dirs, &mut duplicates),
                Outcome::Written(_)
            ));
            Entry { root, dirs, digest }
        }

        fn correct(&self, edits: &[ShotEdit]) -> Result<ShotData, String> {
            correct_shot(&self.dirs, SESSION, "1", edits, versions(), later())
        }

        fn stored(&self) -> ShotData {
            ShotStore::new(&self.dirs.shots)
                .get(&self.digest)
                .expect("a readable shot")
                .expect("a stored shot")
        }

        fn corrections(&self) -> Vec<(String, String)> {
            let shot = self.stored();
            let provenance = shot.provenance.expect("a read's provenance");
            provenance.corrections.into_iter().collect()
        }

        fn result(&self) -> ShotResult {
            load_shot_analysis(&self.dirs.sessions.join(SESSION).join("1")).expect("an analysis")
        }

        fn manifest(&self) -> SwingManifest {
            SwingBundleStore::new(&self.dirs.sessions)
                .get_swing(SESSION, "1")
                .expect("the entry")
        }

        /// Every file under the root and its bytes, for "nothing written" and "no byte moved".
        fn files(&self) -> BTreeMap<PathBuf, Vec<u8>> {
            fn walk(dir: &Path, into: &mut BTreeMap<PathBuf, Vec<u8>>) {
                for entry in fs::read_dir(dir).expect("a directory") {
                    let path = entry.expect("an entry").path();
                    if path.is_dir() {
                        walk(&path, into);
                    } else {
                        let bytes = fs::read(&path).expect("a file");
                        into.insert(path, bytes);
                    }
                }
            }
            let mut files = BTreeMap::new();
            walk(&self.root.0, &mut files);
            files
        }

        /// Rewrite the entry's manifest on disk.
        fn edit_manifest(&self, change: impl FnOnce(&mut Value)) {
            let path = self
                .dirs
                .sessions
                .join(SESSION)
                .join("1")
                .join(MANIFEST_NAME);
            let mut manifest: Value =
                serde_json::from_slice(&fs::read(&path).expect("a manifest")).expect("JSON");
            change(&mut manifest);
            fs::write(&path, manifest.to_string()).expect("written");
        }
    }

    fn versions() -> EngineVersions {
        EngineVersions {
            installed: ANALYSIS_VERSION,
            comparable_from: COMPARABLE_FROM,
        }
    }

    fn later() -> Timestamp {
        Timestamp::parse("2026-11-04T20:30:00+00:00").expect("a timestamp")
    }

    fn typed(field: &str, printed: &str) -> ShotEdit {
        ShotEdit {
            field: field.to_string(),
            printed: Some(printed.to_string()),
            letter: None,
        }
    }

    fn lit(field: &str, printed: &str, letter: &str) -> ShotEdit {
        ShotEdit {
            letter: Some(letter.to_string()),
            ..typed(field, printed)
        }
    }

    fn cleared(field: &str) -> ShotEdit {
        ShotEdit {
            field: field.to_string(),
            printed: None,
            letter: None,
        }
    }

    fn entry_of(field: &str, as_read: &str) -> (String, String) {
        (field.to_string(), as_read.to_string())
    }

    fn measured(result: &ShotResult, name: &str) -> Option<f64> {
        result
            .measurements
            .iter()
            .find(|measurement| measurement.name == name)
            .map(|measurement| measurement.value)
    }

    /// C11: the map keeps what the reader said, however many times a person changes their mind.
    #[test]
    fn the_first_edit_records_the_value_as_read_and_a_second_leaves_it() {
        let entry = Entry::new("first");
        let answered = entry
            .correct(&[typed("carry_distance", "130.5")])
            .expect("an edit");
        assert_eq!(answered, entry.stored());
        assert_eq!(answered.carry_distance, Some(130.5));
        assert_eq!(entry.corrections(), [entry_of("carry_distance", "128.1")]);

        let answered = entry
            .correct(&[typed("carry_distance", "131")])
            .expect("a second edit");
        assert_eq!(answered.carry_distance, Some(131.0));
        assert_eq!(entry.corrections(), [entry_of("carry_distance", "128.1")]);
    }

    /// C11: an empty map says every value is the reader's, so a person changing a value back has
    /// to leave it empty, with the entries of other fields untouched.
    #[test]
    fn an_edit_restoring_the_value_as_read_removes_its_entry() {
        let entry = Entry::new("restore");
        entry
            .correct(&[typed("carry_distance", "130.5"), typed("ball_speed", "96")])
            .expect("two edits");
        let answered = entry
            .correct(&[typed("carry_distance", "128.1")])
            .expect("a restore");
        assert_eq!(answered.carry_distance, Some(128.1));
        assert_eq!(entry.corrections(), [entry_of("ball_speed", "95.0")]);
    }

    /// C11's `""`: `IMG_2893`'s withheld sidespin typed in, and then taken out again.
    #[test]
    fn a_value_that_was_not_read_is_recorded_as_empty_and_clearing_it_restores_that() {
        let entry = Entry::new("unread");
        assert_eq!(entry.stored().sidespin, None);
        let answered = entry
            .correct(&[lit("sidespin", "1500", "D")])
            .expect("a value typed in");
        assert_eq!(answered.sidespin, Some(-1500.0));
        assert_eq!(entry.corrections(), [entry_of("sidespin", "")]);

        let answered = entry.correct(&[cleared("sidespin")]).expect("a clear");
        assert_eq!(answered.sidespin, None);
        assert_eq!(entry.corrections(), []);
    }

    /// C11: a person may say no number at all is right, and the analysis then has none (ADR-010 §2).
    #[test]
    fn clearing_a_value_is_allowed_and_nothing_measures_it() {
        let entry = Entry::new("clear");
        assert_eq!(measured(&entry.result(), "carry_distance_yds"), Some(128.1));

        let answered = entry
            .correct(&[cleared("carry_distance")])
            .expect("a clear");

        assert_eq!(answered.carry_distance, None);
        assert_eq!(entry.corrections(), [entry_of("carry_distance", "128.1")]);
        let result = entry.result();
        assert_eq!(measured(&result, "carry_distance_yds"), None);
        assert!(
            result.unscored.iter().any(|u| u.name == "carry_distance"),
            "{:?}",
            result.unscored
        );
    }

    /// C11: saving is the review, with or without an edit, and what the read warned stays.
    #[test]
    fn saving_with_no_edits_clears_the_flag_and_keeps_the_warnings() {
        let entry = Entry::new("no-edits");
        let warned = entry.stored().provenance.unwrap().warnings;
        assert_eq!(warned.len(), 1);
        assert!(entry
            .result()
            .notes
            .iter()
            .any(|n| n.contains("needs review")));

        let answered = entry.correct(&[]).expect("a review");

        let provenance = answered.provenance.expect("a read's provenance");
        assert!(!provenance.needs_review);
        assert_eq!(provenance.warnings, warned);
        assert_eq!(
            provenance.parse_confidence, 0.7,
            "a read's, and not reviewed"
        );
        assert!(provenance.corrections.is_empty());
    }

    /// The Done line's reader of the flag: a flagged shot is held out of auto-mishit and keys no
    /// launch-monitor row, so review has to be what lets it count.
    #[test]
    fn a_review_is_what_the_corpus_then_reads() {
        let entry = Entry::new("corpus");
        let corpus = |entry: &Entry| read_corpus(&entry.dirs.sessions, "aaron", versions());
        assert!(corpus(&entry).swings[0].shot_needs_review);

        entry
            .correct(&[typed("smash_factor", "1.09")])
            .expect("a review");

        let swings = corpus(&entry).swings;
        assert!(!swings[0].shot_needs_review);
        let carried = swings[0]
            .shot
            .as_ref()
            .expect("the corpus carries the shot");
        assert_eq!(carried.smash_factor, Some(1.09));
    }

    /// The Done line's `shot_analysis.json`, by its shot, its measurements and its notes.
    #[test]
    fn shot_analysis_is_rewritten_to_agree_with_the_shot_store() {
        let entry = Entry::new("agree");
        let answered = entry
            .correct(&[typed("carry_distance", "130.5")])
            .expect("an edit");
        let result = entry.result();
        assert_eq!(result.shot, answered);
        assert_eq!(result.shot, entry.stored());
        assert_eq!(measured(&result, "carry_distance_yds"), Some(130.5));
        assert!(
            !result.notes.iter().any(|n| n.contains("needs review")),
            "{:?}",
            result.notes
        );
    }

    /// C11's "an invalid value", by the user's answer in P5: `ShotData` bounds no reading, so what
    /// is invalid is what is not a finite number. The last two are why it is text: `double.tryParse`
    /// gives `NaN` and `Infinity` for them. The whole call is refused, the valid edit with it.
    #[test]
    fn what_is_not_a_finite_number_is_refused_with_nothing_written() {
        let entry = Entry::new("invalid");
        let before = entry.files();
        for text in [
            "8S.2", "", "  ", "85.2 ft", "85,2", "1e999", "-1e999", "inf", "-inf", "Infinity",
            "NaN",
        ] {
            let refused = entry
                .correct(&[typed("ball_speed", "96"), typed("carry_distance", text)])
                .expect_err(text);
            assert!(refused.contains("Carry Yd"), "{refused}");
            assert!(refused.contains(&pyfmt::str_repr(text)), "{refused}");
            assert_eq!(entry.files(), before, "{text:?} wrote something");
        }
    }

    /// `ShotData`'s own doc: a number that looks wrong is evidence and not a refusal, and a person has
    /// overruled the checks that would say so. So nothing here asks for a range.
    #[test]
    fn any_real_number_stands_however_unlikely() {
        let entry = Entry::new("real");
        for (text, value) in [
            ("-5", -5.0),
            ("1e6", 1e6),
            ("0", 0.0),
            (" 130.5 ", 130.5),
            (".5", 0.5),
            ("+7", 7.0),
        ] {
            let answered = entry.correct(&[typed("carry_distance", text)]).expect(text);
            assert_eq!(answered.carry_distance, Some(value), "{text:?}");
        }
    }

    /// C11: only what the device's screen prints. The impact offset is declared and off the screen,
    /// and the rest are not declared at all, `spin_rate` being what a Full Swing shot does not hold.
    #[test]
    fn a_field_off_the_screen_is_refused_by_name_with_nothing_written() {
        let entry = Entry::new("off-screen");
        let before = entry.files();
        for field in [
            "impact_offset_h",
            "spin_rate",
            "shot_id",
            "provenance",
            "needs_review",
            "",
        ] {
            let refused = entry.correct(&[typed(field, "1")]).expect_err(field);
            assert!(refused.contains(&pyfmt::str_repr(field)), "{refused}");
            assert_eq!(entry.files(), before, "{field:?} wrote something");
        }
    }

    /// P6 finding 5, settled: a person reads `85.2` off the photo, and the parser's division stores it
    /// (`PrintedUnit::to_yards` says why that is not `28.4`). The map stays in `ShotData`'s units.
    #[test]
    fn a_number_is_typed_in_the_screens_unit_and_stored_in_the_shots() {
        let entry = Entry::new("feet");
        let answered = entry
            .correct(&[typed("apex_height", "90")])
            .expect("an edit");
        assert_eq!(answered.apex_height, Some(30.0));
        assert_eq!(
            entry.corrections(),
            [entry_of("apex_height", &pyfmt::repr(85.2 / 3.0))]
        );

        let answered = entry
            .correct(&[typed("apex_height", "85.2")])
            .expect("a restore");
        assert_eq!(answered.apex_height, Some(85.2 / 3.0));
        assert_eq!(entry.corrections(), []);
    }

    /// P6 finding 5, settled: the letter gives the sign, as a lit one does on a read.
    #[test]
    fn a_lit_tile_takes_its_sign_from_the_letter() {
        let entry = Entry::new("letter");
        let direction = |edit: ShotEdit| entry.correct(&[edit]).map(|shot| shot.launch_direction);
        assert_eq!(
            direction(lit("launch_direction", "2.2", "R")),
            Ok(Some(2.2))
        );
        assert_eq!(
            direction(lit("launch_direction", "2.2", "L")),
            Ok(Some(-2.2))
        );
        assert_eq!(entry.corrections(), [entry_of("launch_direction", "-0.8")]);
        assert_eq!(
            direction(lit("launch_direction", "0.8", "L")),
            Ok(Some(-0.8))
        );
        assert_eq!(entry.corrections(), []);
    }

    /// The parser's own rule: zero needs no sign, and it is `+0.0` whatever is lit or typed, so it
    /// cannot read as a change against a read of the same zero.
    #[test]
    fn zero_on_a_lit_tile_needs_no_letter_and_takes_no_sign() {
        let entry = Entry::new("zero");
        for edit in [
            typed("launch_direction", "0"),
            lit("launch_direction", "0", "L"),
            typed("launch_direction", "-0"),
        ] {
            let held = entry.correct(&[edit]).expect("a zero").launch_direction;
            assert_eq!(held.map(f64::to_bits), Some(0.0_f64.to_bits()));
        }
    }

    /// A number on a signed field with no sign is a wrong one half the time (ADR-010 §2), and a typed
    /// `-` is something a person meant, so neither is quietly taken for the other.
    #[test]
    fn a_lit_tile_refuses_what_gives_it_no_sign_and_a_plain_tile_refuses_a_letter() {
        let entry = Entry::new("no-sign");
        let before = entry.files();
        for (edit, said) in [
            (typed("launch_direction", "2.2"), "L or R"),
            (lit("launch_direction", "2.2", "F"), "L or R"),
            (lit("launch_direction", "2.2", "r"), "L or R"),
            (lit("launch_direction", "-2.2", "L"), "no sign of its own"),
            (typed("sidespin", "1500"), "D or F"),
            (lit("carry_distance", "130", "L"), "lights no letter"),
        ] {
            let refused = entry.correct(&[edit]).expect_err(said);
            assert!(refused.contains(said), "{refused}");
            assert_eq!(entry.files(), before, "{said} wrote something");
        }
        // A clear has no number to sign, so a sign control left on a letter says nothing.
        let answered = entry
            .correct(&[lit_cleared("launch_direction", "L")])
            .expect("a clear");
        assert_eq!(answered.launch_direction, None);
    }

    fn lit_cleared(field: &str, letter: &str) -> ShotEdit {
        ShotEdit {
            letter: Some(letter.to_string()),
            ..cleared(field)
        }
    }

    /// `analysed_view` blanks a reading outside `fields_present`, so without the addition the number
    /// a person typed would be stored and silently ignored, which is what the first assert below sees.
    #[test]
    fn a_value_typed_for_a_tile_the_parse_did_not_locate_reaches_the_analysis() {
        let entry = Entry::with("unlocated", |shot| {
            shot["carry_distance"] = Value::Null;
            let present = shot["provenance"]["fields_present"].as_array_mut().unwrap();
            present.retain(|field| field != "carry_distance");
        });
        assert_eq!(measured(&entry.result(), "carry_distance_yds"), None);

        let answered = entry
            .correct(&[typed("carry_distance", "130.5")])
            .expect("a value typed in");

        assert_eq!(measured(&entry.result(), "carry_distance_yds"), Some(130.5));
        let present = answered.provenance.unwrap().fields_present.unwrap();
        assert_eq!(
            present
                .iter()
                .filter(|field| *field == "carry_distance")
                .count(),
            1
        );

        // A second edit adds no second name, and a tile already listed is left where the parser put it.
        let again = entry
            .correct(&[typed("carry_distance", "131")])
            .expect("an edit");
        assert_eq!(again.provenance.unwrap().fields_present.unwrap(), present);
    }

    /// Frozen Python's unstamped shots hold no list, and `printed_on` reads a held value as printed,
    /// so there is nothing to add to and inventing one would name every other tile not printed.
    #[test]
    fn an_unstamped_shot_gains_no_list_of_tiles() {
        let entry = Entry::with("unstamped", |shot| {
            shot["provenance"]["fields_present"] = Value::Null;
        });
        let answered = entry
            .correct(&[typed("backspin", "5100")])
            .expect("an edit");
        assert_eq!(answered.provenance.unwrap().fields_present, None);
    }

    /// An unflagged card can be opened and saved. Bytes unmoved is what keeps a re-import at the lab
    /// reading the entry as identical, so this holds for a number typed back as read, too.
    #[test]
    fn saving_with_nothing_to_change_moves_no_byte() {
        let entry = Entry::with("unchanged", |shot| {
            shot["provenance"]["needs_review"] = Value::Bool(false);
        });
        let before = entry.files();
        entry.correct(&[]).expect("a review");
        assert_eq!(entry.files(), before);
        entry
            .correct(&[
                typed("carry_distance", "128.1"),
                lit("launch_direction", "0.8", "L"),
            ])
            .expect("a review typing back what was read");
        assert_eq!(entry.files(), before);
    }

    /// A review is the entry's change, so it is dated as the repairs are, and it is no other change:
    /// the goal this entry was hit with survives it, which is `set_intent`'s to get wrong.
    #[test]
    fn a_change_stamps_the_manifest_and_moves_nothing_else_on_it() {
        let entry = Entry::new("stamp");
        let drill = PracticeGoal {
            mode: PracticeMode::Drill,
            ..PracticeGoal::default()
        };
        SwingBundleStore::new(&entry.dirs.sessions)
            .set_intent(SESSION, "1", Some(drill), opened_at())
            .expect("a goal");
        let before = entry.manifest();
        entry
            .correct(&[])
            .expect("a flagged shot's review changes it");
        let after = entry.manifest();
        assert_eq!(after.updated_at, later());
        assert_eq!(
            SwingManifest {
                updated_at: before.updated_at,
                ..after
            },
            before
        );
    }

    /// Unsafe ids reach no path, and what is not a counted photo entry is refused before anything is
    /// written, in the sentences `run-shot` uses for the same two manifests.
    #[test]
    fn what_is_not_a_photo_entry_holding_a_shot_is_refused_with_nothing_written() {
        let entry = Entry::new("refused");
        let attempt = |session: &str, swing: &str| {
            correct_shot(&entry.dirs, session, swing, &[], versions(), later()).expect_err(swing)
        };
        assert_eq!(attempt(SESSION, "9"), format!("no swing {SESSION}/9"));
        assert_eq!(attempt("other", "1"), "no swing other/1");
        for (session, swing, said) in [
            (SESSION, "..", "swing id '..'"),
            (SESSION, "a/b", "swing id 'a/b'"),
            ("..", "1", "session id '..'"),
        ] {
            let refused = attempt(session, swing);
            assert!(refused.starts_with(said), "{refused}");
        }

        let photo = entry.manifest().roles[&Role::ShotScreen].clone();
        let as_read = serde_json::to_value(&photo).unwrap();
        entry.edit_manifest(|manifest| manifest["roles"]["face_on"] = as_read);
        let before = entry.files();
        assert_eq!(
            attempt(SESSION, "1"),
            format!("{SESSION}/1 is {SWING_SENTENCE}")
        );
        assert_eq!(entry.files(), before);

        entry.edit_manifest(|manifest| {
            let roles = manifest["roles"].as_object_mut().unwrap();
            roles.remove("face_on");
            roles.remove("shot_screen");
        });
        let before = entry.files();
        assert_eq!(
            attempt(SESSION, "1"),
            format!("{SESSION}/1 has {NO_PHOTO_SENTENCE}")
        );
        assert_eq!(entry.files(), before);
    }

    /// The shot's file is gone, as P9's entry with its reason could find it.
    #[test]
    fn an_entry_whose_shot_the_store_does_not_hold_is_refused() {
        let entry = Entry::new("no-shot");
        fs::remove_dir_all(&entry.dirs.shots).expect("removed");
        let refused = entry.correct(&[]).expect_err("no shot");
        assert!(refused.contains("no shot"), "{refused}");
    }

    /// A second entry holding one photo shares the first's record in the store, so correcting through
    /// it would change the shot the corpus counts, under an entry the corpus leaves out.
    #[test]
    fn an_entry_the_corpus_counts_as_a_duplicate_is_refused_with_nothing_written() {
        let entry = Entry::new("duplicate");
        let store = SwingBundleStore::new(&entry.dirs.sessions);
        let second = open_photo_entry(
            &store,
            &new_entry(&entry.digest, SystemTime::UNIX_EPOCH),
            opened_at(),
        )
        .expect("a second entry");
        assert_eq!(second.swing_id, "2");
        let before = entry.files();
        let refused = correct_shot(&entry.dirs, SESSION, "2", &[], versions(), later())
            .expect_err("a duplicate");
        assert!(
            refused.starts_with(&format!("{SESSION}/2 is not the entry")),
            "{refused}"
        );
        assert_eq!(entry.files(), before);
    }

    /// `shot_analysis.json`'s path held by a directory is a write that fails after the store's has
    /// succeeded, and the store then has to go back, or it says what no analysis does.
    #[test]
    fn an_analysis_that_fails_puts_the_shot_back() {
        let entry = Entry::new("put-back");
        let held = entry.stored();
        let analysis = entry
            .dirs
            .sessions
            .join(SESSION)
            .join("1")
            .join(SHOT_ANALYSIS_NAME);
        fs::remove_file(&analysis).expect("removed");
        fs::create_dir_all(analysis.join("in-the-way")).expect("a directory in its place");

        let refused = entry
            .correct(&[typed("carry_distance", "130.5")])
            .expect_err("no analysis");

        assert!(refused.contains("could not be analysed"), "{refused}");
        assert_eq!(entry.stored(), held);
    }

    /// C11's "fails `ShotData`'s `Validate`". No number can (the struct's doc), and a stored shot that
    /// does not validate cannot be read at all, so only this in-memory shot reaches the clause. It
    /// is here for the day a bound is added: an edit then gets `Validate`'s own message.
    #[test]
    fn what_validate_refuses_is_refused_with_its_message() {
        let mut shot = Entry::new("validate").stored();
        shot.provenance.as_mut().unwrap().parse_confidence = 1.5;
        let refused = with_edits(&shot, &[typed("carry_distance", "130.5")]).expect_err("invalid");
        assert!(
            refused.contains("ShotProvenance.parse_confidence"),
            "{refused}"
        );
    }

    /// A direct feed was never a photo, so there is no read for a person to overrule.
    #[test]
    fn a_shot_no_screen_read_is_refused() {
        let mut shot = Entry::new("direct").stored();
        shot.provenance = None;
        let refused = with_edits(&shot, &[]).expect_err("no provenance");
        assert!(refused.contains("no provenance"), "{refused}");
    }

    /// `hd_golf` is not offered on the phone (D3), and its tiles say what a person could not: which
    /// word lit, and a polarity. Its plain numbers correct as any do, so the rule is the tile's.
    #[test]
    fn a_tile_of_words_or_signed_by_a_word_is_refused_and_a_plain_one_is_not() {
        let mut shot = Entry::new("hd-golf").stored();
        shot.provenance.as_mut().unwrap().device = "hd_golf".to_string();
        for (field, said) in [
            ("shot_type", "holds words"),
            ("impact_position", "holds words"),
            ("club_path", "signed by a word"),
            ("spin_axis", "signed by a word"),
        ] {
            let refused = with_edits(&shot, &[typed(field, "1")]).expect_err(field);
            assert!(refused.contains(said), "{field}: {refused}");
            assert!(refused.contains(&pyfmt::str_repr(field)), "{refused}");
        }
        let corrected = with_edits(&shot, &[typed("carry_distance", "130.5")]).expect("a number");
        assert_eq!(corrected.carry_distance, Some(130.5));
    }
}
