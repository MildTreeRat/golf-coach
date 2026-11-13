//! The impact drill's history on disk: every strike, each club's place on the ladder, each strike's
//! crop and each club's last crop. [M46 P17]
//!
//! **No Python twin**, as `contracts::drill`, whose shapes this writes, has none: the drill was
//! designed after the pivot (ADR-034), and frozen Python never gains it (ADR-035 clause 4). Nor is
//! there a vector. The store writes nothing a second implementation reproduces, so its tests on a
//! scratch root are its gate (`crates/storage/tests/drill_store.rs`).
//!
//! # The layout (the M46 plan's C7)
//!
//! ```text
//! <root>/drills/<player_id>/impact/strike_tape/
//!   strikes.jsonl          one TapeStrike per line, append-only
//!   state.json             the DrillState: each club's stage, level, block, suggestion and history
//!   crops/<strike_id>.jpg  the strike's rectified label crop, as JPEG (D6)
//!   last/<club>.rgb        the club's last crop as raw RGB, the diff's reference (C16)
//! ```
//!
//! `<root>` is the data root, where `sessions/`, `shots/` and `golfers/` are, so the drill sits
//! **beside the corpus and never inside it**. The corpus reader walks `sessions/`, and nothing here
//! is in it: a drill is untracked (ADR-034 §3, the plan's rule 5). The path's last two segments are
//! [`DrillTopic::as_str`] and [`TapeMethod::as_str`], so a directory and its wire name cannot drift,
//! and M44's `low_point/foot_powder/` will sit beside this one. A crop's name is
//! [`crop_file`]'s, the same text a [`TapeStrike::crop`] holds, so the record and the file cannot
//! name two places.
//!
//! # Append-only, and tolerant of a torn line
//!
//! A strike is one line of `strikes.jsonl`, appended and never rewritten. Making the append atomic
//! by rewriting the whole file through a rename would put every strike already held at risk in
//! order to store one more. The cost is that a write cut off mid-line leaves a torn last line. A
//! read skips it and counts it in [`StrikeLog::skipped`], as the shot store's `all` skips a file it
//! cannot read. The next append starts a fresh line first, so the fragment stays a line of its own
//! and never swallows the strike after it.
//!
//! A read also skips and counts a line the contract refuses, a strike of another golfer, and a
//! second line holding an id it has already read. The first line with an id is the one kept, as the
//! lab's merge keeps a held id (C17). None of those is written here, so a count above zero means a
//! torn write, a hand edit or a merge, and it is the caller's to say so.
//!
//! **A strike id already held is refused.** The id names the strike's crop, and every block and
//! level entry in the state names its strikes by it, so a second strike under one id would make
//! one of the two unreachable. The facade draws ids from the state's counter (C8), so a refusal
//! here means the state and the strikes disagree, and refusing is what stops that disagreement
//! from spreading.
//!
//! # The state is read strictly, and never replaced unread
//!
//! `state.json` is written whole through `write_atomically`. **Unlike the bag store's `get`, its
//! reader is not tolerant**: a file that does not read as a [`DrillState`] is
//! [`StoreError::Unreadable`], and [`DrillStore::put_state`] refuses to replace it, with the bag
//! store's write guard and wording. The counters every id is drawn from live in it. Read as absent,
//! the state would restart them at 0 and re-issue ids that the duplicate refusal would then refuse
//! strike after strike, and its next write would replace every club's level and history with a
//! fresh baseline. Since the state is only ever renamed into place, an unreadable one was edited by
//! hand or damaged by the disk, and neither is fixed by writing over it.
//!
//! # The lock
//!
//! C7 says the drill's writes go under "the lock the shot store holds". `shot_store` holds none:
//! on the phone, the shot writes are serialized by the app's `record::one_writer()`, which this
//! crate cannot reach. So the store holds its own for every call, readers included, and that is
//! what makes [`DrillStore::append_strike`]'s check and append one step. The facade holds the
//! app's lock around a whole read, judgment and store (P18), so the export never zips half of one.
//!
//! The lock is **a static, not a field** as `SwingBundleStore`'s is, because the facade opens a store
//! per call, and a lock per store would then serialize nothing. Like the bundle store's, it
//! serializes one process and not two sharing a directory. It is never held across calls, so a
//! caller holding its own lock around several of them cannot deadlock with it. A panic while it is
//! held poisons it, and the next caller takes it anyway, because what it guards is on disk and each
//! write there leaves a whole file or a torn line that a read skips.
//!
//! # Another copy of the files
//!
//! The lab's `golf-lab import-export` merges a phone's drill into its own (C17), and the phone's
//! files arrive as bytes, often out of a zip, with no store around them. [`DrillStore::parse_strikes`]
//! and [`DrillStore::parse_state`] read such bytes by this store's own rules, so that the merge and
//! the store cannot disagree on which lines are strikes or on what a state is. [P21]

