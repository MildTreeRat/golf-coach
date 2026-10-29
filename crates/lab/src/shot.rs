//! The launch-monitor shot for one swing, from the shot store when its parse is current and from
//! the bundle's photo when it is not: `api/pipeline.py::_shot_for`, with
//! `launch_monitor/screen/importer.py::import_screen` from the photo's bytes to the stored record.
//! [M29 P23]
//!
//! # The cache rule gains the one question Python cannot ask
//!
//! Python's `_shot_for` serves whatever the store holds under the photo's sha256, because frozen
//! Python has one parser and every stored shot is its read. Rust has a ledger
//! ([`SCREEN_PARSER_VERSION`]) and [`parse_is_current`] reads it, and every shot frozen Python
//! stored is unstamped (`parser_version` 0), so each reads as stale: re-reading its photo could
//! change it, and M34 measured that it does, on two of the thirteen stored photos. So a stored shot
//! is served only when its parse is current, and a stale one is re-read from the bundle's own copy
//! of the photo through [`ocr::read_photo`] and [`screen::read`], which stamps `parser_version` and
//! `fields_present`. This is the lab as the first caller `parse_is_current`'s doc waited for (§M34).
//!
//! **Every other branch is Python's**, with its log lines and notes word for word: no shot-screen
//! role, OCR skipped by request, the photo missing, a machine that cannot read (Python's
//! `build_recognizer` refusing, here the models missing or not the ones pinned), the broad
//! `except` around the read, and a read of nothing. `force_ocr` is Python's too: the store is not
//! consulted, and the photo is read and stamped as a new import.
//!
//! # A re-read keeps the shot's identity
//!
//! A stale shot's `shot_id`, `session_id`, `timestamp` and `image_path` are carried into its
//! re-read, where a new import (a store miss, or `force_ocr`) is stamped as `import_screen` stamps
//! one: `{session}-{swing}`, the photo's modification time, and the path read. The identity is the
//! shot's, set when it was first imported; the parse is the parser's. And the store holds one shot
//! per photo while a photo can sit in several bundles (`2e81ab05828e…` is in three), so stamping a
//! re-read from the bundle that happened to reach it first would rename a shot the other two
//! already attach, and would make the M29 plan's D20 gate, which compares each bundle's shot with
//! the one its corpus vector holds, see a third kind of change it never declared. A photo whose
//! bytes no longer hash to the manifest's sha256 is not the stored shot's photo, and the difference
//! is noted, as `golf_lab::pose` notes a worker's hash. Its shot is then whatever the store holds
//! under the bytes' own digest, which is the second lookup Python's `import_screen` makes: current,
//! it is served as Python's `cached`; stale, it is re-read keeping its identity; absent, the photo
//! is a new import.
//!
//! # When a stale shot cannot be re-read, it is kept, and the note says so
//!
//! Older numbers beat none, as `golf_lab::pose` keeps an older pose run: the shot is an older
//! parser's read of the same photo, and the swing is still worth judging against it. So OCR skipped
//! by request, a missing photo, a machine that cannot read, and a read that raised each keep the
//! stored shot with a note naming the parser that read it and why it was not re-read. **A re-read
//! that reads nothing is the reader's answer**, not a failure to re-read: no shot attaches, with
//! Python's `failed` note, as pose's `clip_empty` attaches no landmarks however old a run is cached.
//! The store keeps the older file either way, because Python's `import_screen` never deletes one.
//!
//! # What is written
//!
//! The re-read, into the store under its photo's sha256 (`ShotStore::put`, the twin of the one
//! write `import_screen` makes), unless the caller asks for a dry run, which writes nothing and
//! answers `dry-run` where Python's `import_screen(dry_run=True)` does. A write that fails is inside
//! Python's broad `except`, so it is that note, and no shot attaches.
//!
//! # The bulk import's half (M29 P24)
//!
//! [`import_photo`] is `import_screen` alone, for `golf-lab import-photos` (`crate::import`): one
//! photo with no bundle around it, so no manifest, no notes and no older shot kept. It shares this
//! module's cache rule (a stale stored shot is re-read keeping its identity) and its one stamp,
//! [`NewImport`], which is the identity a new import takes: `{session}-{swing}` from [`shot_for`],
//! `{session}-{digest[:12]}` from a store-only import.

