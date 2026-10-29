//! `golf-lab import-photos`: `scripts/import_shot_screens.py`, the bulk import of launch-monitor
//! screen photos into the shot store, and the photo-only entries its `--into-session` adds.
//! [M29 P24]
//!
//! # The store-only import is the script's
//!
//! Point it at photos or directories of them; each is read by [`crate::shot::import_photo`]
//! (`import_screen`) and printed as the script prints it, the same table, the same review list and
//! the same last line, with the script's exit codes: 0 when every shot imported cleanly, 1 when no
//! image was found, the reader cannot run, or any photo read as nothing or was flagged for review,
//! and 2 for a launch-monitor profile no profile names, as the script's `load_profile` refusal
//! does, and for a flag argparse would refuse. What differs from the script, each on purpose:
//!
//! - **A stale stored shot is re-read** and keeps its identity, printed `re-read`, where the
//!   script's `import_screen` answers `cached` (`crate::shot`'s module doc says why). A current one
//!   is `cached`, and `--force` reads the photo again as a new import, as the script's does.
//! - **A photo is a JPEG, a PNG or a HEIC**, the three `crates/ocr` decodes. The script lists
//!   `.bmp` and `.webp`, which the Rust reader has no decoder for, and not `.heic`, the iPhone's
//!   default, which it reads through ffmpeg (the M29 plan's D3).
//! - **A photo that raises is that photo's failure**, named on stderr and counted as unreadable,
//!   and the rest still run. The script's loop catches nothing, so its first unreadable file ended
//!   the run with a traceback and exit 1.
//!
//! # `--into-session S --player P --club C`: photo-only entries
//!
//! No Python twin: the program plan's §M29 gives this lab verb what the old §M35 gave the script.
//! Each photo becomes a **photo entry** in session `S`, a swing directory holding a shot photo and
//! no clip, which `storage::corpus` pools by its photo (ADR-024's M35 addendum), and is analysed
//! into its `shot_analysis.json` by `golf-core run-shot --session`'s own code
//! ([`golf_core::session::run_shot_session`]), so the verb and the reader cannot disagree about
//! what an entry needs (C7). In order, per photo:
//!
//! 1. **Dedupe by photo.** A photo the session already holds, in any swing, is skipped: an entry
//!    is opened once per photo, and the corpus would count a second one as a duplicate anyway.
//!    `--force` reads a photo entry's photo again and re-analyses it; a swing's photo is never
//!    touched, since its shot is read and analysed with its clips.
//! 2. **Open the entry by name**, `S/N` with `N` the store's next swing id, through
//!    [`SwingBundleStore::assign_from_path`] as the upload route places a file: the photo is copied
//!    to `.incoming/` with its modification time (the shot's timestamp is the photo's), then moved
//!    in. By name because the store's automatic rule would slot the photo into the newest swing
//!    lacking one, a filmed swing among them (the store's module doc). The golfer and club are the
//!    flags', and the intent is the session's cursor, as an upload stamps it.
//! 3. **Read the entry's own copy** with the swing's identity, `{S}-{N}`, as `_shot_for` stamps a
//!    bundle's shot, so the store's shot names the copy that stays rather than the file imported.
//! 4. **Analyse it** through `run_shot_session`, whose line is printed under the photo's row.
//!
//! **A photo nothing is read from opens no entry**: the one this run opened is deleted again. A
//! wrong number is worse than none (ADR-014), and an entry with no shot is one the corpus lists as
//! never analysed with a repair, `golf-core run-shot`, that would refuse it for holding no parse.
//! The golfer must be in the registry and the club must be one `parse_club` reads, each refused
//! with exit 2 for `parse_club`'s reason: a refused name costs a retype, and a wrong one files a
//! shot under a golfer nobody is, with no Rust verb to move it. A dry run reads each photo the
//! session does not hold, as the store-only dry run does, and opens, writes and analyses nothing.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use contracts::club::{parse_club, ClubId};
use contracts::intent::PracticeGoal;
use contracts::shot::ShotData;
use contracts::swing::{ANALYSIS_VERSION, COMPARABLE_FROM};
use contracts::Timestamp;
use golf_core::session::{run_shot_session, ShotDirs};
use screen::profile::{load_profile, DeviceProfile};
use storage::bundle_store::{SwingBundleStore, Upload};
use storage::corpus::EngineVersions;
use storage::golfer_store::{slugify, GolferStore};
use storage::manifest::{Role, SwingManifest};
use storage::session_meta::load_session_meta;
use storage::shot_store::{hash_image, ShotStore};

