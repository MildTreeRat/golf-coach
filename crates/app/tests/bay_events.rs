//! `golf-bay-report events`: what the app's own logs say about a session, proven on a log worked by
//! hand. [M38 P30c]
//!
//! **The fixture's numbers are worked in [`sitting`]'s comment**, and every number pinned below comes
//! from that comment and not from running this. Both logs are written by the real appenders
//! (`log_event`, `log_read`), so what is read is the layout the phone writes, and only the times and
//! details are chosen. Nothing here is golf and nothing is judged (rule 5), so no capture is read.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use app::api::export::export_sessions;
use app::api::record::{
    log_event, log_read, AppEvent, EventKind, ReadLog, ReadSource, RecordStatus,
};
use app::api::setup::{create_golfer, get_state, init_state, set_state, Handedness};
use app::bay::{events_report, render_events, EventsReport, Failure, Retake, Spread};
use app::parse_now;
use serde_json::Value;
use tempfile::TempDir;

const SESSION: &str = "2026-11-08-phone-a1b2";

/// A clock time on the bay's day, in the phone's offset.
fn at(clock: &str) -> String {
    format!("2026-11-08T{clock}-05:00")
}

/// `<session>/<swing>`, as a shot's kind leads its detail.
fn shot(swing: u32) -> String {
    format!("{SESSION}/{swing}")
}

/// A digest of one repeated hex digit, so each photo's label says which it is.
fn digest(digit: char) -> String {
    digit.to_string().repeat(64)
}

struct Phone {
    dir: TempDir,
    root: String,
}

impl Phone {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("a scratch directory");
        let root = dir.path().join("processed").display().to_string();
        Phone { dir, root }
    }

    fn root(&self) -> PathBuf {
        PathBuf::from(&self.root)
    }

    fn event(&self, clock: &str, kind: EventKind, detail: &str) {
        let event = AppEvent {
            at: at(clock),
            kind,
            detail: detail.to_string(),
        };
        log_event(self.root.clone(), event).expect("an event");
    }

    fn read(&self, clock: &str, digit: char, source: ReadSource, status: RecordStatus) {
        let read = ReadLog {
            at: at(clock),
            photo_sha256: digest(digit),
            source,
            status,
            boxes: 42,
            ms_vision: 1500,
            ms_rust: 20,
            ms_total: 1600,
        };
        log_read(self.root.clone(), read).expect("a read");
    }

    fn report(&self) -> EventsReport {
        events_report(&self.root(), None).expect("a report")
    }
}

