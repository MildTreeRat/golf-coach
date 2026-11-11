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
//! consulted for a shot to serve, and the photo is read and stamped as a new import. (It is asked
//! one thing, what a person corrected: the section below, which Python has no twin of.)
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
//!
//! # The clip's half (M42 P14), with no Python twin
//!
//! A Live Photo of Full Swing's grid screen carries the grid in its motion part too, beside the
//! replay panel, so a clip imports a shot with no still (the M42 plan's decision 7). No Python
//! ever read a clip, so this half is new. [`read_clip_grid`] decodes the clip, picks one frame by
//! [`GRID_FRAME`] ([`write_grid_frame`]), writes it as a PNG and reads that PNG through the path a
//! still takes, so the numbers half of a clip's shot is held to the still's rule: no wrong value
//! (decision 10), which `tests/full_swing_clips.rs` holds on the five grid clips.
//!
//! The other half (M42 P14b) is [`read_impact_video`]: the impact offset off the same clip's
//! replay, read once the grid has given a shot, on a device that declares a field `from:
//! impact_video`. Call 13's three outcomes are its table: read sets `impact_offset_h` and lists it,
//! refused lists it and leaves it `None` with the refusal's sentence as a warning, which
//! `screen::tile_refusals` names `misread`, and a clip that will not decode lists nothing and warns.
//! **A still never reaches it.** [`import_photo`] takes no frames, so a still's shot never lists the
//! field, which is ADR-034 §2's "not printed": nothing downstream, not even a blank. Neither
//! refusal flags the shot for review, because a flag sets a shot's numbers aside (the corpus's
//! `shot_needs_review`) and the grid's numbers are as good as they were.
//!
//! **A re-read of a stale shot keeps its offset.** The re-read reads the entry's PNG, which holds no
//! replay, so the offset is the older read's to carry ([`carry_impact_video`]), and otherwise a
//! `SCREEN_PARSER_VERSION` bump would quietly make every clip's shot a still's. `import-video` on the
//! clip itself reads the replay again and replaces what was carried.
//!
//! # A re-read keeps a person's correction (the M38 plan's C12), with no Python twin
//!
//! A phone's review writes a number a person read off the photo, and records what the reader said in
//! `corrections` (`golf_core::session::correct_shot`). Without this a re-read would silently put the
//! misread back, and a parser bump would do that to every shot anyone had reviewed. So a read that
//! replaces a stored shot, stale or by `--force`, takes three things per entry in its map from
//! [`carry_corrections`]:
//!
//! - **the field's value**, and a cleared field (no value, an entry holding the old number) is a
//!   correction too, so it stays cleared;
//! - **the entry**, unrewritten, since it says what was read before a person changed it and the new
//!   read is not that;
//! - **the field's place in `fields_present`**, because `analysed_view` blanks a reading outside that
//!   list, so a value kept for a tile this read did not locate would be stored and then ignored. A
//!   value always has its place, as `correct_shot` gives it one. A cleared field has the place it had:
//!   clearing says nothing about whether a tile is there.
//!
//! **Every other field is the new read's, and so are its flag and its warnings**: the review answered
//! the old read's question, and a new flag is a new question to ask. A shot with an empty map is not
//! touched at all, so its bytes are what they were before this existed (`tests/reread.rs` holds both
//! writers to that).
//!
//! `--force` is the one read that looks at the store for no shot to serve, and looks under the bytes'
//! digest, since a correction was made on those bytes and the manifest's digest may be another
//! photo's. A stored file that is not a shot holds no correction to keep and `--force` has always
//! replaced one, so it is not an error there, as it is on the lookup. The cost is that `--force` can
//! no longer undo a correction: typing the first read back on the phone does, and at the lab deleting
//! the store's file does.

use std::path::{Path, PathBuf};

use contracts::capability::capability_for;
use contracts::shot::{parse_is_current, ShotData, SCREEN_PARSER_VERSION};
use contracts::Timestamp;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};
use impact::Frame;
use ocr::models::Models;
use screen::profile::{load_profile, DeviceProfile};
use screen::{ScreenInput, TextBox};
use storage::manifest::{Role, SwingManifest};
use storage::shot_store::{hash_image, ShotStore};
use storage::StoreError;

