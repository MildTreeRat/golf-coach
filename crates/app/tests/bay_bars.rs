//! `golf-bay-report bars`: the bay session's arithmetic, proven before there is a session. [M38 P14]
//!
//! **The ten photos are worked by hand here, in [`ten_photos`]'s comment**, and every number the first
//! tests pin comes from that comment and not from running this. The stores and the log are written by
//! the real writers (`open_photo_entry`, `ShotStore::put`, `log_read`), so what is read is the layout
//! the phone writes, and only the numbers in it are chosen.
//!
//! **The limits are the committed file's**, read at run time. A test holding a limit against a number
//! edits a scratch copy of that text, which is what proves the report has no second copy.
//!
//! **Everything read off the six captures here is provisional: pasted copies** (rule 12), and those
//! files are read and never written (rule 5).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use app::api::export::export_sessions;
use app::api::record::{
    log_read, record_photo, PhotoIn, ReadLog, ReadSource, RecordOutcome, RecordStatus, RowState,
    ShotCard,
};
use app::api::review::{correct_shot, FieldEdit};
use app::api::setup::{create_golfer, get_state, init_state, set_state, Handedness};
use app::bay::{
    bars_report, classify, nearest_rank, render_bars, BarResult, Bars, BarsReport, CellKind,
    SourceChoice, Verdict, BARS_FILE,
};
use app::{club_id, parse_now, Dirs};
use contracts::shot::ShotData;
use golf_core::session::{open_photo_entry, NewPhotoEntry};
use serde_json::{json, Map, Value};
use storage::bundle_store::SwingBundleStore;
use storage::shot_store::{hash_image, ShotStore};
use tempfile::TempDir;

const SESSION: &str = "2026-11-05-phone-a1b2";
const AT: &str = "2026-11-05T10:00:00-05:00";
const CAMERA: SourceChoice = SourceChoice::Only(ReadSource::Camera);

/// The twelve, in `devices.json`'s order, and what a clean photo holds in each.
const TWELVE: [(&str, f64); 12] = [
    ("total_distance", 195.0),
    ("carry_distance", 184.0),
    ("club_head_speed", 90.7),
    ("ball_speed", 125.4),
    ("club_path", -2.0),
    ("club_face_angle", 1.5),
    ("launch_direction", -0.8),
    ("launch_angle", 19.0),
    ("apex_height", 28.4),
    ("smash_factor", 1.38),
    ("backspin", 6200.0),
    ("sidespin", -310.0),
];

/// What the reader and the person made of one cell, where it is not simply read.
#[derive(Clone, Copy)]
enum Edit {
    /// The reader gave this, and the person typed the clean value over it.
    Wrong(f64),
    /// The reader gave nothing, and the person typed the clean value.
    Withheld,
    /// The reader gave nothing, and the screen left it blank.
    Blank,
    /// The reader gave this, and the person cleared it, as the screen left it blank.
    Cleared(f64),
}

/// One photo of a fixture.
#[derive(Clone)]
struct Photo {
    status: RecordStatus,
    source: ReadSource,
    ms_total: i64,
    club: &'static str,
    edits: Vec<(&'static str, Edit)>,
    /// `false` is no stored shot and no entry: a failed read, or one deleted at the phone.
    stored: bool,
    /// When it was logged.
    at: &'static str,
}

fn clean(club: &'static str, ms_total: i64) -> Photo {
    Photo {
        status: RecordStatus::Read,
        source: ReadSource::Camera,
        ms_total,
        club,
        edits: Vec::new(),
        stored: true,
        at: AT,
    }
}

fn failed(ms_total: i64) -> Photo {
    Photo {
        status: RecordStatus::Failed,
        stored: false,
        ..clean("7i", ms_total)
    }
}

impl Photo {
    fn with(mut self, field: &'static str, edit: Edit) -> Self {
        self.edits.push((field, edit));
        self
    }

    fn flagged(mut self) -> Self {
        self.status = RecordStatus::Flagged;
        self
    }

    fn from(mut self, source: ReadSource) -> Self {
        self.source = source;
        self
    }

    fn deleted(mut self) -> Self {
        self.stored = false;
        self
    }

    fn logged_at(mut self, at: &'static str) -> Self {
        self.at = at;
        self
    }
}

/// A photo's bytes, unique to its place in the fixture.
fn digest_of(serial: usize) -> String {
    hash_image(format!("photo {serial}").as_bytes())
}

/// A scratch export holding these photos, written as the phone writes them.
struct Export {
    _dir: TempDir,
    root: PathBuf,
}

impl Export {
    fn of(photos: &[Photo]) -> Self {
        let dir = tempfile::tempdir().expect("a scratch directory");
        let root = dir.path().join("processed");
        let export = Export { _dir: dir, root };
        for (serial, photo) in photos.iter().enumerate() {
            export.write(serial, photo);
        }
        export
    }

    fn root_text(&self) -> String {
        self.root.display().to_string()
    }

    fn dirs(&self) -> Dirs {
        Dirs::of(&self.root)
    }

    fn write(&self, serial: usize, photo: &Photo) {
        let digest = digest_of(serial);
        if photo.stored {
            self.store(serial, photo, &digest);
        }
        self.line_at(
            photo.at,
            &digest,
            photo.status,
            photo.source,
            photo.ms_total,
        );
    }

    fn store(&self, serial: usize, photo: &Photo, digest: &str) {
        let dirs = self.dirs();
        let bytes = format!("photo {serial}");
        let opened = open_photo_entry(
            &SwingBundleStore::new(&dirs.sessions),
            &NewPhotoEntry {
                session_id: SESSION,
                digest,
                bytes: bytes.as_bytes(),
                original_filename: "IMG_0001.jpg",
                modified: SystemTime::UNIX_EPOCH,
                player_id: "aaron",
                club: club_id(photo.club).expect("a club"),
                intent: None,
                floor: 0,
            },
            parse_now(AT).expect("a time"),
        )
        .expect("an entry");

        let mut shot = Map::new();
        let mut corrections = Map::new();
        for (field, value) in TWELVE {
            let edit = photo
                .edits
                .iter()
                .find(|(edited, _)| *edited == field)
                .map(|&(_, edit)| edit);
            let (stored, entry) = match edit {
                None => (Some(value), None),
                Some(Edit::Wrong(read)) => (Some(value), Some(pyfmt::repr(read))),
                Some(Edit::Withheld) => (Some(value), Some(String::new())),
                Some(Edit::Blank) => (None, None),
                Some(Edit::Cleared(read)) => (None, Some(pyfmt::repr(read))),
            };
            if let Some(stored) = stored {
                shot.insert(field.to_string(), json!(stored));
            }
            if let Some(entry) = entry {
                corrections.insert(field.to_string(), json!(entry));
            }
        }
        shot.insert(
            "shot_id".into(),
            json!(format!("{SESSION}-{}", opened.swing_id)),
        );
        shot.insert("session_id".into(), json!(SESSION));
        shot.insert("timestamp".into(), json!(AT));
        shot.insert("source".into(), json!("screen"));
        shot.insert(
            "provenance".into(),
            json!({
                "device": "full_swing",
                "parse_confidence": 1.0,
                "image_sha256": digest,
                "corrections": corrections,
            }),
        );
        let shot: ShotData = serde_json::from_value(Value::Object(shot)).expect("a shot");
        ShotStore::new(&dirs.shots)
            .put(&shot)
            .expect("a stored shot");
    }

    fn line(&self, digest: &str, status: RecordStatus, source: ReadSource, ms_total: i64) {
        self.line_at(AT, digest, status, source, ms_total);
    }

    fn line_at(
        &self,
        at: &str,
        digest: &str,
        status: RecordStatus,
        source: ReadSource,
        ms_total: i64,
    ) {
        log_read(
            self.root_text(),
            ReadLog {
                at: at.to_string(),
                photo_sha256: digest.to_string(),
                source,
                status,
                boxes: 40,
                ms_vision: ms_total / 2,
                ms_rust: 50,
                ms_total,
            },
        )
        .expect("a line");
    }

    fn write_boxes(&self, digest: &str, text: &str) {
        let boxes = self.dirs().app.join("boxes");
        fs::create_dir_all(&boxes).expect("boxes");
        fs::write(boxes.join(format!("{digest}.json")), text).expect("a box file");
    }

    fn report(&self) -> BarsReport {
        self.report_against(&committed())
    }

    fn report_against(&self, bars: &str) -> BarsReport {
        bars_report(&self.root, &Bars::parse(bars).expect("bars"), CAMERA, None).expect("a report")
    }
}

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The committed bars. Rule 5 keeps the file as it is, so this reads it and writes nothing.
fn committed() -> String {
    fs::read_to_string(repo().join(BARS_FILE)).expect("the bars")
}

/// A scratch copy of the bars with one thing changed, refusing a change that would change nothing.
fn edited(from: &str, to: &str) -> String {
    let text = committed();
    assert!(text.contains(from), "the bars do not say {from:?}");
    text.replace(from, to)
}

fn bar<'a>(report: &'a BarsReport, id: &str) -> &'a BarResult {
    report.bars.iter().find(|bar| bar.id == id).expect("a bar")
}

fn missed_bars(report: &BarsReport) -> Vec<&str> {
    report
        .bars
        .iter()
        .filter(|bar| bar.met != Some(true))
        .map(|bar| bar.id.as_str())
        .collect()
}