/// One sitting, written in the order Dart would write it, and its numbers worked by hand.
///
/// ```text
/// events (21)                                     reads (8)
/// 10:00:00 app_start ready
/// 10:00:05 club_changed 7i
/// 10:00:10 camera_opened 7i
///                                                 10:00:20 camera  a read
/// 10:00:40 camera_opened 7i
/// 10:00:45 camera_cancelled 7i
/// 10:01:00 camera_opened 7i
///                                                 10:01:10 camera  b flagged
/// 10:01:11 review_opened S/2, forced
/// 10:01:31 review_saved S/2, 1 edit               (review 20 s)
/// 10:02:00 camera_opened 7i
///                                                 10:02:10 camera  c failed
/// 10:02:30 error, three lines
/// 10:02:40 camera_opened 7i
///                                                 10:02:50 camera  d read   (c's retake, 40 s on)
/// 10:03:00 review_opened S/3                      (left: S/1 opens next)
/// 10:03:05 review_opened S/1
/// 10:03:09 shot_deleted S/1                       (ended by a delete)
/// 10:04:00 club_changed pw
/// 10:04:10 camera_opened pw
///                                                 10:05:20 camera  e read
/// 10:05:30 review_opened S/4
/// 10:05:35 review_saved S/4, 0 edits              (review 5 s)
/// 10:06:00 app_start ready                        (a relaunch)
/// 10:06:10 export_shared completed true, 4 shots
/// 10:06:55 library_picked 2 photos, pw
///                                                 10:07:00 library f failed (no retake asked)
///                                                 10:07:03 library a duplicate
///                                                 10:08:00 camera  9 failed (nothing follows)
/// ```
///
/// Kinds: app_start 2, club_changed 2, camera_opened 6, camera_cancelled 1, library_picked 1,
/// review_opened 4, review_saved 2, shot_deleted 1, export_shared 1, error 1, the other four 0: 21.
/// Recorded shots a, b, d, e: gaps 50, 100 and 150 s, so p50 is rank ceil(1.5) = 2, 100 s, and p95 rank
/// ceil(2.85) = 3, 150 s, the max. Reviews saved in 20 s and 5 s: p50 rank ceil(1.0) = 1 of [5, 20], 5 s,
/// and p95 rank ceil(1.9) = 2, 20 s.
fn sitting(phone: &Phone) {
    use EventKind as K;
    use ReadSource::{Camera, Library};
    use RecordStatus::{Duplicate, Failed, Flagged, Read};
    phone.event("10:00:00", K::AppStart, "ready");
    phone.event("10:00:05", K::ClubChanged, "7i");
    phone.event("10:00:10", K::CameraOpened, "7i");
    phone.read("10:00:20", 'a', Camera, Read);
    phone.event("10:00:40", K::CameraOpened, "7i");
    phone.event("10:00:45", K::CameraCancelled, "7i");
    phone.event("10:01:00", K::CameraOpened, "7i");
    phone.read("10:01:10", 'b', Camera, Flagged);
    phone.event("10:01:11", K::ReviewOpened, &format!("{}, forced", shot(2)));
    phone.event("10:01:31", K::ReviewSaved, &format!("{}, 1 edit", shot(2)));
    phone.event("10:02:00", K::CameraOpened, "7i");
    phone.read("10:02:10", 'c', Camera, Failed);
    phone.event("10:02:30", K::Error, ERROR);
    phone.event("10:02:40", K::CameraOpened, "7i");
    phone.read("10:02:50", 'd', Camera, Read);
    phone.event("10:03:00", K::ReviewOpened, &shot(3));
    phone.event("10:03:05", K::ReviewOpened, &shot(1));
    phone.event("10:03:09", K::ShotDeleted, &shot(1));
    phone.event("10:04:00", K::ClubChanged, "pw");
    phone.event("10:04:10", K::CameraOpened, "pw");
    phone.read("10:05:20", 'e', Camera, Read);
    phone.event("10:05:30", K::ReviewOpened, &shot(4));
    phone.event("10:05:35", K::ReviewSaved, &format!("{}, 0 edits", shot(4)));
    phone.event("10:06:00", K::AppStart, "ready");
    phone.event("10:06:10", K::ExportShared, "completed true, 4 shots");
    phone.event("10:06:55", K::LibraryPicked, "2 photos, pw");
    phone.read("10:07:00", 'f', Library, Failed);
    phone.read("10:07:03", 'a', Library, Duplicate);
    phone.read("10:08:00", '9', Camera, Failed);
}

const ERROR: &str =
    "record: The photo could not be stored.\n#0 RecordFlow.takePhoto\n#1 AppModel.record";

fn sitting_phone() -> Phone {
    let phone = Phone::new();
    sitting(&phone);
    phone
}

fn count(report: &EventsReport, kind: &str) -> u64 {
    report
        .kinds
        .iter()
        .find(|row| row.kind == kind)
        .unwrap_or_else(|| panic!("no row for {kind}"))
        .count
}

fn spread(p50: i64, p95: i64, max: i64, intervals: u64) -> Spread {
    Spread {
        intervals,
        p50_ms: Some(p50),
        p95_ms: Some(p95),
        max_ms: Some(max),
    }
}

// --- the hand-worked sitting ---------------------------------------------------------------------

#[test]
fn the_sitting_counts_every_kind_a_zero_included_in_the_enums_order() {
    let report = sitting_phone().report();
    assert!(report.has_events);
    assert_eq!(report.events, 21);
    let kinds: Vec<(&str, u64)> = report
        .kinds
        .iter()
        .map(|row| (row.kind.as_str(), row.count))
        .collect();
    assert_eq!(
        kinds,
        [
            ("app_start", 2),
            ("club_changed", 2),
            ("mode_changed", 0),
            ("camera_opened", 6),
            ("camera_cancelled", 1),
            ("library_picked", 1),
            ("review_opened", 4),
            ("review_saved", 2),
            ("shot_deleted", 1),
            ("club_set", 0),
            ("intent_set", 0),
            ("mishit_set", 0),
            ("export_shared", 1),
            ("error", 1),
        ]
    );
    assert_eq!(report.first_event.as_deref(), Some(at("10:00:00").as_str()));
    assert_eq!(report.last_event.as_deref(), Some(at("10:06:55").as_str()));
    assert!(report.notes.is_empty(), "{:?}", report.notes);
}