use crate::ffmpeg::Ffmpeg;
use crate::import::spelled;
use crate::read_impact::{read_frames, Outcome, DECODE_WIDTH};

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
    /// Read the photo even when the store holds a current shot for it, as a new import, keeping
    /// only what a person corrected (the module doc).
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

    /// The store a run writes shots to, at the directory as Python's `Path` would have spelled it.
    ///
    /// `ShotStore::put` makes its root, and `--shots-dir new/.` is a root that `create_dir_all`
    /// cannot make off Windows: `mkdir("new/.")` answers `ENOENT` while `new` does not exist, and
    /// `Path::parent` treats the `.` as absent, so the retry goes to `new`'s parent and `new` is
    /// never made. Windows drops the `.` before the filesystem sees it, and Python's `Path` drops it
    /// on construction, so neither ever met this; a Mac found it (M38 P2). Nothing prints this path
    /// (the table's store line is spelled where it prints), so no spelling on Windows moves.
    fn store(&self) -> ShotStore {
        ShotStore::new(spelled(&self.shots_dir))
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

    let store = options.store();
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

/// One photo into the shot store: [`import_photo_with`], with nothing to add to the shot.
pub fn import_photo(
    image_path: &Path,
    new: NewImport<'_>,
    profile: &DeviceProfile,
    options: &ShotOptions,
    reader: &mut dyn PhotoReader,
) -> Result<Imported, String> {
    import_photo_with(image_path, new, profile, options, reader, &mut |_| {})
}

/// One photo into the shot store: `importer.py::import_screen`, with [`shot_for`]'s cache rule.
/// [M29 P24]
///
/// Python's order and Python's one lookup, under the digest of the bytes on disk: a stored shot
/// whose parse is current is `cached` and nothing is read; `options.force_ocr` (the script's
/// `--force`) skips the lookup, and the photo is read and stamped as a new import (`new`). **A stale
/// stored shot is re-read and keeps its identity**, which is [`shot_for`]'s rule and not Python's,
/// for the module doc's reasons: Python serves it as `cached`, and every shot frozen Python stored
/// is stale. **Both keep what a person corrected** ([`carry_corrections`]). `options.dry_run` reads
/// and writes nothing; `options.skip_ocr` is not read, because a bulk import that may not read a
/// photo has nothing to do.
///
/// `Err` is what Python's `import_screen` raises, which ends the script there: a file that cannot
/// be read, a stored shot that is not a shot, a decode or model error (the reader's own message,
/// unwrapped, so `UnsupportedJpeg`'s repair reaches the user: the plan's P2 finding 19), or a write
/// that failed. The caller decides what one costs the run.
///
/// `finish` gets the shot between the read and the store, and is not called for a cached shot or a
/// read of nothing: it is how `import-video` adds the replay's reading (no Python twin, M42 P14b), so
/// one write stores the whole shot and a cached one costs no decode.
pub fn import_photo_with(
    image_path: &Path,
    new: NewImport<'_>,
    profile: &DeviceProfile,
    options: &ShotOptions,
    reader: &mut dyn PhotoReader,
    finish: &mut dyn FnMut(&mut ShotData),
) -> Result<Imported, String> {
    let bytes = std::fs::read(image_path)
        .map_err(|error| format!("could not read image {}: {error}", image_path.display()))?;
    let digest = hash_image(&bytes);
    let store = options.store();
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
    let Some(mut shot) = read_screen(
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
    finish(&mut shot);
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
    // What `--force` replaces, asked only what a person corrected (the module doc).
    let forced_over = options
        .force_ocr
        .then(|| options.store().get(&digest).ok().flatten())
        .flatten();

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
    let mut shot = screen::read(&input).map_err(|unknown| unknown.to_string())?;
    if let (Some(shot), Some(older)) = (shot.as_mut(), stale) {
        carry_impact_video(older, shot);
    }
    if let (Some(shot), Some(older)) = (shot.as_mut(), stale.or(forced_over.as_ref())) {
        carry_corrections(older, shot)?;
    }
    Ok(shot)
}

/// What a person corrected on the shot this read replaces, onto the read: each entry in its map, with
/// the field's value and its place in `fields_present` (the module doc). An empty map returns before
/// anything is touched.
///
/// Through `serde_json::Value`, as `correct_shot`'s `with_edits` is, so the struct stays the only list
/// of its fields, and because the two keys `ShotData` skips at `None` read as null and write as absent
/// that way. A value crosses as the `f64` it is, with no unit or sign to work: it is `ShotData`'s on
/// both sides.
fn carry_corrections(older: &ShotData, shot: &mut ShotData) -> Result<(), String> {
    let (Some(from), Some(onto)) = (older.provenance.as_ref(), shot.provenance.as_mut()) else {
        return Ok(());
    };
    if from.corrections.is_empty() {
        return Ok(());
    }
    let carried = serde_json::to_value(older).map_err(|failed| failed.to_string())?;
    let had_a_place = |field: &str| {
        let present = from.fields_present.as_deref().unwrap_or_default();
        present.iter().any(|located| located == field)
    };
    for (field, as_read) in &from.corrections {
        onto.corrections.insert(field.clone(), as_read.clone());
        if !carried[field.as_str()].is_null() || had_a_place(field) {
            if let Some(present) = onto.fields_present.as_mut() {
                if !present.contains(field) {
                    present.push(field.clone());
                }
            }
        }
    }
    let mut written = serde_json::to_value(&*shot).map_err(|failed| failed.to_string())?;
    for field in from.corrections.keys() {
        written[field.as_str()] = carried[field.as_str()].clone();
    }
    *shot = serde_json::from_value(written).map_err(|refused| refused.to_string())?;
    Ok(())
}

/// Where a Live Photo clip's frames come from: [`Ffmpeg`] in the binary, a fake in the tests.
/// [M42 P14b]
///
/// `width` is what to scale to, `None` for the clip's own size: the grid is read at its own
/// (`write_grid_frame`) and the replay at [`DECODE_WIDTH`] ([`read_impact_video`]), as
/// [`Ffmpeg::decode_frames`] takes them. Any closure of that shape is one, so a test can answer in a
/// line.
pub trait MotionFrames {
    fn frames(&mut self, clip: &Path, width: Option<u32>) -> Result<Vec<Frame>, String>;
}

impl MotionFrames for Ffmpeg {
    fn frames(&mut self, clip: &Path, width: Option<u32>) -> Result<Vec<Frame>, String> {
        self.decode_frames(clip, width)
    }
}

impl<F> MotionFrames for F
where
    F: FnMut(&Path, Option<u32>) -> Result<Vec<Frame>, String>,
{
    fn frames(&mut self, clip: &Path, width: Option<u32>) -> Result<Vec<Frame>, String> {
        self(clip, width)
    }
}

/// The field the impact reader delivers: heel to toe only, high-low being invisible from above (the
/// M42 plan's finding 9).
const IMPACT_FIELD: &str = "impact_offset_h";

/// What every warning an impact read leaves starts with (call 13), so the importer can say which
/// line of a shot's warnings is the replay's, and a re-read can carry it. `ImpactRefusal::sentence`
/// is written to follow it.
pub const IMPACT_VIDEO_WARNING: &str = "impact video: ";

/// The impact offset off `clip`'s replay, into `shot`: [`record_impact_outcome`] over the reader at
/// [`DECODE_WIDTH`]. A device that declares no field `from: impact_video` is left as it is, and asks
/// for no frames. [M42 P14b]
pub fn read_impact_video(shot: &mut ShotData, clip: &Path, motion: &mut dyn MotionFrames) {
    let reads_the_replay = shot
        .provenance
        .as_ref()
        .and_then(|provenance| capability_for(&provenance.device).ok())
        .is_some_and(|capability| capability.reads_impact_video());
    if !reads_the_replay {
        return;
    }
    let outcome = match motion.frames(clip, Some(DECODE_WIDTH)) {
        Ok(frames) => read_frames(&frames),
        Err(error) => Outcome::Undecodable(error),
    };
    record_impact_outcome(shot, &outcome);
}

/// Call 13's three outcomes on a shot. Whatever an earlier read left is replaced, so this is the one
/// place the field, its listing and its warning are written. [M42 P14b]
pub fn record_impact_outcome(shot: &mut ShotData, outcome: &Outcome) {
    let Some(provenance) = shot.provenance.as_mut() else {
        return;
    };
    provenance
        .warnings
        .retain(|warning| !warning.starts_with(IMPACT_VIDEO_WARNING));
    if let Some(present) = provenance.fields_present.as_mut() {
        present.retain(|field| field != IMPACT_FIELD);
        if !matches!(outcome, Outcome::Undecodable(_)) {
            present.push(IMPACT_FIELD.to_string());
        }
    }
    shot.impact_offset_h = None;
    match outcome {
        Outcome::Read(reading) => shot.impact_offset_h = Some(reading.offset_h_mm),
        Outcome::Refused(refusal) => provenance
            .warnings
            .push(format!("{IMPACT_VIDEO_WARNING}{}", refusal.sentence())),
        Outcome::Undecodable(error) => provenance.warnings.push(format!(
            "{IMPACT_VIDEO_WARNING}could not be decoded: {error}"
        )),
    }
}

/// What a stale shot's read of its replay left, onto the re-read of its grid (the module doc).
fn carry_impact_video(older: &ShotData, shot: &mut ShotData) {
    let (Some(from), Some(onto)) = (older.provenance.as_ref(), shot.provenance.as_mut()) else {
        return;
    };
    let listed = from
        .fields_present
        .as_ref()
        .is_some_and(|present| present.iter().any(|field| field == IMPACT_FIELD));
    if listed {
        shot.impact_offset_h = older.impact_offset_h;
        if let Some(present) = onto.fields_present.as_mut() {
            present.push(IMPACT_FIELD.to_string());
        }
    }
    onto.warnings.extend(
        from.warnings
            .iter()
            .filter(|warning| warning.starts_with(IMPACT_VIDEO_WARNING))
            .cloned(),
    );
}

/// Which frame of a Live Photo clip the grid of numbers is read from (the M42 plan's call 14).
/// [M42 P14]
///
/// Both rules stay runnable, by the house's convention for a rejected alternative: P14 measured
/// each on the five grid clips through the clip gate (`tests/full_swing_clips.rs`), and the plan's
/// P14 findings hold both counts. [`GRID_FRAME`] is the one kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridFrame {
    /// [`impact::sharpest_frame`]: the frame with the most gradient energy, ties to the earliest.
    Sharpest,
    /// The middle frame, `len / 2`, which is where the still sits in a Live Photo's motion part.
    Middle,
}

impl GridFrame {
    /// The frame this rule chooses, by index, or `None` for a clip with no frame to choose.
    pub fn choose(self, frames: &[Frame]) -> Option<usize> {
        match self {
            GridFrame::Sharpest => impact::sharpest_frame(frames),
            GridFrame::Middle => (!frames.is_empty()).then_some(frames.len() / 2),
        }
    }
}

/// The rule a clip's grid is read by: call 14's default, kept by P14's measurement.
///
/// Through the clip gate on the five grid clips, neither rule read a wrong value. The sharpest
/// frame read all 60 values (frames 31 of 36, 17 of 61, 21 of 55, 29 of 31 and 41 of 59), and the
/// middle frame 59. The one value the middle frame missed was `IMG_2893`'s sidespin: no `D` or `F`
/// box came back from its frame 15, so the sign was withheld (call 5). Both read `IMG_2892`'s total,
/// which its still withholds (P9: a stray box over the header copy).
pub const GRID_FRAME: GridFrame = GridFrame::Sharpest;

/// The grid frame [`write_grid_frame`] wrote: its index among the clip's frames, how many there
/// were, and the sha256 of the PNG's bytes, which is the photo's digest as any photo's is taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridWritten {
    pub frame: usize,
    pub frames: usize,
    pub digest: String,
}

