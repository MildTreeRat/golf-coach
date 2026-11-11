//! The bay session's arithmetic: an export in, the verdict `docs/plans/m38-bay-thresholds.md` asks for
//! out. [M38 P14]
//!
//! `golf-bay-report` is a thin shell over this (C21), and this holds no golf of its own: what a cell is,
//! what a flag is and what each bar measures are that file's definitions, spelled out where each is
//! counted below, and every limit is read from that file at run time, so no bar has a second copy.
//! Outside `api` because the codegen binds what is `pub` there (rule 7).
//!
//! # What a photo is
//!
//! **A digest with a read line whose status is not `duplicate`.** The log and the store are allowed to
//! disagree, and each way they can is decided here (P10 finding 8):
//!
//! - **The last such line speaks for its digest.** A photo recorded afresh after a delete, or a failed
//!   read retaken with the same bytes, appends a second line. The last is the read that produced what is
//!   stored, and one photo is one photo however many lines it took. A camera photo never repeats its
//!   bytes, so this only reaches a library import.
//! - **A photo with a `read` or `flagged` line and no stored shot was deleted at the phone**, since a
//!   delete writes no line (P13 finding 10). It still counts wherever the file asks only the log, which
//!   is the session's size, T4 and T5, because the file says a later review cannot change a flag and a
//!   delete should not either. It holds no cells, since nothing says what its truth was, and the report
//!   names it. The other reading, leaving it out, would let a person improve T4 by tidying up.
//! - **A stored shot with no read line** is a crash between `record_photo` and `log_read`. Nothing says
//!   how it arrived, and only camera photos count, so it is in no count and the report names it.
//! - **A refusal writes no line at all** (P6 finding 8), so an `Err` at the bay is in no count here.
//!
//! # Whose photos
//!
//! The session is camera photos, so `--source` defaults to that and is the only choice that can earn
//! a verdict. It exists for two later phases and is not in P14's Do line: P30 reads the sign letters and
//! latency "over everything" and P34 "splits bars by source", and neither is allowed to touch this code.
//!
//! # Which sitting
//!
//! **`--since` is not in P14's Do line either, and without it P32's verdict would be wrong.** The file's
//! session is "one sitting at the bay", the export is cumulative (C16), and a reinstall keeps the data
//! (C20). So P30's rehearsal, in-app camera photos of a monitor and so `camera` in the log, is still in
//! P32's zip, where it would count towards every bar. So a read before `--since` is in no count, and
//! the report always says when its reads ran from and to, so a rehearsal left in shows.
//!
//! # What D8's evidence is
//!
//! P32 has to answer "did a box's confidence ever separate a wrong cell from a right one?", and only
//! the parser knows which box landed in which tile, with nothing on `ParsedShot` to say so. So it is
//! asked, as M33's `averaged_boxes` asked it: **a box is behind a cell if taking the box away moves that
//! cell's text or value.** A copy of `build_cells` here would be a second parser nothing could certify.
//! It can overcount, since a label taken away can shift a neighbouring tile, and that errs towards a low
//! confidence reaching a cell rather than hiding it. Only boxes below 1.00 are asked about, because
//! nothing can lower a cell's lowest but one of those, and on M33's arms that is 3 boxes of 251.
//!
//! Whether a read depended on a confidence is asked the same way, by M33's `all_at`: the same boxes read
//! again with every confidence at 1.00 and at 0.00, at [`MIN_CONFIDENCE`].
//!
//! # What `truth` holds that `bars` cannot [M38 P15]
//!
//! `bars` trusts the review, because nothing else can stand behind thirty new photos. So it cannot be a
//! control: an unreviewed `sidespin` not read is a blank to it, and a typed value is the truth. `truth`
//! holds the reader to a table a person typed, M33's, and **a review cannot move its numbers**:
//!
//! - **The cell is what the reader gave**, `provenance.corrections`' value where a field has one, so a
//!   wrong number corrected is still wrong and a blank typed in is still withheld.
//! - **The flag is the log's**, by the bars file's rule, because saving a review clears the stored one.
//!   A shot with no line falls back on the stored flag and the report says so. That is P18's road, whose
//!   test calls `record_photo` and writes no line, and on it nobody has reviewed anything.
//! - **A wrong cell on a flagged photo is `caught`**, M33's word and not one of the bars file's, which
//!   needs none: T1 is what tells the two apart there, and here each is a row of the table.
//! - **The conversion to `ShotData`'s conventions is typed here and read from nothing the reader uses.**
//!   Taking the sign rule from `profiles.json` would have an oracle agree with whatever it is checking.
//! - **It judges against no limit.** Six photos are no session, and P18, P28 and P30 each put these
//!   numbers beside M33's for a person to read. So T1 to T4 are named and no bar is passed or missed.
//!
//! # How a capture is found [M38 P15]
//!
//! **By its digest, and else by the stem of the name it arrived under**, since Photos may re-encode an
//! AirDropped file (P30) and then the digest is lost and the name is what remains. A match by name is
//! labelled a re-encoded copy, because that is the only way bytes differ under one name here. Two entries
//! under one stem are refused rather than chosen between, and hold no cells, since picking one could
//! report a read of the wrong photo. **A failed read keeps no name** (C7), so one that was re-encoded is
//! not found, and the report says how many unmatched failures the log holds.
//!
//! # What `compare` calls equal [M38 P15]
//!
//! **Exactly equal, bit for bit**, which is why it does not call `golf_core::compare`: that is CONFORMANCE
//! §3's comparator, whose float tolerance is for one measurement added up differently by two
//! implementations. Here one reader reads the same bytes twice, so anything inside that tolerance is
//! a difference P13's round trip does not allow. It keeps that comparator's spelling of a path.
//!
//! # What `events` reads [M38 P30c]
//!
//! `.app/events.jsonl` beside `.app/reads.jsonl`, through the same reader and the same `--since` as
//! `bars`, and it judges nothing. The submodule [`events`]' doc says how each line is read.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

use contracts::capability::{capability_for, FieldSource};
use contracts::shot::ShotData;
use contracts::Timestamp;
use screen::parser::{is_lit, parse_screen, ParsedShot, LIT_BLUE_RATIO};
use screen::profile::{load_profile, DeviceProfile};
use screen::validate::validate_parse;
use screen::TextBox;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use storage::bundle_store::SwingBundleStore;
use storage::corpus::read_corpus;
use storage::manifest::{Role, SwingManifest};
use storage::shot_store::ShotStore;
use zip::ZipArchive;

use crate::api::bay::{TruthCell, TruthCellKind, TruthField, TruthPhoto, TruthReport, TruthRoute};
use crate::api::record::{AppEvent, ReadLog, ReadSource, RecordStatus};
use crate::record::{parse_boxes, spelled, EVENTS_NAME, READS_NAME};
use crate::{engine_versions, parse_now, Dirs, DEVICE, MIN_CONFIDENCE};

mod events;
pub use events::{
    events_report, render_events, Camera, EventsReport, Failure, KindCount, Logged, Retake,
    Reviews, Shots, Spread,
};

/// The bars, below the repo's root, where `golf-bay-report` looks unless told otherwise.
pub const BARS_FILE: &str = "docs/plans/m38-bay-thresholds.md";

/// What each bar measures decides its unit, so the table cannot put a bar in the wrong one and be
/// judged by a number that means something else. These are the bars' shapes and hold no limit.
const BAR_UNITS: [(&str, &str); 5] = [
    ("T1", ""),
    ("T2", "%"),
    ("T3", "%"),
    ("T4", "%"),
    ("T5", "ms"),
];

// --- the bars file -------------------------------------------------------------------------------

/// Which side of its number a bar holds on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    AtMost,
    AtLeast,
}

/// One row of the file's table.
#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    pub id: String,
    /// The table's own words, so the report cannot describe a bar differently from the file.
    pub what: String,
    /// The table's own words again, for the report.
    pub limit: String,
    pub side: Side,
    pub number: f64,
}

impl Bar {
    /// `left` against `right`, each already scaled so that a limit exactly met compares equal: `92 of
    /// 100` at 92% is `9200 >= 9200`, with no division to round across the limit.
    fn holds(&self, left: f64, right: f64) -> bool {
        match self.side {
            Side::AtMost => left <= right,
            Side::AtLeast => left >= right,
        }
    }
}

/// Everything the file decides. **There is no default and no fallback**: a file that cannot be parsed
/// is refused, because a verdict against a guessed limit is the second copy this exists to prevent.
#[derive(Debug, Clone, PartialEq)]
pub struct Bars {
    /// The session's definition, `at least N photos`, which is prose and not the table. Read all the
    /// same, so a fixture of ten photos can only be made valid by a copy of the file saying so.
    pub photos: u64,
    /// `at least N clubs`, the same sentence.
    pub clubs: u64,
    pub bars: Vec<Bar>,
}

impl Bars {
    /// The file at `path`, read and parsed.
    pub fn read(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path)
            .map_err(|failed| format!("could not read the bars at {}: {failed}", path.display()))?;
        Self::parse(&text).map_err(|refused| format!("{}: {refused}", path.display()))
    }

    /// Parse the file's text: `## The bars`' table and `**The session**`'s sentence.
    pub fn parse(text: &str) -> Result<Self, String> {
        let (photos, clubs) = session_rule(text)?;
        let mut found: BTreeMap<&str, Bar> = BTreeMap::new();
        let section = text
            .lines()
            .skip_while(|line| line.trim() != "## The bars")
            .skip(1)
            .take_while(|line| !line.starts_with("## "));
        for line in section {
            let row = line.trim();
            if !row.starts_with('|') {
                continue;
            }
            let cells: Vec<&str> = row.trim_matches('|').split('|').map(str::trim).collect();
            let id = cells[0];
            if !id.starts_with('T') {
                continue; // the header, whose first cell is `bar`, and the rule under it
            }
            let &[_, what, limit] = cells.as_slice() else {
                return Err(format!("the row for {id} does not have three cells: {row}"));
            };
            let Some(&(id, unit)) = BAR_UNITS.iter().find(|(known, _)| *known == id) else {
                return Err(format!(
                    "the table has a bar named {id}, and this report measures no such thing, so it \
                     would go unjudged"
                ));
            };
            let (side, number) = parse_limit(id, limit, unit)?;
            let bar = Bar {
                id: id.to_string(),
                what: what.to_string(),
                limit: limit.to_string(),
                side,
                number,
            };
            if found.insert(id, bar).is_some() {
                return Err(format!("the table has {id} twice"));
            }
        }
        let bars = BAR_UNITS
            .iter()
            .map(|(id, _)| {
                found
                    .remove(id)
                    .ok_or_else(|| format!("the table under `## The bars` has no row for {id}"))
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Bars {
            photos,
            clubs,
            bars,
        })
    }

    fn bar(&self, id: &str) -> &Bar {
        self.bars
            .iter()
            .find(|bar| bar.id == id)
            .expect("parse holds the table to one row per bar in BAR_UNITS")
    }
}

