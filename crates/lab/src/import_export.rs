//! `golf-lab import-export`: the phone's export merged into the lab's stores. No Python twin: the
//! frozen lab never had a phone to import from. [M29 P31]
//!
//! # What an export is
//!
//! The M29 plan's D16: **the stored layout itself**, a zip or a directory holding the phone's
//! `data/processed` subtree as `crates/storage` writes it, `sessions/`, `shots/` and `golfers/`, and
//! since M46 P21 `drills/`, so there is no export contract to drift from the stores' own. M38 P1's `export_sessions` writes it,
//! and M38 P4's exit compares the phone's corpus with Rust's `read_corpus` over what this wrote;
//! frozen Python's corpus is not the reference, since it leaves photo entries out by design (the
//! program plan's §M38). An archive or directory whose files all sit in one folder that is none of
//! the four (a zipped `processed/`) is read below that folder.
//!
//! # The merge, file by file (the plan's C11)
//!
//! Each file is placed where the lab's store keeps it, `sessions/S/N/manifest.json` at
//! `--sessions-dir`'s `S/N/manifest.json` and so on, and judged against what is stored there:
//!
//! - **new**: nothing is stored at that path, so it is copied, keeping the export's modification
//!   time where the export records one (a shot photo's time is its shot's when the photo is read);
//! - **identical**: the same bytes are stored there, so it is skipped, and a re-import of an export
//!   copies nothing;
//! - **a conflict**: other bytes are stored there. **One conflict refuses the whole import**: every
//!   such path is named and nothing is written, because a merge that wrote the rest would leave the
//!   stores holding part of an export that someone has to reconcile by hand anyway. Bytes, not
//!   values, for every file but the two below: a session cursor the phone wrote for itself differs
//!   from the lab's in its timestamps, and which of the two is the session's is not this verb's to
//!   guess.
//!
//! **A golfer and a bag are the two with a value rule** (M38's D6, a dated addition to C11), because
//! the phone creates its own golfer, and by bytes its first export would refuse at any lab that knew
//! him. Each applies only where a file differs by bytes, and only to a file the store lists, directly
//! under `golfers/`; below that it goes by bytes:
//!
//! - **A golfer** (`*.golfer.json`) is read on both sides, the stored one by the store's own reader.
//!   The same `player_id` and handedness is one person, and is **kept** as the lab's, with a line
//!   naming each field that differs and both its values, so what the phone said is seen rather than
//!   lost. A different `player_id` or handedness, or a file that does not read as a golfer on either
//!   side, stays a conflict: handedness is the frame of every signed metric, so which of two is true
//!   is not this verb's to guess.
//! - **A bag** (`*.bag.json`) is **kept** as the lab's, with a line saying the phone's was not
//!   applied, and is parsed on neither side, since nothing here would use what it said. Something
//!   other than a file stored at its name holds no bag to keep, and is a conflict.
//!
//! Either's archive entry is still read through, so a checksum that fails is exit 2 as it is for any
//! file, though a kept file's bytes land nowhere.
//!
//! **Two dedupes by photo**, each where the stores already have a rule for one photo held twice:
//!
//! - **A shot** is placed at [`ShotStore::path_for`] of [`ShotStore::key_for`], the store's own key,
//!   which is the photo's sha256 when the shot names one; never at the export's file name. A shot
//!   of a photo the store holds a shot of already is **kept as stored** even when it reads
//!   differently (another reader, or a re-read since), because a shot keyed by its photo is that
//!   photo's, and the store answers `cached` for a photo it has parsed (`import-photos`). A shot
//!   keyed by its `shot_id`, with no photo hash, has no such rule, and a difference is a conflict.
//! - **A photo entry**, a swing directory whose manifest holds a shot photo and no face-on clip,
//!   whose photo its session holds already in any swing, is **skipped whole as a duplicate** and the
//!   stored swing kept: `import-photos --into-session`'s rule, an entry opened once per photo (P24),
//!   which the corpus would apply anyway by listing the second as a `duplicate`. The same swing id
//!   stored with files missing and none different is no duplicate but the same entry, and the
//!   missing files are copied (an entry the lab never analysed gains its `shot_analysis.json`). A
//!   swing earlier in the same export holds its photo as a stored one does. A filmed swing is never
//!   deduped this way, since its clips are what the corpus prefers (ADR-024's M35 addendum).
//!
//! Either is printed, so a correction made on the phone that the stored copy lacks is seen rather
//! than lost quietly.
//!
//! # A drill (M46 P21, the M46 plan's C17)
//!
//! Each golfer's tape drill, `drills/<player_id>/impact/strike_tape/`, is **merged** rather than
//! copied, through the lab's own [`DrillStore`], whose readers judge the export's bytes as they judge
//! the store's own files ([`DrillStore::parse_strikes`], [`DrillStore::parse_state`]):
//!
//! - **`strikes.jsonl`, by `strike_id`.** A strike the store does not hold is appended, in the
//!   export's order, and **a held id is kept** as the store holds it, as a shot of a stored photo is.
//!   A line the store would skip (torn, refused, another golfer's, an id repeated) is skipped here
//!   too and counted, never refused: the file is append-only on the phone, so a line torn there is
//!   torn in every export after it, and refusing it would refuse them all.
//! - **`state.json`, by `updated_at`.** The export's replaces the store's only when it was written
//!   later, and through the store's writer, so this is the one file the verb replaces. One the export
//!   holds that does not read is exit 2, as a torn shot is. One stored that does not read is a
//!   conflict, since the store's writer refuses to write over it and so does this verb: it holds the
//!   counters every id is drawn from (`storage::drill_store`'s module doc).
//! - **`crops/`.** A crop the store lacks is copied, and one it holds stays, by the held id's rule.
//! - **`last/`**, each club's last crop, is the phone's reference for that club's next photo. It is
//!   ignored and counted, since the lab takes no photo and the crops are the strikes' record.
//!
//! Each of the first three prints one line with its counts, and any other file of the tree goes by
//! bytes. The strikes and the state are written after every copied file, the drill's crops among
//! them, and the state last, so a strike never names a crop not yet stored, nor a state a strike.
//! A re-run after a failure part way appends what is missing.
//!
//! # What is refused before anything is compared
//!
//! Exit 2 and nothing written, every reason listed: a path that is not a plain relative one (`..`,
//! an absolute name, a `\` or `:` in a name), a symbolic link, an encrypted entry or one compressed
//! other than stored or deflated, a name the archive holds twice, a file outside the four trees, a
//! shot that does not read as a shot, a manifest that does not read as one or names a file its
//! directory lacks, a drill under a name that is not a golfer's id or whose state does not read, and
//! an archive entry whose checksum fails. Those are the files whose reading
//! the rules above need, and a torn export is caught before it is half-copied; the other files go
//! by bytes, and the stores' tolerant readers judge them as they judge any file. **Ignored** and
//! counted, never copied: a name starting with `.` anywhere in its path (`.DS_Store`, the bundle
//! store's `.incoming/`), anything under `__MACOSX/`, and a `.tmp` or `.part` left by a write that
//! did not finish.
//!
//! # The write
//!
//! Golfers, then shots, then sessions, a swing's `manifest.json` last in its directory, since a swing
//! directory with no manifest is one no reader lists, then each drill's strikes and state (above): an import stopped part way leaves nothing a
//! reader half-sees, and a re-run copies the rest, because what was copied is identical by then.
//! Each file is written beside its place as `.part` and renamed, refusing a file that appeared there
//! since it was compared. No time is stamped and no shot is re-read: an imported file is the
//! phone's bytes, which is what makes the phone's corpus and the lab's comparable.
//!
//! **Exit codes**: 0 every new file copied (or, in a dry run, copyable), with nothing in conflict; 1
//! the import refused on a conflict, nothing written; 2 the export cannot be read as a stored
//! layout, nothing written, or a write failed part way (named, with what was copied before it).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use contracts::drill::{crop_file, DrillState, DrillTopic, TapeMethod, TapeStrike};
use contracts::golfer::Golfer;
use contracts::shot::ShotData;
use golf_core::session::session_holding;
use storage::bag_store;
use storage::bundle_store::SwingBundleStore;
use storage::drill_store::{DrillStore, DRILLS_DIR, LAST_DIR, STATE_NAME, STRIKES_NAME};
use storage::golfer_store::{self, GolferStore};
use storage::manifest::{Role, SwingManifest, MANIFEST_NAME};
use storage::shot_store::{self, ShotStore};
use storage::StoreError;
use zip::extra_fields::ExtraField;
use zip::{CompressionMethod, ZipArchive};

