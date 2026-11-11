//! `golf-bay-report events`: what the app's own logs say about a session, read beside `bars`. [M38 P30c]
//!
//! D16 asks the bay to show "what went well and what did not", and `bars` says only what was read. So
//! this reads `.app/events.jsonl` (P30b), what the person did and every error the app met, beside
//! `.app/reads.jsonl` (P6), what became of each photo. **It judges nothing**: there is no bar here and
//! `m38-bay-thresholds.md` is not opened (rule 5). The bars stay the verdict, and these are the numbers
//! a person reads beside it. Both logs go through `bars`' reader and `--since`, so a torn line is a note
//! naming its number and a rehearsal is left out the same way in both.
//!
//! # How a line is read (P30b finding 7)
//!
//! - **A `detail` is printed and never parsed**, being free text for a person, with one exception: a
//!   shot's kind leads with `<session>/<swing>` up to the first `,` or space, and that is how a review is
//!   paired with what ended it. An `error`'s detail spans lines, so it is printed indented under its time.
//! - **Each log is taken in the order of its times, by instant**, a stable sort, so lines written at one
//!   instant keep their order. Dart writes events without awaiting them and `reads.jsonl`'s `at` trails
//!   the card's first frame, so the two logs are never interleaved here: each number below is one log's.
//! - **A review is `review_opened` to the next `review_saved` or `shot_deleted` naming the same shot,
//!   before another `review_opened`.** A review that is not forced can be backed out of, which writes
//!   nothing, so an open with neither is counted as left. Only a saved review has a time, since a delete
//!   ends a review without one.
//! - **The time between recorded shots is between consecutive `read` and `flagged` lines**, every such
//!   line and not one per photo as `bars` counts them, because this is the session's pace and a photo
//!   recorded afresh after a delete was recorded twice. A `failed` or `duplicate` line opens no entry.
//! - **A failed photo's retake is the next line from the same source**, since the run sheet says to
//!   retake a failure and a camera photo never repeats its bytes. A library photo has no retake: the next
//!   one in a batch is the next photo, and is not asked for again.
//! - **`export_shared` is written once the share sheet closes, after its zip was cut**, so a zip never
//!   holds its own share's line, and the report says so beside the count.
//! - Times between two lines are whole milliseconds, truncated, and the percentiles are `bars`' nearest
//!   rank, so p50 and p95 are each a time that happened.

use std::path::Path;

use contracts::Timestamp;
use serde::Serialize;

use super::{entries_by_digest, lines_since, percentile, photo_label, read_jsonl, Export, LogLine};
use crate::api::record::{AppEvent, EventKind, ReadLog, ReadSource, RecordStatus};
use crate::record::spelled;
use crate::{parse_now, Dirs};

/// Every kind, in `EventKind`'s order, so the counts show a zero rather than leave a kind out.
const KINDS: [EventKind; 14] = [
    EventKind::AppStart,
    EventKind::ClubChanged,
    EventKind::ModeChanged,
    EventKind::CameraOpened,
    EventKind::CameraCancelled,
    EventKind::LibraryPicked,
    EventKind::ReviewOpened,
    EventKind::ReviewSaved,
    EventKind::ShotDeleted,
    EventKind::ClubSet,
    EventKind::IntentSet,
    EventKind::MishitSet,
    EventKind::ExportShared,
    EventKind::Error,
];

/// A kind's place in [`KINDS`]. **The match has no wildcard**, so a kind added to `EventKind` does not
/// compile until it is placed, and a unit test holds each place to its row.
fn place(kind: EventKind) -> usize {
    match kind {
        EventKind::AppStart => 0,
        EventKind::ClubChanged => 1,
        EventKind::ModeChanged => 2,
        EventKind::CameraOpened => 3,
        EventKind::CameraCancelled => 4,
        EventKind::LibraryPicked => 5,
        EventKind::ReviewOpened => 6,
        EventKind::ReviewSaved => 7,
        EventKind::ShotDeleted => 8,
        EventKind::ClubSet => 9,
        EventKind::IntentSet => 10,
        EventKind::MishitSet => 11,
        EventKind::ExportShared => 12,
        EventKind::Error => 13,
    }
}

/// How many lines of one kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KindCount {
    pub kind: String,
    pub count: u64,
}

/// One event as logged, its time and its detail as written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Logged {
    pub at: String,
    pub detail: String,
}

/// Some intervals, in milliseconds: the nearest-rank p50 and p95, and the longest. Each `None` with no
/// interval to measure, which is never 0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Spread {
    pub intervals: u64,
    pub p50_ms: Option<i64>,
    pub p95_ms: Option<i64>,
    pub max_ms: Option<i64>,
}