/// **The hand-worked fixture.** Ten camera photos, twelve cells each, so 120 cells.
///
/// ```text
/// photo  club  status   ms    its twelve cells
/// P1     7i    read     1200  12 read
/// P2     7i    read     1400  12 read
/// P3     7i    read     1600  12 read
/// P4     7i    read     1800  12 read
/// P5     7i    read     2000  12 read
/// P6     pw    read     2200  12 read
/// P7     pw    read     2400  10 read, launch_direction withheld, sidespin blank
/// P8     pw    flagged  2600  11 read, carry_distance wrong (read 148, truth 184)
/// P9     none  failed   3000  12 withheld, counted without a value
/// P10    pw    read     4200  12 read
/// ```
///
/// Read: P1 to P6 and P10 are 7 x 12 = 84, plus P7's 10 and P8's 11, is **105**. Wrong: **1**. Withheld:
/// P7's 1 and P9's 12 is **13**. Blank: **1**. 105 + 1 + 13 + 1 = 120, every cell.
///
/// - **T1 is 0**: the one wrong cell is on P8, which is flagged. PASS at most 0.
/// - **T2 is 105 of 119, 88.2%**: 119 = 120 less the one blank. MISS at least 92%.
/// - **T3 is 8 of 10, 80%, `carry_distance`**: P9 takes one from every field, so a field holds 9 of 10 at
///   best. `carry_distance` loses P8's as well, 8 of 10. `launch_direction` loses P7's, also 8 of 10,
///   and a tie goes to the first in tile order. `sidespin` is 8 of 9, 88.9%, since P7's blank leaves
///   its denominator. MISS at least 83%.
/// - **T4 is 2 of 10, 20%**: P8 and P9. MISS at most 17%.
/// - **T5 is 4200 ms**: nearest rank is ceil(10 x 0.95) = ceil(9.5) = 10, the largest. PASS at most 5000.
/// - **INVALID**: ten is not at least 30, whatever the bars say.
///
/// Per club: 7i is P1 to P5, 5 photos, 60 read. pw is P6, P7, P8 and P10, 4 photos, 1 flagged, 12 + 10 +
/// 11 + 12 = 45 read, 1 wrong, 1 withheld, 1 blank. No club is P9, 1 photo, 1 flagged, 12 withheld.
fn ten_photos() -> Vec<Photo> {
    vec![
        clean("7i", 1200),
        clean("7i", 1400),
        clean("7i", 1600),
        clean("7i", 1800),
        clean("7i", 2000),
        clean("pw", 2200),
        clean("pw", 2400)
            .with("launch_direction", Edit::Withheld)
            .with("sidespin", Edit::Blank),
        clean("pw", 2600)
            .with("carry_distance", Edit::Wrong(148.0))
            .flagged(),
        failed(3000),
        clean("pw", 4200),
    ]
}

#[test]
fn the_ten_photos_come_to_the_hand_worked_cells_and_photos() {
    let report = Export::of(&ten_photos()).report();
    let cells = &report.cells;
    assert_eq!(
        (cells.read, cells.wrong, cells.withheld, cells.blank),
        (105, 1, 13, 1)
    );
    let photos = &report.photos;
    assert_eq!(
        (photos.counted, photos.read, photos.flagged, photos.failed),
        (10, 8, 1, 1)
    );
    assert_eq!(report.fields.len(), TWELVE.len());
    for (row, (field, _)) in report.fields.iter().zip(TWELVE) {
        assert_eq!(row.field, field, "the device's tile order");
    }
}

#[test]
fn the_ten_photos_come_to_each_bars_hand_worked_number() {
    let report = Export::of(&ten_photos()).report();

    let t1 = bar(&report, "T1");
    assert_eq!((t1.number, t1.met), (Some(0.0), Some(true)));

    let t2 = bar(&report, "T2");
    assert_eq!(t2.of, Some((105, 119)));
    assert_eq!(t2.number, Some(10500.0 / 119.0));
    assert_eq!(t2.met, Some(false));

    let t3 = bar(&report, "T3");
    assert_eq!(t3.of, Some((8, 10)));
    assert_eq!(t3.number, Some(80.0));
    assert_eq!(t3.field.as_deref(), Some("carry_distance"));
    assert_eq!(t3.met, Some(false));

    let t4 = bar(&report, "T4");
    assert_eq!(t4.of, Some((2, 10)));
    assert_eq!(t4.number, Some(20.0));
    assert_eq!(t4.met, Some(false));

    let t5 = bar(&report, "T5");
    assert_eq!(t5.number, Some(4200.0));
    assert_eq!(t5.of, Some((10, 10)));
    assert_eq!(t5.met, Some(true));

    assert_eq!(report.verdict, Verdict::Invalid);
}

#[test]
fn the_ten_photos_come_to_the_hand_worked_clubs_and_missed_cells() {
    let report = Export::of(&ten_photos()).report();
    let club = |name: Option<&str>| {
        let row = report
            .clubs
            .iter()
            .find(|row| row.club.as_deref() == name)
            .expect("a club's row");
        (
            row.photos,
            row.flagged,
            row.cells.read,
            row.cells.wrong,
            row.cells.withheld,
            row.cells.blank,
        )
    };
    assert_eq!(club(Some("7i")), (5, 0, 60, 0, 0, 0));
    assert_eq!(club(Some("pw")), (4, 1, 45, 1, 1, 1));
    assert_eq!(club(None), (1, 1, 0, 0, 12, 0));
    assert_eq!(report.session.clubs, ["7i", "pw"]);

    // Every wrong and withheld cell, by photo and field: P7's, P8's, and P9's twelve.
    assert_eq!(report.missed.len(), 14);
    let p7 = &report.missed[0];
    assert_eq!(p7.photo, format!("{SESSION}/7 IMG_0001.jpg"));
    assert_eq!(p7.field, "launch_direction");
    assert_eq!(
        (p7.kind, p7.as_read, p7.truth, p7.flagged),
        (CellKind::Withheld, None, Some(-0.8), false)
    );
    let p8 = &report.missed[1];
    assert_eq!(p8.photo, format!("{SESSION}/8 IMG_0001.jpg"));
    assert_eq!(
        (p8.kind, p8.as_read, p8.truth, p8.flagged),
        (CellKind::Wrong, Some(148.0), Some(184.0), true)
    );
    let p9 = &report.missed[2..];
    assert!(p9.iter().all(|cell| cell.kind == CellKind::Withheld
        && cell.as_read.is_none()
        && cell.truth.is_none()
        && cell.flagged
        && cell.photo.starts_with("photo ")));
    let named: Vec<&str> = p9.iter().map(|cell| cell.field.as_str()).collect();
    assert_eq!(named, TWELVE.map(|(field, _)| field));
}

/// Each line a person reads for the numbers above, whole, so a column cannot quietly show another one.
#[test]
fn the_ten_photos_print_each_number_a_person_reads() {
    let text = render_bars(&Export::of(&ten_photos()).report());
    for line in [
        "Session: INVALID. 10 camera photos, at least 30 asked. 2 clubs, at least 2 asked: 7i, pw.",
        "VERDICT: INVALID. Not a session (10 camera photos of at least 30, 2 clubs of at least 2), so \
         nothing above is judged.",
        "Photos: 10 counted: 8 read, 1 flagged, 1 failed. 0 deleted at the phone.",
        "  success 9 of 10 (90.0%), review 2 of 10 (20.0%)",
        "Cells: 105 read, 1 wrong, 13 withheld, 1 blank.",
        "wrong, as read 148.0, truth 184.0, flagged",
        "launch_direction: withheld, as read not read, truth -0.8",
        "Latency over 10 photos: ms_total p95 4200, median 2000, max 4200.",
    ] {
        assert!(text.contains(line), "{line:?} is not in:\n{text}");
    }
    let line_of = |id: &str| {
        text.lines()
            .find(|line| line.trim_start().starts_with(id))
            .unwrap_or_else(|| panic!("no {id} in:\n{text}"))
    };
    for (id, number, limit, judged) in [
        ("T1 ", "0", "at most 0", "PASS"),
        ("T2 ", "88.2% (105 of 119)", "at least 92%", "MISS"),
        (
            "T3 ",
            "80.0% (8 of 10, carry_distance)",
            "at least 83%",
            "MISS",
        ),
        ("T4 ", "20.0% (2 of 10)", "at most 17%", "MISS"),
        ("T5 ", "4200 ms (rank 10 of 10)", "at most 5000 ms", "PASS"),
    ] {
        let line = line_of(id);
        assert!(line.contains(number), "{number} is not in {line}");
        assert!(line.contains(limit), "{limit} is not in {line}");
        assert!(
            line.trim_end().ends_with(judged),
            "{judged} does not end {line}"
        );
    }
}

// --- both sides of each limit --------------------------------------------------------------------

/// Ten clean photos on two clubs, at one speed, for a test to spoil one way.
fn ten_clean() -> Vec<Photo> {
    (0..10)
        .map(|at| clean(if at < 5 { "7i" } else { "pw" }, 1000))
        .collect()
}

#[test]
fn t1_holds_with_no_wrong_cell_and_misses_on_one_on_an_unflagged_photo() {
    let mut photos = ten_clean();
    photos[3] = clean("7i", 1000).with("ball_speed", Edit::Wrong(152.4));
    let report = Export::of(&photos).report();
    let t1 = bar(&report, "T1");
    assert_eq!((t1.number, t1.met), (Some(1.0), Some(false)));

    let report = Export::of(&ten_clean()).report();
    let t1 = bar(&report, "T1");
    assert_eq!((t1.number, t1.met), (Some(0.0), Some(true)));
}

#[test]
fn a_wrong_cell_on_a_flagged_photo_does_not_count_against_t1() {
    let mut photos = ten_clean();
    photos[3] = clean("7i", 1000)
        .with("ball_speed", Edit::Wrong(152.4))
        .flagged();
    let report = Export::of(&photos).report();
    assert_eq!(report.cells.wrong, 1, "it is still a wrong cell");
    let t1 = bar(&report, "T1");
    assert_eq!((t1.number, t1.met), (Some(0.0), Some(true)));
}

/// What a person clearing a value says: the reader saw a number where the screen left it blank. On an
/// unflagged photo that is the failure T1 looks for.
#[test]
fn a_cleared_value_is_a_wrong_cell_and_counts_against_t1() {
    let mut photos = ten_clean();
    photos[0] = clean("7i", 1000).with("backspin", Edit::Cleared(6200.0));
    let report = Export::of(&photos).report();
    assert_eq!(report.cells.wrong, 1);
    assert_eq!(bar(&report, "T1").met, Some(false));
    assert_eq!(report.missed[0].as_read, Some(6200.0));
    assert_eq!(report.missed[0].truth, None);
}

