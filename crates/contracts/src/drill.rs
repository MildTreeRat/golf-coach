//! The impact drill's shapes: a stored strike, a club's place on the ladder, and the ladder's
//! tables. [M46 P6]
//!
//! **One spelling for three crates.** The drill store (`storage::drill_store`, P17) writes these,
//! the drill's judgments (`analysis::drill`, P14 and P15) read them, and the app's facade
//! (`crates/app`'s `drill`, P18 to P20) builds them, so each lives here once rather than as three
//! copies that drift (ADR-037 clauses 5 to 9, the M46 plan's C7 to C14). **No Python twin**, as with
//! [`crate::strike`] and [`crate::face_impact`]: the drill was designed after the pivot (ADR-034), and
//! frozen Python never gains it (ADR-035 clause 4).
//!
//! # Inputs are stored, verdicts are derived
//!
//! A [`TapeStrike`] keeps what was known when it was made: the club and its category, the label and
//! its place on it, the stage, the circle's radius at the time, the call, the offsets, how the
//! position was got and the reader's version. **Whether it landed inside its circle is never
//! stored** (C8), for [`PracticeMode::is_tracked`](crate::intent::PracticeMode::is_tracked)'s reason:
//! a stored verdict could disagree with its own numbers. Whether it counts toward a block is
//! [`TapeStrike::counts`], derived from `source` the same way. Inside is `analysis::drill`'s (P14),
//! because it is a judgment against a circle and this crate holds shapes.
//!
//! **The category is stored as it was, and not checked against today's club map.** A strike was
//! judged against its category's table, so the category is part of its record. Refusing a stored
//! strike whose category differs from [`category_of`] would refuse the whole history the day
//! `CLUB_CATEGORY` moved a club, which is reading the past by today's rule.
//!
//! # The ladder's tables
//!
//! [`ladder_radii`] is what ships: C9's tables with the first of their two cuts applied. The uncut
//! tables, the user's provisional radii in millimetres from the M46 interview (D9), widest first:
//!
//! | Category | Radii (mm) |
//! |---|---|
//! | `driver` | 30, 26, 22, 19, 16, 14, 12, 10 |
//! | `wood` | 28, 24, 21, 18, 15, 13, 11, 9 |
//! | `hybrid` | 26, 22, 19, 16, 14, 12, 10, 8 |
//! | `long_iron` | 24, 21, 18, 15, 13, 11, 9, 8, 7 |
//! | `mid_iron` | 22, 19, 16, 14, 12, 10, 9, 8, 7, 6 |
//! | `short_iron` | 20, 17, 15, 13, 11, 10, 9, 8, 7, 6, 5 |
//! | `wedge` | 18, 16, 14, 12, 10, 9, 8, 7, 6, 5 |
//! | `putter` | 15, 12, 10, 8, 7, 6, 5, 4 |
//!
//! The two cuts:
//! - **Trimmed** (P4, applied): a radius above the inscribed radius of the category's label
//!   (`impact::tape`'s `LabelType::inscribed_radius_mm`) is removed. So a strike declared off the tape
//!   is outside every circle (C6), which is the only reason that declaration can count as a miss. A
//!   fairway wood takes the hybrid label (P3 finding 8), so `wood` is trimmed at the hybrid's 16.8 mm.
//!   **The putter takes no label** (the user, P3 finding 8), so its table does not ship at all.
//! - **Floored** (P29, not applied): a radius below twice the label type's measured P95 reading error
//!   is to be removed, so that no level is tighter than the reader can resolve (ADR-010 §2). No real
//!   strike is read before P29 (the plan's P3 finding 10), so nothing has measured that error yet, and
//!   the tables ship trimmed and unfloored until then.
//!
//! **A table is indexed by the level a club stores** ([`ClubDrillState::level`]), so revising one
//! re-means every stored level. The floor removes only the tightest radii, from the end, so it moves
//! no level a club still holds a radius for, and a club whose level it removes is refused by
//! [`DrillState`]'s check until it is moved. Any other revision says how stored levels move. Every
//! strike keeps the radius it was judged against, so the record does not move with the table.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::club::{category_of, ClubId};
use crate::face_impact::ImpactMethod;
use crate::golfer::check_player_id;
use crate::intent::ClubCategory;
use crate::{each, ge, nested, ContractError, Timestamp, Validate};