use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use contracts::club::ClubId;
use contracts::drill::{crop_file, DrillState, DrillTopic, TapeMethod, TapeStrike};
use contracts::golfer::check_player_id;
use contracts::Validate;
use pyfmt::str_repr;

use crate::StoreError;

/// The directory every golfer's drills sit under, beside `sessions/` (C7).
pub const DRILLS_DIR: &str = "drills";

/// The strikes, one [`TapeStrike`] per line.
pub const STRIKES_NAME: &str = "strikes.jsonl";

/// The [`DrillState`].
pub const STATE_NAME: &str = "state.json";

/// The directory of each club's last crop.
pub const LAST_DIR: &str = "last";

/// A last crop's suffix: raw RGB, three bytes a pixel and no header (C16).
pub const LAST_SUFFIX: &str = ".rgb";

/// One call here at a time, in this process (the module doc's "The lock").
static STORING: Mutex<()> = Mutex::new(());

fn one_at_a_time() -> MutexGuard<'static, ()> {
    STORING.lock().unwrap_or_else(PoisonError::into_inner)
}

/// What a read of `strikes.jsonl` found.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StrikeLog {
    /// The strikes, in the order they were appended, which is the order they were made.
    pub strikes: Vec<TapeStrike>,
    /// The lines skipped: torn, refused by the contract, another golfer's, or an id already read
    /// (the module doc). **Counted over the whole file**, even when the strikes are one club's,
    /// because a torn line names no club.
    pub skipped: usize,
}

/// One golfer's impact drill by tape, under `<root>/drills/<player_id>/impact/strike_tape/`.
#[derive(Debug, Clone)]
pub struct DrillStore {
    player_id: String,
    dir: PathBuf,
}

impl DrillStore {
    /// The drill of `player_id` under the data root `root`. Nothing is created until a write.
    ///
    /// **The id must be a slug**, because it names a directory: `../aaron` would put a drill outside
    /// `drills/`. The refusal is [`check_player_id`]'s, the one every store's contract makes.
    pub fn open(root: impl AsRef<Path>, player_id: &str) -> Result<Self, StoreError> {
        Self::in_drills(root.as_ref().join(DRILLS_DIR), player_id)
    }

    /// The same store, named by the `drills/` directory itself rather than the data root above it.
    /// The lab's `import-export` takes each tree's directory as a flag of its own, `--drills-dir`
    /// beside `--sessions-dir`, and the directory it names need not be called `drills`. [P21]
    pub fn in_drills(drills_dir: impl AsRef<Path>, player_id: &str) -> Result<Self, StoreError> {
        check_player_id("DrillStore", player_id).map_err(StoreError::Invalid)?;
        let dir = drills_dir
            .as_ref()
            .join(player_id)
            .join(DrillTopic::Impact.as_str())
            .join(TapeMethod::StrikeTape.as_str());
        Ok(DrillStore {
            player_id: player_id.to_string(),
            dir,
        })
    }

    pub fn player_id(&self) -> &str {
        &self.player_id
    }

