//! The impact drill by STRIKEPRO tape: one photo of the label on the club in, the strikes it stored
//! and the card's words out, and the golfer's two declarations, a tap on the new mark and "missed the
//! tape" [M46 P18]. Then the drill's actions: the clubs it can run on, a club's drill opened, a
//! suggestion answered and the golfer's own change of stage [M46 P19]. Then the progress views, a
//! club's and its category's, and the geometry the card and those views draw [M46 P20].
//!
//! The shapes Dart sees and one-line calls (the M38 plan's rule 7). `crate::drill` holds what they
//! do, in what order and why, and every word on the card and the drill's page is
//! `feedback::drill_rules`' or `contracts::drill::TapeNotRead`'s (rule 8).
//!
//! # Drawing the label (C20)
//!
//! Every shape is in **one frame, the crop's**: 0 to 1 across the label's box from its left edge, and
//! 0 to 1 down it from its top edge, the label as printed with its logo at the top. A tap on a crop
//! is taken in it ([`pick_mark`]), and a stored crop is drawn in it, so a dot, the outline and the
//! circle land on a crop where they belong. Dart scales a [`LabelPoint`] by the box it draws in and
//! paints, and works out nothing: a [`LabelCircle`] carries its radius as a share of each side, so it
//! is round in a box of [`LabelShape::aspect`].

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
    /// The club's label type, drawn over the crop at [`TapeOutcome::crop_path`], or alone when no
    /// label was found. [M46 P20]
    pub label: LabelShape,
    /// The circle this photo's strike was hit at, about the label's centre: the radius a strike's
    /// verdict names. `None` in the baseline, which has no circle. When two marks close a block and
    /// open the next, it is the first's, and the second's verdict names its own. [M46 P20]
    pub circle: Option<LabelCircle>,
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
    /// Where it landed on the crop, in the tap's frame, so a picked strike's dot is the tap. `None`
    /// for a strike off the tape or not read, which has no place on the label. [M46 P20]
    pub dot: Option<LabelPoint>,
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
    /// The club's category as `ClubCategory` spells it (`mid_iron`), which [`category_progress`]
    /// takes.
    pub category: String,
    /// The category in words ("Mid irons"), the title [`category_progress`] gives it. [M46 P20]
    pub category_name: String,
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

// --- The progress views [M46 P20] -------------------------------------------------------------------

/// A point on the label, in the crop's frame (the module doc): 0 to 1 from the left edge of the
/// label's box, and 0 to 1 down from its top edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabelPoint {
    pub x: f64,
    pub y: f64,
}

/// A circle on the label, in the crop's frame: its centre, and its radius as a share of the box's
/// width (`rx`) and of its height (`ry`). Drawn as the oval those span, it is a circle in a box of
/// [`LabelShape::aspect`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabelCircle {
    pub x: f64,
    pub y: f64,
    pub rx: f64,
    pub ry: f64,
}

/// A label type's shape, for drawing it.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelShape {
    /// The label type's id, `strikepro_iron`.
    pub label_type: String,
    /// The box's width over its height: the shape Dart draws the label in, and the crop's.
    pub aspect: f64,
    /// The label's edge, the measured outline (`impact::tape`'s `LabelType::outline_mm`). The last
    /// point joins the first.
    pub outline: Vec<LabelPoint>,
    /// The printed cross: the label's centre, and face centre (D2). Every circle is about it.
    pub cross: LabelPoint,
}

/// One strike placed on the label.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrikeDot {
    pub at: LabelPoint,
    /// The stage it was hit in. A differential strike was aimed off centre on purpose (C12), so it is
    /// drawn apart, and it is in neither the pattern nor the heat map.
    pub stage: DrillStage,
}

/// Where the strikes sit and how widely (C12): `analysis::drill::pattern`, over the last 20 placed
/// strikes aimed at centre.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternView {
    /// The strikes' mean position.
    pub centre: LabelPoint,
    /// The spread as a circle about [`PatternView::centre`]: half the strikes lie inside it.
    pub spread: LabelCircle,
    /// "Pattern centre: 3.2 mm toe, 1.0 mm low".
    pub centre_line: String,
    /// "Spread: half within 6.5 mm of the pattern's centre".
    pub spread_line: String,
    /// R50: "Half your strikes within 9.4 mm of the label's centre".
    pub r50_line: String,
}