// --- The constants ------------------------------------------------------------------------------

/// Counted strikes in a block, and in a baseline (C10, C11). **The user's, provisional, M46
/// interview** (D8). What revises it is the user's feel for a block's length once P29's sessions
/// are real.
pub const BLOCK_SIZE: usize = 10;

/// A block with at least this many strikes inside its circle moves the club in a level (D8). **The
/// user's, provisional, M46 interview**, revised the way [`BLOCK_SIZE`] is, and read with it: 7 of
/// 10.
pub const MOVE_IN_AT: usize = 7;

/// A block with at most this many strikes inside its circle moves the club out a level (D8). **The
/// user's, provisional, M46 interview**, revised as [`MOVE_IN_AT`] is. Between the two, the level
/// stays.
pub const MOVE_OUT_AT: usize = 3;

/// The start level is the tightest circle holding at least this many of the baseline's strikes
/// (C11): half of them, so the ladder starts at the tightest circle the baseline already half
/// fills. **The user's, provisional, M46 interview**, revised as [`BLOCK_SIZE`] is. If even the widest holds fewer, the baseline
/// suggests differential practice (C13).
pub const BASELINE_HOLDS: usize = 5;

/// The pattern is taken over a club's last this-many positioned strikes in the baseline, ladder and
/// recentre stages (C12): two blocks' worth. **The user's, provisional, M46 interview**, revised
/// once P29's sessions show how quickly a real pattern moves. Below
/// [`RATE_MINIMUM_N`](crate::strike::RATE_MINIMUM_N) positioned strikes it is absent, never
/// estimated.
pub const PATTERN_WINDOW: usize = 20;

/// The pattern's centre is clearly off when, on some axis, its interval excludes 0 **and** its mean is
/// more than this fraction of the current radius from 0, and a recentre block's goal is its own mean
/// within it (C13). **The user's, provisional, M46 interview**, revised once P29 shows how far a
/// real pattern sits off centre.
pub const RECENTRE_FRACTION: f64 = 1.0 / 3.0;

/// Differential practice is suggested after this many blocks in a row at [`MOVE_OUT_AT`] or fewer
/// inside, at the widest level (C13). **The user's, provisional, M46 interview**. Named here, beside
/// the six the plan listed, so that P15 does not write it as a literal.
pub const DIFFERENTIAL_AFTER_BLOCKS: usize = 2;

/// A differential block with at least this many calls made suggests a fresh baseline (C13). **The
/// user's, provisional, M46 interview**. The same 7 as [`MOVE_IN_AT`] and its own constant, because
/// a call made and a strike inside are different successes and either may be revised alone.
pub const FRESH_BASELINE_AT: usize = 7;

// The tables, widest first, as the module doc's trimmed rows.
// floored at P29: the second cut (the module doc) is applied here, from each label type's measured
// P95 reading error, once P29 has measured one.
const DRIVER_RADII: [f64; 6] = [22.0, 19.0, 16.0, 14.0, 12.0, 10.0];
const WOOD_RADII: [f64; 4] = [15.0, 13.0, 11.0, 9.0];
const HYBRID_RADII: [f64; 5] = [16.0, 14.0, 12.0, 10.0, 8.0];
const LONG_IRON_RADII: [f64; 5] = [13.0, 11.0, 9.0, 8.0, 7.0];
const MID_IRON_RADII: [f64; 7] = [14.0, 12.0, 10.0, 9.0, 8.0, 7.0, 6.0];
const SHORT_IRON_RADII: [f64; 8] = [13.0, 11.0, 10.0, 9.0, 8.0, 7.0, 6.0, 5.0];
const WEDGE_RADII: [f64; 8] = [14.0, 12.0, 10.0, 9.0, 8.0, 7.0, 6.0, 5.0];