impl Spread {
    fn of(values: &[i64]) -> Self {
        Spread {
            intervals: values.len() as u64,
            p50_ms: percentile(values, 50),
            p95_ms: percentile(values, 95),
            max_ms: values.iter().copied().max(),
        }
    }
}

/// The camera's opens and closes with no photo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Camera {
    pub opened: u64,
    pub cancelled: u64,
    /// Each `camera_cancelled`, in time order.
    pub cancels: Vec<Logged>,
}

/// The reviews, paired as the module doc says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reviews {
    pub opened: u64,
    pub saved: u64,
    /// Ended by deleting the shot under review.
    pub deleted: u64,
    /// Opened and ended by neither, which is a back-out.
    pub left: u64,
    /// `review_opened` to its `review_saved`.
    pub time: Spread,
}

/// What `reads.jsonl` says about the session's pace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Shots {
    pub lines: u64,
    /// Lines saying `read` or `flagged`, each a stored shot.
    pub recorded: u64,
    pub failed: u64,
    pub duplicate: u64,
    /// Between consecutive recorded shots.
    pub between: Spread,
}

/// The line that followed a failed photo from the same source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Retake {
    pub at: String,
    /// Its entry and the name it arrived under, as `bars` names a photo, or its digest if it has none.
    pub photo: String,
    pub status: String,
    pub after_ms: i64,
}

/// A failed photo and its retake.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Failure {
    pub at: String,
    /// Its digest, since a failed read keeps no name (C7).
    pub photo: String,
    pub source: String,
    /// `false` for a library photo, which has no retake (the module doc).
    pub sought: bool,
    /// `None` where none was sought or none followed.
    pub retake: Option<Retake>,
}

/// Everything `events` prints, or `--json` writes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventsReport {
    /// `--since`, the lines before it in either log being in no count.
    pub since: Option<String>,
    /// Whether the export holds `.app/events.jsonl` at all. An export cut before P30b does not, and
    /// then every event count below is not known, rather than none.
    pub has_events: bool,
    /// The event lines counted.
    pub events: u64,
    pub first_event: Option<String>,
    pub last_event: Option<String>,
    /// Every kind, in `EventKind`'s order, a zero included.
    pub kinds: Vec<KindCount>,
    /// Each `app_start`. A second or later in one sitting is a relaunch (P30b finding 7).
    pub launches: Vec<Logged>,
    /// Each `error`, verbatim.
    pub errors: Vec<Logged>,
    pub camera: Camera,
    pub reviews: Reviews,
    pub shots: Shots,
    pub failures: Vec<Failure>,
    /// What the logs hold that the counts above leave out, each a sentence.
    pub notes: Vec<String>,
}

/// Each line with its instant, in the order of those instants. A line whose `at` is no time cannot be
/// placed, and is a note: `log_read` and `log_event` refuse to write one, so it is a hand's or a tear's.
fn in_time_order<T: LogLine>(lines: Vec<T>, notes: &mut Vec<String>) -> Vec<(Timestamp, T)> {
    let mut timed = Vec::new();
    for line in lines {
        match parse_now(line.at()) {
            Ok(at) => timed.push((at, line)),
            Err(refused) => notes.push(format!(
                "A {} line at '{}' is in no count, since that is no time: {refused}.",
                T::NOUN,
                line.at()
            )),
        }
    }
    timed.sort_by_key(|(at, _)| *at);
    timed
}

/// From one instant to a later one, in whole milliseconds.
fn ms_between(from: Timestamp, to: Timestamp) -> i64 {
    (to.utc_micros() - from.utc_micros()) / 1000
}

/// The shot a kind's detail leads with, up to the first `,` or space (P30b finding 7).
fn subject(detail: &str) -> &str {
    detail.split([',', ' ']).next().unwrap_or("")
}

fn logged(event: &AppEvent) -> Logged {
    Logged {
        at: event.at.clone(),
        detail: event.detail.clone(),
    }
}

fn reviews_of(events: &[(Timestamp, AppEvent)], notes: &mut Vec<String>) -> Reviews {
    let mut reviews = Reviews {
        opened: 0,
        saved: 0,
        deleted: 0,
        left: 0,
        time: Spread::of(&[]),
    };
    let mut times = Vec::new();
    let mut unpaired = 0;
    let mut open: Option<(&str, Timestamp)> = None;
    for (at, event) in events {
        match event.kind {
            EventKind::ReviewOpened => {
                reviews.opened += 1;
                if open.replace((subject(&event.detail), *at)).is_some() {
                    reviews.left += 1;
                }
            }
            EventKind::ReviewSaved | EventKind::ShotDeleted => match open {
                Some((shot, from)) if shot == subject(&event.detail) => {
                    if event.kind == EventKind::ReviewSaved {
                        reviews.saved += 1;
                        times.push(ms_between(from, *at));
                    } else {
                        reviews.deleted += 1;
                    }
                    open = None;
                }
                // A delete from a shot's detail screen ends no review, and is only counted by kind.
                _ if event.kind == EventKind::ReviewSaved => unpaired += 1,
                _ => {}
            },
            _ => {}
        }
    }
    if open.is_some() {
        reviews.left += 1;
    }
    if unpaired > 0 {
        notes.push(format!(
            "{unpaired} review_saved line(s) name no open review, and are in no review's count or time."
        ));
    }
    reviews.time = Spread::of(&times);
    reviews
}