use crate::import::spelled;

/// The four trees an export holds, the stores' directory names under `data/processed`: the drills'
/// since M46 P21.
pub const TREES: [&str; 4] = ["sessions", "shots", "golfers", DRILLS_DIR];

/// The bytes compared at a time, so a clip is never read whole to learn it is stored already.
const CHUNK: usize = 64 * 1024;

/// Windows' `FILETIME` epoch, 1601-01-01, in its 100 ns ticks before the Unix epoch: the NTFS
/// extra field's time, which 7-Zip and Windows tools write.
const FILETIME_TO_UNIX: u64 = 116_444_736_000_000_000;

/// One `import-export` run, its flags read and its directories resolved by the binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The export: a zip archive, or a directory.
    pub export: PathBuf,
    pub sessions_dir: PathBuf,
    pub shots_dir: PathBuf,
    pub golfers_dir: PathBuf,
    /// The `drills/` directory, which each golfer's drill is opened under (`DrillStore::in_drills`).
    pub drills_dir: PathBuf,
    /// Judge every file and print the report, and write nothing.
    pub dry_run: bool,
}

/// Run `import-export`, the report to `out` and every refusal to `err`, and answer its exit code.
/// An `Err` is only a report that could not be written.
pub fn import_export(
    request: &Request,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> io::Result<u8> {
    let shown = spelled(&request.export);
    let mut export = match Export::open(&request.export) {
        Ok(export) => export,
        Err(why) => {
            writeln!(err, "error: {}: {why}", shown.display())?;
            return Ok(2);
        }
    };
    writeln!(out, "\nImporting the export at {}", shown.display())?;
    for (tree, dir) in TREES.iter().zip([
        &request.sessions_dir,
        &request.shots_dir,
        &request.golfers_dir,
        &request.drills_dir,
    ]) {
        writeln!(out, "  {tree} into {}", spelled(dir).display())?;
    }

    let plan = Plan::make(&mut export, request);
    if !plan.problems.is_empty() {
        writeln!(
            err,
            "error: nothing was imported, because the export cannot be read as a stored layout:"
        )?;
        for problem in &plan.problems {
            writeln!(err, "  {problem}")?;
        }
        return Ok(2);
    }
    if !plan.conflicts.is_empty() {
        writeln!(
            err,
            "error: nothing was imported, because {} file(s) in the export differ from what is \
             stored at the same place:",
            plan.conflicts.len()
        )?;
        for conflict in &plan.conflicts {
            writeln!(err, "  {conflict}")?;
        }
        return Ok(1);
    }
    for line in &plan.lines {
        writeln!(out, "  {line}")?;
    }
    let tally = format!(
        "{} identical, {} left as stored, {} ignored",
        plan.identical, plan.left, plan.ignored
    );
    let total = plan.writes.len() + plan.drill_writes.len();
    if request.dry_run {
        writeln!(
            out,
            "\nDry run: {total} file(s) would be copied; {tally}. Nothing was written."
        )?;
        return Ok(0);
    }
    let failed = |err: &mut dyn Write, dest: &Path, why: &dyn fmt::Display, done: usize| {
        writeln!(
            err,
            "error: writing {}: {why}. {done} of {total} file(s) were copied before it; a re-run \
             copies the rest, since those are stored identically now.",
            dest.display(),
        )
    };
    for (done, (item, dest)) in plan.writes.iter().enumerate() {
        if let Err(why) = place(&mut export, *item, dest) {
            failed(err, dest, &why, done)?;
            return Ok(2);
        }
    }
    for (done, write) in plan.drill_writes.iter().enumerate() {
        if let Err(why) = write.run() {
            failed(err, &write.dest(), &why, plan.writes.len() + done)?;
            return Ok(2);
        }
    }
    writeln!(out, "\n{total} file(s) copied; {tally}.")?;
    Ok(0)
}

/// Where the export's bytes are read from.
struct Export {
    /// `None` for a directory, whose items name their own paths.
    zip: Option<ZipArchive<fs::File>>,
    items: Vec<Item>,
    /// What the listing refused, before any rule ran.
    problems: Vec<String>,
}

/// One file of the export.
struct Item {
    /// Its path below the layout's root, one name per component.
    parts: Vec<String>,
    origin: Origin,
    size: u64,
    /// The time the export records for it, if it records one this can read as UTC.
    modified: Option<SystemTime>,
}

enum Origin {
    File(PathBuf),
    Entry(usize),
}

impl Item {
    /// As the report prints it, `/`-joined on every platform.
    fn rel(&self) -> String {
        self.parts.join("/")
    }
}

impl Export {
    /// A directory is walked; anything else must open as a zip archive.
    fn open(path: &Path) -> Result<Self, String> {
        let meta = fs::metadata(path).map_err(|error| error.to_string())?;
        let mut problems = Vec::new();
        let (zip, mut items) = if meta.is_dir() {
            let items = list_dir(path, &mut problems).map_err(|error| error.to_string())?;
            (None, items)
        } else {
            let file = fs::File::open(path).map_err(|error| error.to_string())?;
            let mut archive = ZipArchive::new(file).map_err(|error| {
                format!("neither a directory nor a zip archive this reads ({error})")
            })?;
            let items = list_zip(&mut archive, &mut problems);
            (Some(archive), items)
        };
        unwrap_folder(&mut items);
        items.sort_by(|a, b| a.parts.cmp(&b.parts));
        Ok(Export {
            zip,
            items,
            problems,
        })
    }

    fn read(&mut self, item: usize) -> io::Result<Box<dyn Read + '_>> {
        match &self.items[item].origin {
            Origin::File(path) => Ok(Box::new(fs::File::open(path)?)),
            Origin::Entry(index) => {
                let archive = self
                    .zip
                    .as_mut()
                    .ok_or_else(|| io::Error::other("an archive entry listed from a directory"))?;
                let entry = archive.by_index(*index).map_err(io::Error::other)?;
                Ok(Box::new(entry))
            }
        }
    }

    fn bytes(&mut self, item: usize) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        self.read(item)?.read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    /// Read an archive entry through, which is when its checksum is checked, so a torn archive is
    /// refused before any write rather than part way through them. A directory's file is not read.
    fn check(&mut self, item: usize) -> io::Result<()> {
        if matches!(self.items[item].origin, Origin::Entry(_)) {
            io::copy(&mut self.read(item)?, &mut io::sink())?;
        }
        Ok(())
    }
}

