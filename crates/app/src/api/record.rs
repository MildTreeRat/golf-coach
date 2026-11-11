//! One photo and the boxes Swift read from it, to a stored, analysed shot and the card showing it, and
//! the diagnostics line the bay report reads. [M38 P6] And since P30b the other log beside it: what the
//! person did in the app, and what went wrong inside it (D16).
//!
//! The shapes Dart sees and three one-line calls (rule 7). `crate::record` holds what they do, in what
//! order and why, and `crate::card` how a shot becomes a card.

use serde::{Deserialize, Serialize};

use crate::api::setup::TargetShape;

/// One photo to record.
#[derive(Debug, Clone, PartialEq)]
pub struct PhotoIn {
    /// The data root, `<Documents>/processed/` on the phone (C7).
    pub root: String,
    /// The photo as Swift left it. Its bytes are what is hashed and stored, and its file name is what
    /// the manifest records as the original, so the name's suffix picks the stored file's extension.
    ///
    /// **Only a `Failed` read moves this file** (C13), and every other answer leaves it where it is.
    pub path: String,
    /// `ScreenReader`'s text, passed through unparsed, because a Dart double would be one more place
    /// a colour's bits could move (rule 9). Either a list of boxes, or an object with a `boxes` key
    /// beside whatever else the reader says about the read, which is the shape M33's committed files
    /// have. Stored as given, whichever it is.
    pub boxes_json: String,
    /// When the photo was taken, RFC 3339 with an offset: Swift's answer (C9).
    pub captured_at: String,
    /// When it was recorded, RFC 3339 with an offset.
    pub now: String,
    /// The shape a challenge called, recorded as `{shot_shaping, call}` over whatever `state.json`'s
    /// mode says (C15). `None` is every other shot.
    pub challenge_call: Option<TargetShape>,
}

/// What became of a photo, spelled as `reads.jsonl` and the bay's bars spell it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordStatus {
    /// Stored and analysed, with nothing asking for a person's eye.
    Read,
    /// Stored and analysed, and it must be reviewed before the next photo (D9).
    Flagged,
    /// A photo already held. Nothing was opened, and the card is the held shot's.
    Duplicate,
    /// Nothing was read, and nothing is left but the diagnostics (C13).
    Failed,
}

/// The answer to one photo, and what `log_read`'s line needs that Dart cannot work out: it never parses
/// the boxes and never hashes the photo.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordOutcome {
    pub status: RecordStatus,
    /// The stored shot's, or the held one's. `None` only on `Failed`.
    pub card: Option<ShotCard>,
    /// What to say above the card: `Failed`'s and `Duplicate`'s sentence.
    pub message: Option<String>,
    pub photo_sha256: String,
    /// How many boxes the reader returned.
    pub boxes: i64,
    /// This call, start to answer, a wait on another photo included.
    pub ms_rust: i64,
}

/// One read as a person sees it (D9).
#[derive(Debug, Clone, PartialEq)]
pub struct ShotCard {
    pub session_id: String,
    pub swing_id: String,
    /// The stored `needs_review`, which is what forces a review.
    pub flagged: bool,
    pub confidence: f64,
    /// `screen::golfer_warnings`, so never the line about the tile layout nobody can act on.
    pub warnings: Vec<String>,
    pub rows: Vec<CardRow>,
}

/// One tile on a card, by `crate::card`'s rules.
#[derive(Debug, Clone, PartialEq)]
pub struct CardRow {
    /// The `ShotData` key.
    pub field: String,
    /// The screen's own label, which names the unit `display` is in.
    pub label: String,
    /// As stored, in `ShotData`'s units and with its sign. `None` was not read, and is never 0.
    pub value: Option<f64>,
    /// As the screen printed it, with the lit letter where the tile has one (`2.2 R`).
    pub display: Option<String>,
    /// `display`'s number alone (`2.2`, and `85.2` under `Apex Ft`): what a review's number field
    /// starts from, and what `correct_shot` takes back, in the screen's unit and not `value`'s.
    pub printed: Option<String>,
    /// `display`'s letter alone (`R`), where one is lit: what a review's sign control starts on. `None`
    /// on a tile with no letters, on one not read, and on a zero, which needs none.
    pub letter: Option<String>,
    pub state: RowState,
    /// The tile's two letters, the negative's first, where it is signed by a lit one.
    pub sign_letters: Option<(String, String)>,
}