/// The target radii a club category's ladder climbs, in millimetres, widest first: index 0 is the
/// widest level, and a level is an index into this. **Empty for a category with no ladder**, the
/// putter (no label fits it) and `all` (no club has it).
///
/// The trimmed tables (the module doc), **unfloored until P29**. A `match` with no wildcard, so a
/// category added to [`ClubCategory`] fails to compile here until someone gives it a table or
/// none, rather than defaulting.
pub fn ladder_radii(category: ClubCategory) -> &'static [f64] {
    match category {
        ClubCategory::Driver => &DRIVER_RADII,
        ClubCategory::Wood => &WOOD_RADII,
        ClubCategory::Hybrid => &HYBRID_RADII,
        ClubCategory::LongIron => &LONG_IRON_RADII,
        ClubCategory::MidIron => &MID_IRON_RADII,
        ClubCategory::ShortIron => &SHORT_IRON_RADII,
        ClubCategory::Wedge => &WEDGE_RADII,
        ClubCategory::Putter | ClubCategory::All => &[],
    }
}

// --- The ids --------------------------------------------------------------------------------------

/// A strike's id, `<install>-<N>` (C8): the phone's install id (M38's, four hex digits) and `n`
/// from [`DrillState::strikes_issued`], so two phones cannot collide.
pub fn strike_id(install: &str, n: i64) -> String {
    format!("{install}-{n}")
}

/// An applied label's id, `<install>-L<N>` (C8), `n` from [`DrillState::labels_issued`].
pub fn label_id(install: &str, n: i64) -> String {
    format!("{install}-L{n}")
}

/// Where a strike's crop sits, relative to the method's directory (C7): `crops/<strike_id>.jpg`.
/// [`TapeStrike::crop`] holds exactly this, so the store and the record cannot name two files.
pub fn crop_file(strike_id: &str) -> String {
    format!("crops/{strike_id}.jpg")
}

// --- The vocabulary -----------------------------------------------------------------------------

/// What a drill trains. The second segment of a drill's directory, `drills/<player_id>/<topic>/`
/// (C7).
///
/// **One variant in M46.** M44's low point is the next, `LowPoint`, whose foot-powder method will
/// sit at `drills/<player_id>/low_point/foot_powder/` beside this one (ADR-037 clause 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DrillTopic {
    /// Where on the face the ball was struck.
    Impact,
}

impl DrillTopic {
    /// The wire name, which is also the directory's name.
    pub const fn as_str(self) -> &'static str {
        match self {
            DrillTopic::Impact => "impact",
        }
    }
}

/// How the impact drill collects its strikes. The third segment of its directory,
/// `drills/<player_id>/impact/<method>/` (C7).
///
/// **One variant in M46**: STRIKEPRO labels, read as that exact product (D1). Another tape later is a
/// new label type, and another way of seeing the strike is a new variant here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TapeMethod {
    /// A STRIKEPRO label on the face, photographed after every strike.
    StrikeTape,
}

impl TapeMethod {
    /// The wire name, which is also the directory's name.
    pub const fn as_str(self) -> &'static str {
        match self {
            TapeMethod::StrikeTape => "strike_tape",
        }
    }

    /// The [`ImpactMethod`] this drill's readings are made by. Two enums because they answer two
    /// questions, how a drill gathers strikes and how one reading was made, and a `match` because a
    /// drill method added is a compile error here until its readings are named.
    pub const fn impact_method(self) -> ImpactMethod {
        match self {
            TapeMethod::StrikeTape => ImpactMethod::StrikeTape,
        }
    }
}

/// Where a club is in the drill (C11, C13, ADR-037 clauses 7 to 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DrillStage {
    /// Ten counted strikes aimed at centre, which set the start level (C11). A club's first stage,
    /// and the one a fresh baseline returns to.
    Baseline,
    /// Blocks of ten against the club's current circle, moving in, out or staying (D8, C10).
    Ladder,
    /// Blocks whose goal is their own mean within [`RECENTRE_FRACTION`] of the radius: moving the
    /// pattern before shrinking it (C13). Suggested, never the default (D11).
    Recentre,
    /// Random heel, centre and toe calls, judged against the club's current radius (D12, C14).
    /// Suggested, never the default (D11).
    Differential,
}

/// The spot a differential strike was called to (D12, C14). High and low are not called in M46.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrikeCall {
    Heel,
    Centre,
    Toe,
}

/// How a counted strike's position was got (C6, ADR-037 clause 4). A strike with none of these was
/// not read, and says why in [`TapeStrike::not_read`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrikeSource {
    /// The reader found the new mark and placed it.
    Read,
    /// **A declaration**: the reader could not tell the new mark apart, and the golfer tapped it on
    /// the crop (C6). Stored as what it is, never as a reading.
    Picked,
    /// **A declaration**: the reader saw no new mark, and the golfer said the ball missed the tape
    /// (C6). No position, and outside every circle, which holds because every table is trimmed at
    /// its label's inscribed radius (the module doc).
    DeclaredOffTape,
}