/// Every file under `root`, refusing what is neither a file nor a directory, as no store writes one.
fn list_dir(root: &Path, problems: &mut Vec<String>) -> io::Result<Vec<Item>> {
    let mut items = Vec::new();
    let mut stack = vec![(root.to_path_buf(), Vec::<String>::new())];
    while let Some((dir, parts)) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let mut here = parts.clone();
            match entry.file_name().into_string() {
                Ok(name) => here.push(name),
                Err(name) => {
                    here.push(name.to_string_lossy().into_owned());
                    problems.push(format!("{}: a name that is not UTF-8", here.join("/")));
                    continue;
                }
            }
            let kind = entry.file_type()?;
            if kind.is_dir() {
                stack.push((entry.path(), here));
            } else if kind.is_file() {
                let meta = entry.metadata()?;
                items.push(Item {
                    parts: here,
                    origin: Origin::File(entry.path()),
                    size: meta.len(),
                    modified: meta.modified().ok(),
                });
            } else {
                problems.push(format!(
                    "{}: neither a file nor a directory (a link, which no store writes)",
                    here.join("/")
                ));
            }
        }
    }
    Ok(items)
}

/// Every file entry of the archive, refusing each one no store could have written.
fn list_zip(archive: &mut ZipArchive<fs::File>, problems: &mut Vec<String>) -> Vec<Item> {
    let mut items = Vec::new();
    let mut seen = BTreeSet::new();
    for index in 0..archive.len() {
        let entry = match archive.by_index_raw(index) {
            Ok(entry) => entry,
            Err(error) => {
                problems.push(format!("entry {index} of the archive: {error}"));
                continue;
            }
        };
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        let parts: Vec<String> = name.split('/').map(str::to_string).collect();
        let refusal = if entry.enclosed_name().is_none() || !parts.iter().all(|part| plain(part)) {
            Some("not a plain relative path".to_string())
        } else if entry.is_symlink() {
            Some("a symbolic link, which no store writes".to_string())
        } else if entry.encrypted() {
            Some("encrypted".to_string())
        } else if !matches!(
            entry.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) {
            Some(format!(
                "compressed as {:?}, where only stored and deflated entries are read",
                entry.compression()
            ))
        } else if !seen.insert(name.clone()) {
            Some("in the archive twice".to_string())
        } else {
            None
        };
        if let Some(why) = refusal {
            problems.push(format!("{name}: {why}"));
            continue;
        }
        items.push(Item {
            modified: zip_modified(entry.extra_data_fields()),
            size: entry.size(),
            origin: Origin::Entry(index),
            parts,
        });
    }
    items
}

/// The entry's time, from the extended-timestamp field (Info-ZIP, macOS and Linux tools) or the NTFS
/// one (Windows tools), both UTC. The DOS time every entry carries is local time with no zone, so a
/// file whose entry has neither keeps the time it is written at.
fn zip_modified<'a>(mut fields: impl Iterator<Item = &'a ExtraField>) -> Option<SystemTime> {
    fields.find_map(|field| match field {
        ExtraField::ExtendedTimestamp(stamp) => stamp
            .mod_time()
            .map(|seconds| UNIX_EPOCH + Duration::from_secs(seconds.into())),
        ExtraField::Ntfs(ntfs) => ntfs
            .mtime()
            .checked_sub(FILETIME_TO_UNIX)
            .map(|ticks| UNIX_EPOCH + Duration::from_nanos(ticks.saturating_mul(100))),
    })
}

/// One name of a relative path that means only itself on Windows and on Unix.
fn plain(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name
            .chars()
            .any(|ch| matches!(ch, '/' | '\\' | ':') || ch.is_control())
}

/// What no store reads, and so nothing copies: the module doc's list.
fn ignored(parts: &[String]) -> bool {
    parts.first().is_some_and(|first| first == "__MACOSX")
        || parts.iter().any(|part| part.starts_with('.'))
        || parts
            .last()
            .is_some_and(|last| last.ends_with(".tmp") || last.ends_with(".part"))
}