/// Whether a row holds what was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowState {
    Read,
    NotRead,
    Corrected,
}

/// Where a photo came from, as the bay's bars ask it: T5 is over camera photos alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadSource {
    Camera,
    Library,
    Scanner,
}

/// One line of `.app/reads.jsonl`, in the order it is written.
///
/// `ms_total` runs from the shutter's image reaching the app to the card's first frame, so only Dart
/// can measure it, which is why this is a call of its own after `record_photo` has answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadLog {
    /// RFC 3339 with an offset.
    pub at: String,
    pub photo_sha256: String,
    pub source: ReadSource,
    pub status: RecordStatus,
    pub boxes: i64,
    pub ms_vision: i64,
    pub ms_rust: i64,
    pub ms_total: i64,
}

/// Store one photo and what was read from it, and answer the card (`crate::record`'s doc).
///
/// `Err` is a refusal or a failure, and not a read of nothing, which is `Failed`: nothing to stamp the
/// shot with, a time with no offset or a photo that cannot be read leaves no file behind. Boxes that are
/// not boxes are kept, since they are the one diagnostic of a reader gone wrong, and open no entry.
pub fn record_photo(photo: PhotoIn) -> Result<RecordOutcome, String> {
    crate::record::record(photo)
}

/// Append one line to `.app/reads.jsonl`, which the bay report reads (P14). Refuses a time with no
/// offset, as every time crossing from Dart is.
pub fn log_read(root: String, read: ReadLog) -> Result<(), String> {
    crate::record::append_read(&root, &read)
}

/// What a person did in the app, or what went wrong inside it, as `.app/events.jsonl` spells it (D16).
/// [M38 P30b]
///
/// **The person's actions, and nothing the stores already say**: a recorded photo is `reads.jsonl`'s
/// line, so it has no kind here, and a cancel, a review or a share leaves nothing in the stores at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// The app read its state and chose the screen it opens on. A second one mid-session is a relaunch.
    AppStart,
    /// The club the next photo is stamped with, from the change sheet or a golfer change clearing it.
    ClubChanged,
    /// The mode, its shape, or the challenge and its ticks, from the change sheet.
    ModeChanged,
    /// The shutter was tapped and the camera shown.
    CameraOpened,
    /// The camera was closed with no photo.
    CameraCancelled,
    /// The library picker answered, with any number of photos, none included.
    LibraryPicked,
    /// A review was put in front of the person, forced or not.
    ReviewOpened,
    /// A review was saved, which is what releases a flagged shot.
    ReviewSaved,
    /// An entry was deleted, from a review or a shot's detail.
    ShotDeleted,
    /// A shot's club was changed after it was recorded.
    ClubSet,
    /// A shot's intent was changed after it was recorded, from a review or a shot's detail.
    IntentSet,
    /// A shot's mishit verdict was set or handed back to the rule.
    MishitSet,
    /// The share sheet closed on an export, finished or not.
    ExportShared,
    /// Something went wrong: an uncaught error, or a refusal the person was shown.
    Error,
}

/// One line of `.app/events.jsonl`, in the order it is written. [M38 P30b]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppEvent {
    /// RFC 3339 with an offset: when it happened, by Dart's clock, which is not always when it was
    /// written, since Dart writes each without waiting on it.
    pub at: String,
    pub kind: EventKind,
    /// Short text for a person, naming the subject (`7i`, `3 photos`, an error's message and its
    /// stack's first lines). Stored as given: it is not golf, and nothing parses it.
    pub detail: String,
}

/// Append one line to `.app/events.jsonl`, beside `log_read`'s, through the same appender. Refuses a
/// time with no offset. [M38 P30b]
pub fn log_event(root: String, event: AppEvent) -> Result<(), String> {
    crate::record::append_event(&root, &event)
}