/// Why the reader placed no mark (C5, ADR-037 clause 3). Each has its sentence and its
/// [`retake_helps`](TapeNotRead::retake_helps) flag, which belongs to the reason as
/// [`UnscoredReason`](crate::unscored::UnscoredReason)'s `refilming_helps` does, so nothing
/// downstream infers it.
///
/// **The reader's own reasons are `impact::tape`'s**, which takes no dependency and so cannot name
/// this enum. The facade (P18) maps them across by wire name, as `impact::tape::category_label`
/// takes a category's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TapeNotRead {
    /// No label was found in the photo.
    NoLabel,
    /// The label was found, but the hosel and shaft were not, or they disagreed with the golfer's
    /// hand. A guessed side is a guessed sign, so the reader refuses (C4).
    NoOrientation,
    /// The label was found and squared, and nothing on it is new since the last strike.
    NoNewMark,
    /// The new mark touches the label's edge, so part of it may be off the tape and its centre is
    /// not where the mark's middle is.
    MarkClipped,
    /// The photo could not be decoded into pixels.
    DecodeFailed,
    /// **Never the reader's: the facade's** (M46 P18, as P6's finding 3 left to it). The reader
    /// could not tell the new mark apart and asked for a tap, and the golfer moved on without one:
    /// the next photo of the club, or a change of stage, discarded the pick. The mark is on the
    /// label, so the club's last crop already holds it and a retake would show nothing new, which
    /// is why this is stored rather than dropped. Exit 2 asks every strike to be placed, declared or
    /// named not read.
    NotPicked,
}

impl TapeNotRead {
    /// What the golfer is told. Plain words, with no number in them (C22).
    pub const fn sentence(self) -> &'static str {
        match self {
            TapeNotRead::NoLabel => {
                "The label was not found in the photo. Take it again with the whole label in frame."
            }
            TapeNotRead::NoOrientation => {
                "The label was found, but not which way the club was facing. Take it again with the \
                 hosel and some of the shaft in frame."
            }
            TapeNotRead::NoNewMark => {
                "No new mark was found on the label. If the ball hit the tape, take it again closer \
                 and sharper. If it missed the tape, say so."
            }
            TapeNotRead::MarkClipped => {
                "The new mark runs off the edge of the label, so where the ball met the face cannot \
                 be placed. Another photo would show the same edge."
            }
            TapeNotRead::DecodeFailed => "The photo could not be opened. Take it again.",
            TapeNotRead::NotPicked => {
                "The new mark could not be told from the old ones, and none was tapped, so this \
                 strike was not placed."
            }
        }
    }

    /// Whether another photo of the same strike could plausibly be read. True for what the camera
    /// caused, and false for [`TapeNotRead::MarkClipped`], where the strike itself landed on the
    /// edge and every photo of it shows the same. [`TapeNotRead::NoNewMark`] is true, because a faint
    /// mark may show in a sharper photo, and it is also where "missed the tape" is offered (C6).
    /// [`TapeNotRead::NotPicked`] is false, because the club's last crop already holds the mark.
    pub const fn retake_helps(self) -> bool {
        match self {
            TapeNotRead::NoLabel
            | TapeNotRead::NoOrientation
            | TapeNotRead::NoNewMark
            | TapeNotRead::DecodeFailed => true,
            TapeNotRead::MarkClipped | TapeNotRead::NotPicked => false,
        }
    }
}

/// What the drill suggests a club do next (C13). **Suggestions, never the default** (D11): the
/// golfer accepts or declines, and a declined one is offered again after each block for which it
/// still holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Suggestion {
    /// The pattern sits off centre: move it before shrinking it.
    Recentre,
    /// The pattern is too big: random heel, centre and toe calls.
    Differential,
    /// A recentre block met its goal: back to the ladder at the same level.
    ReturnToLadder,
    /// A differential block made enough of its calls: a fresh baseline, and the ladder from it.
    FreshBaseline,
}

// --- The strike ------------------------------------------------------------------------------------