fn shots_of(reads: &[(Timestamp, ReadLog)]) -> Shots {
    let recorded: Vec<Timestamp> = reads
        .iter()
        .filter(|(_, line)| matches!(line.status, RecordStatus::Read | RecordStatus::Flagged))
        .map(|(at, _)| *at)
        .collect();
    let gaps: Vec<i64> = recorded
        .windows(2)
        .map(|pair| ms_between(pair[0], pair[1]))
        .collect();
    let with = |status: RecordStatus| {
        reads
            .iter()
            .filter(|(_, line)| line.status == status)
            .count() as u64
    };
    Shots {
        lines: reads.len() as u64,
        recorded: recorded.len() as u64,
        failed: with(RecordStatus::Failed),
        duplicate: with(RecordStatus::Duplicate),
        between: Spread::of(&gaps),
    }
}

fn failures_of(reads: &[(Timestamp, ReadLog)], dirs: &Dirs) -> Vec<Failure> {
    let entries = entries_by_digest(dirs);
    let label = |digest: &str| photo_label(entries.get(digest), digest);
    reads
        .iter()
        .enumerate()
        .filter(|(_, (_, line))| line.status == RecordStatus::Failed)
        .map(|(index, (at, line))| {
            let sought = line.source != ReadSource::Library;
            let next = reads[index + 1..]
                .iter()
                .find(|(_, next)| next.source == line.source);
            let retake = next.filter(|_| sought).map(|(next_at, next)| Retake {
                at: next.at.clone(),
                photo: label(&next.photo_sha256),
                status: spelled(next.status),
                after_ms: ms_between(*at, *next_at),
            });
            Failure {
                at: line.at.clone(),
                photo: label(&line.photo_sha256),
                source: spelled(line.source),
                sought,
                retake,
            }
        })
        .collect()
}

/// `golf-bay-report events`: what the export at `path` logged, from `since` if given.
pub fn events_report(path: &Path, since: Option<Timestamp>) -> Result<EventsReport, String> {
    let export = Export::open(path)?;
    let dirs = Dirs::of(export.root());
    let mut notes = Vec::new();

    let events = read_jsonl::<AppEvent>(&dirs, &mut notes)?;
    let has_events = events.is_some();
    if !has_events {
        notes.push(
            "There is no .app/events.jsonl, so this export was cut before the app kept one (P30b), and \
             only the shots and failed photos, from reads.jsonl, are known."
                .to_string(),
        );
    }
    let events = in_time_order(
        lines_since(&events.unwrap_or_default(), since, &mut notes),
        &mut notes,
    );
    let reads = read_jsonl::<ReadLog>(&dirs, &mut notes)?;
    if reads.is_none() {
        notes.push(
            "There is no .app/reads.jsonl, so no shot, gap or failed photo can be counted."
                .to_string(),
        );
    }
    let reads = in_time_order(
        lines_since(&reads.unwrap_or_default(), since, &mut notes),
        &mut notes,
    );

    let mut counts = [0_u64; KINDS.len()];
    for (_, event) in &events {
        counts[place(event.kind)] += 1;
    }
    let of_kind = |kind: EventKind| -> Vec<Logged> {
        events
            .iter()
            .filter(|(_, event)| event.kind == kind)
            .map(|(_, event)| logged(event))
            .collect()
    };
    let cancels = of_kind(EventKind::CameraCancelled);
    Ok(EventsReport {
        since: since.map(|since| since.to_string()),
        has_events,
        events: events.len() as u64,
        first_event: events.first().map(|(_, event)| event.at.clone()),
        last_event: events.last().map(|(_, event)| event.at.clone()),
        kinds: KINDS
            .iter()
            .zip(counts)
            .map(|(kind, count)| KindCount {
                kind: spelled(*kind),
                count,
            })
            .collect(),
        launches: of_kind(EventKind::AppStart),
        errors: of_kind(EventKind::Error),
        camera: Camera {
            opened: counts[place(EventKind::CameraOpened)],
            cancelled: cancels.len() as u64,
            cancels,
        },
        reviews: reviews_of(&events, &mut notes),
        shots: shots_of(&reads),
        failures: failures_of(&reads, &dirs),
        notes,
    })
}

