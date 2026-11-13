//! `golf-bay-report truth` and `compare`, each with a known answer. [M38 P15]
//!
//! **The control is M33's own record**, `spikes/vision-ocr/results/C0.json`'s `raw` arm: the phone's store
//! over the committed boxes has to come to that file's numbers cell by cell, to the bit. Everything else
//! here moves one thing away from that and says what the report does about it.
//!
//! **Everything read off the six captures here is provisional: pasted copies** (rule 12), and those files
//! are read and never written (rule 5). The stores and the log are written by the real writers, and the
//! other end of `compare` by the real `import-export`, in scratch directories (rule 6).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use app::api::bay::{truth_report, TruthCell, TruthCellKind, TruthPhoto, TruthReport, TruthRoute};
use app::api::export::export_sessions;
use app::api::history::delete_shot;
use app::api::record::{
    log_read, record_photo, PhotoIn, ReadLog, ReadSource, RecordOutcome, RecordStatus, ShotCard,
};
use app::api::review::{correct_shot, FieldEdit};
use app::api::setup::{create_golfer, get_state, init_state, set_state, Handedness};
use app::bay::{compare_corpora, render_comparison, truth_of, Comparison, PASTED};
use app::Dirs;
use golf_lab::import_export::{import_export, Request};
use serde_json::{json, Value};
use storage::shot_store::ShotStore;
use tempfile::TempDir;

const AT: &str = "2026-11-05T10:00:00-05:00";
const EXPORTED: &str = "2026-11-05T11:00:00-05:00";
const CAPTURES: [&str; 6] = [
    "IMG_2889",
    "IMG_2891",
    "IMG_2892",
    "IMG_2893",
    "IMG_2894",
    "2026-10-07-1",
];

/// A number on `IMG_2889`'s screen that no physics check looks at, so a wrong one goes unwarned.
const QUIET_FROM: &str = "\"9899\"";
const QUIET_TO: &str = "\"9898\"";
/// One a physics check does look at, as `tests/review.rs` uses it, so a wrong one is flagged.
const LOUD_FROM: &str = "\"1.08\"";
const LOUD_TO: &str = "\"2.08\"";

fn spike() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spikes/vision-ocr")
}

fn photos_file() -> PathBuf {
    spike().join("photos.json")
}

fn truth_file() -> PathBuf {
    spike().join("truth_full_swing.json")
}

fn raw_boxes(id: &str) -> String {
    let path = spike().join("boxes/C0/raw").join(format!("{id}.json"));
    fs::read_to_string(&path).unwrap_or_else(|failed| panic!("{}: {failed}", path.display()))
}

/// `IMG_2889`'s boxes with one text replaced, held to there being exactly one to replace.
fn edited_boxes(from: &str, to: &str) -> String {
    let text = raw_boxes("IMG_2889");
    assert_eq!(text.matches(from).count(), 1, "{from}");
    text.replace(from, to)
}

fn capture_bytes(id: &str) -> Vec<u8> {
    fs::read(spike().join("captures").join(format!("{id}.jpg"))).expect("a capture")
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

    /// These bytes and boxes under this name, through `record_photo` with no log line, which is P18's road.
    /// Copied first, since a `Failed` read moves its file (P6 finding 11).
    fn record(&self, name: &str, bytes: &[u8], boxes_json: String) -> RecordOutcome {
        let tmp = self.dir.path().join("tmp");
        fs::create_dir_all(&tmp).expect("tmp");
        let to = tmp.join(name);
        fs::write(&to, bytes).expect("a copy");
        record_photo(PhotoIn {
            root: self.root.clone(),
            path: to.display().to_string(),
            boxes_json,
            captured_at: AT.to_string(),
            now: AT.to_string(),
            challenge_call: None,
        })
        .expect("a recorded photo")
    }

    fn log(&self, outcome: &RecordOutcome) {
        log_read(
            self.root.clone(),
            ReadLog {
                at: AT.to_string(),
                photo_sha256: outcome.photo_sha256.clone(),
                source: ReadSource::Library,
                status: outcome.status,
                boxes: outcome.boxes,
                ms_vision: 1270,
                ms_rust: outcome.ms_rust,
                ms_total: 1400,
            },
        )
        .expect("a line");
    }

    /// One capture as it is, its bytes and its committed boxes, unlogged.
    fn capture(&self, id: &str) -> RecordOutcome {
        self.record(&format!("{id}.jpg"), &capture_bytes(id), raw_boxes(id))
    }

    /// One capture with these boxes in place of its own, and logged.
    fn logged(&self, id: &str, boxes_json: String) -> RecordOutcome {
        let outcome = self.record(&format!("{id}.jpg"), &capture_bytes(id), boxes_json);
        self.log(&outcome);
        outcome
    }

    /// `name` holding other bytes than the capture's, as Photos may leave a capture.
    fn reencoded(&self, id: &str, name: &str) -> RecordOutcome {
        let mut bytes = capture_bytes(id);
        bytes.extend_from_slice(name.as_bytes());
        self.record(name, &bytes, raw_boxes(id))
    }

    fn six(&self) -> Vec<RecordOutcome> {
        CAPTURES.iter().map(|id| self.capture(id)).collect()
    }

    fn six_logged(&self) {
        for id in CAPTURES {
            self.logged(id, raw_boxes(id));
        }
    }

    fn truth(&self) -> TruthReport {
        truth_report(
            self.root.clone(),
            photos_file().display().to_string(),
            truth_file().display().to_string(),
        )
        .expect("a report")
    }

    fn correct(&self, card: &ShotCard, edits: Vec<FieldEdit>) {
        correct_shot(
            self.root.clone(),
            card.session_id.clone(),
            card.swing_id.clone(),
            edits,
            AT.to_string(),
        )
        .expect("a review");
    }

    fn export(&self) -> PathBuf {
        let zip = self.dir.path().join("tmp-export.zip");
        export_sessions(
            self.root.clone(),
            zip.display().to_string(),
            EXPORTED.to_string(),
        )
        .expect("an export");
        zip
    }
}

fn card(outcome: &RecordOutcome) -> &ShotCard {
    outcome.card.as_ref().expect("a card")
}

fn photo<'a>(report: &'a TruthReport, id: &str) -> &'a TruthPhoto {
    report
        .photos
        .iter()
        .find(|photo| photo.id == id)
        .expect("a capture")
}