/// Read below one wrapping folder, when every file that counts sits in it and it is not a tree.
fn unwrap_folder(items: &mut [Item]) {
    let mut counted = items.iter().filter(|item| !ignored(&item.parts));
    let Some(first) = counted.next().map(|item| item.parts[0].clone()) else {
        return;
    };
    let wrapped = !TREES.contains(&first.as_str())
        && items
            .iter()
            .filter(|item| !ignored(&item.parts))
            .all(|item| item.parts[0] == first && item.parts.len() > 1);
    if wrapped {
        for item in items
            .iter_mut()
            .filter(|item| item.parts[0] == first && item.parts.len() > 1)
        {
            item.parts.remove(0);
        }
    }
}

/// How the file at a destination compares with the export's.
enum Stored {
    Absent,
    Same,
    Different,
}

/// Compare the export's file with what is stored at `dest`, reading neither past a difference.
fn stored(export: &mut Export, item: usize, dest: &Path) -> io::Result<Stored> {
    let meta = match fs::metadata(dest) {
        Ok(meta) => meta,
        // Under a file, nothing is stored at the path either: Windows says `NotFound` and Unix
        // `NotADirectory`, and both are left to the write to refuse, so a run reads alike on both.
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            export.check(item)?;
            return Ok(Stored::Absent);
        }
        Err(error) => return Err(error),
    };
    if !meta.is_file() || meta.len() != export.items[item].size {
        return Ok(Stored::Different);
    }
    let mut theirs = export.read(item)?;
    let mut ours = fs::File::open(dest)?;
    let (mut left, mut right) = (vec![0u8; CHUNK], vec![0u8; CHUNK]);
    loop {
        let n = fill(&mut theirs, &mut left)?;
        let m = fill(&mut ours, &mut right)?;
        if left[..n] != right[..m] {
            return Ok(Stored::Different);
        }
        if n < CHUNK {
            return Ok(Stored::Same);
        }
    }
}

/// Read until `buf` is full or the reader ends: `read_exact` that answers a short count at the end.
fn fill(reader: &mut dyn Read, buf: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match reader.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok(filled)
}

/// Every file judged, before anything is written.
#[derive(Default)]
struct Plan {
    /// The report's lines, one per golfer file, shot, session file and swing, and per drill one for
    /// its crops, its strikes and its state, in the write order.
    lines: Vec<String>,
    /// What to copy, an item and its place, in the order the module doc gives.
    writes: Vec<(usize, PathBuf)>,
    /// What each drill's merge writes through the lab's store, after every copy (the module doc).
    drill_writes: Vec<DrillWrite>,
    conflicts: Vec<String>,
    problems: Vec<String>,
    identical: usize,
    /// Files a rule left as stored: a kept shot, golfer or bag, a duplicate entry's files, a drill's
    /// strikes or state kept, and a crop held with other bytes.
    left: usize,
    ignored: usize,
}

impl Plan {
    fn make(export: &mut Export, request: &Request) -> Plan {
        let mut plan = Plan {
            problems: std::mem::take(&mut export.problems),
            ..Plan::default()
        };
        let (mut golfers, mut shots, mut sessions, mut drills) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for (index, item) in export.items.iter().enumerate() {
            if ignored(&item.parts) {
                plan.ignored += 1;
                continue;
            }
            match item.parts[0].as_str() {
                "golfers" if item.parts.len() > 1 => golfers.push(index),
                "shots" if item.parts.len() > 1 => shots.push(index),
                "sessions" if item.parts.len() > 1 => sessions.push(index),
                DRILLS_DIR if item.parts.len() > 1 => drills.push(index),
                _ => plan.problems.push(format!(
                    "{}: outside sessions/, shots/, golfers/ and drills/, which are all an export \
                     holds",
                    item.rel()
                )),
            }
        }
        let empty = [&golfers, &shots, &sessions, &drills]
            .iter()
            .all(|tree| tree.is_empty());
        if empty && plan.problems.is_empty() {
            plan.problems
                .push("it holds no file under sessions/, shots/, golfers/ or drills/".to_string());
        }
        if !plan.problems.is_empty() {
            return plan;
        }

        let registry = GolferStore::new(&request.golfers_dir);
        for index in golfers {
            plan.golfers_file(export, index, &registry);
        }
        let store = ShotStore::new(&request.shots_dir);
        let mut placed = BTreeMap::new();
        for index in shots {
            plan.shot(export, index, &store, &mut placed);
        }
        plan.sessions(export, sessions, &request.sessions_dir);
        plan.drills(export, drills, &request.drills_dir);
        plan
    }

    /// The plain rule: new, identical, or a conflict.
    fn file(&mut self, export: &mut Export, index: usize, dest: PathBuf) {
        if self.differs(export, index, &dest) {
            let rel = export.items[index].rel();
            self.conflicts.push(conflict(&rel, &dest));
        }
    }

    /// A file new or identical is settled here. `true` is one stored with other bytes, which is the
    /// caller's rule to judge.
    fn differs(&mut self, export: &mut Export, index: usize, dest: &Path) -> bool {
        let rel = export.items[index].rel();
        match stored(export, index, dest) {
            Ok(Stored::Absent) => {
                self.lines.push(format!("new       {rel}"));
                self.writes.push((index, dest.to_path_buf()));
                false
            }
            Ok(Stored::Same) => {
                self.lines.push(format!("identical {rel}"));
                self.identical += 1;
                false
            }
            Ok(Stored::Different) => true,
            Err(why) => {
                self.problems.push(format!("{rel}: {why}"));
                false
            }
        }
    }