use std::path::{Path, PathBuf};

use contracts::shot::{parse_is_current, ShotData, SCREEN_PARSER_VERSION};
use contracts::Timestamp;
use ocr::models::Models;
use screen::profile::{load_profile, DeviceProfile};
use screen::{ScreenInput, TextBox};
use storage::manifest::{Role, SwingManifest};
use storage::shot_store::{hash_image, ShotStore};
use storage::StoreError;

/// `settings.launch_monitor_profile`'s environment variable, by pydantic-settings' `GOLF_` prefix.
pub const DEVICE_ENV: &str = "GOLF_LAUNCH_MONITOR_PROFILE";
/// `settings.ocr_min_confidence`'s environment variable.
pub const MIN_CONFIDENCE_ENV: &str = "GOLF_OCR_MIN_CONFIDENCE";
/// `config.py`'s default for `launch_monitor_profile`. `tests::the_defaults_are_config_pys` reads
/// it out of `config.py`.
pub const DEFAULT_DEVICE: &str = "hd_golf";
/// `config.py`'s default for `ocr_min_confidence`: below it a read that passed every check is still
/// flagged for review.
pub const DEFAULT_MIN_CONFIDENCE: f64 = 0.6;

/// What a run may vary about the shot: `PipelineOptions`' `shots_dir`, `min_confidence`,
/// `force_ocr` and `skip_ocr`, `settings.launch_monitor_profile`, and the dry run P23's gate reads
/// the stored bundles with.
#[derive(Debug, Clone, PartialEq)]
pub struct ShotOptions {
    pub shots_dir: PathBuf,
    /// The launch-monitor profile the photo is read with, by device name.
    pub device: String,
    pub min_confidence: f64,
    /// Read the photo even when the store holds a current shot for it, as a new import.
    pub force_ocr: bool,
    /// Never read a photo: attach only what the store holds.
    pub skip_ocr: bool,
    /// Read, but write nothing.
    pub dry_run: bool,
}

impl ShotOptions {
    /// `config.py`'s defaults over `shots_dir`, no flag set.
    pub fn new(shots_dir: impl Into<PathBuf>) -> Self {
        ShotOptions {
            shots_dir: shots_dir.into(),
            device: DEFAULT_DEVICE.to_string(),
            min_confidence: DEFAULT_MIN_CONFIDENCE,
            force_ocr: false,
            skip_ocr: false,
            dry_run: false,
        }
    }

    /// [`Self::new`] with the two settings read from `env` by name, so a test can answer for the
    /// process environment. An empty variable counts as unset, the rule `golf-core`'s `DataDirs`
    /// and `golf_lab::pose` keep, and `.env` is not read, for `golf_lab::pose`'s reason. A
    /// threshold that is not a number is refused, as pydantic refuses it when the settings load.
    pub fn configured(
        shots_dir: impl Into<PathBuf>,
        env: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, String> {
        let mut options = Self::new(shots_dir);
        let set = |name: &str| env(name).filter(|value| !value.is_empty());
        if let Some(device) = set(DEVICE_ENV) {
            options.device = device;
        }
        if let Some(text) = set(MIN_CONFIDENCE_ENV) {
            options.min_confidence = text
                .trim()
                .parse()
                .map_err(|_| format!("{MIN_CONFIDENCE_ENV}={text:?} is not a number"))?;
        }
        Ok(options)
    }
}

/// What reading one photo gives the parser: the boxes, and what preprocessing noticed.
#[derive(Debug, Clone, PartialEq)]
pub struct Prepared {
    pub boxes: Vec<TextBox>,
    pub notes: Vec<String>,
}

/// The pixel half of `import_screen`: `build_recognizer`, then `prepare_screen(load_image(path))`.
///
/// A trait so the lookup's every branch can be driven without the models, as `crates/ocr`'s vote
/// is driven by a recognizer closure; [`OcrReader`] is the one the lab runs.
pub trait PhotoReader {
    /// Whether this machine can read a photo at all, and why not: `build_recognizer`.
    fn ready(&mut self) -> Result<(), String>;
    /// One photo's boxes and notes, read with `profile`.
    fn prepare(&mut self, photo: &Path, profile: &DeviceProfile) -> Result<Prepared, String>;
}

/// The Rust reader: `crates/ocr`'s models, opened on the first photo that needs reading.
///
/// Opening reads and hashes the three ONNX files against `crates/ocr/models.json` (the M29 plan's
/// D6), which costs seconds, so a run whose every shot is current opens nothing, and a refusal is
/// remembered so a second photo does not pay for it again, as `golf_lab::pose::Sidecar` remembers
/// a pool that would not start.
pub struct OcrReader {
    dir: PathBuf,
    models: Option<Result<Models, String>>,
    reads: u64,
}

impl OcrReader {
    /// The reader over the models in `dir`.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        OcrReader {
            dir: dir.into(),
            models: None,
            reads: 0,
        }
    }

    /// The reader over `data/models/ocr/` under the repository, found from this crate's source as
    /// `golf-ocr` finds it.
    pub fn configured() -> Self {
        Self::new(crate::repo_root().join(ocr::DEFAULT_DIR))
    }

    /// Whether the models were opened, so a test can hold a run of current shots to opening none.
    pub fn opened(&self) -> bool {
        self.models.is_some()
    }

    /// How many photos it has read.
    pub fn reads(&self) -> u64 {
        self.reads
    }
}