use crate::shot::{import_photo, Imported, NewImport, PhotoReader, ShotOptions};

/// `settings.shot_screens_dir`'s environment variable, where the script looks when given no path.
pub const SHOT_SCREENS_ENV: &str = "GOLF_SHOT_SCREENS_DIR";

/// The suffixes a photo is found by, lowercased: what `crates/ocr`'s `load_image` decodes (the
/// module doc). The script's `_IMAGE_SUFFIXES` is `.jpg .jpeg .png .bmp .webp`.
pub const IMAGE_SUFFIXES: [&str; 5] = ["jpg", "jpeg", "png", "heic", "heif"];

/// One column of the table: a shot's value for it, if the screen printed one.
type Metric = fn(&ShotData) -> Option<f64>;

/// The script's `_SUMMARY_FIELDS`: the metrics worth eyeballing in the table, in its order.
const SUMMARY_FIELDS: [(&str, Metric); 7] = [
    ("carry", |shot| shot.carry_distance),
    ("total", |shot| shot.total_distance),
    ("ball", |shot| shot.ball_speed),
    ("club", |shot| shot.club_head_speed),
    ("smash", |shot| shot.smash_factor),
    ("path", |shot| shot.club_path),
    ("face", |shot| shot.club_face_angle),
];

/// `settings.shot_screens_dir`: `GOLF_SHOT_SCREENS_DIR`, else `data/raw/shot_screens` under the
/// repository. An empty variable is unset, `golf_core::reports::data_dir`'s rule.
pub fn shot_screens_dir(env: impl Fn(&str) -> Option<String>) -> PathBuf {
    env(SHOT_SCREENS_ENV)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            crate::repo_root()
                .join("data")
                .join("raw")
                .join("shot_screens")
        })
}

/// One `import-photos` run, its flags read and its defaults resolved by the binary.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// Photos and directories of them, as given (the script's `paths or [shot_screens_dir]`).
    pub paths: Vec<PathBuf>,
    /// `--session`: what a store-only import files its shots under. `None` is the script's
    /// `screens-{today}`, today in UTC.
    pub session: Option<String>,
    /// The shot store (`--out`), the profile, the threshold, `--force` and `--dry-run`.
    pub options: ShotOptions,
    pub into: Option<IntoSession>,
}

/// `--into-session S --player P --club C`, and where the entries and golfers live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntoSession {
    pub session_id: String,
    /// As typed: a display name or an id, slugged as the career verbs slug `--name`.
    pub player: String,
    /// As typed, in any spelling `parse_club` reads.
    pub club: String,
    pub sessions_dir: PathBuf,
    pub golfers_dir: PathBuf,
}