fn cell<'a>(report: &'a TruthReport, id: &str, field: &str) -> &'a TruthCell {
    photo(report, id)
        .cells
        .iter()
        .find(|cell| cell.field == field)
        .expect("a cell")
}

fn edit(field: &str, printed: &str, letter: Option<&str>) -> FieldEdit {
    FieldEdit {
        field: field.to_string(),
        printed: Some(printed.to_string()),
        letter: letter.map(str::to_string),
    }
}

// --- the control ---------------------------------------------------------------------------------

/// **The Done line.** P6's store over M33's committed `raw` boxes, read straight off a capture each: M33's
/// 71 of 72, 0 wrong, `sidespin` lowest at 5 of 6, 0 flagged. `provisional: pasted copies`.
#[test]
fn the_control_comes_to_m33s_numbers_and_is_labelled_provisional() {
    let phone = Phone::new();
    phone.six();
    let report = phone.truth();

    assert_eq!(report.label.as_deref(), Some(PASTED));
    assert_eq!(PASTED, "provisional: pasted copies");
    assert_eq!((report.read, report.judged), (71, 72));
    assert_eq!((report.wrong, report.caught, report.withheld), (0, 0, 1));
    assert_eq!(report.flagged, 0);
    assert_eq!(report.lowest_field.as_deref(), Some("sidespin"));
    assert_eq!((report.lowest_read, report.lowest_judged), (5, 6));
    assert_eq!(
        (
            report.captures,
            report.matched,
            report.by_digest,
            report.by_name
        ),
        (6, 6, 6, 0)
    );
    assert!(report.held);

    let text = &report.text;
    assert!(text.starts_with("provisional: pasted copies\n"), "{text}");
    for line in [
        "Captures: 6 of 6 matched, 6 by digest and 0 by name.",
        "T1 wrong cells on photos not flagged: 0. Caught on flagged photos: 0.",
        "T2 read cells: 71 of 72 (98.6%)",
        "T3 the lowest field: sidespin, 5 of 6 (83.3%)",
        "T4 flagged photos: 0 of 6 (0.0%)",
        "IMG_2893 sidespin: withheld, as read not read, truth -638.0",
    ] {
        assert!(text.contains(line), "no {line:?} in:\n{text}");
    }
}

/// The totals could agree by luck. **Every one of the 72 cells is the one M33 recorded**: its word, the
/// value it read and the truth `truth()` made of the table, each by `==`, so the conversion here is the
/// Python's to the bit, `apex_ft / 3.0` included. The summary and its bars come from the same file.
#[test]
fn every_cell_is_the_one_m33_recorded_to_the_bit() {
    let phone = Phone::new();
    phone.six();
    let report = phone.truth();

    let results = fs::read_to_string(spike().join("results/C0.json")).expect("M33's results");
    let results: Value = serde_json::from_str(&results).expect("JSON");
    let set = &results["sets"]["fs-still"];
    assert_eq!(set["label"], PASTED, "M33 labelled it too");
    let raw = &set["arms"]["raw"];
    let summary = &raw["summary"];
    assert_eq!(summary["read"], report.read);
    assert_eq!(summary["withheld"], report.withheld);
    assert_eq!(summary["wrong"], report.wrong);
    assert_eq!(summary["caught"], report.caught);
    assert_eq!(summary["flagged"], report.flagged);
    for row in &report.fields {
        assert_eq!(summary["per_field"][&row.field], row.read, "{}", row.field);
        assert_eq!(row.judged, 6, "{}", row.field);
    }

    let shots = raw["shots"].as_object().expect("M33's shots");
    assert_eq!(shots.len(), report.photos.len());
    let mut compared = 0;
    for photo in &report.photos {
        let shot = &shots[&photo.id];
        assert_eq!(shot["flagged"], photo.flagged, "{}", photo.id);
        for cell in &photo.cells {
            let held = &shot["cells"][&cell.field];
            let word = match cell.kind {
                TruthCellKind::Read => "read",
                TruthCellKind::Wrong => "wrong",
                TruthCellKind::Caught => "caught",
                TruthCellKind::Withheld => "withheld",
            };
            assert_eq!(held["cell"], word, "{} {}", photo.id, cell.field);
            assert_eq!(
                held["value"].as_f64(),
                cell.as_read,
                "{} {}",
                photo.id,
                cell.field
            );
            assert_eq!(
                held["truth"].as_f64(),
                Some(cell.truth),
                "{} {}",
                photo.id,
                cell.field
            );
            compared += 1;
        }
    }
    assert_eq!(compared, 72);
    assert_eq!(
        cell(&report, "IMG_2889", "apex_height").truth,
        85.2 / 3.0,
        "the division, not a typed decimal"
    );
    assert_eq!(cell(&report, "IMG_2889", "launch_direction").truth, 10.1);
    assert_eq!(cell(&report, "IMG_2891", "launch_direction").truth, -0.8);
    assert_eq!(cell(&report, "2026-10-07-1", "sidespin").truth, 336.0);
    assert_eq!(cell(&report, "IMG_2889", "sidespin").truth, -4199.0);
}

/// P18's road logs nothing and the bay's logs everything, and M33's numbers come out of both.
#[test]
fn the_control_is_the_same_with_the_log_written() {
    let phone = Phone::new();
    phone.six_logged();
    let report = phone.truth();
    assert_eq!((report.read, report.judged, report.wrong), (71, 72, 0));
    assert_eq!(report.flagged, 0);
    assert_eq!(report.notes, Vec::<String>::new());
    assert!(report.held);
}

/// With no log the flag shown is the stored one, and a person should know that is what they are reading.
#[test]
fn with_no_log_the_report_says_whose_flag_it_shows() {
    let phone = Phone::new();
    phone.six();
    let report = phone.truth();
    assert_eq!(report.notes.len(), 1, "{:?}", report.notes);
    assert!(
        report.notes[0].starts_with("6 photo(s) have no read line"),
        "{:?}",
        report.notes
    );
    assert!(report.notes[0].contains("IMG_2889"), "{:?}", report.notes);
}

/// A zip holds no photo that `truth` reads, and comes to the same table to the last word.
#[test]
fn a_zip_and_a_directory_are_the_same_report() {
    let phone = Phone::new();
    phone.six_logged();
    let from_zip = truth_of(&phone.export(), &photos_file(), &truth_file()).expect("a report");
    assert_eq!(from_zip, phone.truth());
    assert_eq!(from_zip.read, 71);
}