/// Ten photos, `backspin` and `sidespin` blank on all ten, so **exactly 100 cells are judged** and 92%
/// is reachable as 92 of 100. The first `withheld` photos each lose one other cell.
fn hundred_cells(withheld: usize) -> Vec<Photo> {
    (0..10)
        .map(|at| {
            let photo = clean(if at < 5 { "7i" } else { "pw" }, 1000)
                .with("backspin", Edit::Blank)
                .with("sidespin", Edit::Blank);
            if at < withheld {
                photo.with(TWELVE[at].0, Edit::Withheld)
            } else {
                photo
            }
        })
        .collect()
}

#[test]
fn t2_holds_at_exactly_its_limit_and_misses_one_cell_below_it() {
    let report = Export::of(&hundred_cells(8)).report();
    let t2 = bar(&report, "T2");
    assert_eq!((t2.of, t2.met), (Some((92, 100)), Some(true)));

    let report = Export::of(&hundred_cells(9)).report();
    let t2 = bar(&report, "T2");
    assert_eq!((t2.of, t2.met), (Some((91, 100)), Some(false)));
}

#[test]
fn a_blank_is_in_no_denominator() {
    let report = Export::of(&hundred_cells(0)).report();
    assert_eq!(report.cells.blank, 20);
    assert_eq!(bar(&report, "T2").of, Some((100, 100)));
    // A field blank on every photo has no share to be lowest with.
    let t3 = bar(&report, "T3");
    assert_eq!(t3.of, Some((10, 10)));
    assert_eq!(t3.field.as_deref(), Some("total_distance"));
}

#[test]
fn t3_is_the_lowest_field_and_holds_on_five_of_six_and_misses_on_four_of_five() {
    // `launch_angle` blank on four, withheld on one: 5 of 6, 83.3%, is M33's own lowest.
    let mut photos = ten_clean();
    for photo in &mut photos[..4] {
        photo.edits.push(("launch_angle", Edit::Blank));
    }
    photos[4].edits.push(("launch_angle", Edit::Withheld));
    let report = Export::of(&photos).report();
    let t3 = bar(&report, "T3");
    assert_eq!(t3.field.as_deref(), Some("launch_angle"));
    assert_eq!((t3.of, t3.met), (Some((5, 6)), Some(true)));

    // Blank on five, withheld on one: 4 of 5, 80%.
    let mut photos = ten_clean();
    for photo in &mut photos[..5] {
        photo.edits.push(("launch_angle", Edit::Blank));
    }
    photos[5].edits.push(("launch_angle", Edit::Withheld));
    let report = Export::of(&photos).report();
    let t3 = bar(&report, "T3");
    assert_eq!((t3.of, t3.met), (Some((4, 5)), Some(false)));
}

#[test]
fn t4_holds_on_one_flagged_of_ten_and_misses_on_two() {
    let mut photos = ten_clean();
    photos[0] = photos[0].clone().flagged();
    let report = Export::of(&photos).report();
    let t4 = bar(&report, "T4");
    assert_eq!((t4.of, t4.met), (Some((1, 10)), Some(true)));

    photos[1] = failed(1000);
    let report = Export::of(&photos).report();
    let t4 = bar(&report, "T4");
    assert_eq!(
        (t4.of, t4.met),
        (Some((2, 10)), Some(false)),
        "a failed read is flagged"
    );
}

#[test]
fn t5_holds_at_exactly_its_limit_and_misses_a_millisecond_over() {
    let mut photos = ten_clean();
    photos[9].ms_total = 5000;
    let t5 = Export::of(&photos).report();
    let t5 = bar(&t5, "T5");
    assert_eq!((t5.number, t5.met), (Some(5000.0), Some(true)));

    photos[9].ms_total = 5001;
    let t5 = Export::of(&photos).report();
    let t5 = bar(&t5, "T5");
    assert_eq!((t5.number, t5.met), (Some(5001.0), Some(false)));
}

/// Twenty photos: nearest rank is ceil(19.0) = 19, so the largest is not the p95, and the 19th is the
/// number itself and not a blend of two, which is what interpolating would give.
#[test]
fn the_p95_is_nearest_rank_and_not_the_largest_nor_an_interpolation() {
    let photos: Vec<Photo> = (0..20)
        .map(|at| match at {
            19 => clean("7i", 90_000),
            18 => clean("7i", 4000),
            _ => clean("7i", 1000),
        })
        .collect();
    let report = Export::of(&photos).report();
    let t5 = bar(&report, "T5");
    assert_eq!(t5.number, Some(4000.0));
    assert_eq!(t5.of, Some((19, 20)));
    assert_eq!(t5.met, Some(true));
}

#[test]
fn nearest_rank_is_the_ceiling_in_integers() {
    for (photos, rank) in [
        (1, 1),
        (2, 2),
        (10, 10),
        (19, 19),
        (20, 19),
        (21, 20),
        (30, 29),
        (40, 38),
        (100, 95),
        (101, 96),
    ] {
        assert_eq!(nearest_rank(photos, 95), rank, "{photos} photos");
    }
    assert_eq!(nearest_rank(10, 50), 5);
    assert_eq!(
        nearest_rank(0, 95),
        1,
        "no photos is asked by nobody, and is no panic here"
    );
}

#[test]
fn each_cell_is_the_files_definition() {
    assert_eq!(classify(Some(184.0), Some(184.0)), CellKind::Read);
    assert_eq!(classify(Some(148.0), Some(184.0)), CellKind::Wrong);
    assert_eq!(classify(Some(148.0), None), CellKind::Wrong);
    assert_eq!(classify(None, Some(184.0)), CellKind::Withheld);
    assert_eq!(classify(None, None), CellKind::Blank);
    // No tolerance: the last bit counts, and IEEE's equal is equal.
    assert_eq!(classify(Some(0.1 + 0.2), Some(0.3)), CellKind::Wrong);
    assert_eq!(classify(Some(-0.0), Some(0.0)), CellKind::Read);
}

// --- what is a photo -----------------------------------------------------------------------------

#[test]
fn a_duplicate_is_no_photo_for_any_count() {
    let export = Export::of(&ten_clean());
    // A repeat of a photo held, with a latency that would move T5 and a status T4 would count.
    export.line(
        &digest_of(0),
        RecordStatus::Duplicate,
        ReadSource::Camera,
        90_000,
    );
    // A line with no other, whose digest no entry holds.
    export.line(
        &digest_of(77),
        RecordStatus::Duplicate,
        ReadSource::Camera,
        90_000,
    );
    let report = export.report();
    assert_eq!(report.photos.counted, 10);
    assert_eq!(report.session.camera_photos, 10);
    assert_eq!(report.photos.duplicate_lines, 2);
    assert_eq!(bar(&report, "T5").number, Some(1000.0));
    assert_eq!(report.cells.read, 120);
}

#[test]
fn a_library_photo_is_in_no_count_and_the_report_says_it_is_there() {
    let mut photos = ten_clean();
    photos.push(
        clean("7i", 90_000)
            .from(ReadSource::Library)
            .with("ball_speed", Edit::Wrong(1.0)),
    );
    let report = Export::of(&photos).report();
    assert_eq!(report.photos.counted, 10);
    assert_eq!(report.session.camera_photos, 10);
    assert_eq!(report.cells.wrong, 0);
    assert_eq!(bar(&report, "T1").met, Some(true));
    assert_eq!(bar(&report, "T5").number, Some(1000.0));
    assert_eq!(report.photos.by_source["camera"], 10);
    assert_eq!(report.photos.by_source["library"], 1);
}

/// P30 reads latency and the letters over everything, and P34 splits by source. Neither may earn a
/// verdict: a session is camera photos, however many there are.
#[test]
fn another_source_can_be_read_and_never_earns_a_verdict() {
    let mut photos: Vec<Photo> = (0..30)
        .map(|at| clean(if at < 15 { "7i" } else { "pw" }, 1000))
        .collect();
    photos.push(clean("7i", 2000).from(ReadSource::Library));
    photos.push(clean("7i", 3000).from(ReadSource::Scanner));
    let export = Export::of(&photos);
    let bars = Bars::parse(&committed()).expect("bars");

    let read = |source| bars_report(&export.root, &bars, source, None).expect("a report");
    assert_eq!(read(CAMERA).verdict, Verdict::Pass);
    assert_eq!(read(CAMERA).photos.counted, 30);

    let library = read(SourceChoice::Only(ReadSource::Library));
    assert_eq!(library.photos.counted, 1);
    assert_eq!(bar(&library, "T5").number, Some(2000.0));
    assert_eq!(library.verdict, Verdict::Invalid);
    assert!(render_bars(&library).contains("These are library photos"));

    let scanner = read(SourceChoice::Only(ReadSource::Scanner));
    assert_eq!(scanner.photos.counted, 1);
    assert_eq!(bar(&scanner, "T5").number, Some(3000.0));

    let all = read(SourceChoice::All);
    assert_eq!(all.photos.counted, 32);
    assert_eq!(all.verdict, Verdict::Invalid);
}

#[test]
fn a_photo_deleted_at_the_phone_counts_where_only_the_log_is_asked_and_holds_no_cells() {
    let mut photos = ten_clean();
    photos[0] = clean("7i", 4000).flagged().deleted();
    let report = Export::of(&photos).report();
    assert_eq!(
        bar(&report, "T5").number,
        Some(4000.0),
        "its latency was measured"
    );
    assert_eq!(report.photos.counted, 10);
    assert_eq!(report.photos.deleted, 1);
    assert_eq!(report.session.camera_photos, 10);
    assert_eq!(
        bar(&report, "T4").of,
        Some((1, 10)),
        "deleting a flagged photo keeps its flag"
    );
    assert_eq!(report.cells.read, 108);
    assert!(
        report
            .notes
            .iter()
            .any(|note| note.contains("deleted at the phone")),
        "{:?}",
        report.notes
    );
}