    /// The method's directory, which every path below is inside.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn strikes_path(&self) -> PathBuf {
        self.dir.join(STRIKES_NAME)
    }

    pub fn state_path(&self) -> PathBuf {
        self.dir.join(STATE_NAME)
    }

    /// Where a strike's crop is: [`crop_file`] joined to the method's directory, one segment at a
    /// time so that the path is spelled with the platform's separator.
    pub fn crop_path(&self, strike_id: &str) -> PathBuf {
        crop_file(strike_id)
            .split('/')
            .fold(self.dir.clone(), |path, segment| path.join(segment))
    }

    /// Where a club's last crop is, `last/<club>.rgb`, by the club's wire name.
    pub fn last_path(&self, club: ClubId) -> PathBuf {
        self.dir
            .join(LAST_DIR)
            .join(format!("{}{LAST_SUFFIX}", club.as_str()))
    }

    /// Append one strike, as the last line of `strikes.jsonl`.
    ///
    /// Refused, with nothing written: a strike the contract refuses ([`StoreError::Invalid`]), as
    /// the bag store's `save` refuses a bag, since a line every reader skips is a strike lost;
    /// another golfer's strike; an id that is not a file name, since it names the strike's crop;
    /// and an id the file already holds (the module doc says why).
    pub fn append_strike(&self, strike: &TapeStrike) -> Result<(), StoreError> {
        strike.validate().map_err(StoreError::Invalid)?;
        self.check_owner(&strike.player_id, "strike")?;
        check_file_name(&strike.strike_id)?;
        let _held = one_at_a_time();
        let path = self.strikes_path();
        let held = read_or_empty(&path)?;
        if self
            .parse_strikes(&held)
            .strikes
            .iter()
            .any(|stored| stored.strike_id == strike.strike_id)
        {
            return Err(StoreError::Refused(format!(
                "strike {} is already stored; refusing to store it twice",
                str_repr(&strike.strike_id)
            )));
        }
        let mut line = String::new();
        if held.last().is_some_and(|byte| *byte != b'\n') {
            // The torn line the module doc describes: end it, so that it stays a line of its own.
            line.push('\n');
        }
        line.push_str(&serde_json::to_string(strike).expect("a strike serializes"));
        line.push('\n');
        fs::create_dir_all(&self.dir)?;
        fs::File::options()
            .create(true)
            .append(true)
            .open(&path)?
            .write_all(line.as_bytes())?;
        Ok(())
    }

    /// One club's strikes, in the order they were made, and the lines of the whole file skipped.
    /// No file is an empty log.
    pub fn strikes(&self, club: ClubId) -> Result<StrikeLog, StoreError> {
        let mut log = self.all_strikes()?;
        log.strikes.retain(|strike| strike.club == club);
        Ok(log)
    }

    /// Every club's strikes, in the order they were made: a category's roll-up and the lab's
    /// merge read them all.
    pub fn all_strikes(&self) -> Result<StrikeLog, StoreError> {
        let _held = one_at_a_time();
        Ok(self.parse_strikes(&read_or_empty(&self.strikes_path())?))
    }

    /// The drill's state, or `None` before the first was written. A file that is not this golfer's
    /// [`DrillState`] is [`StoreError::Unreadable`] (the module doc says why it is not tolerated).
    pub fn state(&self) -> Result<Option<DrillState>, StoreError> {
        let _held = one_at_a_time();
        self.read_state()
    }

    /// Write the state whole, atomically.
    ///
    /// Refused, with nothing written: a state the contract refuses, another golfer's, and any write
    /// over a `state.json` that does not read (the module doc says why).
    pub fn put_state(&self, state: &DrillState) -> Result<(), StoreError> {
        state.validate().map_err(StoreError::Invalid)?;
        self.check_owner(&state.player_id, "state")?;
        let _held = one_at_a_time();
        match self.read_state() {
            Ok(_) => {}
            Err(StoreError::Unreadable { path, .. }) => {
                return Err(StoreError::Refused(format!(
                    "drill state at {} exists but cannot be read; refusing to overwrite it",
                    path.display()
                )))
            }
            Err(other) => return Err(other),
        }
        fs::create_dir_all(&self.dir)?;
        crate::write_atomically(state, &self.state_path())?;
        Ok(())
    }

    /// Write a strike's crop, atomically, and answer where it went. The bytes are the caller's JPEG,
    /// taken as given. The id is held to the file-name rule [`Self::append_strike`] holds it to.
    pub fn put_crop(&self, strike_id: &str, jpeg: &[u8]) -> Result<PathBuf, StoreError> {
        check_file_name(strike_id)?;
        let path = self.crop_path(strike_id);
        let _held = one_at_a_time();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        crate::write_bytes_atomically(jpeg, &path)?;
        Ok(path)
    }

    /// The club's last crop, as the raw RGB bytes it was written with, or `None` when the club has
    /// none yet.
    ///
    /// **The bytes carry no size.** The crop's grid is the label type's at the reader's pixels per
    /// millimetre, which the caller knows, and a reference of another size is one the reader's diff
    /// refuses into a pick, never a reading (`impact::tape`'s `DiffMiss`).
    pub fn last(&self, club: ClubId) -> Result<Option<Vec<u8>>, StoreError> {
        let _held = one_at_a_time();
        match fs::read(self.last_path(club)) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    /// Replace the club's last crop, atomically, since a diff against half a reference would show
    /// the missing half as a mark. Which outcomes advance it is the facade's (P12's finding 9).
    pub fn put_last(&self, club: ClubId, rgb: &[u8]) -> Result<(), StoreError> {
        let path = self.last_path(club);
        let _held = one_at_a_time();
        fs::create_dir_all(self.dir.join(LAST_DIR))?;
        crate::write_bytes_atomically(rgb, &path)?;
        Ok(())
    }

    /// Refuse a strike or a state of another golfer, which would be stored where nobody looks for it.
    fn check_owner(&self, player_id: &str, what: &str) -> Result<(), StoreError> {
        if player_id == self.player_id {
            return Ok(());
        }
        Err(StoreError::Refused(format!(
            "a {what} of {} does not belong in the drill of {}",
            str_repr(player_id),
            str_repr(&self.player_id)
        )))
    }

    /// The strikes in a `strikes.jsonl`'s bytes, and how many lines were skipped (the module doc).
    /// An empty or blank line is the file's last newline, or a newline written after a torn line, and
    /// is not counted.
    ///
    /// **Public for another copy of the file** (the module doc's last section): the bytes need not
    /// be this store's, and they are read as if they were, this golfer's strikes and no other's.
    pub fn parse_strikes(&self, bytes: &[u8]) -> StrikeLog {
        let mut log = StrikeLog::default();
        let mut seen = HashSet::new();
        for line in bytes.split(|byte| *byte == b'\n') {
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            match serde_json::from_slice::<TapeStrike>(line) {
                Ok(strike)
                    if strike.player_id == self.player_id
                        && seen.insert(strike.strike_id.clone()) =>
                {
                    log.strikes.push(strike);
                }
                _ => log.skipped += 1,
            }
        }
        log
    }

    /// A `state.json`'s bytes as this golfer's [`DrillState`], or why they are not one: the strict
    /// reader [`Self::state`] runs, public for another copy of the file as
    /// [`Self::parse_strikes`] is.
    pub fn parse_state(&self, bytes: &[u8]) -> Result<DrillState, String> {
        let state: DrillState = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        if state.player_id != self.player_id {
            return Err(format!(
                "it holds the drill of {}, not of {}",
                str_repr(&state.player_id),
                str_repr(&self.player_id)
            ));
        }
        Ok(state)
    }

    /// [`Self::state`] for a caller already holding the lock.
    fn read_state(&self) -> Result<Option<DrillState>, StoreError> {
        let path = self.state_path();
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        self.parse_state(&bytes)
            .map(Some)
            .map_err(|reason| StoreError::Unreadable { path, reason })
    }
}

/// A file's bytes, or none when there is no file.
fn read_or_empty(path: &Path) -> io::Result<Vec<u8>> {
    match fs::read(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        read => read,
    }
}

/// Refuse a strike id that is not a plain file name. The facade's ids are `<install>-<N>`, four hex
/// digits and a count (C8), and a crop is named by the id, so a `/` or a `..` in one would put the
/// crop outside `crops/`.
fn check_file_name(strike_id: &str) -> Result<(), StoreError> {
    let plain = !strike_id.is_empty()
        && strike_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'));
    if plain {
        return Ok(());
    }
    Err(StoreError::Refused(format!(
        "strike id {} is not a file name; refusing to store it",
        str_repr(strike_id)
    )))
}