impl PhotoReader for OcrReader {
    fn ready(&mut self) -> Result<(), String> {
        let dir = &self.dir;
        match self
            .models
            .get_or_insert_with(|| Models::open(dir).map_err(|error| error.to_string()))
        {
            Ok(_) => Ok(()),
            Err(why) => Err(why.clone()),
        }
    }

    fn prepare(&mut self, photo: &Path, profile: &DeviceProfile) -> Result<Prepared, String> {
        self.ready()?;
        let Some(Ok(models)) = self.models.as_mut() else {
            unreachable!("ready() answered Ok, so the models are open");
        };
        self.reads += 1;
        let prepared =
            ocr::read_photo(models, photo, profile).map_err(|error| error.to_string())?;
        Ok(Prepared {
            boxes: prepared.boxes,
            notes: prepared.notes,
        })
    }
}

/// Where the shot [`shot_for`] answered with came from.
#[derive(Debug, Clone, PartialEq)]
pub enum Origin {
    /// Nothing was attached and no photo was read; the note says why.
    Nothing,
    /// The store's shot, current for this parser: under the manifest's digest, or under the
    /// photo's own when its bytes are no longer the manifest's (Python's `cached`).
    Stored,
    /// The store's shot, read by an older parser and kept because its photo could not be re-read;
    /// the note says why.
    KeptOlder,
    /// The photo was read this run, whether or not anything was read from it. `replacing` is the
    /// stale shot the read re-reads, or `None` for a new import (a miss, `force_ocr`, or a photo
    /// whose bytes are no longer the manifest's and have no shot of their own), and `written` the
    /// store file, or `None` on a dry run or a read of nothing.
    Read {
        replacing: Option<Box<ShotData>>,
        written: Option<PathBuf>,
    },
}

/// [`shot_for`]'s answer: Python's `(shot, note_if_absent)`, and where the shot came from.
#[derive(Debug, Clone, PartialEq)]
pub struct ShotFor {
    pub shot: Option<ShotData>,
    /// For the caller to log and keep in `notes`, as `analyze_swing_dir` does with `shot_note`.
    pub note: Option<String>,
    pub origin: Origin,
}

impl ShotFor {
    fn nothing(note: String) -> Self {
        ShotFor {
            shot: None,
            note: Some(note),
            origin: Origin::Nothing,
        }
    }
}