/// One strike of the impact drill, one line of `strikes.jsonl` (C7, C8).
///
/// **A position is both offsets or neither.** The tape shows both axes
/// ([`ImpactMethod::StrikeTape`]'s `axes()`), every circle is `hypot(h, v)`, and a pick converts one
/// tap into both, so a strike with one offset has no use and is refused. An absent value is an absent
/// key, as in [`crate::face_impact`] and [`crate::strike`], and a `null` that arrives reads as absent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct TapeStrike {
    /// `<install>-<N>` ([`strike_id`]).
    pub strike_id: String,
    /// The golfer, held to the slug rule because the drill's directory is named by it (C7).
    pub player_id: String,
    pub club: ClubId,
    /// The club's category when the strike was made, whose table its circle came from (the module
    /// doc says why it is not checked against today's map).
    pub category: ClubCategory,
    /// The label type's id, `impact::tape`'s `LabelType::id` (`strikepro_driver`, `strikepro_iron`
    /// or `strikepro_hybrid`). Text because this crate cannot see `impact`.
    pub label_type: String,
    /// Which label applied to the face this strike landed on, `<install>-L<N>` ([`label_id`]).
    pub label_id: String,
    /// This strike's place on its label, from 1: a label holds about 6 to 10 (D1).
    pub n_on_label: i64,
    pub captured_at: Timestamp,
    pub stage: DrillStage,
    /// The radius of the circle this strike was judged against, millimetres. **Absent in the
    /// baseline**, which aims at centre with no circle (C11), and present in every other stage: the
    /// ladder's level, a recentre block's radius, a differential call's `R` (C14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level_radius_mm: Option<f64>,
    /// The spot called, in the differential stage only, and always there (C19 draws one before
    /// every photo, and a strike not read keeps it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call: Option<StrikeCall>,
    /// Heel to toe, millimetres from the label's printed centre, + toward the toe (C4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset_h_mm: Option<f64>,
    /// High to low, millimetres from the label's printed centre, + toward the top line (C4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset_v_mm: Option<f64>,
    /// How the position was got, or absent when the strike was not read. Exactly one of this and
    /// [`not_read`](Self::not_read) is present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<StrikeSource>,
    /// Why the reader placed no mark, when nothing was stored as a position and nothing declared.
    /// Such a strike is kept and never counted (C10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_read: Option<TapeNotRead>,
    /// [`TAPE_READER_VERSION`](crate::face_impact::TAPE_READER_VERSION) when the photo was read.
    pub reader_version: i64,
    /// The rectified label crop, [`crop_file`] of this strike's id, or absent when the reader had no
    /// label to cut out. A read or picked position is always on one, so it is always there for those.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<String>,
}

crate::validated!(TapeStrike);

impl TapeStrike {
    /// Whether this strike counts toward its block (C10): read, picked or declared off the tape.
    /// **Derived, never stored**, as [`PracticeMode::is_tracked`](crate::intent::PracticeMode::is_tracked)
    /// is, and a `match` so that a source added is a compile error here until someone decides.
    pub fn counts(&self) -> bool {
        match self.source {
            Some(StrikeSource::Read | StrikeSource::Picked | StrikeSource::DeclaredOffTape) => true,
            None => false,
        }
    }

    /// The position, `(h, v)` in millimetres, when the strike has one: read or picked.
    pub fn position(&self) -> Option<(f64, f64)> {
        self.offset_h_mm.zip(self.offset_v_mm)
    }
}