/// One clip's grid frame, as the PNG a still's reader reads. [M42 P14]
///
/// The clip is decoded at its own size (`decode(clip, None)`, rotation applied: 1744 × 1308 on the
/// five grid clips, which ffmpeg crops from the 1920 × 1440 stored, P13's finding), `rule` chooses
/// one frame, and that frame is written to `png`, a path the caller gives, losslessly, so the
/// reader sees the decoder's pixels and nothing a second codec did to them. **The PNG takes the
/// clip's modification time**, because the photo's is the shot's timestamp
/// (`ocr::screen_timestamp`) and the clip is when the shot was hit; the time is set on a second
/// handle after the bytes are written and closed, because Windows may move a file's write time
/// when a handle it wrote through closes.
///
/// Every frame is held while the rule runs: 6.8 MB each at that size, so 417 MB for `IMG_2891`'s
/// 61 (P13's finding). That was kept rather than streamed because a clip is at most a couple of
/// seconds and the laptop is the only host that decodes this way; the phone hands its own frames.
///
/// `Err` names the clip: ffmpeg's failure, a clip with no frame, or a PNG that could not be
/// written or stamped.
pub fn write_grid_frame(
    clip: &Path,
    png: &Path,
    rule: GridFrame,
    motion: &mut dyn MotionFrames,
) -> Result<GridWritten, String> {
    let frames = motion.frames(clip, None)?;
    let frame = rule
        .choose(&frames)
        .ok_or_else(|| format!("{} has no frame to read the grid from", clip.display()))?;
    let bytes = encode_png(&frames[frame])
        .map_err(|why| format!("frame {frame} of {}: {why}", clip.display()))?;
    let modified = std::fs::metadata(clip)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| format!("could not read the time of {}: {error}", clip.display()))?;
    std::fs::write(png, &bytes)
        .and_then(|()| std::fs::File::options().write(true).open(png))
        .and_then(|file| file.set_modified(modified))
        .map_err(|error| format!("could not write {}: {error}", png.display()))?;
    Ok(GridWritten {
        frame,
        frames: frames.len(),
        digest: hash_image(&bytes),
    })
}