/// P10 finding 8: a photo recorded afresh after a delete appends a second line for one digest. It is one
/// photo, and the last line is the read that produced what is stored.
#[test]
fn a_digest_with_two_lines_is_one_photo_and_the_last_speaks_for_it() {
    let export = Export::of(&ten_clean());
    export.line(
        &digest_of(0),
        RecordStatus::Flagged,
        ReadSource::Camera,
        4000,
    );
    let report = export.report();
    assert_eq!(report.photos.counted, 10);
    assert_eq!(bar(&report, "T4").of, Some((1, 10)));
    assert_eq!(bar(&report, "T5").number, Some(4000.0));
    assert_eq!(report.cells.read, 120, "its shot's cells, once");
}

#[test]
fn a_failed_read_retaken_with_the_same_bytes_is_the_retakes_read() {
    let retaken = Export::of(&[]);
    retaken.line(&digest_of(0), RecordStatus::Failed, ReadSource::Camera, 900);
    retaken.write(0, &clean("7i", 1000));
    let report = retaken.report();
    assert_eq!(report.photos.counted, 1);
    assert_eq!(report.photos.failed, 0);
    assert_eq!(report.cells.read, 12);
    assert_eq!(report.cells.withheld, 0);
}

#[test]
fn a_stored_shot_with_no_read_line_is_in_no_count_and_named() {
    let export = Export::of(&ten_clean());
    export.store(50, &clean("7i", 1000), &digest_of(50));
    let report = export.report();
    assert_eq!(report.photos.counted, 10);
    assert_eq!(report.cells.read, 120);
    assert!(
        report
            .notes
            .iter()
            .any(|note| note.contains("no read line") && note.contains(&format!("{SESSION}/11"))),
        "{:?}",
        report.notes
    );
}

// --- which sitting -------------------------------------------------------------------------------

/// Thirty clean photos at the bay, and before them what P30's dry run leaves on the phone: six in-app
/// camera photos of a monitor, slow, one of them with a wrong cell and unflagged. A cumulative export
/// carries them, and the log calls them `camera`, so only the time tells them apart.
fn rehearsal_then_bay() -> Vec<Photo> {
    let mut photos: Vec<Photo> = (0..6)
        .map(|_| clean("3i", 90_000).logged_at("2026-11-01T09:00:00-05:00"))
        .collect();
    photos[0].edits.push(("ball_speed", Edit::Wrong(1.0)));
    photos.extend(thirty());
    photos
}

#[test]
fn a_rehearsal_left_in_counts_towards_every_bar_and_shows_in_when_the_reads_ran() {
    let report = Export::of(&rehearsal_then_bay()).report();
    assert_eq!(report.session.camera_photos, 36);
    assert_eq!(report.session.clubs, ["3i", "7i", "pw"]);
    assert_eq!(
        report.verdict,
        Verdict::Miss,
        "and this is the wrong verdict for the session"
    );
    assert_eq!(
        report.photos.first_read.as_deref(),
        Some("2026-11-01T09:00:00-05:00")
    );
    assert_eq!(report.photos.last_read.as_deref(), Some(AT));
    let text = render_bars(&report);
    assert!(
        text.contains("read from 2026-11-01T09:00:00-05:00 to 2026-11-05T10:00:00-05:00"),
        "{text}"
    );
}

#[test]
fn since_leaves_out_what_came_before_and_the_session_is_judged_alone() {
    let export = Export::of(&rehearsal_then_bay());
    let bars = Bars::parse(&committed()).expect("bars");
    let since = parse_now("2026-11-05T00:00:00-05:00").expect("a time");
    let report = bars_report(&export.root, &bars, CAMERA, Some(since)).expect("a report");
    assert_eq!(report.session.camera_photos, 30);
    assert_eq!(report.session.clubs, ["7i", "pw"]);
    assert_eq!(report.cells.wrong, 0);
    assert_eq!(bar(&report, "T5").number, Some(1500.0));
    assert_eq!(report.verdict, Verdict::Pass);
    assert_eq!(report.photos.first_read.as_deref(), Some(AT));
    assert_eq!(report.since.as_deref(), Some("2026-11-05T00:00:00-05:00"));
    assert!(
        report
            .notes
            .iter()
            .any(|note| note.starts_with("6 read line(s) before --since")),
        "{:?}",
        report.notes
    );
    assert!(
        !report
            .notes
            .iter()
            .any(|note| note.contains("no read line")),
        "a shot with its line before --since is logged: {:?}",
        report.notes
    );
    assert!(render_bars(&report)
        .contains("Bars, over camera photos read since 2026-11-05T00:00:00-05:00:"));
}

/// By instant and not by spelling: 10:00 at -05:00 is 15:00Z, and the instant itself is inside.
#[test]
fn since_compares_instants_and_keeps_a_read_at_exactly_that_instant() {
    let export = Export::of(&[clean("7i", 1000)]);
    let bars = Bars::parse(&committed()).expect("bars");
    let counted = |since: &str| {
        let since = parse_now(since).expect("a time");
        bars_report(&export.root, &bars, CAMERA, Some(since))
            .expect("a report")
            .photos
            .counted
    };
    assert_eq!(counted("2026-11-05T15:00:00Z"), 1);
    assert_eq!(counted("2026-11-05T15:00:01Z"), 0);
    assert_eq!(counted("2026-11-05T14:59:59Z"), 1);
    assert_eq!(counted("2026-11-05T12:00:00-03:00"), 1);
}

// --- the verdict ---------------------------------------------------------------------------------

/// A valid session, every bar met: thirty clean photos on two clubs.
fn thirty() -> Vec<Photo> {
    (0..30)
        .map(|at| clean(if at < 15 { "7i" } else { "pw" }, 1500))
        .collect()
}

#[test]
fn every_bar_met_on_a_valid_session_is_pass() {
    let report = Export::of(&thirty()).report();
    assert!(report.session.valid);
    assert_eq!(missed_bars(&report), Vec::<&str>::new());
    assert_eq!(report.verdict, Verdict::Pass);
    assert!(render_bars(&report).contains("VERDICT: PASS."));
}

#[test]
fn any_of_t1_to_t4_missed_is_miss_and_each_is_caught_alone() {
    // T1: one wrong cell on an unflagged photo, and nothing else moves past its limit.
    let mut t1 = thirty();
    t1[0].edits.push(("ball_speed", Edit::Wrong(1.0)));

    // T2: 29 of 360 withheld is 331, 91.9%, spread so no field falls under 83%: 3 on each field at most.
    let mut t2 = thirty();
    for at in 0..29 {
        t2[at].edits.push((TWELVE[at % 12].0, Edit::Withheld));
    }

    // T3: 6 withheld on one field is 24 of 30, 80%, while T2 is 354 of 360, 98.3%.
    let mut t3 = thirty();
    for photo in &mut t3[..6] {
        photo.edits.push(("sidespin", Edit::Withheld));
    }

    // T4: 6 flagged of 30 is 20%.
    let mut t4 = thirty();
    for photo in &mut t4[..6] {
        photo.status = RecordStatus::Flagged;
    }

    for (id, photos) in [("T1", t1), ("T2", t2), ("T3", t3), ("T4", t4)] {
        let report = Export::of(&photos).report();
        assert_eq!(missed_bars(&report), [id], "{id} alone");
        assert_eq!(report.verdict, Verdict::Miss, "{id}");
        assert!(
            render_bars(&report)
                .contains(&format!("VERDICT: MISS. {id} missed, which triggers P33")),
            "{id}"
        );
    }
}

#[test]
fn t2_t3_and_t4_hold_just_inside_their_limits_on_a_valid_session() {
    // T2: 28 of 360 withheld is 332, 92.2%.
    let mut t2 = thirty();
    for at in 0..28 {
        t2[at].edits.push((TWELVE[at % 12].0, Edit::Withheld));
    }
    // T3: 5 of 30 withheld is 25 of 30, 83.3%.
    let mut t3 = thirty();
    for photo in &mut t3[..5] {
        photo.edits.push(("sidespin", Edit::Withheld));
    }
    // T4: 5 of 30 is 16.7%.
    let mut t4 = thirty();
    for photo in &mut t4[..5] {
        photo.status = RecordStatus::Flagged;
    }
    for (id, photos) in [("T2", t2), ("T3", t3), ("T4", t4)] {
        assert_eq!(Export::of(&photos).report().verdict, Verdict::Pass, "{id}");
    }
}

#[test]
fn only_t5_missed_is_miss_latency_and_with_another_it_is_plain_miss() {
    let mut slow = thirty();
    for photo in &mut slow {
        photo.ms_total = 5001;
    }
    let report = Export::of(&slow).report();
    assert_eq!(missed_bars(&report), ["T5"]);
    assert_eq!(report.verdict, Verdict::MissLatency);
    assert!(render_bars(&report).contains("VERDICT: MISS (latency). Only T5 missed"));

    slow[0].edits.push(("ball_speed", Edit::Wrong(1.0)));
    let report = Export::of(&slow).report();
    assert_eq!(missed_bars(&report), ["T1", "T5"]);
    assert_eq!(report.verdict, Verdict::Miss);
    let text = render_bars(&report);
    assert!(
        text.contains("VERDICT: MISS. T1 missed"),
        "T5 is no reason for P33: {text}"
    );
}