    /// A file of `golfers/`: a difference goes by the module doc's value rule (M38's D6) when the
    /// file is one the stores list, and by bytes otherwise.
    fn golfers_file(&mut self, export: &mut Export, index: usize, registry: &GolferStore) {
        let dest = below(registry.root(), &export.items[index].parts[1..]);
        if !self.differs(export, index, &dest) {
            return;
        }
        let rel = export.items[index].rel();
        let name = match export.items[index].parts.as_slice() {
            [_, name] => name.clone(),
            _ => String::new(),
        };
        let golfer_id = name.strip_suffix(golfer_store::SUFFIX);
        if golfer_id.is_none() && !(name.ends_with(bag_store::SUFFIX) && dest.is_file()) {
            return self.conflicts.push(conflict(&rel, &dest));
        }
        // `stored` learns a difference from the size alone and reads no further, and a `kept` file
        // is never written, so nothing else would notice a torn entry. The module doc refuses one
        // before anything is compared, and that is exit 2 here too.
        let bytes = match export.bytes(index) {
            Ok(bytes) => bytes,
            Err(why) => return self.problems.push(format!("{rel}: {why}")),
        };
        let kept = match golfer_id {
            Some(id) => registry
                .get(id)
                .zip(serde_json::from_slice::<Golfer>(&bytes).ok())
                .and_then(|(ours, theirs)| golfer_difference(&ours, &theirs))
                .map(|difference| {
                    format!("the store holds this golfer already, and it stays; {difference}")
                }),
            None => Some(
                "the store holds this golfer's bag already, which differs, and it stays; the \
                 phone's bag was not applied"
                    .to_string(),
            ),
        };
        match kept {
            Some(why) => {
                self.lines.push(format!("kept      {rel}: {why}"));
                self.left += 1;
            }
            None => self.conflicts.push(conflict(&rel, &dest)),
        }
    }

    /// A shot, placed by the store's key and kept as stored when the store holds its photo's shot.
    fn shot(
        &mut self,
        export: &mut Export,
        index: usize,
        store: &ShotStore,
        placed: &mut BTreeMap<PathBuf, String>,
    ) {
        let rel = export.items[index].rel();
        let named_as_shot =
            export.items[index].parts.len() == 2 && rel.ends_with(shot_store::SUFFIX);
        if !named_as_shot {
            // Not a file the store reads (`ShotStore::all` lists `*.shot.json` alone): bytes.
            let dest = below(store.root(), &export.items[index].parts[1..]);
            return self.file(export, index, dest);
        }
        let shot: ShotData = match export
            .bytes(index)
            .map_err(|error| error.to_string())
            .and_then(|bytes| serde_json::from_slice(&bytes).map_err(|error| error.to_string()))
        {
            Ok(shot) => shot,
            Err(why) => {
                self.problems
                    .push(format!("{rel}: not a shot the shot store reads ({why})"));
                return;
            }
        };
        // `key_for`'s own test: a shot names its photo when its hash is there and not empty.
        let by_photo = shot
            .provenance
            .as_ref()
            .and_then(|provenance| provenance.image_sha256.as_deref())
            .is_some_and(|digest| !digest.is_empty());
        let dest = store.path_for(ShotStore::key_for(&shot));
        if let Some(first) = placed.insert(dest.clone(), rel.clone()) {
            self.conflicts.push(format!(
                "{rel} -> {}, where {first} in the export goes too: two shots under one key",
                dest.display()
            ));
            return;
        }
        let stored_as = crate::file_name(&dest);
        let renamed =
            if Some(stored_as.as_str()) == export.items[index].parts.last().map(String::as_str) {
                String::new()
            } else {
                format!(" (stored as {stored_as}, the store's name for its key)")
            };
        match stored(export, index, &dest) {
            Ok(Stored::Absent) => {
                self.lines.push(format!("new       {rel}{renamed}"));
                self.writes.push((index, dest));
            }
            Ok(Stored::Same) => {
                self.lines.push(format!("identical {rel}{renamed}"));
                self.identical += 1;
            }
            Ok(Stored::Different) if by_photo => {
                self.lines.push(format!(
                    "kept      {rel}: the store holds a shot of this photo already, which reads \
                     differently, and it stays"
                ));
                self.left += 1;
            }
            Ok(Stored::Different) => self.conflicts.push(conflict(&rel, &dest)),
            Err(why) => self.problems.push(format!("{rel}: {why}")),
        }
    }