/// A frame as PNG bytes, RGB at eight bits a channel, with the encoder's defaults. Deterministic
/// (no time or text chunk is written), which is what lets one clip imported twice hash to one
/// photo and so land on one store entry (call 14); `tests::a_frame_encodes_to_the_same_png_twice`
/// pins it.
pub fn encode_png(frame: &Frame) -> Result<Vec<u8>, String> {
    let size = |n: usize| u32::try_from(n).map_err(|_| format!("{n} pixels is too large a side"));
    if !frame.is_well_formed() {
        return Err(format!(
            "{} bytes is not {} × {} pixels of RGB",
            frame.rgb.len(),
            frame.width,
            frame.height
        ));
    }
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(
            &frame.rgb,
            size(frame.width)?,
            size(frame.height)?,
            ExtendedColorType::Rgb8,
        )
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}

/// What [`read_clip_grid`] read off one clip: the shot (`None` for a read of nothing, as a still's
/// `failed`), its impact outcome included, and which frame it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct ClipGrid {
    pub shot: Option<ShotData>,
    /// The grid frame's index among the clip's frames, counted from 0.
    pub frame: usize,
    /// How many frames the clip decoded to.
    pub frames: usize,
}

/// A Live Photo clip read with no store around it (the M42 plan's decision 7): its grid frame
/// written to `png` by [`write_grid_frame`], then read through [`read_screen`], the path a still
/// takes, so the profile, the parser's rules and the validator are the still's, unchanged, and then
/// its replay read by [`read_impact_video`]. [M42 P14, the replay since P14b]
///
/// A new import every time: no store is consulted and nothing is written but `png`. The shot's
/// photo is the PNG (its digest, its path, its modification time, which is the clip's), and its
/// identity is `new`'s, as a still's is. It is what the clip gate runs. `golf-lab import-video` takes
/// the same three steps with the store's cache rule between them (`crate::import`), over
/// [`write_grid_frame`], [`import_photo_with`] and [`read_impact_video`].
///
/// Eight arguments where clippy's limit is seven, as `import_export`'s `swing` has: each is one the
/// still's own [`import_photo`] takes or one the clip adds (the clip, where its frame goes, the
/// rule, the decoder), and bundling them would be a struct with no meaning but the lint's.
#[allow(clippy::too_many_arguments)]
pub fn read_clip_grid(
    clip: &Path,
    png: &Path,
    rule: GridFrame,
    new: NewImport<'_>,
    profile: &DeviceProfile,
    options: &ShotOptions,
    motion: &mut dyn MotionFrames,
    reader: &mut dyn PhotoReader,
) -> Result<ClipGrid, String> {
    let written = write_grid_frame(clip, png, rule, motion)?;
    let mut shot = read_screen(png, written.digest, profile, None, new, options, reader)?;
    if let Some(shot) = shot.as_mut() {
        read_impact_video(shot, clip, motion);
    }
    Ok(ClipGrid {
        shot,
        frame: written.frame,
        frames: written.frames,
    })
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

    /// A frame whose every pixel is `shade`, with one brighter column at `x` so frames differ.
    fn frame(shade: u8, x: usize) -> Frame {
        let (width, height) = (6, 4);
        let mut rgb = vec![shade; width * height * 3];
        for y in 0..height {
            let at = (y * width + x) * 3;
            rgb[at..at + 3].copy_from_slice(&[250, 240, 230]);
        }
        Frame { width, height, rgb }
    }

    #[test]
    fn the_middle_rule_takes_half_the_length_and_the_sharpest_is_impacts() {
        let frames: Vec<Frame> = (0..5).map(|i| frame(10 * i as u8, i)).collect();
        assert_eq!(GridFrame::Middle.choose(&frames), Some(2));
        assert_eq!(GridFrame::Middle.choose(&frames[..4]), Some(2));
        assert_eq!(GridFrame::Middle.choose(&frames[..1]), Some(0));
        assert_eq!(GridFrame::Middle.choose(&[]), None);
        assert_eq!(
            GridFrame::Sharpest.choose(&frames),
            impact::sharpest_frame(&frames)
        );
        assert_eq!(GridFrame::Sharpest.choose(&[]), None);
    }

    #[test]
    fn a_frame_encodes_to_the_same_png_twice_and_decodes_to_its_pixels() {
        let original = frame(30, 2);
        let first = encode_png(&original).unwrap();
        assert_eq!(encode_png(&original).unwrap(), first, "deterministic");
        let decoded = image::load_from_memory(&first).unwrap().to_rgb8();
        assert_eq!(
            (decoded.width(), decoded.height()),
            (original.width as u32, original.height as u32)
        );
        assert_eq!(decoded.into_raw(), original.rgb, "lossless");
        let broken = Frame {
            rgb: vec![0; 5],
            ..original
        };
        assert_eq!(
            encode_png(&broken),
            Err("5 bytes is not 6 × 4 pixels of RGB".to_string())
        );
    }

    /// A reader answering with P10's hand-worked `full-swing-grid` boxes, the ones `IMG_2889`'s
    /// still printed, and recording which photo it was asked to read.
    struct Replay {
        prepared: Prepared,
        asked: Vec<PathBuf>,
    }

    impl PhotoReader for Replay {
        fn ready(&mut self) -> Result<(), String> {
            Ok(())
        }

        fn prepare(&mut self, photo: &Path, profile: &DeviceProfile) -> Result<Prepared, String> {
            assert_eq!(profile.device, "full_swing");
            self.asked.push(photo.to_path_buf());
            Ok(self.prepared.clone())
        }
    }

    fn replay() -> Replay {
        let path = crate::repo_root().join("spec/vectors/screen/hand/full-swing-grid.json");
        let document: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let input: ScreenInput = serde_json::from_value(document["input"].clone()).unwrap();
        Replay {
            prepared: Prepared {
                boxes: input.boxes,
                notes: input.notes,
            },
            asked: Vec::new(),
        }
    }

    #[test]
    fn a_clips_grid_frame_is_read_as_a_photo_stamped_with_the_clips_time() {
        let scratch = tempfile::tempdir().unwrap();
        let clip = scratch.path().join("IMG_0001.MOV");
        std::fs::write(&clip, b"not a video: the decoder is the test's").unwrap();
        // A time the PNG could not have by accident: a day before now.
        let then = std::time::SystemTime::now() - std::time::Duration::from_secs(86_400);
        std::fs::File::options()
            .write(true)
            .open(&clip)
            .unwrap()
            .set_modified(then)
            .unwrap();
        let png = scratch.path().join("grid.png");
        let frames: Vec<Frame> = (0..3).map(|i| frame(20, i)).collect();
        let mut asked = Vec::new();
        let mut decode = |path: &Path, width: Option<u32>| {
            asked.push((path.to_path_buf(), width));
            Ok(frames.clone())
        };
        let mut reader = replay();
        let profile = load_profile("full_swing").unwrap();
        let mut options = ShotOptions::new(scratch.path().join("shots"));
        options.device = "full_swing".to_string();

        let grid = read_clip_grid(
            &clip,
            &png,
            GridFrame::Middle,
            NewImport {
                session_id: "clips",
                shot_id: None,
            },
            profile,
            &options,
            &mut decode,
            &mut reader,
        )
        .unwrap();

        assert_eq!(
            asked,
            vec![(clip.clone(), None), (clip.clone(), Some(DECODE_WIDTH))],
            "the grid at its own size, then the replay at the reader's"
        );
        assert_eq!((grid.frame, grid.frames), (1, 3));
        assert_eq!(reader.asked, vec![png.clone()], "the PNG is what is read");
        let bytes = std::fs::read(&png).unwrap();
        assert_eq!(bytes, encode_png(&frames[1]).unwrap());
        let digest = hash_image(&bytes);
        let shot = grid.shot.expect("the hand vector's boxes read as a shot");
        let provenance = shot.provenance.as_ref().unwrap();
        assert_eq!(provenance.image_sha256.as_deref(), Some(digest.as_str()));
        assert_eq!(
            provenance.image_path.as_deref(),
            Some(png.display().to_string().as_str())
        );
        assert_eq!(shot.shot_id, format!("clips-{}", &digest[..12]));
        assert_eq!(shot.timestamp, ocr::screen_timestamp(&clip).unwrap());
        assert_eq!(shot.carry_distance, Some(131.0));
        assert!(
            !scratch.path().join("shots").exists(),
            "nothing is stored: the store is the importer's"
        );
    }

    /// A shot as `IMG_2889`'s still reads: no offset, and the field not listed.
    fn read_shot() -> ShotData {
        let scratch = tempfile::tempdir().unwrap();
        let photo = scratch.path().join("still.png");
        std::fs::write(&photo, b"a stand-in only the reader would open").unwrap();
        let mut options = ShotOptions::new(scratch.path().join("shots"));
        options.dry_run = true;
        let imported = import_photo(
            &photo,
            NewImport {
                session_id: "clips",
                shot_id: None,
            },
            load_profile("full_swing").unwrap(),
            &options,
            &mut replay(),
        )
        .unwrap();
        imported.shot().unwrap().clone()
    }

    fn listed(shot: &ShotData) -> bool {
        let present = shot.provenance.as_ref().unwrap().fields_present.as_ref();
        present.unwrap().iter().any(|field| field == IMPACT_FIELD)
    }

    fn warnings(shot: &ShotData) -> Vec<&str> {
        let warnings = &shot.provenance.as_ref().unwrap().warnings;
        warnings
            .iter()
            .map(String::as_str)
            .filter(|warning| warning.starts_with(IMPACT_VIDEO_WARNING))
            .collect()
    }

    fn reading(offset_h_mm: f64) -> Outcome {
        Outcome::Read(impact::ImpactReading {
            offset_h_mm,
            contact_frame: 6,
            frames: 9,
            ball_center: [480.0, 360.0],
            ball_diameter_px: 24.0,
            heel: [430.0, 330.0],
            toe: [480.0, 360.0],
        })
    }

    /// A still's shot lists nothing, so `tile_refusals` names nothing: not printed (call 13).
    #[test]
    fn a_stills_shot_does_not_list_the_field_and_refuses_nothing_for_it() {
        let still = read_shot();
        assert!(!listed(&still));
        assert_eq!(still.impact_offset_h, None);
        let refused = screen::tile_refusals(&still).unwrap();
        assert!(refused.iter().all(|entry| entry.name != IMPACT_FIELD));
    }

    /// Call 13's three outcomes, each replacing the one before it, as a second read must.
    #[test]
    fn each_outcome_is_calls_and_replaces_the_last() {
        let mut shot = read_shot();

        record_impact_outcome(&mut shot, &reading(4.2));
        assert_eq!(shot.impact_offset_h, Some(4.2));
        assert!(listed(&shot));
        assert!(warnings(&shot).is_empty());
        let refused = screen::tile_refusals(&shot).unwrap();
        assert!(refused.iter().all(|entry| entry.name != IMPACT_FIELD));

        let refusal = Outcome::Refused(impact::ImpactRefusal::NoShaft);
        record_impact_outcome(&mut shot, &refusal);
        assert_eq!(shot.impact_offset_h, None);
        assert!(listed(&shot));
        assert_eq!(
            warnings(&shot),
            [format!(
                "impact video: {}",
                impact::ImpactRefusal::NoShaft.sentence()
            )]
        );
        let entry = screen::tile_refusals(&shot)
            .unwrap()
            .into_iter()
            .find(|entry| entry.name == IMPACT_FIELD)
            .expect("a refusal is named");
        assert_eq!(entry.reason, contracts::unscored::UnscoredReason::Misread);
        assert_eq!(entry.detail, screen::IMPACT_VIDEO_MISREAD);

        let undecodable = Outcome::Undecodable("boom".to_string());
        record_impact_outcome(&mut shot, &undecodable);
        assert_eq!(shot.impact_offset_h, None);
        assert!(!listed(&shot), "a machine problem: not printed");
        assert_eq!(
            warnings(&shot),
            ["impact video: could not be decoded: boom"]
        );
        let refused = screen::tile_refusals(&shot).unwrap();
        assert!(refused.iter().all(|entry| entry.name != IMPACT_FIELD));

        assert!(
            !shot.provenance.as_ref().unwrap().needs_review,
            "the grid's numbers are as good as they were"
        );
    }

    /// A device that declares nothing from the replay is left alone and asks for no frames.
    #[test]
    fn a_device_with_no_impact_video_field_asks_for_no_frames() {
        let mut shot = read_shot();
        shot.provenance.as_mut().unwrap().device = "hd_golf".to_string();
        let before = shot.clone();
        let mut asked = 0;
        let mut motion = |_: &Path, _: Option<u32>| {
            asked += 1;
            Err("not asked".to_string())
        };
        read_impact_video(&mut shot, Path::new("a.mov"), &mut motion);
        assert_eq!(asked, 0);
        assert_eq!(shot, before);
    }

    /// Both of a run's shot writers open the store through `ShotOptions::store`, so `analyze` and
    /// `reanalyze` are held here to what `a_path_is_spelled_as_pythons_str_path` holds the imports.
    #[test]
    fn a_shots_dir_ending_in_a_dot_is_made_though_it_does_not_exist_yet() {
        let scratch = tempfile::tempdir().unwrap();
        let options = ShotOptions::new(scratch.path().join("shots").join("."));
        let written = options.store().put(&read_shot()).unwrap();
        assert!(scratch.path().join("shots").is_dir());
        assert!(written.is_file(), "{}", written.display());
    }

    /// A stale shot's re-read reads the PNG and no replay, so its offset is carried, as a
    /// `SCREEN_PARSER_VERSION` bump must not quietly drop it; a current shot is served as it was.
    #[test]
    fn a_re_read_of_a_stale_clips_shot_keeps_what_its_replay_gave() {
        let scratch = tempfile::tempdir().unwrap();
        let photo = scratch.path().join("grid.png");
        std::fs::write(&photo, b"a stand-in only the reader would open").unwrap();
        let options = ShotOptions::new(scratch.path().join("shots"));
        let store = ShotStore::new(&options.shots_dir);
        let new = NewImport {
            session_id: "clips",
            shot_id: None,
        };
        let profile = load_profile("full_swing").unwrap();

        for outcome in [
            reading(4.2),
            Outcome::Refused(impact::ImpactRefusal::NoBall),
        ] {
            let mut stored = read_shot();
            record_impact_outcome(&mut stored, &outcome);
            let provenance = stored.provenance.as_mut().unwrap();
            provenance.parser_version = 0;
            provenance.image_sha256 = Some(hash_image(&std::fs::read(&photo).unwrap()));
            store.put(&stored).unwrap();

            let reread = import_photo(&photo, new, profile, &options, &mut replay()).unwrap();
            assert_eq!(reread.status(), "re-read");
            let shot = reread.shot().unwrap();
            assert_eq!(shot.impact_offset_h, stored.impact_offset_h);
            assert_eq!(
                shot.provenance.as_ref().unwrap().parser_version,
                SCREEN_PARSER_VERSION
            );
            assert!(listed(shot));
            assert_eq!(warnings(shot), warnings(&stored));
        }
    }

    #[test]
    fn a_clip_with_no_frame_or_no_decode_is_refused_naming_it() {
        let scratch = tempfile::tempdir().unwrap();
        let clip = scratch.path().join("IMG_0002.MOV");
        std::fs::write(&clip, b"").unwrap();
        let png = scratch.path().join("grid.png");
        let mut empty = |_: &Path, _: Option<u32>| Ok(Vec::new());
        assert_eq!(
            write_grid_frame(&clip, &png, GRID_FRAME, &mut empty),
            Err(format!(
                "{} has no frame to read the grid from",
                clip.display()
            ))
        );
        let mut broken = |_: &Path, _: Option<u32>| Err("ffmpeg could not decode".to_string());
        assert_eq!(
            write_grid_frame(&clip, &png, GRID_FRAME, &mut broken),
            Err("ffmpeg could not decode".to_string())
        );
        assert!(!png.exists(), "nothing written for a clip with no frame");
    }

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