/// The launch-monitor shot for this swing: `_shot_for`, with a stale stored shot re-read.
///
/// `Err` only where Python raises: a stored shot that is not a shot (`ShotStore.get`'s
/// `model_validate_json`), which stops the bundle there too. Everything else is an answer, with a
/// note where Python writes one.
pub fn shot_for(
    swing_dir: &Path,
    manifest: &SwingManifest,
    options: &ShotOptions,
    reader: &mut dyn PhotoReader,
    log: &mut dyn FnMut(&str),
) -> Result<ShotFor, StoreError> {
    let Some(role_file) = manifest.roles.get(&Role::ShotScreen) else {
        return Ok(ShotFor::nothing(
            "no shot-screen photo in this bundle — no launch-monitor data to attach".to_string(),
        ));
    };

    let store = ShotStore::new(&options.shots_dir);
    // The stored shot this run re-reads, when the store's is an older parser's.
    let mut stale: Option<ShotData> = None;
    if !options.force_ocr {
        if let Some(cached) = store.get(&role_file.content_sha256)? {
            if parse_is_current(&cached) {
                log(&format!(
                    "  shot_screen: {} from the shot store",
                    cached.shot_id
                ));
                return Ok(ShotFor {
                    shot: Some(cached),
                    note: None,
                    origin: Origin::Stored,
                });
            }
            log(&stale_line(&cached));
            stale = Some(cached);
        }
    }
    // Every way the photo goes unread: the older shot kept with its note, or Python's note alone.
    let unread = |stale: Option<ShotData>, because: String, python: String| match stale {
        Some(older) => ShotFor {
            note: Some(older_shot_note(&older, &because)),
            shot: Some(older),
            origin: Origin::KeptOlder,
        },
        None => ShotFor::nothing(python),
    };

    if options.skip_ocr {
        return Ok(unread(
            stale,
            "shot-screen OCR was skipped by request".to_string(),
            "shot-screen OCR skipped by request — no launch-monitor data attached".to_string(),
        ));
    }

    let image_path = swing_dir.join(&role_file.filename);
    if !image_path.exists() {
        let missing = format!(
            "shot-screen photo {} is missing from the swing directory",
            role_file.filename
        );
        return Ok(unread(stale, missing.clone(), missing));
    }

    if let Err(why) = reader.ready() {
        let python = format!("no launch-monitor data attached: {why}");
        return Ok(unread(stale, why, python));
    }

    log(&format!(
        "  shot_screen: reading {} ...",
        role_file.original_filename
    ));
    let broad =
        |why: &str| format!("the shot screen could not be read — no numbers attached: {why}");
    // Everything from here on is inside Python's broad `except`, `import_screen`'s own store
    // lookup included, and in its order: `load_profile` is an argument to `import_screen`, which
    // reads the bytes first thing.
    let profile = match load_profile(&options.device) {
        Ok(profile) => profile,
        Err(unknown) => {
            let why = unknown.to_string();
            return Ok(unread(stale, why.clone(), broad(&why)));
        }
    };
    let digest = match std::fs::read(&image_path) {
        Ok(bytes) => hash_image(&bytes),
        Err(error) => {
            let why = format!("could not read image {}: {error}", image_path.display());
            return Ok(unread(stale, why.clone(), broad(&why)));
        }
    };
    // `hash_note` is the caller's to log, as every note here is.
    let mut hash_note = None;
    if digest != role_file.content_sha256 {
        hash_note = Some(format!(
            "the shot-screen photo on disk hashes to {digest}, not to the {} its manifest \
             records, so it was read as a new photo",
            role_file.content_sha256
        ));
        // The store's shot under the manifest's digest is another photo's, so it is neither kept
        // nor replaced. The photo's own is whatever `import_screen` finds under the bytes' digest,
        // and the cache rule above holds for it: a current one is Python's `cached`, and a stale
        // one is re-read keeping its identity.
        stale = None;
        if !options.force_ocr {
            match store.get(&digest) {
                Err(error) => return Ok(ShotFor::nothing(broad(&error.to_string()))),
                Ok(Some(cached)) if parse_is_current(&cached) => {
                    log("  shot_screen: cached");
                    return Ok(ShotFor {
                        shot: Some(cached),
                        note: hash_note,
                        origin: Origin::Stored,
                    });
                }
                Ok(Some(cached)) => {
                    log(&stale_line(&cached));
                    stale = Some(cached);
                }
                Ok(None) => {}
            }
        }
    }
    let swing_shot_id = format!("{}-{}", manifest.session_id, manifest.swing_id);
    let read = read_screen(
        &image_path,
        digest,
        profile,
        stale.as_ref(),
        NewImport {
            session_id: &manifest.session_id,
            shot_id: Some(&swing_shot_id),
        },
        options,
        reader,
    );
    let shot = match read {
        Ok(read) => read,
        Err(why) => return Ok(unread(stale, why.clone(), broad(&why))),
    };
    // Whatever is stale here is this photo's shot, which the read replaces.
    let replacing = stale.map(Box::new);
    let Some(shot) = shot else {
        return Ok(ShotFor {
            shot: None,
            note: Some(
                "the shot screen could not be read (failed) — no numbers attached".to_string(),
            ),
            origin: Origin::Read {
                replacing,
                written: None,
            },
        });
    };

    let written = if options.dry_run {
        None
    } else {
        match store.put(&shot) {
            Ok(path) => Some(path),
            Err(error) => {
                let why = error.to_string();
                return Ok(unread(
                    replacing.map(|older| *older),
                    format!("its re-read could not be stored: {why}"),
                    broad(&why),
                ));
            }
        }
    };
    log(&format!(
        "  shot_screen: {}",
        if options.dry_run { "dry-run" } else { "parsed" }
    ));
    Ok(ShotFor {
        shot: Some(shot),
        note: hash_note,
        origin: Origin::Read { replacing, written },
    })
}