    /// `sessions/`: a file of the tree or of a session goes by the plain rule, and each swing
    /// directory, `sessions/S/N/`, is judged whole.
    fn sessions(&mut self, export: &mut Export, indices: Vec<usize>, root: &Path) {
        let mut swings: BTreeMap<(String, SwingOrder), Vec<usize>> = BTreeMap::new();
        for index in indices {
            let parts = &export.items[index].parts;
            if parts.len() >= 4 {
                let key = (parts[1].clone(), swing_order(&parts[2]));
                swings.entry(key).or_default().push(index);
            } else {
                let dest = below(root, &parts[1..]);
                self.file(export, index, dest);
            }
        }
        let store = SwingBundleStore::new(root);
        // The photos this import places, by session, so an export's own second entry for one
        // photo is a duplicate of its first, as it would be of a stored one.
        let mut placing: BTreeSet<(String, String)> = BTreeSet::new();
        for ((session, (_, _, swing)), indices) in swings {
            self.swing(
                export,
                &store,
                &mut placing,
                &session,
                &swing,
                indices,
                root,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn swing(
        &mut self,
        export: &mut Export,
        store: &SwingBundleStore,
        placing: &mut BTreeSet<(String, String)>,
        session: &str,
        swing: &str,
        mut indices: Vec<usize>,
        root: &Path,
    ) {
        let rel = format!("sessions/{session}/{swing}");
        let is_manifest = |item: &Item| item.parts.len() == 4 && item.parts[3] == MANIFEST_NAME;
        // The manifest last, so a swing directory is listed by no reader until it is whole.
        indices.sort_by_key(|&index| is_manifest(&export.items[index]));
        let manifest = match indices
            .iter()
            .find(|&&index| is_manifest(&export.items[index]))
        {
            None => None,
            Some(&index) => match export
                .bytes(index)
                .map_err(|error| error.to_string())
                .and_then(|bytes| {
                    serde_json::from_slice::<SwingManifest>(&bytes)
                        .map_err(|error| error.to_string())
                }) {
                Ok(manifest) => Some(manifest),
                Err(why) => {
                    self.problems.push(format!(
                        "{rel}/{MANIFEST_NAME}: not a manifest the store reads ({why})"
                    ));
                    return;
                }
            },
        };
        if let Some(manifest) = &manifest {
            let held: BTreeSet<&str> = indices
                .iter()
                .filter(|&&index| export.items[index].parts.len() == 4)
                .map(|&index| export.items[index].parts[3].as_str())
                .collect();
            let missing: Vec<&str> = manifest
                .roles
                .values()
                .map(|file| file.filename.as_str())
                .filter(|filename| !held.contains(filename))
                .collect();
            if !missing.is_empty() {
                self.problems.push(format!(
                    "{rel}: {MANIFEST_NAME} names {}, which the export does not hold",
                    missing.join(", ")
                ));
                return;
            }
        }

        let mut judged = Vec::with_capacity(indices.len());
        for index in indices {
            let dest = below(root, &export.items[index].parts[1..]);
            match stored(export, index, &dest) {
                Ok(state) => judged.push((index, dest, state)),
                Err(why) => self
                    .problems
                    .push(format!("{}: {why}", export.items[index].rel())),
            }
        }
        let files = judged.len();
        if judged
            .iter()
            .all(|(_, _, state)| matches!(state, Stored::Same))
        {
            self.lines
                .push(format!("identical {rel} ({files} file(s))"));
            self.identical += files;
            return;
        }

        let photo = manifest
            .as_ref()
            .and_then(|manifest| manifest.roles.get(&Role::ShotScreen))
            .map(|photo| photo.content_sha256.clone());
        let filmed = manifest
            .as_ref()
            .is_some_and(|manifest| manifest.roles.contains_key(&Role::FaceOn));
        if let Some(digest) = photo.as_ref().filter(|_| !filmed) {
            // The same swing stored with files missing and none different is filled in rather than
            // deduped: nothing stored would change, and the export has what the store lacks (an
            // entry the lab never analysed, say).
            let differs = judged
                .iter()
                .any(|(_, _, state)| matches!(state, Stored::Different));
            let holder = session_holding(store, session, digest)
                .filter(|held| differs || held.swing_id != swing)
                .map(|held| format!("in swing {}", held.swing_id))
                .or_else(|| {
                    placing
                        .contains(&(session.to_string(), digest.clone()))
                        .then(|| "in a swing earlier in this export".to_string())
                });
            if let Some(holder) = holder {
                self.lines.push(format!(
                    "duplicate {rel}: session {session} holds its photo already, {holder}, and \
                     that swing stays"
                ));
                self.left += files;
                return;
            }
        }
        // Filmed or not, a swing this import places holds its photo for the entries after it.
        if let Some(digest) = photo {
            placing.insert((session.to_string(), digest));
        }

        let (mut new, mut same) = (0, 0);
        for (index, dest, state) in judged {
            match state {
                Stored::Absent => {
                    new += 1;
                    self.writes.push((index, dest));
                }
                Stored::Same => same += 1,
                Stored::Different => self
                    .conflicts
                    .push(conflict(&export.items[index].rel(), &dest)),
            }
        }
        self.identical += same;
        self.lines.push(if same == 0 {
            format!("new       {rel} ({new} file(s))")
        } else {
            format!("added     {rel} ({new} new file(s), {same} identical)")
        });
    }
}

/// A drill's merge (the module doc's "A drill").
impl Plan {
    /// `drills/`: each golfer's tape drill is merged, and any other file of the tree, M44's low
    /// point's one day, goes by the plain rule until its own rule is written.
    fn drills(&mut self, export: &mut Export, indices: Vec<usize>, root: &Path) {
        let (topic, method) = (DrillTopic::Impact.as_str(), TapeMethod::StrikeTape.as_str());
        let mut tape: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for index in indices {
            let parts = &export.items[index].parts;
            if parts.len() > 4 && parts[2] == topic && parts[3] == method {
                tape.entry(parts[1].clone()).or_default().push(index);
            } else {
                let dest = below(root, &parts[1..]);
                self.file(export, index, dest);
            }
        }
        for (player_id, indices) in tape {
            self.drill(export, &player_id, indices, root);
        }
    }

    /// One golfer's tape drill: its other files by bytes, then its crops, its strikes and its state,
    /// in the order they are written.
    fn drill(&mut self, export: &mut Export, player_id: &str, indices: Vec<usize>, root: &Path) {
        let rel = export.items[indices[0]].parts[..4].join("/");
        let store = match DrillStore::in_drills(spelled(root), player_id) {
            Ok(store) => store,
            Err(why) => {
                return self
                    .problems
                    .push(format!("{rel}: not a golfer's drill ({why})"))
            }
        };
        let (mut strikes, mut state, mut crops) = (None, None, Vec::new());
        for index in indices {
            let rest = &export.items[index].parts[4..];
            match rest {
                [name] if name == STRIKES_NAME => strikes = Some(index),
                [name] if name == STATE_NAME => state = Some(index),
                [dir, ..] if dir == LAST_DIR => self.ignored += 1,
                _ if is_crop(rest) => crops.push(index),
                _ => {
                    let dest = below(root, &export.items[index].parts[1..]);
                    self.file(export, index, dest);
                }
            }
        }
        self.crops(export, crops, root);
        if let Some(index) = strikes {
            self.strikes(export, index, &store);
        }
        if let Some(index) = state {
            self.state(export, index, store);
        }
    }

    /// A drill's crops, in one line: copied where the store lacks one, and one it holds kept.
    fn crops(&mut self, export: &mut Export, indices: Vec<usize>, root: &Path) {
        let Some(&first) = indices.first() else {
            return;
        };
        let rel = export.items[first].parts[..5].join("/");
        let (mut new, mut same, mut kept) = (0, 0, 0);
        for index in indices {
            let dest = below(root, &export.items[index].parts[1..]);
            // `stored` learns a difference from the size alone, and a kept crop is never written, so
            // it is read through here: a torn entry is exit 2, as it is for any file.
            let judged = stored(export, index, &dest).and_then(|state| match state {
                Stored::Different => export.check(index).map(|()| state),
                state => Ok(state),
            });
            match judged {
                Ok(Stored::Absent) => {
                    new += 1;
                    self.writes.push((index, dest));
                }
                Ok(Stored::Same) => same += 1,
                Ok(Stored::Different) => kept += 1,
                Err(why) => self
                    .problems
                    .push(format!("{}: {why}", export.items[index].rel())),
            }
        }
        let (status, mut counts) = match (new, same + kept) {
            (0, _) if kept == 0 => ("identical", format!("{same} file(s)")),
            (0, _) => ("kept", format!("{same} identical")),
            (_, 0) => ("new", format!("{new} file(s)")),
            _ => ("added", format!("{new} new file(s), {same} identical")),
        };
        if kept > 0 {
            counts.push_str(&format!(", {kept} held with other bytes, which stay"));
        }
        self.lines.push(format!("{status:<9} {rel} ({counts})"));
        self.identical += same;
        self.left += kept;
    }

    /// A drill's `strikes.jsonl`, merged by id: the strikes the store lacks are appended, and a held
    /// id is kept as stored.
    fn strikes(&mut self, export: &mut Export, index: usize, store: &DrillStore) {
        let rel = export.items[index].rel();
        let theirs = match export.bytes(index) {
            Ok(bytes) => store.parse_strikes(&bytes),
            Err(why) => return self.problems.push(format!("{rel}: {why}")),
        };
        let ours = match store.all_strikes() {
            Ok(ours) => ours,
            Err(why) => {
                return self
                    .problems
                    .push(format!("{rel}: the store's strikes cannot be read ({why})"))
            }
        };
        let by_id: BTreeMap<&str, &TapeStrike> = ours
            .strikes
            .iter()
            .map(|strike| (strike.strike_id.as_str(), strike))
            .collect();
        let (mut new, mut same, mut other) = (Vec::new(), 0, 0);
        for strike in theirs.strikes {
            match by_id.get(strike.strike_id.as_str()) {
                None => new.push(strike),
                Some(stored) if **stored == strike => same += 1,
                Some(_) => other += 1,
            }
        }
        let held = same + other;
        let (status, mut counts) = match (new.len(), held) {
            (0, _) if other == 0 => ("identical", format!("{same} strike(s)")),
            (0, _) => ("kept", format!("{held} held")),
            (fresh, 0) => ("new", format!("{fresh} strike(s)")),
            (fresh, _) => ("added", format!("{fresh} new strike(s), {held} held")),
        };
        if other > 0 {
            counts.push_str(&format!(
                ", {other} held under its id with other values, which stay"
            ));
        }
        if theirs.skipped > 0 {
            counts.push_str(&format!(
                ", {} line(s) skipped as the store skips them",
                theirs.skipped
            ));
        }
        self.lines.push(format!("{status:<9} {rel} ({counts})"));
        if !new.is_empty() {
            self.drill_writes
                .push(DrillWrite::Strikes(store.clone(), new));
        } else if other == 0 {
            self.identical += 1;
        } else {
            self.left += 1;
        }
    }

    /// A drill's `state.json`, by `updated_at`: the export's is written where the store holds none
    /// or an older one, and the store's stays otherwise.
    fn state(&mut self, export: &mut Export, index: usize, store: DrillStore) {
        let rel = export.items[index].rel();
        let dest = store.state_path();
        let judged = match stored(export, index, &dest) {
            Ok(judged) => judged,
            Err(why) => return self.problems.push(format!("{rel}: {why}")),
        };
        // The export's is read whatever the store holds, so one that does not read is refused even
        // where it would never be written, as a shot that does not read is.
        let theirs = match export
            .bytes(index)
            .map_err(|why| why.to_string())
            .and_then(|bytes| store.parse_state(&bytes))
        {
            Ok(theirs) => theirs,
            Err(why) => {
                return self
                    .problems
                    .push(format!("{rel}: not a drill state the store reads ({why})"))
            }
        };
        let ours = match judged {
            Stored::Same => {
                self.lines.push(format!("identical {rel}"));
                self.identical += 1;
                return;
            }
            Stored::Absent => None,
            Stored::Different => match store.state() {
                Ok(ours) => ours,
                Err(StoreError::Unreadable { reason, .. }) => {
                    return self.conflicts.push(format!(
                        "{}: the store's does not read ({reason}), and nothing here writes over \
                         it",
                        conflict(&rel, &dest)
                    ))
                }
                Err(why) => {
                    return self
                        .problems
                        .push(format!("{rel}: the store's state cannot be read ({why})"))
                }
            },
        };
        match ours {
            None => self.lines.push(format!("new       {rel}")),
            Some(ours) if theirs.updated_at > ours.updated_at => self.lines.push(format!(
                "replaced  {rel}: the export's was written later ({}, the store's {})",
                theirs.updated_at, ours.updated_at
            )),
            Some(ours) => {
                self.lines.push(format!(
                    "kept      {rel}: the store's was written at or after the export's ({}, the \
                     export's {}), and it stays",
                    ours.updated_at, theirs.updated_at
                ));
                self.left += 1;
                return;
            }
        }
        self.drill_writes
            .push(DrillWrite::State(store, Box::new(theirs)));
    }
}

/// A drill's crop, `crops/<strike_id>.jpg` below the method's directory, by the contract's own
/// [`crop_file`], so the name a strike records and the name judged here cannot drift apart.
fn is_crop(rest: &[String]) -> bool {
    let [_, name] = rest else {
        return false;
    };
    Path::new(name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|id| crop_file(id) == rest.join("/"))
}

/// A write a drill's merge makes through the lab's [`DrillStore`], after every copy (the module doc).
enum DrillWrite {
    /// The export's strikes the store does not hold, in the export's order.
    Strikes(DrillStore, Vec<TapeStrike>),
    /// The export's state, where the store holds none or one written earlier.
    State(DrillStore, Box<DrillState>),
}

impl DrillWrite {
    fn dest(&self) -> PathBuf {
        match self {
            DrillWrite::Strikes(store, _) => store.strikes_path(),
            DrillWrite::State(store, _) => store.state_path(),
        }
    }

    fn run(&self) -> Result<(), StoreError> {
        match self {
            DrillWrite::Strikes(store, strikes) => strikes
                .iter()
                .try_for_each(|strike| store.append_strike(strike)),
            DrillWrite::State(store, state) => store.put_state(state),
        }
    }
}

/// What the report says of two records D6 calls one person, or `None` when they are not.
///
/// Destructured whole, with no `..`, so a field [`Golfer`] gains stops this compiling until someone
/// says whether it is identity or a difference. A file can differ by bytes with no field differing:
/// its spacing, a key the contract does not read, or one instant in two spellings, which
/// `Timestamp`'s equality calls equal.
fn golfer_difference(stored: &Golfer, theirs: &Golfer) -> Option<String> {
    let Golfer {
        player_id,
        display_name,
        handedness,
        created_at,
    } = stored;
    if *player_id != theirs.player_id || *handedness != theirs.handedness {
        return None;
    }
    let mut differing = Vec::new();
    if *display_name != theirs.display_name {
        differing.push(format!(
            "display_name ({:?}, stored {display_name:?})",
            theirs.display_name
        ));
    }
    if *created_at != theirs.created_at {
        differing.push(format!(
            "created_at ({}, stored {created_at})",
            theirs.created_at
        ));
    }
    Some(if differing.is_empty() {
        "no field it reads differs, only the file's bytes".to_string()
    } else {
        format!("the export differs in {}", differing.join(", "))
    })
}

/// A swing id in the order the bundle store numbers them: `2` before `10`, and a name after
/// every number.
type SwingOrder = (bool, u64, String);

fn swing_order(id: &str) -> SwingOrder {
    match id.parse::<u64>() {
        Ok(number) => (false, number, id.to_string()),
        Err(_) => (true, 0, id.to_string()),
    }
}

/// `root` joined with each name, so the path is spelled with the platform's separator.
fn below(root: &Path, parts: &[String]) -> PathBuf {
    parts
        .iter()
        .fold(spelled(root), |path, part| path.join(part))
}

fn conflict(rel: &str, dest: &Path) -> String {
    format!("{rel} -> {}", dest.display())
}

/// Copy one file into its place: written beside it as `.part`, given the export's time, renamed.
fn place(export: &mut Export, item: usize, dest: &Path) -> io::Result<()> {
    let parent = dest
        .parent()
        .ok_or_else(|| io::Error::other("a destination with no directory"))?;
    fs::create_dir_all(parent)?;
    let part = parent.join(format!(
        "{}.{}.part",
        crate::file_name(dest),
        std::process::id()
    ));
    let modified = export.items[item].modified;
    let placed = (|| {
        let mut file = fs::File::create(&part)?;
        io::copy(&mut export.read(item)?, &mut file)?;
        if let Some(modified) = modified {
            file.set_modified(modified)?;
        }
        drop(file);
        // `rename` replaces a file on both platforms, and this verb never replaces one.
        if dest.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "a file appeared there after it was compared",
            ));
        }
        fs::rename(&part, dest)
    })();
    if placed.is_err() {
        let _ = fs::remove_file(&part);
    }
    placed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(path: &str) -> Vec<String> {
        path.split('/').map(str::to_string).collect()
    }