impl Validate for TapeStrike {
    fn validate(&self) -> Result<(), ContractError> {
        check_player_id("TapeStrike", &self.player_id)?;
        ge("TapeStrike.n_on_label", self.n_on_label, 1)?;
        // 0 is not an entry on the reader's ledger (`TAPE_READER_VERSION`'s doc).
        ge("TapeStrike.reader_version", self.reader_version, 1)?;
        for (field, value) in [
            ("TapeStrike.offset_h_mm", self.offset_h_mm),
            ("TapeStrike.offset_v_mm", self.offset_v_mm),
        ] {
            if let Some(value) = value.filter(|v| !v.is_finite()) {
                return Err(refuse(
                    field,
                    format!("{value} is not a finite number of millimetres"),
                ));
            }
        }
        let has_offset = self.offset_h_mm.is_some() || self.offset_v_mm.is_some();
        match (self.source, self.not_read) {
            (Some(_), Some(reason)) => {
                return Err(refuse(
                    "TapeStrike.not_read",
                    format!(
                        "a strike that was placed or declared names no reason it was not read \
                         (got {})",
                        reason_name(reason)
                    ),
                ))
            }
            (None, None) => return Err(refuse(
                "TapeStrike.source",
                "a strike is read, picked, declared off the tape or not read with a reason, and \
                     this one is none of them",
            )),
            (Some(StrikeSource::Read | StrikeSource::Picked), None) => {
                if self.position().is_none() {
                    return Err(refuse(
                        "TapeStrike.offset_h_mm",
                        "a read or picked strike holds both offsets, and this one does not",
                    ));
                }
                if self.crop.is_none() {
                    return Err(refuse(
                        "TapeStrike.crop",
                        "a read or picked position is on a crop, and this strike names none",
                    ));
                }
            }
            (Some(StrikeSource::DeclaredOffTape), None) => {
                if has_offset {
                    return Err(refuse(
                        "TapeStrike.offset_h_mm",
                        "a strike declared off the tape has no position, so it holds no offset",
                    ));
                }
            }
            (None, Some(_)) => {
                if has_offset {
                    return Err(refuse(
                        "TapeStrike.offset_h_mm",
                        "a strike that was not read has no position, so it holds no offset",
                    ));
                }
            }
        }
        match (self.stage, self.call) {
            (DrillStage::Differential, None) => {
                return Err(refuse(
                    "TapeStrike.call",
                    "a differential strike was hit at a call, and this one names none",
                ))
            }
            (DrillStage::Baseline | DrillStage::Ladder | DrillStage::Recentre, Some(_)) => {
                return Err(refuse(
                    "TapeStrike.call",
                    "only a differential strike is called",
                ))
            }
            _ => {}
        }
        match (self.stage, self.level_radius_mm) {
            (DrillStage::Baseline, Some(radius)) => {
                return Err(refuse(
                    "TapeStrike.level_radius_mm",
                    format!("the baseline aims at centre with no circle (got {radius})"),
                ))
            }
            (DrillStage::Ladder | DrillStage::Recentre | DrillStage::Differential, None) => {
                return Err(refuse(
                    "TapeStrike.level_radius_mm",
                    "every stage past the baseline is judged against a circle, and this strike \
                     names none",
                ))
            }
            (_, Some(radius)) if !(radius.is_finite() && radius > 0.0) => {
                return Err(refuse(
                    "TapeStrike.level_radius_mm",
                    format!("{radius} is not a positive, finite radius"),
                ))
            }
            _ => {}
        }
        if let Some(crop) = &self.crop {
            let expected = crop_file(&self.strike_id);
            if *crop != expected {
                return Err(refuse(
                    "TapeStrike.crop",
                    format!("a strike's crop is {expected}, not {crop}"),
                ));
            }
        }
        Ok(())
    }
}

/// A reason's wire name, for a refusal's message.
fn reason_name(reason: TapeNotRead) -> &'static str {
    match reason {
        TapeNotRead::NoLabel => "no_label",
        TapeNotRead::NoOrientation => "no_orientation",
        TapeNotRead::NoNewMark => "no_new_mark",
        TapeNotRead::MarkClipped => "mark_clipped",
        TapeNotRead::DecodeFailed => "decode_failed",
        TapeNotRead::NotPicked => "not_picked",
    }
}

// --- The state -------------------------------------------------------------------------------------

/// The label on a club's face now, and how many strikes it has taken (C8, P18's counters).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppliedLabel {
    /// `<install>-L<N>` ([`label_id`]).
    pub label_id: String,
    /// The last strike's [`TapeStrike::n_on_label`] on it, so the next strike's is one more.
    pub n_on_label: i64,
}

impl Validate for AppliedLabel {
    fn validate(&self) -> Result<(), ContractError> {
        ge("AppliedLabel.n_on_label", self.n_on_label, 1)
    }
}