// --- a cell --------------------------------------------------------------------------------------

/// What the control cannot show, which is that a wrong number would be seen. This one goes unwarned, so
/// it is rule 11's stop, and the report does not hold.
#[test]
fn a_wrong_number_on_an_unflagged_photo_is_wrong_and_the_report_does_not_hold() {
    let phone = Phone::new();
    let outcome = phone.logged("IMG_2889", edited_boxes(QUIET_FROM, QUIET_TO));
    assert_eq!(outcome.status, RecordStatus::Read);
    let report = phone.truth();

    let wrong = cell(&report, "IMG_2889", "backspin");
    assert_eq!(wrong.kind, TruthCellKind::Wrong);
    assert_eq!(wrong.as_read, Some(9898.0));
    assert_eq!(wrong.truth, 9899.0);
    assert_eq!((report.wrong, report.caught), (1, 0));
    assert_eq!(report.judged, 12);
    assert_eq!(report.read, 11);
    assert!(!report.held);
    assert!(
        report
            .text
            .contains("IMG_2889 backspin: wrong, as read 9898.0, truth 9899.0"),
        "{}",
        report.text
    );
    assert!(
        report
            .text
            .contains("T1 wrong cells on photos not flagged: 1."),
        "{}",
        report.text
    );
}

/// M33's word for a wrong number a person was asked to look at. It costs T2, and it is no failure of T1.
#[test]
fn a_wrong_number_on_a_flagged_photo_is_caught_and_counts_against_the_read_share_only() {
    let phone = Phone::new();
    let outcome = phone.logged("IMG_2889", edited_boxes(LOUD_FROM, LOUD_TO));
    assert_eq!(outcome.status, RecordStatus::Flagged);
    for id in &CAPTURES[1..] {
        phone.logged(id, raw_boxes(id));
    }
    let report = phone.truth();

    assert_eq!(
        cell(&report, "IMG_2889", "smash_factor").kind,
        TruthCellKind::Caught
    );
    assert_eq!((report.wrong, report.caught), (0, 1));
    assert_eq!((report.read, report.judged), (70, 72));
    assert_eq!(report.flagged, 1);
    // A tie at five of six, which goes to the first in the device's tile order.
    assert_eq!(report.lowest_field.as_deref(), Some("smash_factor"));
    assert_eq!((report.lowest_read, report.lowest_judged), (5, 6));
    assert!(photo(&report, "IMG_2889").flagged);
    assert!(
        report.held,
        "only a wrong number nobody was warned of stops it"
    );
}

/// **A review cannot move these numbers, which is why `bars` could not be the control** (P14 finding 3).
/// A wrong number corrected to the truth is what the reader gave, so it is still wrong.
#[test]
fn a_person_correcting_a_wrong_number_does_not_make_the_reader_right() {
    let phone = Phone::new();
    let outcome = phone.logged("IMG_2889", edited_boxes(QUIET_FROM, QUIET_TO));
    phone.correct(card(&outcome), vec![edit("backspin", "9899", None)]);
    let report = phone.truth();
    let wrong = cell(&report, "IMG_2889", "backspin");
    assert_eq!(wrong.kind, TruthCellKind::Wrong);
    assert_eq!(wrong.as_read, Some(9898.0));
    assert_eq!(report.wrong, 1);
}

/// The other way round: typing in what was not read leaves a cell the reader withheld.
#[test]
fn a_person_typing_in_what_was_not_read_leaves_it_withheld() {
    let phone = Phone::new();
    let outcome = phone.logged("IMG_2893", raw_boxes("IMG_2893"));
    phone.correct(card(&outcome), vec![edit("sidespin", "638", Some("D"))]);
    let report = phone.truth();
    let withheld = cell(&report, "IMG_2893", "sidespin");
    assert_eq!(withheld.kind, TruthCellKind::Withheld);
    assert_eq!(withheld.as_read, None);
    assert_eq!(report.withheld, 1);
}

/// And what the reader had right stays read, whatever a person then typed over it.
#[test]
fn a_person_changing_a_right_number_does_not_make_the_reader_wrong() {
    let phone = Phone::new();
    let outcome = phone.logged("IMG_2889", raw_boxes("IMG_2889"));
    phone.correct(card(&outcome), vec![edit("backspin", "1234", None)]);
    let report = phone.truth();
    let right = cell(&report, "IMG_2889", "backspin");
    assert_eq!(right.kind, TruthCellKind::Read);
    assert_eq!(right.as_read, Some(9899.0));
    assert_eq!(report.wrong, 0);
}

/// Saving a review clears the stored flag, so the log is what says it was flagged, the bars file's rule.
/// A wrong number flagged at read time stays caught, and is not turned into an unwarned one by the save.
#[test]
fn a_saved_review_does_not_turn_a_caught_cell_into_a_wrong_one() {
    let phone = Phone::new();
    let outcome = phone.logged("IMG_2889", edited_boxes(LOUD_FROM, LOUD_TO));
    phone.correct(card(&outcome), Vec::new());
    let report = phone.truth();
    assert!(photo(&report, "IMG_2889").flagged);
    assert_eq!((report.wrong, report.caught, report.flagged), (0, 1, 1));
}

/// With no line it can only be the stored flag, and that is the cost the note names.
#[test]
fn with_no_line_a_saved_review_is_what_the_stored_flag_says() {
    let phone = Phone::new();
    let boxes = edited_boxes(LOUD_FROM, LOUD_TO);
    let outcome = phone.record("IMG_2889.jpg", &capture_bytes("IMG_2889"), boxes);
    assert_eq!(outcome.status, RecordStatus::Flagged);
    assert!(
        photo(&phone.truth(), "IMG_2889").flagged,
        "before the review"
    );
    phone.correct(card(&outcome), Vec::new());
    let report = phone.truth();
    assert!(!photo(&report, "IMG_2889").flagged);
    assert_eq!(report.wrong, 1);
    assert!(report
        .notes
        .iter()
        .any(|note| note.contains("which a review clears")));
}

// --- a route -------------------------------------------------------------------------------------