#[test]
fn the_sitting_keeps_each_error_verbatim_with_its_time_and_each_launch() {
    let report = sitting_phone().report();
    assert_eq!(report.errors.len(), 1);
    assert_eq!(report.errors[0].at, at("10:02:30"));
    assert_eq!(report.errors[0].detail, ERROR);
    let launches: Vec<&str> = report.launches.iter().map(|l| l.at.as_str()).collect();
    assert_eq!(launches, [at("10:00:00"), at("10:06:00")]);
}

#[test]
fn the_sitting_comes_to_the_hand_worked_camera_and_reviews() {
    let report = sitting_phone().report();
    assert_eq!(report.camera.opened, 6);
    assert_eq!(report.camera.cancelled, 1);
    assert_eq!(report.camera.cancels[0].at, at("10:00:45"));
    assert_eq!(report.camera.cancels[0].detail, "7i");

    let reviews = &report.reviews;
    assert_eq!(
        (reviews.opened, reviews.saved, reviews.deleted, reviews.left),
        (4, 2, 1, 1)
    );
    assert_eq!(reviews.time, spread(5_000, 20_000, 20_000, 2));
}

#[test]
fn the_sitting_comes_to_the_hand_worked_gaps_between_recorded_shots() {
    let shots = sitting_phone().report().shots;
    assert_eq!(
        (shots.lines, shots.recorded, shots.failed, shots.duplicate),
        (8, 4, 3, 1)
    );
    assert_eq!(shots.between, spread(100_000, 150_000, 150_000, 3));
}

#[test]
fn each_failed_photo_stands_beside_the_retake_that_followed_it() {
    let failures = sitting_phone().report().failures;
    let photo = |digit: char| format!("photo {}", digit.to_string().repeat(12));
    assert_eq!(
        failures,
        [
            Failure {
                at: at("10:02:10"),
                photo: photo('c'),
                source: "camera".to_string(),
                sought: true,
                retake: Some(Retake {
                    at: at("10:02:50"),
                    photo: photo('d'),
                    status: "read".to_string(),
                    after_ms: 40_000,
                }),
            },
            Failure {
                at: at("10:07:00"),
                photo: photo('f'),
                source: "library".to_string(),
                sought: false,
                retake: None,
            },
            Failure {
                at: at("10:08:00"),
                photo: photo('9'),
                source: "camera".to_string(),
                sought: true,
                retake: None,
            },
        ]
    );
}

#[test]
fn the_sitting_prints_each_number_a_person_reads() {
    let text = render_events(&sitting_phone().report());
    for expected in [
        format!(
            "Events: 21 lines, from {} to {}.\n",
            at("10:00:00"),
            at("10:06:55")
        ),
        "  camera_opened     6\n".to_string(),
        "  mode_changed      0\n".to_string(),
        "  error             1\n".to_string(),
        format!(
            "Launches: 2. A second or later is a relaunch: the app was killed, or it crashed.\n  {}  ready\n  {}  ready\n",
            at("10:00:00"),
            at("10:06:00")
        ),
        format!(
            "Errors: 1, each as logged.\n  {}\n      record: The photo could not be stored.\n      #0 RecordFlow.takePhoto\n      #1 AppModel.record\n",
            at("10:02:30")
        ),
        format!(
            "Camera: 6 opened, 1 cancelled.\n  cancelled {}  7i\n",
            at("10:00:45")
        ),
        "Reviews: 4 opened: 2 saved, 1 ended by a delete, 1 left with neither.\n  opened to saved: p50 5.0 s, p95 20.0 s, max 20.0 s (2 review(s))\n".to_string(),
        "Shots: 8 read lines: 4 recorded (read or flagged), 3 failed, 1 duplicate.\n  between consecutive recorded shots: p50 100.0 s, p95 150.0 s, max 150.0 s (3 gap(s))\n".to_string(),
        "Failed photos: 3.\n".to_string(),
        format!(
            "  {}  camera  photo cccccccccccc: retaken {} as photo dddddddddddd, read, 40.0 s later\n",
            at("10:02:10"),
            at("10:02:50")
        ),
        format!(
            "  {}  library photo ffffffffffff: a library photo has no retake\n",
            at("10:07:00")
        ),
        format!(
            "  {}  camera  photo 999999999999: no camera photo followed\n",
            at("10:08:00")
        ),
    ] {
        assert!(text.contains(&expected), "missing {expected:?} in\n{text}");
    }
    assert!(!text.contains("Notes:"), "{text}");
}