/// Run `import-photos`, printing as it goes, and answer its exit code. `now` is the clock, read
/// once by the binary: the default session's date, and the entries' `created_at`.
pub fn import_photos(
    request: &Request,
    reader: &mut dyn PhotoReader,
    now: Timestamp,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> io::Result<u8> {
    let into = match request.into.as_ref().map(Entries::check).transpose() {
        Ok(into) => into,
        Err(refused) => {
            writeln!(err, "error: {refused}")?;
            return Ok(2);
        }
    };
    let images = images_in(&request.paths, err)?;
    if images.is_empty() {
        let given: Vec<String> = request
            .paths
            .iter()
            .map(|path| spelled(path).display().to_string())
            .collect();
        writeln!(err, "error: no images found in {}", given.join(", "))?;
        return Ok(1);
    }
    let profile = match load_profile(&request.options.device) {
        Ok(profile) => profile,
        Err(unknown) => {
            // `print(f"error: {exc}")` of a `KeyError`, whose `str` is its message's `repr`.
            writeln!(err, "error: {}", pyfmt::str_repr(&unknown.to_string()))?;
            return Ok(2);
        }
    };
    let session_id = request
        .session
        .clone()
        .unwrap_or_else(|| format!("screens-{}", now.date_ymd()));
    // `build_recognizer`, before any photo: the script fails fast when the machine cannot read.
    if let Err(why) = reader.ready() {
        writeln!(err, "error: {why}")?;
        return Ok(1);
    }

    let mut run = Run {
        options: &request.options,
        profile,
        reader,
        now,
        out,
        err,
        failed: 0,
        flagged: Vec::new(),
        not_analysed: 0,
    };
    match &into {
        None => writeln!(
            run.out,
            "\nImporting {} image(s) as session {} [{}]",
            images.len(),
            pyfmt::str_repr(&session_id),
            profile.device
        )?,
        Some(entries) => writeln!(
            run.out,
            "\nImporting {} image(s) into session {} as photo entries (golfer {}, club {}) [{}]",
            images.len(),
            pyfmt::str_repr(&entries.session_id),
            entries.player_id,
            entries.club.as_str(),
            profile.device
        )?,
    }
    for image in &images {
        match &into {
            None => run.store_only(image, &session_id)?,
            Some(entries) => run.file_into_session(image, entries)?,
        }
    }
    if let Some(entries) = &into {
        entries.tidy();
    }

    let Run {
        out,
        failed,
        flagged,
        not_analysed,
        ..
    } = run;
    for shot in &flagged {
        let provenance = shot
            .provenance
            .as_ref()
            .expect("a flagged shot has provenance");
        writeln!(
            out,
            "\n  {} needs review:",
            provenance.image_path.as_deref().unwrap_or("None")
        )?;
        for warning in &provenance.warnings {
            writeln!(out, "    - {warning}")?;
        }
    }
    if !request.options.dry_run {
        writeln!(
            out,
            "\nStore: {}",
            spelled(&request.options.shots_dir).display()
        )?;
        if let Some(entries) = &into {
            writeln!(out, "Session: {}", entries.session_dir().display())?;
        }
    }
    let clean = failed == 0 && flagged.is_empty() && not_analysed == 0;
    if !clean {
        match &into {
            None => writeln!(
                out,
                "\n{failed} unreadable, {} flagged for review.",
                flagged.len()
            )?,
            Some(_) => writeln!(
                out,
                "\n{failed} unreadable, {} flagged for review, {not_analysed} not analysed.",
                flagged.len()
            )?,
        }
        return Ok(1);
    }
    writeln!(out, "\n{} shot(s) imported cleanly.", images.len())?;
    Ok(0)
}

/// The script's `_images_in`: each directory's photos sorted, each file taken if its suffix is a
/// photo's, anything else warned about and skipped. A file named but missing is kept, so its read
/// says so.
fn images_in(paths: &[PathBuf], err: &mut dyn Write) -> io::Result<Vec<PathBuf>> {
    let is_photo = |path: &Path| {
        path.extension()
            .and_then(|suffix| suffix.to_str())
            .is_some_and(|suffix| IMAGE_SUFFIXES.contains(&suffix.to_lowercase().as_str()))
    };
    let mut found = Vec::new();
    for path in paths {
        if path.is_dir() {
            let mut children: Vec<PathBuf> = fs::read_dir(path)?
                .filter_map(|entry| Some(entry.ok()?.path()))
                .filter(|child| child.is_file() && is_photo(child))
                .map(|child| spelled(&child))
                .collect();
            children.sort_by_key(|child| sort_key(child));
            found.extend(children);
        } else if is_photo(path) {
            found.push(spelled(path));
        } else {
            writeln!(
                err,
                "warning: skipping {} (not a JPEG, PNG or HEIC photo)",
                spelled(path).display()
            )?;
        }
    }
    Ok(found)
}

/// A path as Python's `str(Path)` spells it: on Windows each `/` becomes `\`, and on both a doubled
/// separator or a `.` component goes. So the table, the store line and the `image_path` a shot is
/// stamped with read as the script's whichever way the shell passed the path (Git Bash passes
/// `C:/...`), and they name the same files. The bundle pipeline's refusals spell a swing directory
/// with it too (`pipeline`, M29 P25).
pub fn spelled(path: &Path) -> PathBuf {
    path.components().collect()
}

/// `sorted(path.iterdir())`'s order: a `WindowsPath` compares case-folded, a `PosixPath` as is.
fn sort_key(path: &Path) -> (String, String) {
    let name = path.to_string_lossy().into_owned();
    if cfg!(windows) {
        (name.to_lowercase(), name)
    } else {
        (name.clone(), name)
    }
}

/// The script's `_format_row`.
fn row(name: &str, status: &str, shot: Option<&ShotData>) -> String {
    let Some(shot) = shot else {
        return format!("  {name:<28} {status}");
    };
    let metrics: Vec<String> = SUMMARY_FIELDS
        .iter()
        .filter_map(|(label, value)| value(shot).map(|v| format!("{label}={}", pyfmt::g(v))))
        .collect();
    let confidence = shot
        .provenance
        .as_ref()
        .map_or(1.0, |provenance| provenance.parse_confidence);
    let flag = if needs_review(shot) { "  REVIEW" } else { "" };
    format!(
        "  {name:<28} {status:<7} conf={}{flag}\n      {}",
        pyfmt::fixed(confidence, 2),
        metrics.join(" ")
    )
}

fn needs_review(shot: &ShotData) -> bool {
    shot.provenance
        .as_ref()
        .is_some_and(|provenance| provenance.needs_review)
}

/// The run's state: what every photo is read with, where it prints, and the script's tallies.
struct Run<'a> {
    options: &'a ShotOptions,
    profile: &'a DeviceProfile,
    reader: &'a mut dyn PhotoReader,
    now: Timestamp,
    out: &'a mut dyn Write,
    err: &'a mut dyn Write,
    failed: usize,
    flagged: Vec<ShotData>,
    /// Photo entries whose analysis was refused or could not be written (`--into-session`).
    not_analysed: usize,
}