/// `at most 0`, `at least 92%`, `at most 5000 ms`. The unit is required, and no other is taken.
fn parse_limit(id: &str, limit: &str, unit: &str) -> Result<(Side, f64), String> {
    let refused = || {
        let wants = if unit.is_empty() {
            "a bare number".to_string()
        } else {
            format!("a number in {unit}")
        };
        format!("{id}'s limit is '{limit}', where `at most` or `at least` and {wants} is asked")
    };
    let (side, rest) = if let Some(rest) = limit.strip_prefix("at most ") {
        (Side::AtMost, rest)
    } else if let Some(rest) = limit.strip_prefix("at least ") {
        (Side::AtLeast, rest)
    } else {
        return Err(refused());
    };
    let number = rest
        .strip_suffix(unit)
        .and_then(|number| number.trim().parse::<f64>().ok())
        .filter(|number| number.is_finite())
        .ok_or_else(refused)?;
    Ok((side, number))
}

/// `at least 30 photos` and `at least 2 clubs`, from the definition of the session.
fn session_rule(text: &str) -> Result<(u64, u64), String> {
    let refused =
        |what: &str| format!("the definition of **The session** has no `at least N {what}`");
    let start = text
        .find("**The session**")
        .ok_or_else(|| refused("photos"))?;
    let bullet = &text[start..];
    let bullet = &bullet[..bullet.find("\n- ").unwrap_or(bullet.len())];
    let words: Vec<&str> = bullet.split_whitespace().collect();
    let at_least = |noun: &str| {
        words.windows(4).find_map(|four| match *four {
            ["at", "least", number, word] if word.trim_end_matches(',') == noun => {
                number.parse::<u64>().ok()
            }
            _ => None,
        })
    };
    Ok((
        at_least("photos").ok_or_else(|| refused("photos"))?,
        at_least("clubs").ok_or_else(|| refused("clubs"))?,
    ))
}

// --- the export ----------------------------------------------------------------------------------

/// An export as a directory the stores can read, so a zip and a directory are one path from here on.
pub struct Export {
    root: PathBuf,
    _scratch: Option<Scratch>,
}

impl Export {
    /// A directory is read where it stands, and anything else must open as a zip.
    ///
    /// **Only the JSON leaves a zip**, into a scratch directory removed on drop: the stores, the log and
    /// the boxes are what is read, and a session's photos are a hundred megabytes nothing here opens.
    pub fn open(path: &Path) -> Result<Self, String> {
        let shown = path.display();
        let meta = fs::metadata(path).map_err(|failed| format!("{shown}: {failed}"))?;
        let (root, scratch) = if meta.is_dir() {
            (path.to_path_buf(), None)
        } else {
            let scratch = Scratch::new().map_err(|failed| format!("a scratch dir: {failed}"))?;
            unpack_json(path, &scratch.0).map_err(|refused| format!("{shown}: {refused}"))?;
            (scratch.0.clone(), Some(scratch))
        };
        let dirs = Dirs::of(&root);
        if ![&dirs.sessions, &dirs.shots, &dirs.golfers, &dirs.app]
            .iter()
            .any(|dir| dir.is_dir())
        {
            return Err(format!(
                "{shown} is not an export: it holds no sessions/, shots/, golfers/ or .app/"
            ));
        }
        Ok(Export {
            root,
            _scratch: scratch,
        })
    }

    /// Where the export's layout is, a data root as `Dirs` reads one.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// A directory removed on drop. By hand because `tempfile` is a dev-dependency here, and as a normal
/// one it would ship in the bridge's lock for the sake of a tool that never runs on a phone.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> io::Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let at = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("golf-bay-report-{}-{at}", process::id()));
        fs::create_dir(&path)?;
        Ok(Scratch(path))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn unpack_json(archive: &Path, into: &Path) -> Result<(), String> {
    let file = fs::File::open(archive).map_err(|failed| failed.to_string())?;
    let mut zip = ZipArchive::new(file)
        .map_err(|failed| format!("neither a directory nor a zip archive this reads ({failed})"))?;
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|failed| format!("entry {index} of the archive: {failed}"))?;
        let name = entry.name().to_string();
        if entry.is_dir() || !(name.ends_with(".json") || name.ends_with(".jsonl")) {
            continue;
        }
        let relative = entry
            .enclosed_name()
            .ok_or_else(|| format!("{name} is not a plain relative path"))?;
        let to = into.join(relative);
        let written = to
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| fs::File::create(&to))
            .and_then(|mut out| io::copy(&mut entry, &mut out));
        written.map_err(|failed| format!("could not unpack {name}: {failed}"))?;
    }
    Ok(())
}

// --- the report ----------------------------------------------------------------------------------

/// Whose photos the numbers are over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceChoice {
    Only(ReadSource),
    All,
}

impl SourceChoice {
    /// `camera`, `library`, `scanner` or `all`.
    pub fn parse(text: &str) -> Result<Self, String> {
        match text {
            "camera" => Ok(Self::Only(ReadSource::Camera)),
            "library" => Ok(Self::Only(ReadSource::Library)),
            "scanner" => Ok(Self::Only(ReadSource::Scanner)),
            "all" => Ok(Self::All),
            other => Err(format!(
                "--source is camera, library, scanner or all, not '{other}'"
            )),
        }
    }

    fn admits(self, source: ReadSource) -> bool {
        match self {
            Self::Only(only) => only == source,
            Self::All => true,
        }
    }

    fn name(self) -> String {
        match self {
            Self::Only(only) => spelled(only),
            Self::All => "all".to_string(),
        }
    }
}

/// One cell, by the file's definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CellKind {
    Read,
    Wrong,
    Withheld,
    Blank,
}

/// **Compared as IEEE doubles with no tolerance**, so `-0.0` is `0.0` and nothing else moves. A value
/// a person cleared is `Wrong`: the reader saw a number where the screen left it blank.
pub fn classify(as_read: Option<f64>, truth: Option<f64>) -> CellKind {
    match (as_read, truth) {
        (None, None) => CellKind::Blank,
        (None, Some(_)) => CellKind::Withheld,
        (Some(read), Some(truth)) if read == truth => CellKind::Read,
        (Some(_), _) => CellKind::Wrong,
    }
}

/// The nearest-rank percentile's rank, 1-based: `ceil(n * percent / 100)`. In integers, since
/// `0.95 * 20.0` landing a hair over 19 in floats would pick the wrong photo.
pub fn nearest_rank(photos: usize, percent: usize) -> usize {
    (photos * percent).div_ceil(100).max(1)
}

/// How many cells of each kind.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Cells {
    pub read: u64,
    pub wrong: u64,
    pub withheld: u64,
    pub blank: u64,
}

impl Cells {
    fn add(&mut self, kind: CellKind) {
        match kind {
            CellKind::Read => self.read += 1,
            CellKind::Wrong => self.wrong += 1,
            CellKind::Withheld => self.withheld += 1,
            CellKind::Blank => self.blank += 1,
        }
    }

    /// The bars' denominator. A blank is in no denominator.
    pub fn judged(&self) -> u64 {
        self.read + self.wrong + self.withheld
    }
}

/// The session's size against the file's definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Session {
    pub valid: bool,
    pub camera_photos: u64,
    pub needs_photos: u64,
    /// The clubs on camera photos. A failed read has no entry and so no club.
    pub clubs: Vec<String>,
    pub needs_clubs: u64,
}

/// One bar's number and whether it holds.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BarResult {
    pub id: String,
    pub what: String,
    pub limit: String,
    /// The count, the percentage or the milliseconds. `None` is nothing to measure, which never holds.
    pub number: Option<f64>,
    /// A share's numerator and denominator, or T5's rank and photos.
    pub of: Option<(u64, u64)>,
    /// T3's lowest field. A tie goes to the first in the device's tile order.
    pub field: Option<String>,
    /// `Some` wherever there is a number, on an `INVALID` session as well, since a dry run (P30) reads
    /// them. The verdict is what says they judged nothing.
    pub met: Option<bool>,
}

/// The file's three outcomes, and the refusal to judge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Verdict {
    #[serde(rename = "PASS")]
    Pass,
    #[serde(rename = "MISS")]
    Miss,
    #[serde(rename = "MISS (latency)")]
    MissLatency,
    #[serde(rename = "INVALID")]
    Invalid,
}

/// How the photos fell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Photos {
    /// Those over the chosen source: `read` + `flagged` + `failed`.
    pub counted: u64,
    pub read: u64,
    pub flagged: u64,
    pub failed: u64,
    /// Of `read` and `flagged`, those with no stored shot (the module doc).
    pub deleted: u64,
    /// Lines saying `duplicate`, each no photo for any count.
    pub duplicate_lines: u64,
    /// Every photo in the export by where it came from, whichever source was chosen.
    pub by_source: BTreeMap<String, u64>,
    /// When the first and last of `counted` were logged, as written, so a sitting can be seen to be one.
    pub first_read: Option<String>,
    pub last_read: Option<String>,
}

/// One field's cells.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FieldRow {
    pub field: String,
    pub cells: Cells,
}

/// A wrong or withheld cell, by photo and field.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MissedCell {
    pub photo: String,
    pub field: String,
    pub kind: CellKind,
    /// In `ShotData`'s units and with its sign: the apex in yards, and a lit tile signed.
    pub as_read: Option<f64>,
    pub truth: Option<f64>,
    pub flagged: bool,
}

/// One club's photos and cells. `club` is `None` for a photo with no entry, a failed read's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClubRow {
    pub club: Option<String>,
    pub photos: u64,
    pub flagged: u64,
    pub cells: Cells,
}

/// `ms_total` is the bar's. The two others say where it went, for a latency-only miss at P32.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Latency {
    pub photos: u64,
    pub total_p95: Option<i64>,
    pub total_median: Option<i64>,
    pub total_max: Option<i64>,
    pub vision_p95: Option<i64>,
    pub rust_p95: Option<i64>,
}

/// How many boxes in one range of confidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Bucket {
    pub range: String,
    pub boxes: u64,
}

/// A wrong cell and the lowest confidence behind it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WrongCell {
    pub photo: String,
    pub field: String,
    /// `None` when its photo has no boxes in the export.
    pub lowest: Option<f64>,
}