    #[test]
    fn a_plain_name_means_only_itself() {
        for name in [
            "manifest.json",
            "2026-11-02",
            "shot_screen.ab12.jpg",
            "..x",
            "a b",
        ] {
            assert!(plain(name), "{name}");
        }
        for name in ["", ".", "..", "a\\b", "c:", "a/b", "tab\there"] {
            assert!(!plain(name), "{name:?}");
        }
    }

    #[test]
    fn what_no_store_reads_is_ignored() {
        for path in [
            ".DS_Store",
            "sessions/.incoming/x.part",
            "__MACOSX/sessions/._manifest.json",
            "sessions/S/1/manifest.tmp",
            "shots/abc.shot.json.4242.part",
        ] {
            assert!(ignored(&parts(path)), "{path}");
        }
        for path in ["sessions/S/1/manifest.json", "golfers/aaron.bag.json"] {
            assert!(!ignored(&parts(path)), "{path}");
        }
    }

    fn item(path: &str) -> Item {
        Item {
            parts: parts(path),
            origin: Origin::Entry(0),
            size: 0,
            modified: None,
        }
    }

    #[test]
    fn one_wrapping_folder_is_read_through_and_a_tree_is_not() {
        let mut items = vec![
            item("processed/sessions/S/1/manifest.json"),
            item("processed/golfers/aaron.golfer.json"),
            item("__MACOSX/processed/._golfers"),
        ];
        unwrap_folder(&mut items);
        let paths: Vec<String> = items.iter().map(Item::rel).collect();
        assert_eq!(
            paths,
            [
                "sessions/S/1/manifest.json",
                "golfers/aaron.golfer.json",
                "__MACOSX/processed/._golfers"
            ]
        );

        // A layout with one tree is not a wrapper, and two folders are not one.
        for layout in [
            vec!["sessions/S/1/manifest.json"],
            vec!["a/sessions/x", "b/shots/y"],
            vec!["processed/sessions/x", "README"],
        ] {
            let mut items: Vec<Item> = layout.iter().map(|path| item(path)).collect();
            unwrap_folder(&mut items);
            let after: Vec<String> = items.iter().map(Item::rel).collect();
            assert_eq!(after, layout, "{layout:?}");
        }
    }