impl Run<'_> {
    /// One photo's row, and the tallies it moves: no shot is unreadable, a flagged one is listed.
    fn report(&mut self, image: &Path, status: &str, shot: Option<&ShotData>) -> io::Result<()> {
        writeln!(self.out, "{}", row(&crate::file_name(image), status, shot))?;
        match shot {
            None => self.failed += 1,
            Some(shot) if needs_review(shot) => self.flagged.push(shot.clone()),
            Some(_) => {}
        }
        Ok(())
    }

    /// `import_photo` over `read`, and the row for `image`, the photo as given, which `read` is
    /// unless an entry's copy is read in its place; a raise is the photo's failure, named on
    /// stderr.
    fn read(
        &mut self,
        image: &Path,
        read: &Path,
        new: NewImport<'_>,
    ) -> io::Result<Option<Imported>> {
        match import_photo(read, new, self.profile, self.options, &mut *self.reader) {
            Ok(imported) => {
                self.report(image, imported.status(), imported.shot())?;
                Ok(Some(imported))
            }
            Err(why) => {
                self.report(image, "failed", None)?;
                writeln!(self.err, "error: {why}")?;
                Ok(None)
            }
        }
    }

    /// The script's loop body: one photo into the store under `session_id`.
    fn store_only(&mut self, image: &Path, session_id: &str) -> io::Result<()> {
        let new = NewImport {
            session_id,
            shot_id: None,
        };
        self.read(image, image, new).map(drop)
    }

    /// One photo into the session as a photo entry (the module doc's four steps).
    fn file_into_session(&mut self, image: &Path, entries: &Entries) -> io::Result<()> {
        let bytes = match fs::read(image) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.report(image, "failed", None)?;
                return writeln!(
                    self.err,
                    "error: could not read image {}: {error}",
                    image.display()
                );
            }
        };
        let digest = hash_image(&bytes);
        let session = entries.session_id.as_str();

        if let Some(held) = entries.holding(&digest) {
            let at = format!("{session}/{}", held.swing_id);
            if held.roles.contains_key(&Role::FaceOn) {
                self.skipped(image, &digest)?;
                return self.entry_line(&format!(
                    "{at}: skipped: this session already holds the photo, in a swing with a \
                     face-on clip, whose shot is read and analysed with it"
                ));
            }
            if !self.options.force_ocr {
                self.skipped(image, &digest)?;
                return self.entry_line(&format!(
                    "{at}: skipped: this session already holds the photo (--force reads it again \
                     and re-analyses the entry)"
                ));
            }
            let copy = entries.photo_of(&held);
            let shot_id = format!("{session}-{}", held.swing_id);
            let new = NewImport {
                session_id: session,
                shot_id: Some(&shot_id),
            };
            let read = self.read(image, &copy, new)?;
            if self.options.dry_run {
                return self.entry_line(&format!(
                    "{at}: would be re-analysed (a dry run writes nothing)"
                ));
            }
            if read.as_ref().and_then(Imported::shot).is_some() {
                return self.analyse(entries, &held.swing_id);
            }
            return self.entry_line(&format!(
                "{at}: left as it was, since no shot was read from the photo this time"
            ));
        }

        if self.options.dry_run {
            let new = NewImport {
                session_id: session,
                shot_id: None,
            };
            self.read(image, image, new)?;
            return self.entry_line(&format!(
                "{session}: would open a photo entry for it (a dry run writes nothing)"
            ));
        }

        let opened = match entries.open(image, &digest, &bytes, self.now) {
            Ok(opened) => opened,
            Err(error) => {
                self.report(image, "failed", None)?;
                return writeln!(
                    self.err,
                    "error: could not open a photo entry in {session} for {}: {error}",
                    image.display()
                );
            }
        };
        let shot_id = format!("{session}-{}", opened.swing_id);
        let new = NewImport {
            session_id: session,
            shot_id: Some(&shot_id),
        };
        let read = self.read(image, &entries.photo_of(&opened), new)?;
        if read.as_ref().and_then(Imported::shot).is_some() {
            return self.analyse(entries, &opened.swing_id);
        }
        // Nothing was read, so the entry this run opened goes again (the module doc).
        if let Err(error) = entries.store.delete_swing(session, &opened.swing_id) {
            writeln!(
                self.err,
                "error: {session}/{} holds a photo no shot was read from, and could not be removed: \
                 {error}",
                opened.swing_id
            )?;
        }
        self.entry_line(&format!(
            "{session}: no photo entry opened, since no shot was read from the photo"
        ))
    }

    /// A photo the session already holds: its row, with whatever shot the store holds for it.
    fn skipped(&mut self, image: &Path, digest: &str) -> io::Result<()> {
        let stored = ShotStore::new(&self.options.shots_dir)
            .get(digest)
            .ok()
            .flatten();
        writeln!(
            self.out,
            "{}",
            row(&crate::file_name(image), "skipped", stored.as_ref())
        )?;
        if let Some(shot) = stored.filter(needs_review) {
            self.flagged.push(shot);
        }
        Ok(())
    }

    /// `golf-core run-shot --session S --swing N`, printed under the photo's row.
    fn analyse(&mut self, entries: &Entries, swing_id: &str) -> io::Result<()> {
        let answer = run_shot_session(
            &entries.session_id,
            Some(swing_id),
            &entries.dirs(&self.options.shots_dir),
            EngineVersions {
                installed: ANALYSIS_VERSION,
                comparable_from: COMPARABLE_FROM,
            },
        );
        for line in answer.stdout.lines() {
            self.entry_line(line)?;
        }
        self.err.write_all(answer.stderr.as_bytes())?;
        if answer.code != 0 {
            self.not_analysed += 1;
        }
        Ok(())
    }

    fn entry_line(&mut self, line: &str) -> io::Result<()> {
        writeln!(self.out, "      {line}")
    }
}