#[test]
fn the_session_is_invalid_under_thirty_camera_photos_and_valid_at_thirty() {
    let mut photos = thirty();
    assert!(Export::of(&photos).report().session.valid);

    let last = photos.pop().expect("a photo");
    let report = Export::of(&photos).report();
    assert!(!report.session.valid);
    assert_eq!(report.session.camera_photos, 29);
    assert_eq!(
        report.verdict,
        Verdict::Invalid,
        "every bar met, and still not a session"
    );
    assert_eq!(missed_bars(&report), Vec::<&str>::new());

    // Neither a library photo nor a repeat makes up the thirtieth.
    photos.push(last.from(ReadSource::Library));
    assert_eq!(Export::of(&photos).report().verdict, Verdict::Invalid);
    let export = Export::of(&photos[..29]);
    export.line(
        &digest_of(0),
        RecordStatus::Duplicate,
        ReadSource::Camera,
        1500,
    );
    assert_eq!(export.report().verdict, Verdict::Invalid);
}

#[test]
fn the_session_is_invalid_on_one_club_and_a_failed_read_brings_none() {
    let one_club: Vec<Photo> = (0..30).map(|_| clean("7i", 1500)).collect();
    let report = Export::of(&one_club).report();
    assert!(!report.session.valid);
    assert_eq!(report.session.clubs, ["7i"]);
    assert_eq!(report.verdict, Verdict::Invalid);

    // A failed read counts towards thirty, and has no entry to say which club hit it.
    let mut with_failed = one_club;
    with_failed[29] = failed(1500);
    let report = Export::of(&with_failed).report();
    assert_eq!(report.session.camera_photos, 30);
    assert_eq!(report.session.clubs, ["7i"]);
    assert_eq!(report.verdict, Verdict::Invalid);

    // One photo of a second club is two clubs, as the file words it.
    let mut two = with_failed;
    two[0] = clean("pw", 1500);
    let report = Export::of(&two).report();
    assert_eq!(report.session.clubs, ["7i", "pw"]);
    assert!(report.session.valid);
}

// --- no second copy ------------------------------------------------------------------------------

/// **The proof the Done line asks for.** One export, unchanged, and only the scratch copy of the bars
/// moves. Each limit is also held to both sides of itself exactly, on a number the real limits never
/// land on.
#[test]
fn editing_a_limit_in_a_scratch_copy_changes_the_verdict() {
    let export = Export::of(&thirty());
    assert_eq!(export.report().verdict, Verdict::Pass);
    for (from, to, verdict) in [
        (
            "| at most 5000 ms |",
            "| at most 1499 ms |",
            Verdict::MissLatency,
        ),
        ("| at most 5000 ms |", "| at most 1500 ms |", Verdict::Pass),
        ("| at least 92% |", "| at least 100.5% |", Verdict::Miss),
        ("| at least 83% |", "| at least 100.5% |", Verdict::Miss),
        ("| at most 17% |", "| at least 1% |", Verdict::Miss),
        ("| at most 0 |", "| at least 1 |", Verdict::Miss),
    ] {
        let report = export.report_against(&edited(from, to));
        assert_eq!(report.verdict, verdict, "{from} to {to}");
    }

    // And a session that misses passes once a copy allows it.
    let mut photos = thirty();
    photos[0].edits.push(("ball_speed", Edit::Wrong(1.0)));
    let export = Export::of(&photos);
    assert_eq!(export.report().verdict, Verdict::Miss);
    let allowed = export.report_against(&edited("| at most 0 |", "| at most 1 |"));
    assert_eq!(allowed.verdict, Verdict::Pass);
}

#[test]
fn each_limit_holds_exactly_at_a_scratch_number_and_misses_just_past_it() {
    let mut one_wrong = ten_clean();
    one_wrong[0].edits.push(("ball_speed", Edit::Wrong(1.0)));
    let export = Export::of(&one_wrong);
    assert_eq!(bar(&export.report(), "T1").met, Some(false));
    let report = export.report_against(&edited("| at most 0 |", "| at most 1 |"));
    assert_eq!(bar(&report, "T1").met, Some(true));

    let photos = ten_photos();
    let export = Export::of(&photos);
    for (id, from, holds, misses) in [
        (
            "T3",
            "| at least 83% |",
            "| at least 80% |",
            "| at least 81% |",
        ),
        (
            "T4",
            "| at most 17% |",
            "| at most 20% |",
            "| at most 19% |",
        ),
        (
            "T5",
            "| at most 5000 ms |",
            "| at most 4200 ms |",
            "| at most 4199 ms |",
        ),
    ] {
        let report = export.report_against(&edited(from, holds));
        assert_eq!(bar(&report, id).met, Some(true), "{id} at {holds}");
        let report = export.report_against(&edited(from, misses));
        assert_eq!(bar(&report, id).met, Some(false), "{id} at {misses}");
    }

    // T2 is 105 of 119, which no integer percent lands on, so a decimal limit either side of it.
    let report = export.report_against(&edited("| at least 92% |", "| at least 88.2% |"));
    assert_eq!(bar(&report, "T2").met, Some(true));
    let report = export.report_against(&edited("| at least 92% |", "| at least 88.3% |"));
    assert_eq!(bar(&report, "T2").met, Some(false));

    // A limit's words reach the report as the file wrote them.
    let report = export.report_against(&edited("| at most 17% |", "| at most 20% |"));
    assert_eq!(bar(&report, "T4").limit, "at most 20%");
}

/// The session's size is prose and not the table, and it has no second copy either.
#[test]
fn the_sessions_size_comes_from_the_file_too() {
    let export = Export::of(&ten_photos());
    assert_eq!(export.report().verdict, Verdict::Invalid);
    let report = export.report_against(&edited("at least 30 photos", "at least 10 photos"));
    assert!(report.session.valid);
    assert_eq!(
        report.verdict,
        Verdict::Miss,
        "T2, T3 and T4, judged at last"
    );

    let report = export.report_against(&edited("at least 2 clubs", "at least 3 clubs"));
    assert!(!report.session.valid);
    let report =
        Export::of(&thirty()).report_against(&edited("at least 2 clubs", "at least 3 clubs"));
    assert_eq!(report.verdict, Verdict::Invalid);
}

#[test]
fn the_committed_table_parses_to_what_it_says() {
    let bars = Bars::parse(&committed()).expect("the committed bars");
    assert_eq!((bars.photos, bars.clubs), (30, 2));
    let limits: Vec<(&str, &str, f64)> = bars
        .bars
        .iter()
        .map(|bar| (bar.id.as_str(), bar.limit.as_str(), bar.number))
        .collect();
    assert_eq!(
        limits,
        [
            ("T1", "at most 0", 0.0),
            ("T2", "at least 92%", 92.0),
            ("T3", "at least 83%", 83.0),
            ("T4", "at most 17%", 17.0),
            ("T5", "at most 5000 ms", 5000.0),
        ]
    );
}

#[test]
fn a_bars_file_that_does_not_say_is_refused_rather_than_guessed_at() {
    let t3_row =
        "| T3 | the lowest field's read cells, of its read + wrong + withheld | at least 83% |\n";
    for (text, says) in [
        (edited(t3_row, ""), "no row for T3"),
        (edited("| at most 0 |", "| at most none |"), "T1's limit"),
        (edited("| at least 92% |", "| at least 92 |"), "T2's limit"),
        (edited("| at most 17% |", "| 17% |"), "T4's limit"),
        (
            edited("| at most 5000 ms |", "| at most 5000 |"),
            "T5's limit",
        ),
        (edited("| at most 0 |", "| at most inf |"), "T1's limit"),
        (edited(t3_row, "| T6 | a sixth | at most 1 |\n"), "T6"),
        (edited(t3_row, &format!("{t3_row}{t3_row}")), "T3 twice"),
        (
            edited("at least 30 photos", "30 photos"),
            "at least N photos",
        ),
        (edited("at least 2 clubs", "2 clubs"), "at least N clubs"),
        (edited("## The bars", "## Bars"), "no row for T1"),
    ] {
        let refused = Bars::parse(&text).expect_err(says);
        assert!(refused.contains(says), "{refused:?} does not say {says:?}");
    }
}

// --- what is read, and what is refused -----------------------------------------------------------

#[test]
fn a_torn_line_in_the_log_is_noted_and_costs_nothing_else() {
    let export = Export::of(&ten_clean());
    let log = export.dirs().app.join("reads.jsonl");
    let mut text = fs::read_to_string(&log).expect("the log");
    text.push_str("{\"at\":\"2026-11-05T1");
    fs::write(&log, text).expect("a torn log");
    let report = export.report();
    assert_eq!(report.photos.counted, 10);
    assert!(
        report
            .notes
            .iter()
            .any(|note| note.starts_with("Line 11 of reads.jsonl")),
        "{:?}",
        report.notes
    );
}

#[test]
fn a_torn_shot_refuses_the_report_and_names_the_photo() {
    let export = Export::of(&ten_clean());
    let shots = ShotStore::new(export.dirs().shots);
    fs::write(shots.path_for(&digest_of(4)), "{\"shot_id").expect("a torn shot");
    let bars = Bars::parse(&committed()).expect("bars");
    let refused =
        bars_report(&export.root, &bars, CAMERA, None).expect_err("no number from a torn shot");
    assert!(refused.contains(&format!("{SESSION}/5")), "{refused}");
}

#[test]
fn a_correction_that_is_no_number_refuses_the_report_and_says_which() {
    let export = Export::of(&[clean("7i", 1000)]);
    let shots = ShotStore::new(export.dirs().shots);
    let mut shot = shots.get(&digest_of(0)).expect("a shot").expect("a shot");
    shot.provenance
        .as_mut()
        .expect("a provenance")
        .corrections
        .insert("carry_distance".to_string(), "lots".to_string());
    shots.put(&shot).expect("a shot");
    let bars = Bars::parse(&committed()).expect("bars");
    let refused =
        bars_report(&export.root, &bars, CAMERA, None).expect_err("no guess at what was read");
    assert!(refused.contains("carry_distance"), "{refused}");
    assert!(refused.contains("'lots'"), "{refused}");
}