/// **P30's case**: every capture's bytes differ and its name remains. Same numbers, and each is labelled.
#[test]
fn a_re_encoded_capture_is_found_by_its_name_and_labelled_a_copy() {
    let phone = Phone::new();
    for id in CAPTURES {
        phone.reencoded(id, &format!("{id}.jpg"));
    }
    let report = phone.truth();
    assert_eq!(
        (report.matched, report.by_digest, report.by_name),
        (6, 0, 6)
    );
    assert_eq!((report.read, report.judged, report.wrong), (71, 72, 0));
    assert!(report.held);
    assert!(report
        .photos
        .iter()
        .all(|photo| photo.route == TruthRoute::Name));
    assert!(
        report.text.contains("name: a re-encoded copy"),
        "{}",
        report.text
    );
    assert!(
        report
            .text
            .contains("Captures: 6 of 6 matched, 0 by digest and 6 by name."),
        "{}",
        report.text
    );
}

/// A copy's line is under the copy's digest and not the capture's, so that is where its flag is read.
#[test]
fn a_re_encoded_captures_flag_is_the_one_logged_for_the_copy() {
    let phone = Phone::new();
    let mut bytes = capture_bytes("IMG_2889");
    bytes.extend_from_slice(b"re-encoded");
    let outcome = phone.record("IMG_2889.jpg", &bytes, edited_boxes(LOUD_FROM, LOUD_TO));
    assert_eq!(outcome.status, RecordStatus::Flagged);
    phone.log(&outcome);
    phone.correct(card(&outcome), Vec::new());
    let report = phone.truth();
    let held = photo(&report, "IMG_2889");
    assert_eq!(held.route, TruthRoute::Name);
    assert!(held.flagged, "the review cleared the stored one");
    assert_eq!((report.wrong, report.caught), (0, 1));
}

/// An entry found by name with nothing under its digest is no read of anything, which nothing writes, so
/// it stands for nothing and is not called a read of the capture.
#[test]
fn an_entry_with_nothing_under_it_stands_for_nothing() {
    let phone = Phone::new();
    let outcome = phone.reencoded("IMG_2889", "IMG_2889.jpg");
    let dirs = Dirs::of(&phone.root);
    fs::remove_file(ShotStore::new(&dirs.shots).path_for(&outcome.photo_sha256)).expect("a shot");
    fs::remove_file(
        dirs.app
            .join("boxes")
            .join(format!("{}.json", outcome.photo_sha256)),
    )
    .expect("its boxes");
    let report = phone.truth();
    assert_eq!(photo(&report, "IMG_2889").route, TruthRoute::Absent);
    assert_eq!(report.matched, 0);
}

/// A stem and not a name, so a HEIC of a `.jpg` capture is found, and each route prints its own word.
#[test]
fn the_name_is_compared_without_its_extension() {
    let phone = Phone::new();
    phone.reencoded("IMG_2889", "IMG_2889.HEIC");
    phone.capture("IMG_2891");
    let report = phone.truth();
    assert_eq!(photo(&report, "IMG_2889").route, TruthRoute::Name);
    assert_eq!(photo(&report, "IMG_2891").route, TruthRoute::Digest);
    assert_eq!(photo(&report, "IMG_2892").route, TruthRoute::Absent);
    let entry = photo(&report, "IMG_2889")
        .entry
        .as_deref()
        .expect("an entry");
    assert!(entry.ends_with("IMG_2889.HEIC"), "{entry}");
    let lines: Vec<&str> = report.text.lines().collect();
    let shown = |id: &str| {
        lines
            .iter()
            .find(|line| line.starts_with(&format!("  {id} ")))
            .copied()
            .expect("a row")
    };
    assert!(
        shown("IMG_2889").contains("name: a re-encoded copy"),
        "{lines:?}"
    );
    assert!(shown("IMG_2891").contains(" digest "), "{lines:?}");
    assert!(shown("IMG_2891").contains("IMG_2891.jpg"), "{lines:?}");
    assert!(
        shown("IMG_2892").contains("not in this export"),
        "{lines:?}"
    );
}

/// A digest is proof and a name is not, so a wrong read under the same name cannot take a capture's place.
#[test]
fn a_digest_beats_a_name() {
    let phone = Phone::new();
    let mut other = capture_bytes("IMG_2889");
    other.extend_from_slice(b"another photo under that name");
    phone.record("IMG_2889.jpg", &other, edited_boxes(QUIET_FROM, QUIET_TO));
    phone.capture("IMG_2889");
    let report = phone.truth();
    assert_eq!(photo(&report, "IMG_2889").route, TruthRoute::Digest);
    assert_eq!(report.wrong, 0);
}

/// Two copies under one stem and no digest: either could be the one that was photographed, so neither is
/// chosen, nothing holds cells and it does not hold, and the note names both.
#[test]
fn two_entries_under_one_name_are_named_and_neither_is_chosen() {
    let phone = Phone::new();
    phone.reencoded("IMG_2889", "IMG_2889.jpg");
    phone.reencoded("IMG_2889", "IMG_2889.HEIC");
    let report = phone.truth();
    let held = photo(&report, "IMG_2889");
    assert_eq!(held.route, TruthRoute::Ambiguous);
    assert_eq!(held.cells, Vec::new());
    assert_eq!(held.entry, None);
    assert_eq!(report.matched, 0);
    assert!(!report.held);
    assert!(
        report.text.contains("name: ambiguous, no cells"),
        "{}",
        report.text
    );
    let note = report
        .notes
        .iter()
        .find(|note| note.contains("matches no digest"))
        .expect("a note");
    assert!(
        note.contains("IMG_2889.jpg") && note.contains("IMG_2889.HEIC"),
        "{note}"
    );
    assert!(
        !report
            .notes
            .iter()
            .any(|note| note.contains("other photo(s)")),
        "they are this capture's, whichever it was: {:?}",
        report.notes
    );
}

/// A capture not found is in no denominator, and the report does not hold: `0 wrong` over five of six is
/// not the control's answer.
#[test]
fn a_capture_not_in_the_export_is_named_counted_nowhere_and_stops_it_holding() {
    let phone = Phone::new();
    for id in &CAPTURES[..5] {
        phone.capture(id);
    }
    let report = phone.truth();
    assert_eq!((report.captures, report.matched), (6, 5));
    assert_eq!(report.judged, 60);
    assert_eq!(report.wrong, 0);
    assert_eq!(report.flagged, 0);
    assert_eq!(photo(&report, "2026-10-07-1").route, TruthRoute::Absent);
    assert_eq!(photo(&report, "2026-10-07-1").entry, None);
    assert!(!report.held);
    assert!(
        report.text.contains("not in this export"),
        "{}",
        report.text
    );
    assert!(
        report.text.contains("T4 flagged photos: 0 of 5"),
        "{}",
        report.text
    );
}