/// D8's evidence (the module doc).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Confidences {
    pub boxes: u64,
    pub photos: u64,
    pub lowest: Option<f64>,
    /// Highest first, and only the ranges holding a box.
    pub histogram: Vec<Bucket>,
    pub below_one: u64,
    /// Of `below_one`, those a cell's text or value moves without. The rest are strays.
    pub below_one_behind_a_cell: u64,
    /// Photos where a value differs between their own confidences, all 1.00 and all 0.00. A read
    /// depending on a confidence, and the parser reads none, so a name here is a surprise.
    pub values_moved: Vec<String>,
    /// Photos flagged on their own confidences and not with every box at 1.00.
    pub flagged_by_a_confidence: Vec<String>,
    pub read_cells: u64,
    pub read_cells_below_one: u64,
    pub read_cells_lowest: Option<f64>,
    pub wrong_cells: Vec<WrongCell>,
    /// Of `wrong_cells`, those whose lowest is below the lowest of every read cell, so a threshold
    /// could have caught them at no cost.
    pub wrong_below_every_read: u64,
    /// P32's question: `no wrong cell`, `yes`, `partly` or `no`, or `unknown` if no wrong cell's photo
    /// has its boxes in the export.
    pub separates: String,
}

/// One box that is a sign letter exactly, as the parser's `lit_sign` spells that.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LetterRow {
    pub photo: String,
    pub letter: String,
    /// The tile it would sign. Every such box in the image is listed, the tile's or not, because only
    /// the parser knows which cell it landed in, so `x` and `y` are here to tell the two apart.
    pub field: String,
    pub x: f64,
    pub y: f64,
    pub rgb: Option<[f64; 3]>,
    /// Blue over the lesser of red and green. `None` with no colour, or none of red or green.
    pub ratio: Option<f64>,
    /// `LIT_BLUE_RATIO - ratio`, positive on the lit side, as M33's table has it.
    pub margin: Option<f64>,
    /// `screen::parser::is_lit`, which the brightness floor can part from the margin's sign.
    pub lit: Option<bool>,
}

/// The letters against `LIT_BLUE_RATIO`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Letters {
    pub blue_ratio: f64,
    pub rows: Vec<LetterRow>,
    /// The lit letter nearest the line.
    pub smallest_lit: Option<f64>,
    /// The unlit letter nearest the line.
    pub smallest_unlit: Option<f64>,
}

/// Everything `bars` prints, or `--json` writes.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BarsReport {
    pub source: String,
    /// `--since`, the reads before it being in no count.
    pub since: Option<String>,
    pub session: Session,
    pub bars: Vec<BarResult>,
    pub verdict: Verdict,
    pub photos: Photos,
    pub cells: Cells,
    /// In the device's tile order.
    pub fields: Vec<FieldRow>,
    pub missed: Vec<MissedCell>,
    pub clubs: Vec<ClubRow>,
    pub latency: Latency,
    pub confidences: Confidences,
    pub letters: Letters,
    /// What the export holds that the counts above leave out, each a sentence.
    pub notes: Vec<String>,
}

/// The twelve, as the file defines them: what `devices.json` declares from the screen.
fn screen_fields() -> Result<Vec<String>, String> {
    let declared = capability_for(DEVICE).map_err(|refused| refused.to_string())?;
    Ok(declared
        .fields
        .iter()
        .filter(|field| field.from == FieldSource::Screen)
        .map(|field| field.field.clone())
        .collect())
}

/// What `.app/reads.jsonl` says, in the order it was written.
///
/// **A line that is not a read is noted and not fatal.** A crash can tear the last one, and a report
/// refused over it would cost the whole session's verdict. The note says what no count holds.
fn read_lines(dirs: &Dirs, notes: &mut Vec<String>) -> Result<Vec<ReadLog>, String> {
    let lines = read_log(dirs, notes)?;
    if lines.is_none() {
        notes.push("There is no .app/reads.jsonl, so no photo can be counted.".to_string());
    }
    Ok(lines.unwrap_or_default())
}

/// One of the two logs under `.app/`, read the same way whichever it is, so `events` has no second
/// tolerant reader to drift from `bars`' (P30c). [M38 P30c]
trait LogLine: DeserializeOwned + Clone {
    /// The file's name under [`Dirs::app`].
    const FILE: &'static str;
    /// What one line is, after "is not", in a note.
    const WHAT: &'static str;
    /// What one line is, before "line(s)", in a note.
    const NOUN: &'static str;
    /// When it was written, RFC 3339 with an offset.
    fn at(&self) -> &str;
}

impl LogLine for ReadLog {
    const FILE: &'static str = READS_NAME;
    const WHAT: &'static str = "a read";
    const NOUN: &'static str = "read";
    fn at(&self) -> &str {
        &self.at
    }
}

impl LogLine for AppEvent {
    const FILE: &'static str = EVENTS_NAME;
    const WHAT: &'static str = "an event";
    const NOUN: &'static str = "event";
    fn at(&self) -> &str {
        &self.at
    }
}

/// The log, or `None` if there is none, which only the caller can say what it means: `bars` counts
/// nothing without one, and `truth` still has the stores.
fn read_log(dirs: &Dirs, notes: &mut Vec<String>) -> Result<Option<Vec<ReadLog>>, String> {
    read_jsonl(dirs, notes)
}

/// Either log, line by line in the order written. A line that is not one is a note naming its number.
fn read_jsonl<T: LogLine>(dirs: &Dirs, notes: &mut Vec<String>) -> Result<Option<Vec<T>>, String> {
    let path = dirs.app.join(T::FILE);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(failed) if failed.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(failed) => return Err(format!("could not read {}: {failed}", path.display())),
    };
    let text = String::from_utf8_lossy(&bytes);
    let mut lines = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<T>(line) {
            Ok(read) => lines.push(read),
            Err(failed) => notes.push(format!(
                "Line {} of {} is not {}, and is in no count: {failed}.",
                index + 1,
                T::FILE,
                T::WHAT
            )),
        }
    }
    Ok(Some(lines))
}

/// The lines at or after `since`, by instant, so an offset cannot move a read across it. A line whose
/// `at` is no time is among those left out, and `log_read` and `log_event` refuse to write one.
fn lines_since<T: LogLine>(
    lines: &[T],
    since: Option<Timestamp>,
    notes: &mut Vec<String>,
) -> Vec<T> {
    let Some(since) = since else {
        return lines.to_vec();
    };
    let kept: Vec<T> = lines
        .iter()
        .filter(|line| parse_now(line.at()).is_ok_and(|at| at >= since))
        .cloned()
        .collect();
    if kept.len() < lines.len() {
        notes.push(format!(
            "{} {} line(s) before --since are in no count.",
            lines.len() - kept.len(),
            T::NOUN
        ));
    }
    kept
}

/// Each digest's last line that is not `duplicate`, in the order those lines were written.
fn photo_lines(lines: &[ReadLog]) -> Vec<&ReadLog> {
    let mut last = BTreeMap::new();
    for (at, line) in lines.iter().enumerate() {
        if line.status != RecordStatus::Duplicate {
            last.insert(line.photo_sha256.as_str(), at);
        }
    }
    lines
        .iter()
        .enumerate()
        .filter(|(at, line)| last.get(line.photo_sha256.as_str()) == Some(at))
        .map(|(_, line)| line)
        .collect()
}

/// Every photo entry by its photo's hash, the first holding it if a second golfer also does (P6
/// finding 7), which only a person with two golfers and one photo can reach.
fn entries_by_digest(dirs: &Dirs) -> BTreeMap<String, SwingManifest> {
    let store = SwingBundleStore::new(&dirs.sessions);
    let mut entries = BTreeMap::new();
    for session in store.list_session_ids() {
        for manifest in store.get_session(&session) {
            if let Some(photo) = manifest.roles.get(&Role::ShotScreen) {
                entries
                    .entry(photo.content_sha256.clone())
                    .or_insert(manifest);
            }
        }
    }
    entries
}

/// How a photo is named in the report: its entry, and the name it arrived under, since that is what a
/// person finds it by. A failed read has neither.
fn photo_label(entry: Option<&SwingManifest>, digest: &str) -> String {
    match entry {
        Some(entry) => {
            let name = entry
                .roles
                .get(&Role::ShotScreen)
                .map_or("", |photo| photo.original_filename.as_str());
            format!("{}/{} {name}", entry.session_id, entry.swing_id)
        }
        None => format!("photo {}", digest.get(..12).unwrap_or(digest)),
    }
}

/// One cell of one photo: its field, as read, and the truth.
type PhotoCell = (String, Option<f64>, Option<f64>);

/// One stored shot's cells, in `fields`' order. The truth is the stored value
/// after a person's review, and as read is the `corrections` entry where a field has one (the file's
/// definitions). **A `corrections` value is a stored number** (P7 finding 10): yards for the apex, signed
/// for a lit tile, so it needs no conversion. `""` means the reader gave nothing.
fn cells_of(shot: &ShotData, fields: &[String]) -> Result<Vec<PhotoCell>, String> {
    let provenance = shot.provenance.as_ref().ok_or_else(|| {
        format!(
            "shot {} holds no provenance, so no screen read it",
            shot.shot_id
        )
    })?;
    let written = serde_json::to_value(shot).map_err(|failed| failed.to_string())?;
    fields
        .iter()
        .map(|field| {
            // A number crosses `serde_json::Value` with its bits, as `card_of` relies on.
            let truth = written.get(field).and_then(|held| held.as_f64());
            let as_read = match provenance.corrections.get(field) {
                None => truth,
                Some(entry) if entry.is_empty() => None,
                Some(entry) => Some(
                    entry
                        .parse::<f64>()
                        .ok()
                        .filter(|number| number.is_finite())
                        .ok_or_else(|| {
                            format!(
                                "shot {}'s correction of {field} is '{entry}', which is no number",
                                shot.shot_id
                            )
                        })?,
                ),
            };
            Ok((field.clone(), as_read, truth))
        })
        .collect()
}

/// One photo's boxes, or why not, which is a note and not a refusal: the bars do not read them.
fn boxes_of(dirs: &Dirs, digest: &str) -> Result<Vec<TextBox>, String> {
    let path = dirs.app.join("boxes").join(format!("{digest}.json"));
    let text = fs::read_to_string(&path).map_err(|failed| match failed.kind() {
        io::ErrorKind::NotFound => "no boxes are held".to_string(),
        _ => format!("its boxes could not be read: {failed}"),
    })?;
    parse_boxes(&text)
}

/// `boxes` with every confidence at one value, M33's `all_at`.
fn all_at(boxes: &[TextBox], confidence: f64) -> Vec<TextBox> {
    let mut boxes = boxes.to_vec();
    for text_box in &mut boxes {
        text_box.confidence = confidence;
    }
    boxes
}

/// Whether taking a box away moved this tile's text or value.
fn tile_moved(without: &ParsedShot, read: &ParsedShot, label: &str, target: &str) -> bool {
    without.raw_fields.get(label) != read.raw_fields.get(label)
        || without.values.get(target) != read.values.get(target)
}