/// One judged block, or one baseline, and the level it left the club at (C7's level history).
///
/// **Every judged block leaves an entry, whatever its stage**, with `from == to` where the level
/// stayed. Only so can C13's "two blocks in a row at 3 or fewer at the widest level" be seen, since
/// at the widest a failed block cannot move out. An abandoned block is never judged and leaves none
/// (C10). The block's stage and how many landed inside are read from its strikes, never stored here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LevelEntry {
    pub at: Timestamp,
    /// The level before, absent before the club's first baseline set one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<usize>,
    /// The level after.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<usize>,
    /// The judged block's counted strikes, by [`TapeStrike::strike_id`], in the order they landed.
    pub block: Vec<String>,
}

/// Nothing to check on its own: its levels are as they were when it was judged, so holding them to
/// today's table would refuse the history the day a table was revised (the module doc).
impl Validate for LevelEntry {
    fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

/// One club's place in the drill, an entry of `state.json` (C7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct ClubDrillState {
    pub stage: DrillStage,
    /// The club's level, an index into [`ladder_radii`] of its category (0 is the widest). Absent
    /// until the first baseline sets it, and present in every stage after.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<usize>,
    /// The counted strikes of the block in progress, or of the baseline in progress, by id. A block
    /// is judged the moment its tenth lands, so a stored one holds fewer than [`BLOCK_SIZE`].
    #[serde(default)]
    pub block: Vec<String>,
    /// The suggestion waiting for the golfer's answer. A declined one is cleared, never remembered,
    /// so "not now" is never "never" (C13).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<Suggestion>,
    /// Every judged block and baseline, oldest first ([`LevelEntry`]).
    #[serde(default)]
    pub history: Vec<LevelEntry>,
    /// The label on the face now, absent before the club's first label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<AppliedLabel>,
}

crate::validated!(ClubDrillState);

impl Validate for ClubDrillState {
    fn validate(&self) -> Result<(), ContractError> {
        if self.level.is_none() && self.stage != DrillStage::Baseline {
            return Err(refuse(
                "ClubDrillState.level",
                "only a club in its first baseline has no level yet",
            ));
        }
        if self.block.len() >= BLOCK_SIZE {
            return Err(refuse(
                "ClubDrillState.block",
                format!(
                    "a block is judged at its {BLOCK_SIZE}th counted strike, so a stored one holds \
                     fewer (got {})",
                    self.block.len()
                ),
            ));
        }
        each("ClubDrillState.history", &self.history)?;
        nested("ClubDrillState.label", self.label.as_ref())
    }
}

/// One golfer's impact drill by tape, `state.json` (C7): each club's place, and the counters the
/// ids are drawn from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct DrillState {
    pub player_id: String,
    /// Strike ids issued so far, so the next is [`strike_id`] of one more (C8).
    pub strikes_issued: i64,
    /// Label ids issued so far, so the next is [`label_id`] of one more (C8).
    pub labels_issued: i64,
    /// Each club with a drill, in bag order.
    #[serde(default)]
    pub clubs: BTreeMap<ClubId, ClubDrillState>,
    /// When this was last written. The lab's import keeps the newer of two (C17).
    pub updated_at: Timestamp,
}

crate::validated!(DrillState);

impl Validate for DrillState {
    fn validate(&self) -> Result<(), ContractError> {
        check_player_id("DrillState", &self.player_id)?;
        ge("DrillState.strikes_issued", self.strikes_issued, 0)?;
        ge("DrillState.labels_issued", self.labels_issued, 0)?;
        for (club, state) in &self.clubs {
            let owner = format!("DrillState.clubs[{club}]");
            state.validate().map_err(|e| ContractError {
                field: format!("{owner} -> {}", e.field),
                problem: e.problem,
            })?;
            let radii = ladder_radii(category_of(*club));
            if radii.is_empty() {
                return Err(refuse(
                    &owner,
                    format!(
                        "a {} has no ladder, so it holds no drill",
                        category_of(*club).as_str()
                    ),
                ));
            }
            if let Some(level) = state.level.filter(|level| *level >= radii.len()) {
                return Err(refuse(
                    &format!("{owner}.level"),
                    format!(
                        "level {level} is past the {} table's {} levels",
                        category_of(*club).as_str(),
                        radii.len()
                    ),
                ));
            }
        }
        Ok(())
    }
}

fn refuse(field: &str, problem: impl Into<String>) -> ContractError {
    ContractError {
        field: field.to_string(),
        problem: problem.into(),
    }
}