/// P30's step 2 and 3: a photo that is no capture is in no count, and the report says it is there.
#[test]
fn another_photo_in_the_export_is_counted_nowhere_and_noted() {
    let phone = Phone::new();
    phone.six_logged();
    let rehearsal = phone.record(
        "camera-1.jpg",
        b"a photo of the monitor",
        edited_boxes(QUIET_FROM, QUIET_TO),
    );
    phone.log(&rehearsal);
    let report = phone.truth();
    assert_eq!((report.read, report.judged, report.wrong), (71, 72, 0));
    assert!(report.held);
    assert!(
        report
            .notes
            .iter()
            .any(|note| note.starts_with("1 other photo(s) in the export match no capture")),
        "{:?}",
        report.notes
    );
}

// --- a photo with no shot ------------------------------------------------------------------------

/// M33's `classify(None, want)`, with the log saying so: twelve withheld and flagged.
#[test]
fn a_failed_read_is_twelve_withheld_and_flagged() {
    let phone = Phone::new();
    let outcome = phone.logged("IMG_2889", "[]".to_string());
    assert_eq!(outcome.status, RecordStatus::Failed);
    for id in &CAPTURES[1..] {
        phone.logged(id, raw_boxes(id));
    }
    let report = phone.truth();
    let held = photo(&report, "IMG_2889");
    assert_eq!(held.route, TruthRoute::Digest);
    assert!(held.failed_read && held.flagged);
    assert_eq!(held.cells.len(), 12);
    assert!(held
        .cells
        .iter()
        .all(|cell| { cell.kind == TruthCellKind::Withheld && cell.as_read.is_none() }));
    assert_eq!(cell(&report, "IMG_2889", "carry_distance").truth, 131.0);
    // Twelve, and M33's own one on `IMG_2893`.
    assert_eq!(
        (report.read, report.withheld, report.judged, report.flagged),
        (59, 13, 72, 1)
    );
    assert!(report.held, "a read of nothing warned nobody wrongly");
    assert!(
        report.text.contains(", flagged, a failed read"),
        "{}",
        report.text
    );
}

/// P18's road, where a failed read leaves only its boxes (C7) and no line says what happened.
#[test]
fn a_failed_read_with_no_line_is_known_by_its_boxes() {
    let phone = Phone::new();
    let outcome = phone.record("IMG_2889.jpg", &capture_bytes("IMG_2889"), "[]".to_string());
    assert_eq!(outcome.status, RecordStatus::Failed);
    let report = phone.truth();
    let held = photo(&report, "IMG_2889");
    assert_eq!(held.route, TruthRoute::Digest);
    assert!(held.failed_read && held.flagged);
    assert_eq!(report.withheld, 12);
}

/// A photo with a line saying it was read and no shot was deleted at the phone (`bars`' rule). There is
/// nothing to hold to the truth, so it holds no cells and it is not called a failed read.
#[test]
fn a_capture_deleted_at_the_phone_holds_no_cells() {
    let phone = Phone::new();
    let outcome = phone.logged("IMG_2889", raw_boxes("IMG_2889"));
    let shot = card(&outcome);
    delete_shot(
        phone.root.clone(),
        shot.session_id.clone(),
        shot.swing_id.clone(),
    )
    .expect("a delete");
    let report = phone.truth();
    let held = photo(&report, "IMG_2889");
    assert_eq!(held.route, TruthRoute::Deleted);
    assert!(!held.failed_read && !held.flagged);
    assert_eq!(held.cells, Vec::new());
    assert_eq!(report.matched, 0);
    assert!(!report.held);
    assert!(
        report.text.contains("deleted at the phone"),
        "{}",
        report.text
    );
}

/// A failed read keeps no name, so a re-encoded capture that failed cannot be found. The report says how
/// many failures could be it, and only when a capture is missing to be one of them.
#[test]
fn a_failure_that_could_be_a_lost_capture_is_said_so() {
    let phone = Phone::new();
    let failed = phone.record("camera-1.jpg", b"a failed read", "[]".to_string());
    phone.log(&failed);
    for id in &CAPTURES[..5] {
        phone.capture(id);
    }
    let hint = "a failed read keeps no name";
    let report = phone.truth();
    assert!(
        report.notes.iter().any(|note| note.contains(hint)),
        "{:?}",
        report.notes
    );

    phone.capture(CAPTURES[5]);
    let report = phone.truth();
    assert!(
        !report.notes.iter().any(|note| note.contains(hint)),
        "{:?}",
        report.notes
    );
}

/// A failed read that is a capture's own has been found, so it cannot be the one still missing.
#[test]
fn a_captures_own_failure_is_not_offered_as_the_lost_one() {
    let phone = Phone::new();
    phone.logged("IMG_2889", "[]".to_string());
    let report = phone.truth();
    assert_eq!(photo(&report, "IMG_2891").route, TruthRoute::Absent);
    assert!(
        !report
            .notes
            .iter()
            .any(|note| note.contains("a failed read keeps no name")),
        "{:?}",
        report.notes
    );
}

// --- the label and the files ---------------------------------------------------------------------

/// `photos.json` with each capture run through `change`, written to a scratch file.
fn scratch_photos(dir: &Path, change: impl Fn(&mut Value)) -> PathBuf {
    let mut photos: Value =
        serde_json::from_slice(&fs::read(photos_file()).expect("photos")).expect("JSON");
    for capture in photos["captures"].as_array_mut().expect("captures") {
        change(capture);
    }
    let path = dir.join("photos.json");
    fs::write(&path, photos.to_string()).expect("a scratch file");
    path
}

/// The table with `from` replaced by `to`, written to a scratch file.
fn scratch_truth(dir: &Path, from: &str, to: &str) -> PathBuf {
    let text = fs::read_to_string(truth_file()).expect("the table");
    assert!(text.contains(from), "{from}");
    let path = dir.join("truth.json");
    fs::write(&path, text.replacen(from, to, 1)).expect("a scratch file");
    path
}