/// C21's heat map: the density of the strikes aimed at centre over the label, highest at 1.
#[derive(Debug, Clone, PartialEq)]
pub struct HeatMapView {
    /// Rows from the top of the label's box down, each a row of cells from its left edge across, every
    /// value from 0 to 1. Cell `[r][c]` spans `c * cell_width` to `(c + 1) * cell_width` across, and
    /// the same down by rows. The last row and column may run past the box, so the painter clips to
    /// [`LabelShape::outline`].
    pub rows: Vec<Vec<f64>>,
    /// A cell's width as a share of the box's.
    pub cell_width: f64,
    /// A cell's height as a share of the box's.
    pub cell_height: f64,
}

/// One judged block or baseline in a club's level history, in words.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryRow {
    /// The day it was judged, `2026-11-13`, at the phone's offset when it was.
    pub date: String,
    /// The block's stage ("Ladder"), or `None` when the record holds none of its strikes any more: a
    /// line left out ([`ProgressView::record_note`]).
    pub stage: Option<String>,
    /// The circle before ("12 mm"), or "no level" before the club's first baseline set one.
    pub from: String,
    /// The circle after.
    pub to: String,
}

/// One club in a category's view.
#[derive(Debug, Clone, PartialEq)]
pub struct ClubLevelRow {
    /// `ClubId`'s spelling.
    pub club: String,
    /// Its stage in words ("Ladder").
    pub stage_name: String,
    /// Its level line ("Level 3 of 7: 10 mm"), or "No level yet: the baseline sets it".
    pub level_line: String,
}

/// A club's drill progress, or a category's pooled over its clubs (D10, D13).
///
/// The label is drawn as the golfer looks at the face of the club, shaft up, with the label stuck
/// logo-up: the toe on the left for a right-handed golfer (C4), and [`ProgressView::left_side`] says
/// which end is which. A strike is drawn where it sat on its own label, whichever way up that label
/// was stuck.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressView {
    /// The club ("7i"), or the category ("Mid irons").
    pub title: String,
    pub label: LabelShape,
    /// The word for the label's left end, "Toe" or "Heel".
    pub left_side: String,
    /// The word for its right end.
    pub right_side: String,
    /// The club's current circle, about the label's centre. `None` before its first baseline sets a
    /// level, and for a category, whose clubs each have their own ([`ProgressView::clubs`]).
    pub circle: Option<LabelCircle>,
    /// The club's level line. `None` for a category.
    pub level_line: Option<String>,
    /// Every strike placed on the label, read or picked, oldest first, differential strikes
    /// included. A strike off the tape or not read has no place and no dot.
    pub dots: Vec<StrikeDot>,
    /// `None` below 10 placed strikes aimed at centre, never an estimate.
    pub pattern: Option<PatternView>,
    /// What the pattern still needs when it is `None` ("The pattern needs 6 more strikes placed on
    /// the label."), and `None` when it is shown.
    pub pattern_needs: Option<String>,
    /// From the same strikes as the pattern, every one rather than its last 20, and shown from the
    /// same count.
    pub heat_map: Option<HeatMapView>,
    /// What the heat map still needs when it is `None`, and `None` when it is shown.
    pub heat_map_needs: Option<String>,
    /// The club's judged blocks, oldest first. Empty for a category.
    pub history: Vec<HistoryRow>,
    /// A category's clubs with a drill, in bag order. Empty for a club.
    pub clubs: Vec<ClubLevelRow>,
    /// Lines of the drill's record that could not be read and are not in this view, or `None`.
    pub record_note: Option<String>,
}

/// One club's progress: its strikes on its label, its circle and level, its pattern and heat map,
/// and its level history. A club with no strikes answers an empty view. **Writes nothing.**
///
/// Refused for a club no label fits, a golfer not on this phone, or a record that cannot be read.
pub fn drill_progress(
    root: String,
    player_id: String,
    club: String,
) -> Result<ProgressView, String> {
    crate::drill::club_progress(&root, &player_id, &club)
}

/// A category's progress (D10): every club of it with a drill, their strikes pooled on the category's
/// label, and each club's level listed. `category` is [`DrillClubView::category`]'s spelling.
/// **Writes nothing.**
///
/// Refused for a category that does not exist or that no label fits, a golfer not on this phone, or
/// a record that cannot be read.
pub fn category_progress(
    root: String,
    player_id: String,
    category: String,
) -> Result<ProgressView, String> {
    crate::drill::category_progress(&root, &player_id, &category)
}
