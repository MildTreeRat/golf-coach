//! The impact drill by STRIKEPRO tape: one photo of the label on the club in, the strikes it stored
//! and the card's words out, and the golfer's two declarations, a tap on the new mark and "missed the
//! tape" [M46 P18]. Then the drill's actions: the clubs it can run on, a club's drill opened, a
//! suggestion answered and the golfer's own change of stage [M46 P19].
//!
//! The shapes Dart sees and one-line calls (the M38 plan's rule 7). `crate::drill` holds what they
//! do, in what order and why, and every word on the card and the drill's page is
//! `feedback::drill_rules`' or `contracts::drill::TapeNotRead`'s (rule 8). P20 adds the progress
//! views.

/// The spot a differential strike was called to (C19): drawn in Dart, uniformly over the three,
/// and passed with each photo. High and low are not called in M46.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrillCall {
    Heel,
    Centre,
    Toe,
}

/// One photo of the label on the club, decoded by Swift (C16).
#[derive(Debug, Clone, PartialEq)]
pub struct TapeIn {
    /// The data root, `<Documents>/processed/` on the phone.
    pub root: String,
    /// The golfer whose drill this is, a golfer on this phone: the hand that checks which way the
    /// club lay is theirs (C4).
    pub player_id: String,
    /// `ClubId`'s own spelling (`7i`). A club whose category takes no label is refused.
    pub club: String,
    /// The photo as raw RGBA, 4 bytes a pixel row by row with no header, as Swift wrote it **directly
    /// under `<root>/.app/tmp/`**, and refused anywhere else. Once that holds, the file is Rust's:
    /// it is deleted whatever the answer, except that a photo with no label found is kept as a JPEG
    /// in `.app/failed/` (C16).
    pub raw_path: String,
    pub width: i64,
    pub height: i64,
    /// When the photo was taken, RFC 3339 with an offset: Swift's answer.
    pub captured_at: String,
    /// When it was recorded, RFC 3339 with an offset.
    pub now: String,
    /// The call this strike was hit at: present exactly when the club is in differential practice,
    /// and refused otherwise (C19). A photo not read keeps it, so Dart draws the next only after a
    /// strike that counted.
    pub call: Option<DrillCall>,
}

/// What became of a photo, or of a declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TapeStatus {
    /// One or two strikes stored where the reader placed them. Two means a photo was missed (C10).
    Read,
    /// The golfer's tap stored the strike: `pick_mark`'s answer.
    Picked,
    /// The golfer said the ball missed the tape: `declare_off_tape`'s answer.
    OffTape,
    /// The label was found, but its new mark could not be told apart. The crop waits for a tap
    /// (`pick_mark`), and nothing is counted until then.
    NeedsPick,
    /// The label was found, and nothing on it is new. The strike waits for "missed the tape"
    /// (`declare_off_tape`) or a retake, and a tap is taken too (`pick_mark`), for a strike landing on
    /// an old mark.
    NoNewMark,
    /// Nothing was placed, and `message` says why. The strike is stored, and never counted.
    NotRead,
}

/// The answer to one photo or one declaration: the reading card's words (C20).
#[derive(Debug, Clone, PartialEq)]
pub struct TapeOutcome {
    pub status: TapeStatus,
    /// The strikes this call stored, in the order they landed: one or two on `Read`, one on
    /// `Picked`, `OffTape` and `NotRead`, and none while a strike waits.
    pub strikes: Vec<TapeStrikeCard>,
    /// The waiting strike's id, which `pick_mark` and `declare_off_tape` take: on `NeedsPick` and
    /// `NoNewMark` only.
    pub pending_id: Option<String>,
    /// The label as cut out and squared, a JPEG: the one to tap on `NeedsPick` and `NoNewMark`, and
    /// the stored strike's otherwise. `None` when no label was found.
    ///
    /// **A tap is taken across this image**, 0 to 1 from its left edge and from its top edge, as it is
    /// drawn: Rust turns that into millimetres on the label.
    pub crop_path: Option<String>,
    /// Why nothing was placed: `NotRead`'s and `NoNewMark`'s sentence.
    pub message: Option<String>,
    /// Whether another photo of this strike could be read: true only on `NotRead` and `NoNewMark`
    /// when the reason says a retake helps.
    pub retake_helps: bool,
    /// The club's baseline or block so far, after this call ("Block: 6 of 10, 4 inside").
    pub block_line: String,
    /// The result of the block this call finished, and the next target, when it finished one.
    pub block_result: Option<String>,
    /// What the drill suggests after the block this call finished, when it suggests something: the
    /// sentence [`DrillView::suggestion`] carries until the golfer answers it
    /// ([`answer_suggestion`]). `None` on every call that finished no block, even while an older
    /// suggestion waits, so that the card shows a suggestion once, beside the result that raised it.
    pub suggestion: Option<String>,
}

/// One stored strike on the reading card.
#[derive(Debug, Clone, PartialEq)]
pub struct TapeStrikeCard {
    pub strike_id: String,
    /// Where it landed ("8.4 mm toe, 2.1 mm high"), "(picked)" after a tap, "off the tape" or "not
    /// read".
    pub position: String,
    /// Against the circle it was hit at ("Inside 12 mm"), or the call ("Toe call: made"). `None` in
    /// the baseline, which has no circle, and for a strike not read.
    pub verdict: Option<String>,
}

/// Read one photo of the label on the club, store what it shows, and judge the block when it is
/// full (`crate::drill`'s doc).
///
/// `Err` is a refusal or a failure, never a photo read as nothing, which is `NotRead`: a time with
/// no offset, a club no label fits, a golfer not on this phone, a call where none is wanted or none
/// where one is, a drill record that cannot be read, or a store that cannot be written.
pub fn record_tape_photo(photo: TapeIn) -> Result<TapeOutcome, String> {
    crate::drill::record(photo)
}