// --- the text ------------------------------------------------------------------------------------

/// Milliseconds as seconds to a tenth, which is as fine as a person at the bay can act on.
fn seconds(ms: i64) -> String {
    format!("{:.1} s", ms as f64 / 1000.0)
}

fn spread(spread: &Spread, of: &str) -> String {
    match (spread.p50_ms, spread.p95_ms, spread.max_ms) {
        (Some(p50), Some(p95), Some(max)) => format!(
            "p50 {}, p95 {}, max {} ({} {of})",
            seconds(p50),
            seconds(p95),
            seconds(max),
            spread.intervals
        ),
        _ => format!("none to measure (0 {of})"),
    }
}

/// The report as a person reads it, and as P32 copies it into the results file.
pub fn render_events(report: &EventsReport) -> String {
    let mut out = String::new();
    let mut line = |text: String| {
        out.push_str(&text);
        out.push('\n');
    };
    let since = report
        .since
        .as_ref()
        .map_or_else(String::new, |since| format!(" since {since}"));

    if report.has_events {
        line(format!(
            "Events{since}: {} lines{}.",
            report.events,
            match (&report.first_event, &report.last_event) {
                (Some(first), Some(last)) => format!(", from {first} to {last}"),
                _ => String::new(),
            }
        ));
        for kind in &report.kinds {
            line(format!("  {:<17} {}", kind.kind, kind.count));
        }
        line(
            "  An export_shared line is written once its share sheet closes, after its zip was cut, so \
             this zip's own share is not among them."
                .to_string(),
        );
        line(String::new());

        line(format!(
            "Launches: {}. A second or later is a relaunch: the app was killed, or it crashed.",
            report.launches.len()
        ));
        for launch in &report.launches {
            line(format!("  {}  {}", launch.at, launch.detail));
        }
        line(String::new());

        line(format!("Errors: {}, each as logged.", report.errors.len()));
        for error in &report.errors {
            line(format!("  {}", error.at));
            for text in error.detail.lines() {
                line(format!("      {text}"));
            }
        }
        line(String::new());

        let camera = &report.camera;
        line(format!(
            "Camera: {} opened, {} cancelled.",
            camera.opened, camera.cancelled
        ));
        for cancel in &camera.cancels {
            line(format!("  cancelled {}  {}", cancel.at, cancel.detail));
        }
        line(String::new());

        let reviews = &report.reviews;
        line(format!(
            "Reviews: {} opened: {} saved, {} ended by a delete, {} left with neither.",
            reviews.opened, reviews.saved, reviews.deleted, reviews.left
        ));
        line(format!(
            "  opened to saved: {}",
            spread(&reviews.time, "review(s)")
        ));
        line(String::new());
    } else {
        line(format!(
            "Events{since}: this export holds no .app/events.jsonl, so what the person did and what \
             went wrong are not known."
        ));
        line(String::new());
    }

    let shots = &report.shots;
    line(format!(
        "Shots{since}: {} read lines: {} recorded (read or flagged), {} failed, {} duplicate.",
        shots.lines, shots.recorded, shots.failed, shots.duplicate
    ));
    line(format!(
        "  between consecutive recorded shots: {}",
        spread(&shots.between, "gap(s)")
    ));
    line(String::new());

    line(format!("Failed photos: {}.", report.failures.len()));
    for failure in &report.failures {
        let after = match (&failure.retake, failure.sought) {
            (Some(retake), _) => format!(
                "retaken {} as {}, {}, {} later",
                retake.at,
                retake.photo.trim_end(),
                retake.status,
                seconds(retake.after_ms)
            ),
            (None, true) => format!("no {} photo followed", failure.source),
            (None, false) => "a library photo has no retake".to_string(),
        };
        line(format!(
            "  {}  {:<7} {}: {after}",
            failure.at, failure.source, failure.photo
        ));
    }

    if !report.notes.is_empty() {
        line(String::new());
        line("Notes:".to_string());
        for note in &report.notes {
            line(format!("  - {note}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_is_placed_at_its_own_row() {
        for (row, kind) in KINDS.iter().enumerate() {
            assert_eq!(place(*kind), row, "{kind:?}");
        }
    }

    #[test]
    fn a_subject_runs_to_the_first_comma_or_space() {
        assert_eq!(
            subject("2026-11-08-phone-a1b2/3, forced, 2 of 3"),
            "2026-11-08-phone-a1b2/3"
        );
        assert_eq!(
            subject("2026-11-08-phone-a1b2/3 pw"),
            "2026-11-08-phone-a1b2/3"
        );
        assert_eq!(
            subject("2026-11-08-phone-a1b2/3"),
            "2026-11-08-phone-a1b2/3"
        );
        assert_eq!(subject(""), "");
    }
}