/// What one photo's boxes add to [`Confidences`], and each field's lowest confidence behind it.
fn weigh_boxes(
    boxes: &[TextBox],
    profile: &DeviceProfile,
    label: &str,
    tally: &mut Confidences,
) -> BTreeMap<String, f64> {
    let validated =
        |boxes: &[TextBox]| validate_parse(parse_screen(boxes, profile), MIN_CONFIDENCE);
    let own = validated(boxes);
    let ceiling = validated(&all_at(boxes, 1.0));
    let floor = validated(&all_at(boxes, 0.0));
    if own.values != ceiling.values
        || own.values != floor.values
        || own.raw_fields != ceiling.raw_fields
        || own.raw_fields != floor.raw_fields
    {
        tally.values_moved.push(label.to_string());
    }
    if own.needs_review && !ceiling.needs_review {
        tally.flagged_by_a_confidence.push(label.to_string());
    }

    tally.photos += 1;
    let read = parse_screen(boxes, profile);
    let mut lowest: BTreeMap<String, f64> = BTreeMap::new();
    for (at, text_box) in boxes.iter().enumerate() {
        tally.boxes += 1;
        if tally.lowest.is_none_or(|known| text_box.confidence < known) {
            tally.lowest = Some(text_box.confidence);
        }
        count_box(&mut tally.histogram, text_box.confidence);
        if text_box.confidence >= 1.0 {
            continue;
        }
        tally.below_one += 1;
        let mut without = boxes.to_vec();
        without.remove(at);
        let without = parse_screen(&without, profile);
        let mut behind_a_cell = false;
        for tile in profile.stored_fields() {
            let target = tile.target.as_deref().expect("a stored field has a target");
            if tile_moved(&without, &read, &tile.label, target) {
                behind_a_cell = true;
                let held = lowest.entry(target.to_string()).or_insert(1.0);
                if text_box.confidence < *held {
                    *held = text_box.confidence;
                }
            }
        }
        tally.below_one_behind_a_cell += u64::from(behind_a_cell);
    }
    lowest
}

/// Bucket one box. `1.00` is its own range, because on M33's arms nearly every box is exactly that,
/// and a `0.90-1.00` row would hide the one distinction Vision's numbers make.
fn count_box(histogram: &mut Vec<Bucket>, confidence: f64) {
    let bucket = ((confidence * 10.0).floor() as i64).clamp(0, 10);
    let range = if bucket == 10 {
        "1.00".to_string()
    } else {
        format!("0.{bucket}0-0.{bucket}9")
    };
    let at = match histogram.binary_search_by(|held| range.cmp(&held.range)) {
        Ok(at) => at,
        Err(at) => {
            histogram.insert(at, Bucket { range, boxes: 0 });
            at
        }
    };
    histogram[at].boxes += 1;
}

/// Every box in the photo that is one sign letter exactly, by the rule `parser::lit_sign` reads with.
fn letter_rows(boxes: &[TextBox], profile: &DeviceProfile, label: &str, rows: &mut Vec<LetterRow>) {
    for text_box in boxes {
        let letter = pyfmt::upper(pyfmt::strip(&text_box.text));
        let tile = profile
            .fields
            .iter()
            .find(|tile| tile.lit_sign.iter().any(|lit| lit.token == letter));
        let Some(field) = tile.and_then(|tile| tile.target.clone()) else {
            continue;
        };
        let ratio = text_box.rgb.and_then(|[red, green, blue]| {
            let floor = red.min(green);
            (floor != 0.0).then(|| blue / floor)
        });
        rows.push(LetterRow {
            photo: label.to_string(),
            letter,
            field,
            x: text_box.x,
            y: text_box.y,
            rgb: text_box.rgb,
            ratio,
            margin: ratio.map(|ratio| LIT_BLUE_RATIO - ratio),
            lit: text_box.rgb.map(is_lit),
        });
    }
}

/// The nearest-rank percentile of one column of the log.
fn percentile(values: &[i64], percent: usize) -> Option<i64> {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    (!sorted.is_empty()).then(|| sorted[nearest_rank(sorted.len(), percent) - 1])
}

/// A share as a bar's result, judged in integers (`Bar::holds`).
fn share_result(bar: &Bar, of: u64, total: u64, field: Option<String>) -> BarResult {
    let met = (total > 0).then(|| bar.holds(of as f64 * 100.0, bar.number * total as f64));
    BarResult {
        id: bar.id.clone(),
        what: bar.what.clone(),
        limit: bar.limit.clone(),
        number: (total > 0).then(|| of as f64 * 100.0 / total as f64),
        of: Some((of, total)),
        field,
        met,
    }
}

/// A count or a time as a bar's result.
fn number_result(bar: &Bar, number: Option<f64>, of: Option<(u64, u64)>) -> BarResult {
    BarResult {
        id: bar.id.clone(),
        what: bar.what.clone(),
        limit: bar.limit.clone(),
        number,
        of,
        field: None,
        met: number.map(|number| bar.holds(number, bar.number)),
    }
}

/// The file's three outcomes. **A bar with nothing to measure has not been met**, so a valid session
/// cannot pass on an absence.
fn verdict_of(valid: bool, bars: &[BarResult]) -> Verdict {
    if !valid {
        return Verdict::Invalid;
    }
    let missed = |id: &BarResult| id.met != Some(true);
    if bars.iter().filter(|bar| bar.id != "T5").any(missed) {
        Verdict::Miss
    } else if bars.iter().any(missed) {
        Verdict::MissLatency
    } else {
        Verdict::Pass
    }
}

/// `golf-bay-report bars`: the export at `path` judged against `bars`, over `source`'s photos.
pub fn bars_report(
    path: &Path,
    bars: &Bars,
    source: SourceChoice,
    since: Option<Timestamp>,
) -> Result<BarsReport, String> {
    let export = Export::open(path)?;
    let dirs = Dirs::of(export.root());
    let fields = screen_fields()?;
    let profile = load_profile(DEVICE).map_err(|refused| refused.to_string())?;
    let shots = ShotStore::new(&dirs.shots);
    let entries = entries_by_digest(&dirs);

    let mut notes = Vec::new();
    let every_line = read_lines(&dirs, &mut notes)?;
    let lines = lines_since(&every_line, since, &mut notes);
    let photos = photo_lines(&lines);

    let mut tally = Photos {
        counted: 0,
        read: 0,
        flagged: 0,
        failed: 0,
        deleted: 0,
        duplicate_lines: lines
            .iter()
            .filter(|line| line.status == RecordStatus::Duplicate)
            .count() as u64,
        by_source: BTreeMap::new(),
        first_read: None,
        last_read: None,
    };
    let mut camera_photos = 0;
    let mut camera_clubs = BTreeSet::new();
    let mut cells = Cells::default();
    let mut by_field: Vec<FieldRow> = fields
        .iter()
        .map(|field| FieldRow {
            field: field.clone(),
            cells: Cells::default(),
        })
        .collect();
    let mut clubs: BTreeMap<Option<String>, ClubRow> = BTreeMap::new();
    let mut missed = Vec::new();
    let mut wrong_unflagged = 0;
    let mut timings = Vec::new();
    let mut confidences = Confidences {
        boxes: 0,
        photos: 0,
        lowest: None,
        histogram: Vec::new(),
        below_one: 0,
        below_one_behind_a_cell: 0,
        values_moved: Vec::new(),
        flagged_by_a_confidence: Vec::new(),
        read_cells: 0,
        read_cells_below_one: 0,
        read_cells_lowest: None,
        wrong_cells: Vec::new(),
        wrong_below_every_read: 0,
        separates: String::new(),
    };
    let mut letters = Vec::new();
    let mut deleted = Vec::new();
    let mut boxless = Vec::new();

    for line in &photos {
        let digest = line.photo_sha256.as_str();
        let entry = entries.get(digest);
        *tally.by_source.entry(spelled(line.source)).or_insert(0) += 1;
        let club = entry
            .and_then(|entry| entry.club)
            .map(|club| club.as_str().to_string());
        if line.source == ReadSource::Camera {
            camera_photos += 1;
            camera_clubs.extend(club.clone());
        }
        if !source.admits(line.source) {
            continue;
        }

        let label = photo_label(entry, digest);
        let flagged = matches!(line.status, RecordStatus::Flagged | RecordStatus::Failed);
        tally.counted += 1;
        tally.first_read.get_or_insert_with(|| line.at.clone());
        tally.last_read = Some(line.at.clone());
        match line.status {
            RecordStatus::Read => tally.read += 1,
            RecordStatus::Flagged => tally.flagged += 1,
            RecordStatus::Failed => tally.failed += 1,
            RecordStatus::Duplicate => unreachable!("photo_lines leaves out duplicates"),
        }
        timings.push((line.ms_total, line.ms_vision, line.ms_rust));
        let row = clubs.entry(club.clone()).or_insert(ClubRow {
            club,
            photos: 0,
            flagged: 0,
            cells: Cells::default(),
        });
        row.photos += 1;
        row.flagged += u64::from(flagged);

        let lowest = match boxes_of(&dirs, digest) {
            Ok(boxes) => {
                letter_rows(&boxes, profile, &label, &mut letters);
                Some(weigh_boxes(&boxes, profile, &label, &mut confidences))
            }
            Err(why) => {
                boxless.push(format!("{label}: {why}"));
                None
            }
        };

        // **A failed read is twelve withheld cells, counted without a value**: its truth is never typed
        // in, since there is no shot to type it into.
        let photo_cells = if line.status == RecordStatus::Failed {
            fields
                .iter()
                .map(|field| (field.clone(), None, None))
                .collect()
        } else {
            let shot = shots
                .get(digest)
                .map_err(|failed| format!("{label}: {failed}"))?;
            match shot {
                Some(shot) => cells_of(&shot, &fields)?,
                None => {
                    tally.deleted += 1;
                    deleted.push(label.clone());
                    Vec::new()
                }
            }
        };

        for (at, (field, as_read, truth)) in photo_cells.into_iter().enumerate() {
            let kind = if line.status == RecordStatus::Failed {
                CellKind::Withheld
            } else {
                classify(as_read, truth)
            };
            cells.add(kind);
            by_field[at].cells.add(kind);
            row.cells.add(kind);
            if matches!(kind, CellKind::Wrong | CellKind::Withheld) {
                missed.push(MissedCell {
                    photo: label.clone(),
                    field: field.clone(),
                    kind,
                    as_read,
                    truth,
                    flagged,
                });
            }
            wrong_unflagged += u64::from(kind == CellKind::Wrong && !flagged);
            let behind = lowest
                .as_ref()
                .map(|lowest| lowest.get(&field).copied().unwrap_or(1.0));
            match kind {
                CellKind::Read => {
                    if let Some(behind) = behind {
                        confidences.read_cells += 1;
                        confidences.read_cells_below_one += u64::from(behind < 1.0);
                        if confidences
                            .read_cells_lowest
                            .is_none_or(|known| behind < known)
                        {
                            confidences.read_cells_lowest = Some(behind);
                        }
                    }
                }
                CellKind::Wrong => confidences.wrong_cells.push(WrongCell {
                    photo: label.clone(),
                    field,
                    lowest: behind,
                }),
                CellKind::Withheld | CellKind::Blank => {}
            }
        }
    }

    // Every line and not only those since `--since`, or a rehearsal's shot would read as unlogged.
    let logged: BTreeSet<&str> = photo_lines(&every_line)
        .into_iter()
        .map(|line| line.photo_sha256.as_str())
        .collect();
    let unlogged: Vec<String> = entries
        .iter()
        .filter(|(digest, _)| !logged.contains(digest.as_str()))
        .map(|(digest, entry)| photo_label(Some(entry), digest))
        .collect();
    if !deleted.is_empty() {
        notes.push(format!(
            "{} photo(s) have a read line and no stored shot, so were deleted at the phone: they count \
             for the session's size, T4 and T5, and hold no cells: {}.",
            deleted.len(),
            deleted.join(", ")
        ));
    }
    if !unlogged.is_empty() {
        notes.push(format!(
            "{} stored shot(s) have no read line, so nothing says how they arrived, and they are in no \
             count: {}.",
            unlogged.len(),
            unlogged.join(", ")
        ));
    }
    if !boxless.is_empty() {
        notes.push(format!(
            "{} photo(s) have no boxes to weigh, so are in neither the confidences nor the sign letters: \
             {}.",
            boxless.len(),
            boxless.join("; ")
        ));
    }

    let (wrong_below, separates) = separation(&confidences);
    confidences.wrong_below_every_read = wrong_below;
    confidences.separates = separates;

    let totals: Vec<i64> = timings.iter().map(|&(total, _, _)| total).collect();
    let visions: Vec<i64> = timings.iter().map(|&(_, vision, _)| vision).collect();
    let rusts: Vec<i64> = timings.iter().map(|&(_, _, rust)| rust).collect();
    let latency = Latency {
        photos: tally.counted,
        total_p95: percentile(&totals, 95),
        total_median: percentile(&totals, 50),
        total_max: totals.iter().copied().max(),
        vision_p95: percentile(&visions, 95),
        rust_p95: percentile(&rusts, 95),
    };

    let lowest_field = lowest_field(&by_field);
    let flagged_photos = tally.flagged + tally.failed;
    let results = vec![
        number_result(bars.bar("T1"), Some(wrong_unflagged as f64), None),
        share_result(bars.bar("T2"), cells.read, cells.judged(), None),
        match lowest_field {
            Some(row) => share_result(
                bars.bar("T3"),
                row.cells.read,
                row.cells.judged(),
                Some(row.field.clone()),
            ),
            None => share_result(bars.bar("T3"), 0, 0, None),
        },
        share_result(bars.bar("T4"), flagged_photos, tally.counted, None),
        number_result(
            bars.bar("T5"),
            latency.total_p95.map(|p95| p95 as f64),
            (!totals.is_empty())
                .then(|| (nearest_rank(totals.len(), 95) as u64, totals.len() as u64)),
        ),
    ];

    let session = Session {
        // **A session is camera photos**, so another source's numbers can be read and earn no verdict.
        valid: source == SourceChoice::Only(ReadSource::Camera)
            && camera_photos >= bars.photos
            && camera_clubs.len() as u64 >= bars.clubs,
        camera_photos,
        needs_photos: bars.photos,
        clubs: camera_clubs.into_iter().collect(),
        needs_clubs: bars.clubs,
    };
    let verdict = verdict_of(session.valid, &results);

    let letters = Letters {
        blue_ratio: LIT_BLUE_RATIO,
        smallest_lit: nearest_the_line(&letters, true),
        smallest_unlit: nearest_the_line(&letters, false),
        rows: letters,
    };

    Ok(BarsReport {
        source: source.name(),
        since: since.map(|since| since.to_string()),
        session,
        bars: results,
        verdict,
        photos: tally,
        cells,
        fields: by_field,
        missed,
        clubs: clubs.into_values().collect(),
        latency,
        confidences,
        letters,
        notes,
    })
}