/// The golfer's tap on a waiting strike's crop (C6), `x` and `y` from 0 to 1 across the image at
/// [`TapeOutcome::crop_path`]. The strike is stored as picked, a declaration, and counted.
pub fn pick_mark(
    root: String,
    pending_id: String,
    x: f64,
    y: f64,
    now: String,
) -> Result<TapeOutcome, String> {
    crate::drill::pick(&root, &pending_id, x, y, &now)
}

/// "Missed the tape" for a strike waiting with no new mark (C6). It is stored with no position,
/// counted, and outside every circle. Refused for a strike whose new mark was seen.
pub fn declare_off_tape(
    root: String,
    pending_id: String,
    now: String,
) -> Result<TapeOutcome, String> {
    crate::drill::declare(&root, &pending_id, &now)
}

// --- The drill's actions [M46 P19] ------------------------------------------------------------------

/// Where a club is in the drill, as `contracts::drill::DrillStage` spells it. Declared here rather
/// than taken from `contracts`, as `api::setup`'s enums are, because the codegen binds only what
/// `app::api` holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrillStage {
    /// Ten strikes aimed at centre, which set the club's level.
    Baseline,
    /// Blocks of ten against the club's circle, moving in, out or staying.
    Ladder,
    /// Blocks whose goal is their own centre: moving the pattern before shrinking it.
    Recentre,
    /// Heel, centre and toe calls, each photo passed with its call.
    Differential,
}

/// One club of the golfer's bag, as the drill's club picker lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct DrillClubView {
    /// `ClubId`'s spelling, which [`start_drill`] takes.
    pub club: String,
    /// The club's category as `ClubCategory` spells it (`mid_iron`), which a category's progress
    /// view takes (P20).
    pub category: String,
    /// The STRIKEPRO label this club takes (`strikepro_iron`), or `None` for a club no label fits.
    pub label_type: Option<String>,
    /// Whether [`start_drill`] opens this club: false only for a club no label fits (the putter),
    /// which is listed so the picker can say why.
    pub can_start: bool,
    /// Where the club is in the drill, or `None` before its first strike is stored and for a club
    /// that cannot start.
    pub stage: Option<DrillStage>,
    /// [`DrillClubView::stage`] in words ("Ladder"), with it.
    pub stage_name: Option<String>,
    /// The line under the club's name: its level ("Level 3 of 7: 10 mm"), "No level yet: the
    /// baseline sets it", or "No STRIKEPRO label fits this club".
    pub line: String,
}

/// One club's drill as its page shows it, between photos.
#[derive(Debug, Clone, PartialEq)]
pub struct DrillView {
    /// `ClubId`'s spelling.
    pub club: String,
    pub stage: DrillStage,
    /// The stage in words ("Recentring").
    pub stage_name: String,
    /// The club's level ("Level 3 of 7: 10 mm"), or "No level yet: the baseline sets it".
    pub level_line: String,
    /// The baseline or block so far ("Block: 6 of 10, 4 inside"), as the card's after a photo.
    pub block_line: String,
    /// The suggestion waiting for the golfer's answer ([`answer_suggestion`]), in words with why it
    /// was made, or `None`. Worked again from the drill's record each time, never stored as words.
    pub suggestion: Option<String>,
    /// The club is in differential practice, so each photo is passed with a call (C19): Dart draws
    /// one and shows it before the strike.
    pub call_wanted: bool,
}

/// Every club in the golfer's bag, in bag order, with where each is in the drill. A club no label
/// fits is listed and cannot start. Empty when the golfer has no bag.
///
/// `Err` only when the drill's record on this phone cannot be read, which is never shown as no
/// drill (the plan's P17 finding 4).
pub fn drill_clubs(root: String, player_id: String) -> Result<Vec<DrillClubView>, String> {
    crate::drill::clubs(&root, &player_id)
}

/// Open the club's drill, or resume it where it stands. A club with no drill yet is shown at the
/// start of its baseline, and **nothing is written**: the club's first stored strike starts it
/// (`record_tape_photo`), so opening a club and leaving it leaves no trace.
///
/// Refused for a club no label fits, a golfer not on this phone, or a record that cannot be read.
pub fn start_drill(
    root: String,
    player_id: String,
    club: String,
    now: String,
) -> Result<DrillView, String> {
    crate::drill::start(&root, &player_id, &club, &now)
}

/// The golfer's answer to the waiting suggestion. Accepting moves the club to the stage it names
/// (recentring, differential practice, back to the ladder, or a fresh baseline), at the same level,
/// and abandons the block in progress. Declining only clears it, and the drill offers it again after
/// any later block for which it still holds: "not now" is never "never".
///
/// Refused when no suggestion waits.
pub fn answer_suggestion(
    root: String,
    player_id: String,
    club: String,
    accept: bool,
    now: String,
) -> Result<DrillView, String> {
    crate::drill::answer_suggestion(&root, &player_id, &club, accept, &now)
}

/// The golfer's own change of stage, at the same level. It abandons the block in progress, whose
/// strikes stay stored and are never judged (C10), and clears a waiting suggestion. The stage the
/// club is already in changes nothing.
///
/// Refused past the baseline for a club whose first baseline has not set a level yet.
pub fn set_stage(
    root: String,
    player_id: String,
    club: String,
    stage: DrillStage,
    now: String,
) -> Result<DrillView, String> {
    crate::drill::set_stage(&root, &player_id, &club, stage, &now)
}