/// Who a new import is: `import_screen`'s `session_id` and its `shot_id` override. [M29 P24]
///
/// The one stamp rule, factored out of [`read_screen`] when `golf-lab import-photos` became its
/// second caller (the M29 plan's P23 finding 6): [`shot_for`] passes `{session}-{swing}`, as
/// `_shot_for` passes it to `import_screen`, and a store-only import passes no override, so the id
/// is `import_screen`'s own `{session}-{digest[:12]}`. A re-read of a stale shot uses neither: it
/// keeps the stale shot's identity (the module doc).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewImport<'a> {
    pub session_id: &'a str,
    pub shot_id: Option<&'a str>,
}

impl NewImport<'_> {
    /// `shot_id or f"{session_id}-{digest[:12]}"`, the slice by code point as Python's is.
    fn shot_id(&self, digest: &str) -> String {
        match self.shot_id {
            Some(shot_id) => shot_id.to_string(),
            None => format!(
                "{}-{}",
                self.session_id,
                digest.chars().take(12).collect::<String>()
            ),
        }
    }
}

/// What [`import_photo`] made of one photo: `import_screen`'s `(status, shot)`, plus the re-read
/// Python has no twin of. [`Imported::status`] is the word the script's table prints.
#[derive(Debug, Clone, PartialEq)]
pub enum Imported {
    /// The store's shot, current for this parser, and no read: Python's `cached`.
    Cached(Box<ShotData>),
    /// The photo was read and a shot came of it. `replacing` is the stale stored shot it re-reads,
    /// keeping that shot's identity, or `None` for a new import; `written` is the store file, or
    /// `None` on a dry run.
    Read {
        shot: Box<ShotData>,
        replacing: Option<Box<ShotData>>,
        written: Option<PathBuf>,
    },
    /// The photo was read and nothing was read from it: Python's `failed`. The store keeps
    /// whatever it held, because `import_screen` never deletes a file.
    Nothing,
}

impl Imported {
    /// `cached`, `parsed`, `dry-run` and `failed` are `import_screen`'s statuses. `re-read` is the
    /// stale branch's, seven characters so the script's `{status:<7}` column still lines up.
    pub fn status(&self) -> &'static str {
        match self {
            Imported::Cached(_) => "cached",
            Imported::Read { written: None, .. } => "dry-run",
            Imported::Read {
                replacing: Some(_), ..
            } => "re-read",
            Imported::Read { .. } => "parsed",
            Imported::Nothing => "failed",
        }
    }

    pub fn shot(&self) -> Option<&ShotData> {
        match self {
            Imported::Cached(shot) => Some(shot),
            Imported::Read { shot, .. } => Some(shot),
            Imported::Nothing => None,
        }
    }
}