    #[test]
    fn swings_sort_as_the_store_numbers_them() {
        let mut ids: Vec<SwingOrder> = ["10", "b", "2", "a", "1"]
            .into_iter()
            .map(swing_order)
            .collect();
        ids.sort();
        let sorted: Vec<&str> = ids.iter().map(|id| id.2.as_str()).collect();
        assert_eq!(sorted, ["1", "2", "10", "a", "b"]);
    }

    /// Both UTC fields, read off the bytes a tool writes; the extended-timestamp field through the
    /// archive is `tests/import_export.rs`' zip case.
    #[test]
    fn an_ntfs_time_reads_as_utc_and_the_first_time_field_wins() {
        // 2026-11-02T10:00:00Z.
        let seconds = 1_793_613_600_u64;
        let mut ntfs = vec![0, 0, 0, 0, 1, 0, 24, 0];
        ntfs.extend((FILETIME_TO_UNIX + seconds * 10_000_000 + 5).to_le_bytes());
        ntfs.extend([0u8; 16]);
        let ntfs = zip::extra_fields::Ntfs::try_from_reader(&mut ntfs.as_slice(), 32).unwrap();
        let mut stamp = vec![1u8];
        stamp.extend(1_000_u32.to_le_bytes());
        let stamp = zip::extra_fields::ExtendedTimestamp::try_from_reader(&mut stamp.as_slice(), 5)
            .unwrap();
        let fields = [ExtraField::Ntfs(ntfs), ExtraField::ExtendedTimestamp(stamp)];
        assert_eq!(
            zip_modified(fields.iter()),
            Some(UNIX_EPOCH + Duration::from_nanos(seconds * 1_000_000_000 + 500)),
            "100 ns ticks after 1601, as UTC"
        );
        assert_eq!(
            zip_modified(fields[1..].iter()),
            Some(UNIX_EPOCH + Duration::from_secs(1_000))
        );
        assert_eq!(zip_modified([].iter()), None, "the DOS time is never read");
    }
}