#[test]
fn a_shot_no_screen_read_refuses_the_report() {
    let export = Export::of(&[clean("7i", 1000)]);
    let shots = ShotStore::new(export.dirs().shots);
    let mut shot = shots.get(&digest_of(0)).expect("a shot").expect("a shot");
    shot.provenance = None;
    fs::write(
        shots.path_for(&digest_of(0)),
        serde_json::to_string(&shot).expect("a shot"),
    )
    .expect("a shot");
    let bars = Bars::parse(&committed()).expect("bars");
    let refused =
        bars_report(&export.root, &bars, CAMERA, None).expect_err("nothing to compare with");
    assert!(refused.contains("no provenance"), "{refused}");
}

#[test]
fn what_is_not_an_export_is_refused_and_says_so() {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let bars = Bars::parse(&committed()).expect("bars");
    let refused = bars_report(dir.path(), &bars, CAMERA, None).expect_err("an empty directory");
    assert!(refused.contains("is not an export"), "{refused}");

    let text = dir.path().join("notes.txt");
    fs::write(&text, "not a zip").expect("a file");
    let refused = bars_report(&text, &bars, CAMERA, None).expect_err("a text file");
    assert!(
        refused.contains("neither a directory nor a zip"),
        "{refused}"
    );

    let missing = dir.path().join("missing.zip");
    assert!(bars_report(&missing, &bars, CAMERA, None).is_err());
}

/// An export holding a golfer and nothing else is a phone that has not been used. It says so.
#[test]
fn an_export_with_no_log_is_invalid_and_notes_why() {
    let export = Export::of(&[]);
    fs::create_dir_all(export.dirs().golfers).expect("golfers");
    let report = export.report();
    assert_eq!(report.verdict, Verdict::Invalid);
    assert_eq!(report.photos.counted, 0);
    assert_eq!(
        missed_bars(&report),
        ["T2", "T3", "T4", "T5"],
        "T1 counts nothing, and is 0"
    );
    assert!(
        report
            .notes
            .iter()
            .any(|note| note.contains("no .app/reads.jsonl")),
        "{:?}",
        report.notes
    );
    let text = render_bars(&report);
    assert!(text.contains("nothing to measure"), "{text}");
}

/// A valid session where no cell was ever judged has nothing to pass on. Only a person clearing every
/// value could reach it, and it must not read as a pass.
#[test]
fn a_bar_with_nothing_to_measure_is_not_met_on_a_valid_session() {
    let photos: Vec<Photo> = (0..30)
        .map(|at| {
            let mut photo = clean(if at < 15 { "7i" } else { "pw" }, 1500);
            photo.edits = TWELVE
                .iter()
                .map(|&(field, _)| (field, Edit::Blank))
                .collect();
            photo
        })
        .collect();
    let report = Export::of(&photos).report();
    assert!(report.session.valid);
    assert_eq!(bar(&report, "T2").met, None);
    assert_eq!(bar(&report, "T3").met, None);
    assert_eq!(report.verdict, Verdict::Miss);
}

// --- the sign letters and the confidences ---------------------------------------------------------

/// `IMG_2889`'s `R` from the table in `crates/screen/src/parser.rs`: 90 / 222 = 0.40540..., so its margin
/// is 0.7 - 0.40540... = +0.29459.... `F`: 176 / 183 = 0.96174..., margin -0.26174....
#[test]
fn each_sign_letter_is_held_against_the_ratio_with_its_margin() {
    let export = Export::of(&[clean("7i", 1000)]);
    export.write_boxes(
        &digest_of(0),
        r#"[{"text":"R","x":10,"y":20,"width":5,"height":5,"confidence":1.0,"rgb":[239,222,90]},
            {"text":" f ","x":30,"y":40,"width":5,"height":5,"confidence":1.0,"rgb":[187,183,176]},
            {"text":"DD","x":50,"y":60,"width":5,"height":5,"confidence":1.0,"rgb":[239,222,90]},
            {"text":"R2","x":50,"y":60,"width":5,"height":5,"confidence":1.0,"rgb":[239,222,90]},
            {"text":"D","x":70,"y":80,"width":5,"height":5,"confidence":1.0},
            {"text":"L","x":90,"y":99,"width":5,"height":5,"confidence":1.0,"rgb":[0,0,0]}]"#,
    );
    let letters = export.report().letters;
    assert_eq!(letters.blue_ratio, 0.7);
    let rows = &letters.rows;
    let spelled: Vec<&str> = rows.iter().map(|row| row.letter.as_str()).collect();
    assert_eq!(
        spelled,
        ["R", "F", "D", "L"],
        "`DD` and `R2` are not letters"
    );

    let r = &rows[0];
    assert_eq!(r.field, "launch_direction");
    assert_eq!(r.ratio, Some(90.0 / 222.0));
    assert_eq!(r.margin, Some(0.7 - 90.0 / 222.0));
    assert_eq!(r.lit, Some(true));
    assert_eq!((r.x, r.y), (10.0, 20.0));

    let f = &rows[1];
    assert_eq!(f.field, "sidespin");
    assert_eq!(f.margin, Some(0.7 - 176.0 / 183.0));
    assert_eq!(f.lit, Some(false));

    let no_colour = &rows[2];
    assert_eq!(
        (no_colour.ratio, no_colour.margin, no_colour.lit),
        (None, None, None)
    );

    let black = &rows[3];
    assert_eq!(
        (black.ratio, black.margin, black.lit),
        (None, None, Some(false))
    );

    assert_eq!(letters.smallest_lit, Some(0.7 - 90.0 / 222.0));
    assert_eq!(letters.smallest_unlit, Some(0.7 - 176.0 / 183.0));
}

/// Yellow but dim: its ratio is on the lit side, and `LIT_MIN_BRIGHTNESS` is why it is not lit. The margin
/// says what the ratio says and `lit` says what the parser decided, and the report does not reconcile
/// them, since a row like this is what someone would want to look at.
#[test]
fn a_dim_letter_shows_a_lit_side_margin_and_is_not_lit() {
    let export = Export::of(&[clean("7i", 1000)]);
    export.write_boxes(
        &digest_of(0),
        r#"[{"text":"R","x":10,"y":20,"width":5,"height":5,"confidence":1.0,"rgb":[100,100,10]}]"#,
    );
    let letters = export.report().letters;
    assert_eq!(letters.rows[0].margin, Some(0.7 - 0.1));
    assert_eq!(letters.rows[0].lit, Some(false));
    assert_eq!(letters.smallest_lit, None);
    assert_eq!(letters.smallest_unlit, Some(0.7 - 0.1));
}

#[test]
fn the_box_confidences_are_counted_into_their_ranges() {
    let export = Export::of(&[clean("7i", 1000)]);
    let boxes: Vec<Value> = [1.0, 1.0, 1.0, 0.9999, 0.95, 0.5, 0.30000001192092896, 0.3, 0.0]
        .into_iter()
        .map(|confidence| {
            json!({"text":"stray","x":1,"y":1,"width":1,"height":1,"confidence":confidence})
        })
        .collect();
    // The object shape as well as the list, as `PhotoIn::boxes_json` allows.
    export.write_boxes(
        &digest_of(0),
        &json!({"boxes": boxes, "revision": 3}).to_string(),
    );
    // What a crash can leave beside it, which P6 finding 8 says never to read.
    fs::write(
        export
            .dirs()
            .app
            .join("boxes")
            .join(format!("{}.tmp", digest_of(0))),
        "not boxes",
    )
    .expect("a leftover");
    let held = export.report().confidences;
    assert_eq!((held.boxes, held.photos), (9, 1));
    assert_eq!(held.lowest, Some(0.0));
    let ranges: Vec<(&str, u64)> = held
        .histogram
        .iter()
        .map(|bucket| (bucket.range.as_str(), bucket.boxes))
        .collect();
    assert_eq!(
        ranges,
        [
            ("1.00", 3),
            ("0.90-0.99", 2),
            ("0.50-0.59", 1),
            ("0.30-0.39", 2),
            ("0.00-0.09", 1),
        ]
    );
    assert_eq!(held.below_one, 6);
    assert_eq!(held.below_one_behind_a_cell, 0, "`stray` is no tile's");
}

#[test]
fn a_photo_with_no_boxes_is_named_and_the_bars_do_not_need_them() {
    let report = Export::of(&ten_photos()).report();
    assert_eq!(report.confidences.photos, 0);
    assert_eq!(
        report.confidences.read_cells, 0,
        "nothing can be said about these"
    );
    assert_eq!(
        report.confidences.separates, "unknown",
        "a wrong cell with no boxes says nothing"
    );
    assert!(
        report
            .notes
            .iter()
            .any(|note| note.contains("10 photo(s) have no boxes")),
        "{:?}",
        report.notes
    );
    assert_eq!(bar(&report, "T2").of, Some((105, 119)));
}

#[test]
fn an_unreadable_box_file_is_a_note_and_not_a_refusal() {
    let export = Export::of(&[clean("7i", 1000)]);
    export.write_boxes(&digest_of(0), "[{\"text\"");
    let report = export.report();
    assert_eq!(report.cells.read, 12);
    assert!(
        report
            .notes
            .iter()
            .any(|note| note.contains("not a list of boxes")),
        "{:?}",
        report.notes
    );
}

// --- the real writers, on M33's six ---------------------------------------------------------------

const CAPTURES: [&str; 6] = [
    "IMG_2889",
    "IMG_2891",
    "IMG_2892",
    "IMG_2893",
    "IMG_2894",
    "2026-10-07-1",
];

fn spike() -> PathBuf {
    repo().join("spikes/vision-ocr")
}

fn raw_boxes(id: &str) -> String {
    let path = spike().join("boxes/C0/raw").join(format!("{id}.json"));
    fs::read_to_string(&path).unwrap_or_else(|failed| panic!("{}: {failed}", path.display()))
}

/// A data root as first-run setup leaves it, fed through the facade as the app feeds it.
struct Phone {
    dir: TempDir,
    root: String,
}