/// One photo into the shot store: `importer.py::import_screen`, with [`shot_for`]'s cache rule.
/// [M29 P24]
///
/// Python's order and Python's one lookup, under the digest of the bytes on disk: a stored shot
/// whose parse is current is `cached` and nothing is read; `options.force_ocr` (the script's
/// `--force`) skips the lookup, and the photo is read and stamped as a new import (`new`). **A stale
/// stored shot is re-read and keeps its identity**, which is [`shot_for`]'s rule and not Python's,
/// for the module doc's reasons: Python serves it as `cached`, and every shot frozen Python stored
/// is stale. `options.dry_run` reads and writes nothing; `options.skip_ocr` is not read, because a
/// bulk import that may not read a photo has nothing to do.
///
/// `Err` is what Python's `import_screen` raises, which ends the script there: a file that cannot
/// be read, a stored shot that is not a shot, a decode or model error (the reader's own message,
/// unwrapped, so `UnsupportedJpeg`'s repair reaches the user: the plan's P2 finding 19), or a write
/// that failed. The caller decides what one costs the run.
pub fn import_photo(
    image_path: &Path,
    new: NewImport<'_>,
    profile: &DeviceProfile,
    options: &ShotOptions,
    reader: &mut dyn PhotoReader,
) -> Result<Imported, String> {
    let bytes = std::fs::read(image_path)
        .map_err(|error| format!("could not read image {}: {error}", image_path.display()))?;
    let digest = hash_image(&bytes);
    let store = ShotStore::new(&options.shots_dir);
    let mut stale = None;
    if !options.force_ocr {
        match store.get(&digest).map_err(|error| error.to_string())? {
            Some(cached) if parse_is_current(&cached) => {
                return Ok(Imported::Cached(Box::new(cached)))
            }
            Some(cached) => stale = Some(cached),
            None => {}
        }
    }
    let Some(shot) = read_screen(
        image_path,
        digest,
        profile,
        stale.as_ref(),
        new,
        options,
        reader,
    )?
    else {
        return Ok(Imported::Nothing);
    };
    let written = if options.dry_run {
        None
    } else {
        Some(store.put(&shot).map_err(|error| {
            format!(
                "could not store the shot read from {}: {error}",
                image_path.display()
            )
        })?)
    };
    Ok(Imported::Read {
        shot: Box::new(shot),
        replacing: stale.map(Box::new),
        written,
    })
}

/// `import_screen` from the photo's digest on: read, parse, stamp. Answers the shot, `None` for a
/// read of nothing; `Err` is anything Python's broad `except` would have caught, as its message.
fn read_screen(
    image_path: &Path,
    digest: String,
    profile: &DeviceProfile,
    stale: Option<&ShotData>,
    new: NewImport<'_>,
    options: &ShotOptions,
    reader: &mut dyn PhotoReader,
) -> Result<Option<ShotData>, String> {
    let prepared = reader.prepare(image_path, profile)?;

    // The stale shot's identity, which is this photo's; `import_screen`'s stamp for a new import.
    let (shot_id, session_id, timestamp, path) = match stale {
        Some(older) => (
            older.shot_id.clone(),
            older.session_id.clone(),
            older.timestamp,
            older
                .provenance
                .as_ref()
                .and_then(|provenance| provenance.image_path.clone())
                .unwrap_or_else(|| image_path.display().to_string()),
        ),
        None => (
            new.shot_id(&digest),
            new.session_id.to_string(),
            screen_timestamp(image_path)?,
            image_path.display().to_string(),
        ),
    };
    let input = ScreenInput {
        device: profile.device.clone(),
        boxes: prepared.boxes,
        notes: prepared.notes,
        shot_id,
        session_id,
        timestamp,
        image_sha256: Some(digest),
        image_path: Some(path),
        min_confidence: options.min_confidence,
    };
    let shot = screen::read(&input).map_err(|unknown| unknown.to_string())?;
    Ok(shot)
}

/// The log line for a stored shot an older parser read, which this run re-reads.
fn stale_line(shot: &ShotData) -> String {
    format!(
        "  shot_screen: {} in the shot store was read by {}, re-reading with screen parser \
         {SCREEN_PARSER_VERSION}",
        shot.shot_id,
        parsed_by(shot)
    )
}

fn screen_timestamp(image_path: &Path) -> Result<Timestamp, String> {
    ocr::screen_timestamp(image_path).map_err(|error| format!("{}: {error}", image_path.display()))
}