/// T3's field: the lowest share of read cells among fields with any judged, the first of a tie standing.
fn lowest_field(rows: &[FieldRow]) -> Option<&FieldRow> {
    rows.iter()
        .filter(|row| row.cells.judged() > 0)
        // Compared as cross-multiplied integers, and `<` so the first of a tie stands.
        .fold(None::<&FieldRow>, |lowest, row| match lowest {
            Some(held)
                if held.cells.read * row.cells.judged() <= row.cells.read * held.cells.judged() =>
            {
                Some(held)
            }
            _ => Some(row),
        })
}

/// The wrong cells whose lowest confidence sits below every read cell's, and P32's question in a word.
fn separation(confidences: &Confidences) -> (u64, String) {
    let wrong = &confidences.wrong_cells;
    if wrong.is_empty() {
        return (0, "no wrong cell".to_string());
    }
    let below = confidences.read_cells_lowest.map_or(0, |read_lowest| {
        wrong
            .iter()
            .filter(|cell| cell.lowest.is_some_and(|lowest| lowest < read_lowest))
            .count() as u64
    });
    let word = if wrong.iter().all(|cell| cell.lowest.is_none()) {
        "unknown"
    } else if below == 0 {
        "no"
    } else if below == wrong.len() as u64 {
        "yes"
    } else {
        "partly"
    };
    (below, word.to_string())
}

/// The margin nearest `LIT_BLUE_RATIO` on one side, which is what P30 and P32 record.
fn nearest_the_line(rows: &[LetterRow], lit: bool) -> Option<f64> {
    rows.iter()
        .filter(|row| row.lit == Some(lit))
        .filter_map(|row| row.margin)
        .fold(None, |nearest: Option<f64>, margin| {
            Some(nearest.map_or(margin, |held| {
                if lit {
                    held.min(margin)
                } else {
                    held.max(margin)
                }
            }))
        })
}

// --- the truth -----------------------------------------------------------------------------------

/// What a table read off M33's captures says it is (rule 12).
pub const PASTED: &str = "provisional: pasted copies";

/// `spikes/vision-ocr/photos.json`, in the keys this reads.
#[derive(Debug, Deserialize)]
struct PhotosFile {
    captures: Vec<Capture>,
}

/// One photo the truth table has a row for.
#[derive(Debug, Deserialize)]
struct Capture {
    id: String,
    path: String,
    sha256: String,
    device: String,
    truth: String,
    copy_of: Option<CopyOf>,
}

/// The photo a capture is a copy of, which is what its table's `provisional` rests on.
#[derive(Debug, Deserialize)]
struct CopyOf {
    sha256: String,
}

/// `spikes/vision-ocr/truth_full_swing.json`.
#[derive(Debug, Deserialize)]
struct TruthFile {
    photos: BTreeMap<String, TruthRow>,
}

/// One row, in the table's own spelling: a signed tile as its magnitude and its lit letter, an apex in
/// feet. Every column is required, so a row with one missing is refused and not read as withheld.
#[derive(Debug, Deserialize)]
struct TruthRow {
    carry: f64,
    total: f64,
    club: f64,
    ball: f64,
    path: f64,
    face: f64,
    ball_dir: (f64, String),
    launch: f64,
    apex_ft: f64,
    smash: f64,
    backspin: f64,
    sidespin: (f64, String),
}

impl TruthRow {
    /// In `ShotData`'s conventions, as M33's `truth()` has them, and in `fields`' order. **`apex_ft / 3.0`
    /// and not a typed decimal**: that division is the parser's own float, and a comparison with no
    /// tolerance would fail a correct read against the nearest two-decimal spelling of it.
    fn in_stored_conventions(&self, id: &str, fields: &[String]) -> Result<Vec<f64>, String> {
        let signed = |(magnitude, letter): &(f64, String), minus: &str, plus: &str| {
            if letter == minus {
                Ok(-*magnitude)
            } else if letter == plus {
                Ok(*magnitude)
            } else {
                Err(format!(
                    "the truth for {id}: {letter} is neither {minus} nor {plus}"
                ))
            }
        };
        let columns = [
            ("carry_distance", self.carry),
            ("total_distance", self.total),
            ("club_head_speed", self.club),
            ("ball_speed", self.ball),
            ("club_path", self.path),
            ("club_face_angle", self.face),
            ("launch_direction", signed(&self.ball_dir, "L", "R")?),
            ("launch_angle", self.launch),
            ("apex_height", self.apex_ft / 3.0),
            ("smash_factor", self.smash),
            ("backspin", self.backspin),
            ("sidespin", signed(&self.sidespin, "D", "F")?),
        ];
        fields
            .iter()
            .map(|field| {
                columns
                    .iter()
                    .find(|(column, _)| column == field)
                    .map(|&(_, value)| value)
                    .ok_or_else(|| {
                        format!(
                            "the device declares {field}, and the truth table has no such column"
                        )
                    })
            })
            .collect()
    }
}

fn read_json<T: DeserializeOwned>(path: &Path, what: &str) -> Result<T, String> {
    let shown = path.display();
    let text = fs::read_to_string(path)
        .map_err(|failed| format!("could not read the {what} at {shown}: {failed}"))?;
    serde_json::from_str(&text).map_err(|failed| format!("the {what} at {shown}: {failed}"))
}

/// What an export holds for one digest.
enum Held {
    Shot(Box<ShotData>),
    /// M33's `None`: nothing was stored, and to a golfer that is a retake.
    Failed,
    /// A `read` or `flagged` line and no shot (the module doc).
    Deleted,
}

/// What the stores and the log hold under `digest`, `bars`' order of asking. **The boxes are how a read
/// of nothing is known with no line**, which is P18's road: `record_photo` keeps them before it looks at
/// them (C7, C13). The only other way to boxes, no shot and no line is a delete after a crash ate the
/// line, and to read that as a retake costs one capture's cells and hides nothing.
fn held_for(
    digest: &str,
    lines: &BTreeMap<&str, &ReadLog>,
    shots: &ShotStore,
    dirs: &Dirs,
) -> Result<Option<Held>, String> {
    let line = lines.get(digest);
    if line.is_some_and(|line| line.status == RecordStatus::Failed) {
        return Ok(Some(Held::Failed));
    }
    let short = digest.get(..12).unwrap_or(digest);
    let shot = shots
        .get(digest)
        .map_err(|failed| format!("photo {short}: {failed}"))?;
    Ok(match shot {
        Some(shot) => Some(Held::Shot(Box::new(shot))),
        None if line.is_some() => Some(Held::Deleted),
        None if dirs
            .app
            .join("boxes")
            .join(format!("{digest}.json"))
            .is_file() =>
        {
            Some(Held::Failed)
        }
        None => None,
    })
}

