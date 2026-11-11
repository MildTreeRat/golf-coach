//! What the phone's reads come to against a known truth, for the gates that run through the bridge
//! (P18) and the report that runs on the Mac. [M38 P15]
//!
//! The shapes Dart sees and one one-line call (rule 7). `crate::bay` holds what they do and why.

use serde::Serialize;

/// How a capture was found in the export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TruthRoute {
    /// Its bytes are the capture's.
    Digest,
    /// Under its name with other bytes: **a re-encoded copy**, as Photos may make of an AirDropped file.
    Name,
    /// Under its name, in more than one entry, so none was chosen and it holds no cells.
    Ambiguous,
    /// Logged as read, with no shot stored, so it was deleted at the phone and holds no cells.
    Deleted,
    /// Not in this export.
    Absent,
}

/// One field of one capture, M33's four words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TruthCellKind {
    /// What the reader gave is the truth, compared as IEEE doubles with no tolerance.
    Read,
    /// What the reader gave is not the truth, and nothing warned the golfer. Rule 11's hard stop.
    Wrong,
    /// What the reader gave is not the truth, on a photo that was flagged.
    Caught,
    /// The reader gave nothing.
    Withheld,
}

/// One field of one capture, in stored units: the apex in yards, and a lit tile signed.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TruthCell {
    pub field: String,
    pub kind: TruthCellKind,
    /// What the reader gave. A person's correction does not move this.
    pub as_read: Option<f64>,
    pub truth: f64,
}

/// One capture.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TruthPhoto {
    /// The capture's id in the photos file.
    pub id: String,
    pub route: TruthRoute,
    /// The entry and the name it arrived under, or the digest of a failed read, which has neither.
    pub entry: Option<String>,
    /// The log's word at read time, or the stored flag where there is no line.
    pub flagged: bool,
    /// Nothing was stored, so every cell is withheld and it counts as flagged.
    pub failed_read: bool,
    /// In the device's tile order, and empty if the capture holds none.
    pub cells: Vec<TruthCell>,
}

/// One field across the captures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TruthField {
    pub field: String,
    pub read: i64,
    /// Every cell of this field in a matched capture.
    pub judged: i64,
}

/// The reader held to a table a person typed. **Numbers beside M33's and no verdict**: `crate::bay`'s
/// module doc says why.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TruthReport {
    /// `provisional: pasted copies` where a capture's digest is not its original's (rule 12).
    pub label: Option<String>,
    /// Rows in the photos file.
    pub captures: i64,
    /// Of those, the ones holding cells. Every count below is over them.
    pub matched: i64,
    pub by_digest: i64,
    pub by_name: i64,
    pub read: i64,
    /// On photos not flagged, which is T1's number.
    pub wrong: i64,
    pub caught: i64,
    pub withheld: i64,
    /// `read + wrong + caught + withheld`.
    pub judged: i64,
    pub flagged: i64,
    /// The lowest field by read share, the first of a tie in the device's tile order.
    pub lowest_field: Option<String>,
    pub lowest_read: i64,
    pub lowest_judged: i64,
    /// No wrong cell, and every capture found, so that a run over nothing cannot read as clean.
    pub held: bool,
    /// In the device's tile order.
    pub fields: Vec<TruthField>,
    pub photos: Vec<TruthPhoto>,
    /// What the counts leave out, each a sentence.
    pub notes: Vec<String>,
    /// All of the above as a table, since Dart formats nothing (rule 8). P18 prints this.
    #[serde(skip)]
    pub text: String,
}

/// The reads under `root` held to the truth table at `truth_path`, for the captures at `photos_path`.
/// `Err` is a file that cannot be read or does not say what is asked of it.
pub fn truth_report(
    root: String,
    photos_path: String,
    truth_path: String,
) -> Result<TruthReport, String> {
    crate::bay::truth_of(
        std::path::Path::new(&root),
        std::path::Path::new(&photos_path),
        std::path::Path::new(&truth_path),
    )
}