/// On data rather than memory, as M33's `run` has it: the label is earned by a digest differing from its
/// original's, so the originals, when they run, come out with no such word on them.
#[test]
fn the_label_rests_on_a_capture_being_a_copy() {
    let phone = Phone::new();
    phone.six();
    let dir = tempfile::tempdir().expect("a scratch directory");
    let root = Path::new(&phone.root);

    let originals = scratch_photos(dir.path(), |capture| {
        capture["copy_of"]["sha256"] = capture["sha256"].clone();
    });
    let report = truth_of(root, &originals, &truth_file()).expect("a report");
    assert_eq!(report.label, None);
    assert!(!report.text.contains(PASTED), "{}", report.text);
    assert_eq!(report.read, 71);

    let bare = scratch_photos(dir.path(), |capture| {
        capture
            .as_object_mut()
            .expect("a capture")
            .remove("copy_of");
    });
    let report = truth_of(root, &bare, &truth_file()).expect("a report");
    assert_eq!(report.label, None);
}

/// A photos file's capture says which row it answers to, and the row is what it is held to.
#[test]
fn a_capture_answers_to_the_row_it_names() {
    let phone = Phone::new();
    phone.capture("IMG_2889");
    let dir = tempfile::tempdir().expect("a scratch directory");
    let photos = scratch_photos(dir.path(), |capture| {
        if capture["id"] == "IMG_2889" {
            capture["truth"] = json!("IMG_2891");
        }
    });
    let report = truth_of(Path::new(&phone.root), &photos, &truth_file()).expect("a report");
    assert_eq!(cell(&report, "IMG_2889", "carry_distance").truth, 162.0);
    assert_eq!(
        cell(&report, "IMG_2889", "carry_distance").kind,
        TruthCellKind::Wrong
    );
}

/// Each says which file and what is the matter with it. None becomes a number.
#[test]
fn what_cannot_be_held_to_a_truth_is_refused_and_says_why() {
    let phone = Phone::new();
    phone.capture("IMG_2889");
    let dir = tempfile::tempdir().expect("a scratch directory");
    let scratch = dir.path();
    let root = Path::new(&phone.root);
    let refused =
        |photos: &Path, truth: &Path| truth_of(root, photos, truth).expect_err("a refusal");

    let letter = scratch_truth(scratch, "[10.1, \"R\"]", "[10.1, \"X\"]");
    let said = refused(&photos_file(), &letter);
    assert_eq!(said, "the truth for IMG_2889: X is neither L nor R");

    let spin = scratch_truth(scratch, "[4199, \"D\"]", "[4199, \"L\"]");
    let said = refused(&photos_file(), &spin);
    assert_eq!(said, "the truth for IMG_2889: L is neither D nor F");

    let column = scratch_truth(scratch, "\"backspin\": 9899,", "");
    let said = refused(&photos_file(), &column);
    assert!(
        said.contains("the truth table at") && said.contains("backspin"),
        "{said}"
    );

    let row = scratch_photos(scratch, |capture| capture["truth"] = json!("IMG_0000"));
    let said = refused(&row, &truth_file());
    assert_eq!(
        said,
        "IMG_2889 answers to the truth row IMG_0000, and the table has none"
    );

    let device = scratch_photos(scratch, |capture| capture["device"] = json!("hd_golf"));
    let said = refused(&device, &truth_file());
    assert_eq!(
        said,
        "IMG_2889 is a capture of hd_golf, and this reads full_swing"
    );

    let none = scratch.join("none.json");
    fs::write(&none, "{\"captures\": []}").expect("a scratch file");
    assert!(refused(&none, &truth_file()).ends_with("holds no captures"));

    let garbled = scratch.join("garbled.json");
    fs::write(&garbled, "not json").expect("a scratch file");
    let said = refused(&garbled, &truth_file());
    assert!(said.starts_with("the photos file at"), "{said}");
    let said = refused(&photos_file(), &garbled);
    assert!(said.starts_with("the truth table at"), "{said}");

    let said = refused(&scratch.join("missing.json"), &truth_file());
    assert!(
        said.starts_with("could not read the photos file at"),
        "{said}"
    );

    let empty = tempfile::tempdir().expect("a scratch directory");
    let said = truth_of(empty.path(), &photos_file(), &truth_file()).expect_err("a refusal");
    assert!(said.contains("is not an export"), "{said}");
}

/// The bars' refusal, the same function under it: no number can be given for a cell like this.
#[test]
fn a_correction_that_is_no_number_refuses_the_report() {
    let phone = Phone::new();
    let outcome = phone.logged("IMG_2889", raw_boxes("IMG_2889"));
    let path = ShotStore::new(&Dirs::of(&phone.root).shots).path_for(&outcome.photo_sha256);
    let mut shot: Value = serde_json::from_slice(&fs::read(&path).expect("a shot")).expect("JSON");
    shot["provenance"]["corrections"]["backspin"] = json!("a lot");
    fs::write(&path, shot.to_string()).expect("a scratch edit");
    let said =
        truth_of(Path::new(&phone.root), &photos_file(), &truth_file()).expect_err("refused");
    assert!(said.contains("correction of backspin is 'a lot'"), "{said}");
}

// --- compare -------------------------------------------------------------------------------------

/// `golf-lab import-export` run for real into empty scratch directories, as P32 runs it.
struct Lab {
    _dir: TempDir,
    dirs: Dirs,
}

impl Lab {
    fn importing(export: &Path) -> Self {
        let dir = tempfile::tempdir().expect("a scratch directory");
        let dirs = Dirs::of(dir.path().join("processed"));
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = import_export(
            &Request {
                export: export.to_path_buf(),
                sessions_dir: dirs.sessions.clone(),
                shots_dir: dirs.shots.clone(),
                golfers_dir: dirs.golfers.clone(),
                drills_dir: dirs.drills.clone(),
                dry_run: false,
            },
            &mut out,
            &mut err,
        )
        .expect("a report");
        assert_eq!(code, 0, "{}", String::from_utf8_lossy(&err));
        Lab { _dir: dir, dirs }
    }

    /// One entry's manifest, parsed, changed and written back: the scratch copy's one edit.
    fn edit_manifest(&self, shot: &ShotCard, change: impl Fn(&mut Value)) {
        let path = self
            .dirs
            .sessions
            .join(&shot.session_id)
            .join(&shot.swing_id)
            .join("manifest.json");
        let mut manifest: Value =
            serde_json::from_slice(&fs::read(&path).expect("a manifest")).expect("JSON");
        change(&mut manifest);
        fs::write(&path, manifest.to_string()).expect("an edit");
    }