/// `--into-session`, checked: the session, the golfer's id, the club, the cursor's intent.
struct Entries {
    session_id: String,
    player_id: String,
    club: ClubId,
    intent: Option<PracticeGoal>,
    sessions_dir: PathBuf,
    golfers_dir: PathBuf,
    store: SwingBundleStore,
    /// Whether the session directory was there before this run, so one this run made and left
    /// empty (every photo unreadable) is removed rather than listed as a session.
    existed: bool,
}

impl Entries {
    /// The flags' checks, each a refusal the script's argparse would exit 2 on.
    fn check(into: &IntoSession) -> Result<Self, String> {
        let session_id = into.session_id.as_str();
        let plain = !session_id.is_empty()
            && !session_id.starts_with('.')
            && !session_id.contains(['/', '\\', ':']);
        if !plain {
            return Err(format!(
                "--into-session takes a session id, one directory name such as 2026-10-23, not {}",
                pyfmt::str_repr(session_id)
            ));
        }
        let Some(club) = parse_club(&into.club) else {
            // `golf-core club-profile`'s refusal, for `parse_club`'s reason.
            return Err(format!(
                "'{}' is not a club. Try '7i', '7 iron', 'driver' or 'pw'.\n'wedge' and 'iron' \
                 name a category rather than a club, so neither is accepted.",
                into.club
            ));
        };
        let player_id = slugify(&into.player);
        if player_id.is_empty() {
            return Err(format!(
                "name {} has no usable characters for an id",
                pyfmt::str_repr(&into.player)
            ));
        }
        let golfers = GolferStore::new(&into.golfers_dir);
        if golfers.get(&player_id).is_none() {
            let known: Vec<String> = golfers
                .list_all()
                .into_iter()
                .map(|golfer| golfer.player_id)
                .collect();
            return Err(format!(
                "golfer {} is not in the registry at {} ({})",
                pyfmt::str_repr(&player_id),
                spelled(&into.golfers_dir).display(),
                if known.is_empty() {
                    "it holds no golfer".to_string()
                } else {
                    format!("it holds {}", known.join(", "))
                }
            ));
        }
        let session_dir = into.sessions_dir.join(session_id);
        Ok(Entries {
            session_id: session_id.to_string(),
            player_id,
            club,
            intent: load_session_meta(&session_dir).intent,
            sessions_dir: spelled(&into.sessions_dir),
            golfers_dir: into.golfers_dir.clone(),
            store: SwingBundleStore::new(&into.sessions_dir),
            existed: session_dir.is_dir(),
        })
    }

