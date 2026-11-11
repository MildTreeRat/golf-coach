//! `.app/state.json`, the app's own settings, and where the facade's plain enums meet `contracts`'.
//! [M38 P5]
//!
//! Outside `api` because the codegen binds what is `pub` there (rule 7), and it would try to bind a
//! trait impl as well. So `api::setup` holds the shapes Dart sees, and this holds what they do.
//!
//! **This file stays home** (C7): an export carries the read diagnostics and not this, so nothing
//! outside the phone ever reads it, and no version key or vector stands behind its shape.
//!
//! # Why a read never refuses
//!
//! An unreadable or missing file reads as the defaults, the stores' rule: a corrupt file should cost
//! choosing a club again, not an error on the screen someone is holding between swings. A key the
//! file lacks takes its default (`#[serde(default)]` on [`AppState`]), so a later build's new setting
//! does not cost the older ones. What that costs is the `install_id`, and the next `init_state` mints
//! another, which only names later sessions differently (C8).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use contracts::golfer::Handedness as ContractHandedness;
use contracts::intent::{PracticeMode as ContractMode, TargetShape as ContractShape};

use crate::api::setup::{AppState, Appearance, Handedness, PracticeMode, TargetShape};
use crate::Dirs;

/// C7's name, under [`Dirs::app`].
const STATE_NAME: &str = "state.json";

/// One writer at a time, in this process.
///
/// **Nothing serializes bridge calls** (the M38 plan's P3 finding 4), and bridged functions are async
/// in Dart, so a change sheet saving twice can overlap. Each write here also reads, to keep the
/// `install_id`, and two writers sharing one `.tmp` could rename a torn file into place, which reads
/// as the defaults and loses the id. A lock costs nothing at this size. A unique temporary name
/// would fix only the second of those.
static WRITER: Mutex<()> = Mutex::new(());

fn writer() -> MutexGuard<'static, ()> {
    // A panic elsewhere holding this left no half-made value behind it, since the file is renamed
    // into place whole, so the lock is still good.
    WRITER.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The first run's settings: no id, no golfer, no club, fundamentals, dark (D13).
///
/// `Default` is here and not derived in `api`, because the codegen may bind a `Default` it can see
/// as one more function Dart can call.
impl Default for AppState {
    fn default() -> Self {
        AppState {
            install_id: None,
            player_id: None,
            club: None,
            mode: PracticeMode::Fundamentals,
            target_shape: None,
            challenge: false,
            challenge_shapes: Vec::new(),
            appearance: Appearance::Dark,
            last_export_at: None,
            last_export_shots: None,
        }
    }
}

fn path_under(root: &Path) -> PathBuf {
    Dirs::of(root).app.join(STATE_NAME)
}

/// What is stored, or the defaults (the module doc).
pub(crate) fn read(root: &Path) -> AppState {
    fs::read(path_under(root))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// A `.tmp` beside it and a rename, as every store here writes. `storage::write_atomically` is that
/// crate's own, and this file is not a store's.
fn write(root: &Path, state: &AppState) -> io::Result<()> {
    let path = path_under(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(state).map_err(io::Error::other)?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, &path)
}

/// C8's `install`: 4 lowercase hex digits. The id goes into every session's name for good and no
/// verb renames one, so a wrong one is refused here rather than by the store at the first shot.
fn is_install_id(text: &str) -> bool {
    text.len() == 4 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Store `install_id` if the file holds none, and answer what is stored, which is the older id if
/// there is one. Nothing is written then, so a relaunch cannot disturb a file it has no reason to.
pub(crate) fn init(root: &Path, install_id: &str) -> Result<AppState, String> {
    if !is_install_id(install_id) {
        return Err(format!(
            "'{install_id}' is not an install id: 4 lowercase hex digits"
        ));
    }
    let _writer = writer();
    let state = read(root);
    if state.install_id.is_some() {
        return Ok(state);
    }
    let state = AppState {
        install_id: Some(install_id.to_string()),
        ..state
    };
    write(root, &state).map_err(|error| error.to_string())?;
    Ok(state)
}

/// Store `state`, with the `install_id` the file already holds, whatever `state` says. Answers what
/// was stored.
///
/// **[`init`] is the one writer of the id.** It names every session this phone makes, so a stale or
/// default copy in Dart's model must not move it or clear it.
pub(crate) fn replace(root: &Path, state: AppState) -> Result<AppState, String> {
    let _writer = writer();
    let state = AppState {
        install_id: read(root).install_id,
        ..state
    };
    write(root, &state).map_err(|error| error.to_string())?;
    Ok(state)
}

impl From<Handedness> for ContractHandedness {
    fn from(handedness: Handedness) -> Self {
        match handedness {
            Handedness::Right => ContractHandedness::Right,
            Handedness::Left => ContractHandedness::Left,
        }
    }
}

impl From<ContractHandedness> for Handedness {
    fn from(handedness: ContractHandedness) -> Self {
        match handedness {
            ContractHandedness::Right => Handedness::Right,
            ContractHandedness::Left => Handedness::Left,
        }
    }
}

impl From<PracticeMode> for ContractMode {
    fn from(mode: PracticeMode) -> Self {
        match mode {
            PracticeMode::Fundamentals => ContractMode::Fundamentals,
            PracticeMode::ShotShaping => ContractMode::ShotShaping,
            PracticeMode::Performance => ContractMode::Performance,
            PracticeMode::Drill => ContractMode::Drill,
        }
    }
}

impl From<ContractMode> for PracticeMode {
    fn from(mode: ContractMode) -> Self {
        match mode {
            ContractMode::Fundamentals => PracticeMode::Fundamentals,
            ContractMode::ShotShaping => PracticeMode::ShotShaping,
            ContractMode::Performance => PracticeMode::Performance,
            ContractMode::Drill => PracticeMode::Drill,
        }
    }
}

impl From<TargetShape> for ContractShape {
    fn from(shape: TargetShape) -> Self {
        match shape {
            TargetShape::Straight => ContractShape::Straight,
            TargetShape::Draw => ContractShape::Draw,
            TargetShape::Fade => ContractShape::Fade,
        }
    }
}

impl From<ContractShape> for TargetShape {
    fn from(shape: ContractShape) -> Self {
        match shape {
            ContractShape::Straight => TargetShape::Straight,
            ContractShape::Draw => TargetShape::Draw,
            ContractShape::Fade => TargetShape::Fade,
        }
    }
}