    fn compare(&self, export: &Path) -> Comparison {
        compare_corpora(export, &self.dirs.sessions, "aaron").expect("a comparison")
    }
}

/// **The Done line, its first half.** P13's round trip: six captures exported and imported, and `compare`
/// says equal. From the zip and from the phone's own directory, and they are the same answer.
#[test]
fn compare_says_equal_on_the_round_trip() {
    let phone = Phone::new();
    phone.six_logged();
    let zip = phone.export();
    let lab = Lab::importing(&zip);

    let answer = lab.compare(&zip);
    assert_eq!(answer.difference, None);
    assert_eq!((answer.swings, answer.excluded), (6, 0));
    assert_eq!((answer.other_swings, answer.other_excluded), (6, 0));
    assert_eq!(answer, lab.compare(Path::new(&phone.root)));
    assert_eq!(
        render_comparison(&answer),
        "Corpus of aaron: equal, 6 swing(s) and 0 excluded on each side.\n"
    );
}

/// **The Done line, its second half.** One value changed in a scratch copy, and the path says where.
#[test]
fn compare_names_the_path_when_one_value_is_changed() {
    let phone = Phone::new();
    let outcomes: Vec<RecordOutcome> = CAPTURES
        .iter()
        .map(|id| phone.logged(id, raw_boxes(id)))
        .collect();
    let zip = phone.export();
    let lab = Lab::importing(&zip);
    lab.edit_manifest(card(&outcomes[2]), |manifest| {
        manifest["club"] = json!("8i");
    });

    let answer = lab.compare(&zip);
    let difference = answer.difference.as_ref().expect("a difference");
    assert_eq!(difference.path, "swings[2].club");
    assert_eq!(difference.in_export, Some(json!("7i")));
    assert_eq!(difference.in_other, Some(json!("8i")));
    assert_eq!(
        render_comparison(&answer),
        "Corpus of aaron: DIFFERENT, first at swings[2].club: \"7i\" in the export, \"8i\" in the \
         other. 6 swing(s) and 0 excluded in the export, 6 and 0 in the other.\n"
    );
}

/// A shot the lab does not have. The first path is an aggregate, since keys are walked sorted and `swings`
/// comes late, so the answer also says how many each side holds, which is the fact a person wants.
#[test]
fn compare_names_a_shot_only_one_side_holds() {
    let phone = Phone::new();
    let outcomes: Vec<RecordOutcome> = CAPTURES
        .iter()
        .map(|id| phone.logged(id, raw_boxes(id)))
        .collect();
    let zip = phone.export();
    let lab = Lab::importing(&zip);
    let gone = card(&outcomes[1]);
    fs::remove_dir_all(
        lab.dirs
            .sessions
            .join(&gone.session_id)
            .join(&gone.swing_id),
    )
    .expect("an entry removed from the scratch copy");

    let answer = lab.compare(&zip);
    let difference = answer.difference.as_ref().expect("a difference");
    assert_eq!(difference.path, "metric_counts.ball_speed_mph");
    assert_eq!(difference.in_export, Some(json!(6)));
    assert_eq!(difference.in_other, Some(json!(5)));
    assert_eq!((answer.swings, answer.other_swings), (6, 5));
    let shown = render_comparison(&answer);
    assert!(
        shown.contains("6 swing(s) and 0 excluded in the export, 5 and 0 in the other."),
        "{shown}"
    );
}

/// A golfer the export holds nothing for reads as an empty corpus, and two of those are equal, so a
/// mistyped `--player` would answer the exit with a comparison of nothing.
#[test]
fn compare_refuses_to_call_two_empty_corpora_equal() {
    let phone = Phone::new();
    phone.six_logged();
    let zip = phone.export();
    let lab = Lab::importing(&zip);
    let said = compare_corpora(&zip, &lab.dirs.sessions, "arron").expect_err("a refusal");
    assert!(said.contains("'arron'"), "{said}");
    assert!(said.contains("nothing was compared"), "{said}");
}

/// One empty side is a real answer, and it is the lab that has not been imported into.
#[test]
fn compare_says_different_when_only_one_side_is_empty() {
    let phone = Phone::new();
    phone.six_logged();
    let dir = tempfile::tempdir().expect("a scratch directory");
    let answer = compare_corpora(&phone.export(), dir.path(), "aaron").expect("a comparison");
    assert!(answer.difference.is_some());
}

#[test]
fn compare_refuses_what_is_not_there() {
    let phone = Phone::new();
    phone.six_logged();
    let zip = phone.export();
    let dir = tempfile::tempdir().expect("a scratch directory");
    let missing = dir.path().join("missing");
    let said = compare_corpora(&zip, &missing, "aaron").expect_err("a refusal");
    assert!(said.ends_with("is not a directory"), "{said}");
    let said = compare_corpora(dir.path(), dir.path(), "aaron").expect_err("a refusal");
    assert!(said.contains("is not an export"), "{said}");
}

// --- the command ---------------------------------------------------------------------------------

fn bin(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_golf-bay-report"))
        .args(args)
        .output()
        .expect("a run")
}

fn code(out: &Output) -> i32 {
    out.status.code().expect("an exit code")
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).expect("text")
}

fn stderr(out: &Output) -> String {
    String::from_utf8(out.stderr.clone()).expect("text")
}

fn truth_args<'a>(export: &'a str, photos: &'a str, truth: &'a str) -> [&'a str; 6] {
    ["truth", export, "--photos", photos, "--truth", truth]
}

#[test]
fn the_truth_command_prints_the_table_and_exits_0_when_it_holds() {
    let phone = Phone::new();
    phone.six_logged();
    let (photos, truth) = (photos_file(), truth_file());
    let (photos, truth) = (photos.to_str().unwrap(), truth.to_str().unwrap());
    let zip = phone.export();
    let zip = zip.to_str().unwrap();

    let out = bin(&truth_args(zip, photos, truth));
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.starts_with(&format!("golf-bay-report truth: {zip}\n\nprovisional")),
        "{text}"
    );
    assert!(text.contains("T2 read cells: 71 of 72"), "{text}");

    let mut args = truth_args(zip, photos, truth).to_vec();
    args.push("--json");
    let out = bin(&args);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let json: Value = serde_json::from_str(&stdout(&out)).expect("JSON");
    assert_eq!(json["read"], 71);
    assert_eq!(json["label"], PASTED);
    assert_eq!(json["photos"][3]["route"], "digest");
    assert_eq!(json["photos"][3]["cells"][11]["kind"], "withheld");
    assert_eq!(
        json.get("text"),
        None,
        "the same words as the fields, and only once"
    );
}