// --- --since, and what an export may lack --------------------------------------------------------

/// `--since` in another offset names the same instant, and a line at exactly it is kept, in both logs.
/// It is printed as it was given, as `bars` prints it.
#[test]
fn since_leaves_out_the_earlier_lines_of_both_logs_by_instant() {
    let phone = sitting_phone();
    let since = parse_now("2026-11-08T15:04:00Z").expect("a time");
    let report = events_report(&phone.root(), Some(since)).expect("a report");
    assert_eq!(report.since.as_deref(), Some("2026-11-08T15:04:00Z"));
    assert_eq!(report.events, 7);
    assert_eq!(report.first_event.as_deref(), Some(at("10:04:00").as_str()));
    assert_eq!(count(&report, "club_changed"), 1);
    assert_eq!(count(&report, "camera_opened"), 1);
    assert_eq!(count(&report, "error"), 0);
    assert_eq!(report.launches.len(), 1);
    assert_eq!(
        (
            report.reviews.opened,
            report.reviews.saved,
            report.reviews.left
        ),
        (1, 1, 0)
    );
    assert_eq!(report.reviews.time, spread(5_000, 5_000, 5_000, 1));
    assert_eq!((report.shots.lines, report.shots.recorded), (4, 1));
    assert_eq!(report.shots.between.intervals, 0);
    assert_eq!(report.shots.between.p50_ms, None);
    let failed: Vec<&str> = report.failures.iter().map(|f| f.at.as_str()).collect();
    assert_eq!(failed, [at("10:07:00"), at("10:08:00")]);
    assert_eq!(
        report.notes,
        [
            "14 event line(s) before --since are in no count.",
            "4 read line(s) before --since are in no count.",
        ]
    );
    let text = render_events(&report);
    assert!(
        text.contains("Events since 2026-11-08T15:04:00Z: 7 lines"),
        "{text}"
    );
    assert!(
        text.contains("between consecutive recorded shots: none to measure (0 gap(s))"),
        "{text}"
    );
}

/// P30b finding 7: an export cut before P30b has no events' log, and that is said rather than failed on.
#[test]
fn an_export_with_no_events_log_says_so_and_still_reads_the_reads() {
    let phone = Phone::new();
    phone.read("10:00:20", 'a', ReadSource::Camera, RecordStatus::Read);
    phone.read("10:01:20", 'b', ReadSource::Camera, RecordStatus::Read);
    let report = phone.report();
    assert!(!report.has_events);
    assert_eq!(report.events, 0);
    assert_eq!(report.shots.between, spread(60_000, 60_000, 60_000, 1));
    assert_eq!(report.notes.len(), 1);
    assert!(
        report.notes[0].starts_with("There is no .app/events.jsonl"),
        "{:?}",
        report.notes
    );
    let text = render_events(&report);
    assert!(
        text.contains("Events: this export holds no .app/events.jsonl"),
        "{text}"
    );
    assert!(!text.contains("Errors:"), "{text}");
}

#[test]
fn an_export_with_no_reads_log_says_so_and_still_reads_the_events() {
    let phone = Phone::new();
    phone.event("10:00:00", EventKind::AppStart, "ready");
    let report = phone.report();
    assert_eq!(report.events, 1);
    assert_eq!(report.shots.lines, 0);
    assert_eq!(
        report.notes,
        ["There is no .app/reads.jsonl, so no shot, gap or failed photo can be counted."]
    );
}

/// A crash can tear the last line, and a report refused over it would cost the session's evidence.
#[test]
fn a_line_that_does_not_parse_is_a_note_naming_its_number_and_in_no_count() {
    let phone = sitting_phone();
    let path = phone.root().join(".app/events.jsonl");
    let mut log = fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("the log");
    log.write_all(b"{\"at\":\"2026-11-08T10:09:00-05:00\",\"kind\":\"err")
        .expect("a torn line");
    drop(log);
    let report = phone.report();
    assert_eq!(report.events, 21);
    assert_eq!(report.notes.len(), 1);
    assert!(
        report.notes[0]
            .starts_with("Line 22 of events.jsonl is not an event, and is in no count: "),
        "{:?}",
        report.notes
    );
}