/// The digests each stem of an entry's arrival name holds, for what a digest could not find.
fn entries_by_stem(entries: &BTreeMap<String, SwingManifest>) -> BTreeMap<&str, Vec<&str>> {
    let mut stems: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (digest, entry) in entries {
        let stem = entry
            .roles
            .get(&Role::ShotScreen)
            .and_then(|photo| Path::new(&photo.original_filename).file_stem())
            .and_then(OsStr::to_str);
        if let Some(stem) = stem {
            stems.entry(stem).or_default().push(digest);
        }
    }
    stems
}

/// One capture's cell, M33's `classify`: **a wrong number on a flagged photo is `Caught`**, since a flag
/// questions the shot and a person is asked to look. [`classify`] holds the comparison.
fn truth_cell(field: &str, as_read: Option<f64>, truth: f64, flagged: bool) -> TruthCell {
    let kind = match classify(as_read, Some(truth)) {
        CellKind::Read => TruthCellKind::Read,
        CellKind::Withheld => TruthCellKind::Withheld,
        CellKind::Wrong if flagged => TruthCellKind::Caught,
        CellKind::Wrong => TruthCellKind::Wrong,
        CellKind::Blank => unreachable!("a truth row holds a value in every column"),
    };
    TruthCell {
        field: field.to_string(),
        kind,
        as_read,
        truth,
    }
}

/// `golf-bay-report truth`, and `truth_report` across the bridge: the reads in the export at `export`
/// (the module doc), held to the table at `truth`, for the captures at `photos`.
pub fn truth_of(export: &Path, photos: &Path, truth: &Path) -> Result<TruthReport, String> {
    let captures = read_json::<PhotosFile>(photos, "photos file")?.captures;
    if captures.is_empty() {
        return Err(format!(
            "the photos file at {} holds no captures",
            photos.display()
        ));
    }
    let table = read_json::<TruthFile>(truth, "truth table")?.photos;
    let export = Export::open(export)?;
    let dirs = Dirs::of(export.root());
    let fields = screen_fields()?;
    let shots = ShotStore::new(&dirs.shots);
    let entries = entries_by_digest(&dirs);
    let stems = entries_by_stem(&entries);

    let mut notes = Vec::new();
    let lines = read_log(&dirs, &mut notes)?.unwrap_or_default();
    let by_digest: BTreeMap<&str, &ReadLog> = photo_lines(&lines)
        .into_iter()
        .map(|line| (line.photo_sha256.as_str(), line))
        .collect();

    // On data rather than memory, as M33's `run` has it: a capture whose digest is not its original's.
    let pasted = captures.iter().any(|capture| {
        capture
            .copy_of
            .as_ref()
            .is_some_and(|original| original.sha256 != capture.sha256)
    });

    let mut photos_out = Vec::new();
    let mut used = BTreeSet::new();
    let mut unlogged = Vec::new();
    for capture in &captures {
        if capture.device != DEVICE {
            return Err(format!(
                "{} is a capture of {}, and this reads {DEVICE}",
                capture.id, capture.device
            ));
        }
        let row = table.get(&capture.truth).ok_or_else(|| {
            format!(
                "{} answers to the truth row {}, and the table has none",
                capture.id, capture.truth
            )
        })?;
        let want = row.in_stored_conventions(&capture.truth, &fields)?;

        let mut digest = capture.sha256.as_str();
        let mut route = TruthRoute::Digest;
        let mut held = held_for(digest, &by_digest, &shots, &dirs)?;
        if held.is_none() {
            let stem = Path::new(&capture.path)
                .file_stem()
                .and_then(OsStr::to_str)
                .unwrap_or(&capture.path);
            match stems.get(stem).map(Vec::as_slice) {
                Some([only]) => {
                    digest = only;
                    route = TruthRoute::Name;
                    held = held_for(digest, &by_digest, &shots, &dirs)?;
                }
                Some(several) => {
                    route = TruthRoute::Ambiguous;
                    let names: Vec<String> = several
                        .iter()
                        .map(|digest| photo_label(entries.get(*digest), digest))
                        .collect();
                    used.extend(several.iter().copied());
                    notes.push(format!(
                        "{} matches no digest and {} entries by name, so none was chosen: {}.",
                        capture.id,
                        several.len(),
                        names.join(", ")
                    ));
                }
                None => route = TruthRoute::Absent,
            }
        }

        let mut photo = TruthPhoto {
            id: capture.id.clone(),
            route,
            entry: None,
            flagged: false,
            failed_read: false,
            cells: Vec::new(),
        };
        if route != TruthRoute::Ambiguous {
            photo.entry = held
                .as_ref()
                .map(|_| photo_label(entries.get(digest), digest));
        }
        match held {
            Some(Held::Shot(shot)) => {
                used.insert(digest);
                let read = cells_of(&shot, &fields)?;
                photo.flagged = match by_digest.get(digest) {
                    Some(line) => {
                        matches!(line.status, RecordStatus::Flagged | RecordStatus::Failed)
                    }
                    None => {
                        unlogged.push(capture.id.clone());
                        shot.provenance
                            .as_ref()
                            .is_some_and(|provenance| provenance.needs_review)
                    }
                };
                photo.cells = read
                    .iter()
                    .zip(&want)
                    .map(|((field, as_read, _), &truth)| {
                        truth_cell(field, *as_read, truth, photo.flagged)
                    })
                    .collect();
            }
            Some(Held::Failed) => {
                used.insert(digest);
                photo.flagged = true;
                photo.failed_read = true;
                photo.cells = fields
                    .iter()
                    .zip(&want)
                    .map(|(field, &truth)| truth_cell(field, None, truth, true))
                    .collect();
            }
            Some(Held::Deleted) => {
                used.insert(digest);
                photo.route = TruthRoute::Deleted;
            }
            None if route == TruthRoute::Ambiguous => {}
            // An entry found by name with neither a shot, a line nor boxes under its digest, which
            // nothing writes, so it can stand for nothing.
            None => photo.route = TruthRoute::Absent,
        }
        photos_out.push(photo);
    }

    if !unlogged.is_empty() {
        notes.push(format!(
            "{} photo(s) have no read line, so the flag shown is the one stored, which a review clears: \
             {}.",
            unlogged.len(),
            unlogged.join(", ")
        ));
    }
    let every: BTreeSet<&str> = entries
        .keys()
        .map(String::as_str)
        .chain(by_digest.keys().copied())
        .collect();
    let others = every.difference(&used).count();
    if others > 0 {
        notes.push(format!(
            "{others} other photo(s) in the export match no capture, and are in no count."
        ));
    }
    let lost = photos_out
        .iter()
        .any(|photo| photo.route == TruthRoute::Absent);
    let failures = by_digest
        .iter()
        .filter(|(digest, line)| line.status == RecordStatus::Failed && !used.contains(*digest))
        .count();
    if lost && failures > 0 {
        notes.push(format!(
            "{failures} of the log's failed reads match no capture, and a failed read keeps no name, so \
             a re-encoded capture that failed to read would be among them."
        ));
    }

    Ok(truth_table(photos_out, &fields, pasted, notes))
}

/// The counts, the words and the text, from each capture's cells.
fn truth_table(
    photos: Vec<TruthPhoto>,
    fields: &[String],
    pasted: bool,
    notes: Vec<String>,
) -> TruthReport {
    let mut by_field: Vec<FieldRow> = fields
        .iter()
        .map(|field| FieldRow {
            field: field.clone(),
            cells: Cells::default(),
        })
        .collect();
    let mut caught = 0;
    for photo in &photos {
        for (at, cell) in photo.cells.iter().enumerate() {
            let kind = match cell.kind {
                TruthCellKind::Read => CellKind::Read,
                TruthCellKind::Withheld => CellKind::Withheld,
                TruthCellKind::Wrong => CellKind::Wrong,
                TruthCellKind::Caught => {
                    caught += 1;
                    CellKind::Wrong
                }
            };
            by_field[at].cells.add(kind);
        }
    }
    let sum = |of: fn(&Cells) -> u64| by_field.iter().map(|row| of(&row.cells)).sum::<u64>() as i64;
    let judged_cells = sum(Cells::judged);
    let matched = photos
        .iter()
        .filter(|photo| !photo.cells.is_empty())
        .count() as i64;
    let wrong = sum(|cells| cells.wrong) - caught;
    let lowest = lowest_field(&by_field);
    let mut report = TruthReport {
        label: pasted.then(|| PASTED.to_string()),
        captures: photos.len() as i64,
        matched,
        by_digest: count_route(&photos, TruthRoute::Digest),
        by_name: count_route(&photos, TruthRoute::Name),
        read: sum(|cells| cells.read),
        wrong,
        caught,
        withheld: sum(|cells| cells.withheld),
        judged: judged_cells,
        flagged: photos.iter().filter(|photo| photo.flagged).count() as i64,
        lowest_field: lowest.map(|row| row.field.clone()),
        lowest_read: lowest.map_or(0, |row| row.cells.read as i64),
        lowest_judged: lowest.map_or(0, |row| row.cells.judged() as i64),
        held: wrong == 0 && matched == photos.len() as i64,
        fields: by_field
            .iter()
            .map(|row| TruthField {
                field: row.field.clone(),
                read: row.cells.read as i64,
                judged: row.cells.judged() as i64,
            })
            .collect(),
        photos,
        notes,
        text: String::new(),
    };
    report.text = render_truth(&report);
    report
}

fn count_route(photos: &[TruthPhoto], route: TruthRoute) -> i64 {
    photos.iter().filter(|photo| photo.route == route).count() as i64
}

// --- the corpus ----------------------------------------------------------------------------------

/// The first place two corpora part. `None` on a side is a key it does not have.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CorpusDifference {
    pub path: String,
    pub in_export: Option<Value>,
    pub in_other: Option<Value>,
}

/// What `golf-bay-report compare` found.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Comparison {
    pub player: String,
    /// The export's.
    pub swings: u64,
    pub excluded: u64,
    /// The other side's. The first difference is often an aggregate, since keys are walked sorted, so
    /// these are what says a whole shot is missing.
    pub other_swings: u64,
    pub other_excluded: u64,
    /// `None` is equal.
    pub difference: Option<CorpusDifference>,
}