    fn session_dir(&self) -> PathBuf {
        self.sessions_dir.join(&self.session_id)
    }

    fn dirs(&self, shots: &Path) -> ShotDirs {
        ShotDirs {
            sessions: self.sessions_dir.clone(),
            golfers: self.golfers_dir.clone(),
            shots: shots.to_path_buf(),
        }
    }

    /// The swing in this session already holding these bytes as its photo, if one does.
    fn holding(&self, digest: &str) -> Option<SwingManifest> {
        session_holding(&self.store, &self.session_id, digest)
    }

    /// Where a swing's photo is on disk.
    fn photo_of(&self, manifest: &SwingManifest) -> PathBuf {
        let filename = manifest
            .roles
            .get(&Role::ShotScreen)
            .map(|photo| photo.filename.as_str())
            .unwrap_or_default();
        self.session_dir().join(&manifest.swing_id).join(filename)
    }

    /// Open a photo entry for `image` under the next swing id: copied into `.incoming/` with the
    /// photo's modification time, then placed by the store as the upload route places a file.
    fn open(
        &self,
        image: &Path,
        digest: &str,
        bytes: &[u8],
        now: Timestamp,
    ) -> io::Result<SwingManifest> {
        let incoming = self.sessions_dir.join(".incoming");
        fs::create_dir_all(&incoming)?;
        let tmp = incoming.join(format!("{digest}.{}.part", std::process::id()));
        fs::write(&tmp, bytes)?;
        let placed = fs::metadata(image)
            .and_then(|meta| meta.modified())
            .and_then(|modified| {
                fs::File::options()
                    .write(true)
                    .open(&tmp)?
                    .set_modified(modified)
            })
            .and_then(|()| {
                let swing_id = self.store.next_swing_id(&self.session_id);
                let original = crate::file_name(image);
                self.store.assign_from_path(
                    &Upload {
                        session_id: &self.session_id,
                        role: Role::ShotScreen,
                        tmp_path: &tmp,
                        digest,
                        original_filename: &original,
                        content_type: content_type(image),
                        size_bytes: i64::try_from(bytes.len()).unwrap_or(i64::MAX),
                        swing_id: Some(&swing_id),
                        player_id: Some(&self.player_id),
                        club: Some(self.club),
                        intent: self.intent.as_ref(),
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
        self.store
            .get_swing(&placed.session_id, &placed.swing_id)
            .ok_or_else(|| io::Error::other("the entry's manifest could not be read back"))
    }

    /// Remove the session directory if this run made it and left it empty.
    fn tidy(&self) {
        if !self.existed {
            // `remove_dir` refuses a directory that holds anything, which is the check.
            let _ = fs::remove_dir(self.session_dir());
        }
    }
}

/// The swing in `session_id` already holding these bytes as its photo, if one does: the dedupe the
/// store's automatic rule makes, which opening an entry by name skips. `import-photos
/// --into-session` opens an entry once per photo by it, and `import-export` (P31) skips an exported
/// photo entry by it, so the two cannot disagree about which photo a session holds.
pub(crate) fn session_holding(
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
fn content_type(image: &Path) -> &'static str {
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