/// Each log is taken by its times, so a review is paired with its save whatever order the lines landed in.
#[test]
fn lines_are_taken_in_the_order_of_their_times_and_not_of_the_file() {
    let phone = Phone::new();
    phone.event(
        "10:00:30",
        EventKind::ReviewSaved,
        &format!("{}, 1 edit", shot(1)),
    );
    phone.event("10:00:00", EventKind::ReviewOpened, &shot(1));
    let report = phone.report();
    assert_eq!(
        (
            report.reviews.opened,
            report.reviews.saved,
            report.reviews.left
        ),
        (1, 1, 0)
    );
    assert_eq!(report.reviews.time, spread(30_000, 30_000, 30_000, 1));
    assert_eq!(report.first_event.as_deref(), Some(at("10:00:00").as_str()));
}

/// A delete from a shot's detail screen ends no review, and a save naming another shot is noted.
#[test]
fn only_a_save_or_delete_naming_the_open_review_ends_it() {
    let phone = Phone::new();
    phone.event("10:00:00", EventKind::ReviewOpened, &shot(1));
    phone.event("10:00:05", EventKind::ShotDeleted, &shot(2));
    phone.event(
        "10:00:09",
        EventKind::ReviewSaved,
        &format!("{}, 0 edits", shot(3)),
    );
    let report = phone.report();
    let reviews = &report.reviews;
    assert_eq!(
        (reviews.opened, reviews.saved, reviews.deleted, reviews.left),
        (1, 0, 0, 1)
    );
    assert_eq!(reviews.time.intervals, 0);
    assert_eq!(
        report.notes,
        [
            "There is no .app/reads.jsonl, so no shot, gap or failed photo can be counted.",
            "1 review_saved line(s) name no open review, and are in no review's count or time.",
        ]
    );
}

// --- a zip, and the command ----------------------------------------------------------------------

/// The real `export_sessions` cuts the zip, so this is the file P32 will be handed.
#[test]
fn a_zip_and_a_directory_are_the_same_report() {
    let phone = sitting_phone();
    init_state(phone.root.clone(), "a1b2".to_string()).expect("an install id");
    create_golfer(
        phone.root.clone(),
        "Aaron".to_string(),
        Handedness::Right,
        at("09:00:00"),
    )
    .expect("a golfer");
    let mut state = get_state(phone.root.clone());
    state.player_id = Some("aaron".to_string());
    set_state(phone.root.clone(), state).expect("a state");
    let zip = phone.dir.path().join("export.zip");
    export_sessions(
        phone.root.clone(),
        zip.display().to_string(),
        at("11:00:00"),
    )
    .expect("an export");
    let from_zip = events_report(&zip, None).expect("a report from the zip");
    assert_eq!(from_zip, phone.report());
    assert_eq!(from_zip.events, 21);
}

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_golf-bay-report"))
}

#[test]
fn the_command_prints_the_report_and_exits_0_since_it_judges_nothing() {
    let phone = sitting_phone();
    let out = bin()
        .arg("events")
        .arg(phone.root())
        .output()
        .expect("a run");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout).expect("text");
    assert!(text.starts_with(&format!(
        "golf-bay-report events: {}\n\nEvents: 21 lines",
        phone.root().display()
    )));
    assert!(text.contains("Errors: 1, each as logged."), "{text}");
}

#[test]
fn the_command_writes_json_and_takes_since() {
    let phone = sitting_phone();
    let out = bin()
        .arg("events")
        .arg(phone.root())
        .args(["--since", "2026-11-08T10:04:00-05:00", "--json"])
        .output()
        .expect("a run");
    assert!(out.status.success());
    let json: Value = serde_json::from_slice(&out.stdout).expect("JSON");
    assert_eq!(json["events"], 7);
    assert_eq!(json["reviews"]["saved"], 1);
    assert_eq!(json["shots"]["between"]["p50_ms"], Value::Null);
}

#[test]
fn the_command_refuses_a_naive_since_an_unknown_flag_and_no_export_with_2() {
    let phone = sitting_phone();
    let root = phone.root();
    for args in [
        vec!["--since", "2026-11-08T10:04:00"],
        vec!["--source", "camera"],
        vec![],
    ] {
        let mut command = bin();
        command.arg("events");
        // The last case has no export at all.
        if !args.is_empty() {
            command.arg(&root);
        }
        let out = command.args(&args).output().expect("a run");
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(
            String::from_utf8_lossy(&out.stderr).starts_with("golf-bay-report events: "),
            "{args:?}"
        );
    }
}