/// `read_corpus` over the export and over `sessions_dir`.
///
/// **Two empty corpora are refused and not called equal.** An unknown golfer is an empty corpus, as
/// `read_corpus` documents, so a mistyped `--player` would otherwise answer the milestone's exit with
/// a comparison of nothing. So is a `sessions_dir` that is no directory, which reads the same way.
pub fn compare_corpora(
    export: &Path,
    sessions_dir: &Path,
    player: &str,
) -> Result<Comparison, String> {
    if !sessions_dir.is_dir() {
        return Err(format!("{} is not a directory", sessions_dir.display()));
    }
    let export = Export::open(export)?;
    let phone = read_corpus(&Dirs::of(export.root()).sessions, player, engine_versions());
    let other = read_corpus(sessions_dir, player, engine_versions());
    let empty = |corpus: &contracts::career::CareerCorpus| {
        corpus.swings.is_empty() && corpus.excluded.is_empty()
    };
    if empty(&phone) && empty(&other) {
        return Err(format!(
            "neither side holds a swing for '{player}', so nothing was compared. It is a golfer's \
             player_id, e.g. aaron"
        ));
    }
    let as_json = |corpus: &contracts::career::CareerCorpus| {
        serde_json::to_value(corpus).map_err(|failed| failed.to_string())
    };
    Ok(Comparison {
        player: player.to_string(),
        swings: phone.swings.len() as u64,
        excluded: phone.excluded.len() as u64,
        other_swings: other.swings.len() as u64,
        other_excluded: other.excluded.len() as u64,
        difference: first_difference(&as_json(&phone)?, &as_json(&other)?, ""),
    })
}

/// In sorted key order, as the documents hold them. Lists of two lengths part at the list, because an
/// element missing from the middle would shift every one after it into a difference that is not one.
fn first_difference(export: &Value, other: &Value, path: &str) -> Option<CorpusDifference> {
    match (export, other) {
        (Value::Object(left), Value::Object(right)) => {
            let keys: BTreeSet<&String> = left.keys().chain(right.keys()).collect();
            keys.into_iter().find_map(|key| {
                let here = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                match (left.get(key), right.get(key)) {
                    (Some(left), Some(right)) => first_difference(left, right, &here),
                    (left, right) => Some(CorpusDifference {
                        path: here,
                        in_export: left.cloned(),
                        in_other: right.cloned(),
                    }),
                }
            })
        }
        (Value::Array(left), Value::Array(right)) if left.len() == right.len() => left
            .iter()
            .zip(right)
            .enumerate()
            .find_map(|(at, (left, right))| {
                first_difference(left, right, &format!("{path}[{at}]"))
            }),
        (left, right) if left == right => None,
        (left, right) => Some(CorpusDifference {
            path: path.to_string(),
            in_export: Some(left.clone()),
            in_other: Some(right.clone()),
        }),
    }
}

// --- the text ------------------------------------------------------------------------------------

/// `count of total (share%)`, or `count of 0` with no share to give.
fn share(count: u64, total: u64) -> String {
    if total == 0 {
        format!("{count} of 0")
    } else {
        format!(
            "{count} of {total} ({:.1}%)",
            count as f64 * 100.0 / total as f64
        )
    }
}

/// `0 photos`, or how many and which.
fn photos_named(photos: &[String]) -> String {
    if photos.is_empty() {
        "0 photos".to_string()
    } else {
        format!("{} photos ({})", photos.len(), photos.join(", "))
    }
}

fn or_none<T: ToString>(value: Option<T>) -> String {
    value.map_or_else(|| "none".to_string(), |value| value.to_string())
}

fn cell_number(value: Option<f64>) -> String {
    value.map_or_else(|| "not read".to_string(), pyfmt::repr)
}

fn bar_number(bar: &BarResult) -> String {
    let Some(number) = bar.number else {
        return "nothing to measure".to_string();
    };
    match (bar.id.as_str(), bar.of, &bar.field) {
        ("T5", Some((rank, photos)), _) => format!("{number} ms (rank {rank} of {photos})"),
        (_, Some((of, total)), Some(field)) => format!("{number:.1}% ({of} of {total}, {field})"),
        (_, Some((of, total)), None) => format!("{number:.1}% ({of} of {total})"),
        _ => format!("{number}"),
    }
}

/// The verdict line: the file's outcome and what follows from it, or why there is no verdict.
fn verdict_line(report: &BarsReport) -> String {
    let session = &report.session;
    match report.verdict {
        Verdict::Pass => "VERDICT: PASS. All five bars met, so the app's path stands.".to_string(),
        Verdict::Miss => {
            let missed: Vec<&str> = report
                .bars
                .iter()
                .filter(|bar| bar.id != "T5" && bar.met != Some(true))
                .map(|bar| bar.id.as_str())
                .collect();
            format!(
                "VERDICT: MISS. {} missed, which triggers P33 and P34. It does not fail the milestone by \
                 itself: the user decides what ships with both numbers in front of them.",
                missed.join(", ")
            )
        }
        Verdict::MissLatency => "VERDICT: MISS (latency). Only T5 missed, so the scanner would not \
                                 help and P33 is not triggered. The user decides at P32."
            .to_string(),
        Verdict::Invalid if report.source != "camera" => format!(
            "VERDICT: INVALID. These are {} photos, and a session is camera photos, so nothing above \
             is judged.",
            report.source
        ),
        Verdict::Invalid => format!(
            "VERDICT: INVALID. Not a session ({} camera photos of at least {}, {} clubs of at least {}), \
             so nothing above is judged.",
            session.camera_photos,
            session.needs_photos,
            session.clubs.len(),
            session.needs_clubs
        ),
    }
}

/// The report as a person reads it, and as P32 copies it into the results file.
pub fn render_bars(report: &BarsReport) -> String {
    let mut out = String::new();
    let mut line = |text: String| {
        out.push_str(&text);
        out.push('\n');
    };
    let session = &report.session;
    line(format!(
        "Session: {}. {} camera photos, at least {} asked. {} clubs, at least {} asked{}.",
        if session.valid { "VALID" } else { "INVALID" },
        session.camera_photos,
        session.needs_photos,
        session.clubs.len(),
        session.needs_clubs,
        if session.clubs.is_empty() {
            String::new()
        } else {
            format!(": {}", session.clubs.join(", "))
        }
    ));
    line(String::new());

    line(format!(
        "Bars, over {} photos{}:",
        report.source,
        report
            .since
            .as_ref()
            .map_or_else(String::new, |since| format!(" read since {since}"))
    ));
    for bar in &report.bars {
        line(format!(
            "  {:3} {:<70} {:<34} {:<18} {}",
            bar.id,
            bar.what,
            bar_number(bar),
            bar.limit,
            match bar.met {
                Some(true) => "PASS",
                Some(false) => "MISS",
                None => "MISS (nothing to measure)",
            }
        ));
    }
    line(verdict_line(report));
    line(String::new());

    let photos = &report.photos;
    line(format!(
        "Photos: {} counted: {} read, {} flagged, {} failed. {} deleted at the phone.",
        photos.counted, photos.read, photos.flagged, photos.failed, photos.deleted
    ));
    line(format!(
        "  success {}, review {}",
        share(photos.read + photos.flagged, photos.counted),
        share(photos.flagged + photos.failed, photos.counted)
    ));
    if let (Some(first), Some(last)) = (&photos.first_read, &photos.last_read) {
        line(format!(
            "  read from {first} to {last}. A session is one sitting: --since leaves out what came before."
        ));
    }
    let sources: Vec<String> = photos
        .by_source
        .iter()
        .map(|(source, count)| format!("{source} {count}"))
        .collect();
    line(format!(
        "  every photo in the export: {}. Lines saying duplicate, no photo for any count: {}.",
        sources.join(", "),
        photos.duplicate_lines
    ));
    line(String::new());

    let cells = &report.cells;
    line(format!(
        "Cells: {} read, {} wrong, {} withheld, {} blank.",
        cells.read, cells.wrong, cells.withheld, cells.blank
    ));
    line(format!(
        "  {:<18} {:>5} {:>6} {:>9} {:>6}  read share",
        "field", "read", "wrong", "withheld", "blank"
    ));
    for row in &report.fields {
        line(format!(
            "  {:<18} {:>5} {:>6} {:>9} {:>6}  {}",
            row.field,
            row.cells.read,
            row.cells.wrong,
            row.cells.withheld,
            row.cells.blank,
            share(row.cells.read, row.cells.judged())
        ));
    }
    line(String::new());

    line("Wrong and withheld cells, by photo and field, in stored units:".to_string());
    if report.missed.is_empty() {
        line("  none".to_string());
    }
    for cell in &report.missed {
        line(format!(
            "  {} {}: {}, as read {}, truth {}{}",
            cell.photo,
            cell.field,
            if cell.kind == CellKind::Wrong {
                "wrong"
            } else {
                "withheld"
            },
            cell_number(cell.as_read),
            cell_number(cell.truth),
            if cell.flagged { ", flagged" } else { "" }
        ));
    }
    line(String::new());

    line("Per club:".to_string());
    line(format!(
        "  {:<8} {:>6} {:>8} {:>5} {:>6} {:>9} {:>6}",
        "club", "photos", "flagged", "read", "wrong", "withheld", "blank"
    ));
    for row in &report.clubs {
        line(format!(
            "  {:<8} {:>6} {:>8} {:>5} {:>6} {:>9} {:>6}",
            row.club.as_deref().unwrap_or("no club"),
            row.photos,
            row.flagged,
            row.cells.read,
            row.cells.wrong,
            row.cells.withheld,
            row.cells.blank
        ));
    }
    line(String::new());

    let latency = &report.latency;
    line(format!(
        "Latency over {} photos: ms_total p95 {}, median {}, max {}. p95 of ms_vision {}, of ms_rust {}.",
        latency.photos,
        or_none(latency.total_p95),
        or_none(latency.total_median),
        or_none(latency.total_max),
        or_none(latency.vision_p95),
        or_none(latency.rust_p95)
    ));
    line(String::new());

    let held = &report.confidences;
    line(format!(
        "Box confidences (D8's evidence): {} boxes on {} photos, lowest {}.",
        held.boxes,
        held.photos,
        held.lowest
            .map_or_else(|| "none".to_string(), |lowest| format!("{lowest:.2}"))
    ));
    for bucket in &held.histogram {
        line(format!("  {:<10} {:>6}", bucket.range, bucket.boxes));
    }
    line(format!(
        "  below 1.00: {} boxes, {} behind a tile's cell and the rest strays.",
        held.below_one, held.below_one_behind_a_cell
    ));
    line(format!(
        "  did a cell's read depend on one? Re-read with every box at 1.00 and at 0.00: a value moved on \
         {}, and a confidence alone flagged {}.",
        photos_named(&held.values_moved),
        photos_named(&held.flagged_by_a_confidence)
    ));
    line(format!(
        "  read cells: {}, {} with a box below 1.00 behind them, lowest {}.",
        held.read_cells,
        held.read_cells_below_one,
        held.read_cells_lowest
            .map_or_else(|| "none".to_string(), |lowest| format!("{lowest:.2}"))
    ));
    for cell in &held.wrong_cells {
        line(format!(
            "  wrong: {} {}, lowest box {}",
            cell.photo,
            cell.field,
            cell.lowest
                .map_or_else(|| "unknown".to_string(), |lowest| format!("{lowest:.2}"))
        ));
    }
    line(format!(
        "  did a confidence separate a wrong cell from a right one? {}: {} of {} wrong cells sit below \
         every read cell.",
        held.separates,
        held.wrong_below_every_read,
        held.wrong_cells.len()
    ));
    line(String::new());

    let letters = &report.letters;
    let margin =
        |margin: Option<f64>| margin.map_or_else(|| "none".to_string(), |m| format!("{m:+.3}"));
    line(format!(
        "Sign letters against LIT_BLUE_RATIO {}: smallest margins {} lit, {} unlit. Positive is the lit side.",
        letters.blue_ratio,
        margin(letters.smallest_lit),
        margin(letters.smallest_unlit)
    ));
    if letters.rows.is_empty() {
        line("  none".to_string());
    }
    for row in &letters.rows {
        let rgb = row.rgb.map_or_else(
            || "no colour".to_string(),
            |[red, green, blue]| format!("{red:.0},{green:.0},{blue:.0}"),
        );
        line(format!(
            "  {} {} at {:.0},{:.0} ({}): rgb {}, b/min {}, margin {}, lit {}",
            row.photo,
            row.letter,
            row.x,
            row.y,
            row.field,
            rgb,
            row.ratio
                .map_or_else(|| "none".to_string(), |ratio| format!("{ratio:.3}")),
            margin(row.margin),
            or_none(row.lit)
        ));
    }

    if !report.notes.is_empty() {
        line(String::new());
        line("Notes:".to_string());
        for note in &report.notes {
            line(format!("  {note}"));
        }
    }
    out
}