impl Phone {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("a scratch directory");
        let root = dir.path().join("processed").display().to_string();
        init_state(root.clone(), "a1b2".to_string()).expect("an install id");
        create_golfer(
            root.clone(),
            "Aaron".to_string(),
            Handedness::Right,
            AT.to_string(),
        )
        .expect("a golfer");
        let mut state = get_state(root.clone());
        state.player_id = Some("aaron".to_string());
        state.club = Some("7i".to_string());
        set_state(root.clone(), state).expect("a state");
        Phone { dir, root }
    }

    /// One capture and these boxes, recorded and logged as a camera photo. `salt` makes the bytes a
    /// different photo, as a second photograph of one screen would be. Copied first, since a `Failed`
    /// read moves its file (P6 finding 11).
    fn feed(&self, id: &str, salt: &str, boxes_json: String) -> RecordOutcome {
        let tmp = self.dir.path().join("tmp");
        fs::create_dir_all(&tmp).expect("tmp");
        let to = tmp.join(format!("{id}{salt}.jpg"));
        let mut bytes =
            fs::read(spike().join("captures").join(format!("{id}.jpg"))).expect("a capture");
        bytes.extend_from_slice(salt.as_bytes());
        fs::write(&to, bytes).expect("a copy");
        let outcome = record_photo(PhotoIn {
            root: self.root.clone(),
            path: to.display().to_string(),
            boxes_json,
            captured_at: AT.to_string(),
            now: AT.to_string(),
            challenge_call: None,
        })
        .expect("a recorded photo");
        log_read(
            self.root.clone(),
            ReadLog {
                at: AT.to_string(),
                photo_sha256: outcome.photo_sha256.clone(),
                source: ReadSource::Camera,
                status: outcome.status,
                boxes: outcome.boxes,
                ms_vision: 1270,
                ms_rust: outcome.ms_rust,
                ms_total: 1400,
            },
        )
        .expect("a line");
        outcome
    }

    fn correct(&self, card: &ShotCard, edit: FieldEdit) -> ShotCard {
        correct_shot(
            self.root.clone(),
            card.session_id.clone(),
            card.swing_id.clone(),
            vec![edit],
            AT.to_string(),
        )
        .expect("a correction")
    }

    fn report(&self) -> BarsReport {
        let bars = Bars::parse(&committed()).expect("bars");
        bars_report(Path::new(&self.root), &bars, CAMERA, None).expect("a report")
    }

    /// The six captures, each read as M33 read it.
    fn six(&self) -> Vec<ShotCard> {
        CAPTURES
            .iter()
            .map(|id| self.feed(id, "", raw_boxes(id)).card.expect("a card"))
            .collect()
    }
}

/// The boxes' numbers M33's log records for `raw`, reached through the phone's own store and log:
/// 248 boxes at 1.00 and 3 at 0.30, and smallest margins of +0.337 and -0.133 (`spikes/vision-ocr/log.md`).
/// `provisional: pasted copies`.
#[test]
fn m33s_six_through_the_phones_writers_come_to_m33s_box_numbers() {
    let phone = Phone::new();
    phone.six();
    let report = phone.report();

    let held = &report.confidences;
    assert_eq!((held.boxes, held.photos), (251, 6));
    let ranges: Vec<(&str, u64)> = held
        .histogram
        .iter()
        .map(|bucket| (bucket.range.as_str(), bucket.boxes))
        .collect();
    assert_eq!(ranges, [("1.00", 248), ("0.30-0.39", 3)]);
    assert_eq!(held.below_one, 3);
    // M33's log calls all three strays. One is behind a tile's cell, and that cell reads right, which
    // is D8's first datum: a low confidence behind a right cell, and never behind a wrong one.
    assert_eq!(held.below_one_behind_a_cell, 1);
    assert_eq!((held.read_cells, held.read_cells_below_one), (71, 1));
    assert_eq!(held.read_cells_lowest, Some(0.30000001192092896));
    assert_eq!(
        held.values_moved,
        Vec::<String>::new(),
        "M33: a confidence reaches the score alone"
    );
    assert_eq!(
        held.flagged_by_a_confidence,
        Vec::<String>::new(),
        "M33: none"
    );
    assert_eq!(held.separates, "no wrong cell");

    let rounded = |margin: Option<f64>| format!("{:+.3}", margin.expect("a margin"));
    assert_eq!(rounded(report.letters.smallest_lit), "+0.337");
    assert_eq!(rounded(report.letters.smallest_unlit), "-0.133");

    // No truth has been typed, so M33's one `sidespin` not read is a blank here, and nothing is missed.
    assert_eq!(report.photos.counted, 6);
    assert_eq!(report.photos.read, 6);
    assert_eq!(report.cells.read, 71);
    assert_eq!(report.cells.blank, 1);
    assert_eq!(report.missed.len(), 0);
}

/// Typing in what was not read, as P31's run sheet asks, turns M33's blank into its withheld cell, and
/// its T2 and T3 follow: 71 of 72 and 5 of 6, the numbers P15 will reach from M33's own truth.
/// `provisional: pasted copies`.
#[test]
fn the_review_turns_a_not_read_cell_into_a_withheld_one() {
    let phone = Phone::new();
    let cards = phone.six();
    let (card, row) = cards
        .iter()
        .find_map(|card| {
            let row = card
                .rows
                .iter()
                .find(|row| row.state == RowState::NotRead)?;
            Some((card, row))
        })
        .expect("M33's one cell not read");
    assert_eq!(row.field, "sidespin");
    phone.correct(
        card,
        FieldEdit {
            field: row.field.clone(),
            printed: Some("300".to_string()),
            letter: Some("F".to_string()),
        },
    );

    let report = phone.report();
    assert_eq!(report.cells.read, 71);
    assert_eq!(report.cells.withheld, 1);
    assert_eq!(report.cells.blank, 0);
    assert_eq!(bar(&report, "T1").met, Some(true));
    assert_eq!(bar(&report, "T2").of, Some((71, 72)));
    let t3 = bar(&report, "T3");
    assert_eq!(t3.of, Some((5, 6)));
    assert_eq!(t3.field.as_deref(), Some("sidespin"));
    assert_eq!(t3.met, Some(true), "5 of 6 is M33's limit, scaled");
    assert_eq!(report.missed[0].kind, CellKind::Withheld);
    assert_eq!(report.missed[0].as_read, None);
    assert_eq!(report.missed[0].truth, Some(300.0));
}

/// What `correct_shot` really writes for the two rows where a printed number is not a stored one: the
/// apex in feet and a lit tile's letter. `corrections` and the field are both in stored units, so an
/// untouched value typed back is no cell moved and a real change is one wrong cell, compared by value.
#[test]
fn a_real_correction_is_a_wrong_cell_in_stored_units() {
    let phone = Phone::new();
    let cards = phone.six();
    let card = &cards[0];
    let apex = card
        .rows
        .iter()
        .find(|row| row.field == "apex_height")
        .expect("an apex");
    let as_read = apex.value.expect("read");
    phone.correct(
        card,
        FieldEdit {
            field: "apex_height".to_string(),
            printed: Some("90".to_string()),
            letter: None,
        },
    );
    let report = phone.report();
    assert_eq!(report.cells.wrong, 1);
    assert_eq!(bar(&report, "T1").met, Some(false));
    let cell = &report.missed[0];
    assert_eq!(cell.field, "apex_height");
    assert_eq!(cell.as_read, Some(as_read));
    assert_eq!(cell.truth, Some(30.0), "90 feet is 30 yards");
    assert_eq!(
        report.confidences.separates, "no",
        "its boxes are all at 1.00, as most are"
    );
}

/// An empty save releases a flagged shot (C11), and the log's line is what T4 reads: "a later review
/// cannot change it". The photo is `IMG_2889` with one value edited so a physics check flags it, as
/// `tests/review.rs` does, since no capture trips one.
#[test]
fn a_later_review_cannot_change_a_flag() {
    let phone = Phone::new();
    let text = raw_boxes("IMG_2889").replace("\"1.08\"", "\"2.08\"");
    let outcome = phone.feed("IMG_2889", "", text);
    assert_eq!(outcome.status, RecordStatus::Flagged);
    let card = outcome.card.expect("a card");
    let released = correct_shot(
        phone.root.clone(),
        card.session_id,
        card.swing_id,
        Vec::new(),
        AT.to_string(),
    )
    .expect("a review");
    assert!(!released.flagged, "a person has looked");

    let report = phone.report();
    assert_eq!(report.photos.flagged, 1);
    assert_eq!(bar(&report, "T4").of, Some((1, 1)));
}

/// Both forms of the export answer the same, the zip having been cut by the real `export_sessions`.
/// P13 finding 10: the zip's names are sorted, with no directory entries.
#[test]
fn a_zip_and_a_directory_are_the_same_report() {
    let phone = Phone::new();
    phone.six();
    let zip = phone.dir.path().join("export.zip");
    export_sessions(
        phone.root.clone(),
        zip.display().to_string(),
        AT.to_string(),
    )
    .expect("an export");

    let bars = Bars::parse(&committed()).expect("bars");
    let from_zip = bars_report(&zip, &bars, CAMERA, None).expect("a report from the zip");
    assert_eq!(from_zip, phone.report());
    assert_eq!(from_zip.photos.counted, 6);
}

#[test]
fn opening_a_zip_leaves_no_scratch_directory_behind() {
    let phone = Phone::new();
    phone.six();
    let zip = phone.dir.path().join("export.zip");
    export_sessions(
        phone.root.clone(),
        zip.display().to_string(),
        AT.to_string(),
    )
    .expect("an export");

    let export = app::bay::Export::open(&zip).expect("an export");
    let unpacked = export.root().to_path_buf();
    assert!(unpacked.join(".app/reads.jsonl").is_file());
    assert!(unpacked.join("sessions").is_dir());
    let photos = fs::read_dir(unpacked.join("sessions"))
        .expect("sessions")
        .flatten()
        .flat_map(|session| fs::read_dir(session.path()).into_iter().flatten().flatten())
        .flat_map(|swing| fs::read_dir(swing.path()).into_iter().flatten().flatten())
        .filter(|file| file.file_name().to_string_lossy().ends_with(".jpg"))
        .count();
    assert_eq!(photos, 0, "no photo leaves the zip");
    drop(export);
    assert!(!unpacked.exists());
}