/// **A wrong cell is what P28's script is there to stop on**, and so is a capture not found: 3, with the
/// table printed all the same.
#[test]
fn the_truth_command_exits_3_on_a_wrong_cell_and_on_a_missing_capture() {
    let (photos, truth) = (photos_file(), truth_file());
    let (photos, truth) = (photos.to_str().unwrap(), truth.to_str().unwrap());

    let phone = Phone::new();
    phone.logged("IMG_2889", edited_boxes(QUIET_FROM, QUIET_TO));
    for id in &CAPTURES[1..] {
        phone.logged(id, raw_boxes(id));
    }
    let out = bin(&truth_args(&phone.root, photos, truth));
    assert_eq!(code(&out), 3, "{}", stderr(&out));
    assert!(stdout(&out).contains("backspin: wrong"), "{}", stdout(&out));

    let phone = Phone::new();
    for id in &CAPTURES[1..] {
        phone.logged(id, raw_boxes(id));
    }
    let out = bin(&truth_args(&phone.root, photos, truth));
    assert_eq!(code(&out), 3, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("not in this export"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn the_truth_command_exits_2_on_a_refusal_and_says_why() {
    let phone = Phone::new();
    phone.capture("IMG_2889");
    let (photos, truth) = (photos_file(), truth_file());
    let (photos, truth) = (photos.to_str().unwrap(), truth.to_str().unwrap());

    for (args, said) in [
        (
            vec!["truth", &phone.root, "--photos", photos],
            "--truth is required",
        ),
        (
            vec!["truth", &phone.root, "--truth", truth],
            "--photos is required",
        ),
        (
            vec!["truth", "--photos", photos, "--truth", truth],
            "an export is required",
        ),
        (
            vec!["truth", &phone.root, "--photos"],
            "--photos needs a value",
        ),
        (
            vec![
                "truth",
                &phone.root,
                "--photos",
                photos,
                "--truth",
                truth,
                "--nope",
            ],
            "unknown flag '--nope'",
        ),
        (
            vec![
                "truth",
                &phone.root,
                &phone.root,
                "--photos",
                photos,
                "--truth",
                truth,
            ],
            "is a second",
        ),
        (
            vec![
                "truth",
                &phone.root,
                "--photos",
                "/no/such/file",
                "--truth",
                truth,
            ],
            "could not read the photos file",
        ),
    ] {
        let out = bin(&args);
        assert_eq!(code(&out), 2, "{args:?}");
        assert!(
            stderr(&out).starts_with("golf-bay-report truth: "),
            "{}",
            stderr(&out)
        );
        assert!(stderr(&out).contains(said), "{args:?}: {}", stderr(&out));
    }
}

#[test]
fn the_compare_command_says_equal_exits_0_and_a_difference_exits_3() {
    let phone = Phone::new();
    let outcomes: Vec<RecordOutcome> = CAPTURES
        .iter()
        .map(|id| phone.logged(id, raw_boxes(id)))
        .collect();
    let zip = phone.export();
    let zip = zip.to_str().unwrap();
    let lab = Lab::importing(Path::new(zip));
    let sessions = lab.dirs.sessions.display().to_string();

    let args = [
        "compare",
        zip,
        "--sessions-dir",
        &sessions,
        "--player",
        "aaron",
    ];
    let out = bin(&args);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("Corpus of aaron: equal"),
        "{}",
        stdout(&out)
    );
    assert!(
        stdout(&out).starts_with(&format!(
            "golf-bay-report compare: {zip} against {sessions}\n"
        )),
        "{}",
        stdout(&out)
    );

    lab.edit_manifest(card(&outcomes[2]), |manifest| {
        manifest["club"] = json!("8i");
    });
    let out = bin(&args);
    assert_eq!(code(&out), 3, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("first at swings[2].club"),
        "{}",
        stdout(&out)
    );

    // The last of a flag given twice, as `bars`' flags have it.
    let out = bin(&[
        "compare",
        zip,
        "--sessions-dir",
        "/no/such/dir",
        "--sessions-dir",
        &sessions,
        "--player",
        "aaron",
    ]);
    assert_eq!(code(&out), 3, "{}", stderr(&out));

    let mut json_args = args.to_vec();
    json_args.push("--json");
    let out = bin(&json_args);
    assert_eq!(code(&out), 3, "{}", stderr(&out));
    let json: Value = serde_json::from_str(&stdout(&out)).expect("JSON");
    assert_eq!(json["difference"]["path"], "swings[2].club");
    assert_eq!(json["difference"]["in_other"], "8i");
}

#[test]
fn the_compare_command_exits_2_on_a_refusal() {
    let phone = Phone::new();
    phone.six_logged();
    let zip = phone.export();
    let zip = zip.to_str().unwrap();
    let lab = Lab::importing(Path::new(zip));
    let sessions = lab.dirs.sessions.display().to_string();
    for (args, said) in [
        (
            vec!["compare", zip, "--player", "aaron"],
            "--sessions-dir is required",
        ),
        (
            vec!["compare", zip, "--sessions-dir", &sessions],
            "--player is required",
        ),
        (
            vec![
                "compare",
                zip,
                "--sessions-dir",
                &sessions,
                "--player",
                "arron",
            ],
            "nothing was compared",
        ),
    ] {
        let out = bin(&args);
        assert_eq!(code(&out), 2, "{args:?}");
        assert!(
            stderr(&out).starts_with("golf-bay-report compare: "),
            "{}",
            stderr(&out)
        );
        assert!(stderr(&out).contains(said), "{args:?}: {}", stderr(&out));
    }
}

/// `bars` keeps its rule: a verdict is an answer and not a failure, so a `MISS` still exits 0.
#[test]
fn every_verb_is_named_by_help_and_a_bad_verb_exits_1() {
    let out = bin(&["--help"]);
    assert_eq!(code(&out), 0);
    for verb in ["bars", "truth", "compare"] {
        assert!(
            stdout(&out).contains(&format!("golf-bay-report {verb} ")),
            "{}",
            stdout(&out)
        );
    }
    assert_eq!(code(&bin(&["nonsense"])), 1);
    assert_eq!(code(&bin(&[])), 1);
}