/// The truth table as a person reads it, as P18 and P28 copy it into their findings.
fn render_truth(report: &TruthReport) -> String {
    let mut out = String::new();
    let mut line = |text: String| {
        out.push_str(&text);
        out.push('\n');
    };
    if let Some(label) = &report.label {
        line(label.clone());
        line(String::new());
    }
    line(format!(
        "Captures: {} of {} matched, {} by digest and {} by name.",
        report.matched, report.captures, report.by_digest, report.by_name
    ));
    for photo in &report.photos {
        let count =
            |kind: TruthCellKind| photo.cells.iter().filter(|cell| cell.kind == kind).count();
        let route = match photo.route {
            TruthRoute::Digest => "digest",
            TruthRoute::Name => "name: a re-encoded copy",
            TruthRoute::Ambiguous => "name: ambiguous, no cells",
            TruthRoute::Deleted => "deleted at the phone, no cells",
            TruthRoute::Absent => "not in this export",
        };
        let mut row = format!("  {:<14} {route:<30}", photo.id);
        if !photo.cells.is_empty() {
            row.push_str(&format!(
                " read {}, wrong {}, caught {}, withheld {}{}{}",
                count(TruthCellKind::Read),
                count(TruthCellKind::Wrong),
                count(TruthCellKind::Caught),
                count(TruthCellKind::Withheld),
                if photo.flagged { ", flagged" } else { "" },
                if photo.failed_read {
                    ", a failed read"
                } else {
                    ""
                },
            ));
        }
        if let Some(entry) = &photo.entry {
            row.push_str(&format!("  [{entry}]"));
        }
        line(row);
    }
    line(String::new());

    line(format!(
        "T1 wrong cells on photos not flagged: {}. Caught on flagged photos: {}.",
        report.wrong, report.caught
    ));
    line(format!(
        "T2 read cells: {}",
        share(report.read as u64, report.judged as u64)
    ));
    line(format!(
        "T3 the lowest field: {}",
        match &report.lowest_field {
            Some(field) => format!(
                "{field}, {}",
                share(report.lowest_read as u64, report.lowest_judged as u64)
            ),
            None => "none".to_string(),
        }
    ));
    line(format!(
        "T4 flagged photos: {}",
        share(report.flagged as u64, report.matched as u64)
    ));
    line(String::new());

    line(format!(
        "Cells: {} read, {} wrong, {} caught, {} withheld.",
        report.read, report.wrong, report.caught, report.withheld
    ));
    for row in &report.fields {
        line(format!(
            "  {:<18} {}",
            row.field,
            share(row.read as u64, row.judged as u64)
        ));
    }
    line(String::new());

    line("Wrong, caught and withheld cells, in stored units:".to_string());
    let mut missed = 0;
    for photo in &report.photos {
        for cell in &photo.cells {
            let kind = match cell.kind {
                TruthCellKind::Read => continue,
                TruthCellKind::Wrong => "wrong",
                TruthCellKind::Caught => "caught",
                TruthCellKind::Withheld => "withheld",
            };
            missed += 1;
            line(format!(
                "  {} {}: {kind}, as read {}, truth {}",
                photo.id,
                cell.field,
                cell_number(cell.as_read),
                pyfmt::repr(cell.truth)
            ));
        }
    }
    if missed == 0 {
        line("  none".to_string());
    }

    if !report.notes.is_empty() {
        line(String::new());
        line("Notes:".to_string());
        for note in &report.notes {
            line(format!("  {note}"));
        }
    }
    out
}

/// A value of a difference: a list or an object by its size, since a whole corpus is no use in a line.
fn brief(value: Option<&Value>) -> String {
    match value {
        None => "no such key".to_string(),
        Some(Value::Array(items)) => format!("a list of {}", items.len()),
        Some(Value::Object(keys)) => format!("an object of {} keys", keys.len()),
        Some(scalar) => scalar.to_string(),
    }
}

/// `compare`'s answer as a person reads it. The first line's `equal` is what P28's script and P32 read.
pub fn render_comparison(comparison: &Comparison) -> String {
    match &comparison.difference {
        None => format!(
            "Corpus of {}: equal, {} swing(s) and {} excluded on each side.\n",
            comparison.player, comparison.swings, comparison.excluded
        ),
        Some(difference) => format!(
            "Corpus of {}: DIFFERENT, first at {}: {} in the export, {} in the other. {} swing(s) and {} \
             excluded in the export, {} and {} in the other.\n",
            comparison.player,
            difference.path,
            brief(difference.in_export.as_ref()),
            brief(difference.in_other.as_ref()),
            comparison.swings,
            comparison.excluded,
            comparison.other_swings,
            comparison.other_excluded
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn path_of(left: Value, right: Value) -> Option<String> {
        first_difference(&left, &right, "").map(|difference| difference.path)
    }

    /// `tests/bay_truth.rs` reaches this through real corpora, where a few of these shapes cannot be made
    /// to happen on demand.
    #[test]
    fn first_difference_spells_a_path_as_the_re_records_comparator_does() {
        assert_eq!(path_of(json!({"a": 1}), json!({"a": 1})), None);
        assert_eq!(path_of(json!(1), json!(2)), Some(String::new()));
        assert_eq!(
            path_of(
                json!({"a": {"b": [1, {"c": 2}]}}),
                json!({"a": {"b": [1, {"c": 3}]}})
            ),
            Some("a.b[1].c".to_string())
        );
    }

    #[test]
    fn first_difference_takes_keys_in_sorted_order_and_stops_at_the_first() {
        assert_eq!(
            path_of(
                json!({"z": 1, "b": 1, "a": 1}),
                json!({"z": 2, "b": 2, "a": 1})
            ),
            Some("b".to_string())
        );
    }

    #[test]
    fn first_difference_names_a_key_only_one_side_has_and_says_which_side() {
        let only_left = first_difference(&json!({"a": 1}), &json!({}), "").expect("a difference");
        assert_eq!(only_left.path, "a");
        assert_eq!(
            (only_left.in_export, only_left.in_other),
            (Some(json!(1)), None)
        );
        let only_right = first_difference(&json!({}), &json!({"a": 1}), "").expect("a difference");
        assert_eq!(
            (only_right.in_export, only_right.in_other),
            (None, Some(json!(1)))
        );
    }

    #[test]
    fn first_difference_parts_lists_of_two_lengths_at_the_list() {
        let difference =
            first_difference(&json!({"a": [1, 2, 3]}), &json!({"a": [2, 3]}), "").expect("one");
        assert_eq!(difference.path, "a");
        assert_eq!(
            brief(difference.in_export.as_ref()),
            "a list of 3",
            "a whole corpus is no use in a line"
        );
        assert_eq!(brief(difference.in_other.as_ref()), "a list of 2");
        assert_eq!(brief(None), "no such key");
        assert_eq!(brief(Some(&json!({"a": 1, "b": 2}))), "an object of 2 keys");
        assert_eq!(brief(Some(&json!(1.5))), "1.5");
    }

    /// **Why this is not `golf_core::compare`** (the module doc): inside its tolerance, this difference
    /// passes there. One reader reading one set of bytes twice has no such excuse.
    #[test]
    fn first_difference_holds_a_float_to_the_bit() {
        let (one, nearly) = (1.0, 1.0 + 1e-15);
        assert_ne!(one, nearly);
        assert_eq!(
            path_of(json!({"a": one}), json!({"a": nearly})),
            Some("a".to_string())
        );
        assert_eq!(path_of(json!({"a": -0.0}), json!({"a": 0.0})), None, "IEEE");
    }

    /// The columns are typed here apart from the device, and a device adding a thirteenth is refused.
    #[test]
    fn the_truth_tables_columns_are_the_twelve_the_device_declares() {
        let row: TruthRow = serde_json::from_value(json!({
            "carry": 1, "total": 2, "club": 3, "ball": 4, "path": 5, "face": 6,
            "ball_dir": [7, "R"], "launch": 8, "apex_ft": 9, "smash": 10, "backspin": 11,
            "sidespin": [12, "F"],
        }))
        .expect("a row");
        let fields = screen_fields().expect("the device");
        assert_eq!(fields.len(), 12);
        let values = row.in_stored_conventions("a", &fields).expect("twelve");
        let of = |field: &str| {
            let at = fields.iter().position(|held| held == field).expect(field);
            values[at]
        };
        assert_eq!(
            [
                of("carry_distance"),
                of("total_distance"),
                of("club_head_speed"),
                of("ball_speed"),
                of("club_path"),
                of("club_face_angle"),
                of("launch_direction"),
                of("launch_angle"),
                of("apex_height"),
                of("smash_factor"),
                of("backspin"),
                of("sidespin"),
            ],
            [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 3.0, 10.0, 11.0, 12.0]
        );

        let thirteen = vec!["impact_offset_h".to_string()];
        let said = row
            .in_stored_conventions("a", &thirteen)
            .expect_err("refused");
        assert_eq!(
            said,
            "the device declares impact_offset_h, and the truth table has no such column"
        );
    }
}