/// P13 finding 10: a golfer with only failed reads is a valid export with no `sessions/`, and the report
/// answers it. Each read of nothing is twelve withheld cells and a flag.
#[test]
fn an_export_of_only_failed_reads_is_invalid_and_does_not_fail() {
    let phone = Phone::new();
    let outcome = phone.feed("IMG_2889", "", "[]".to_string());
    assert_eq!(outcome.status, RecordStatus::Failed);
    let zip = phone.dir.path().join("export.zip");
    export_sessions(
        phone.root.clone(),
        zip.display().to_string(),
        AT.to_string(),
    )
    .expect("an export");

    let bars = Bars::parse(&committed()).expect("bars");
    let report = bars_report(&zip, &bars, CAMERA, None).expect("a report");
    assert_eq!(report.verdict, Verdict::Invalid);
    assert_eq!(report.photos.failed, 1);
    assert_eq!(report.cells.withheld, TWELVE.len() as u64);
    assert_eq!(bar(&report, "T4").of, Some((1, 1)));
    assert_eq!(report.clubs[0].club, None);
}

/// Every box at 0.30 and the bottom row of four tiles gone, labels and values together so nothing lands
/// in another tile: 8 of 12 tiles is 0.7 x 8/12 = 0.467 from coverage, so 0.30 of confidence is 0.557,
/// under 0.6, and 1.00 of it is 0.767, over. So the flag is a confidence's and no physics check's.
#[test]
fn a_flag_only_the_confidences_put_there_is_named() {
    let phone = Phone::new();
    let mut read: Value = serde_json::from_str(&raw_boxes("IMG_2892")).expect("boxes");
    let boxes = read["boxes"].as_array_mut().expect("boxes");
    boxes.retain(|text_box| {
        let y = text_box["y"].as_f64().expect("a y");
        !(1400.0..1600.0).contains(&y)
    });
    for text_box in boxes {
        text_box["confidence"] = json!(0.3);
    }
    let outcome = phone.feed("IMG_2892", "", read.to_string());
    assert_eq!(outcome.status, RecordStatus::Flagged);

    let held = phone.report().confidences;
    assert_eq!(held.flagged_by_a_confidence.len(), 1, "{held:?}");
    assert!(held.flagged_by_a_confidence[0].contains("IMG_2892"));
    assert_eq!(
        held.values_moved,
        Vec::<String>::new(),
        "only the flag moved"
    );
    assert_eq!(held.read_cells_lowest, Some(0.3));
    assert!(held.below_one_behind_a_cell > 0);
    assert!(
        held.below_one > held.below_one_behind_a_cell,
        "the strays are told from the rest"
    );
}

/// A physics check's flag is still there with every box at 1.00, so it is not a confidence's.
#[test]
fn a_flag_a_physics_check_put_there_is_not_named() {
    let phone = Phone::new();
    let text = raw_boxes("IMG_2889").replace("\"1.08\"", "\"2.08\"");
    let outcome = phone.feed("IMG_2889", "", text);
    assert_eq!(outcome.status, RecordStatus::Flagged);
    let held = phone.report().confidences;
    assert_eq!(held.flagged_by_a_confidence, Vec::<String>::new());
}

/// `IMG_2892`'s ball speed box at 0.20: a person says it was wrong, and only its own cell has that box
/// behind it, so a threshold between 0.20 and the rest would have caught it for nothing. A second wrong
/// cell with every box at 1.00 is the one no threshold catches, which is a `partly`.
#[test]
fn a_confidence_separating_a_wrong_cell_from_a_right_one_is_said_so() {
    let phone = Phone::new();
    let mut read: Value = serde_json::from_str(&raw_boxes("IMG_2892")).expect("boxes");
    let plain_card = |phone: &Phone, read: &Value| {
        phone
            .feed("IMG_2892", "", read.to_string())
            .card
            .expect("a card")
    };
    let probe = Phone::new();
    let probed = plain_card(&probe, &read);
    let ball_speed = probed
        .rows
        .iter()
        .find(|row| row.field == "ball_speed")
        .and_then(|row| row.printed.clone())
        .expect("a ball speed");
    let boxes = read["boxes"].as_array_mut().expect("boxes");
    let lowered = boxes
        .iter_mut()
        .filter(|text_box| text_box["text"].as_str() == Some(ball_speed.as_str()))
        .map(|text_box| text_box["confidence"] = json!(0.2))
        .count();
    assert_eq!(lowered, 1, "one box prints {ball_speed}");

    let card = plain_card(&phone, &read);
    phone.correct(
        &card,
        FieldEdit {
            field: "ball_speed".to_string(),
            printed: Some("1".to_string()),
            letter: None,
        },
    );
    let held = phone.report().confidences;
    assert_eq!(held.wrong_cells.len(), 1);
    assert_eq!(held.wrong_cells[0].field, "ball_speed");
    assert_eq!(held.wrong_cells[0].lowest, Some(0.2));
    assert_eq!(held.read_cells_below_one, 0);
    assert_eq!(held.read_cells_lowest, Some(1.0));
    assert_eq!(held.wrong_below_every_read, 1);
    assert_eq!(held.separates, "yes");

    let card = phone
        .feed("IMG_2892", "", read.to_string())
        .card
        .expect("the held card");
    phone.correct(
        &card,
        FieldEdit {
            field: "carry_distance".to_string(),
            printed: Some("1".to_string()),
            letter: None,
        },
    );
    let held = phone.report().confidences;
    assert_eq!(held.wrong_cells.len(), 2);
    assert_eq!(held.wrong_below_every_read, 1);
    assert_eq!(held.separates, "partly");
}

// --- the command ---------------------------------------------------------------------------------

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_golf-bay-report"))
}

#[test]
fn the_command_reads_the_repos_bars_by_default_and_prints_the_verdict() {
    let export = Export::of(&thirty());
    let out = bin().arg("bars").arg(&export.root).output().expect("a run");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout).expect("text");
    assert!(text.starts_with(&format!(
        "golf-bay-report bars: {}\n",
        export.root.display()
    )));
    assert!(text.contains("VERDICT: PASS."), "{text}");
}

/// `--bars` is a real path taken at run time, and the same binary's verdict moves with it.
#[test]
fn the_command_takes_a_scratch_copy_of_the_bars_and_the_verdict_moves() {
    let export = Export::of(&thirty());
    let dir = tempfile::tempdir().expect("a scratch directory");
    let copy = dir.path().join("bars.md");
    fs::write(&copy, edited("| at most 5000 ms |", "| at most 1000 ms |")).expect("a copy");
    let out = bin()
        .arg("bars")
        .arg(&export.root)
        .arg("--bars")
        .arg(&copy)
        .output()
        .expect("a run");
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).expect("text");
    assert!(text.contains("VERDICT: MISS (latency)."), "{text}");
}

#[test]
fn the_command_writes_json_with_the_same_numbers() {
    let export = Export::of(&ten_photos());
    let out = bin()
        .arg("bars")
        .arg(&export.root)
        .arg("--json")
        .output()
        .expect("a run");
    assert!(out.status.success());
    let written: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert_eq!(written["verdict"], "INVALID");
    assert_eq!(written["session"]["camera_photos"], 10);
    assert_eq!(written["bars"][1]["id"], "T2");
    assert_eq!(written["bars"][1]["of"], json!([105, 119]));
    assert_eq!(written["bars"][1]["met"], false);
    assert_eq!(written["missed"][1]["kind"], "wrong");
    assert_eq!(written["photos"]["by_source"]["camera"], 10);
}

#[test]
fn the_command_takes_a_source() {
    let mut photos = ten_clean();
    photos.push(clean("7i", 2000).from(ReadSource::Library));
    let export = Export::of(&photos);
    let out = bin()
        .arg("bars")
        .arg(&export.root)
        .args(["--source", "all", "--json"])
        .output()
        .expect("a run");
    assert!(out.status.success());
    let written: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert_eq!(written["source"], "all");
    assert_eq!(written["photos"]["counted"], 11);
}

#[test]
fn the_command_takes_a_since_and_refuses_one_with_no_offset() {
    let export = Export::of(&rehearsal_then_bay());
    let out = bin()
        .arg("bars")
        .arg(&export.root)
        .args(["--since", "2026-11-05T00:00:00-05:00"])
        .output()
        .expect("a run");
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).expect("text");
    assert!(text.contains("VERDICT: PASS."), "{text}");

    let out = bin()
        .arg("bars")
        .arg(&export.root)
        .args(["--since", "2026-11-05T00:00:00"])
        .output()
        .expect("a run");
    assert_eq!(out.status.code(), Some(2));
    let refused = String::from_utf8(out.stderr).expect("text");
    assert!(refused.contains("--since"), "{refused}");
    assert!(refused.contains("offset"), "{refused}");
}

#[test]
fn the_command_refuses_with_exit_two_and_a_bad_verb_with_exit_one() {
    let export = Export::of(&ten_clean());
    let root = export.root.display().to_string();
    for args in [
        vec!["bars"],
        vec!["bars", &root, "--frob"],
        vec!["bars", &root, "--bars"],
        vec!["bars", &root, "--since"],
        vec!["bars", &root, "--source", "phone"],
        vec!["bars", &root, "--bars", "/no/such/bars.md"],
        vec!["bars", "/no/such/export.zip"],
        vec!["bars", &root, "second"],
    ] {
        let out = bin().args(&args).output().expect("a run");
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
        assert!(!out.stderr.is_empty(), "{args:?}");
    }
    let out = bin().arg("frob").output().expect("a run");
    assert_eq!(out.status.code(), Some(1));
    let out = bin().output().expect("a run");
    assert_eq!(out.status.code(), Some(1));
    let out = bin().arg("--help").output().expect("a run");
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stdout).contains("bars <export.zip|dir>"));
}