/// Which parser read a stored shot, as a log line and a note name it. 0 is the unstamped default,
/// every shot frozen Python's parser wrote, which predates the ledger.
fn parsed_by(shot: &ShotData) -> String {
    parser_name(
        shot.provenance
            .as_ref()
            .map_or(0, |provenance| provenance.parser_version),
    )
}

/// A parser by its ledger entry, as [`parsed_by`] names it: `golf-lab reanalyze`'s reason for a
/// result whose shot an older parser read says it in these words too (P27).
pub(crate) fn parser_name(version: i64) -> String {
    match version {
        0 => "an unversioned screen parser".to_string(),
        version => format!("screen parser {version}"),
    }
}

/// Why a swing carries a shot this build's parser did not read, in `_older_pose_note`'s shape.
///
/// Said rather than substituted quietly, for that note's reason: a reader comparing this result with
/// one whose shot was re-read has to be able to see where a difference came from.
pub fn older_shot_note(older: &ShotData, because: &str) -> String {
    format!(
        "the launch-monitor numbers for {} were read by {} rather than by screen parser \
         {SCREEN_PARSER_VERSION}, and the photo could not be re-read because {because}",
        older.shot_id,
        parsed_by(older)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_config_pys() {
        // Frozen Python's `Settings` defaults, read from the source rather than copied, so the lab
        // and the frozen server read a photo with the same profile and threshold until M40.
        let config = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src/golf_coach/config.py");
        let text = std::fs::read_to_string(&config).unwrap();
        let declared = |name: &str| {
            text.lines()
                .map(str::trim)
                .find(|line| line.starts_with(&format!("{name}: ")))
                .unwrap_or_else(|| panic!("config.py declares {name}"))
                .to_string()
        };
        assert_eq!(
            declared("launch_monitor_profile"),
            format!("launch_monitor_profile: str = \"{DEFAULT_DEVICE}\"")
        );
        assert_eq!(
            declared("ocr_min_confidence"),
            format!("ocr_min_confidence: float = {DEFAULT_MIN_CONFIDENCE}")
        );
    }

    #[test]
    fn the_settings_are_the_environments_and_empty_is_unset() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(key, _)| *key == name)
                    .map(|(_, value)| value.to_string())
            }
        };
        let options = ShotOptions::configured("shots", env(&[])).unwrap();
        assert_eq!(options, ShotOptions::new("shots"));
        let options = ShotOptions::configured(
            "shots",
            env(&[(DEVICE_ENV, "full_swing"), (MIN_CONFIDENCE_ENV, " 0.75 ")]),
        )
        .unwrap();
        assert_eq!(
            (options.device.as_str(), options.min_confidence),
            ("full_swing", 0.75)
        );
        let options =
            ShotOptions::configured("shots", env(&[(DEVICE_ENV, ""), (MIN_CONFIDENCE_ENV, "")]))
                .unwrap();
        assert_eq!(options, ShotOptions::new("shots"));
        let refused = ShotOptions::configured("shots", env(&[(MIN_CONFIDENCE_ENV, "high")]));
        assert_eq!(
            refused,
            Err("GOLF_OCR_MIN_CONFIDENCE=\"high\" is not a number".to_string())
        );
    }

    #[test]
    fn a_parser_is_named_by_its_ledger_entry_and_zero_is_unversioned() {
        let mut shot: ShotData = serde_json::from_value(serde_json::json!({
            "shot_id": "2026-08-10-1", "session_id": "2026-08-10",
            "timestamp": "2026-08-10T01:38:46.828488Z", "source": "screen",
            "provenance": {"device": "hd_golf", "parse_confidence": 0.9}
        }))
        .unwrap();
        assert_eq!(parsed_by(&shot), "an unversioned screen parser");
        assert_eq!(
            older_shot_note(&shot, "shot-screen OCR was skipped by request"),
            format!(
                "the launch-monitor numbers for 2026-08-10-1 were read by an unversioned screen \
                 parser rather than by screen parser {SCREEN_PARSER_VERSION}, and the photo could \
                 not be re-read because shot-screen OCR was skipped by request"
            )
        );
        shot.provenance.as_mut().unwrap().parser_version = 7;
        assert_eq!(parsed_by(&shot), "screen parser 7");
    }
}
